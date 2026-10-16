//! A conclusion shows the axioms (the chosen principles and givens) it
//! ultimately rests on, and changing one shows what falls
//! (req:a-conclusion-shows-the-axioms-it-rests-on, Anthony 2026-10-14:
//! "let's keep (d) and (e)").
//!
//! An AXIOM is a TERMINAL PREMISE — a premise that is not itself concluded by
//! any Inference. Walking backward through the inference chain (the node's
//! concluding Inferences → their premises → if a premise is itself concluded,
//! recurse) finds the set of axioms the conclusion ultimately rests on.
//!
//! # The honest limit
//!
//! The foundations tell what a conclusion follows FROM. They never say the
//! axioms are right. In an abstract design they are values, and may be
//! contested by design.
//!
//! # The guard (con:no-gap-is-raised-for-a-conclusion-that-has-no-inference)
//!
//! A node not concluded by any Inference has no foundations — it IS a terminal
//! node. No gap, finding, or count fires.

use reflow2_core::DesignGraph;
use reflow2_core::inference::Truth;
use reflow2_core::nodes::Props;

const INFERENCE: &str = "Inference";
const PREMISE_OF: &str = "PREMISE_OF";
const CONCLUDES: &str = "CONCLUDES";

fn graph() -> DesignGraph {
    DesignGraph::open_in_memory().expect("in-memory graph")
}

fn type_of(g: &DesignGraph, id: &str) -> String {
    let held = g.node_types_holding(id).expect("types holding");
    assert_eq!(held.len(), 1, "{id} held by exactly one type: {held:?}");
    held[0].clone()
}

fn inference(
    g: &mut DesignGraph,
    id: &str,
    rule: Option<&str>,
    k: Option<i64>,
    premises: &[&str],
    conclusion: &str,
) {
    let mut props = Props::new().set("name", format!("step {id}"));
    if let Some(r) = rule {
        props = props.set("rule", r);
    }
    if let Some(k) = k {
        props = props.set("k", k);
    }
    g.create_node(INFERENCE, id, props)
        .unwrap_or_else(|e| panic!("Inference {id}: {e}"));
    for p in premises {
        let ty = type_of(g, p);
        g.create_edge(PREMISE_OF, &ty, p, INFERENCE, id, Props::new())
            .unwrap_or_else(|e| panic!("{p} PREMISE_OF {id}: {e}"));
    }
    let ty = type_of(g, conclusion);
    g.create_edge(CONCLUDES, INFERENCE, id, &ty, conclusion, Props::new())
        .unwrap_or_else(|e| panic!("{id} CONCLUDES {conclusion}: {e}"));
}

fn requirement(g: &mut DesignGraph, id: &str) {
    g.add_requirement(id, &format!("claim {id}"), "A claim.")
        .unwrap();
}

fn constraint(g: &mut DesignGraph, id: &str) {
    g.create_node(
        "Constraint",
        id,
        Props::new()
            .set("name", format!("principle {id}"))
            .set("category", "technical")
            .set("statement", format!("A chosen principle: {id}.")),
    )
    .unwrap();
}

fn decision(g: &mut DesignGraph, id: &str) {
    g.create_node(
        "Decision",
        id,
        Props::new()
            .set("name", format!("choice {id}"))
            .set("decision", "Recorded.")
            .set("status", "accepted"),
    )
    .unwrap();
}

// ─── Three-layer chain ──────────────────────────────────────────────────────

#[test]
fn a_three_layer_chain_finds_the_axioms_not_the_intermediates() {
    // Axiom A1 and axiom A2 → Inference I1 → lemma L → Inference I2 → theorem T
    // foundations_of(T) = {A1, A2}, NOT L (intermediate, not terminal).
    let mut g = graph();

    // Axioms: constraints accepted by the owner.
    constraint(&mut g, "con:axiom-a1");
    constraint(&mut g, "con:axiom-a2");

    // Lemma: a requirement concluded from the axioms.
    requirement(&mut g, "req:lemma-l");
    inference(
        &mut g,
        "inf:step1",
        Some("all"),
        None,
        &["con:axiom-a1", "con:axiom-a2"],
        "req:lemma-l",
    );

    // Theorem: a requirement concluded from the lemma.
    requirement(&mut g, "req:theorem-t");
    inference(
        &mut g,
        "inf:step2",
        Some("all"),
        None,
        &["req:lemma-l"],
        "req:theorem-t",
    );

    let foundations = g
        .foundations_of("req:theorem-t")
        .expect("foundations_of works");
    let ids: std::collections::BTreeSet<&str> = foundations.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        ids,
        ["con:axiom-a1", "con:axiom-a2"]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        "foundations should be A1 and A2, not intermediate L"
    );

    // The intermediate lemma should NOT appear in the foundations.
    assert!(
        !ids.contains("req:lemma-l"),
        "lemma is intermediate and must not appear in foundations"
    );
}

// ─── Alternative derivations ────────────────────────────────────────────────

