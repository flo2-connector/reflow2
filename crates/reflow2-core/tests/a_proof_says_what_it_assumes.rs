//! A PROOF SAYS WHAT IT ASSUMES — phase 2 of the fixes Anthony accepted on
//! 2026-10-17 after the primes trial on flo2.io
//! (fact:the-primes-trial-rung-1-ran-on-flo2-io-and-its-friction-names-what-counts-2026-10-17).
//!
//! On the trial's own shapes:
//!
//! - Euclid's reductio, "suppose the primes are finite", recorded HONESTLY: the
//!   supposition is an `assumed` premise of the step that derives the
//!   contradiction, and the theorem's step discharges it
//!   (req:a-proof-can-discharge-an-assumption-and-generalize-over-a-variable,
//!   Anthony's choice: a premise role);
//! - "for an arbitrary finite list L", generalized in inf:euclid, declared and
//!   CHECKED: no premise in the chain may fix L (his choice: declared and
//!   checked);
//! - an induction step discharging its hypothesis and generalizing over k;
//! - statement kinds, a conjecture that reads refuted, two lines stated
//!   independent and checked, and a definition that carries no budget words
//!   (req:a-proof-is-designed-with-a-skill-and-the-tools-fit-mathematics).
//!
//! Written against the generic write path (an edge's `role`, a step's
//! `discharges`, a statement's `fixes` are plain properties), so each test
//! compiles on 0.85.0 and fails there on the behaviour it pins.

use reflow2_core::DesignGraph;
use reflow2_core::inference::{InferenceSpec, Truth};
use reflow2_core::nodes::Props;
use serde_json::Value as Json;

fn graph() -> DesignGraph {
    DesignGraph::open_in_memory().expect("in-memory graph")
}

fn type_of(g: &DesignGraph, id: &str) -> String {
    let held = g.node_types_holding(id).expect("types holding");
    assert_eq!(held.len(), 1, "{id}: {held:?}");
    held[0].clone()
}

fn statement(g: &mut DesignGraph, id: &str) {
    g.add_requirement(id, &format!("statement {id}"), "A mathematical statement.")
        .unwrap();
}

