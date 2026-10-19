//! A PROOF STATES WHAT IT REALLY DOES — phase 3 of the build list Anthony chose
//! on 2026-10-18 (all four phases), building
//! `req:a-proof-can-state-a-contradiction-cases-suppositions-and-cited-theorems`
//! and fixing one soundness defect the Arrow trial found.
//!
//! - **0. The eigenvariable rule** (Arrow trial, observer note N19): a step
//!   generalized over F while an UNDISCHARGED hypothesis mentioned F, and it
//!   read `holds`. Natural deduction allows ∀-introduction over x only when x
//!   is free in no assumption still open at that step. A generalization over v
//!   now does not hold while an assumption open at the step NAMES v — lists it
//!   in `variables` or `fixes` — or declares no `variables` at all, which
//!   leaves the question unanswerable.
//!   `fact:root-cause-the-generalization-check-read-only-fixes-and-never-the-assumptions-open-at-the-step-2026-10-18`.
//! - **1. A falsum** (statement_kind `falsum`, a contradiction derived inside a
//!   reductio) reads FALSE, never `holds`; the reductio resting on it holds.
//! - **2. A proof by cases** names its cases (`cases`) and the record saying
//!   they are exhaustive (`cases_exhaustive`): without it the step does not
//!   hold. The exhaustiveness is that record's word, never reflow2's.
//! - **3. A hypothesis** (statement_kind `hypothesis`) reads `assumed` or
//!   `discharged`, never "stated, not shown".
//! - **4. A cited result** (a theorem, lemma or corollary taken
//!   as given) reads `cited`, distinct from an axiom or a definition; one no
//!   pinned source documents reads `cited_unverified`, and what rests on it
//!   reads `conditional`, never plain `holds`.
//! - **5. The design-a-proof skill** has a recipe for each, and agrees with the
//!   tools.
//!
//! The measured cases are the trial fixtures (π+e, Arrow, Brocard, and the
//! primes and FTA rungs from the MCP crate). Every test is a CLASS over every
//! fixture where a fixture can carry it, plus the hand-built shape it names.
//! Written against the generic write path and read as JSON, so the file
//! compiles on main b32fe1adc; each test FAILED there first.

use reflow2_core::DesignGraph;
use reflow2_core::export::GraphExport;
use reflow2_core::foundation::core::Value;
use reflow2_core::nodes::Props;
use serde_json::Value as Json;

const PI_E: &str = include_str!("fixtures/proof_trial_pi_plus_e.json");
const ARROW: &str = include_str!("fixtures/proof_trial_arrow.json");
const BROCARD: &str = include_str!("fixtures/proof_trial_brocard.json");
const RUNG_1: &str =
    include_str!("../../reflow2-mcp/tests/fixtures/proof_trial_rung_1_primes.json");
const RUNG_2: &str = include_str!("../../reflow2-mcp/tests/fixtures/proof_trial_rung_2_fta.json");
const SKILL: &str = include_str!("../../../getting-started/skills/design-a-proof/SKILL.md");

