//! HOW AN ARGUMENT READS — the second PR of phase 6
//! (`epoch:planned-p6-argument-gaps`), on Anthony's word of 2026-10-20.
//!
//! - **`req:a-conclusion-whose-every-route-is-contested-reads-contested`**: a
//!   claim no route holds, but which a route would hold were the disputes
//!   under it settled its way, reads `contested` and names what stands
//!   against each such route — never `unproven`, the word that also covers a
//!   claim nothing supports. A foundation taken as given under a standing
//!   objection reads contested, and so does what rests on it; the read-back's
//!   tally counts them.
//! - **`req:the-read-back-under-a-claim-shows-a-dispute-kept-open-and-the-owners-chosen-reading`**:
//!   a `contested` Decision governing a claim's steps is read with the claim;
//!   the side it holds (GOVERNED_BY `ruling: held`) reads `held`, the others
//!   `rival`; read on the Decision, the report lists its `positions`.
//! - **`req:a-reply-can-concede-an-objection-against-one-reading-and-move-to-another`**:
//!   a CONTRADICTS marked `alignment: conceding`, with `moves_to`.
//! - **`req:statement-kinds-cover-an-objection-a-reply-an-unstated-premise-and-a-method-rule`**.
//! - **`req:a-step-can-say-it-is-not-an-inference`**: rule `immediate`.
//! - **`req:what-a-claim-rests-on-excludes-the-premises-of-objections-against-it`**.
//! - **`req:a-line-of-reasoning-is-held-in-its-project-and-its-gaps-fit-an-argument`**.
//!
//! Every test is a CLASS over every fixture that can carry it — the two
//! argument trials and the five proof trials — plus the measured shape built by
//! hand. Read as JSON through the generic write path, so the file compiles on
//! main fc5d9f8; each test FAILED there first.

use std::collections::{BTreeMap, BTreeSet};

use reflow2_core::DesignGraph;
use reflow2_core::export::GraphExport;
use reflow2_core::foundation::core::Value;
use reflow2_core::nodes::Props;
use serde_json::Value as Json;

const TROLLEY: &str = include_str!("fixtures/argument_trial_trolley.json");
const COGITO: &str = include_str!("fixtures/argument_trial_cogito.json");
const PI_E: &str = include_str!("fixtures/proof_trial_pi_plus_e.json");
const ARROW: &str = include_str!("fixtures/proof_trial_arrow.json");
const BROCARD: &str = include_str!("fixtures/proof_trial_brocard.json");
const RUNG_1: &str =
    include_str!("../../reflow2-mcp/tests/fixtures/proof_trial_rung_1_primes.json");
const RUNG_2: &str = include_str!("../../reflow2-mcp/tests/fixtures/proof_trial_rung_2_fta.json");

const ALL: [(&str, &str); 7] = [
    ("trolley", TROLLEY),
    ("cogito", COGITO),
    ("pi+e", PI_E),
    ("arrow", ARROW),
    ("brocard", BROCARD),
    ("rung-1", RUNG_1),
    ("rung-2", RUNG_2),
];

fn load(doc: &str) -> DesignGraph {
    let doc: GraphExport = serde_json::from_str(doc).expect("fixture parses");
    let mut g = DesignGraph::open_in_memory().expect("in-memory graph");
    g.import_graph(&doc).expect("fixture imports");
    g
}

fn graph() -> DesignGraph {
    DesignGraph::open_in_memory().expect("in-memory graph")
}

fn json<T: serde::Serialize>(v: T) -> Json {
    serde_json::to_value(v).expect("serializes")
}