fn axiom(g: &mut DesignGraph, id: &str) {
    g.add_constraint(
        id,
        &format!("axiom {id}"),
        "A definition or axiom.",
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
}

fn step(g: &mut DesignGraph, id: &str, premises: &[&str], conclusion: &str) {
    g.add_inference(&InferenceSpec {
        id: id.to_string(),
        rule: "all".to_string(),
        premises: premises.iter().map(|p| p.to_string()).collect(),
        conclusion: conclusion.to_string(),
        ..Default::default()
    })
    .unwrap_or_else(|e| panic!("step {id}: {e}"));
}

/// Mark `premise` an ASSUMED premise of `step`, through the generic write
/// path: the edge's own `role`.
fn assume(g: &mut DesignGraph, premise: &str, step_id: &str) {
    let ty = type_of(g, premise);
    g.create_edge(
        "PREMISE_OF",
        &ty,
        premise,
        "Inference",
        step_id,
        Props::new().set("role", "assumed"),
    )
    .unwrap_or_else(|e| panic!("{premise} assumed in {step_id}: {e}"));
}

fn list(items: &[&str]) -> reflow2_core::foundation::core::Value {
    reflow2_core::foundation::core::Value::List(
        items
            .iter()
            .map(|s| reflow2_core::foundation::core::Value::String(s.to_string()))
            .collect(),
    )
}

fn set_list(g: &mut DesignGraph, id: &str, key: &str, items: &[&str]) {
    let ty = type_of(g, id);
    g.upsert_node(&ty, id, Props::new().set(key, list(items)))
        .unwrap_or_else(|e| panic!("{id}.{key}: {e}"));
}

fn json<T: serde::Serialize>(v: T) -> Json {
    serde_json::to_value(v).expect("serializes")
}

fn gaps(g: &DesignGraph) -> Vec<Json> {
    g.detect_gaps()
        .unwrap()
        .into_iter()
        .map(|g| json(&g))
        .collect()
}

fn gaps_of(g: &DesignGraph, source: &str) -> Vec<Json> {
    gaps(g)
        .into_iter()
        .filter(|x| x["gap_source"] == source)
        .collect()
}

fn report(g: &DesignGraph, claim: Option<&str>) -> Json {
    json(g.reasoning_report(claim, Some(200)).unwrap())
}

fn claim_in<'a>(r: &'a Json, id: &str) -> &'a Json {
    r["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("{id} not in the read-back: {r:#}"))
}

/// Euclid's proof as a REDUCTIO, recorded honestly. Lemma 6 (no finite list
/// holds every prime) is shown directly; then:
///
/// - `req:suppose-finite` — "suppose the primes are finite, listed p1..pk" —
///   is an ASSUMED premise of `inf:absurd`, which derives the contradiction
///   `req:contradiction` from it, Lemma 6 and the definition of finite;
/// - `inf:theorem` concludes "there are infinitely many primes" from the
///   contradiction, and (with `discharge`) DISCHARGES the supposition;
/// - the theorem refutes the supposition (CONTRADICTS), as it should.
fn euclid_by_contradiction(g: &mut DesignGraph, discharge: bool) {
    for a in ["con:def-prime", "con:def-finite", "con:axiom-arith"] {
        axiom(g, a);
    }
    for s in [
        "req:lemma-euclid",
        "req:suppose-finite",
        "req:contradiction",
        "req:theorem",
    ] {
        statement(g, s);
    }
    step(
        g,
        "inf:euclid",
        &["con:def-prime", "con:axiom-arith"],
        "req:lemma-euclid",
    );
    step(
        g,
        "inf:absurd",
        &["req:lemma-euclid", "con:def-finite"],
        "req:contradiction",
    );
    assume(g, "req:suppose-finite", "inf:absurd");
    step(
        g,
        "inf:theorem",
        &["req:contradiction", "con:def-finite"],
        "req:theorem",
    );
    if discharge {
        set_list(g, "inf:theorem", "discharges", &["req:suppose-finite"]);
    }
    let (tt, st) = (type_of(g, "req:theorem"), type_of(g, "req:suppose-finite"));
    g.create_edge(
        "CONTRADICTS",
        &tt,
        "req:theorem",
        &st,
        "req:suppose-finite",
        Props::new(),
    )
    .unwrap();
}

#[test]
fn a_reductio_holds_its_supposition_as_an_assumption_and_the_theorem_does_not_rest_on_it() {
    let mut g = graph();
    euclid_by_contradiction(&mut g, true);

    // The supposition is FALSE — the theorem refutes it — and the step that
    // assumes it still holds: an assumption is held inside the argument.
    assert_eq!(
        g.premise_truth("req:suppose-finite").unwrap().0,
        Truth::Fails
    );
    let absurd = json(g.inference_reading("inf:absurd").unwrap().unwrap());
    assert_eq!(absurd["verdict"], "holds", "{absurd:#}");
    let assumed = absurd["premises"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "req:suppose-finite")
        .expect("the supposition is a premise");
    assert_eq!(assumed["role"], "assumed", "{absurd:#}");
    assert_eq!(absurd["assumes"], serde_json::json!(["req:suppose-finite"]));

    // Inside the argument, the contradiction holds only under the supposition.
    let inside = json(g.proof_reading("req:contradiction").unwrap().unwrap());
    assert_eq!(inside["status"], "conditional", "{inside:#}");

    // The theorem holds outright: its step discharges the supposition, so it
    // rests on no assumption — truth, foundations and proof status.
    assert_eq!(g.premise_truth("req:theorem").unwrap().0, Truth::Holds);
    let proof = json(g.proof_reading("req:theorem").unwrap().unwrap());
    assert_eq!(proof["status"], "shown", "{proof:#}");
    let foundations: Vec<String> = g
        .foundations_of("req:theorem")
        .unwrap()
        .into_iter()
        .map(|f| f.id)
        .collect();
    assert!(
        !foundations.contains(&"req:suppose-finite".to_string()),
        "an assumption is never a foundation: {foundations:?}"
    );
    let theorem = json(g.inference_reading("inf:theorem").unwrap().unwrap());
    assert!(theorem.get("assumes").is_none(), "{theorem:#}");
    assert_eq!(
        theorem["discharges"],
        serde_json::json!(["req:suppose-finite"])
    );

    // The read-back says "assumed, discharged by inf:theorem", on the list and
    // on the premise itself.
    let r = report(&g, Some("req:theorem"));
    let a = &r["assumptions"][0];
    assert_eq!(a["id"], "req:suppose-finite", "{r:#}");
    assert_eq!(a["reads"], "assumed, discharged by inf:theorem");
    let contradiction = claim_in(&r, "req:contradiction");
    let why = contradiction["support"][0]["premises"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "req:suppose-finite")
        .unwrap()["why"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        why.starts_with("assumed, discharged by inf:theorem"),
        "{why}"
    );
    // Not something the reasoning rests on, and never an uncited premise.
    assert!(
        !r["rests_on"]
            .as_array()
            .unwrap()
            .contains(&"req:suppose-finite".into())
    );
    assert!(
        gaps_of(&g, "reasoning_not_as_stated").is_empty(),
        "{:#?}",
        gaps(&g)
    );
}

#[test]
fn losing_a_discharged_assumption_costs_the_theorem_nothing_in_the_what_if() {
    let mut g = graph();
    euclid_by_contradiction(&mut g, true);
    // SINCE 2026-10-20 (req:a-what-if-on-an-assumed-principle-shows-what-
    // holds-only-under-it) a what-if on an assumption reaches the steps that
    // take it up, and reads what held only under it as no longer supported —
    // the reductio's inner claims. The theorem, which DISCHARGES it, is
    // untouched: its row reads still supported, and nothing reads refuted.
    let rows = json(g.inference_walk(&["req:suppose-finite"]).unwrap());
    let theorem_row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["inference"] == "inf:theorem");
    if let Some(t) = theorem_row {
        assert_eq!(t["conclusion_reads"], "still_supported", "{rows:#}");
    }
    assert!(
        rows.as_array()
            .unwrap()
            .iter()
            .all(|r| r["conclusion_reads"] != "refuted"),
        "{rows:#}"
    );
    // Losing the lemma the reductio really rests on still costs the theorem.
    let rows = json(g.inference_walk(&["req:lemma-euclid"]).unwrap());
    let theorem_row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["inference"] == "inf:theorem")
        .expect("the walk reaches the theorem's step");
    assert_eq!(theorem_row["conclusion_reads"], "unsupported", "{rows:#}");
}

