//! The records a choice rests on carry a truth, a counter-argument is its own
//! statement, and a correct proof reads correct.
//!
//! Two accepted requirements (Anthony, 2026-10-18, phase 1 of the build list):
//!
//! - `req:the-records-a-choice-rests-on-carry-a-truth-and-a-counter-argument-is-its-own-statement`
//!   (A): a measured, standing finding reads `holds` as a premise; answering a
//!   counter-argument never reads a ChangeEvent false; an accepted Decision
//!   concluded by a chain reads the owner's acceptance AND the chain's verdict
//!   side by side, and its read-back says what it `chose`.
//! - `req:a-correct-proof-reads-correct-assumptions-objections-and-conditional-claims`
//!   (D, its top three): a discharged assumption is never `unsupported`; a
//!   deliberate objection is never a structural contradiction to heal; a
//!   conditional claim never carries a top-level `holds`.
//!
//! # The measured cases, as fixtures
//!
//! Exported read-only from flo2.io's engine on 2026-10-18 (0.87.0), the way
//! the design-copy tool of the time (retired 2026-10-30) read a design:
//!
//! - π+e (082888103637f0e7): its supposition `req:suppose-both-algebraic`,
//!   discharged by `inf:reductio`, was listed under `unsupported`;
//! - Arrow (6308149cf93f7723): 8 discharged assumptions under `unsupported`;
//! - Brocard (dd55db78c7cccda8): `req:conj-finitely-many`, conditional on the
//!   abc conjecture, read top-level `truth: holds`; the deliberate objection
//!   `req:objection-scholze-stix` was a detect_defects `contradiction`;
//! - case 1 of the proof of concept
//!   (`fact:reasoning-embedded-in-a-traditional-design-works-within-one-design-and-stops-at-premises-with-no-truth-and-at-design-borders-2026-10-18`):
//!   a slice of reflow2's own design around
//!   `dec:idea-a-cross-type-near-match-is-a-note-and-only-a-same-type-one-refuses`,
//!   whose measured premise read "reflow2 computes no truth", whose objection
//!   carried on a ChangeEvent read that history `fails` once answered, and
//!   whose read-back showed the deliberation's text, not its `chose`.
//!
//! The two earlier trial rungs (primes, the fundamental theorem) ride along
//! from the MCP crate's fixtures, so each class below is asked of six designs.
//!
//! Every test is a CLASS: an invariant over every node of every fixture, plus
//! the hand-built variants a fixture does not hold (a superseded fact, an
//! accidental contradiction). Each failed on main c9e6ca4 (0.87.0) first.
//! Readings are compared as JSON, so the file compiles on main.

use std::collections::BTreeSet;

use reflow2_core::DesignGraph;
use reflow2_core::export::GraphExport;
use reflow2_core::foundation::core::Value;
use reflow2_core::inference::Truth;
use reflow2_core::nodes::{Props, edge, node};
use serde_json::Value as Json;

const PI_E: &str = include_str!("fixtures/proof_trial_pi_plus_e.json");
const ARROW: &str = include_str!("fixtures/proof_trial_arrow.json");
const BROCARD: &str = include_str!("fixtures/proof_trial_brocard.json");
const CASE_1: &str = include_str!("fixtures/poc_case_1_a_decision_concluded_by_its_chain.json");
const RUNG_1: &str =
    include_str!("../../reflow2-mcp/tests/fixtures/proof_trial_rung_1_primes.json");
const RUNG_2: &str = include_str!("../../reflow2-mcp/tests/fixtures/proof_trial_rung_2_fta.json");

const ALL: [(&str, &str); 6] = [
    ("pi+e", PI_E),
    ("arrow", ARROW),
    ("brocard", BROCARD),
    ("case-1", CASE_1),
    ("rung-1", RUNG_1),
    ("rung-2", RUNG_2),
];

const DEC: &str = "dec:idea-a-cross-type-near-match-is-a-note-and-only-a-same-type-one-refuses";
const CHG: &str = "chg:a-cross-type-near-match-is-named-as-a-layer";
const VER: &str = "ver:a-near-match-across-types-or-within-reasoning-is-a-note-tests";
const RUNG_2_FACT: &str = "fact:the-proof-trial-rung-2-ran-on-flo2-io-and-what-counts-holds-while-recording-a-proof-is-still-costly-2026-10-17";
const FIRST_CHAIN: &str =
    "inf:the-near-match-note-choice-rests-on-the-rung-2-measurement-and-the-passing-class-test";
