//! The changelog view (`cap:changelog-view`, `ver:changelog-view`).
//!
//! The check was written BEFORE the code, so these are the cases the design
//! said would constitute proof. Each one is a way the capability could report
//! success while being wrong:
//!
//! 1. buckets **mapped** from graph vocabulary rather than guessed;
//! 2. the `[Unreleased]` window is everything after the last **deployed**
//!    release, not the last release node;
//! 3. an empty delta renders as an **empty draft**, not an invented entry;
//! 4. the output is a **draft** — no entry claims what a consumer should do,
//!    because the graph cannot know it.
//!
//! The load-bearing ones are 2 and 4. A window bounded by the newest release
//! node looks identical to a correct one until somebody plans a release ahead
//! of time — which this project now does routinely — and an entry that quietly
//! asserts consumer impact is the commit-log dump Keep a Changelog names an
//! antipattern.

use reflow2_core::nodes::{Props, edge, node};
use reflow2_core::{
    BalanceSource, ChangeAction, ChangeRecord, ChangeType, ChangelogBucket, DesignGraph, EpochType,
    StatusSnapshotOutcome, StatusSnapshotPayload, StatusSnapshotReport, StoredAs, changelog_rule,
    status_snapshot_id,
};

/// Two releases, each pinned to an epoch, with `before` deployed. The shape
/// every test here starts from.
fn two_releases() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_epoch("epoch:one", "v1 cut", EpochType::ReleaseCut, 10)
        .unwrap();
    g.add_epoch("epoch:two", "v2 cut", EpochType::ReleaseCut, 20)
        .unwrap();
    g.precedes("epoch:one", "epoch:two").unwrap();

    g.create_node(
        node::RELEASE,
        "rel:one",
        Props::new()
            .set("name", "v1.0.0")
            .set("version", "1.0.0")
            .set("status", "deployed"),
    )
    .unwrap();
    g.create_node(
        node::RELEASE,
        "rel:two",
        Props::new()
            .set("name", "v2.0.0")
            .set("version", "2.0.0")
            .set("status", "planned"),
    )
    .unwrap();
    g.create_edge(
        edge::AT_EPOCH,
        node::RELEASE,
        "rel:one",
        node::DESIGN_EPOCH,
        "epoch:one",
        Props::new(),
    )
    .unwrap();
    g.create_edge(
        edge::AT_EPOCH,
        node::RELEASE,
        "rel:two",
        node::DESIGN_EPOCH,
        "epoch:two",
        Props::new(),
    )
    .unwrap();
    g
}

fn a_requirement(g: &mut DesignGraph, id: &str, name: &str) {
    g.add_requirement(id, name, "a statement").unwrap();
}

// -------------------------------------------------------------------------
// 1. Buckets are MAPPED, and every entry says which rule mapped it.
// -------------------------------------------------------------------------

#[test]
fn every_bucket_is_reached_by_a_named_rule_and_never_by_a_guess() {
    let mut g = two_releases();
    a_requirement(&mut g, "req:new", "A new one");
    a_requirement(&mut g, "req:edited", "An edited one");
    a_requirement(&mut g, "req:gone", "A removed one");
    a_requirement(&mut g, "req:retired", "A deprecated one");

    for (ev, target, ct, action) in [
        (
            "chg:a",
            "req:new",
            ChangeType::NewFeature,
            ChangeAction::Added,
        ),
        (
            "chg:b",
            "req:edited",
            ChangeType::ScopeChange,
            ChangeAction::Modified,
        ),
        (
            "chg:c",
            "req:gone",
            ChangeType::Refactor,
            ChangeAction::Removed,
        ),
        (
            "chg:d",
            "req:retired",
            ChangeType::Deprecation,
            ChangeAction::Removed,
        ),
    ] {
        g.record_change(ChangeRecord {
            epoch_id: "epoch:two",
            change_event_id: ev,
            name: "a change",
            change_type: ct,
            subject: None,
            target_type: node::REQUIREMENT,
            target_id: target,
            action,
            repair: None,
        })
        .unwrap();
    }

    let draft = g.changelog_view(Some("rel:one"), Some("rel:two")).unwrap();

    let find = |id: &str| {
        draft
            .entries
            .iter()
            .find(|e| e.subject_id == id)
            .unwrap_or_else(|| panic!("no entry for {id} in {:?}", draft.entries))
            .clone()
    };

    assert_eq!(find("req:new").bucket, ChangelogBucket::Added);
    assert_eq!(find("req:new").rule, changelog_rule::ACTION_ADDED);
    assert_eq!(find("req:edited").bucket, ChangelogBucket::Changed);
    assert_eq!(find("req:edited").rule, changelog_rule::ACTION_MODIFIED);
    assert_eq!(find("req:gone").bucket, ChangelogBucket::Removed);
    assert_eq!(find("req:gone").rule, changelog_rule::ACTION_REMOVED);

    // Deprecation is NOT Removed: retirement with the intent recorded is a
    // different claim from a thing that simply left, and Keep a Changelog
    // keeps them apart because consumers act on them differently.
    assert_eq!(find("req:retired").bucket, ChangelogBucket::Deprecated);
    assert_eq!(
        find("req:retired").rule,
        changelog_rule::ACTION_REMOVED_DEPRECATION
    );

    // Every entry carries a rule from the declared set — nothing arrives in a
    // bucket without naming what put it there.
    for e in &draft.entries {
        assert!(
            changelog_rule::ALL.contains(&e.rule.as_str()),
            "entry {e:?} carries a rule outside the declared set"
        );
        assert!(!e.evidence.is_empty(), "a mapping with no evidence: {e:?}");
    }
}

