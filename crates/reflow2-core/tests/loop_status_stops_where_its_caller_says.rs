//! `loop_status` stops where its caller says, and names what it did not compute.
//!
//! THE CLASS, not the instance (req:loop-status-answers-within-a-time-budget-and-names-the-sections-it-did-not-compute;
//! fact:root-cause-loop-status-on-reflow2s-design-takes-38-s-on-flo2-io-past-its-30-s-door-gaps-and-defects-each-about-half-2026-11-07):
//! a report that recomputes whole-design analyses on every call will cross
//! any fixed door as the design grows, so the report must be able to stop at
//! a check point and say, section by section, what it left out. Speed is not
//! asserted here (a duration in a shared suite measures the machine); the
//! STOP is, through `reflow2_core::stop` with a deterministic answer, which is
//! what keeps the core clock-free.
//!
//! What is pinned:
//! 1. With no stop scope open, every section is computed and `not_computed`
//!    is empty: every existing caller gets exactly the answer it always did.
//! 2. Told to stop at once, the floor is still computed, all five whole-design
//!    analyses are named in order with the call that computes each alone,
//!    their counts read NULL (never zero), and the answer is not clean.
//! 3. Told to stop inside gap detection, the cheaper sections before it are
//!    kept whole, gaps is named as started, and defects as never begun.
//! 4. A sweep stopped part-way keeps nothing: the next whole read recomputes
//!    it and finds what an unstopped read finds.
//! 5. A design with nothing owed reads clean when whole and NOT clean when a
//!    section was left out — "I did not look" never reads as "nothing found".
//!
//! FAILED FIRST on main at 3256b8188: `reflow2_core::stop` and
//! `LoopStatus::not_computed` did not exist, so this file did not compile.

use reflow2_core::DesignGraph;
use reflow2_core::stop::{StopReason, within};

/// Gaps (an unsatisfied requirement, an unallocated capability) and a defect
/// (two components that depend on each other).
fn graph_with_gaps_and_a_defect() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("in-memory graph");
    g.add_requirement("req:r", "R", "The thing records what it is told.")
        .unwrap();
    g.add_capability("cap:c", "C", "Records.", None).unwrap();
    g.add_component("cmp:a", "A", "does a", None).unwrap();
    g.add_component("cmp:b", "B", "does b", None).unwrap();
    g.depends_on("cmp:a", "cmp:b").unwrap();
    g.depends_on("cmp:b", "cmp:a").unwrap();
    g
}

#[test]
fn with_no_stop_scope_every_section_is_computed() {
    let g = graph_with_gaps_and_a_defect();
    let s = g.loop_status().unwrap();
    assert!(s.not_computed.is_empty(), "{:?}", s.not_computed);
    assert!(s.unsurfaced_gaps.unwrap() > 0, "the design has open gaps");
    assert!(
        s.structural_defects.unwrap() > 0,
        "and a circular dependency"
    );
    assert!(s.unexamined_claims.is_some());
    assert!(
        !s.next.iter().any(|l| l.contains("NOT COMPUTED")),
        "{:?}",
        s.next
    );
}

