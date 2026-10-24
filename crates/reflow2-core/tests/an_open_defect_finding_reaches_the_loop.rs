//! An open defect finding reaches the loop until a change or a ruling closes it
//! (req:a-limit-a-measurement-breaks-reads-broken-and-reaches-the-loop).
//!
//! THE CLASS, NOT THE INSTANCE. A defect found by use or by a measurement is a
//! TemporalFact with `fact_type: defect` (dec:defects-are-temporal-facts,
//! 2026-08-06), and that decision said in its own text that "nothing in the
//! LOOP reads these yet ... that reader is the next increment". It was never
//! built. Two root-cause walks found the absence again on 2026-09-28 and
//! changed no code, and both were recorded as defect findings themselves, so
//! the loop never showed them either. Then it recurred twice:
//!
//! - SeattleArch, 2026-10-23: the roof measured 3 7/16" over the height
//!   maximum and all 24 unit areas off the area table, each recorded as a
//!   defect finding, and loop_status, detect_gaps and detect_defects said
//!   nothing about any of them.
//! - the Shop sample trial, 2026-10-24: the barn lean-to measured 12 ft from
//!   the well against a 100 ft limit, recorded as a defect finding, reached no
//!   gap and no line of `next`.
//!
//! So these tests pin the reader, on every kind of subject, and every way a
//! finding stops being open: a `valid_to`, an `INVALIDATES` from the change or
//! the decision that fixed it, and a ruling on its gap. A decision that merely
//! RESTS on the finding (the Shop sample's fix by a choice) does not close it.
//!
//! OBSERVED FAILING FIRST on main 61c05ad74 (v0.91.0 + #794): every test that
//! asserts the line or the gap failed there, and the ones asserting a closed
//! finding stays out passed vacuously, as they must.

use reflow2_core::graph::DesignGraph;
use reflow2_core::nodes::{Props, edge, node};

const FACT: &str = "fact:barn-lean-to-12-ft-from-the-well";

/// The Shop sample's case, on a subject of the given type.
fn design_with_open_finding(subject_type: &str, subject_id: &str) -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_project("proj:shop", "Shop sample").expect("project");
    match subject_type {
        t if t == node::COMPONENT => {
            g.add_component(subject_id, "Barn lean-to", "the back 12 ft of the T", None)
                .expect("component");
        }
        t if t == node::CONSTRAINT => {
            g.add_constraint(
                subject_id,
                "Livestock barns stay at least 100 ft from the well",
                "WAC 173-160-171(3)(b)(v), applied around the existing well.",
                Some("regulatory"),
                Some("distance_to_well_ft"),
                Some(100.0),
                None,
                Some("minimum"),
            )
            .expect("constraint");
        }
        _ => {}
    }
    g.upsert_node(
        node::TEMPORAL_FACT,
        FACT,
        Props::new()
            .set(
                "name",
                "LIMIT BROKEN: the barn lean-to sits about 12 ft from the well head; the limit is 100 ft",
            )
            .set("statement", "Measured in phase-b-well-and-septic-distances.calc.json.")
            .set("fact_type", "defect")
            .set("basis", "measured")
            .set("subject_id", subject_id)
            .set("valid_from", "2026-10-24"),
    )
    .expect("finding");
    g
}

/// The gaps that ask about the finding AS A DEFECT. A finding can also be a
/// premise of a line of reasoning, and the reasoning's own gaps (an uncited
/// premise, an unconfirmed step) are about that line, not about whether the
/// thing is still broken, so they are left out here.
fn finding_gaps(g: &DesignGraph) -> Vec<String> {
    g.detect_gaps()
        .expect("gaps")
        .into_iter()
        .filter(|c| c.affected_ids.iter().any(|a| a == FACT))
        .map(|c| {
            serde_json::to_value(c.gap_source)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default()
        })
        .filter(|s| s == "open_defect_finding" || s == "defect_overtaken_by_change")
        .collect()
}

fn open_finding_line(g: &DesignGraph) -> Option<String> {
    g.loop_status()
        .expect("loop")
        .next
        .into_iter()
        .find(|l| l.contains("open defect finding"))
}

#[test]
fn an_open_defect_finding_on_any_subject_is_a_line_of_next_and_a_gap() {
    for (ty, id) in [
        (node::COMPONENT, "cmp:barn-lean-to"),
        (
            node::CONSTRAINT,
            "con:limit-contamination-sources-100-ft-from-the-well",
        ),
        (node::PROJECT, "proj:shop"),
    ] {
        let g = design_with_open_finding(ty, id);
        let line = open_finding_line(&g);
        assert!(
            line.as_deref()
                .is_some_and(|l| l.starts_with("1 open defect finding")),
            "an open defect finding on a {ty} must be a line of `next`, counted; got {:?}",
            g.loop_status().expect("loop").next
        );
        assert_eq!(
            finding_gaps(&g),
            vec!["open_defect_finding".to_string()],
            "an open defect finding on a {ty} must reach detect_gaps, once"
        );
        assert!(
            !g.loop_status().expect("loop").clean,
            "a design holding an open defect finding is not clean"
        );
    }
}

