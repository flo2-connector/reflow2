//! AN INCREMENT THAT WILL NOT BE FINISHED IS CLOSED HONESTLY —
//! req:an-increment-that-will-not-be-finished-can-be-closed-honestly
//! (Anthony, 2026-11-12), with his two choices of 2026-11-17:
//!
//! - Besides arriving, a planned increment closes three ways: `deferred`
//!   (still meant, later), `dropped` (no longer meant, with the reason) and
//!   `superseded` (by another named increment). The close is the owner's word.
//! - No unfinished item is defaulted: each is moved to another increment (a
//!   recorded slip) or dropped from it, and on a DEFERRAL may stay with it,
//!   always as an explicit answer.
//! - Dropping a leftover removes it from that increment only: the item keeps
//!   its own status.
//!
//! OBSERVED FAILING first: written before `close_increment` existed, on
//! origin/main e2bbdc6f3, where none of it compiles; and on that main
//! `set_epoch_status` writes no status but planned and arrived, so an increment
//! that would not be finished could only stay listed open for ever or be
//! marked arrived.

use reflow2_core::close_increment::{CloseAs, IncrementClose, Leftover, LeftoverFate};
use reflow2_core::graph::DesignGraph;
use reflow2_core::nodes::{edge, node};
use reflow2_core::temporal::{ChangeAction, ChangeRecord, ChangeType, EpochType, ScheduleOutcome};

fn design() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_project("proj:1", "Thing").expect("project");
    g.add_contributor("who:owner", "The Owner", Some("person"), None, None)
        .expect("owner");
    // An epoch that HAS happened, to file the plan's snapshot in.
    g.add_epoch("epoch:now", "Now", EpochType::Revision, 1)
        .expect("now");
    g
}

fn plan(g: &mut DesignGraph, id: &str) {
    g.plan_epoch(id, &format!("Increment {id}"), EpochType::Milestone, 2)
        .expect("plan");
}

fn need(g: &mut DesignGraph, req: &str, epoch: &str, modality: &str) {
    g.add_requirement(req, req, "it must hold")
        .expect("requirement");
    g.schedule_for(
        node::REQUIREMENT,
        req,
        node::DESIGN_EPOCH,
        epoch,
        modality,
        None,
    )
    .expect("schedule");
}

fn deliver(g: &mut DesignGraph, req: &str, suffix: &str) {
    let (cap, art, ver) = (
        format!("cap:{suffix}"),
        format!("art:{suffix}"),
        format!("ver:{suffix}"),
    );
    g.add_capability(&cap, suffix, "does it", Some("realized"))
        .expect("capability");
    g.satisfies(&cap, req).expect("satisfies");
    g.add_artifact(
        &art,
        suffix,
        Some("code"),
        Some(&format!("src/{suffix}.rs")),
    )
    .expect("artifact");
    g.realizes(&art, node::CAPABILITY, &cap, None, None)
        .expect("realizes");
    g.add_verification(&ver, suffix, Some("test"), None, None)
        .expect("verification");
    g.verifies(&ver, node::CAPABILITY, &cap).expect("verifies");
    g.set_verification_status(&ver, "passing", None, None)
        .expect("passing");
}

fn moves(item: &str, to: &str) -> Leftover {
    Leftover {
        item_id: item.into(),
        fate: LeftoverFate::Moves { to: to.into() },
    }
}

fn dropped(item: &str, reason: &str) -> Leftover {
    Leftover {
        item_id: item.into(),
        fate: LeftoverFate::Dropped {
            reason: reason.into(),
        },
    }
}

fn stays(item: &str) -> Leftover {
    Leftover {
        item_id: item.into(),
        fate: LeftoverFate::Stays,
    }
}

fn close<'a>(epoch: &'a str, how: CloseAs, leftovers: &'a [Leftover]) -> IncrementClose<'a> {
    IncrementClose {
        epoch_id: epoch,
        how,
        reason: Some("the owner said so"),
        superseded_by: None,
        leftovers,
        recorded_in: Some("epoch:now"),
        approver: "who:owner",
        acted_at: Some("2026-11-17"),
    }
}

