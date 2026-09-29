//! A budget says how its contributions compose, and every reader of its
//! verdict follows what it says (req:a-budget-says-whether-its-parts-add-up-or-run-along-a-path).
//!
//! The class, not the instance: the verdict, the closure leg's margin and the
//! KPP breach all read ONE judged rollup, and the tests below pin each reader
//! against it — a reader left comparing the plain sum is how the owner's
//! 33 ms write path read "46 ms, exceeded" (I18, 2026-09-29). These use the
//! API the fix introduces, so they do not compile before it; the failing-first
//! observation is the MCP-level tests/a_budget_says_how_it_composes.rs, run on
//! a build of main.

use reflow2_core::budget::BudgetVerdict;
use reflow2_core::graph::DesignGraph;
use reflow2_core::nodes::{Props, edge, node};

/// The owner agent's write path: gateway 5 → router 2 → host 6 → store 20,
/// with the rule delta 13 beside the commit. Path 33, sum 46.
fn write_path(limit: f64, composition: Option<&str>) -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_project("proj:1", "Engine").expect("project");
    g.add_constraint(
        "con:write",
        "Durable write p99",
        "A confirmed write stays under its limit.",
        Some("budget"),
        Some("latency_ms"),
        Some(limit),
        None,
        None,
    )
    .expect("constraint");
    if let Some(c) = composition {
        g.set_constraint_composition("con:write", c)
            .expect("composition");
    }
    for (id, ms) in [
        ("cmp:gw", 5.0),
        ("cmp:router", 2.0),
        ("cmp:host", 6.0),
        ("cmp:store", 20.0),
        ("cmp:rules", 13.0),
    ] {
        g.add_component(id, id, "a stage", None).expect("cmp");
        g.constrains("con:write", "Component", id, Some(ms), None, None, None)
            .expect("edge");
    }
    for (from, to) in [
        ("cmp:gw", "cmp:router"),
        ("cmp:router", "cmp:host"),
        ("cmp:host", "cmp:store"),
        ("cmp:host", "cmp:rules"),
    ] {
        g.create_edge(
            edge::DEPENDS_ON,
            node::COMPONENT,
            from,
            node::COMPONENT,
            to,
            Props::new(),
        )
        .expect("dep");
    }
    g
}

#[test]
fn a_path_budget_is_judged_on_its_heaviest_path() {
    let g = write_path(40.0, Some("path"));
    let r = g.budget_report("con:write").expect("report");
    assert_eq!(r.total, 46.0);
    assert_eq!(r.worst_path_total, 33.0);
    assert_eq!(r.judged_on, "path");
    assert_eq!(r.judged_total, Some(33.0));
    assert_eq!(r.verdict, BudgetVerdict::Within);
    assert!(r.composition_note.contains("33") && r.composition_note.contains("46"));
}

#[test]
fn a_path_budget_over_its_limit_along_the_path_is_exceeded() {
    let g = write_path(30.0, Some("path"));
    let r = g.budget_report("con:write").expect("report");
    assert_eq!(r.verdict, BudgetVerdict::Exceeded, "33 > 30 along the path");
}

#[test]
fn an_undeclared_budget_keeps_the_sum_and_names_the_path_it_did_not_read() {
    let g = write_path(40.0, None);
    let r = g.budget_report("con:write").expect("report");
    assert_eq!(r.composition, None, "absent means nobody said");
    assert_eq!(r.judged_on, "sum");
    assert_eq!(r.judged_total, Some(46.0));
    assert_eq!(
        r.verdict,
        BudgetVerdict::Exceeded,
        "today's verdict is kept — changing it on a design that never asked is the \
         inference this refuses"
    );
    assert!(
        r.composition_note.contains("did NOT read") && r.composition_note.contains("33"),
        "{}",
        r.composition_note
    );
}

#[test]
fn a_declared_sum_is_judged_on_the_sum_even_with_a_path_present() {
    let g = write_path(40.0, Some("sum"));
    let r = g.budget_report("con:write").expect("report");
    assert_eq!(r.judged_on, "sum");
    assert_eq!(r.verdict, BudgetVerdict::Exceeded);
    assert!(!r.composition_note.contains("did NOT read"));
}

#[test]
fn a_composition_outside_the_set_is_refused_and_names_the_set() {
    let mut g = write_path(40.0, None);
    let err = g
        .set_constraint_composition("con:write", "average")
        .expect_err("not a composition")
        .to_string();
    assert!(err.contains("sum") && err.contains("path"), "{err}");
}

