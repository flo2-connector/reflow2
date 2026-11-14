//! A LOOP CHECK ASKED AGAIN WITH NO WRITE SINCE ANSWERS FROM WHAT IT KEPT
//! (`req:a-loop-check-asked-again-with-no-write-since-answers-from-what-it-kept`,
//! Anthony 2026-11-13).
//!
//! MEASURED 2026-11-13 on reflow2's own design at 8,916 nodes, engine 0.98.0
//! (`fact:measured-the-four-measurements-the-loop-check-idea-asked-for-2026-11-13`):
//! a `loop_status` asked again with no write since took 3.1 to 3.4 s, against
//! a budget of 2,000 ms (`con:loop-status-warm-latency-ms`). Gap and defect
//! detection were kept per state of the design
//! (`dec:derived-scans-are-memoised-per-write-generation`); loop closure
//! (62% of it), the check list, the confirmation ledger and the reasoning
//! debt were worked out again on every ask from records no write had touched.
//!
//! THIS ASSERTS STRUCTURE, NOT DURATION, as
//! `a_derived_scan_is_computed_once_per_write.rs` does and for its reason: how
//! many times each part was actually WORKED OUT ([`DesignGraph::kept_builds`]),
//! which no load on the machine moves.
//!
//! And the other half, which is the invariant the requirement carries: a kept
//! result is NEVER SERVED STALE. A write drops it. A call handed different
//! readings of another design (`reflow2_core::far`), which no write to this
//! design announces, gets the answer for ITS readings. A pass told to stop
//! part-way keeps nothing.
//!
//! NOT TESTED, because nothing can make it happen: a write landing WHILE a
//! part is being worked out. A write takes `&mut DesignGraph`, so none can
//! land beside a `&self` read. [`DesignGraph::kept`] still compares the write
//! generation before it keeps, for the day that changes.

use std::sync::Arc;

use reflow2_core::DesignGraph;
use reflow2_core::far::{self, FarReading, FarReadings, FarState};
use reflow2_core::inference::{InferenceSpec, Truth};
use reflow2_core::relate::RelationLink;
use reflow2_core::stop::{StopReason, within};

/// The parts of a loop check kept by name, as `DesignGraph::kept` names them.
const PARTS: [&str; 4] = [
    "verification_recency",
    "confirmation_ledger",
    "reasoning_debt",
    "loop_closure",
];

/// A design with something for each part to read: a check that claims an
/// outcome, a capability, a defect (two components that depend on each
/// other), and a claim one confirmed step concludes from a node in ANOTHER
/// design. Returns the design and the id of its reference to that node.
fn design() -> (DesignGraph, String) {
    let mut g = DesignGraph::open_in_memory_as("near").unwrap();
    g.add_requirement("req:claim", "Claim", "A false answer is proven.")
        .unwrap();
    g.add_capability("cap:c", "C", "Records.", None).unwrap();
    g.add_component("cmp:a", "A", "does a", None).unwrap();
    g.add_component("cmp:b", "B", "does b", None).unwrap();
    g.depends_on("cmp:a", "cmp:b").unwrap();
    g.depends_on("cmp:b", "cmp:a").unwrap();
    g.add_verification("ver:v", "V", Some("test"), None, None)
        .unwrap();
    g.verifies("ver:v", "Capability", "cap:c").unwrap();
    g.set_verification_status("ver:v", "passing", Some("2026-11-13T00:00:00Z"), None)
        .unwrap();
    let xref = g
        .ensure_design_reference("far", "req:theorem", Some("Requirement"), None)
        .unwrap();
    g.review_relations(
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
    g.add_inference(&InferenceSpec {
        id: "inf:claim".into(),
        rule: "all".into(),
        premises: vec![xref.clone()],
        conclusion: "req:claim".into(),
        confirmed_by: Some("who:checker".into()),
        confirmed_how: Some("person".into()),
        confirmed_at: Some("2026-10-19T00:00:00Z".into()),
        ..Default::default()
    })
    .unwrap();
    (g, xref)
}

/// The far theorem read as holding there and SHOWN, not proven: the claim
/// here then reads shown, where with nothing handed in it reads unproven.
fn shown_there(xref: &str) -> Option<Arc<FarReadings>> {
    let mut map = FarReadings::new();
    map.insert(
        xref.to_string(),
        FarReading {
            design: "far".into(),
            node_id: "req:theorem".into(),
            state: FarState::Read,
            truth: Truth::Holds,
            why: "one step concludes it from an axiom taken as given".into(),
            proof_status: Some("shown".into()),
            holds_only_if: Vec::new(),
        },
    );
    Some(Arc::new(map))
}

fn json<T: serde::Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).unwrap()
}