#[test]
fn an_assumption_no_step_discharges_is_reported_and_its_claims_read_conditional() {
    let mut g = graph();
    euclid_by_contradiction(&mut g, false);

    let proof = json(g.proof_reading("req:theorem").unwrap().unwrap());
    assert_eq!(proof["status"], "conditional", "{proof:#}");
    assert!(
        proof["why"]
            .as_str()
            .unwrap()
            .contains("req:suppose-finite"),
        "{proof:#}"
    );
    let f = json(g.foundations_reading("req:theorem").unwrap().unwrap());
    assert_eq!(
        f["assumes"],
        serde_json::json!(["req:suppose-finite"]),
        "{f:#}"
    );

    let r = report(&g, None);
    let a = &r["assumptions"][0];
    assert!(
        a["reads"]
            .as_str()
            .unwrap()
            .starts_with("assumed, never discharged"),
        "{r:#}"
    );
    let found = gaps_of(&g, "reasoning_not_as_stated");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0]["title"]
            .as_str()
            .unwrap()
            .contains("never discharged"),
        "{found:#?}"
    );
    let debt = json(g.loop_status().unwrap())["reasoning"].clone();
    assert_eq!(debt["undischarged_assumptions"], 1, "{debt:#}");
    assert_eq!(
        debt["undischarged_assumption_ids"],
        serde_json::json!(["req:suppose-finite"])
    );
}