const SECOND_CHAIN: &str =
    "inf:the-near-match-note-choice-stands-while-its-class-test-replaying-rung-2-passes";

fn load(doc: &str) -> DesignGraph {
    let doc: GraphExport = serde_json::from_str(doc).expect("fixture parses");
    let mut g = DesignGraph::open_in_memory().expect("in-memory graph");
    g.import_graph(&doc).expect("fixture imports");
    g
}

fn ids_of(g: &DesignGraph, node_type: &str) -> Vec<String> {
    g.scan_nodes(node_type)
        .expect("scan")
        .into_iter()
        .map(|n| n.node_id)
        .collect()
}

/// The whole-design read-back, as the wire shows it.
fn report(g: &DesignGraph) -> Json {
    serde_json::to_value(
        g.reasoning_report(None, Some(500))
            .expect("reasoning_report"),
    )
    .expect("serializes")
}

fn report_on(g: &DesignGraph, claim: &str) -> Json {
    serde_json::to_value(
        g.reasoning_report(Some(claim), Some(500))
            .expect("reasoning_report"),
    )
    .expect("serializes")
}

fn claim<'a>(r: &'a Json, id: &str) -> &'a Json {
    r["claims"]
        .as_array()
        .expect("claims")
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("{id} is not in the read-back"))
}