fn builds(g: &DesignGraph) -> Vec<u64> {
    PARTS.iter().map(|p| g.kept_builds(p)).collect()
}

/// THE CASE. Asked three times with no write between: each part is worked out
/// ONCE, and the three answers are the same answer.
#[test]
fn asked_again_with_no_write_each_part_is_worked_out_once() {
    let (g, _) = design();
    assert_eq!(
        builds(&g),
        [0, 0, 0, 0],
        "nothing is worked out until asked"
    );

    let first = g.loop_status().unwrap();
    assert_eq!(builds(&g), [1, 1, 1, 1], "the first ask works each out");
    assert!(first.not_computed.is_empty(), "{:?}", first.not_computed);
    assert!(
        first.reasoning.is_some() && first.loop_closure.is_some(),
        "the fixture gives the reasoning and loop closure something to read"
    );

    let second = g.loop_status().unwrap();
    let third = g.loop_status().unwrap();
    assert_eq!(
        builds(&g),
        [1, 1, 1, 1],
        "two more asks with no write since work none of them out again"
    );
    assert_eq!(json(&second), json(&first));
    assert_eq!(json(&third), json(&first));

    // The checks loop_status is made of, asked on their own, read the same
    // kept results.
    g.loop_closure().unwrap();
    g.confirmation_ledger().unwrap();
    g.reasoning_debt().unwrap();
    g.verification_recency().unwrap();
    g.detect_gaps().unwrap();
    assert_eq!(builds(&g), [1, 1, 1, 1]);
}

/// The defect sweep's description of what it looked at was worked out on
/// every `detect_defects`, although the defects under it were kept.
#[test]
fn the_defect_sweep_asked_again_is_worked_out_once() {
    let (mut g, _) = design();
    let a = g.detect_defects().unwrap();
    let b = g.detect_defects().unwrap();
    assert_eq!(g.kept_builds("detect_defects"), 1);
    assert_eq!(json(&a), json(&b));
    assert!(!a.defects.is_empty(), "the fixture holds a defect");

    g.delete_edge("DEPENDS_ON", "cmp:b", "cmp:a").unwrap();
    let c = g.detect_defects().unwrap();
    assert_eq!(g.kept_builds("detect_defects"), 2, "a write drops it");
    assert!(
        c.defects.len() < a.defects.len(),
        "and the sweep after the write is of the design after the write"
    );
}

/// NEVER STALE, the write half: any write drops what was kept, and the next
/// ask is of the design as the write left it.
#[test]
fn a_write_drops_what_was_kept_and_the_next_ask_reads_the_design_as_written() {
    let (mut g, _) = design();
    let before = g.loop_status().unwrap();
    assert_eq!(builds(&g), [1, 1, 1, 1]);

    g.add_verification("ver:w", "W", Some("test"), None, None)
        .unwrap();
    g.set_verification_status("ver:w", "failing", Some("2026-11-13T01:00:00Z"), None)
        .unwrap();

    let after = g.loop_status().unwrap();
    assert_eq!(builds(&g), [2, 2, 2, 2], "each part is worked out again");
    assert_eq!(after.verifications.len(), before.verifications.len() + 1);
    assert_eq!(
        after.verifications[0].verification_id, "ver:w",
        "the failing check the write added leads the list"
    );
    assert_eq!(
        after.loop_closure.as_ref().unwrap().checks,
        before.loop_closure.as_ref().unwrap().checks + 1
    );

    // And it is the answer a design that never kept anything gives.
    let (mut fresh, _) = design();
    fresh
        .add_verification("ver:w", "W", Some("test"), None, None)
        .unwrap();
    fresh
        .set_verification_status("ver:w", "failing", Some("2026-11-13T01:00:00Z"), None)
        .unwrap();
    assert_eq!(json(&after), json(&fresh.loop_status().unwrap()));
}

