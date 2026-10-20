//! A PREMISE IN ANOTHER DESIGN READS AS IT READS THERE
//! (`req:reasoning-crosses-designs-within-a-hub`, parts 1 and 3;
//! `reflow2_core::far`). The real path through a registry and a hub is
//! `reflow2-mcp`'s `reasoning_crosses_designs_within_a_hub.rs`; this holds the
//! core's half on two in-memory designs: the far reading is taken from the far
//! design with `read_for_another_design` and handed to the near one's
//! evaluation with `far::scoped`, as the server does per call.

use std::sync::Arc;

use reflow2_core::DesignGraph;
use reflow2_core::far::{self, FarReading, FarReadings, FarState};
use reflow2_core::inference::{InferenceSpec, Truth};
use reflow2_core::relate::RelationLink;

/// F: an axiom taken as given, and a theorem one step concludes from it.
fn far_design(confirmed: bool) -> DesignGraph {
    let mut f = DesignGraph::open_in_memory_as("fermat").unwrap();
    f.add_requirement("req:axiom", "Axiom", "Fermat's little theorem.")
        .unwrap();
    f.upsert_node(
        "Requirement",
        "req:axiom",
        reflow2_core::nodes::Props::new().set("taken_as_given", true),
    )
    .unwrap();
    f.add_requirement(
        "req:theorem",
        "Theorem",
        "A failed test proves n composite.",
    )
    .unwrap();
    f.add_inference(&InferenceSpec {
        id: "inf:theorem".into(),
        rule: "all".into(),
        premises: vec!["req:axiom".into()],
        conclusion: "req:theorem".into(),
        confirmed_by: confirmed.then(|| "who:checker".into()),
        confirmed_how: confirmed.then(|| "person".into()),
        confirmed_at: confirmed.then(|| "2026-10-19T00:00:00Z".into()),
        ..Default::default()
    })
    .unwrap();
    f
}

/// C: a claim one step concludes from a reference to F's `far_node`.
fn near_design(far_node: &str, confirmed: bool) -> (DesignGraph, String) {
    let mut c = DesignGraph::open_in_memory_as("calc").unwrap();
    c.add_requirement("req:claim", "Claim", "A false answer is proven.")
        .unwrap();
    let xref = c
        .ensure_design_reference("fermat", far_node, Some("Requirement"), None)
        .unwrap();
    c.review_relations(
        "Requirement",
        "req:claim",
        &[RelationLink {
            relation: "DEPENDS_ON".into(),
            other_type: "Resource".into(),
            other_id: xref.clone(),
            evidence: "cites the theorem proved there".into(),
            incoming: false,
            properties: Default::default(),
        }],
        None,
    )
    .unwrap();
    c.add_inference(&InferenceSpec {
        id: "inf:claim".into(),
        rule: "all".into(),
        premises: vec![xref.clone()],
        conclusion: "req:claim".into(),
        confirmed_by: confirmed.then(|| "who:checker".into()),
        confirmed_how: confirmed.then(|| "person".into()),
        confirmed_at: confirmed.then(|| "2026-10-19T00:00:00Z".into()),
        ..Default::default()
    })
    .unwrap();
    (c, xref)
}

fn readings(xref: &str, reading: FarReading) -> Option<Arc<FarReadings>> {
    let mut map = FarReadings::new();
    map.insert(xref.to_string(), reading);
    Some(Arc::new(map))
}

fn status(g: &DesignGraph, id: &str) -> String {
    g.proof_reading(id)
        .unwrap()
        .map(|p| p.status.as_str().to_string())
        .unwrap_or_default()
}

#[test]
fn the_references_a_design_reasons_from_are_listed_for_the_server_to_read() {
    let (c, xref) = near_design("req:theorem", false);
    assert_eq!(
        c.reasoning_references().unwrap(),
        vec![(xref, "fermat".to_string(), "req:theorem".to_string())]
    );
}

