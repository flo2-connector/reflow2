//! Every recorded breach still standing is one reading: a gap and a line of
//! loop_status's `next`, until something closes it (Anthony 2026-10-24, "Yes,
//! one 'open breach'", dec:every-recorded-breach-still-standing-is-one-open-breach;
//! req:a-limit-a-measurement-breaks-reads-broken-and-reaches-the-loop).
//!
//! Four records say something is broken, and before this only two narrow
//! ones reached the loop (a KPP's exceeded budget, an untriaged environment-
//! rule violation):
//!
//! - an open DEFECT FINDING (`fact_type: defect`). dec:defects-are-temporal-facts
//!   (2026-08-06) said "nothing in the LOOP reads these yet ... that reader is
//!   the next increment". It was never built; two root-cause walks found it
//!   missing on 2026-09-28 and changed no code, and both were stored as defect
//!   findings themselves, so nothing showed them again. It recurred in
//!   SeattleArch (2026-10-23: the roof 3 7/16" over its height limit) and the
//!   Shop sample trial (2026-10-24: a barn 12 ft from a well against 100 ft).
//! - an EXCEEDED budget on a limit that is not a KPP;
//! - a VIOLATES edge into a Constraint;
//! - an environment-rule violation ruled `rejected` (must fix) and not
//!   resolved: the triage took it out of open_violation, and nothing tracked
//!   the fix.
//!
//! The first half of this file pins the defect finding on every subject and
//! every way it closes; the second half pins the other three kinds.
//!
//! OBSERVED FAILING FIRST. The defect-finding tests failed 4 of 6 on main
//! 61c05ad74 (the two that assert a closed finding stays out passed
//! vacuously). The renamed reading and the three other kinds failed on
//! 66a8ea279, where only open defect findings were read.

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
        .filter(|s| s == "open_breach" || s == "defect_overtaken_by_change")
        .collect()
}

fn open_finding_line(g: &DesignGraph) -> Option<String> {
    g.loop_status()
        .expect("loop")
        .next
        .into_iter()
        .find(|l| l.contains("open breach"))
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
                .is_some_and(|l| l.starts_with("1 open breach")),
            "an open defect finding on a {ty} must be a line of `next`, counted; got {:?}",
            g.loop_status().expect("loop").next
        );
        assert_eq!(
            finding_gaps(&g),
            vec!["open_breach".to_string()],
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
    assert_eq!(finding_gaps(&g), vec!["open_breach".to_string()]);
    assert!(open_finding_line(&g).is_some());
}

// ─── The other three kinds of breach ───────────────────────────────────────

fn breach_gaps_on(g: &DesignGraph, id: &str) -> Vec<String> {
    g.detect_gaps()
        .expect("gaps")
        .into_iter()
        .filter(|c| c.affected_ids.iter().any(|a| a == id))
        .map(|c| {
            serde_json::to_value(c.gap_source)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default()
        })
        .filter(|s| s == "open_breach" || s == "kpp_breached" || s == "open_violation")
        .collect()
}

fn coverage(category: Option<&str>, covered: f64) -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_project("proj:shop", "Shop sample").expect("project");
    g.add_constraint(
        "con:lot-coverage",
        "Buildings cover at most 60 percent of the lot",
        "BCC 11.07.080(c).",
        category,
        Some("covered_area_sq_ft"),
        Some(74284.9),
        None,
        Some("maximum"),
    )
    .expect("constraint");
    g.add_component("cmp:buildings", "Buildings", "every roof", None)
        .expect("component");
    g.constrains(
        "con:lot-coverage",
        node::COMPONENT,
        "cmp:buildings",
        Some(covered),
        Some("measured"),
        Some("2026-10-24"),
        None,
    )
    .expect("constrains");
    g
}

#[test]
fn an_exceeded_limit_that_is_not_a_kpp_is_an_open_breach_until_it_fits() {
    let mut g = coverage(Some("regulatory"), 80000.0);
    assert_eq!(
        breach_gaps_on(&g, "con:lot-coverage"),
        vec!["open_breach".to_string()]
    );
    assert!(
        open_finding_line(&g).is_some_and(|l| l.starts_with("1 open breach")),
        "{:?}",
        g.loop_status().expect("loop").next
    );
    // The design changes and the limit fits: nothing is left to close.
    g.constrains(
        "con:lot-coverage",
        node::COMPONENT,
        "cmp:buildings",
        Some(8368.0),
        Some("measured"),
        Some("2026-10-25"),
        None,
    )
    .expect("re-measured");
    assert!(breach_gaps_on(&g, "con:lot-coverage").is_empty());
    assert_eq!(open_finding_line(&g), None);
}

