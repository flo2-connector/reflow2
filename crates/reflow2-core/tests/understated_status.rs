//! A status that claims LESS than the design shows is asked about — the mirror
//! of `status_contradiction`, which only ever looked at the other direction.
//!
//! `status_contradiction` fires on OVERSTATEMENT: a Capability `verified` with
//! no passing check, a Requirement `met` with nothing satisfying it. Nothing
//! looked the other way, so a status that fell behind reality was silent:
//! `cap:governance-proposal` read `planned` while the skill that realizes it was
//! being served (dec:idea-should-an-understated-status-be-detected, 2026-08-11),
//! and flo2's design held all 21 Components at `planned` — the edge gateway
//! among them, serving production — while capabilities at `realized` and
//! `verified` were allocated to them (2026-09-30).
//!
//! The finding is `understated_status`. These tests match it by its KEY STRING,
//! not the enum variant, for two reasons: the key is load-bearing (gap ids hash
//! it, and an acknowledgement is stored under the id), and it let this file be
//! written and watched failing against the code before the detector existed.
//!
//! What counts as "the design shows it built" is deliberately DIRECT evidence:
//! - an Artifact that REALIZES the node and whose OWN status is `realized` or
//!   `verified` — an Artifact registered as `planned` or with no status is not
//!   evidence of a build
//!   (fact:a-planned-artifact-that-realizes-a-capability-is-counted-as-built-2026-09-28);
//! - a Verification at `passing` that VERIFIES the node;
//! - for a Component, also a live Capability at `realized`/`verified` allocated
//!   to it;
//! - for a Release, a DEPLOYED_TO edge that says it is deployed there (not one
//!   marked `planned`, which is an intended deployment).

use reflow2_core::detect::GapCandidate;
use reflow2_core::graph::DesignGraph;
use reflow2_core::nodes::{Props, edge, node};

const KEY: &str = "understated_status";

fn graph() -> DesignGraph {
    DesignGraph::open_in_memory().expect("open")
}

fn understated(g: &DesignGraph) -> Vec<GapCandidate> {
    g.detect_gaps()
        .expect("detect")
        .into_iter()
        .filter(|x| x.gap_source.as_str() == KEY)
        .collect()
}

fn about<'a>(gaps: &'a [GapCandidate], id: &str) -> Option<&'a GapCandidate> {
    gaps.iter().find(|x| x.affected_ids.iter().any(|a| a == id))
}

fn cap(g: &mut DesignGraph, id: &str, status: &str) {
    g.add_capability(id, id, "does the thing", Some(status))
        .expect("capability");
}

fn component(g: &mut DesignGraph, id: &str, status: Option<&str>) {
    let mut props = Props::new()
        .set("name", id)
        .set("purpose", "hosts the thing");
    if let Some(s) = status {
        props = props.set("status", s);
    }
    g.create_node(node::COMPONENT, id, props)
        .expect("component");
}

/// An Artifact with exactly the status given — `None` leaves it unset, which is
/// what `add_artifact` writes when nobody says.
fn artifact(g: &mut DesignGraph, id: &str, status: Option<&str>) {
    let mut props = Props::new()
        .set("name", id)
        .set("location", format!("src/{id}.rs"));
    if let Some(s) = status {
        props = props.set("status", s);
    }
    g.create_node(node::ARTIFACT, id, props).expect("artifact");
}

fn realizes(g: &mut DesignGraph, art: &str, target_type: &str, target: &str) {
    g.realizes(art, target_type, target, None, None)
        .expect("realizes");
}

fn check(g: &mut DesignGraph, id: &str, target_type: &str, target: &str, status: &str) {
    g.add_verification(id, id, Some("test"), Some("unit"), None)
        .expect("verification");
    g.verifies(id, target_type, target).expect("verifies");
    g.set_verification_status(id, status, None, None)
        .expect("verification status");
}