/// NEVER STALE, the half no write announces: what another design reads as is
/// handed in per call. A call handed different readings gets the reasoning
/// debt for ITS readings, and a call handed the same ones again (a new
/// allocation, the same content, as the server makes per call) reads what was
/// kept.
#[test]
fn a_call_handed_other_readings_of_another_design_gets_the_answer_for_its_own() {
    // What each reading gives on a design that has kept nothing.
    let (fresh, xref) = design();
    let unreached = fresh.loop_status().unwrap();
    let (fresh, _) = design();
    let shown = far::scoped(shown_there(&xref), || fresh.loop_status()).unwrap();
    assert_ne!(
        json(&unreached.reasoning),
        json(&shown.reasoning),
        "the fixture must tell the two readings apart, or this proves nothing"
    );

    let (g, xref) = design();
    assert_eq!(json(&g.loop_status().unwrap()), json(&unreached));
    assert_eq!(g.kept_builds("reasoning_debt"), 1);

    let handed = far::scoped(shown_there(&xref), || g.loop_status()).unwrap();
    assert_eq!(
        json(&handed),
        json(&shown),
        "the answer for the readings this call was handed, not the one kept for the last call"
    );
    assert_eq!(g.kept_builds("reasoning_debt"), 2);

    // The same content in a new allocation: nothing is worked out again.
    let again = far::scoped(shown_there(&xref), || g.loop_status()).unwrap();
    assert_eq!(json(&again), json(&shown));
    assert_eq!(g.kept_builds("reasoning_debt"), 2);

    // And back to a call handed nothing.
    assert_eq!(json(&g.loop_status().unwrap()), json(&unreached));
    assert_eq!(g.kept_builds("reasoning_debt"), 3);
}

/// The same, for gap detection alone, which was kept per state of the design
/// before this file existed. Uses nothing this change added, so it runs
/// unchanged against the engine before it.
#[test]
fn gap_detection_follows_the_readings_a_call_was_handed() {
    let (fresh, xref) = design();
    let unreached = fresh.detect_gaps().unwrap();
    let (fresh, _) = design();
    let shown = far::scoped(shown_there(&xref), || fresh.detect_gaps()).unwrap();
    assert_ne!(
        json(&unreached),
        json(&shown),
        "the fixture must tell the two readings apart, or this proves nothing"
    );

    let (g, xref) = design();
    assert_eq!(json(&g.detect_gaps().unwrap()), json(&unreached));
    let handed = far::scoped(shown_there(&xref), || g.detect_gaps()).unwrap();
    assert_eq!(
        json(&handed),
        json(&shown),
        "the gaps for the readings this call was handed, not the ones kept for the last call"
    );
    assert_eq!(json(&g.detect_gaps().unwrap()), json(&unreached));
}

/// A pass told to stop part-way keeps nothing: the next ask works it out
/// whole, and gets what a design never stopped gets.
#[test]
fn a_part_told_to_stop_keeps_nothing() {
    let (g, _) = design();
    let stopped = within(
        |at| (at == "loop_status:loop_closure:check").then_some(StopReason::Budget),
        || g.loop_closure(),
    );
    assert_eq!(
        reflow2_core::stop::stopped(&stopped.expect_err("it stops at its first check")),
        Some(StopReason::Budget)
    );
    assert_eq!(g.kept_builds("loop_closure"), 0, "nothing was kept");

    // Through loop_status: the section is named as not computed, and what ran
    // before it is kept.
    let part = within(
        |at| (at == "loop_status:loop_closure:check").then_some(StopReason::Budget),
        || g.loop_status(),
    )
    .unwrap();
    let named: Vec<&str> = part.not_computed.iter().map(|n| n.section).collect();
    assert_eq!(named[0], "loop_closure", "{named:?}");
    assert!(part.loop_closure.is_none());
    assert_eq!(g.kept_builds("loop_closure"), 0);
    assert_eq!(g.kept_builds("confirmation_ledger"), 1);

    let whole = g.loop_status().unwrap();
    assert!(whole.not_computed.is_empty());
    assert_eq!(g.kept_builds("loop_closure"), 1);
    assert_eq!(
        g.kept_builds("confirmation_ledger"),
        1,
        "what the stopped ask finished is not worked out again"
    );
    let (fresh, _) = design();
    assert_eq!(json(&whole), json(&fresh.loop_status().unwrap()));
}