fn status(g: &DesignGraph, epoch: &str) -> String {
    g.get_node(node::DESIGN_EPOCH, epoch)
        .expect("read")
        .expect("exists")
        .properties
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("arrived")
        .to_string()
}

fn scheduled_for(g: &DesignGraph, item: &str) -> Vec<String> {
    let mut to: Vec<String> = g
        .outgoing(item, Some(edge::SCHEDULED_FOR))
        .expect("edges")
        .into_iter()
        .map(|e| e.to_id)
        .collect();
    to.sort();
    to
}

fn refusal(r: Result<impl std::fmt::Debug, reflow2_core::foundation::core::DynoError>) -> String {
    format!("{:?}", r.expect_err("the close is refused"))
}

// ─── The three closes, and what each asks ──────────────────────────────────

#[test]
fn a_moved_item_slips_on_the_record_and_the_closed_increment_reads_it_deferred_to_where_it_went() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    plan(&mut g, "epoch:b");
    need(&mut g, "req:one", "epoch:a", "required");
    need(&mut g, "req:two", "epoch:a", "expected");
    need(&mut g, "req:done", "epoch:a", "expected");
    deliver(&mut g, "req:done", "done");

    let leftovers = [moves("req:one", "epoch:b"), moves("req:two", "epoch:b")];
    let closed = g
        .close_increment(&close("epoch:a", CloseAs::Dropped, &leftovers))
        .expect("the close is accepted");

    assert_eq!(status(&g, "epoch:a"), "dropped");
    assert_eq!((closed.delivered, closed.leftovers.len()), (1, 2));
    // Both moved: the second removal is not refused for the plan having moved
    // since the snapshot the close took.
    assert_eq!(scheduled_for(&g, "req:one"), vec!["epoch:b".to_string()]);
    assert_eq!(scheduled_for(&g, "req:two"), vec!["epoch:b".to_string()]);
    // The delivered item was not touched.
    assert_eq!(scheduled_for(&g, "req:done"), vec!["epoch:a".to_string()]);
    // The modality moved with it: a required claim stays required.
    let m = g
        .outgoing("req:one", Some(edge::SCHEDULED_FOR))
        .expect("edges")[0]
        .properties
        .get("modality")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    assert_eq!(m.as_deref(), Some("required"));

    // The slip is on the record: arrival_delta reads the plan as it was, and
    // each moved item as deferred to where it went, not the plan as always
    // smaller.
    assert!(
        closed.snapshot_id.is_some(),
        "the plan was snapshotted first"
    );
    let delta = g.arrival_delta("epoch:a").expect("delta");
    let one = delta
        .items
        .iter()
        .find(|i| i.item_id == "req:one")
        .expect("req:one is in the baseline");
    assert_eq!(
        one.outcome,
        ScheduleOutcome::Deferred {
            now_due_at: vec!["epoch:b".into()]
        }
    );
    assert!(
        delta.missed_obligations.is_empty(),
        "a moved obligation is not missed: {:?}",
        delta.missed_obligations
    );
    assert!(
        delta
            .notes
            .iter()
            .any(|n| n.contains("closed as `dropped`")),
        "{:?}",
        delta.notes
    );
}

