//! A PREMISE SAYS WHETHER IT RESTS ON ITS OWNER'S INTENT OR ON ITS DELIVERY.
//!
//! `dec:a-premise-says-whether-it-rests-on-intent-or-delivery` (Anthony,
//! 2026-10-24), from the root cause walked on the Shop sample trial
//! (`fact:root-cause-a-design-choice-read-its-requirements-by-delivery-because-the-premise-truth-reused-the-delivery-line-2026-10-24`):
//! 19 design choices recorded as Decisions concluded by one step each, every
//! premise a requirement, constraint, finding or decision. Every chain read
//! `unknown`, because a requirement premise read only its DELIVERY ("not
//! delivered, and its status (accepted) says neither met nor dropped").
//!
//! A requirement record carries two propositions: "X is required" (true once
//! its owner accepts it) and "X holds in the system" (true once delivered). A
//! premise now says which it rests on, on its PREMISE_OF link (`reads_as`:
//! `intent` or `delivery`), and unmarked:
//!
//! - a premise of a step that CONCLUDES A DECISION reads as INTENT;
//! - every other step keeps DELIVERY, as before.
//!
//! As intent, a Requirement, Capability or Interface reads by its owner's
//! acceptance: accepted holds, proposed unknown, dropped (or rejected) fails.
//! Proofs, arguments and verification arguments read exactly as before:
//! the last test pins every trial fixture's readings to a golden taken on the
//! unfixed code (main 61c05ad74).
//!
//! Every test is a class over the readers that read a premise: the step's
//! verdict, the decision's chain, the uncited-premise list, the reasoning
//! debt ("what has lost its reason now"), `conclusion_does_not_follow`, the
//! foundations, and the what-if walk. The file compiles on main: the marker
//! is drawn as an edge property and every reading is compared as JSON. Each
//! test was observed FAILING on main 61c05ad74 first.

use std::collections::BTreeMap;

use reflow2_core::DesignGraph;
use reflow2_core::export::GraphExport;
use reflow2_core::foundation::core::Value;
use reflow2_core::nodes::Props;
use reflow2_core::propagate::PropagateOptions;
use serde_json::{Value as Json, json};

fn graph() -> DesignGraph {
    DesignGraph::open_in_memory().expect("in-memory graph")
}

fn to_json<T: serde::Serialize>(v: T) -> Json {
    serde_json::to_value(v).expect("serializes")
}

fn type_of(g: &DesignGraph, id: &str) -> String {
    let held = g.node_types_holding(id).expect("types holding");
    assert_eq!(held.len(), 1, "{id}: {held:?}");
    held[0].clone()
}

fn edge(g: &mut DesignGraph, from: &str, rel: &str, to: &str, props: Props) {
    let (ft, tt) = (type_of(g, from), type_of(g, to));
    g.create_edge(rel, &ft, from, &tt, to, props)
        .unwrap_or_else(|e| panic!("{from} {rel} {to}: {e}"));
}

fn requirement(g: &mut DesignGraph, id: &str, status: &str) {
    g.add_requirement(id, &format!("requirement {id}"), "The owner's need.")
        .unwrap();
    g.upsert_node("Requirement", id, Props::new().set("status", status))
        .unwrap();
}

fn decision(g: &mut DesignGraph, id: &str, status: &str) {
    g.upsert_node(
        "Decision",
        id,
        Props::new()
            .set("name", format!("decision {id}"))
            .set("decision", "A choice.")
            .set("status", status),
    )
    .unwrap();
}

fn capability(g: &mut DesignGraph, id: &str, status: &str) {
    g.upsert_node(
        "Capability",
        id,
        Props::new()
            .set("name", format!("capability {id}"))
            .set("description", "What a part does.")
            .set("status", status),
    )
    .unwrap();
}

fn interface(g: &mut DesignGraph, id: &str) {
    g.upsert_node(
        "Interface",
        id,
        Props::new().set("name", format!("interface {id}")),
    )
    .unwrap();
}