#[test]
fn an_exceeded_kpp_keeps_its_own_breach_and_is_not_asked_twice() {
    let g = coverage(Some("kpp"), 80000.0);
    assert_eq!(
        breach_gaps_on(&g, "con:lot-coverage"),
        vec!["kpp_breached".to_string()]
    );
}

#[test]
fn a_ruling_on_an_exceeded_limit_takes_it_out_of_the_loop() {
    let mut g = coverage(None, 80000.0);
    let gap = g
        .detect_gaps()
        .expect("gaps")
        .into_iter()
        .find(|c| {
            c.affected_ids.iter().any(|a| a == "con:lot-coverage")
                && serde_json::to_value(c.gap_source)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    == Some("open_breach".to_string())
        })
        .expect("the breach's gap");
    g.acknowledge_gap(&gap.id, &gap.affected_ids, "Variance granted by the county")
        .expect("ruling");
    assert!(breach_gaps_on(&g, "con:lot-coverage").is_empty());
    assert_eq!(open_finding_line(&g), None);
}

#[test]
fn a_violates_edge_into_a_constraint_is_an_open_breach_until_it_is_removed() {
    let mut g = coverage(Some("regulatory"), 8368.0);
    g.create_edge(
        edge::VIOLATES,
        node::COMPONENT,
        "cmp:buildings",
        node::CONSTRAINT,
        "con:lot-coverage",
        Props::new().set("evidence", "the carport was left out of the take-off"),
    )
    .expect("violates");
    assert_eq!(
        breach_gaps_on(&g, "con:lot-coverage"),
        vec!["open_breach".to_string()]
    );
    assert!(open_finding_line(&g).is_some());
    g.delete_edge(edge::VIOLATES, "cmp:buildings", "con:lot-coverage")
        .expect("removed");
    assert!(breach_gaps_on(&g, "con:lot-coverage").is_empty());
    assert_eq!(open_finding_line(&g), None);
}

fn well_rule() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_project("proj:shop", "Shop sample").expect("project");
    g.add_component("cmp:barn-lean-to", "Barn lean-to", "a part", None)
        .expect("component");
    g.add_environment_rule(
        "erule:well-setback",
        "Livestock barns stay 100 ft from a well",
        "WAC 173-160-171(3)(b)(v).",
        Some("regulatory"),
        Some("Washington Department of Ecology"),
        None,
        None,
        None,
        None,
    )
    .expect("rule");
    g.violates_rule(
        node::COMPONENT,
        "cmp:barn-lean-to",
        "erule:well-setback",
        Some("check"),
        Some("critical"),
        Some("12 ft from the well head"),
    )
    .expect("flagged");
    g
}

#[test]
fn a_rule_violation_ruled_must_fix_stays_open_until_it_is_resolved() {
    let mut g = well_rule();
    assert_eq!(
        breach_gaps_on(&g, "erule:well-setback"),
        vec!["open_violation".to_string()],
        "untriaged, it is asked about as a violation, once"
    );
    g.set_violation_status(
        "cmp:barn-lean-to",
        "erule:well-setback",
        "rejected",
        Some("must move"),
    )
    .expect("ruled must fix");
    assert_eq!(
        breach_gaps_on(&g, "erule:well-setback"),
        vec!["open_breach".to_string()],
        "ruled must-fix and not yet fixed, it is still broken"
    );
    assert!(open_finding_line(&g).is_some());
    // Fixed: the edge records when, and it is kept for audit.
    g.create_edge(
        edge::VIOLATES_RULE,
        node::COMPONENT,
        "cmp:barn-lean-to",
        node::ENVIRONMENT_RULE,
        "erule:well-setback",
        Props::new()
            .set("status", "rejected")
            .set("resolved_at", "2026-10-24")
            .set("evidence", "12 ft from the well head"),
    )
    .expect("resolved");
    assert!(breach_gaps_on(&g, "erule:well-setback").is_empty());
    assert_eq!(open_finding_line(&g), None);
}

#[test]
fn a_waived_rule_violation_is_not_a_breach() {
    let mut g = well_rule();
    g.set_violation_status(
        "cmp:barn-lean-to",
        "erule:well-setback",
        "confirmed",
        Some("variance"),
    )
    .expect("waived");
    assert!(breach_gaps_on(&g, "erule:well-setback").is_empty());
    assert_eq!(open_finding_line(&g), None);
}