#[test]
fn the_close_is_one_dated_change_carrying_every_fate_and_reason_and_the_owners_signature() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    plan(&mut g, "epoch:b");
    need(&mut g, "req:one", "epoch:a", "expected");
    need(&mut g, "req:two", "epoch:a", "expected");

    let leftovers = [
        moves("req:one", "epoch:b"),
        dropped("req:two", "nobody needs it now"),
    ];
    let closed = g
        .close_increment(&close("epoch:a", CloseAs::Dropped, &leftovers))
        .expect("closed");

    let chg = g
        .get_node(node::CHANGE_EVENT, &closed.change_event_id)
        .expect("read")
        .expect("the close's change exists");
    let summary = chg.properties["summary"].as_str().expect("summary");
    for needle in [
        "closed as dropped",
        "the owner said so",
        "'req:one' moved to 'epoch:b'",
        "'req:two' dropped from it (nobody needs it now)",
    ] {
        assert!(summary.contains(needle), "{needle:?} in {summary}");
    }
    assert_eq!(chg.properties["detected_at"].as_str(), Some("2026-11-17"));
    let changed: Vec<String> = g
        .outgoing(&closed.change_event_id, Some(edge::CHANGED))
        .expect("edges")
        .into_iter()
        .map(|e| e.to_id)
        .collect();
    for id in ["epoch:a", "req:one", "req:two", "epoch:b"] {
        assert!(changed.contains(&id.to_string()), "{id} in {changed:?}");
    }
    assert!(
        g.has_approver("epoch:a").expect("read"),
        "signed by the owner"
    );
    let epoch = g
        .get_node(node::DESIGN_EPOCH, "epoch:a")
        .expect("read")
        .expect("exists");
    assert_eq!(
        epoch.properties["close_reason"].as_str(),
        Some("the owner said so")
    );
}

#[test]
fn a_dropped_leftover_leaves_the_increment_only_and_keeps_its_own_status() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    need(&mut g, "req:one", "epoch:a", "expected");
    let before = g
        .get_node(node::REQUIREMENT, "req:one")
        .expect("read")
        .expect("exists")
        .properties
        .get("status")
        .cloned();

    let leftovers = [dropped("req:one", "out of scope")];
    g.close_increment(&close("epoch:a", CloseAs::Dropped, &leftovers))
        .expect("closed");

    assert!(scheduled_for(&g, "req:one").is_empty());
    let after = g
        .get_node(node::REQUIREMENT, "req:one")
        .expect("read")
        .expect("the requirement is still there")
        .properties
        .get("status")
        .cloned();
    assert_eq!(before, after, "closing a plan retires no intent");
    let delta = g.arrival_delta("epoch:a").expect("delta");
    assert_eq!(delta.items[0].outcome, ScheduleOutcome::Discontinued);
}

#[test]
fn a_deferred_increment_may_keep_its_items_and_leaves_the_open_list_counted_apart() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    plan(&mut g, "epoch:b");
    need(&mut g, "req:one", "epoch:a", "required");
    need(&mut g, "req:two", "epoch:a", "expected");

    let leftovers = [stays("req:one"), moves("req:two", "epoch:b")];
    g.close_increment(&close("epoch:a", CloseAs::Deferred, &leftovers))
        .expect("deferred");

    assert_eq!(status(&g, "epoch:a"), "deferred");
    assert_eq!(scheduled_for(&g, "req:one"), vec!["epoch:a".to_string()]);
    let open = g.open_increments().expect("open");
    assert!(
        open.increments.iter().all(|i| i.epoch_id != "epoch:a"),
        "a deferred increment is closed for now"
    );
    assert_eq!((open.open, open.deferred), (1, 1));
    assert!(
        open.counts_line().contains("1 deferred"),
        "{}",
        open.counts_line()
    );
    // What stays is a forecast, never a miss: the delta says it was deferred.
    let delta = g.arrival_delta("epoch:a").expect("delta");
    assert!(
        delta.notes.iter().any(|n| n.contains("DEFERRED")),
        "{:?}",
        delta.notes
    );
}

#[test]
fn reopening_a_deferred_increment_makes_it_open_again_and_clears_the_close() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    need(&mut g, "req:one", "epoch:a", "expected");
    let leftovers = [stays("req:one")];
    let first = g
        .close_increment(&close("epoch:a", CloseAs::Deferred, &leftovers))
        .expect("deferred");

    g.set_epoch_status("epoch:a", "planned").expect("reopened");
    let epoch = g
        .get_node(node::DESIGN_EPOCH, "epoch:a")
        .expect("read")
        .expect("exists");
    assert!(!epoch.properties.contains_key("close_reason"));
    let open = g.open_increments().expect("open");
    assert_eq!((open.open, open.deferred), (1, 0));

    // Closed again, it gets its own record: the first close is history.
    let second = g
        .close_increment(&close("epoch:a", CloseAs::Deferred, &leftovers))
        .expect("deferred again");
    assert_ne!(first.change_event_id, second.change_event_id);
    assert!(
        g.get_node(node::CHANGE_EVENT, &first.change_event_id)
            .expect("read")
            .is_some()
    );
}