/// A step, drawn by hand: each premise with its `reads_as` marker when one
/// is given (`None` leaves it unmarked).
fn step(g: &mut DesignGraph, id: &str, premises: &[(&str, Option<&str>)], conclusion: &str) {
    g.upsert_node(
        "Inference",
        id,
        Props::new().set("name", id).set("rule", "all"),
    )
    .unwrap();
    for (p, reads_as) in premises {
        let props = match reads_as {
            Some(r) => Props::new().set("reads_as", *r),
            None => Props::new(),
        };
        edge(g, p, "PREMISE_OF", id, props);
    }
    edge(g, id, "CONCLUDES", conclusion, Props::new());
}

fn reading(g: &DesignGraph, step: &str) -> Json {
    to_json(g.inference_reading(step).unwrap().expect("the step exists"))
}

fn premise<'a>(r: &'a Json, id: &str) -> &'a Json {
    r["premises"]
        .as_array()
        .expect("premises")
        .iter()
        .find(|p| p["id"] == id)
        .unwrap_or_else(|| panic!("{id} is not a premise in {r}"))
}

fn chain(g: &DesignGraph, decision: &str) -> String {
    g.chain_reading(decision)
        .unwrap()
        .map(|(t, _)| to_json(t).as_str().unwrap().to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn strs(v: &Json) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

// ─── 1: an unmarked premise of a decision's step reads as intent ───────────

/// EVERY STATUS a requirement can carry, as an UNMARKED premise of a step
/// that concludes a Decision: accepted and met hold (the owner accepted the
/// need), proposed and deferred are unknown (not accepted, or set aside),
/// dropped fails. Each says it was read as intent. The step follows; the
/// decision's chain holds with it, and is left unsupported (never refuted)
/// when the step fails. Measured on the Shop sample: an accepted
/// requirement read "not delivered, and its status (accepted) says neither
/// met nor dropped", so every chain read unknown.
#[test]
fn an_unmarked_requirement_premise_of_a_decisions_step_reads_by_its_owners_acceptance() {
    for (status, want) in [
        ("accepted", "holds"),
        ("met", "holds"),
        ("proposed", "unknown"),
        ("deferred", "unknown"),
        ("dropped", "fails"),
    ] {
        let mut g = graph();
        requirement(&mut g, "req:need", status);
        decision(&mut g, "dec:choice", "accepted");
        step(&mut g, "inf:why", &[("req:need", None)], "dec:choice");
        let r = reading(&g, "inf:why");
        let p = premise(&r, "req:need");
        assert_eq!(p["truth"], want, "a {status} requirement as intent: {p}");
        assert_eq!(p["reads_as"], "intent", "{status}: {p}");
        assert!(
            p["why"].as_str().unwrap().contains("intent"),
            "{status}: the reason says it was read as intent: {p}"
        );
        assert_eq!(r["verdict"], want, "{status}: the step follows its premise");
        // A chain whose step fails leaves its decision UNSUPPORTED, never
        // refuted (req:unsupported-is-not-refuted).
        let chain_want = if want == "fails" { "unknown" } else { want };
        assert_eq!(
            chain(&g, "dec:choice"),
            chain_want,
            "{status}: the decision's chain"
        );
    }
}

// ─── 2: marked delivery, the same premise reads as it always has ───────────

/// The same premises MARKED `delivery` read exactly as `premise_truth` reads
/// them, which is today's reading: an accepted requirement nothing delivers
/// is unknown, a met one holds, a dropped one fails. A release or go/no-go
/// decision rests on delivery and says so.
#[test]
fn a_premise_marked_delivery_reads_as_it_always_has() {
    for (status, want) in [
        ("accepted", "unknown"),
        ("met", "holds"),
        ("proposed", "unknown"),
        ("dropped", "fails"),
    ] {
        let mut g = graph();
        requirement(&mut g, "req:need", status);
        decision(&mut g, "dec:ship", "accepted");
        step(
            &mut g,
            "inf:why-ship",
            &[("req:need", Some("delivery"))],
            "dec:ship",
        );
        let r = reading(&g, "inf:why-ship");
        let p = premise(&r, "req:need");
        let (today, why) = g.premise_truth("req:need").unwrap();
        assert_eq!(p["truth"], to_json(today), "{status}: delivery is today's");
        assert_eq!(p["truth"], want, "{status}: {p}");
        assert_eq!(p["why"], why, "{status}: the same reason as today");
        assert_eq!(
            p["reads_as"], "delivery",
            "{status}: the marker is read back"
        );
    }
}

// ─── 3: a step concluding anything but a Decision is unchanged ─────────────

/// A step that concludes a REQUIREMENT (a claim, an assurance argument) keeps
/// delivery when unmarked: its accepted requirement premise reads unknown, as
/// before, and the read-back carries no marker. Marked `intent`, the same
/// premise reads by acceptance: the marker is the premise's, whatever the
/// step concludes.
#[test]
fn a_step_concluding_a_claim_keeps_delivery_unless_a_premise_says_intent() {
    let mut g = graph();
    requirement(&mut g, "req:need", "accepted");
    requirement(&mut g, "req:claim", "proposed");
    step(&mut g, "inf:claim", &[("req:need", None)], "req:claim");
    let r = reading(&g, "inf:claim");
    let p = premise(&r, "req:need");
    assert_eq!(p["truth"], "unknown", "delivery, as before: {p}");
    assert!(
        p.get("reads_as").is_none(),
        "unmarked delivery says nothing new: {p}"
    );
    assert_eq!(r["verdict"], "unknown");

    let mut g = graph();
    requirement(&mut g, "req:need", "accepted");
    requirement(&mut g, "req:claim", "proposed");
    step(
        &mut g,
        "inf:claim",
        &[("req:need", Some("intent"))],
        "req:claim",
    );
    let p = premise(&reading(&g, "inf:claim"), "req:need").clone();
    assert_eq!(p["truth"], "holds", "marked intent: {p}");
    assert_eq!(p["reads_as"], "intent");
}

// ─── 4: a capability and an interface read by the acceptance behind them ───

/// A CAPABILITY has no acceptance status of its own (its status is how far it
/// is BUILT), so as intent it reads by the need it serves: it holds when it
/// SATISFIES an accepted (or met) requirement, is unknown when every need it
/// serves is only proposed (or it serves none), and fails when every need it
/// serves is dropped. Its build status never enters: a planned capability
/// serving an accepted need holds as intent.
#[test]
fn a_capability_premise_as_intent_reads_by_the_acceptance_of_the_need_it_serves() {
    for (need_status, want) in [
        (Some("accepted"), "holds"),
        (Some("met"), "holds"),
        (Some("proposed"), "unknown"),
        (Some("dropped"), "fails"),
        (None, "unknown"),
    ] {
        let mut g = graph();
        capability(&mut g, "cap:part-does-it", "planned");
        if let Some(s) = need_status {
            requirement(&mut g, "req:need", s);
            edge(
                &mut g,
                "cap:part-does-it",
                "SATISFIES",
                "req:need",
                Props::new(),
            );
        }
        decision(&mut g, "dec:choice", "accepted");
        step(
            &mut g,
            "inf:why",
            &[("cap:part-does-it", None)],
            "dec:choice",
        );
        let p = premise(&reading(&g, "inf:why"), "cap:part-does-it").clone();
        assert_eq!(p["truth"], want, "need {need_status:?}: {p}");
        assert_eq!(p["reads_as"], "intent");
        // Delivery, marked, is today's: not realized and checked.
        let mut g2 = graph();
        capability(&mut g2, "cap:part-does-it", "planned");
        decision(&mut g2, "dec:ship", "accepted");
        step(
            &mut g2,
            "inf:ship",
            &[("cap:part-does-it", Some("delivery"))],
            "dec:ship",
        );
        assert_eq!(
            premise(&reading(&g2, "inf:ship"), "cap:part-does-it")["truth"],
            "unknown"
        );
    }
}

/// An INTERFACE has no status at all, so as intent it reads by the Decision
/// that governs it: it holds when an accepted Decision governs it, is
/// unknown when none does, and fails when every Decision governing it was
/// rejected or superseded. A Decision cannot lend acceptance to a premise of
/// the step that concludes it: an interface governed only by the decision
/// its own step concludes reads unknown and says the support would be
/// circular.
#[test]
fn an_interface_premise_as_intent_reads_by_the_decision_that_governs_it() {
    for (governor, want) in [
        (Some("accepted"), "holds"),
        (Some("rejected"), "fails"),
        (Some("superseded"), "fails"),
        (Some("proposed"), "unknown"),
        (None, "unknown"),
    ] {
        let mut g = graph();
        interface(&mut g, "ifc:join");
        if let Some(s) = governor {
            decision(&mut g, "dec:the-joins-exist", s);
            edge(
                &mut g,
                "ifc:join",
                "GOVERNED_BY",
                "dec:the-joins-exist",
                Props::new(),
            );
        }
        decision(&mut g, "dec:encode-the-joins", "accepted");
        step(
            &mut g,
            "inf:why",
            &[("ifc:join", None)],
            "dec:encode-the-joins",
        );
        let p = premise(&reading(&g, "inf:why"), "ifc:join").clone();
        assert_eq!(p["truth"], want, "governor {governor:?}: {p}");
        assert_eq!(p["reads_as"], "intent");
    }

    // Governed only by the decision the step concludes: circular.
    let mut g = graph();
    interface(&mut g, "ifc:join");
    decision(&mut g, "dec:encode-the-joins", "accepted");
    edge(
        &mut g,
        "ifc:join",
        "GOVERNED_BY",
        "dec:encode-the-joins",
        Props::new(),
    );
    step(
        &mut g,
        "inf:why",
        &[("ifc:join", None)],
        "dec:encode-the-joins",
    );
    let p = premise(&reading(&g, "inf:why"), "ifc:join").clone();
    assert_eq!(p["truth"], "unknown", "{p}");
    assert!(p["why"].as_str().unwrap().contains("circular"), "{p}");
}

/// A REQUIREMENT that exists because of the very decision the step
/// concludes (GOVERNED_BY it) cannot be a reason for that decision: as
/// intent it reads unknown and says the support would be circular, however
/// accepted it is.
#[test]
fn a_requirement_governed_by_the_decision_its_step_concludes_is_no_reason_for_it() {
    let mut g = graph();
    requirement(&mut g, "req:made-by-the-choice", "accepted");
    decision(&mut g, "dec:choice", "accepted");
    edge(
        &mut g,
        "req:made-by-the-choice",
        "GOVERNED_BY",
        "dec:choice",
        Props::new(),
    );
    step(
        &mut g,
        "inf:why",
        &[("req:made-by-the-choice", None)],
        "dec:choice",
    );
    let p = premise(&reading(&g, "inf:why"), "req:made-by-the-choice").clone();
    assert_eq!(p["truth"], "unknown", "{p}");
    assert!(p["why"].as_str().unwrap().contains("circular"), "{p}");
}

// ─── 5: statements read as themselves, whatever the step concludes ─────────

/// A STATEMENT — a requirement with a `statement_kind`, or one taken as
/// given — is never a need: as a premise of a decision's step it reads as
/// it always has, and carries no marker. A proposition nobody has shown stays
/// unknown; a definition taken as given holds as a stated foundation.
#[test]
fn a_statement_premise_of_a_decisions_step_reads_as_itself() {
    let mut g = graph();
    for (id, kind, given) in [
        ("req:a-proposition", "proposition", false),
        ("req:a-definition", "definition", true),
        ("req:a-piece-of-evidence", "evidence", false),
    ] {
        g.add_requirement(id, &format!("statement {id}"), "A statement.")
            .unwrap();
        let mut props = Props::new()
            .set("statement_kind", kind)
            .set("status", "accepted");
        if given {
            props = props.set("taken_as_given", Value::Bool(true));
        }
        g.upsert_node("Requirement", id, props).unwrap();
    }
    decision(&mut g, "dec:choice", "accepted");
    step(
        &mut g,
        "inf:why",
        &[
            ("req:a-proposition", None),
            ("req:a-definition", None),
            ("req:a-piece-of-evidence", None),
        ],
        "dec:choice",
    );
    let r = reading(&g, "inf:why");
    for id in [
        "req:a-proposition",
        "req:a-definition",
        "req:a-piece-of-evidence",
    ] {
        let p = premise(&r, id);
        let (today, why) = g.premise_truth(id).unwrap();
        assert_eq!(p["truth"], to_json(today), "{id}: {p}");
        assert_eq!(p["why"], why, "{id}");
        assert!(
            p.get("reads_as").is_none(),
            "{id}: a statement has no marker: {p}"
        );
    }
}

// ─── 6: the readers that read a premise agree ──────────────────────────────

/// The Shop sample's shape: an accepted decision concluded by a step over an
/// accepted requirement, a planned capability serving it, an accepted
/// decision and a constraint. Every reader says the same thing:
/// the chain holds, the decision is NOT listed as having lost its reason,
/// no `conclusion_does_not_follow` is raised for it, its proof status is no
/// longer `unproven`, and the requirement and the capability are listed as
/// held on their owner's word, never as uncited premises.
#[test]
fn every_reader_of_a_design_choice_agrees_its_chain_holds_on_accepted_intent() {
    let mut g = graph();
    requirement(&mut g, "req:reusable-medium", "accepted");
    capability(&mut g, "cap:beds-hold-a-medium", "planned");
    edge(
        &mut g,
        "cap:beds-hold-a-medium",
        "SATISFIES",
        "req:reusable-medium",
        Props::new(),
    );
    decision(&mut g, "dec:aquaponics", "accepted");
    decision(&mut g, "dec:clay-and-coir", "accepted");
    step(
        &mut g,
        "inf:why-clay-and-coir",
        &[
            ("req:reusable-medium", None),
            ("cap:beds-hold-a-medium", None),
            ("dec:aquaponics", None),
        ],
        "dec:clay-and-coir",
    );

    assert_eq!(chain(&g, "dec:clay-and-coir"), "holds");
    let debt = to_json(
        g.reasoning_debt()
            .unwrap()
            .expect("a design with reasoning"),
    );
    assert!(
        !strs(&debt["decision_without_a_holding_chain_ids"])
            .contains(&"dec:clay-and-coir".to_string()),
        "not listed as having lost its reason: {debt}"
    );
    assert!(
        !strs(&debt["uncited_premise_ids"]).contains(&"req:reusable-medium".to_string()),
        "a premise held on its owner's word is not uncited: {debt}"
    );
    assert!(
        strs(&debt["held_on_the_owners_word_ids"]).contains(&"req:reusable-medium".to_string())
            && strs(&debt["held_on_the_owners_word_ids"])
                .contains(&"cap:beds-hold-a-medium".to_string()),
        "listed as held on its owner's word: {debt}"
    );
    let unfollowed = to_json(g.conclusions_that_do_not_follow().unwrap());
    assert!(
        !unfollowed.to_string().contains("dec:clay-and-coir"),
        "no conclusion_does_not_follow for it: {unfollowed}"
    );
    let proof = to_json(g.proof_reading("dec:clay-and-coir").unwrap());
    assert_ne!(proof["status"], "unproven", "{proof}");

    let report = to_json(
        g.reasoning_report(Some("dec:clay-and-coir"), Some(500))
            .unwrap(),
    );
    assert!(
        !strs(&report["uncited_premises"]).contains(&"req:reusable-medium".to_string()),
        "{}",
        report["uncited_premises"]
    );
    assert!(
        strs(&report["held_on_the_owners_word"]).contains(&"req:reusable-medium".to_string()),
        "{}",
        report
    );
    let unsupported: Vec<String> = report["unsupported"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        !unsupported.contains(&"req:reusable-medium".to_string()),
        "an accepted need a choice rests on is not unsupported: {unsupported:?}"
    );
    // The foundations read each premise as its step reads it.
    let foundations = to_json(g.foundations_of("dec:clay-and-coir").unwrap());
    let f = foundations
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "req:reusable-medium")
        .expect("a foundation");
    assert_eq!(f["truth"], "holds", "{f}");
}

/// The same choice over a PROPOSED requirement still reads unknown, and the
/// readers still say so: nobody accepted that need, so nothing about the
/// choice is established by it. Intent is the owner's word, never the
/// record's existence.
#[test]
fn a_design_choice_over_a_proposed_need_still_reads_unsupported_everywhere() {
    let mut g = graph();
    requirement(&mut g, "req:alexs-brief", "proposed");
    decision(&mut g, "dec:choice", "accepted");
    step(
        &mut g,
        "inf:why",
        &[("req:alexs-brief", None)],
        "dec:choice",
    );
    assert_eq!(chain(&g, "dec:choice"), "unknown");
    let debt = to_json(g.reasoning_debt().unwrap().unwrap());
    assert!(
        strs(&debt["decision_without_a_holding_chain_ids"]).contains(&"dec:choice".to_string()),
        "{debt}"
    );
    // Named as what awaits the owner's word, never as an uncited premise:
    // what it lacks is acceptance, not a source.
    assert_eq!(
        strs(&debt["awaiting_the_owners_word_ids"]),
        vec!["req:alexs-brief"],
        "{debt}"
    );
    assert!(!strs(&debt["uncited_premise_ids"]).contains(&"req:alexs-brief".to_string()));
    let next = to_json(g.loop_status().unwrap())["next"].to_string();
    assert!(
        next.contains("WHAT AWAITS THE OWNER'S WORD") && next.contains("req:alexs-brief"),
        "{next}"
    );
}

// ─── 7: a what-if and a dropped need reach the choice ──────────────────────

/// A choice that rests on accepted intent loses that support when the need
/// is DROPPED (its intent now fails, so the step fails and the chain does
/// not hold) and when a what-if takes the need away (the walk reports the
/// step broken): the change walk reads the intent premise, not its delivery.
#[test]
fn dropping_the_accepted_need_or_a_what_if_on_it_reaches_the_choice() {
    let mut g = graph();
    requirement(&mut g, "req:need", "accepted");
    decision(&mut g, "dec:choice", "accepted");
    step(&mut g, "inf:why", &[("req:need", None)], "dec:choice");
    assert_eq!(chain(&g, "dec:choice"), "holds");

    let w = to_json(
        g.propagate_from(&["req:need"], PropagateOptions::default())
            .unwrap()
            .summarize(),
    );
    let rows = w["inferences"].as_array().cloned().unwrap_or_default();
    let row = rows
        .iter()
        .find(|r| r["inference"] == "inf:why")
        .unwrap_or_else(|| panic!("the what-if reaches the step: {w}"));
    assert_eq!(row["before"], "holds", "it held before the what-if: {row}");
    assert_ne!(row["after"], "holds", "{row}");

    g.upsert_node(
        "Requirement",
        "req:need",
        Props::new().set("status", "dropped"),
    )
    .unwrap();
    assert_ne!(chain(&g, "dec:choice"), "holds");
    assert_eq!(
        premise(&reading(&g, "inf:why"), "req:need")["truth"],
        "fails"
    );
}

// ─── 8: proofs, arguments and verification arguments read as before ───────

pub(crate) const FIXTURES: [(&str, &str); 17] = [
    (
        "capstone",
        include_str!("fixtures/argument_trial_capstone.json"),
    ),
    (
        "cogito",
        include_str!("fixtures/argument_trial_cogito.json"),
    ),
    (
        "gettier",
        include_str!("fixtures/argument_trial_gettier.json"),
    ),
    (
        "lucas-penrose",
        include_str!("fixtures/argument_trial_lucas_penrose.json"),
    ),
    (
        "trolley",
        include_str!("fixtures/argument_trial_trolley.json"),
    ),
    (
        "poc-case-1",
        include_str!("fixtures/poc_case_1_a_decision_concluded_by_its_chain.json"),
    ),
    (
        "navier-stokes",
        include_str!("fixtures/proof_map_navier_stokes.json"),
    ),
    ("riemann", include_str!("fixtures/proof_map_riemann.json")),
    ("arrow", include_str!("fixtures/proof_trial_arrow.json")),
    ("brocard", include_str!("fixtures/proof_trial_brocard.json")),
    (
        "four-colour",
        include_str!("fixtures/proof_trial_four_colour.json"),
    ),
    (
        "heat",
        include_str!("fixtures/proof_trial_heat_equation.json"),
    ),
    (
        "logic",
        include_str!("fixtures/proof_trial_logic_rung.json"),
    ),
    ("pi+e", include_str!("fixtures/proof_trial_pi_plus_e.json")),
    (
        "no-inference",
        include_str!("fixtures/reports_without_an_inference.json"),
    ),
    (
        "rung-1",
        include_str!("../../reflow2-mcp/tests/fixtures/proof_trial_rung_1_primes.json"),
    ),
    (
        "rung-2",
        include_str!("../../reflow2-mcp/tests/fixtures/proof_trial_rung_2_fta.json"),
    ),
];

const GOLDEN: &str = "tests/fixtures/premise_readings_before_intent_or_delivery.json";

/// What a design's reasoning READS, projected so a wording change elsewhere
/// cannot move it: every step's verdict and each premise's truth, every
/// claim's truth and proof status, the lists of what the reasoning rests on,
/// cites nothing, leaves unsupported or takes as given, the debt's counts,
/// and the conclusions that do not follow.
fn readings_of(doc: &str) -> Json {
    let doc: GraphExport = serde_json::from_str(doc).expect("fixture parses");
    let mut g = graph();
    g.import_graph(&doc).expect("fixture imports");
    let mut steps: BTreeMap<String, Json> = BTreeMap::new();
    for s in g.scan_nodes("Inference").unwrap() {
        let r = reading(&g, &s.node_id);
        let premises: BTreeMap<String, Json> = r["premises"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                (
                    p["id"].as_str().unwrap().to_string(),
                    json!([p["truth"], p["role"], p["reads_as"]]),
                )
            })
            .collect();
        steps.insert(
            s.node_id,
            json!({"verdict": r["verdict"], "premises": premises}),
        );
    }
    let report = to_json(g.reasoning_report(None, Some(500)).unwrap());
    let claims: BTreeMap<String, Json> = report["claims"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["id"].as_str().unwrap().to_string(),
                json!([c["truth"], c["proof_status"]]),
            )
        })
        .collect();
    let ids = |v: &Json| -> Vec<String> {
        let mut out: Vec<String> = v
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|x| match x.as_str() {
                        Some(s) => s.to_string(),
                        None => x["id"].as_str().unwrap_or_default().to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.sort();
        out
    };
    let debt = g.reasoning_debt().unwrap().map(to_json).map(|d| {
        json!([
            d["steps"],
            d["unconfirmed_steps"],
            d["uncited_premises"],
            d["shown_not_proven"],
            d["stated_foundations"],
            d["undischarged_assumptions"],
            d["reasons_lost"],
            d["decisions_without_a_holding_chain"],
            d["stale_checks"],
        ])
    });
    let unfollowed: Vec<String> = to_json(g.conclusions_that_do_not_follow().unwrap())
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u.to_string())
        .collect();
    json!({
        "steps": steps,
        "claims": claims,
        "rests_on": ids(&report["rests_on"]),
        "uncited_premises": ids(&report["uncited_premises"]),
        "stated_foundations": ids(&report["stated_foundations"]),
        "unsupported": ids(&report["unsupported"]),
        "debt": debt,
        "unfollowed": unfollowed,
    })
}