const ALL: [(&str, &str); 5] = [
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

fn list(items: &[&str]) -> Value {
    Value::List(items.iter().map(|s| Value::String(s.to_string())).collect())
}

fn set(g: &mut DesignGraph, id: &str, key: &str, value: Value) {
    let ty = type_of(g, id);
    g.upsert_node(&ty, id, Props::new().set(key, value))
        .unwrap_or_else(|e| panic!("{id}.{key}: {e}"));
}

fn set_list(g: &mut DesignGraph, id: &str, key: &str, items: &[&str]) {
    set(g, id, key, list(items));
}

fn statement(g: &mut DesignGraph, id: &str, kind: Option<&str>, variables: Option<&[&str]>) {
    g.add_requirement(id, &format!("statement {id}"), "A mathematical statement.")
        .unwrap();
    if let Some(k) = kind {
        set(g, id, "statement_kind", Value::String(k.to_string()));
    }
    if let Some(v) = variables {
        set_list(g, id, "variables", v);
    }
}

fn definition(g: &mut DesignGraph, id: &str) {
    g.add_constraint(
        id,
        &format!("definition {id}"),
        "A definition.",
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    set(g, id, "statement_kind", Value::String("definition".into()));
}

fn edge(g: &mut DesignGraph, from: &str, rel: &str, to: &str, props: Props) {
    let (ft, tt) = (type_of(g, from), type_of(g, to));
    g.create_edge(rel, &ft, from, &tt, to, props)
        .unwrap_or_else(|e| panic!("{from} {rel} {to}: {e}"));
}

/// A step through the generic write path: the Inference, its PREMISE_OFs
/// (`assumed` ones carrying the role) and its one CONCLUDES.
fn step(g: &mut DesignGraph, id: &str, premises: &[&str], assumed: &[&str], conclusion: &str) {
    g.upsert_node(
        "Inference",
        id,
        Props::new().set("name", id).set("rule", "all"),
    )
    .unwrap();
    for p in premises {
        edge(g, p, "PREMISE_OF", id, Props::new());
    }
    for p in assumed {
        edge(g, p, "PREMISE_OF", id, Props::new().set("role", "assumed"));
    }
    edge(g, id, "CONCLUDES", conclusion, Props::new());
}

fn reading(g: &DesignGraph, step: &str) -> Json {
    json(g.inference_reading(step).unwrap().expect("a step"))
}

fn statement_reads(g: &DesignGraph, id: &str) -> Json {
    json(g.statement_reading(id).unwrap())
        .get("reads")
        .cloned()
        .unwrap_or(Json::Null)
}

fn claim(g: &DesignGraph, id: &str) -> Json {
    let r = json(g.reasoning_report(Some(id), Some(500)).unwrap());
    r["claims"]
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

fn props(g: &DesignGraph, id: &str) -> std::collections::HashMap<String, Value> {
    let ty = type_of(g, id);
    g.get_node(&ty, id).unwrap().unwrap().properties
}

/// The variables an assumption names: its `variables` and what its `fixes`
/// bind. `None` when it declares no `variables` at all.
fn names(g: &DesignGraph, id: &str) -> Option<Vec<String>> {
    let p = props(g, id);
    let listed = |key: &str| -> Option<Vec<String>> {
        match p.get(key) {
            Some(Value::List(items)) => Some(
                items
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.split('=').next().unwrap_or(s).trim().to_string())
                    .collect(),
            ),
            _ => None,
        }
    };
    let mut out = listed("variables")?;
    out.extend(listed("fixes").unwrap_or_default());
    Some(out)
}

// ─── 0. The eigenvariable rule ───────────────────────────────────────────────

/// THE CLASS, over every generalizing step of every fixture: a generalization
/// holds only when no assumption still open at the step names a variable it
/// generalizes over (or leaves the question unanswerable by declaring none).
/// Asked twice per step: as the fixture stands, and with the step's own
/// discharges taken away, which opens exactly the hypotheses the trial
/// designers closed there (Arrow's theorem over F, FTA's conditional proofs).
/// On main the Arrow theorem without its discharges read `holds` while its
/// open hypothesis on F named F: N19, reproduced.
#[test]
fn a_generalization_does_not_hold_over_a_variable_an_open_assumption_names() {
    let mut asked = 0;
    let mut named = 0;
    for (name, doc) in ALL {
        let base = load(doc);
        let steps: Vec<(String, Vec<String>, bool)> = base
            .scan_nodes("Inference")
            .unwrap()
            .into_iter()
            .filter_map(|n| {
                let over = match n.properties.get("generalizes_over") {
                    Some(Value::List(v)) if !v.is_empty() => v
                        .iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect::<Vec<_>>(),
                    _ => return None,
                };
                let discharges = matches!(
                    n.properties.get("discharges"),
                    Some(Value::List(v)) if !v.is_empty()
                );
                Some((n.node_id, over, discharges))
            })
            .collect();
        for (st, over, discharges) in steps {
            for open_its_discharges in [false, true] {
                if open_its_discharges && !discharges {
                    continue;
                }
                let mut g = load(doc);
                if open_its_discharges {
                    set(&mut g, &st, "discharges", Value::List(Vec::new()));
                }
                let r = reading(&g, &st);
                let open = strs(&r["assumes"]);
                let offending: Vec<String> = open
                    .iter()
                    .filter(|a| match names(&g, a) {
                        None => true,
                        Some(vs) => vs.iter().any(|v| over.contains(v)),
                    })
                    .cloned()
                    .collect();
                let fixed = !r["generalization"]["fixed_by"]
                    .as_array()
                    .map(Vec::is_empty)
                    .unwrap_or(true);
                asked += 1;
                if !offending.is_empty() {
                    named += 1;
                    assert_eq!(
                        r["generalization"]["holds"], false,
                        "{name} {st} (discharges opened: {open_its_discharges}): it generalizes \
                         over {over:?} while {offending:?} is open and names one of them: {r:#}"
                    );
                    assert_ne!(r["verdict"], "holds", "{name} {st}: {r:#}");
                    for a in &offending {
                        assert!(
                            r["note"].as_str().unwrap_or("").contains(a.as_str()),
                            "{name} {st}: the reason does not name {a}: {r:#}"
                        );
                    }
                } else if !fixed {
                    assert_eq!(r["generalization"]["holds"], true, "{name} {st}: {r:#}");
                }
            }
        }
    }
    assert!(asked > 20, "only {asked} generalizations asked");
    assert!(
        named >= 3,
        "only {named} generalizations ran under an open assumption naming their variable"
    );
}

/// N19 itself, by hand: a hypothesis about F is assumed, a step generalizes
/// over F while it is still open, and only a LATER step discharges it. That
/// order certifies ∀F before the hypothesis on F is closed, which natural
/// deduction forbids. Discharging first and generalizing in the same step (a
/// conditional proof over F, Arrow's own last step) stays valid, and so does
/// an open hypothesis about another variable.
#[test]
fn the_n19_shape_a_generalization_under_an_open_hypothesis_about_its_variable_does_not_hold() {
    let build = |hyp_vars: Option<&[&str]>| {
        let mut g = graph();
        definition(&mut g, "con:def-unanimity");
        statement(&mut g, "req:hyp-f", None, hyp_vars);
        statement(&mut g, "req:f-dictatorial", None, Some(&["F"]));
        statement(&mut g, "req:every-f-dictatorial", None, None);
        statement(&mut g, "req:thm", Some("theorem"), None);
        step(
            &mut g,
            "inf:under-hyp",
            &["con:def-unanimity"],
            &["req:hyp-f"],
            "req:f-dictatorial",
        );
        step(
            &mut g,
            "inf:generalize",
            &["req:f-dictatorial"],
            &[],
            "req:every-f-dictatorial",
        );
        set_list(&mut g, "inf:generalize", "generalizes_over", &["F"]);
        step(
            &mut g,
            "inf:discharge",
            &["req:every-f-dictatorial"],
            &[],
            "req:thm",
        );
        set_list(&mut g, "inf:discharge", "discharges", &["req:hyp-f"]);
        g
    };

    // The hypothesis names F: the generalization does not hold.
    let g = build(Some(&["F"]));
    let r = reading(&g, "inf:generalize");
    assert_eq!(r["verdict"], "unknown", "{r:#}");
    assert_eq!(r["generalization"]["holds"], false, "{r:#}");
    assert_eq!(
        r["generalization"]["named_by_open_assumptions"][0]["id"], "req:hyp-f",
        "{r:#}"
    );
    assert!(r["note"].as_str().unwrap().contains("req:hyp-f"), "{r:#}");
    // So the theorem is unsupported, never refuted, and the loop counts it.
    assert_ne!(proof_status(&g, "req:thm"), "shown");
    assert_eq!(g.premise_truth("req:thm").unwrap().0.as_str(), "unknown");
    assert_eq!(
        json(g.loop_status().unwrap())["reasoning"]["generalization_not_holding_ids"],
        serde_json::json!(["inf:generalize"])
    );

    // A hypothesis about another variable leaves the generalization valid.
    let g = build(Some(&["G"]));
    assert_eq!(reading(&g, "inf:generalize")["verdict"], "holds");
    assert_eq!(proof_status(&g, "req:thm"), "shown");

    // One that declares an EMPTY list names nothing, so it is valid too.
    let g = build(Some(&[]));
    assert_eq!(reading(&g, "inf:generalize")["verdict"], "holds");

    // One that declares no variables at all leaves the check unanswerable.
    let g = build(None);
    let r = reading(&g, "inf:generalize");
    assert_eq!(r["verdict"], "unknown", "{r:#}");
    assert_eq!(
        r["generalization"]["undeclared_open_assumptions"],
        serde_json::json!(["req:hyp-f"]),
        "{r:#}"
    );

    // Discharge first, then generalize, in one step: valid, as in Arrow.
    let mut g = build(Some(&["F"]));
    set(
        &mut g,
        "inf:discharge",
        "discharges",
        Value::List(Vec::new()),
    );
    set_list(&mut g, "inf:generalize", "discharges", &["req:hyp-f"]);
    let r = reading(&g, "inf:generalize");
    assert_eq!(r["verdict"], "holds", "{r:#}");
    assert_eq!(r["generalization"]["holds"], true, "{r:#}");
}

// ─── 0b. A conditional claim never refutes outside its supposition ──────────

/// The opposing CONTRADICTS edges into `target`, read: (objection, standing,
/// why).
fn objections(g: &DesignGraph, target: &str) -> Vec<(String, String, String)> {
    g.objections_to(target)
        .unwrap()
        .into_iter()
        .map(|o| (o.id, o.standing.to_string(), o.why))
        .collect()
}

/// THE CLASS, over every fixture: every claim that holds only under an
/// assumption no step discharges, set against a claim that holds on no
/// assumption at all (a fresh definition), CONTESTS it and never refutes it,
/// and the objection's reading names the supposition. Natural deduction's
/// scope rule: outside the supposition, only "if S then not-T" is
/// established, and that refutes nothing. On main b32fe1adc each target read
/// refuted ("refuted by …, which holds"): the cogito trial's shape
/// (flo2.io 989598dd02246a81), asked of every conditional claim the trial
/// designs hold.
#[test]
fn a_claim_holding_only_under_an_open_supposition_contests_and_never_refutes_outside_it() {
    let mut asked = 0;
    for (name, doc) in ALL {
        let mut g = load(doc);
        let mut conditional: Vec<(String, Vec<String>)> = Vec::new();
        for ty in ["Requirement", "Constraint"] {
            for n in g.scan_nodes(ty).unwrap() {
                let under = g.assumptions_under(&n.node_id).unwrap();
                if !under.is_empty() && g.premise_truth(&n.node_id).unwrap().0.as_str() == "holds" {
                    conditional.push((n.node_id, under));
                }
            }
        }
        conditional.truncate(12);
        for (i, (c, under)) in conditional.iter().enumerate() {
            let target = format!("con:outside-every-supposition-{i}");
            definition(&mut g, &target);
            edge(
                &mut g,
                c,
                "CONTRADICTS",
                &target,
                Props::new().set("alignment", "opposing").set(
                    "evidence",
                    "a conditional claim set against an unconditional one",
                ),
            );
            let (truth, why) = g.premise_truth(&target).unwrap();
            assert_ne!(
                truth.as_str(),
                "fails",
                "{name}: {c} holds only under {under:?} and still REFUTES {target}: {why}"
            );
            let o = objections(&g, &target);
            assert_eq!(o.len(), 1, "{name} {target}: {o:?}");
            assert_eq!(o[0].1, "stands", "{name}: {c} against {target}: {o:?}");
            assert!(
                under.iter().any(|a| o[0].2.contains(a.as_str())),
                "{name}: the objection's reading should name the supposition {under:?}: {o:?}"
            );
            asked += 1;
        }
    }
    assert!(asked >= 10, "only {asked} conditional claims were asked");
}

/// The cogito trial itself, by hand (989598dd02246a81): the functionalist
/// verdict holds only under "suppose an AI authentically doubts", and it
/// opposes the Cartesian verdict, which rests on no supposition. It contests
/// that verdict and does not refute it. INSIDE the supposition's scope — a
/// target that itself holds only under the same supposition — the refutation
/// stands, as it does inside any hypothetical argument. And a conditional
/// REPLY does not answer an objection that holds outright: it contests it, so
/// the claim it was meant to reinstate reads contested, not holds.
#[test]
fn the_cogito_shape_a_conditional_verdict_contests_the_other_and_a_conditional_reply_answers_nothing()
 {
    let mut g = graph();
    definition(&mut g, "con:functionalism");
    definition(&mut g, "con:discourse-v-two-tests");
    statement(
        &mut g,
        "req:suppose-ai-authentically-doubts",
        None,
        Some(&[]),
    );
    statement(&mut g, "req:ai-is-res-cogitans-if", None, None);
    statement(&mut g, "req:ai-not-res-cogitans", None, None);
    step(
        &mut g,
        "inf:functionalist-verdict",
        &["con:functionalism"],
        &["req:suppose-ai-authentically-doubts"],
        "req:ai-is-res-cogitans-if",
    );
    step(
        &mut g,
        "inf:cartesian-verdict",
        &["con:discourse-v-two-tests"],
        &[],
        "req:ai-not-res-cogitans",
    );
    edge(
        &mut g,
        "req:ai-is-res-cogitans-if",
        "CONTRADICTS",
        "req:ai-not-res-cogitans",
        Props::new()
            .set("alignment", "opposing")
            .set("evidence", "both cannot hold"),
    );
    let (truth, why) = g.premise_truth("req:ai-not-res-cogitans").unwrap();
    assert_eq!(truth.as_str(), "unknown", "contested, never refuted: {why}");
    assert!(why.contains("req:suppose-ai-authentically-doubts"), "{why}");
    assert_ne!(proof_status(&g, "req:ai-not-res-cogitans"), "unproven");
    let c = claim(&g, "req:ai-not-res-cogitans");
    assert_eq!(c["objections"][0]["standing"], "stands", "{c:#}");

    // Inside the scope: a target that holds only under the same supposition
    // is refuted by it.
    statement(&mut g, "req:inside", None, None);
    step(
        &mut g,
        "inf:inside",
        &["con:functionalism"],
        &["req:suppose-ai-authentically-doubts"],
        "req:inside",
    );
    edge(
        &mut g,
        "req:ai-is-res-cogitans-if",
        "CONTRADICTS",
        "req:inside",
        Props::new()
            .set("alignment", "opposing")
            .set("evidence", "both cannot hold under the supposition"),
    );
    assert_eq!(g.premise_truth("req:inside").unwrap().0.as_str(), "fails");

    // A conditional reply answers nothing. An objection that holds outright
    // refutes X; a reply holding only under a supposition contests the
    // objection, so X is contested, never reinstated.
    let mut g = graph();
    definition(&mut g, "con:a");
    definition(&mut g, "con:b");
    statement(&mut g, "req:x", None, None);
    step(&mut g, "inf:x", &["con:a"], &[], "req:x");
    edge(
        &mut g,
        "con:b",
        "CONTRADICTS",
        "req:x",
        Props::new()
            .set("alignment", "opposing")
            .set("evidence", "an objection that holds"),
    );
    assert_eq!(g.premise_truth("req:x").unwrap().0.as_str(), "fails");
    statement(&mut g, "req:suppose", None, Some(&[]));
    statement(&mut g, "req:reply", None, None);
    step(
        &mut g,
        "inf:reply",
        &["con:a"],
        &["req:suppose"],
        "req:reply",
    );
    edge(
        &mut g,
        "req:reply",
        "CONTRADICTS",
        "con:b",
        Props::new()
            .set("alignment", "opposing")
            .set("evidence", "a reply that holds only under a supposition"),
    );
    let (truth, why) = g.premise_truth("con:b").unwrap();
    assert_eq!(
        truth.as_str(),
        "unknown",
        "the objection is contested, not answered: {why}"
    );
    assert_eq!(
        g.premise_truth("req:x").unwrap().0.as_str(),
        "unknown",
        "X is contested, never reinstated by a conditional reply"
    );
}

/// The same class, at the other places a holding verdict is consumed: a
/// requirement that a step concludes only under an open supposition is not
/// DELIVERED by the roll-up, and a `met` status resting on it is a conclusion
/// that does not follow. On main both counted the conditional step as
/// holding: the requirement read delivered, and its `met` went unreported.
#[test]
fn a_conditional_result_is_neither_delivered_nor_enough_for_a_claimed_status() {
    let mut g = graph();
    definition(&mut g, "con:a");
    statement(&mut g, "req:suppose", None, Some(&[]));
    statement(&mut g, "req:only-under-it", None, None);
    step(
        &mut g,
        "inf:under",
        &["con:a"],
        &["req:suppose"],
        "req:only-under-it",
    );
    assert_eq!(
        g.requirement_delivered_by("req:only-under-it").unwrap(),
        None,
        "a result inside an argument is not delivered"
    );
    set(
        &mut g,
        "req:only-under-it",
        "status",
        Value::String("met".into()),
    );
    let gaps: Vec<Json> = g
        .detect_gaps()
        .unwrap()
        .into_iter()
        .map(json)
        .filter(|x| x["gap_source"] == "conclusion_does_not_follow")
        .collect();
    assert!(
        gaps.iter()
            .any(|x| strs(&x["affected_ids"]).contains(&"req:only-under-it".to_string())),
        "a `met` resting only on a supposition is past what its steps support: {gaps:#?}"
    );
    // Discharged, it is delivered as before.
    statement(&mut g, "req:if-supposed", None, None);
    step(
        &mut g,
        "inf:discharge",
        &["req:only-under-it"],
        &[],
        "req:if-supposed",
    );
    set_list(&mut g, "inf:discharge", "discharges", &["req:suppose"]);
    assert!(
        g.requirement_delivered_by("req:if-supposed")
            .unwrap()
            .is_some()
    );
}

// ─── 1. A falsum reads false ─────────────────────────────────────────────────

/// π+e as the trial designer could not record it (friction P2): the
/// contradiction "π is algebraic and π is transcendental" is a statement of
/// its own, derived from the supposition's consequence and Lindemann, and the
/// reductio rests on it and discharges the supposition.
fn pi_e_with_a_falsum(g: &mut DesignGraph) {
    g.add_requirement(
        "req:pi-both-algebraic-and-transcendental",
        "π is algebraic and transcendental",
        "⊥: π is algebraic (from the supposition) and transcendental (Lindemann).",
    )
    .unwrap();
    set(
        g,
        "req:pi-both-algebraic-and-transcendental",
        "statement_kind",
        Value::String("falsum".into()),
    );
    step(
        g,
        "inf:absurd",
        &[
            "req:pi-algebraic",
            "req:lindemann-pi-transcendental",
            "con:def-transcendental",
        ],
        &[],
        "req:pi-both-algebraic-and-transcendental",
    );
    // The reductio now rests on the falsum alone.
    for p in [
        "req:pi-algebraic",
        "req:lindemann-pi-transcendental",
        "con:def-transcendental",
    ] {
        g.delete_edge("PREMISE_OF", p, "inf:reductio").unwrap();
    }
    edge(
        g,
        "req:pi-both-algebraic-and-transcendental",
        "PREMISE_OF",
        "inf:reductio",
        Props::new(),
    );
}

#[test]
fn a_falsum_reads_false_and_the_reductio_resting_on_it_holds() {
    let mut g = load(PI_E);
    pi_e_with_a_falsum(&mut g);
    let bottom = "req:pi-both-algebraic-and-transcendental";
    // The falsum is FALSE: never holds, never conditional, never "refuted".
    let (truth, why) = g.premise_truth(bottom).unwrap();
    assert_eq!(truth.as_str(), "fails", "{why}");
    assert_eq!(statement_reads(&g, bottom), "false");
    let c = claim(&g, bottom);
    assert_eq!(c["truth"], "fails", "{c:#}");
    assert!(
        c["why"]
            .as_str()
            .unwrap()
            .contains("req:suppose-both-algebraic"),
        "the falsum should say which assumption it shows cannot hold: {c:#}"
    );
    // The step deriving it holds, and so does the reductio resting on it.
    assert_eq!(reading(&g, "inf:absurd")["verdict"], "holds");
    let r = reading(&g, "inf:reductio");
    assert_eq!(r["verdict"], "holds", "{r:#}");
    assert_eq!(r["premises"][0]["truth"], "holds", "{r:#}");
    // The theorem no longer rests on the supposition.
    assert!(strs(&claim(&g, "req:theorem-pi-plus-e-or-pi-times-e")["assumes"]).is_empty());

    // A falsum derived under NO assumption says the foundations are
    // inconsistent, and the loop counts it.
    let mut g = graph();
    definition(&mut g, "con:a");
    definition(&mut g, "con:not-a");
    statement(&mut g, "req:bottom", Some("falsum"), None);
    step(
        &mut g,
        "inf:clash",
        &["con:a", "con:not-a"],
        &[],
        "req:bottom",
    );
    assert_eq!(statement_reads(&g, "req:bottom"), "inconsistent");
    assert_eq!(
        json(g.loop_status().unwrap())["reasoning"]["inconsistency_ids"],
        serde_json::json!(["req:bottom"])
    );
    // And one nothing derives reads stated, false all the same.
    statement(&mut g, "req:bottom-2", Some("falsum"), None);
    assert_eq!(statement_reads(&g, "req:bottom-2"), "stated");
    assert_eq!(g.premise_truth("req:bottom-2").unwrap().0.as_str(), "fails");
}

// ─── 2. A proof by cases says whether its cases are declared exhaustive ──────

/// "n² + n is even": case n even, case n odd, and the record that every
/// integer is one or the other.
fn by_cases(g: &mut DesignGraph) {
    definition(g, "con:def-even");
    statement(g, "req:case-even", Some("hypothesis"), Some(&["n"]));
    statement(g, "req:case-odd", Some("hypothesis"), Some(&["n"]));
    statement(g, "req:even-when-even", None, Some(&["n"]));
    statement(g, "req:even-when-odd", None, Some(&["n"]));
    statement(g, "req:n2-plus-n-even", Some("theorem"), Some(&["n"]));
    step(
        g,
        "inf:case-even",
        &["con:def-even"],
        &["req:case-even"],
        "req:even-when-even",
    );
    step(
        g,
        "inf:case-odd",
        &["con:def-even"],
        &["req:case-odd"],
        "req:even-when-odd",
    );
    step(
        g,
        "inf:by-cases",
        &["req:even-when-even", "req:even-when-odd"],
        &[],
        "req:n2-plus-n-even",
    );
    set_list(
        g,
        "inf:by-cases",
        "cases",
        &["req:case-even", "req:case-odd"],
    );
}

#[test]
fn a_case_split_holds_only_when_its_cases_are_declared_exhaustive() {
    let mut g = graph();
    by_cases(&mut g);
    // The cases are named and discharged, so the theorem does not rest on
    // them; but nothing says they are all the cases.
    let r = reading(&g, "inf:by-cases");
    assert_eq!(r["cases"]["declared"], false, "{r:#}");
    assert_eq!(r["verdict"], "unknown", "{r:#}");
    assert!(r["note"].as_str().unwrap().contains("exhaustive"), "{r:#}");
    assert!(
        strs(&r["assumes"]).is_empty(),
        "the cases are discharged: {r:#}"
    );
    assert_eq!(
        json(g.loop_status().unwrap())["reasoning"]["cases_not_declared_exhaustive_ids"],
        serde_json::json!(["inf:by-cases"])
    );

    // The owner's word: "every integer is even or odd", taken as given.
    definition(&mut g, "con:every-n-even-or-odd");
    set(
        &mut g,
        "inf:by-cases",
        "cases_exhaustive",
        Value::String("con:every-n-even-or-odd".into()),
    );
    edge(
        &mut g,
        "con:every-n-even-or-odd",
        "PREMISE_OF",
        "inf:by-cases",
        Props::new(),
    );
    let r = reading(&g, "inf:by-cases");
    assert_eq!(r["cases"]["declared"], true, "{r:#}");
    assert_eq!(r["cases"]["exhaustive_by"], "con:every-n-even-or-odd");
    assert_eq!(r["cases"]["truth"], "holds", "{r:#}");
    assert_eq!(r["verdict"], "holds", "{r:#}");
    assert_eq!(proof_status(&g, "req:n2-plus-n-even"), "shown");
    // The two hypotheses read discharged by the split.
    assert_eq!(statement_reads(&g, "req:case-even"), "discharged");

    // An exhaustiveness record that is only conjectured does not carry it.
    statement(&mut g, "req:maybe-exhaustive", Some("conjecture"), None);
    set(
        &mut g,
        "inf:by-cases",
        "cases_exhaustive",
        Value::String("req:maybe-exhaustive".into()),
    );
    let r = reading(&g, "inf:by-cases");
    assert_eq!(r["cases"]["declared"], true, "{r:#}");
    assert_ne!(r["verdict"], "holds", "{r:#}");
}

/// Arrow's group-contraction step splits on two cases and only `discharges`
/// them; named as `cases`, it reads that nothing declares them exhaustive.
#[test]
fn arrows_case_split_says_its_cases_are_not_declared_exhaustive() {
    let mut g = load(ARROW);
    assert_eq!(reading(&g, "inf:lemma-6")["verdict"], "holds");
    set_list(
        &mut g,
        "inf:lemma-6",
        "cases",
        &["req:gc-case-1-hyp", "req:gc-case-2-hyp"],
    );
    let r = reading(&g, "inf:lemma-6");
    assert_eq!(r["cases"]["declared"], false, "{r:#}");
    assert_eq!(r["verdict"], "unknown", "{r:#}");
}

// ─── 3. A hypothesis has a kind of its own ───────────────────────────────────

/// EVERY node a step of a fixture takes up as an assumption, given the kind
/// `hypothesis`, reads `assumed` or `discharged` as the design has it, never
/// "stated, not shown" (Arrow N20, π+e P3).
#[test]
fn every_assumption_recorded_as_a_hypothesis_reads_assumed_or_discharged() {
    let mut read = 0;
    for (name, doc) in ALL {
        let mut g = load(doc);
        let assumptions = g.assumptions().unwrap();
        for a in &assumptions {
            set(
                &mut g,
                &a.id,
                "statement_kind",
                Value::String("hypothesis".into()),
            );
        }
        for a in &assumptions {
            let want = if a.discharged_by.is_empty() {
                "assumed"
            } else {
                "discharged"
            };
            assert_eq!(statement_reads(&g, &a.id), want, "{name} {}", a.id);
            assert!(
                !g.statement_reading(&a.id)
                    .unwrap()
                    .unwrap()
                    .why
                    .contains("stated, not shown"),
                "{name} {}",
                a.id
            );
            read += 1;
        }
    }
    assert!(read >= 10, "only {read} assumptions read");

    // A hypothesis no step takes up says so.
    let mut g = graph();
    statement(&mut g, "req:h", Some("hypothesis"), None);
    assert_eq!(statement_reads(&g, "req:h"), "not_assumed");
}

// ─── 4. A cited result is cited, verified or not ─────────────────────────────

const CITED_KINDS: [&str; 3] = ["theorem", "lemma", "corollary"];

/// EVERY result of every fixture taken as given (Lindemann, Fermat's little
/// theorem, Overholt, Berend–Osgood ...) is a CITATION: it reads `cited` or
/// `cited_unverified`, never "stated" or "taken as given" as if it were an
/// axiom, and its claim never reads a plain `holds` unless a pinned source
/// documents it. No fixture pins one, so every one reads unverified.
#[test]
fn every_result_taken_as_given_reads_cited_and_unverified_until_a_pinned_source_documents_it() {
    let mut cited = 0;
    for (name, doc) in ALL {
        let g = load(doc);
        for ty in ["Requirement", "Constraint"] {
            for n in g.scan_nodes(ty).unwrap() {
                let kind = n.properties.get("statement_kind").and_then(Value::as_str);
                let given = n.properties.get("taken_as_given").and_then(Value::as_bool);
                if !(kind.is_some_and(|k| CITED_KINDS.contains(&k)) && given == Some(true)) {
                    continue;
                }
                cited += 1;
                assert_eq!(
                    statement_reads(&g, &n.node_id),
                    "cited_unverified",
                    "{name} {}",
                    n.node_id
                );
                let c = claim(&g, &n.node_id);
                assert_ne!(c["truth"], "holds", "{name} {}: {c:#}", n.node_id);
                assert!(
                    strs(&c["rests_on_unverified"]).contains(&n.node_id),
                    "{name} {}: {c:#}",
                    n.node_id
                );
            }
        }
        // And every definition or axiom still reads taken as given.
        for n in g.scan_nodes("Constraint").unwrap() {
            if matches!(
                n.properties.get("statement_kind").and_then(Value::as_str),
                Some("definition" | "axiom")
            ) {
                assert_eq!(statement_reads(&g, &n.node_id), "taken_as_given");
            }
        }
    }
    assert!(cited >= 5, "only {cited} cited results");
}

/// π+e's theorem rests on two recalled citations, so it reads conditional on
/// them. Pinning each source (an Artifact with a checksum that DOCUMENTS it)
/// makes each `cited`, and the theorem reads shown again.
#[test]
fn a_claim_resting_on_an_unverified_citation_reads_conditional_until_its_source_is_pinned() {
    let mut g = load(PI_E);
    let thm = "req:theorem-pi-plus-e-or-pi-times-e";
    let citations = [
        "req:lindemann-pi-transcendental",
        "req:algebraic-numbers-closed",
    ];
    assert_eq!(proof_status(&g, thm), "conditional");
    let c = claim(&g, thm);
    assert_eq!(c["truth"], "conditional", "{c:#}");
    for cit in citations {
        assert!(
            strs(&c["rests_on_unverified"]).contains(&cit.to_string()),
            "{c:#}"
        );
    }
    for (i, cit) in citations.iter().enumerate() {
        let art = format!("art:source-{i}");
        g.upsert_node(
            "Artifact",
            &art,
            Props::new()
                .set("name", format!("the source of {cit}"))
                .set("location", "https://doi.org/10.1007/example")
                .set("checksum", "sha256:0123456789abcdef"),
        )
        .unwrap();
        edge(&mut g, &art, "DOCUMENTS", cit, Props::new());
        assert_eq!(statement_reads(&g, cit), "cited");
        assert_eq!(claim(&g, cit)["truth"], "holds");
    }
    assert_eq!(proof_status(&g, thm), "shown");
    assert_eq!(claim(&g, thm)["truth"], "holds");
    // An unpinned source (no checksum) is recalled, not verified.
    let mut g = load(PI_E);
    g.upsert_node(
        "Artifact",
        "art:recalled",
        Props::new().set("name", "Lindemann 1882, recalled"),
    )
    .unwrap();
    edge(
        &mut g,
        "art:recalled",
        "DOCUMENTS",
        "req:lindemann-pi-transcendental",
        Props::new(),
    );
    assert_eq!(
        statement_reads(&g, "req:lindemann-pi-transcendental"),
        "cited_unverified"
    );
    assert_eq!(
        json(g.loop_status().unwrap())["reasoning"]["unverified_citations"],
        2
    );
}

// ─── 5. The skill has a recipe for each, and agrees with the tools ──────────

#[test]
fn the_design_a_proof_skill_has_a_recipe_for_each_and_agrees_with_the_tools() {
    let kinds = reflow2_core::schema::enum_values("Requirement", "statement_kind")
        .expect("statement_kind is an enum");
    for k in &kinds {
        assert!(
            SKILL.contains(&format!("`{k}`")),
            "the skill never names the statement kind `{k}`"
        );
    }
    for word in [
        "`cases`",
        "`cases_exhaustive`",
        "`cited`",
        "`cited_unverified`",
        "`falsum`",
        "`hypothesis`",
        "`inconsistent`",
    ] {
        assert!(SKILL.contains(word), "the skill never says {word}");
    }
    // P4: the skill said a theorem is never taken as given, while a cited
    // theorem is exactly that.
    assert!(
        !SKILL.contains("None of them is taken as given"),
        "the skill still says a theorem is never taken as given"
    );
    // The eigenvariable rule is said where a generalization is.
    assert!(
        SKILL.contains("open"),
        "the skill does not say a generalization is refused under an open assumption"
    );
}