fn assert_below_overstatement(gap: &GapCandidate) {
    assert!(
        gap.severity < 0.70,
        "an understated status is a record fallen behind the evidence, not a claim with \
         nothing behind it — it must rank below status_contradiction's 0.70, got {}",
        gap.severity
    );
}

// ---- Capabilities ---------------------------------------------------------

#[test]
fn a_planned_capability_a_realized_artifact_builds_is_asked_about() {
    let mut g = graph();
    cap(&mut g, "cap:skill", "planned");
    artifact(&mut g, "art:skill", Some("realized"));
    realizes(&mut g, "art:skill", node::CAPABILITY, "cap:skill");

    let gaps = understated(&g);
    let hit = about(&gaps, "cap:skill").unwrap_or_else(|| {
        panic!("a built capability still reading `planned` must be asked about; got {gaps:?}")
    });
    assert_eq!(hit.affected_ids, ["cap:skill"]);
    assert_below_overstatement(hit);
    assert!(
        hit.title.contains("built") && hit.title.contains("planned"),
        "the title says what it means — built, but still says planned: {}",
        hit.title
    );
    assert!(
        hit.description.contains("set_capability_status"),
        "the finding names the call that moves the status: {}",
        hit.description
    );
    assert!(
        hit.description.contains("why not") || hit.description.contains("why it"),
        "and the other honest answer — say why not: {}",
        hit.description
    );
    assert!(
        hit.evidence.contains("art:skill"),
        "the evidence names what shows it built: {}",
        hit.evidence
    );
}

#[test]
fn a_planned_capability_a_passing_check_verifies_is_asked_about() {
    let mut g = graph();
    cap(&mut g, "cap:checked", "planned");
    check(&mut g, "ver:it", node::CAPABILITY, "cap:checked", "passing");

    let gaps = understated(&g);
    let hit = about(&gaps, "cap:checked")
        .unwrap_or_else(|| panic!("a passing check shows it works; got {gaps:?}"));
    assert!(
        hit.evidence.contains("ver:it"),
        "the evidence names the check: {}",
        hit.evidence
    );
}

/// fact:a-planned-artifact-that-realizes-a-capability-is-counted-as-built-2026-09-28:
/// an Artifact registered as PLANNED is a file nobody has written, and it must
/// never be read as proof of a build. Nor is one whose status nobody set.
#[test]
fn an_artifact_that_is_itself_planned_or_unstated_is_not_evidence_of_a_build() {
    let mut g = graph();
    cap(&mut g, "cap:a", "planned");
    artifact(&mut g, "art:planned", Some("planned"));
    realizes(&mut g, "art:planned", node::CAPABILITY, "cap:a");
    cap(&mut g, "cap:b", "planned");
    artifact(&mut g, "art:unstated", None);
    realizes(&mut g, "art:unstated", node::CAPABILITY, "cap:b");

    let gaps = understated(&g);
    assert!(
        about(&gaps, "cap:a").is_none(),
        "a planned artifact is not a build: {gaps:?}"
    );
    assert!(
        about(&gaps, "cap:b").is_none(),
        "an artifact nobody gave a status is not a build: {gaps:?}"
    );
}

#[test]
fn a_check_that_has_not_passed_shows_nothing() {
    for status in ["planned", "failing", "skipped", "blocked", "superseded"] {
        let mut g = graph();
        cap(&mut g, "cap:a", "planned");
        check(&mut g, "ver:a", node::CAPABILITY, "cap:a", status);
        assert!(
            about(&understated(&g), "cap:a").is_none(),
            "a {status} check does not show the capability works"
        );
    }
}