/// EVERY PROOF, ARGUMENT AND VERIFICATION ARGUMENT the trials recorded reads
/// exactly as it did before a premise could say intent or delivery: the
/// readings of all 17 fixtures equal a golden taken on the unfixed code
/// (main 61c05ad74). None of them concludes a Decision with a need as a
/// premise (the capstone's decisions rest on propositions, which are
/// statements), so the default changes nothing there; this pins it.
/// `REFLOW2_BLESS_PREMISE_GOLDEN=1` rewrites the golden, which is only ever
/// right on the unfixed code.
#[test]
fn every_proof_and_argument_trial_reads_exactly_as_before() {
    let now: BTreeMap<&str, Json> = FIXTURES
        .iter()
        .map(|(name, doc)| (*name, readings_of(doc)))
        .collect();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(GOLDEN);
    if std::env::var("REFLOW2_BLESS_PREMISE_GOLDEN").is_ok() {
        std::fs::write(&path, serde_json::to_string_pretty(&now).unwrap() + "\n").unwrap();
        return;
    }
    let golden: BTreeMap<String, Json> =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("the golden exists"))
            .expect("the golden parses");
    assert_eq!(
        golden.len(),
        FIXTURES.len(),
        "every fixture is in the golden"
    );
    for (name, reads) in &now {
        let was = &golden[*name];
        if reads != was {
            for key in [
                "steps",
                "claims",
                "rests_on",
                "uncited_premises",
                "stated_foundations",
                "unsupported",
                "debt",
                "unfollowed",
            ] {
                assert_eq!(reads[key], was[key], "{name}: `{key}` moved");
            }
            panic!("{name}: the readings moved");
        }
    }
}