/// An induction: P(1) by computation-free arithmetic; the inductive step
/// assumes P(k) and derives P(k+1); the step concluding "P(k) implies P(k+1)"
/// discharges P(k) and generalizes over k; induction concludes P(n) for all n.
fn induction(g: &mut DesignGraph) {
    for a in ["con:axiom-arith", "con:axiom-induction"] {
        axiom(g, a);
    }
    for s in [
        "req:p-1",
        "req:p-k",
        "req:p-k-plus-1",
        "req:p-k-implies-p-k-plus-1",
        "req:p-all-n",
    ] {
        statement(g, s);
    }
    set_list(g, "req:p-k", "variables", &["k"]);
    set_list(g, "req:p-k-plus-1", "variables", &["k"]);
    step(g, "inf:base", &["con:axiom-arith"], "req:p-1");
    step(g, "inf:from-p-k", &["con:axiom-arith"], "req:p-k-plus-1");
    assume(g, "req:p-k", "inf:from-p-k");
    step(
        g,
        "inf:step",
        &["req:p-k-plus-1"],
        "req:p-k-implies-p-k-plus-1",
    );
    set_list(g, "inf:step", "discharges", &["req:p-k"]);
    set_list(g, "inf:step", "generalizes_over", &["k"]);
    step(
        g,
        "inf:induction",
        &[
            "con:axiom-induction",
            "req:p-1",
            "req:p-k-implies-p-k-plus-1",
        ],
        "req:p-all-n",
    );
}

#[test]
fn an_induction_step_discharges_its_hypothesis_and_generalizes_over_k() {
    let mut g = graph();
    induction(&mut g);
    assert_eq!(g.premise_truth("req:p-k-plus-1").unwrap().0, Truth::Holds);
    assert_eq!(
        json(g.proof_reading("req:p-k-plus-1").unwrap().unwrap())["status"],
        "conditional"
    );
    let step_reading = json(g.inference_reading("inf:step").unwrap().unwrap());
    assert_eq!(step_reading["verdict"], "holds", "{step_reading:#}");
    assert_eq!(
        step_reading["generalization"]["holds"], true,
        "{step_reading:#}"
    );
    assert_eq!(
        step_reading["generalization"]["mentioned_by"],
        serde_json::json!(["req:p-k", "req:p-k-plus-1"])
    );
    assert_eq!(
        json(g.proof_reading("req:p-all-n").unwrap().unwrap())["status"],
        "shown"
    );
    assert!(gaps_of(&g, "reasoning_not_as_stated").is_empty());

    // Fix k somewhere beneath the generalization and the step stops holding:
    // a statement about k = 3 proves nothing for an arbitrary k.
    set_list(&mut g, "req:p-k", "fixes", &["k=3"]);
    let step_reading = json(g.inference_reading("inf:step").unwrap().unwrap());
    assert_eq!(step_reading["verdict"], "unknown", "{step_reading:#}");
    assert!(
        step_reading["note"]
            .as_str()
            .unwrap()
            .contains("req:p-k fixes k=3"),
        "{step_reading:#}"
    );
    // Its conclusion is unsupported, never refuted.
    assert_eq!(
        g.premise_truth("req:p-k-implies-p-k-plus-1").unwrap().0,
        Truth::Unknown
    );
}

/// Euclid's Lemma 6 shown for "an arbitrary finite list L": the claims under
/// it mention L, and the step generalizes over it.
fn euclid_for_an_arbitrary_list(g: &mut DesignGraph) {
    for a in ["con:def-prime", "con:axiom-arith"] {
        axiom(g, a);
    }
    for s in [
        "req:e1-n-at-least-2",
        "req:e4-n-has-prime-divisor",
        "req:e5-q-not-listed",
        "req:lemma-euclid",
    ] {
        statement(g, s);
    }
    for s in [
        "req:e1-n-at-least-2",
        "req:e4-n-has-prime-divisor",
        "req:e5-q-not-listed",
    ] {
        set_list(g, s, "variables", &["L"]);
    }
    step(
        g,
        "inf:e1",
        &["con:axiom-arith", "con:def-prime"],
        "req:e1-n-at-least-2",
    );
    step(
        g,
        "inf:e4",
        &["req:e1-n-at-least-2", "con:def-prime"],
        "req:e4-n-has-prime-divisor",
    );
    step(
        g,
        "inf:e5",
        &["req:e4-n-has-prime-divisor"],
        "req:e5-q-not-listed",
    );
    step(
        g,
        "inf:euclid",
        &["req:e5-q-not-listed"],
        "req:lemma-euclid",
    );
    set_list(g, "inf:euclid", "generalizes_over", &["L"]);
}