/// DIRECT evidence only. A file that builds the COMPONENT, or a suite that
/// checks it, is the coarser claim: measured 2026-09-30 on reflow2's own
/// design, accepting it would call 43 more planned capabilities built, each
/// merely allocated to a part some file realizes.
#[test]
fn evidence_one_hop_away_on_the_component_does_not_count() {
    let mut g = graph();
    cap(&mut g, "cap:a", "planned");
    component(&mut g, "cmp:x", Some("realized"));
    g.allocate("cap:a", "cmp:x").expect("allocate");
    artifact(&mut g, "art:x", Some("realized"));
    realizes(&mut g, "art:x", node::COMPONENT, "cmp:x");
    check(&mut g, "ver:x", node::COMPONENT, "cmp:x", "passing");

    assert!(
        about(&understated(&g), "cap:a").is_none(),
        "the component being built is not evidence that THIS function is"
    );
}

/// `in_progress` is compatible with a file that exists and a check that
/// passes for part of the work; only `planned` — "not started", and the
/// schema's default — claims less than a build.
#[test]
fn in_progress_and_later_statuses_are_not_understated() {
    for status in ["in_progress", "realized", "verified"] {
        let mut g = graph();
        cap(&mut g, "cap:a", status);
        artifact(&mut g, "art:a", Some("realized"));
        realizes(&mut g, "art:a", node::CAPABILITY, "cap:a");
        check(&mut g, "ver:a", node::CAPABILITY, "cap:a", "passing");
        assert!(
            about(&understated(&g), "cap:a").is_none(),
            "status {status} does not claim less than a build"
        );
    }
}

#[test]
fn moving_the_status_clears_it() {
    let mut g = graph();
    cap(&mut g, "cap:a", "planned");
    artifact(&mut g, "art:a", Some("realized"));
    realizes(&mut g, "art:a", node::CAPABILITY, "cap:a");
    assert!(about(&understated(&g), "cap:a").is_some());

    g.set_capability_status("cap:a", "realized")
        .expect("status");
    assert!(
        about(&understated(&g), "cap:a").is_none(),
        "the answer the finding asks for closes it"
    );
}

/// A capability an ACCEPTED Decision withdrew was built and then decided
/// against. Asking to move its status forward has no right answer.
#[test]
fn a_discontinued_capability_is_not_asked_about() {
    let mut g = graph();
    cap(&mut g, "cap:gone", "planned");
    artifact(&mut g, "art:gone", Some("realized"));
    realizes(&mut g, "art:gone", node::CAPABILITY, "cap:gone");
    g.add_decision("dec:withdraw", "Withdrawn", "Nothing replaces it.", None)
        .expect("decision");
    g.set_decision_status("dec:withdraw", "accepted")
        .expect("accept");
    g.create_edge(
        edge::OBSOLETES,
        node::DECISION,
        "dec:withdraw",
        node::CAPABILITY,
        "cap:gone",
        std::collections::HashMap::new(),
    )
    .expect("obsoletes");

    assert!(about(&understated(&g), "cap:gone").is_none());
}

/// Per capability, like its overstatement mirror: "this one is `planned` on
/// purpose" is a claim about ONE capability and must not cover the next.
#[test]
fn each_capability_gets_its_own_finding() {
    let mut g = graph();
    for id in ["cap:a", "cap:b"] {
        cap(&mut g, id, "planned");
        let art = format!("art:{id}");
        artifact(&mut g, &art, Some("realized"));
        realizes(&mut g, &art, node::CAPABILITY, id);
    }
    let gaps = understated(&g);
    let a = about(&gaps, "cap:a").expect("a");
    let b = about(&gaps, "cap:b").expect("b");
    assert_ne!(a.id, b.id, "one judgement per capability");
    assert_eq!(a.affected_ids, ["cap:a"]);
}

// ---- Components -----------------------------------------------------------