fn type_of(g: &DesignGraph, id: &str) -> String {
    let held = g.node_types_holding(id).expect("types holding");
    assert_eq!(held.len(), 1, "{id}: {held:?}");
    held[0].clone()
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

fn edge(g: &mut DesignGraph, from: &str, rel: &str, to: &str, props: Props) {
    let (ft, tt) = (type_of(g, from), type_of(g, to));
    g.create_edge(rel, &ft, from, &tt, to, props)
        .unwrap_or_else(|e| panic!("{from} {rel} {to}: {e}"));
}

fn objects(g: &mut DesignGraph, from: &str, to: &str) {
    edge(
        g,
        from,
        "CONTRADICTS",
        to,
        Props::new()
            .set("alignment", "opposing")
            .set("evidence", "an objection"),
    );
}

/// A statement: taken as given (it holds) or not (a `req:` one is unknown on
/// its own), with a kind.
fn statement(g: &mut DesignGraph, id: &str, kind: &str, given: bool) {
    g.add_requirement(id, &format!("statement {id}"), "A statement.")
        .unwrap();
    let mut props = Props::new().set("statement_kind", kind);
    if given {
        props = props.set("taken_as_given", Value::Bool(true));
    }
    g.upsert_node("Requirement", id, props).unwrap();
}

fn step(g: &mut DesignGraph, id: &str, rule: &str, premises: &[&str], conclusion: &str) {
    g.upsert_node(
        "Inference",
        id,
        Props::new().set("name", id).set("rule", rule),
    )
    .unwrap();
    for p in premises {
        edge(g, p, "PREMISE_OF", id, Props::new());
    }
    edge(g, id, "CONCLUDES", conclusion, Props::new());
}

fn reading(g: &DesignGraph, step: &str) -> Json {
    json(g.inference_reading(step).unwrap().expect("a step"))
}

fn report(g: &DesignGraph, id: Option<&str>) -> Json {
    json(g.reasoning_report(id, Some(500)).unwrap())
}

fn claim(g: &DesignGraph, id: &str) -> Json {
    report(g, Some(id))["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .cloned()
        .unwrap_or_else(|| panic!("{id} not in its own read-back"))
}

fn proof_status(g: &DesignGraph, id: &str) -> Json {
    json(g.proof_reading(id).unwrap())["status"].clone()
}

fn statement_reads(g: &DesignGraph, id: &str) -> Json {
    json(g.statement_reading(id).unwrap())["reads"].clone()
}

fn concluded(g: &DesignGraph) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for st in g.scan_nodes("Inference").unwrap() {
        if let Some(c) = g.conclusion_of(&st.node_id).unwrap() {
            out.insert(c);
        }
    }
    out
}

// ─── 1. Contested, not unproven ──────────────────────────────────────────────

/// THE CLASS, over every claim a step concludes in every fixture: a claim no
/// route holds reads `contested` exactly when one of its routes is held up
/// only by a dispute (a step whose `contested_by` names what stands), and
/// then the read-back's tally (`still_contested`, or the dispute it is kept
/// open by) counts it; a step is such a route when it does not hold and every
/// premise keeping it from holding is itself contested. On main the cogito's
/// "I exist" read `unproven` while its readings were only contested, and no
/// step said what held it up.
#[test]
fn a_claim_held_up_only_by_disputes_reads_contested() {
    let mut contested = 0;
    for (name, doc) in ALL {
        let g = load(doc);
        let whole = report(&g, None);
        let tallied: BTreeSet<String> = whole["still_contested"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|c| c["claim"].as_str().map(str::to_string))
            .collect();
        for c in concluded(&g) {
            let steps = g.inferences_concluding(&c).unwrap();
            let readings: Vec<Json> = steps.iter().map(|s| reading(&g, s)).collect();
            // The oracle for a contested route, from the premises' own
            // readings: it does not hold, nothing under it fails or refutes,
            // and every premise that is not holding is itself contested.
            for (s, r) in steps.iter().zip(&readings) {
                if r["verdict"] != "unknown"
                    || r.get("note")
                        .is_some_and(|n| !n.is_null() && r["objected"].is_null())
                {
                    continue;
                }
                let held_up = r["premises"].as_array().unwrap().iter().all(|p| {
                    p["truth"] == "holds"
                        || (p["truth"] == "unknown"
                            && p["why"]
                                .as_str()
                                .unwrap_or_default()
                                .starts_with("contested"))
                });
                if held_up && r["objected"] != "defeated" {
                    assert!(
                        !strs(&r["contested_by"]).is_empty(),
                        "{name}: {s} is held up only by disputes and does not say which: {r:#}"
                    );
                }
            }
            let any_holds = readings.iter().any(|r| r["verdict"] == "holds");
            let routes_contested = readings
                .iter()
                .any(|r| !strs(&r["contested_by"]).is_empty());
            let status = proof_status(&g, &c);
            if !any_holds
                && routes_contested
                && g.premise_truth(&c).unwrap().0.as_str() == "unknown"
            {
                contested += 1;
                assert_eq!(status, "contested", "{name}: {c}");
                let cl = claim(&g, &c);
                assert!(
                    tallied.contains(&c) || !cl["contested_by_design"].is_null(),
                    "{name}: the read-back's tally does not count {c}"
                );
            }
            if status == "unproven" {
                assert!(
                    !routes_contested,
                    "{name}: {c} reads unproven although a route is contested"
                );
            }
        }
    }
    assert!(contested >= 2, "the class is not vacuous: {contested}");
}

/// THE MEASURED SHAPE, the cogito's: "I exist" is contested, naming each
/// contested reading and what stands against it, and "I think" — whose one
/// route rests on a premise an objection contests — reads contested too.
#[test]
fn the_cogitos_conclusion_reads_contested_not_unproven() {
    let g = load(COGITO);
    for id in ["req:i-exist", "req:i-am-thinking"] {
        let p = json(g.proof_reading(id).unwrap());
        assert_eq!(p["status"], "contested", "{id}: {p:#}");
        assert!(
            p["why"]
                .as_str()
                .unwrap()
                .contains("req:obj-only-thinking-is-certain"),
            "{id} names what stands against its route: {p:#}"
        );
    }
}

/// A FOUNDATION TAKEN AS GIVEN UNDER A STANDING OBJECTION reads contested,
/// and so does what rests on it (the stage-5 re-read's Premises 8 and 9).
#[test]
fn a_given_foundation_under_a_standing_objection_reads_contested_and_so_does_what_rests_on_it() {
    let mut g = graph();
    statement(&mut g, "req:premise-8", "proposition", true);
    statement(&mut g, "req:objection-to-8", "objection", false);
    statement(&mut g, "req:fragments-reading", "proposition", false);
    step(
        &mut g,
        "inf:from-8",
        "all",
        &["req:premise-8"],
        "req:fragments-reading",
    );
    objects(&mut g, "req:objection-to-8", "req:premise-8");
    assert_eq!(statement_reads(&g, "req:premise-8"), "contested");
    assert_eq!(proof_status(&g, "req:fragments-reading"), "contested");
    assert_eq!(
        strs(&reading(&g, "inf:from-8")["contested_by"]),
        vec!["req:objection-to-8".to_string()]
    );
    let whole = report(&g, None);
    let tallied: Vec<String> = whole["still_contested"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["claim"].as_str().map(str::to_string))
        .collect();
    assert!(
        tallied.contains(&"req:fragments-reading".to_string()),
        "{whole:#}"
    );
    // A claim nothing supports stays unproven, whatever objects to it.
    statement(&mut g, "req:bare", "proposition", false);
    statement(&mut g, "req:unsupported-premise", "proposition", false);
    step(
        &mut g,
        "inf:bare",
        "all",
        &["req:unsupported-premise"],
        "req:bare",
    );
    assert_eq!(proof_status(&g, "req:bare"), "unproven");
}

// ─── 2. A dispute kept open, read both ways ──────────────────────────────────

/// THE CLASS, over every step a `contested` Decision governs in every
/// fixture: the read-back of the claim it concludes lists that Decision
/// under `contested_by_design` and on the claim; and read on each contested
/// Decision, the report lists every side it governs under `positions`.
/// On main the cogito's "I exist" read `contested_by_design: []`.
#[test]
fn a_dispute_kept_open_is_read_from_the_claim_and_from_the_question() {
    let mut seen = 0;
    for (name, doc) in ALL {
        let g = load(doc);
        for d in g.scan_nodes("Decision").unwrap() {
            if d.properties.get("status").and_then(Value::as_str) != Some("contested") {
                continue;
            }
            let sides: BTreeSet<String> = g
                .incoming(&d.node_id, Some("GOVERNED_BY"))
                .unwrap()
                .into_iter()
                .map(|e| e.from_id)
                .collect();
            let rep = report(&g, Some(&d.node_id));
            let positions: BTreeSet<String> = rep["positions"]
                .as_array()
                .unwrap_or_else(|| panic!("{name}: {} lists no positions: {rep:#}", d.node_id))
                .iter()
                .filter_map(|p| p["id"].as_str().map(str::to_string))
                .collect();
            assert_eq!(positions, sides, "{name}: {}", d.node_id);
            for side in &sides {
                if type_of(&g, side) != "Inference" {
                    continue;
                }
                let Some(c) = g.conclusion_of(side).unwrap() else {
                    continue;
                };
                seen += 1;
                let rep = report(&g, Some(&c));
                let held: Vec<String> = rep["contested_by_design"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|h| h["decision"].as_str().map(str::to_string))
                    .collect();
                assert!(held.contains(&d.node_id), "{name}: {c}: {rep:#}");
                assert_eq!(claim(&g, &c)["contested_by_design"], d.node_id.as_str());
            }
        }
    }
    assert!(seen >= 3, "the class is not vacuous: {seen}");
}

/// THE OWNER'S CHOSEN READING: marking reading B held (GOVERNED_BY `ruling:
/// held`) makes the read-back say B is held and A and C are rivals.
#[test]
fn the_owners_chosen_reading_reads_held_and_the_others_rival() {
    let mut g = load(COGITO);
    g.governed_by(
        "Inference",
        "inf:cogito-as-intuition",
        "Decision",
        "dec:which-reading-of-the-cogito",
        Some("held"),
        Some("the owner holds reading B as Descartes's own"),
    )
    .unwrap();
    let c = claim(&g, "req:i-exist");
    let roles: BTreeMap<String, String> = c["support"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            (
                s["inference"].as_str().unwrap().to_string(),
                s["as_reading"].as_str().unwrap_or("none").to_string(),
            )
        })
        .collect();
    assert_eq!(roles["inf:cogito-as-intuition"], "held", "{c:#}");
    assert_eq!(roles["inf:cogito-as-syllogism"], "rival", "{c:#}");
    assert_eq!(roles["inf:cogito-as-performance"], "rival", "{c:#}");
    let rep = report(&g, Some("req:i-exist"));
    assert_eq!(
        rep["contested_by_design"][0]["held_reading"], "inf:cogito-as-intuition",
        "{rep:#}"
    );
}