#[test]
fn a_change_no_rule_covers_is_reported_unfiled_rather_than_dropped() {
    let mut g = two_releases();
    a_requirement(&mut g, "req:odd", "An odd one");
    // A CHANGED edge with no `action` at all — the shape a future writer could
    // introduce. It must not vanish, and must not be guessed into a bucket.
    g.add_change_event(
        "chg:odd",
        "no action recorded",
        ChangeType::Resync,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    g.pin_at_epoch(node::CHANGE_EVENT, "chg:odd", "epoch:two")
        .unwrap();
    g.create_edge(
        edge::CHANGED,
        node::CHANGE_EVENT,
        "chg:odd",
        node::REQUIREMENT,
        "req:odd",
        Props::new(),
    )
    .unwrap();

    let draft = g.changelog_view(Some("rel:one"), Some("rel:two")).unwrap();

    assert!(
        draft.entries.iter().all(|e| e.subject_id != "req:odd"),
        "an unmapped change must not be filed into a bucket"
    );
    assert_eq!(draft.unmapped.len(), 1, "and must not be dropped either");
    assert_eq!(draft.unmapped[0].subject_id, "req:odd");
    assert!(draft.needs_a_human.iter().any(|s| s.contains("no bucket")));
}

// -------------------------------------------------------------------------
// 2. [Unreleased] is bounded by the last DEPLOYED release.
// -------------------------------------------------------------------------

#[test]
fn unreleased_is_bounded_by_the_last_deployed_release_not_the_newest_one() {
    let mut g = two_releases();
    // rel:two exists, sits LATER on the axis, and is only `planned`. A window
    // bounded by "the newest release node" would start at epoch:two and miss
    // everything below — which is exactly the work [Unreleased] is for.
    a_requirement(&mut g, "req:after", "Landed after v1 shipped");
    g.record_change(ChangeRecord {
        epoch_id: "epoch:two",
        change_event_id: "chg:after",
        name: "after the deployed cut",
        change_type: ChangeType::NewFeature,
        subject: None,
        target_type: node::REQUIREMENT,
        target_id: "req:after",
        action: ChangeAction::Added,
        repair: None,
    })
    .unwrap();

    let draft = g.changelog_view(None, None).unwrap();

    assert_eq!(draft.heading, "[Unreleased]");
    assert_eq!(
        draft.from.as_deref(),
        Some("rel:one"),
        "the base must be the DEPLOYED release, not the newest node"
    );
    assert_eq!(draft.from_sequence, Some(10));
    assert!(
        draft.entries.iter().any(|e| e.subject_id == "req:after"),
        "work after the deployed cut belongs in [Unreleased], got {:?}",
        draft.entries
    );
}

#[test]
fn a_release_with_no_epoch_is_announced_rather_than_silently_widening_the_window() {
    let mut g = two_releases();
    // The defect found live on 2026-07-31: v0.19.0 was cut with its epoch
    // created but never linked, and nothing said so.
    g.create_node(
        node::RELEASE,
        "rel:orphan",
        Props::new()
            .set("name", "v3.0.0")
            .set("version", "3.0.0")
            .set("status", "deployed"),
    )
    .unwrap();

    let draft = g
        .changelog_view(Some("rel:orphan"), Some("rel:two"))
        .unwrap();

    assert!(
        draft.notes.iter().any(|n| n.contains("no epoch")),
        "a release with no AT_EPOCH must be called out, got {:?}",
        draft.notes
    );
    assert_eq!(draft.from_sequence, None);
}

// -------------------------------------------------------------------------
// 3. An empty delta is an empty draft, not an invented entry.
// -------------------------------------------------------------------------

#[test]
fn an_empty_window_renders_an_empty_draft_and_invents_nothing() {
    let g = two_releases();
    let draft = g.changelog_view(Some("rel:one"), Some("rel:two")).unwrap();

    assert!(
        draft.entries.is_empty(),
        "nothing changed, so nothing is drafted — got {:?}",
        draft.entries
    );
    assert!(draft.unmapped.is_empty());
    assert!(
        draft.manifest.appeared.is_empty() && draft.manifest.left.is_empty(),
        "neither release ships anything yet"
    );
    // Still a draft, still headed, still honest about what a person owes.
    assert!(draft.is_draft);
    assert_eq!(draft.heading, "[2.0.0]");
    assert!(!draft.needs_a_human.is_empty());
}

#[test]
fn the_manifest_delta_files_what_appeared_and_what_left() {
    let mut g = two_releases();
    g.add_artifact("art:kept", "kept.rs", None, None).unwrap();
    g.add_artifact("art:new", "new.rs", None, None).unwrap();
    g.add_artifact("art:gone", "gone.rs", None, None).unwrap();

    g.release_includes("rel:one", node::ARTIFACT, "art:kept", None)
        .unwrap();
    g.release_includes("rel:one", node::ARTIFACT, "art:gone", None)
        .unwrap();
    g.release_includes("rel:two", node::ARTIFACT, "art:kept", None)
        .unwrap();
    g.release_includes("rel:two", node::ARTIFACT, "art:new", None)
        .unwrap();

    let draft = g.changelog_view(Some("rel:one"), Some("rel:two")).unwrap();

    assert_eq!(draft.manifest.appeared, vec!["art:new".to_string()]);
    assert_eq!(draft.manifest.left, vec!["art:gone".to_string()]);

    let added: Vec<_> = draft
        .entries
        .iter()
        .filter(|e| e.bucket == ChangelogBucket::Added)
        .map(|e| e.subject_id.as_str())
        .collect();
    assert_eq!(added, vec!["art:new"]);
    let removed: Vec<_> = draft
        .entries
        .iter()
        .filter(|e| e.bucket == ChangelogBucket::Removed)
        .map(|e| e.subject_id.as_str())
        .collect();
    assert_eq!(removed, vec!["art:gone"]);
    // art:kept is in both manifests and therefore in neither list — a
    // changelog reports the DELTA, not the inventory.
    assert!(draft.entries.iter().all(|e| e.subject_id != "art:kept"));
}

// -------------------------------------------------------------------------
// 4. It is a DRAFT: no entry claims what a consumer should do.
// -------------------------------------------------------------------------

#[test]
fn no_entry_asserts_consumer_impact_and_the_obligation_is_named_instead() {
    let mut g = two_releases();
    a_requirement(&mut g, "req:x", "Something");
    g.record_change(ChangeRecord {
        epoch_id: "epoch:two",
        change_event_id: "chg:x",
        name: "a change",
        change_type: ChangeType::NewFeature,
        subject: None,
        target_type: node::REQUIREMENT,
        target_id: "req:x",
        action: ChangeAction::Added,
        repair: None,
    })
    .unwrap();

    let draft = g.changelog_view(Some("rel:one"), Some("rel:two")).unwrap();

    assert!(draft.is_draft, "is_draft is not negotiable");
    assert!(
        !draft.entries.is_empty(),
        "this test is worthless without an entry to inspect"
    );

    // An entry carries WHAT moved and WHY it is in this bucket. It must not
    // carry an instruction — that is the half the graph cannot know, and
    // asserting it would be the fabrication `dec:report-dont-judge` forbids.
    for e in &draft.entries {
        let text = format!("{} {} {}", e.subject_name, e.rule, e.evidence).to_lowercase();
        for imperative in [
            "you should",
            "upgrade to",
            "you must",
            "re-run",
            "no action required",
        ] {
            assert!(
                !text.contains(imperative),
                "entry {e:?} asserts consumer impact via {imperative:?}"
            );
        }
    }

    // And the obligation is NAMED rather than left absent, so a reader cannot
    // mistake a draft for a finished changelog.
    assert!(
        draft
            .needs_a_human
            .iter()
            .any(|s| s.to_lowercase().contains("consumer")),
        "the consumer-impact obligation must be stated: {:?}",
        draft.needs_a_human
    );
}

#[test]
fn the_same_window_twice_produces_the_identical_draft() {
    let mut g = two_releases();
    a_requirement(&mut g, "req:a", "A");
    a_requirement(&mut g, "req:b", "B");
    for (ev, target) in [("chg:1", "req:a"), ("chg:2", "req:b")] {
        g.record_change(ChangeRecord {
            epoch_id: "epoch:two",
            change_event_id: ev,
            name: "a change",
            change_type: ChangeType::NewFeature,
            subject: None,
            target_type: node::REQUIREMENT,
            target_id: target,
            action: ChangeAction::Added,
            repair: None,
        })
        .unwrap();
    }

    let a = g.changelog_view(Some("rel:one"), Some("rel:two")).unwrap();
    let b = g.changelog_view(Some("rel:one"), Some("rel:two")).unwrap();
    assert_eq!(a, b, "a derived view must be deterministic");
}

#[test]
fn a_drift_accept_that_repaired_the_code_is_fixed_not_changed() {
    let mut g = two_releases();
    g.add_artifact("art:thing", "thing.rs", None, Some("src/thing.rs"))
        .unwrap();
    // The baseline the accept below moves. It has to be established as its own
    // act (BL-157) — and note what this replaced: the fixture used to pass
    // "sha256:aaaa" as `add_artifact`'s fourth argument, which is the LOCATION.
    // The artifact had no checksum at all and the test passed anyway, because
    // nothing had ever checked that an accept had something to accept.
    g.set_artifact_checksum(
        "art:thing",
        "sha256:aaaa",
        reflow2_core::DriftDisposition::BaselineEstablished,
        None,
        Some("2026-07-30"),
    )
    .unwrap();

    // `set_artifact_checksum` with DesignHolds writes its own ChangeEvent and
    // marks the CHANGED edge `accepted_baseline`. That edge carries
    // action=modified, so if the accept rule were tried AFTER the modified
    // rule this would be filed as Changed and the Fixed bucket would be
    // unreachable in practice while still existing in the enum.
    let (_, ev) = g
        .set_artifact_checksum(
            "art:thing",
            "sha256:bbbb",
            reflow2_core::DriftDisposition::DesignHolds {
                change_type: ChangeType::TestFailureFix,
            },
            Some("restored intended behaviour"),
            Some("2026-07-31"),
        )
        .unwrap();
    g.pin_at_epoch(node::CHANGE_EVENT, &ev, "epoch:two")
        .unwrap();

    let draft = g.changelog_view(Some("rel:one"), Some("rel:two")).unwrap();
    let entry = draft
        .entries
        .iter()
        .find(|e| e.subject_id == "art:thing")
        .unwrap_or_else(|| panic!("no entry for the accepted artifact: {:?}", draft.entries));

    assert_eq!(entry.bucket, ChangelogBucket::Fixed);
    assert_eq!(entry.rule, changelog_rule::ACCEPT_TEST_FAILURE_FIX);
}

#[test]
fn an_accept_that_is_not_a_fix_stays_out_of_the_fixed_bucket() {
    let mut g = two_releases();
    g.add_artifact("art:refactored", "r.rs", None, Some("src/r.rs"))
        .unwrap();
    g.set_artifact_checksum(
        "art:refactored",
        "sha256:aaaa",
        reflow2_core::DriftDisposition::BaselineEstablished,
        None,
        Some("2026-07-30"),
    )
    .unwrap();
    // The counterweight: a `design_holds` accept for a REFACTOR carried no
    // design meaning and certainly fixed nothing. Without this case the rule
    // above could be "any accept is Fixed" and still pass.
    let (_, ev) = g
        .set_artifact_checksum(
            "art:refactored",
            "sha256:cccc",
            reflow2_core::DriftDisposition::DesignHolds {
                change_type: ChangeType::Refactor,
            },
            None,
            None,
        )
        .unwrap();
    g.pin_at_epoch(node::CHANGE_EVENT, &ev, "epoch:two")
        .unwrap();

    let draft = g.changelog_view(Some("rel:one"), Some("rel:two")).unwrap();
    let entry = draft
        .entries
        .iter()
        .find(|e| e.subject_id == "art:refactored")
        .unwrap();
    assert_eq!(entry.bucket, ChangelogBucket::Changed);
    assert_eq!(entry.rule, changelog_rule::ACTION_MODIFIED);
}

// =========================================================================
// The balance line (`cap:changelog-view-carries-a-balance-line`,
// `dec:how-the-changelog-balance-is-counted`, option C).
//
// Each release keeps a STATUS SNAPSHOT, written by `release_includes_all` with
// `apply: true` at the cut: the status of every Requirement, Capability,
// Decision and Verification, and whether an accepted decision has withdrawn
// it. The first is stored whole and each later one as a delta from the
// previous release's. `changelog_view` compares two releases' reconstructed
// states and reports what was added beside what was taken away.
//
// The ways this could report success while being wrong, which these tests
// exist to catch:
//
// - a count of ZERO where an end simply kept no snapshot — the false
//   reassurance the measure exists to prevent;
// - a delta that does not reconstruct to what the release saw;
// - a re-run of the cut quietly rewriting what a past release saw;
// - a withdrawn capability read as live because its status did not move;
// - the window's own blind spot (added and dropped inside it) left unsaid.
// =========================================================================

/// One of each kind of intent the balance counts, at a starting status.
fn intent(g: &mut DesignGraph) {
    for id in ["req:keep", "req:drop", "req:defer"] {
        g.add_requirement(id, id, "a statement").unwrap();
        g.set_requirement_status(id, "accepted").unwrap();
    }
    for id in ["cap:keep", "cap:gone", "cap:withdrawn"] {
        g.add_capability(id, id, "a capability", Some("realized"))
            .unwrap();
    }
    for id in ["dec:stay", "dec:super", "dec:rej"] {
        g.add_decision(id, id, "a decision", None).unwrap();
        g.set_decision_status(id, "accepted").unwrap();
    }
    g.add_decision("dec:later", "dec:later", "a decision", None)
        .unwrap();
    for id in ["ver:ok", "ver:breaks"] {
        g.add_verification(id, id, Some("test"), None, None)
            .unwrap();
        g.set_verification_status(id, "passing", None, None)
            .unwrap();
    }
}

/// A cut in the order `flow:release-cut` runs it: the release exists, its
/// manifest is derived with `apply: true` (step 4, which keeps the status
/// snapshot), and only THEN is it pinned to its epoch (step 5). Deployed, so
/// `[Unreleased]` reads from it.
fn cut(g: &mut DesignGraph, rel: &str, epoch: &str, seq: i64) -> StatusSnapshotReport {
    g.create_node(
        node::RELEASE,
        rel,
        Props::new()
            .set("name", rel)
            .set("version", rel)
            .set("status", "deployed"),
    )
    .unwrap();
    let manifest = g.release_includes_all(rel, &[], true).unwrap();
    g.add_epoch(epoch, epoch, EpochType::ReleaseCut, seq)
        .unwrap();
    g.pin_at_epoch(node::RELEASE, rel, epoch).unwrap();
    manifest.status_snapshot
}

fn payload_of(g: &DesignGraph, rel: &str) -> StatusSnapshotPayload {
    let snap = g
        .get_node(node::SNAPSHOT, &status_snapshot_id(rel))
        .unwrap()
        .unwrap_or_else(|| panic!("no status snapshot kept for {rel}"));
    let state = snap
        .properties
        .get("state")
        .and_then(reflow2_core::foundation::core::Value::as_str)
        .expect("the payload is the Snapshot's `state`");
    serde_json::from_str(state).expect("the payload parses")
}

fn ids(items: &[reflow2_core::BalanceItem]) -> Vec<&str> {
    items.iter().map(|i| i.id.as_str()).collect()
}

#[test]
fn the_cut_keeps_a_status_snapshot_of_every_requirement_capability_decision_and_check() {
    let mut g = DesignGraph::open_in_memory().unwrap();
    intent(&mut g);
    g.create_node(
        node::RELEASE,
        "rel:one",
        Props::new().set("name", "one").set("status", "planned"),
    )
    .unwrap();

    // A dry run says what it WOULD keep and writes nothing.
    let dry = g.release_includes_all("rel:one", &[], false).unwrap();
    assert_eq!(dry.status_snapshot.outcome, StatusSnapshotOutcome::DryRun);
    assert_eq!(dry.status_snapshot.statuses, Some(12));
    assert!(
        g.get_node(node::SNAPSHOT, &status_snapshot_id("rel:one"))
            .unwrap()
            .is_none(),
        "a dry run must write nothing"
    );

    let kept = g
        .release_includes_all("rel:one", &[], true)
        .unwrap()
        .status_snapshot;
    assert_eq!(kept.outcome, StatusSnapshotOutcome::Written);
    assert_eq!(kept.stored_as, Some(StoredAs::Whole));
    assert_eq!(kept.base_release, None);
    assert_eq!(kept.entries, Some(12), "the first is stored whole");
    assert_eq!(kept.statuses, Some(12));
    assert!(
        kept.note.contains("12"),
        "the reply says how many entries it holds: {}",
        kept.note
    );

    // Stored on the existing Snapshot type, linked to the release.
    let snap = g
        .get_node(node::SNAPSHOT, &status_snapshot_id("rel:one"))
        .unwrap()
        .unwrap();
    assert_eq!(
        snap.properties
            .get("target_id")
            .and_then(reflow2_core::foundation::core::Value::as_str),
        Some("rel:one")
    );
    assert!(
        g.outgoing("rel:one", Some(edge::HAS_SNAPSHOT))
            .unwrap()
            .iter()
            .any(|e| e.to_id == status_snapshot_id("rel:one")),
        "the release links to its snapshot"
    );

    let p = payload_of(&g, "rel:one");
    assert_eq!(p.stored_as, StoredAs::Whole);
    assert_eq!(
        p.covers,
        ["Capability", "Decision", "Requirement", "Verification"]
    );
    assert_eq!(
        p.set["Requirement"]["req:drop"].status.as_deref(),
        Some("accepted")
    );
    assert_eq!(
        p.set["Decision"]["dec:later"].status.as_deref(),
        Some("proposed")
    );
    assert_eq!(
        p.set["Verification"]["ver:breaks"].status.as_deref(),
        Some("passing")
    );
    assert_eq!(
        g.release_statuses("rel:one").unwrap(),
        Some(g.current_statuses().unwrap())
    );
}

#[test]
fn a_discontinued_node_is_kept_as_discontinued_because_its_status_does_not_move() {
    let mut g = DesignGraph::open_in_memory().unwrap();
    intent(&mut g);
    cut(&mut g, "rel:one", "epoch:one", 10);

    g.add_decision("dec:withdraw", "withdraw it", "withdrawn", None)
        .unwrap();
    g.set_decision_status("dec:withdraw", "accepted").unwrap();
    g.create_edge(
        edge::OBSOLETES,
        node::DECISION,
        "dec:withdraw",
        node::CAPABILITY,
        "cap:withdrawn",
        Props::new(),
    )
    .unwrap();
    cut(&mut g, "rel:two", "epoch:two", 20);

    let two = g.release_statuses("rel:two").unwrap().unwrap();
    let withdrawn = &two["Capability"]["cap:withdrawn"];
    assert_eq!(
        withdrawn.status.as_deref(),
        Some("realized"),
        "the stored status still records what was built"
    );
    assert!(withdrawn.discontinued, "and the snapshot says it is gone");
    assert!(!two["Capability"]["cap:keep"].discontinued);

    let balance = g
        .changelog_view(Some("rel:one"), Some("rel:two"))
        .unwrap()
        .balance;
    let caps = balance.capabilities.expect("both ends keep a snapshot");
    assert_eq!(ids(&caps.taken_away.items), ["cap:withdrawn"]);
    assert!(
        caps.taken_away.items[0].how.contains("withdrawn"),
        "{}",
        caps.taken_away.items[0].how
    );
}

#[test]
fn three_cuts_keep_one_whole_snapshot_and_two_deltas_that_reconstruct_exactly() {
    let mut g = DesignGraph::open_in_memory().unwrap();
    intent(&mut g);
    let one = cut(&mut g, "rel:one", "epoch:one", 10);
    let at_one = g.current_statuses().unwrap();

    g.set_requirement_status("req:drop", "dropped").unwrap();
    g.add_requirement("req:new", "new", "a statement").unwrap();
    let two = cut(&mut g, "rel:two", "epoch:two", 20);
    let at_two = g.current_statuses().unwrap();

    g.set_verification_status("ver:breaks", "failing", None, None)
        .unwrap();
    g.delete_node(node::CAPABILITY, "cap:gone").unwrap();
    let three = cut(&mut g, "rel:three", "epoch:three", 30);
    let at_three = g.current_statuses().unwrap();

    assert_eq!(one.stored_as, Some(StoredAs::Whole));
    assert_eq!(two.stored_as, Some(StoredAs::Delta));
    assert_eq!(two.base_release.as_deref(), Some("rel:one"));
    assert_eq!(two.entries, Some(2), "one status moved, one node added");
    assert_eq!(two.statuses, Some(13));
    assert_eq!(three.stored_as, Some(StoredAs::Delta));
    assert_eq!(three.base_release.as_deref(), Some("rel:two"));
    assert_eq!(three.entries, Some(2), "one status moved, one node removed");
    assert_eq!(three.statuses, Some(12));

    let p2 = payload_of(&g, "rel:two");
    assert_eq!(
        p2.base_snapshot.as_deref(),
        Some(status_snapshot_id("rel:one").as_str())
    );
    assert_eq!(
        p2.set["Requirement"]["req:drop"].status.as_deref(),
        Some("dropped")
    );
    assert!(p2.set["Requirement"].contains_key("req:new"));
    assert!(
        !p2.set.contains_key("Capability"),
        "a delta holds only what moved"
    );
    let p3 = payload_of(&g, "rel:three");
    assert_eq!(p3.removed["Capability"], ["cap:gone"]);

    assert_eq!(g.release_statuses("rel:one").unwrap(), Some(at_one));
    assert_eq!(g.release_statuses("rel:two").unwrap(), Some(at_two));
    assert_eq!(g.release_statuses("rel:three").unwrap(), Some(at_three));
}

#[test]
fn a_rerun_of_apply_leaves_a_past_snapshot_exactly_as_it_was() {
    let mut g = DesignGraph::open_in_memory().unwrap();
    intent(&mut g);
    cut(&mut g, "rel:one", "epoch:one", 10);
    let before = g
        .get_node(node::SNAPSHOT, &status_snapshot_id("rel:one"))
        .unwrap()
        .unwrap();
    let at_cut = g.release_statuses("rel:one").unwrap();

    // The design moves on, and the cut's derivation is run again.
    g.set_requirement_status("req:keep", "dropped").unwrap();
    g.add_artifact("art:late", "late.rs", None, Some("src/late.rs"))
        .unwrap();
    let again = g.release_includes_all("rel:one", &[], true).unwrap();

    assert_eq!(
        again.status_snapshot.outcome,
        StatusSnapshotOutcome::AlreadyKept
    );
    assert_eq!(again.status_snapshot.entries, Some(12));
    let after = g
        .get_node(node::SNAPSHOT, &status_snapshot_id("rel:one"))
        .unwrap()
        .unwrap();
    assert_eq!(
        before.properties, after.properties,
        "what a release saw must not move"
    );
    assert_eq!(g.release_statuses("rel:one").unwrap(), at_cut);
}

#[test]
fn a_release_whose_manifest_was_frozen_earlier_gets_no_snapshot_dated_today() {
    let mut g = DesignGraph::open_in_memory().unwrap();
    intent(&mut g);
    g.add_artifact("art:a", "a.rs", None, Some("src/a.rs"))
        .unwrap();
    g.create_node(
        node::RELEASE,
        "rel:old",
        Props::new().set("name", "old").set("status", "deployed"),
    )
    .unwrap();
    // Cut before releases kept a snapshot: its manifest was typed out.
    g.release_includes("rel:old", node::ARTIFACT, "art:a", None)
        .unwrap();

    let report = g
        .release_includes_all("rel:old", &[], true)
        .unwrap()
        .status_snapshot;
    assert_eq!(report.outcome, StatusSnapshotOutcome::NotTaken);
    assert!(
        report.note.contains("frozen"),
        "the reply says why: {}",
        report.note
    );
    assert!(
        g.get_node(node::SNAPSHOT, &status_snapshot_id("rel:old"))
            .unwrap()
            .is_none(),
        "today's statuses must not be written down as what an old release saw"
    );
}

#[test]
fn each_balance_count_says_what_it_counted_and_how() {
    let mut g = DesignGraph::open_in_memory().unwrap();
    intent(&mut g);
    cut(&mut g, "rel:one", "epoch:one", 10);

    // Requirements: one added, one dropped, one deferred.
    g.add_requirement("req:new", "new", "a statement").unwrap();
    g.set_requirement_status("req:drop", "dropped").unwrap();
    g.set_requirement_status("req:defer", "deferred").unwrap();
    // Capabilities: one added, one removed, one withdrawn.
    g.add_capability("cap:new", "new", "a capability", None)
        .unwrap();
    g.delete_node(node::CAPABILITY, "cap:gone").unwrap();
    g.add_decision("dec:withdraw", "withdraw it", "withdrawn", None)
        .unwrap();
    g.set_decision_status("dec:withdraw", "accepted").unwrap();
    g.create_edge(
        edge::OBSOLETES,
        node::DECISION,
        "dec:withdraw",
        node::CAPABILITY,
        "cap:withdrawn",
        Props::new(),
    )
    .unwrap();
    // Decisions: two accepted (dec:later, dec:withdraw), two reversed.
    g.set_decision_status("dec:later", "accepted").unwrap();
    g.set_decision_status("dec:super", "superseded").unwrap();
    g.set_decision_status("dec:rej", "rejected").unwrap();
    // Checks: one went from passing to failing.
    g.set_verification_status("ver:breaks", "failing", None, None)
        .unwrap();

    cut(&mut g, "rel:two", "epoch:two", 20);
    // A fix a check caught, pinned into the window.
    g.add_change_event(
        "chg:fix",
        "a check caught it",
        ChangeType::TestFailureFix,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    g.pin_at_epoch(node::CHANGE_EVENT, "chg:fix", "epoch:two")
        .unwrap();

    let draft = g.changelog_view(Some("rel:one"), Some("rel:two")).unwrap();
    let b = draft.balance;
    assert!(b.counted, "both ends keep a snapshot: {:?}", b.notes);
    assert_eq!(b.from.source, BalanceSource::Snapshot);
    assert_eq!(b.to.source, BalanceSource::Snapshot);
    assert_eq!(b.from.id.as_deref(), Some("rel:one"));
    assert_eq!(b.to.id.as_deref(), Some("rel:two"));

    let reqs = b.requirements.unwrap();
    assert_eq!(ids(&reqs.added.items), ["req:new"]);
    assert_eq!(ids(&reqs.taken_away.items), ["req:defer", "req:drop"]);
    assert_eq!(reqs.added.count, 1);
    assert_eq!(reqs.taken_away.count, 2);
    let drop = &reqs.taken_away.items[1];
    assert_eq!(drop.before.as_deref(), Some("accepted"));
    assert_eq!(drop.after.as_deref(), Some("dropped"));

    let caps = b.capabilities.unwrap();
    assert_eq!(ids(&caps.added.items), ["cap:new"]);
    assert_eq!(ids(&caps.taken_away.items), ["cap:gone", "cap:withdrawn"]);
    assert!(
        caps.taken_away.items[0]
            .how
            .contains("no longer in the design")
    );

    let decs = b.decisions.unwrap();
    assert_eq!(ids(&decs.added.items), ["dec:later", "dec:withdraw"]);
    assert_eq!(ids(&decs.taken_away.items), ["dec:rej", "dec:super"]);

    let checks = b.checks_now_failing.unwrap();
    assert_eq!(ids(&checks.items), ["ver:breaks"]);
    assert_eq!(checks.count, 1);

    assert_eq!(ids(&b.failures_that_came_back.items), ["chg:fix"]);

    // Every count says how it was counted.
    for column in [
        &reqs.added,
        &reqs.taken_away,
        &caps.added,
        &caps.taken_away,
        &decs.added,
        &decs.taken_away,
        &checks,
        &b.failures_that_came_back,
    ] {
        assert!(
            !column.how_counted.is_empty(),
            "a count with no method: {column:?}"
        );
        assert_eq!(column.count, column.items.len());
    }
    // And it reports; it never judges.
    assert!(
        b.limits
            .iter()
            .any(|l| l.contains("never says a design does too much")),
        "{:?}",
        b.limits
    );
}

#[test]
fn an_end_with_no_snapshot_is_named_and_never_counted_as_zero() {
    // Two releases cut before releases kept snapshots.
    let mut g = two_releases();
    a_requirement(&mut g, "req:x", "x");

    let b = g
        .changelog_view(Some("rel:one"), Some("rel:two"))
        .unwrap()
        .balance;
    assert!(!b.counted);
    assert_eq!(b.from.source, BalanceSource::NotKept);
    assert!(b.requirements.is_none(), "no count, rather than a zero");
    assert!(b.capabilities.is_none());
    assert!(b.decisions.is_none());
    assert!(b.checks_now_failing.is_none());
    assert!(
        b.notes.iter().any(|n| n.contains("'rel:one'")),
        "the end with no snapshot is named: {:?}",
        b.notes
    );

    // An epoch end is named the same way: only a release keeps one.
    let b = g
        .changelog_view(Some("epoch:one"), Some("rel:two"))
        .unwrap()
        .balance;
    assert!(!b.counted);
    assert!(
        b.notes.iter().any(|n| n.contains("'epoch:one'")),
        "{:?}",
        b.notes
    );
}

#[test]
fn something_added_and_taken_away_inside_one_window_is_in_neither_column_and_the_limit_says_so() {
    let mut g = DesignGraph::open_in_memory().unwrap();
    intent(&mut g);
    cut(&mut g, "rel:one", "epoch:one", 10);
    g.add_requirement("req:brief", "came and went", "a statement")
        .unwrap();
    g.set_requirement_status("req:brief", "dropped").unwrap();
    cut(&mut g, "rel:two", "epoch:two", 20);

    let b = g
        .changelog_view(Some("rel:one"), Some("rel:two"))
        .unwrap()
        .balance;
    let reqs = b.requirements.unwrap();
    assert!(!ids(&reqs.added.items).contains(&"req:brief"));
    assert!(!ids(&reqs.taken_away.items).contains(&"req:brief"));
    assert!(
        b.limits.iter().any(|l| l.contains("neither column")),
        "the net-count limit is stated: {:?}",
        b.limits
    );
}

#[test]
fn an_unreleased_window_reads_the_last_deployed_release_against_the_live_design() {
    let mut g = DesignGraph::open_in_memory().unwrap();
    intent(&mut g);
    cut(&mut g, "rel:one", "epoch:one", 10);
    g.add_requirement("req:since", "since the cut", "a statement")
        .unwrap();
    g.set_decision_status("dec:super", "superseded").unwrap();

    let draft = g.changelog_view(None, None).unwrap();
    assert_eq!(draft.heading, "[Unreleased]");
    let b = draft.balance;
    assert!(b.counted, "{:?}", b.notes);
    assert_eq!(b.from.id.as_deref(), Some("rel:one"));
    assert_eq!(b.from.source, BalanceSource::Snapshot);
    assert_eq!(b.to.source, BalanceSource::LiveDesign);
    assert_eq!(b.to.id, None);
    assert_eq!(ids(&b.requirements.unwrap().added.items), ["req:since"]);
    assert_eq!(ids(&b.decisions.unwrap().taken_away.items), ["dec:super"]);
    assert!(
        b.limits.iter().any(|l| l.contains("live design")),
        "{:?}",
        b.limits
    );
}

#[test]
fn a_release_status_snapshot_is_not_read_as_the_release_nodes_own_history() {
    // It hangs from the release by HAS_SNAPSHOT, as a node's own prior state
    // does. Read as one, it would hand the schedule guard and arrival_delta a
    // snapshot of an EMPTY plan for the release.
    let mut g = DesignGraph::open_in_memory().unwrap();
    intent(&mut g);
    cut(&mut g, "rel:one", "epoch:one", 10);
    assert!(
        g.snapshots_of("rel:one").unwrap().is_empty(),
        "the status snapshot is not the release's own history"
    );
}