#[test]
fn alternative_derivations_give_the_union_of_axioms() {
    // T concluded by I1 (from A1) and by I2 (from A2, A3).
    // Foundations of T = {A1, A2, A3} (the union).
    let mut g = graph();

    constraint(&mut g, "con:a1");
    constraint(&mut g, "con:a2");
    constraint(&mut g, "con:a3");

    requirement(&mut g, "req:theorem");

    inference(
        &mut g,
        "inf:path1",
        Some("all"),
        None,
        &["con:a1"],
        "req:theorem",
    );
    inference(
        &mut g,
        "inf:path2",
        Some("all"),
        None,
        &["con:a2", "con:a3"],
        "req:theorem",
    );

    let foundations = g.foundations_of("req:theorem").expect("foundations_of");
    let ids: std::collections::BTreeSet<&str> = foundations.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        ids,
        ["con:a1", "con:a2", "con:a3"]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        "foundations = union across alternative derivations"
    );
}

// ─── No inference means no foundations ───────────────────────────────────────

#[test]
fn a_node_with_no_inference_has_no_foundations() {
    let mut g = graph();
    requirement(&mut g, "req:standalone");

    let foundations = g.foundations_of("req:standalone").expect("foundations_of");
    assert!(
        foundations.is_empty(),
        "a node with no concluding Inference has no foundations (it IS terminal)"
    );
}

// ─── SHOWN WHEN: the voting rule ────────────────────────────────────────────

#[test]
fn a_voting_rules_conclusion_lists_the_principles_and_dropping_one_breaks_it() {
    // A small abstract design: a voting rule.
    // Principles: "one person one vote", "majority decides".
    // Conclusion: "the election result is legitimate".
    let mut g = graph();

    // Axioms: chosen principles.
    decision(&mut g, "dec:one-person-one-vote");
    decision(&mut g, "dec:majority-decides");

    // Conclusion.
    requirement(&mut g, "req:legitimate-result");
    g.set_requirement_status("req:legitimate-result", "met")
        .unwrap();

    inference(
        &mut g,
        "inf:legitimacy",
        Some("all"),
        None,
        &["dec:one-person-one-vote", "dec:majority-decides"],
        "req:legitimate-result",
    );

    // The foundations list exactly the chosen principles.
    let foundations = g
        .foundations_of("req:legitimate-result")
        .expect("foundations");
    let ids: std::collections::BTreeSet<&str> = foundations.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        ids,
        ["dec:majority-decides", "dec:one-person-one-vote"]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
    );

    // Every foundation's truth: both decisions are accepted → holds.
    for f in &foundations {
        assert_eq!(
            f.truth,
            Truth::Holds,
            "{} should hold (accepted decision)",
            f.id
        );
    }

    // Dropping one principle: change its status to rejected.
    g.create_node(
        "Decision",
        "dec:majority-decides",
        Props::new()
            .set("name", "majority decides")
            .set("decision", "Recorded.")
            .set("status", "rejected"),
    )
    .unwrap();

    // The change walk from the dropped principle reports the conclusion broken.
    let walk = g.inference_walk(&["dec:majority-decides"]).expect("walk");
    assert!(
        !walk.is_empty(),
        "dropping a principle must show what falls"
    );
    let broken: Vec<&str> = walk
        .iter()
        .filter(|r| r.impact == "broken")
        .filter_map(|r| r.conclusion.as_deref())
        .collect();
    assert!(
        broken.contains(&"req:legitimate-result"),
        "the conclusion should be broken when its axiom is dropped"
    );
}

// ─── The derived relation `foundations` ──────────────────────────────────────

#[test]
fn the_derived_relation_foundations_lists_the_designs_axioms() {
    let mut g = graph();

    constraint(&mut g, "con:principle-x");
    constraint(&mut g, "con:principle-y");
    requirement(&mut g, "req:conclusion");

    inference(
        &mut g,
        "inf:derive",
        Some("all"),
        None,
        &["con:principle-x", "con:principle-y"],
        "req:conclusion",
    );

    // The derived relation: nodes that are foundations of at least one conclusion.
    let axioms = g.design_foundations().expect("design_foundations");
    let ids: std::collections::BTreeSet<&str> = axioms.iter().map(|s| s.as_str()).collect();
    assert!(ids.contains("con:principle-x"));
    assert!(ids.contains("con:principle-y"));
    assert!(
        !ids.contains("req:conclusion"),
        "the conclusion itself is not an axiom"
    );
}

// ─── The honest limit ───────────────────────────────────────────────────────

#[test]
fn the_honest_limit_text_is_present_on_foundations() {
    let mut g = graph();
    constraint(&mut g, "con:value");
    requirement(&mut g, "req:derived");

    inference(
        &mut g,
        "inf:step",
        Some("all"),
        None,
        &["con:value"],
        "req:derived",
    );

    let foundations = g.foundations_of("req:derived").expect("foundations");
    assert!(!foundations.is_empty());

    // The honest limit comes from the FoundationsReading, which includes it.
    let reading = g
        .foundations_reading("req:derived")
        .expect("foundations_reading");
    assert!(
        reading.is_some(),
        "a concluded node has a foundations reading"
    );
    let reading = reading.unwrap();
    assert!(
        reading.honest_limit.contains("follows from"),
        "the honest limit must say what the conclusion follows from: {:?}",
        reading.honest_limit
    );
    assert!(
        reading.honest_limit.contains("never"),
        "the honest limit must say the axioms are never claimed right: {:?}",
        reading.honest_limit
    );
}