// ─── 3. A reply that concedes and moves ──────────────────────────────────────

/// Descartes' answer to the syllogism objections: granted against reading A,
/// the argument is meant as reading B. The objection counts against A by the
/// arguer's own admission, A supports nothing, B is named, and "I exist"
/// still holds through B.
#[test]
fn a_reply_concedes_an_objection_against_one_reading_and_moves_to_another() {
    let mut g = graph();
    statement(&mut g, "req:i-am-thinking", "proposition", true);
    statement(
        &mut g,
        "req:whatever-thinks-exists",
        "unstated_premise",
        true,
    );
    statement(&mut g, "req:i-exist", "proposition", false);
    step(
        &mut g,
        "inf:reading-a-syllogism",
        "all",
        &["req:i-am-thinking", "req:whatever-thinks-exists"],
        "req:i-exist",
    );
    step(
        &mut g,
        "inf:reading-b-intuition",
        "immediate",
        &["req:i-am-thinking"],
        "req:i-exist",
    );
    statement(&mut g, "req:obj-begs-the-question", "objection", false);
    objects(
        &mut g,
        "req:obj-begs-the-question",
        "inf:reading-a-syllogism",
    );
    statement(&mut g, "req:reply-not-a-syllogism", "reply", true);
    edge(
        &mut g,
        "req:reply-not-a-syllogism",
        "CONTRADICTS",
        "req:obj-begs-the-question",
        Props::new()
            .set("alignment", "conceding")
            .set("moves_to", "inf:reading-b-intuition")
            .set(
                "evidence",
                "granted against the syllogism; the cogito is an intuition",
            ),
    );
    let a = reading(&g, "inf:reading-a-syllogism");
    assert_eq!(a["objected"], "conceded", "{a:#}");
    assert_eq!(a["verdict"], "fails", "{a:#}");
    assert_eq!(a["objections"][0]["standing"], "conceded", "{a:#}");
    assert_eq!(
        a["objections"][0]["moves_to"], "inf:reading-b-intuition",
        "{a:#}"
    );
    assert_eq!(
        a["objections"][0]["conceded_by"], "req:reply-not-a-syllogism",
        "{a:#}"
    );
    assert_eq!(g.premise_truth("req:i-exist").unwrap().0.as_str(), "holds");
    assert_eq!(statement_reads(&g, "req:reply-not-a-syllogism"), "concedes");
    assert_eq!(statement_reads(&g, "req:obj-begs-the-question"), "conceded");
    let rep = report(&g, Some("req:i-exist"));
    assert!(
        rep["steps_objected"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["step"] == "inf:reading-a-syllogism" && s["reads"] == "conceded"),
        "{rep:#}"
    );
}