#[test]
fn a_generalization_over_l_holds_until_a_premise_in_its_chain_fixes_l() {
    let mut g = graph();
    euclid_for_an_arbitrary_list(&mut g);
    let r = json(g.inference_reading("inf:euclid").unwrap().unwrap());
    assert_eq!(r["verdict"], "holds", "{r:#}");
    assert_eq!(r["generalization"]["over"], serde_json::json!(["L"]));
    assert_eq!(r["generalization"]["holds"], true);
    assert_eq!(g.premise_truth("req:lemma-euclid").unwrap().0, Truth::Holds);

    // Two levels down, E1 is restated for one list, L = (2, 3, 5).
    set_list(&mut g, "req:e1-n-at-least-2", "fixes", &["L=2,3,5"]);
    let r = json(g.inference_reading("inf:euclid").unwrap().unwrap());
    assert_eq!(r["verdict"], "unknown", "{r:#}");
    assert_eq!(
        r["generalization"]["fixed_by"],
        serde_json::json!([{"id": "req:e1-n-at-least-2", "fixes": ["L=2,3,5"]}])
    );
    assert_eq!(
        g.premise_truth("req:lemma-euclid").unwrap().0,
        Truth::Unknown
    );
    let found = gaps_of(&g, "reasoning_not_as_stated");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0]["title"].as_str().unwrap().contains("inf:euclid"));
    assert_eq!(
        json(g.loop_status().unwrap())["reasoning"]["generalizations_not_holding"],
        1
    );

    // Another variable fixed is no violation.
    set_list(&mut g, "req:e1-n-at-least-2", "fixes", &["n=40"]);
    assert_eq!(
        json(g.inference_reading("inf:euclid").unwrap().unwrap())["verdict"],
        "holds"
    );
}

#[test]
fn a_computation_at_one_value_does_not_carry_a_generalization() {
    let mut g = graph();
    axiom(&mut g, "con:def-prime");
    g.add_verification(
        "ver:calc-n40",
        "n² + n + 41 at n = 40",
        Some("analysis"),
        None,
        None,
    )
    .unwrap();
    g.set_verification_status(
        "ver:calc-n40",
        "passing",
        Some("2026-10-17"),
        Some("1681 = 41·41"),
    )
    .unwrap();
    set_list(&mut g, "ver:calc-n40", "fixes", &["n=40"]);
    statement(&mut g, "req:always-prime");
    step(
        &mut g,
        "inf:bad",
        &["ver:calc-n40", "con:def-prime"],
        "req:always-prime",
    );
    set_list(&mut g, "inf:bad", "generalizes_over", &["n"]);
    let r = json(g.inference_reading("inf:bad").unwrap().unwrap());
    assert_eq!(r["verdict"], "unknown", "{r:#}");
    assert!(
        r["note"]
            .as_str()
            .unwrap()
            .contains("ver:calc-n40 fixes n=40"),
        "{r:#}"
    );
}

#[test]
fn a_statement_says_what_it_is_and_a_conjecture_reads_open_then_refuted() {
    let mut g = graph();
    for (id, kind) in [
        ("con:def-prime", "definition"),
        ("req:conjecture", "conjecture"),
        ("req:theorem", "theorem"),
        ("req:counterexample", "lemma"),
    ] {
        if id.starts_with("con:") {
            axiom(&mut g, id);
        } else {
            statement(&mut g, id);
        }
        let ty = type_of(&g, id);
        g.upsert_node(&ty, id, Props::new().set("statement_kind", kind))
            .unwrap();
    }
    step(&mut g, "inf:x", &["con:def-prime"], "req:counterexample");

    let r = report(&g, None);
    let all = report(&g, Some("req:conjecture"));
    assert_eq!(
        claim_in(&all, "req:conjecture")["statement"]["reads"],
        "open",
        "{all:#}"
    );
    let theorem = report(&g, Some("req:theorem"));
    assert_eq!(
        claim_in(&theorem, "req:theorem")["statement"]["reads"],
        "stated"
    );
    let lemma = claim_in(&r, "req:counterexample");
    assert_eq!(lemma["statement"]["kind"], "lemma", "{r:#}");
    assert_eq!(lemma["statement"]["reads"], "shown");
    // A definition reads taken as given, with no `taken_as_given` written:
    // it is a stated foundation, never an uncited premise.
    let def = claim_in(&r, "con:def-prime");
    assert_eq!(def["statement"]["reads"], "taken_as_given");
    assert_eq!(def["taken_as_given"], true);
    assert_eq!(
        r["stated_foundations"],
        serde_json::json!(["con:def-prime"])
    );
    assert!(
        r["uncited_premises"].as_array().unwrap().is_empty(),
        "{r:#}"
    );

    // The counterexample, once it holds, refutes the conjecture.
    let (ct, kt) = (
        type_of(&g, "req:counterexample"),
        type_of(&g, "req:conjecture"),
    );
    g.create_edge(
        "CONTRADICTS",
        &ct,
        "req:counterexample",
        &kt,
        "req:conjecture",
        Props::new(),
    )
    .unwrap();
    let all = report(&g, Some("req:conjecture"));
    assert_eq!(
        claim_in(&all, "req:conjecture")["statement"]["reads"],
        "refuted",
        "{all:#}"
    );
}