#[test]
fn with_nothing_handed_in_a_reference_reads_unreachable_never_holding() {
    let (c, xref) = near_design("req:theorem", true);
    let (truth, why) = c.premise_truth(&xref).unwrap();
    assert_eq!(truth, Truth::Unknown);
    assert!(why.contains("unreachable"), "{why}");
    assert_eq!(status(&c, "req:claim"), "unproven");
}

#[test]
fn a_far_theorem_shown_there_holds_here_and_the_claim_is_shown() {
    let f = far_design(false);
    let (c, xref) = near_design("req:theorem", true);
    let reading = f.read_for_another_design("req:theorem").unwrap();
    assert_eq!(reading.state, FarState::Read);
    assert_eq!(reading.proof_status.as_deref(), Some("shown"));
    far::scoped(readings(&xref, reading), || {
        let (truth, why) = c.premise_truth(&xref).unwrap();
        assert_eq!(truth, Truth::Holds, "{why}");
        assert!(why.contains("fermat") && why.contains("shown"), "{why}");
        // Every step HERE is confirmed, but the far result is only shown.
        assert_eq!(status(&c, "req:claim"), "shown");
    });
}

#[test]
fn a_far_theorem_proven_there_lets_a_confirmed_line_here_be_proven() {
    let f = far_design(true);
    let (c, xref) = near_design("req:theorem", true);
    let reading = f.read_for_another_design("req:theorem").unwrap();
    assert_eq!(reading.proof_status.as_deref(), Some("proven"));
    far::scoped(readings(&xref, reading), || {
        assert_eq!(status(&c, "req:claim"), "proven");
    });
}

#[test]
fn what_the_far_result_holds_only_under_makes_the_claim_conditional_here() {
    let (c, xref) = near_design("req:theorem", true);
    let reading = FarReading {
        design: "fermat".into(),
        node_id: "req:theorem".into(),
        state: FarState::Read,
        truth: Truth::Holds,
        why: "it holds only if the unverified citation req:flt says what it is recalled to say"
            .into(),
        proof_status: Some("conditional".into()),
        holds_only_if: vec!["fermat:req:flt".into()],
    };
    far::scoped(readings(&xref, reading), || {
        let p = c.proof_reading("req:claim").unwrap().unwrap();
        assert_eq!(p.status.as_str(), "conditional", "{}", p.why);
        assert!(p.why.contains("fermat:req:flt"), "{}", p.why);
    });
}

#[test]
fn a_far_node_the_far_design_no_longer_holds_reads_stale() {
    let f = far_design(true);
    let (c, xref) = near_design("req:gone", true);
    let reading = f.read_for_another_design("req:gone").unwrap();
    assert_eq!(reading.state, FarState::Stale);
    far::scoped(readings(&xref, reading), || {
        let (truth, why) = c.premise_truth(&xref).unwrap();
        assert_eq!(truth, Truth::Unknown);
        assert!(why.contains("stale"), "{why}");
        assert_eq!(status(&c, "req:claim"), "unproven");
    });
}

#[test]
fn a_far_node_that_fails_there_fails_here() {
    let (c, xref) = near_design("req:theorem", true);
    let reading = FarReading {
        design: "fermat".into(),
        node_id: "req:theorem".into(),
        state: FarState::Read,
        truth: Truth::Fails,
        why: "refuted by a counterexample".into(),
        proof_status: Some("unproven".into()),
        holds_only_if: vec![],
    };
    far::scoped(readings(&xref, reading), || {
        assert_eq!(c.premise_truth(&xref).unwrap().0, Truth::Fails);
    });
}

#[test]
fn a_walk_continues_from_the_references_to_what_it_reached() {
    let (c, xref) = near_design("req:theorem", true);
    let reached = ["req:axiom".to_string(), "req:theorem".to_string()]
        .into_iter()
        .collect();
    assert_eq!(c.references_into("fermat", &reached).unwrap(), vec![xref]);
    assert!(
        c.references_into("another", &reached).unwrap().is_empty(),
        "a reference into another design is not entered from this one"
    );
}