/// THE WHOLE ANSWER is kept too, per person asked about, so the part of
/// `loop_status` that is none of the analyses above (0.62 to 0.68 s of a
/// repeat ask at the measured size) is not worked out again either.
#[test]
fn the_whole_answer_is_kept_for_each_person_asked_about() {
    let (mut g, _) = design();
    g.add_contributor("who:p", "P", None, None, None).unwrap();

    let everyone = g.loop_status().unwrap();
    assert_eq!(g.kept_builds("loop_status"), 1);
    assert_eq!(json(&g.loop_status().unwrap()), json(&everyone));
    assert_eq!(
        g.kept_builds("loop_status"),
        1,
        "asked again: not worked out"
    );

    // Asked about one person: its own answer, kept beside the other.
    let p = g.loop_status_for(Some("who:p")).unwrap();
    assert!(p.scope.is_some() && everyone.scope.is_none());
    assert_eq!(g.kept_builds("loop_status:who:p"), 1);
    assert_eq!(json(&g.loop_status_for(Some("who:p")).unwrap()), json(&p));
    assert_eq!(json(&g.loop_status().unwrap()), json(&everyone));
    assert_eq!(
        (
            g.kept_builds("loop_status"),
            g.kept_builds("loop_status:who:p")
        ),
        (1, 1),
        "each is worked out once, and neither is handed out as the other"
    );
    assert_eq!(builds(&g), [1, 1, 1, 1], "over the same kept parts");

    // A person the design does not hold is still refused, kept answer or not.
    assert!(g.loop_status_for(Some("who:nobody")).is_err());

    // A write drops both.
    g.add_capability("cap:d", "D", "Reads.", None).unwrap();
    g.loop_status().unwrap();
    g.loop_status_for(Some("who:p")).unwrap();
    assert_eq!(
        (
            g.kept_builds("loop_status"),
            g.kept_builds("loop_status:who:p")
        ),
        (2, 2)
    );
}

/// An answer that names a section it left out is the stopped call's alone:
/// it is not kept, and the next ask is whole.
#[test]
fn an_answer_that_left_a_section_out_is_not_kept() {
    let (g, _) = design();
    let part = within(
        |at| (at == "loop_status:defects").then_some(StopReason::Budget),
        || g.loop_status(),
    )
    .unwrap();
    assert_eq!(part.not_computed.len(), 1, "{:?}", part.not_computed);

    let whole = g.loop_status().unwrap();
    assert!(whole.not_computed.is_empty(), "{:?}", whole.not_computed);
    let (fresh, _) = design();
    assert_eq!(json(&whole), json(&fresh.loop_status().unwrap()));
}

/// A caller told to stop at a section is NOT handed the kept answer: it
/// stops there and names the section, exactly as it did before anything was
/// kept (`loop_status_stops_where_its_caller_says.rs` holds the same on a
/// design owed nothing). The kept answer is still there for the next caller.
#[test]
fn a_caller_told_to_stop_at_a_section_still_stops_there() {
    let (g, _) = design();
    let whole = g.loop_status().unwrap();
    assert_eq!(g.kept_builds("loop_status"), 1);

    for section in [
        "loop_status:confirmation_ledger",
        "loop_status:reasoning",
        "loop_status:loop_closure",
        "loop_status:gaps",
        "loop_status:defects",
    ] {
        let part = within(
            move |at| (at == section).then_some(StopReason::Budget),
            || g.loop_status(),
        )
        .unwrap();
        assert_eq!(
            part.not_computed.len(),
            1,
            "{section}: {:?}",
            part.not_computed
        );
    }
    // A scope that stops nowhere is handed the kept answer.
    let unstopped = within(|_| None, || g.loop_status()).unwrap();
    assert_eq!(json(&unstopped), json(&whole));
    assert_eq!(json(&g.loop_status().unwrap()), json(&whole));
    assert_eq!(
        g.kept_builds("loop_status"),
        1,
        "the stopped asks neither used the kept answer nor replaced it"
    );
}