// ─── 4. Statement kinds for an argument ──────────────────────────────────────

#[test]
fn an_objection_a_reply_an_unstated_premise_and_a_method_rule_each_read_accordingly() {
    let mut g = graph();
    statement(&mut g, "req:claim", "proposition", true);
    statement(&mut g, "req:objection", "objection", false);
    objects(&mut g, "req:objection", "req:claim");
    assert_eq!(statement_reads(&g, "req:objection"), "stands");
    statement(&mut g, "req:reply", "reply", true);
    objects(&mut g, "req:reply", "req:objection");
    assert_eq!(statement_reads(&g, "req:reply"), "answers");
    assert_eq!(statement_reads(&g, "req:objection"), "answered");
    statement(&mut g, "req:aimless", "objection", false);
    assert_eq!(statement_reads(&g, "req:aimless"), "stated");
    // An unstated premise is not taken as given unless it says so.
    statement(&mut g, "req:unstated", "unstated_premise", false);
    let r = json(g.statement_reading("req:unstated").unwrap());
    assert_eq!(r["reads"], "stated", "{r:#}");
    assert!(
        r["why"].as_str().unwrap().contains("unstated premise"),
        "{r:#}"
    );
    // A rule of method is taken as given by choice.
    g.add_requirement(
        "req:method",
        "Method",
        "Accept only what is clear and distinct.",
    )
    .unwrap();
    g.upsert_node(
        "Requirement",
        "req:method",
        Props::new().set("statement_kind", "method_rule"),
    )
    .unwrap();
    assert_eq!(statement_reads(&g, "req:method"), "taken_as_given");
    assert_eq!(g.premise_truth("req:method").unwrap().0.as_str(), "holds");
}

