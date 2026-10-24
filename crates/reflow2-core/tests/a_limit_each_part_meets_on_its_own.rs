//! A limit each part meets on its own (`composition: each`): every stated
//! contribution is judged alone, the verdict reads the worst part and names
//! it, and an unstated or empty set reads incomplete
//! (dec:a-limit-each-part-meets-on-its-own-is-composition-each, Anthony
//! 2026-10-24).
//!
//! THE CASE. The Shop sample trial (2026-10-24) recorded 23 site limits:
//! setbacks, a 100 ft well clearance, a right-of-way that nothing may enter.
//! A clearance is not a budget. Its parts do not add: two parts 6 ft from a
//! line do not make 12 ft of setback. With only `sum` and `path` to choose
//! from, the barn lean-to's measured 12 ft from the well, recorded against the
//! 100 ft limit, rolled up to 114.36 and read WITHIN; a minimum with no
//! contribution yet read EXCEEDED; and the agent recorded only the nearest
//! part's distance to get any honest number at all.
//!
//! OBSERVED FAILING FIRST on 66a8ea279 (PR #795 before this change): every
//! test here fails at `set_constraint_composition(.., "each")`, which refused
//! the value.

use reflow2_core::budget::BudgetVerdict;
use reflow2_core::graph::DesignGraph;
use reflow2_core::nodes::{Props, edge, node};

const CON: &str = "con:limit-contamination-sources-100-ft-from-the-well";

fn clearance(direction: &str, limit: f64, category: Option<&str>) -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_project("proj:shop", "Shop sample").expect("project");
    g.add_constraint(
        CON,
        "Livestock barns, manure and compost stay at least 100 ft from the well",
        "WAC 173-160-171(3)(b)(v), kept around the existing well.",
        category,
        Some("distance_to_well_ft"),
        Some(limit),
        None,
        Some(direction),
    )
    .expect("constraint");
    g.set_constraint_composition(CON, "each")
        .expect("`each` is a composition: a limit each part meets on its own");
    g
}

fn part(g: &mut DesignGraph, id: &str, ft: Option<f64>) {
    g.add_component(id, id, "a part", None).expect("component");
    g.constrains(
        CON,
        node::COMPONENT,
        id,
        ft,
        Some("measured"),
        Some("2026-10-24"),
        None,
    )
    .expect("constrains");
}

#[test]
fn the_barn_breaks_the_well_clearance_and_is_named() {
    let mut g = clearance("minimum", 100.0, Some("regulatory"));
    part(&mut g, "cmp:barn-lean-to", Some(12.0));
    part(&mut g, "cmp:farm-loop", Some(102.36));
    let r = g.budget_report(CON).expect("report");
    assert_eq!(r.verdict, BudgetVerdict::Exceeded, "{}", r.composition_note);
    assert_eq!(r.judged_on, "each");
    assert_eq!(
        r.judged_total,
        Some(12.0),
        "the worst part is the one judged"
    );
    assert!(
        r.composition_note.contains("cmp:barn-lean-to"),
        "the verdict names the part that breaks it: {}",
        r.composition_note
    );
}

#[test]
fn short_parts_never_add_up_to_a_clearance() {
    let mut g = clearance("minimum", 10.0, None);
    part(&mut g, "cmp:a", Some(6.0));
    part(&mut g, "cmp:b", Some(6.0));
    let r = g.budget_report(CON).expect("report");
    assert_eq!(r.verdict, BudgetVerdict::Exceeded);
    assert_eq!(r.judged_total, Some(6.0));
}

#[test]
fn every_part_clear_reads_within_on_the_nearest() {
    let mut g = clearance("minimum", 100.0, None);
    part(&mut g, "cmp:farm-loop", Some(102.36));
    part(&mut g, "cmp:shelter", Some(102.9));
    let r = g.budget_report(CON).expect("report");
    assert_eq!(r.verdict, BudgetVerdict::Within);
    assert_eq!(r.judged_total, Some(102.36));
    assert!(r.composition_note.contains("cmp:farm-loop"));
}

#[test]
fn an_unstated_part_leaves_it_open_unless_a_stated_one_already_breaks_it() {
    let mut g = clearance("minimum", 100.0, None);
    part(&mut g, "cmp:farm-loop", Some(102.36));
    part(&mut g, "cmp:barn-lean-to", None);
    assert_eq!(
        g.budget_report(CON).expect("report").verdict,
        BudgetVerdict::Incomplete,
        "the unmeasured barn may be the part that breaks it"
    );
    part(&mut g, "cmp:septic", Some(40.0));
    assert_eq!(
        g.budget_report(CON).expect("report").verdict,
        BudgetVerdict::Exceeded,
        "a stated part already short is a breach whatever the unstated one reads"
    );
}

#[test]
fn an_empty_limit_reads_incomplete_never_exceeded() {
    let g = clearance("minimum", 100.0, None);
    let r = g.budget_report(CON).expect("report");
    assert_eq!(
        r.verdict,
        BudgetVerdict::Incomplete,
        "{}",
        r.composition_note
    );
    assert_eq!(r.judged_total, None);
}

#[test]
fn a_maximum_each_part_meets_reads_the_tallest() {
    let mut g = clearance("maximum", 40.0, None);
    part(&mut g, "cmp:shop-roof", Some(31.5));
    part(&mut g, "cmp:solar-mast", Some(45.0));
    let r = g.budget_report(CON).expect("report");
    assert_eq!(r.verdict, BudgetVerdict::Exceeded);
    assert_eq!(r.judged_total, Some(45.0));
    assert!(r.composition_note.contains("cmp:solar-mast"));
}

#[test]
fn a_kpp_judged_on_each_part_is_breached_by_its_worst_part() {
    let mut g = clearance("minimum", 100.0, Some("kpp"));
    part(&mut g, "cmp:barn-lean-to", Some(12.0));
    part(&mut g, "cmp:farm-loop", Some(102.36));
    let breached = g.detect_gaps().expect("gaps").into_iter().any(|c| {
        serde_json::to_value(c.gap_source)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            == Some("kpp_breached".to_string())
    });
    assert!(breached, "the KPP breach reads the same judged rollup");
}

#[test]
fn a_reading_of_each_does_not_need_dependencies_drawn() {
    // `path` needs DEPENDS_ON among the parts; `each` needs nothing but the
    // readings, so drawing one changes nothing.
    let mut g = clearance("minimum", 100.0, None);
    part(&mut g, "cmp:a", Some(120.0));
    part(&mut g, "cmp:b", Some(130.0));
    g.create_edge(
        edge::DEPENDS_ON,
        node::COMPONENT,
        "cmp:a",
        node::COMPONENT,
        "cmp:b",
        Props::new(),
    )
    .expect("dep");
    let r = g.budget_report(CON).expect("report");
    assert_eq!(r.verdict, BudgetVerdict::Within);
    assert_eq!(r.judged_total, Some(120.0));
}

#[test]
fn a_summed_minimum_nothing_has_measured_reads_incomplete_not_exceeded() {
    // The same empty reading for an undeclared (summed) minimum: nothing
    // stated is nothing measured, so it is not a breach anyone recorded.
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_constraint(
        "con:setback",
        "At least 10 ft from the side lines",
        "BCC 11.07.090(a)(4).",
        Some("regulatory"),
        Some("distance_ft"),
        Some(10.0),
        None,
        Some("minimum"),
    )
    .expect("constraint");
    assert_eq!(
        g.budget_report("con:setback").expect("report").verdict,
        BudgetVerdict::Incomplete
    );
}