#[test]
fn a_superseded_increment_names_the_one_that_took_its_place() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    plan(&mut g, "epoch:b");
    need(&mut g, "req:one", "epoch:a", "expected");
    let leftovers = [moves("req:one", "epoch:b")];

    let mut c = close("epoch:a", CloseAs::Superseded, &leftovers);
    let none = refusal(g.close_increment(&c));
    assert!(none.contains("superseded_by"), "{none}");
    c.superseded_by = Some("epoch:a");
    assert!(refusal(g.close_increment(&c)).contains("cannot supersede itself"));
    c.superseded_by = Some("epoch:nowhere");
    assert!(refusal(g.close_increment(&c)).contains("no DesignEpoch 'epoch:nowhere'"));
    assert_eq!(
        status(&g, "epoch:a"),
        "planned",
        "every refusal wrote nothing"
    );

    c.superseded_by = Some("epoch:b");
    g.close_increment(&c).expect("superseded");
    let epoch = g
        .get_node(node::DESIGN_EPOCH, "epoch:a")
        .expect("read")
        .expect("exists");
    assert_eq!(epoch.properties["status"].as_str(), Some("superseded"));
    assert_eq!(epoch.properties["superseded_by"].as_str(), Some("epoch:b"));

    // Only a supersession names a successor.
    plan(&mut g, "epoch:c");
    let mut wrong = close("epoch:c", CloseAs::Deferred, &[]);
    wrong.superseded_by = Some("epoch:b");
    assert!(refusal(g.close_increment(&wrong)).contains("only a `superseded` close"));
}

// ─── Nothing defaulted, nothing half-written ───────────────────────────────

#[test]
fn every_unfinished_item_needs_exactly_one_answer_and_one_refusal_names_every_problem() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    plan(&mut g, "epoch:b");
    need(&mut g, "req:one", "epoch:a", "expected");
    need(&mut g, "req:two", "epoch:a", "expected");
    need(&mut g, "req:three", "epoch:a", "expected");
    need(&mut g, "req:done", "epoch:a", "expected");
    deliver(&mut g, "req:done", "done");
    g.add_requirement("req:elsewhere", "elsewhere", "x")
        .expect("req");

    let leftovers = [
        moves("req:one", "epoch:b"),
        moves("req:one", "epoch:b"),
        dropped("req:done", "already delivered"),
        dropped("req:elsewhere", "never here"),
        stays("req:two"),
    ];
    let why = refusal(g.close_increment(&close("epoch:a", CloseAs::Dropped, &leftovers)));
    for needle in [
        "'req:one' is answered for twice",
        "'req:done' is scheduled for 'epoch:a' but is not unfinished",
        "'req:elsewhere' is not scheduled for 'epoch:a'",
        "'req:two' cannot stay",
        "1 unfinished item(s) have no answer: req:three",
        "nothing was written",
    ] {
        assert!(why.contains(needle), "{needle:?} in {why}");
    }
    assert_eq!(status(&g, "epoch:a"), "planned");
    assert_eq!(scheduled_for(&g, "req:one"), vec!["epoch:a".to_string()]);
    assert!(!g.has_approver("epoch:a").expect("read"));
}

#[test]
fn a_drop_needs_its_reason_and_each_dropped_leftover_needs_one_too() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    need(&mut g, "req:one", "epoch:a", "expected");
    let leftovers = [dropped("req:one", "  ")];
    let mut c = close("epoch:a", CloseAs::Dropped, &leftovers);
    c.reason = None;
    assert!(refusal(g.close_increment(&c)).contains("needs the reason"));
    c.reason = Some("gone");
    assert!(refusal(g.close_increment(&c)).contains("dropped from the increment with no reason"));
    assert_eq!(status(&g, "epoch:a"), "planned");
}