// ─── 5. A step that is not an inference ──────────────────────────────────────

#[test]
fn a_step_can_say_it_is_not_an_inference() {
    let mut g = graph();
    statement(&mut g, "req:i-exist", "proposition", false);
    // An immediate grasp on no premise holds, and reads as one.
    g.add_inference(&reflow2_core::inference::InferenceSpec {
        id: "inf:grasped".to_string(),
        rule: "immediate".to_string(),
        k: None,
        premises: Vec::new(),
        conclusion: "req:i-exist".to_string(),
        rule_name: Some("simple intuition of the mind".to_string()),
        warrant: Some("grasped in one act, not deduced".to_string()),
        name: None,
        confirmed_by: None,
        confirmed_how: None,
        confirmed_at: None,
        assumed: Vec::new(),
        discharges: None,
        generalizes_over: None,
        independent_of: None,
        independent_except: None,
        cases: None,
        cases_exhaustive: None,
        rule_node: None,
        establishes: None,
        conclusion_form: None,
        as_intent: Vec::new(),
        as_delivery: Vec::new(),
    })
    .expect("an immediate step may name no premise");
    let r = reading(&g, "inf:grasped");
    assert_eq!(r["verdict"], "holds", "{r:#}");
    assert_eq!(r["rule"], "immediate", "{r:#}");
    assert!(
        g.premise_truth("inf:grasped")
            .unwrap()
            .1
            .contains("not an inference"),
        "it reads as not an inference"
    );
    assert_eq!(g.premise_truth("req:i-exist").unwrap().0.as_str(), "holds");
    // An objection can still aim at it.
    statement(&mut g, "req:lichtenberg", "objection", false);
    objects(&mut g, "req:lichtenberg", "inf:grasped");
    assert_eq!(reading(&g, "inf:grasped")["objected"], "contested");
    // Any other rule still needs a premise.
    statement(&mut g, "req:other", "proposition", false);
    let refused = g.add_inference(&reflow2_core::inference::InferenceSpec {
        id: "inf:empty".to_string(),
        rule: "all".to_string(),
        k: None,
        premises: Vec::new(),
        conclusion: "req:other".to_string(),
        rule_name: None,
        warrant: None,
        name: None,
        confirmed_by: None,
        confirmed_how: None,
        confirmed_at: None,
        assumed: Vec::new(),
        discharges: None,
        generalizes_over: None,
        independent_of: None,
        independent_except: None,
        cases: None,
        cases_exhaustive: None,
        rule_node: None,
        establishes: None,
        conclusion_form: None,
        as_intent: Vec::new(),
        as_delivery: Vec::new(),
    });
    assert!(refused.is_err());
}