/// A corollary and a proposition read as a theorem or lemma does: `stated`
/// until a step concludes them, then the conclusion's proof status. Neither
/// is taken as given, and both are values `statement_kind` may say (Anthony,
/// 2026-10-17: "Add corollary and proposition").
#[test]
fn a_corollary_and_a_proposition_read_stated_until_a_step_concludes_them() {
    for kind in ["corollary", "proposition"] {
        assert!(
            reflow2_core::inference::STATEMENT_KINDS.contains(&kind),
            "{kind} is a statement kind"
        );
        let mut g = graph();
        axiom(&mut g, "con:def");
        g.upsert_node(
            "Constraint",
            "con:def",
            Props::new().set("statement_kind", "definition"),
        )
        .unwrap();
        statement(&mut g, "req:result");
        g.upsert_node(
            "Requirement",
            "req:result",
            Props::new().set("statement_kind", kind),
        )
        .unwrap_or_else(|e| panic!("statement_kind {kind}: {e}"));

        let read = json(g.statement_reading("req:result").unwrap().unwrap());
        assert_eq!(read["kind"], kind, "{read:#}");
        assert_eq!(read["reads"], "stated", "{read:#}");
        assert_eq!(
            read["why"],
            format!("a {kind} no step concludes yet: stated, not shown")
        );

        step(&mut g, "inf:result", &["con:def"], "req:result");
        let read = json(g.statement_reading("req:result").unwrap().unwrap());
        assert_eq!(read["kind"], kind, "{read:#}");
        assert_eq!(read["reads"], "shown", "{read:#}");
        let r = report(&g, None);
        let claim = claim_in(&r, "req:result");
        assert!(claim.get("taken_as_given").is_none(), "{kind}: {r:#}");
        assert_eq!(claim["statement"]["reads"], "shown", "{kind}: {r:#}");

        // Used as a premise of a further step, it is never a stated
        // foundation: only a definition or an axiom is taken as given.
        statement(&mut g, "req:next");
        step(&mut g, "inf:next", &["req:result"], "req:next");
        let r = report(&g, None);
        assert_eq!(
            r["stated_foundations"],
            serde_json::json!(["con:def"]),
            "{kind}: {r:#}"
        );
    }
}

/// The trial's two proofs of the theorem — Euclid's and Goldbach's — sharing
/// Lemma 5 (and Lemma 4 and Lemma 1 under it) and Lemma 2.
fn two_proofs(g: &mut DesignGraph) {
    for a in [
        "con:def-prime",
        "con:def-divides",
        "con:axiom-arith",
        "con:def-finite",
    ] {
        axiom(g, a);
    }
    for s in [
        "req:l1",
        "req:l2",
        "req:l4",
        "req:l5",
        "req:e3",
        "req:e4",
        "req:euclid",
        "req:f1",
        "req:f5",
        "req:theorem",
    ] {
        statement(g, s);
    }
    step(
        g,
        "inf:l1",
        &["con:def-divides", "con:axiom-arith"],
        "req:l1",
    );
    step(
        g,
        "inf:l2",
        &["con:def-divides", "con:axiom-arith"],
        "req:l2",
    );
    step(
        g,
        "inf:l4",
        &["con:def-divides", "con:axiom-arith"],
        "req:l4",
    );
    step(
        g,
        "inf:l5",
        &["con:def-prime", "req:l1", "req:l4"],
        "req:l5",
    );
    step(g, "inf:e3", &["con:def-prime", "req:l2"], "req:e3");
    step(g, "inf:e4", &["con:def-prime", "req:l5"], "req:e4");
    step(g, "inf:euclid", &["req:e3", "req:e4"], "req:euclid");
    step(
        g,
        "inf:by-euclid",
        &["req:euclid", "con:def-finite"],
        "req:theorem",
    );
    step(g, "inf:f1", &["con:axiom-arith", "req:l2"], "req:f1");
    step(g, "inf:f5", &["req:f1", "req:l5"], "req:f5");
    step(
        g,
        "inf:by-goldbach",
        &["req:f5", "con:def-finite"],
        "req:theorem",
    );
    set_list(g, "inf:by-goldbach", "independent_of", &["inf:by-euclid"]);
}