/// flo2's shape: every Component at the default `planned`, while capabilities
/// at `realized` and `verified` are allocated to them.
fn flo2_shaped() -> DesignGraph {
    let mut g = graph();
    component(&mut g, "cmp:edge-gateway", None);
    component(&mut g, "cmp:web-frontend", Some("planned"));
    component(&mut g, "cmp:not-started", Some("planned"));
    cap(&mut g, "cap:route", "verified");
    cap(&mut g, "cap:auth", "realized");
    cap(&mut g, "cap:render", "realized");
    cap(&mut g, "cap:someday", "planned");
    g.allocate("cap:route", "cmp:edge-gateway").expect("alloc");
    g.allocate("cap:auth", "cmp:edge-gateway").expect("alloc");
    g.allocate("cap:render", "cmp:web-frontend").expect("alloc");
    g.allocate("cap:someday", "cmp:not-started").expect("alloc");
    g
}

#[test]
fn a_planned_component_hosting_built_capabilities_is_asked_about() {
    let g = flo2_shaped();
    let gaps = understated(&g);
    let hit = about(&gaps, "cmp:edge-gateway").unwrap_or_else(|| {
        panic!(
            "a component whose capabilities are built must not read `planned` unasked; got {gaps:?}"
        )
    });
    assert_below_overstatement(hit);
    assert!(
        hit.title.contains("built") && hit.title.contains("planned"),
        "{}",
        hit.title
    );
    assert!(
        hit.description.contains("add_component"),
        "the finding names the call that moves a component's status: {}",
        hit.description
    );
    assert!(
        hit.evidence.contains("cap:route") && hit.evidence.contains("cap:auth"),
        "the evidence names the built capabilities it hosts: {}",
        hit.evidence
    );
    assert!(
        about(&gaps, "cmp:not-started").is_none(),
        "a component hosting only planned work IS planned: {gaps:?}"
    );
}

/// ONE finding per design, keyed on the SET of components — measured
/// 2026-09-30: this rule raised 79 on reflow2's own design, 11 on flo2's and 6
/// on a third, exactly the designs that never move `Component.status` off its
/// default, and 0 on the three designs that keep it. A per-component flood of
/// true findings is still read as noise (BL-73); one finding names the
/// practice and lists every component.
#[test]
fn the_components_are_one_finding_keyed_on_the_set() {
    let mut g = flo2_shaped();
    let gaps = understated(&g);
    let on_components: Vec<&GapCandidate> = gaps
        .iter()
        .filter(|x| x.affected_ids.iter().any(|a| a.starts_with("cmp:")))
        .collect();
    assert_eq!(
        on_components.len(),
        1,
        "one finding, not one per component: {gaps:?}"
    );
    assert_eq!(
        on_components[0].affected_ids,
        ["cmp:edge-gateway", "cmp:web-frontend"],
        "every understated component is named, in a stable order"
    );
    let first = on_components[0].id.clone();

    // Acknowledged, it stays acknowledged while the set holds…
    g.acknowledge_gap(&first, &on_components[0].affected_ids, "known, being moved")
        .expect("ack");
    assert!(
        understated(&g)
            .iter()
            .all(|x| x.affected_ids.iter().all(|a| !a.starts_with("cmp:"))),
        "acknowledge_gap works on it as on any other gap"
    );
    assert!(
        g.reviewed_gaps()
            .expect("reviewed")
            .iter()
            .any(|r| r.gap_id == first),
        "and the judgement is on the record"
    );

    // …and a component newly built behind a `planned` status asks again.
    component(&mut g, "cmp:store", Some("planned"));
    cap(&mut g, "cap:persist", "realized");
    g.allocate("cap:persist", "cmp:store").expect("alloc");
    let again = understated(&g);
    let hit = about(&again, "cmp:store")
        .unwrap_or_else(|| panic!("a new member of the set must re-ask; got {again:?}"));
    assert_ne!(hit.id, first, "a different set is a different question");
}

#[test]
fn a_component_that_keeps_its_status_is_not_asked_about() {
    for status in ["in_progress", "realized", "verified"] {
        let mut g = graph();
        component(&mut g, "cmp:x", Some(status));
        cap(&mut g, "cap:a", "realized");
        g.allocate("cap:a", "cmp:x").expect("alloc");
        assert!(
            about(&understated(&g), "cmp:x").is_none(),
            "a component at {status} does not claim less than its built function"
        );
    }
}