fn prop(g: &DesignGraph, ty: &str, id: &str, key: &str) -> Option<String> {
    g.get_node(ty, id).expect("get").and_then(|n| {
        n.properties
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}

// ─── A (1): a measured, standing finding holds as a premise ─────────────────

/// EVERY TemporalFact of every fixture: measured (or unstated, which the
/// schema defaults to measured), not invalidated, not ended, not refuted or
/// contested reads `holds`; and the same fact made superseded (`valid_to`),
/// invalidated (`INVALIDATES`) or a forecast reads something else, never
/// `fails` (a lost or stale finding is unsupported, not refuted).
#[test]
fn a_measured_standing_finding_holds_and_a_superseded_invalidated_or_forecast_one_does_not() {
    let mut facts_read = 0;
    for (name, doc) in ALL {
        let g = load(doc);
        for id in ids_of(&g, node::TEMPORAL_FACT) {
            let (truth, why) = g.premise_truth(&id).expect("truth");
            let basis = prop(&g, node::TEMPORAL_FACT, &id, "basis");
            let ended = prop(&g, node::TEMPORAL_FACT, &id, "valid_to").is_some()
                || !g.outgoing(&id, Some(edge::VALID_TO)).unwrap().is_empty();
            let invalidated = !g.incoming(&id, Some(edge::INVALIDATES)).unwrap().is_empty();
            let objected = !g.objections_to(&id).unwrap().is_empty();
            let standing = basis.as_deref() != Some("forecast")
                && !ended
                && !invalidated
                && !objected
                && !g.is_discontinued(&id).unwrap();
            if standing {
                assert_eq!(
                    truth,
                    Truth::Holds,
                    "{name}: {id} is a standing measured finding and reads {truth:?}: {why}"
                );
            } else {
                assert_ne!(
                    truth,
                    Truth::Holds,
                    "{name}: {id} is not standing and reads holds: {why}"
                );
            }
            facts_read += 1;
        }
    }
    assert!(facts_read > 0, "no fixture held a TemporalFact");

    // The three ways a finding stops standing, each on every fact of case 1.
    for (way, f) in [("superseded", 0usize), ("invalidated", 1), ("forecast", 2)] {
        let mut g = load(CASE_1);
        for id in ids_of(&g, node::TEMPORAL_FACT) {
            match f {
                0 => {
                    g.upsert_node(
                        node::TEMPORAL_FACT,
                        &id,
                        Props::new().set("valid_to", "2026-10-18"),
                    )
                    .unwrap();
                }
                1 => {
                    g.create_edge(
                        edge::INVALIDATES,
                        node::CHANGE_EVENT,
                        CHG,
                        node::TEMPORAL_FACT,
                        &id,
                        Props::new(),
                    )
                    .unwrap();
                }
                _ => {
                    g.upsert_node(
                        node::TEMPORAL_FACT,
                        &id,
                        Props::new().set("basis", "forecast"),
                    )
                    .unwrap();
                }
            }
            let (truth, why) = g.premise_truth(&id).unwrap();
            assert_eq!(truth, Truth::Unknown, "{way} {id} reads {truth:?}: {why}");
            assert!(
                why.contains(way) || (way == "superseded" && why.contains("valid_to")),
                "{way} {id}: the why does not say so: {why}"
            );
        }
    }
}

/// The proof of concept's first chain rests on the rung-2 measurement and the
/// passing class test: it reads `holds` now, not `unknown`.
#[test]
fn the_choice_resting_on_a_measurement_and_a_passing_check_holds() {
    let g = load(CASE_1);
    let (fact, why) = g.premise_truth(RUNG_2_FACT).unwrap();
    assert_eq!(fact, Truth::Holds, "{why}");
    let step = g.inference_reading(FIRST_CHAIN).unwrap().expect("step");
    assert_eq!(step.verdict, Truth::Holds, "{:?}", step.premises);
}

// ─── A (2): a counter-argument is its own statement ─────────────────────────

/// No ChangeEvent of any fixture ever reads `fails`: it records that something
/// happened, and answering an argument it carried never makes the history
/// false. On main, case 1's ChangeEvent read "refuted by the check".
#[test]
fn no_change_event_reads_false() {
    for (name, doc) in ALL {
        let g = load(doc);
        for id in ids_of(&g, node::CHANGE_EVENT) {
            let (truth, why) = g.premise_truth(&id).unwrap();
            assert_ne!(
                truth,
                Truth::Fails,
                "{name}: {id} (history) reads false: {why}"
            );
        }
    }
}

/// The counter-argument re-expressed as a statement of its own: it opposes the
/// Decision, the passing check answers IT, and the Decision reads `holds`
/// while the ChangeEvent that first carried the argument is untouched.
#[test]
fn a_counter_argument_recorded_as_its_own_statement_is_answered_there() {
    let mut g = load(CASE_1);
    g.delete_edge(edge::CONTRADICTS, CHG, DEC).unwrap();
    g.delete_edge(edge::CONTRADICTS, VER, CHG).unwrap();
    let objection = "req:objection-a-cross-type-note-trades-a-false-alarm-for-a-missed-catch";
    g.upsert_node(
        node::REQUIREMENT,
        objection,
        Props::new()
            .set(
                "name",
                "Objection: a note across types trades a false alarm for a missed catch",
            )
            .set(
                "statement",
                "Excluding cross-type pairs from the guard removes a real catch.",
            )
            .set("statement_kind", "proposition")
            .set("source", CHG),
    )
    .unwrap();
    g.create_edge(
        edge::CONTRADICTS,
        node::REQUIREMENT,
        objection,
        node::DECISION,
        DEC,
        Props::new().set("alignment", "opposing"),
    )
    .unwrap();
    g.create_edge(
        edge::CONTRADICTS,
        node::VERIFICATION,
        VER,
        node::REQUIREMENT,
        objection,
        Props::new().set("alignment", "opposing"),
    )
    .unwrap();
    let (o, why) = g.premise_truth(objection).unwrap();
    assert_eq!(o, Truth::Fails, "the answered objection: {why}");
    let (d, why) = g.premise_truth(DEC).unwrap();
    assert_eq!(d, Truth::Holds, "the Decision: {why}");
    let (c, why) = g.premise_truth(CHG).unwrap();
    assert_ne!(c, Truth::Fails, "the ChangeEvent: {why}");
    let r = report_on(&g, DEC);
    let objections = claim(&r, DEC)["objections"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        objections
            .iter()
            .any(|x| x["id"] == objection && x["standing"] == "answered"),
        "{objections:?}"
    );
}

// ─── A (3): acceptance and chain side by side ───────────────────────────────

/// Every accepted Decision a step concludes, in every fixture, reads `holds`
/// unless an objection contests or refutes it, and its read-back carries the
/// owner's `acceptance` beside the chain's own `chain` verdict, and says what
/// it chose.
#[test]
fn an_accepted_decision_reads_its_acceptance_and_its_chain_side_by_side() {
    let mut asked = 0;
    for (name, doc) in ALL {
        let g = load(doc);
        let r = report(&g);
        for id in ids_of(&g, node::DECISION) {
            if g.inferences_concluding(&id).unwrap().is_empty()
                || prop(&g, node::DECISION, &id, "status").as_deref() != Some("accepted")
            {
                continue;
            }
            asked += 1;
            let objected = g
                .objections_to(&id)
                .unwrap()
                .iter()
                .any(|o| o.standing != "answered");
            let (truth, why) = g.premise_truth(&id).unwrap();
            if !objected {
                assert_eq!(truth, Truth::Holds, "{name}: {id}: {why}");
            }
            let c = claim(&r, &id);
            assert_eq!(c["acceptance"], "accepted", "{name}: {id}: {c}");
            assert!(
                c["chain"]["verdict"].is_string(),
                "{name}: {id}: no chain verdict: {c}"
            );
            if let Some(chose) = prop(&g, node::DECISION, &id, "chose") {
                assert_eq!(
                    c["says"],
                    Json::String(chose),
                    "{name}: {id} reads its deliberation, not its choice"
                );
            }
        }
    }
    assert!(
        asked > 0,
        "no fixture held an accepted Decision concluded by a step"
    );
}

/// The first, incomplete chain: with only the step that rests on the
/// measurement and the check, and the measurement's support taken away, the
/// accepted Decision still reads `holds` (the owner's word) and its chain reads
/// `unknown` beside it, never "the decision is unsupported".
#[test]
fn a_first_incomplete_chain_never_reads_as_the_decision_being_unsupported() {
    let mut g = load(CASE_1);
    // The first chain alone, with nothing standing against the Decision.
    g.delete_edge(edge::CONCLUDES, SECOND_CHAIN, DEC).unwrap();
    g.delete_edge(edge::CONTRADICTS, CHG, DEC).unwrap();

    // The change walk still shows the break: losing the check leaves the
    // chain unsupported, and the row says the Decision now stands on its
    // owner's acceptance alone, not that another line still supports it.
    let walk = g.inference_walk(&[VER]).unwrap();
    let row = walk
        .iter()
        .find(|w| w.inference == FIRST_CHAIN)
        .expect("row");
    assert_eq!(row.impact, "unsupported", "{row:?}");
    assert_eq!(
        row.conclusion_reads, "accepted_without_a_holding_chain",
        "{row:?}"
    );
    assert!(row.still_supported_by.is_empty(), "{row:?}");

    // An incomplete chain: its measured premise made a forecast.
    g.upsert_node(
        node::TEMPORAL_FACT,
        RUNG_2_FACT,
        Props::new().set("basis", "forecast"),
    )
    .unwrap();
    let (truth, why) = g.premise_truth(DEC).unwrap();
    assert_eq!(truth, Truth::Holds, "{why}");
    assert!(why.contains("accepted") && why.contains("chain"), "{why}");
    let r = report_on(&g, DEC);
    let c = claim(&r, DEC);
    assert_eq!(c["acceptance"], "accepted", "{c}");
    assert_eq!(c["chain"]["verdict"], "unknown", "{c}");
    assert!(
        !r["unsupported"]
            .as_array()
            .unwrap()
            .iter()
            .any(|u| u["id"] == DEC),
        "{}",
        r["unsupported"]
    );
}

// ─── D (1): a discharged assumption is never unsupported ────────────────────

/// In every fixture, no assumption a step discharges — and no node is used
/// only as an assumption — is listed under `unsupported`.
#[test]
fn a_discharged_assumption_is_never_unsupported() {
    let mut discharged_seen = 0;
    for (name, doc) in ALL {
        let g = load(doc);
        let r = report(&g);
        let unsupported: BTreeSet<String> = r["unsupported"]
            .as_array()
            .unwrap()
            .iter()
            .map(|u| u["id"].as_str().unwrap().to_string())
            .collect();
        for a in g.assumptions().unwrap() {
            if a.discharged_by.is_empty() {
                continue;
            }
            let ordinary_premise = g
                .outgoing(&a.id, Some(edge::PREMISE_OF))
                .unwrap()
                .iter()
                .any(|e| e.properties.get("role").and_then(Value::as_str) != Some("assumed"));
            if ordinary_premise {
                continue;
            }
            discharged_seen += 1;
            assert!(
                !unsupported.contains(&a.id),
                "{name}: {} is discharged by {:?} and listed unsupported",
                a.id,
                a.discharged_by
            );
        }
    }
    // π+e's one, Arrow's eight, and the earlier rungs'.
    assert!(discharged_seen >= 9, "{discharged_seen}");
}

// ─── D (2): a deliberate objection is not a defect; an accidental one is ────

/// In every fixture, no opposing CONTRADICTS in a line of reasoning is a
/// detect_defects `contradiction`, and loop_status counts none. Brocard's
/// Scholze-Stix objection was one on main.
#[test]
fn a_deliberate_objection_is_never_a_structural_contradiction() {
    for (name, doc) in ALL {
        let g = load(doc);
        let sweep = g.detect_defects().unwrap();
        let contradictions: Vec<_> = sweep
            .defects
            .iter()
            .filter(|d| d.category.as_str() == "contradiction")
            .map(|d| d.affected_ids.clone())
            .collect();
        assert!(contradictions.is_empty(), "{name}: {contradictions:?}");
        let ls = g.loop_status().unwrap();
        assert_eq!(ls.structural_defects, sweep.defects.len(), "{name}");
    }
}

/// An accidental contradiction — two requirements, neither a statement nor in
/// any line of reasoning — added to every fixture is still reported, by
/// detect_defects and in loop_status's count.
#[test]
fn an_accidental_contradiction_is_still_reported() {
    for (name, doc) in ALL {
        let mut g = load(doc);
        let before = g.loop_status().unwrap().structural_defects;
        g.add_requirement(
            "req:the-box-weighs-under-a-kilo",
            "Under a kilo",
            "The box weighs under a kilo.",
        )
        .unwrap();
        g.add_requirement(
            "req:the-box-carries-a-lead-ballast",
            "A lead ballast",
            "The box carries 5 kg of lead.",
        )
        .unwrap();
        g.create_edge(
            edge::CONTRADICTS,
            node::REQUIREMENT,
            "req:the-box-carries-a-lead-ballast",
            node::REQUIREMENT,
            "req:the-box-weighs-under-a-kilo",
            Props::new().set("alignment", "opposing"),
        )
        .unwrap();
        let found = g.detect_defects().unwrap().defects.iter().any(|d| {
            d.category.as_str() == "contradiction"
                && d.affected_ids
                    .contains(&"req:the-box-weighs-under-a-kilo".to_string())
        });
        assert!(
            found,
            "{name}: the accidental contradiction is not reported"
        );
        assert!(
            g.loop_status().unwrap().structural_defects > before,
            "{name}"
        );
    }
}

// ─── D (3): a conditional claim does not read holds ─────────────────────────

/// Every claim of every fixture whose proof_status is `conditional` reads
/// `truth: conditional` (never `holds`), names what it is conditional on in
/// its why, and lists it: an assumption under `assumes`, or, since phase 3
/// (req:a-proof-can-state-a-contradiction-cases-suppositions-and-cited-theorems,
/// item 4), an unverified citation under `rests_on_unverified`.
#[test]
fn a_conditional_claim_never_reads_holds() {
    let mut seen = 0;
    for (name, doc) in ALL {
        let g = load(doc);
        let r = report(&g);
        for c in r["claims"].as_array().unwrap() {
            if c["proof_status"] != "conditional" {
                continue;
            }
            seen += 1;
            assert_eq!(
                c["truth"], "conditional",
                "{name}: {}: {}",
                c["id"], c["why"]
            );
            let mut on: Vec<Json> = c["assumes"].as_array().cloned().unwrap_or_default();
            on.extend(
                c["rests_on_unverified"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
            );
            assert!(!on.is_empty(), "{name}: {}", c["id"]);
            for a in &on {
                assert!(
                    c["why"].as_str().unwrap().contains(a.as_str().unwrap()),
                    "{name}: {}: {}",
                    c["id"],
                    c["why"]
                );
            }
        }
    }
    assert!(seen >= 3, "{seen}");
}

// ─── A (4): the served skills agree ─────────────────────────────────────────

/// capture-intent, brainstorm and the served instructions say the same thing
/// about a Decision concluded by an Inference: yes, when its premises are
/// checkable. On main capture-intent said "Not a Decision".
#[test]
fn the_served_skills_agree_a_decision_may_be_a_chains_conclusion() {
    const RULE: &str = "A Decision may be a chain's conclusion";
    for (name, text) in [
        (
            "capture-intent",
            include_str!("../../../getting-started/skills/capture-intent/SKILL.md"),
        ),
        (
            "brainstorm",
            include_str!("../../../getting-started/skills/brainstorm/SKILL.md"),
        ),
        (
            "AGENTS.md",
            include_str!("../../../getting-started/AGENTS.md"),
        ),
    ] {
        assert!(text.contains(RULE), "{name} does not say: {RULE}");
        assert!(
            !text.contains("**Not a Decision**: a choice explained"),
            "{name} still says an Inference never concludes a Decision"
        );
    }
}

/// The argumentation graph measured on 0.87.0, where detect_defects listed all
/// 7 attacks as contradictions: C→B→A (a settled reinstatement: C refutes B,
/// so B's objection to A is answered), X↔Y (a mutual attack) and P→Q→S→P (an
/// odd cycle), every one an opposing CONTRADICTS between propositions taken
/// as given. Settled attacks are never reported. The undecided ones (the
/// mutual attack and the cycle) read contested in reasoning_report; they are
/// a defect only while they state no reason, which is what an accidental
/// contradiction looks like, and never once they state one.
#[test]
fn settled_attacks_are_not_defects_and_undecided_ones_are_only_when_unargued() {
    fn graph(with_reasons: bool) -> DesignGraph {
        let mut g = DesignGraph::open_in_memory().unwrap();
        for id in ["A", "B", "C", "X", "Y", "P", "Q", "S"] {
            g.upsert_node(
                node::REQUIREMENT,
                &format!("req:{id}"),
                Props::new()
                    .set("name", format!("Proposition {id}"))
                    .set("statement", format!("Proposition {id}."))
                    .set("statement_kind", "proposition")
                    .set("taken_as_given", true),
            )
            .unwrap();
        }
        for (from, to) in [
            ("C", "B"),
            ("B", "A"),
            ("X", "Y"),
            ("Y", "X"),
            ("P", "Q"),
            ("Q", "S"),
            ("S", "P"),
        ] {
            let mut props = Props::new().set("alignment", "opposing");
            if with_reasons {
                props = props.set("evidence", format!("{from} attacks {to}."));
            }
            g.create_edge(
                edge::CONTRADICTS,
                node::REQUIREMENT,
                &format!("req:{from}"),
                node::REQUIREMENT,
                &format!("req:{to}"),
                props,
            )
            .unwrap();
        }
        g
    }
    let reported = |g: &DesignGraph| -> BTreeSet<(String, String)> {
        g.detect_defects()
            .unwrap()
            .defects
            .iter()
            .filter(|d| d.category.as_str() == "contradiction")
            .map(|d| (d.affected_ids[0].clone(), d.affected_ids[1].clone()))
            .collect()
    };
    // Unargued: the settled chain is not reported; the undecided five are.
    let g = graph(false);
    assert_eq!(
        g.premise_truth("req:A").unwrap().0,
        Truth::Holds,
        "reinstated"
    );
    assert_eq!(g.premise_truth("req:B").unwrap().0, Truth::Fails, "refuted");
    let found = reported(&g);
    for settled in [("req:C", "req:B"), ("req:B", "req:A")] {
        assert!(
            !found.contains(&(settled.0.to_string(), settled.1.to_string())),
            "settled {settled:?} reported: {found:?}"
        );
    }
    assert_eq!(
        found.len(),
        5,
        "the mutual attack and the odd cycle, unargued: {found:?}"
    );
    // Argued: nothing is a defect, and the undecided read as still contested.
    let g = graph(true);
    assert!(reported(&g).is_empty(), "{:?}", reported(&g));
    let r = report(&g);
    let contested: BTreeSet<String> = r["still_contested"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["claim"].as_str().unwrap().to_string())
        .collect();
    for id in ["req:X", "req:Y", "req:P", "req:Q", "req:S"] {
        assert!(
            contested.contains(id),
            "{id} is undecided and not read as contested: {contested:?}"
        );
    }
    assert!(!contested.contains("req:A"), "{contested:?}");
}