// ─── 6. What a claim rests on ────────────────────────────────────────────────

/// Everything `claim`'s own support reaches, walked from the stored links:
/// the premises of each step concluding a node, and through a step used as a
/// premise — never an objection.
fn own_support(g: &DesignGraph, claim: &str) -> BTreeSet<String> {
    let mut seen: BTreeSet<String> = BTreeSet::from([claim.to_string()]);
    let mut queue = vec![claim.to_string()];
    while let Some(x) = queue.pop() {
        let mut steps = g.inferences_concluding(&x).unwrap();
        if type_of(g, &x) == "Inference" {
            steps.push(x.clone());
        }
        for st in steps {
            for p in g.premises_of(&st).unwrap() {
                if seen.insert(p.clone()) {
                    queue.push(p);
                }
            }
        }
    }
    seen
}

/// THE CLASS, over every claim a step concludes in every fixture: its
/// read-back's `rests_on` lies within its own support and names no step, and
/// its `uncited_premises` lie within `rests_on`. On main "Switch: turning
/// permissible" rested on the Footbridge judgment (a premise of an objection
/// to one of its principles) and listed steps as uncited premises.
#[test]
fn what_a_claim_rests_on_is_its_own_support_and_never_a_step() {
    for (name, doc) in ALL {
        let g = load(doc);
        for c in concluded(&g) {
            let rep = report(&g, Some(&c));
            let own = own_support(&g, &c);
            let rests_on = strs(&rep["rests_on"]);
            for p in &rests_on {
                assert!(
                    own.contains(p),
                    "{name}: {c} rests on {p}, outside its support"
                );
                assert_ne!(
                    type_of(&g, p),
                    "Inference",
                    "{name}: {c} rests on the step {p}"
                );
            }
            for p in strs(&rep["uncited_premises"]) {
                assert!(rests_on.contains(&p), "{name}: {c}: uncited {p}");
            }
        }
    }
    let g = load(TROLLEY);
    let rep = report(&g, Some("req:v-switch-turn-permissible"));
    assert_eq!(strs(&rep["rests_on"]), vec!["con:case-switch".to_string()]);
    assert!(
        strs(&rep["against_rests_on"])
            .contains(&"req:int-footbridge-most-judge-impermissible".to_string()),
        "{rep:#}"
    );
}

// ─── 7. Held in its project, with gaps that fit ──────────────────────────────

/// THE CLASS, over every fixture with one Project: the Project's scope holds
/// every step and what it concludes; and a design whose Project names an
/// argument or a proof is asked no artifact standard and no closure over
/// "live requirements". On main the trolley project's region was 1 node and
/// both gaps were raised.
#[test]
fn a_line_of_reasoning_is_held_in_its_project_and_its_gaps_fit_an_argument() {
    let mut argued = 0;
    for (name, doc) in ALL {
        let g = load(doc);
        let projects = g.scan_nodes("Project").unwrap();
        if projects.len() != 1 {
            continue;
        }
        let p = &projects[0];
        let region = g.scope_region(&p.node_id, 2).unwrap();
        for st in g.scan_nodes("Inference").unwrap() {
            assert!(
                region.contains(&st.node_id),
                "{name}: {} outside {}",
                st.node_id,
                p.node_id
            );
            if let Some(c) = g.conclusion_of(&st.node_id).unwrap() {
                assert!(region.contains(&c), "{name}: {c} outside {}", p.node_id);
            }
        }
        let domain = p
            .properties
            .get("domain")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_lowercase();
        if domain.contains("argument") || domain.contains("proof") {
            argued += 1;
            for gap in g.detect_gaps().unwrap() {
                let source = json(&gap)["gap_source"].as_str().unwrap().to_string();
                assert!(
                    !matches!(
                        source.as_str(),
                        "artifact_standard_undeclared" | "closure_criterion_undeclared"
                    ),
                    "{name}: a build's gap raised on an argument: {}",
                    gap.title
                );
            }
        }
    }
    assert!(argued >= 2, "the class is not vacuous: {argued}");
}