/// The component's OWN evidence counts too, as it does for a capability: a
/// realized file that realizes it, or a passing check on it.
#[test]
fn a_planned_component_a_realized_file_builds_is_asked_about() {
    let mut g = graph();
    component(&mut g, "cmp:x", Some("planned"));
    artifact(&mut g, "art:x", Some("realized"));
    realizes(&mut g, "art:x", node::COMPONENT, "cmp:x");
    component(&mut g, "cmp:y", Some("planned"));
    check(&mut g, "ver:y", node::COMPONENT, "cmp:y", "passing");
    component(&mut g, "cmp:z", Some("planned"));
    artifact(&mut g, "art:z", Some("planned"));
    realizes(&mut g, "art:z", node::COMPONENT, "cmp:z");

    let gaps = understated(&g);
    assert!(about(&gaps, "cmp:x").is_some(), "{gaps:?}");
    assert!(about(&gaps, "cmp:y").is_some(), "{gaps:?}");
    assert!(
        about(&gaps, "cmp:z").is_none(),
        "a planned file is not a build: {gaps:?}"
    );
}

// ---- Releases -------------------------------------------------------------

/// The release half the test in tests/detect.rs assigned to this rule:
/// `rel:v0380` was tagged, published and deployed while its status still read
/// `planned`. A DEPLOYED_TO edge that is itself `planned` is an INTENDED
/// deployment ("to be deployed there") and shows nothing.
#[test]
fn a_planned_release_that_is_deployed_is_asked_about() {
    let mut g = graph();
    g.add_environment("env:prod", "Production", None, None)
        .expect("env");
    for (rel, deployment) in [
        ("rel:out", Some("active")),
        ("rel:unmarked", None),
        ("rel:intended", Some("planned")),
    ] {
        g.add_release(rel, rel, Some("0.1.0"), None)
            .expect("release");
        g.deploy_to(rel, "env:prod", deployment).expect("deploy");
    }
    g.add_release("rel:roadmap", "rel:roadmap", Some("0.2.0"), None)
        .expect("release");

    let gaps = understated(&g);
    let out = about(&gaps, "rel:out").unwrap_or_else(|| panic!("{gaps:?}"));
    assert_below_overstatement(out);
    assert!(
        out.title.contains("planned"),
        "the title says the status it still reads: {}",
        out.title
    );
    assert!(
        about(&gaps, "rel:unmarked").is_some(),
        "unset reads as deployed there now"
    );
    assert!(
        about(&gaps, "rel:intended").is_none(),
        "a planned deployment is intent, not a deployment: {gaps:?}"
    );
    assert!(
        about(&gaps, "rel:roadmap").is_none(),
        "a planned release with no deployment IS planned"
    );
}

// ---- Acknowledgement ------------------------------------------------------

#[test]
fn acknowledge_gap_works_on_it_as_on_any_other_gap() {
    let mut g = graph();
    cap(&mut g, "cap:a", "planned");
    artifact(&mut g, "art:a", Some("realized"));
    realizes(&mut g, "art:a", node::CAPABILITY, "cap:a");
    let gaps = understated(&g);
    let hit = about(&gaps, "cap:a").expect("raised");
    g.acknowledge_gap(
        &hit.id,
        &hit.affected_ids,
        "planned on purpose: the file is a stub",
    )
    .expect("ack");

    assert!(
        about(&understated(&g), "cap:a").is_none(),
        "an acknowledged finding leaves the open list"
    );
    let reviewed = g.reviewed_gaps().expect("reviewed");
    let r = reviewed
        .iter()
        .find(|r| r.gap_id == hit.id)
        .expect("the acknowledgement is on the record");
    assert!(r.reason.contains("stub"));
}