// ─── 9: add_inference writes the marker, per premise ───────────────────────

fn spec(id: &str, premises: &[&str], conclusion: &str) -> reflow2_core::inference::InferenceSpec {
    reflow2_core::inference::InferenceSpec {
        id: id.to_string(),
        rule: "all".to_string(),
        premises: premises.iter().map(|p| p.to_string()).collect(),
        conclusion: conclusion.to_string(),
        ..Default::default()
    }
}

fn marks(g: &DesignGraph, step: &str) -> BTreeMap<String, Option<String>> {
    g.incoming(step, Some("PREMISE_OF"))
        .unwrap()
        .into_iter()
        .map(|e| {
            (
                e.from_id,
                e.properties
                    .get("reads_as")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            )
        })
        .collect()
}

/// `add_inference` takes `as_intent` and `as_delivery`, each naming premises
/// of the step, and writes `reads_as` on each link; a revise marks a link
/// already drawn, keeping what else it carried and drawing no second link.
/// It refuses, before anything is written, an id that is not a premise of the
/// step, an assumed premise, a type that reads as itself, and a premise named
/// in both.
#[test]
fn add_inference_marks_each_premise_and_refuses_a_marker_that_cannot_apply() {
    let mut g = graph();
    requirement(&mut g, "req:need", "accepted");
    requirement(&mut g, "req:shipped", "met");
    capability(&mut g, "cap:does-it", "planned");
    decision(&mut g, "dec:ship", "accepted");
    let mut s = spec("inf:why-ship", &["req:need", "req:shipped"], "dec:ship");
    s.as_delivery = vec!["req:shipped".to_string()];
    g.add_inference(&s).expect("marks on a new step");
    let m = marks(&g, "inf:why-ship");
    assert_eq!(m["req:shipped"].as_deref(), Some("delivery"));
    assert_eq!(m["req:need"], None, "unmarked stays unmarked");
    let r = reading(&g, "inf:why-ship");
    assert_eq!(premise(&r, "req:need")["reads_as"], "intent");
    assert_eq!(premise(&r, "req:shipped")["reads_as"], "delivery");

    // A revise marks a link already drawn: one link, its mark written.
    let mut s = spec("inf:why-ship", &[], "dec:ship");
    s.as_delivery = vec!["req:need".to_string()];
    g.add_inference(&s)
        .expect("a revise marks an existing premise");
    let links: Vec<_> = g
        .incoming("inf:why-ship", Some("PREMISE_OF"))
        .unwrap()
        .into_iter()
        .filter(|e| e.from_id == "req:need")
        .collect();
    assert_eq!(links.len(), 1, "no second link: {links:?}");
    assert_eq!(
        marks(&g, "inf:why-ship")["req:need"].as_deref(),
        Some("delivery")
    );
    assert_eq!(
        premise(&reading(&g, "inf:why-ship"), "req:need")["truth"],
        "unknown",
        "delivery: an accepted need nothing delivers"
    );

    // Refusals, nothing written.
    g.add_requirement("req:a-proposition", "statement", "A statement.")
        .unwrap();
    g.upsert_node(
        "Requirement",
        "req:a-proposition",
        Props::new().set("statement_kind", "proposition"),
    )
    .unwrap();
    for (premises, assumed, intent, delivery, says) in [
        (
            vec!["req:need"],
            vec![],
            vec!["cap:does-it"],
            vec![],
            "not a premise",
        ),
        (
            vec!["req:need"],
            vec!["cap:does-it"],
            vec!["cap:does-it"],
            vec![],
            "assumed",
        ),
        (
            vec!["req:a-proposition"],
            vec![],
            vec!["req:a-proposition"],
            vec![],
            "reads as itself",
        ),
        (
            vec!["req:need"],
            vec![],
            vec!["req:need"],
            vec!["req:need"],
            "both",
        ),
    ] {
        let mut s = spec("inf:refused", &premises, "dec:ship");
        s.assumed = assumed.iter().map(|x: &&str| x.to_string()).collect();
        s.as_intent = intent.iter().map(|x: &&str| x.to_string()).collect();
        s.as_delivery = delivery.iter().map(|x: &&str| x.to_string()).collect();
        let e = g.add_inference(&s).expect_err(says).to_string();
        assert!(e.contains(says), "{says}: {e}");
        assert!(
            g.get_node("Inference", "inf:refused").unwrap().is_none(),
            "{says}: nothing written"
        );
    }
}