#[test]
fn a_path_budget_with_no_dependency_drawn_reaches_no_numeric_verdict() {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_constraint(
        "con:lat",
        "Latency",
        "Under 40.",
        Some("budget"),
        Some("latency_ms"),
        Some(40.0),
        None,
        None,
    )
    .expect("constraint");
    g.set_constraint_composition("con:lat", "path")
        .expect("composition");
    for id in ["cmp:a", "cmp:b"] {
        g.add_component(id, id, "a part", None).expect("cmp");
        g.constrains("con:lat", "Component", id, Some(30.0), None, None, None)
            .expect("edge");
    }
    let r = g.budget_report("con:lat").expect("report");
    assert_eq!(r.judged_total, None);
    assert_eq!(
        r.verdict,
        BudgetVerdict::Incomplete,
        "taking the max of unjoined parts (30) would pass a budget whose true figure is \
         unknown — for a maximum, the dangerous direction"
    );
    assert!(
        r.composition_note.contains("DEPENDS_ON"),
        "{}",
        r.composition_note
    );
}

#[test]
fn a_path_budget_on_a_minimum_reaches_no_numeric_verdict() {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_constraint(
        "con:tp",
        "Throughput",
        "At least 100.",
        Some("budget"),
        Some("rate_ops"),
        Some(100.0),
        None,
        Some("minimum"),
    )
    .expect("constraint");
    g.set_constraint_composition("con:tp", "path")
        .expect("composition");
    g.add_component("cmp:a", "a", "a part", None).expect("cmp");
    g.constrains(
        "con:tp",
        "Component",
        "cmp:a",
        Some(500.0),
        None,
        None,
        None,
    )
    .expect("edge");
    let r = g.budget_report("con:tp").expect("report");
    assert_eq!(r.verdict, BudgetVerdict::Incomplete);
    assert!(
        r.composition_note.contains("MAXIMUM"),
        "{}",
        r.composition_note
    );
}

#[test]
fn an_unstated_contributor_leaves_a_path_verdict_open_unless_the_overrun_is_proven() {
    // Within on the stated path, one contributor unstated: open.
    let mut g = write_path(40.0, Some("path"));
    g.add_component("cmp:cache", "cache", "a stage", None)
        .expect("cmp");
    g.constrains(
        "con:write",
        "Component",
        "cmp:cache",
        None,
        None,
        None,
        None,
    )
    .expect("edge");
    let r = g.budget_report("con:write").expect("report");
    assert_eq!(r.verdict, BudgetVerdict::Incomplete);
    // Over the limit on the stated path already: provably exceeded, because
    // an unknown can only make a path heavier.
    let mut g = write_path(30.0, Some("path"));
    g.add_component("cmp:cache", "cache", "a stage", None)
        .expect("cmp");
    g.constrains(
        "con:write",
        "Component",
        "cmp:cache",
        None,
        None,
        None,
        None,
    )
    .expect("edge");
    let r = g.budget_report("con:write").expect("report");
    assert_eq!(r.verdict, BudgetVerdict::Exceeded);
}

#[test]
fn the_closure_margin_is_kept_against_the_rollup_the_verdict_read() {
    // Path 33 against 40 with a 5 ms margin: 33 <= 35 closes. The sum (46)
    // would have failed it — a margin read off the sum would reopen the
    // very false alarm the verdict no longer raises.
    let mut g = write_path(40.0, Some("path"));
    g.set_constraint_margin("con:write", 5.0).expect("margin");
    g.set_closure_criterion("proj:1", &["budgets"], 1.0)
        .expect("criterion");
    let c = g.closure_report().expect("closure");
    let leg = c
        .legs
        .iter()
        .find(|l| l.leg == "budgets")
        .expect("budgets leg");
    assert_eq!((leg.swept, leg.closed), (1, 1), "{c:#?}");

    // And a margin the path does NOT keep is named with the path figure.
    let mut g = write_path(40.0, Some("path"));
    g.set_constraint_margin("con:write", 10.0).expect("margin");
    g.set_closure_criterion("proj:1", &["budgets"], 1.0)
        .expect("criterion");
    let c = g.closure_report().expect("closure");
    let hole = c.first_hole.expect("a hole");
    assert!(hole.why.contains("path 33"), "{}", hole.why);
}

#[test]
fn every_budget_is_read_at_once_and_a_prohibition_is_counted_not_dropped() {
    let mut g = write_path(40.0, Some("path"));
    g.add_constraint(
        "con:mass",
        "Mass",
        "At most 10 kg.",
        Some("budget"),
        Some("mass_kg"),
        Some(10.0),
        None,
        None,
    )
    .expect("constraint");
    g.add_constraint(
        "con:no-pii",
        "No PII leaves the device",
        "No personal data leaves the device.",
        None,
        None,
        None,
        None,
        None,
    )
    .expect("constraint");
    let s = g.budget_reports().expect("sweep");
    let ids: Vec<&str> = s.budgets.iter().map(|r| r.constraint_id.as_str()).collect();
    assert_eq!(ids, vec!["con:mass", "con:write"]);
    assert_eq!(s.swept, 2);
    assert_eq!(s.not_budgets, 1);
    assert_eq!(
        s.by_verdict.get("within"),
        Some(&2),
        "the path budget (33 of 40) and the empty mass budget (0 of 10): {:?}",
        s.by_verdict
    );
}