#[test]
fn two_lines_stated_independent_are_checked_against_the_lemmas_the_statement_names() {
    let mut g = graph();
    two_proofs(&mut g);
    // Named: Lemma 5 and Lemma 2. Lemma 4 and Lemma 1 lie under Lemma 5, and
    // the definitions share no step, so the statement holds.
    set_list(
        &mut g,
        "inf:by-goldbach",
        "independent_except",
        &["req:l2", "req:l5"],
    );
    let r = json(g.inference_reading("inf:by-goldbach").unwrap().unwrap());
    let ind = &r["independence"][0];
    assert_eq!(ind["of"], "inf:by-euclid", "{r:#}");
    assert_eq!(ind["holds"], true, "{r:#}");
    assert_eq!(ind["shares_named"], serde_json::json!(["req:l2", "req:l5"]));
    assert!(gaps_of(&g, "reasoning_not_as_stated").is_empty());

    // Name only Lemma 5, and the shared step under Lemma 2 makes it false.
    set_list(&mut g, "inf:by-goldbach", "independent_except", &["req:l5"]);
    let r = json(g.inference_reading("inf:by-goldbach").unwrap().unwrap());
    let ind = &r["independence"][0];
    assert_eq!(ind["holds"], false, "{r:#}");
    assert_eq!(ind["shared_steps"], serde_json::json!(["inf:l2"]));
    let found = gaps_of(&g, "reasoning_not_as_stated");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0]["title"]
            .as_str()
            .unwrap()
            .contains("stated independent")
    );
    let rep = report(&g, Some("req:theorem"));
    assert_eq!(rep["independence"][0]["holds"], false, "{rep:#}");
    // The theorem itself still holds: a false statement about the lines is
    // reported, and changes no truth.
    assert_eq!(g.premise_truth("req:theorem").unwrap().0, Truth::Holds);
}

#[test]
fn a_definition_recorded_as_a_constraint_carries_no_budget_direction() {
    let mut g = graph();
    axiom(&mut g, "con:def-prime");
    let def = g.get_node("Constraint", "con:def-prime").unwrap().unwrap();
    assert!(
        !def.properties.contains_key("direction"),
        "a definition carries no budget words: {:?}",
        def.properties
    );
    // A budget given a limit still reads `maximum`, exactly as before.
    g.add_constraint(
        "con:mass",
        "Mass",
        "Under 40 kg.",
        None,
        Some("mass_kg"),
        Some(40.0),
        None,
        None,
    )
    .unwrap();
    let mass = g.get_node("Constraint", "con:mass").unwrap().unwrap();
    assert_eq!(
        mass.properties.get("direction").and_then(|v| v.as_str()),
        Some("maximum")
    );

    // A store written before keeps its `direction` through an export and an
    // import: nothing is added and nothing is lost.
    g.upsert_node(
        "Constraint",
        "con:old-def",
        Props::new()
            .set("name", "an old definition")
            .set("statement", "stored by 0.85.0, with the default it wrote")
            .set("direction", "maximum"),
    )
    .unwrap();
    let doc = g.export_graph().unwrap();
    let mut back = graph();
    back.import_graph(&doc).unwrap();
    for id in ["con:def-prime", "con:mass", "con:old-def"] {
        assert_eq!(
            back.get_node("Constraint", id).unwrap().unwrap().properties,
            g.get_node("Constraint", id).unwrap().unwrap().properties,
            "{id} round-trips unchanged"
        );
    }
}