#[test]
fn told_to_stop_at_once_it_keeps_the_floor_and_names_every_analysis() {
    let g = graph_with_gaps_and_a_defect();
    let whole = g.loop_status().unwrap();
    // A fresh graph, so nothing is answered from a memo the whole read filled.
    let g = graph_with_gaps_and_a_defect();
    let part = within(|_| Some(StopReason::Budget), || g.loop_status()).unwrap();

    let named: Vec<&str> = part.not_computed.iter().map(|n| n.section).collect();
    assert_eq!(
        named,
        [
            "confirmation_ledger",
            "reasoning",
            "loop_closure",
            "gaps",
            "defects"
        ],
        "every whole-design analysis, in the order it runs"
    );
    for n in &part.not_computed {
        assert_eq!(n.why, "budget");
        assert!(!n.started, "{} never began", n.section);
        assert!(!n.call.is_empty() && !n.fields.is_empty(), "{n:?}");
    }
    assert_eq!(
        part.not_computed[3].call.split(' ').next(),
        Some("detect_gaps")
    );
    assert_eq!(part.not_computed[4].call, "detect_defects");
    // NULL, never zero.
    assert_eq!(part.unsurfaced_gaps, None);
    assert_eq!(part.answered_with_open_gap, None);
    assert_eq!(part.answered_naming_their_record, None);
    assert_eq!(part.structural_defects, None);
    assert_eq!(part.unexamined_claims, None);
    assert!(part.reasoning.is_none());
    assert!(part.loop_closure.is_none());
    // The floor is the whole read's floor.
    assert_eq!(part.unanswered_questions, whole.unanswered_questions);
    assert_eq!(part.unproven_capabilities, whole.unproven_capabilities);
    assert_eq!(part.breaches_open, whole.breaches_open);
    assert_eq!(part.follow_ups_open, whole.follow_ups_open);
    // Never clean, and `next` leads with what was left out.
    assert!(!part.clean);
    for (i, n) in part.not_computed.iter().enumerate() {
        assert!(
            part.next[i].starts_with("NOT COMPUTED THIS CALL") && part.next[i].contains(n.call),
            "{:?}",
            part.next
        );
    }
    let json = serde_json::to_value(&part).unwrap();
    assert!(json["unsurfaced_gaps"].is_null(), "{json}");
    assert!(json["structural_defects"].is_null(), "{json}");
}

#[test]
fn told_to_stop_inside_gap_detection_it_keeps_what_ran_before() {
    let whole = graph_with_gaps_and_a_defect().loop_status().unwrap();
    let g = graph_with_gaps_and_a_defect();
    // A deadline: once passed, it stays passed.
    let passed = std::cell::Cell::new(false);
    let part = within(
        move |at| {
            if at.starts_with("gaps:") {
                passed.set(true);
            }
            passed.get().then_some(StopReason::Budget)
        },
        || g.loop_status(),
    )
    .unwrap();

    assert_eq!(
        part.unexamined_claims, whole.unexamined_claims,
        "the ledger ran"
    );
    let named: Vec<(&str, bool)> = part
        .not_computed
        .iter()
        .map(|n| (n.section, n.started))
        .collect();
    assert_eq!(
        named,
        [("gaps", true), ("defects", false)],
        "gaps was stopped part-way, defects never began"
    );
    assert!(part.not_computed[0].stopped_at.starts_with("gaps:"));
    assert!(!part.clean);
}

#[test]
fn a_sweep_stopped_part_way_keeps_nothing() {
    let g = graph_with_gaps_and_a_defect();
    let stopped = within(
        |at| (at == "gaps:unallocated_capabilities").then_some(StopReason::CallerGone),
        || g.detect_gaps(),
    );
    let err = stopped.expect_err("the sweep stops at that detector");
    assert_eq!(
        reflow2_core::stop::stopped(&err),
        Some(StopReason::CallerGone)
    );
    let before = g.derived_recomputes();
    let now = g.detect_gaps().unwrap();
    assert_eq!(
        g.derived_recomputes(),
        before + 1,
        "nothing half-computed was kept: the whole read sweeps again"
    );
    let fresh = graph_with_gaps_and_a_defect().detect_gaps().unwrap();
    let ids = |v: &[reflow2_core::GapCandidate]| v.iter().map(|g| g.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&now), ids(&fresh));
}

#[test]
fn a_design_owed_nothing_is_clean_only_when_whole() {
    let g = DesignGraph::open_in_memory().unwrap();
    let whole = g.loop_status().unwrap();
    assert!(
        whole.clean,
        "an empty design owes nothing: {:?}",
        whole.next
    );
    let part = within(
        |at| (at == "loop_status:defects").then_some(StopReason::Budget),
        || g.loop_status(),
    )
    .unwrap();
    assert_eq!(part.not_computed.len(), 1);
    assert!(!part.clean, "a section left out is not a clean bill");
    let scoped = {
        let mut g = DesignGraph::open_in_memory().unwrap();
        g.add_contributor("who:p", "P", None, None, None).unwrap();
        within(
            |_| Some(StopReason::Budget),
            || g.loop_status_for(Some("who:p")),
        )
        .unwrap()
    };
    assert!(!scoped.clean, "nor scoped to a person: {:?}", scoped.next);
    assert!(scoped.next[0].starts_with("NOT COMPUTED THIS CALL"));
}