#[test]
fn an_item_moves_only_to_an_open_increment() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    need(&mut g, "req:one", "epoch:a", "expected");
    for (to, needle) in [
        ("epoch:a", "the increment being closed"),
        ("epoch:now", "reads `arrived`"),
        ("epoch:nowhere", "no DesignEpoch"),
    ] {
        let leftovers = [moves("req:one", to)];
        let why = refusal(g.close_increment(&close("epoch:a", CloseAs::Dropped, &leftovers)));
        assert!(why.contains(needle), "{needle:?} in {why}");
    }
}

#[test]
fn an_unrecorded_plan_needs_an_arrived_epoch_to_file_its_snapshot_in() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    plan(&mut g, "epoch:b");
    need(&mut g, "req:one", "epoch:a", "expected");
    let leftovers = [moves("req:one", "epoch:b")];
    let mut c = close("epoch:a", CloseAs::Dropped, &leftovers);
    c.recorded_in = None;
    assert!(refusal(g.close_increment(&c)).contains("`recorded_in`"));
    c.recorded_in = Some("epoch:b");
    assert!(refusal(g.close_increment(&c)).contains("has not happened"));
    assert_eq!(scheduled_for(&g, "req:one"), vec!["epoch:a".to_string()]);

    // With nothing leaving, no snapshot is needed and none is asked for.
    plan(&mut g, "epoch:c");
    let mut empty = close("epoch:c", CloseAs::Dropped, &[]);
    empty.recorded_in = None;
    let closed = g.close_increment(&empty).expect("closed");
    assert!(closed.snapshot_id.is_none());
}

#[test]
fn only_an_open_increment_closes_and_the_approver_must_be_a_contributor() {
    let mut g = design();
    let why = refusal(g.close_increment(&close("epoch:now", CloseAs::Dropped, &[])));
    assert!(why.contains("only an open increment"), "{why}");

    plan(&mut g, "epoch:a");
    let mut c = close("epoch:a", CloseAs::Dropped, &[]);
    c.approver = "who:nobody";
    assert!(refusal(g.close_increment(&c)).contains("no Contributor 'who:nobody'"));
    assert_eq!(status(&g, "epoch:a"), "planned");

    g.close_increment(&close("epoch:a", CloseAs::Dropped, &[]))
        .expect("closed");
    let again = refusal(g.close_increment(&close("epoch:a", CloseAs::Deferred, &[])));
    assert!(again.contains("reopen it first"), "{again}");
}

// ─── The other doors ───────────────────────────────────────────────────────

#[test]
fn set_epoch_status_writes_no_close_and_a_dropped_increment_does_not_arrive_unreopened() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    for s in ["deferred", "dropped", "superseded"] {
        let why = format!(
            "{:?}",
            g.set_epoch_status("epoch:a", s).expect_err("refused")
        );
        assert!(why.contains("close_increment"), "{s}: {why}");
    }
    g.close_increment(&close("epoch:a", CloseAs::Dropped, &[]))
        .expect("closed");
    let why = format!(
        "{:?}",
        g.set_epoch_status("epoch:a", "arrived")
            .expect_err("refused")
    );
    assert!(why.contains("reopen it first"), "{why}");
    g.set_epoch_status("epoch:a", "planned").expect("reopen");
    g.set_epoch_status("epoch:a", "arrived")
        .expect("then arrive");
}

#[test]
fn history_is_never_recorded_into_a_closed_increment() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    g.close_increment(&close("epoch:a", CloseAs::Dropped, &[]))
        .expect("closed");
    g.add_requirement("req:x", "x", "y").expect("req");
    let why = format!(
        "{:?}",
        g.record_change(ChangeRecord {
            epoch_id: "epoch:a",
            change_event_id: "chg:into-a-closed-one",
            name: "a change",
            change_type: ChangeType::ScopeChange,
            subject: None,
            target_type: node::REQUIREMENT,
            target_id: "req:x",
            action: ChangeAction::Modified,
            repair: None,
        })
        .expect_err("refused")
    );
    assert!(why.contains("never happened"), "{why}");
}