#[test]
fn a_finding_that_is_not_a_defect_is_not_counted() {
    let mut g = design_with_open_finding(node::COMPONENT, "cmp:barn-lean-to");
    g.upsert_node(
        node::TEMPORAL_FACT,
        FACT,
        Props::new().set("fact_type", "finding"),
    )
    .expect("retype");
    assert_eq!(open_finding_line(&g), None);
    assert!(finding_gaps(&g).is_empty());
}

#[test]
fn a_valid_to_closes_it() {
    let mut g = design_with_open_finding(node::COMPONENT, "cmp:barn-lean-to");
    g.upsert_node(
        node::TEMPORAL_FACT,
        FACT,
        Props::new().set("valid_to", "2026-10-24"),
    )
    .expect("close");
    assert_eq!(open_finding_line(&g), None);
    assert!(finding_gaps(&g).is_empty());
}

#[test]
fn a_decision_resting_on_it_does_not_close_it_and_invalidates_from_that_decision_does() {
    let mut g = design_with_open_finding(node::COMPONENT, "cmp:barn-lean-to");
    // The Shop sample's fix by a choice: the finding is a premise of the step
    // that concludes the decision moving the stables 100 ft away.
    g.upsert_node(
        node::DECISION,
        "dec:stables-move-100-ft-from-the-well",
        Props::new()
            .set(
                "name",
                "The stables and coop move to a shelter 100 ft from the well",
            )
            .set("decision", "Move them.")
            .set("status", "accepted"),
    )
    .expect("decision");
    g.upsert_node(
        node::INFERENCE,
        "inf:why-the-stables-move",
        Props::new().set("name", "Why the stables move"),
    )
    .expect("inference");
    g.create_edge(
        edge::PREMISE_OF,
        node::TEMPORAL_FACT,
        FACT,
        node::INFERENCE,
        "inf:why-the-stables-move",
        Props::new(),
    )
    .expect("premise");
    g.create_edge(
        edge::CONCLUDES,
        node::INFERENCE,
        "inf:why-the-stables-move",
        node::DECISION,
        "dec:stables-move-100-ft-from-the-well",
        Props::new(),
    )
    .expect("concludes");
    assert!(
        open_finding_line(&g).is_some(),
        "a decision that rests on the finding answered it; it did not close it"
    );

    g.invalidates(
        node::DECISION,
        "dec:stables-move-100-ft-from-the-well",
        node::TEMPORAL_FACT,
        FACT,
        Some("the stables and coop no longer go in the lean-to"),
        Some("2026-10-24"),
    )
    .expect("invalidates");
    assert_eq!(open_finding_line(&g), None);
    assert!(finding_gaps(&g).is_empty(), "{:?}", finding_gaps(&g));
}

#[test]
fn a_ruling_on_its_gap_takes_it_out_of_the_loop() {
    let mut g = design_with_open_finding(node::COMPONENT, "cmp:barn-lean-to");
    let gap = g
        .detect_gaps()
        .expect("gaps")
        .into_iter()
        .find(|c| c.affected_ids.iter().any(|a| a == FACT))
        .expect("the finding's gap, before any ruling");
    g.acknowledge_gap(
        &gap.id,
        &gap.affected_ids,
        "Ruled: the sample keeps this as a record of Alex's plan; the stables are moved",
    )
    .expect("ruling");
    assert_eq!(open_finding_line(&g), None);
    assert!(finding_gaps(&g).is_empty());
}

#[test]
fn a_finding_the_overtaken_question_asks_about_is_asked_once_and_stays_in_the_loop() {
    let mut g = design_with_open_finding(node::COMPONENT, "cmp:barn-lean-to");
    // A repair on the subject, the same day: `defect_overtaken_by_change`
    // asks whether it was the fix. That question is enough; a second gap
    // beside it would count one fault twice.
    g.create_node(
        node::CHANGE_EVENT,
        "chg:lean-to-replanned",
        Props::new()
            .set("name", "The lean-to is replanned")
            .set("change_type", "defect_fix")
            .set("subject", "system")
            .set("detected_at", "2026-10-24"),
    )
    .expect("change");
    g.create_edge(
        edge::CHANGED,
        node::CHANGE_EVENT,
        "chg:lean-to-replanned",
        node::COMPONENT,
        "cmp:barn-lean-to",
        Props::new(),
    )
    .expect("changed");
    assert_eq!(
        finding_gaps(&g),
        vec!["defect_overtaken_by_change".to_string()],
        "one fault, one question"
    );
    assert!(
        open_finding_line(&g).is_some(),
        "asked about or not, it is open until something closes it"
    );

    // Judged: that change did not fix it. The finding is still open, so its
    // own gap asks again.
    let asked = g
        .detect_gaps()
        .expect("gaps")
        .into_iter()
        .find(|c| c.affected_ids.iter().any(|a| a == FACT))
        .expect("the overtaken question");
    g.acknowledge_gap(
        &asked.id,
        &asked.affected_ids,
        "That replanning moved the door, not the stables",
    )
    .expect("judged");
    assert_eq!(finding_gaps(&g), vec!["open_defect_finding".to_string()]);
    assert!(open_finding_line(&g).is_some());
}
