//! A RESULT REFUTES ONLY ON THE OWNER'S SIGNED RULING, AND A CLAIM IT SUPPORTS
//! READS SUPPORTED GIVEN THE MODEL HELD ON PURPOSE.
//!
//! - `req:the-owner-can-rule-a-prediction-refuted-by-a-named-result`, governed
//!   by `dec:a-result-refutes-a-prediction-only-on-the-owners-ruling-2026-10-28`
//!   (Anthony 2026-10-28, gap 1 of the design-a-phenomenon skill, #807): a
//!   result that contradicts a prediction still only CONTESTS it on its own
//!   (`dec:idea-an-objection-resting-on-evidence-contests-or-refutes` stands).
//!   The owner rules with acts reflow2 already has: a Decision, accepted and
//!   signed, that CONCEDES the result (CONTRADICTS `alignment: conceding`),
//!   optionally concluded by a step from the result and the prediction it
//!   missed. The claim then reads refuted BY THE RULING, naming the ruling, the
//!   result and the model and idealizations held on purpose beneath it; the
//!   hypothesis discharged into the prediction reads refuted with it.
//!   Withdrawn, the result only contests again; unsigned, it never ruled; a
//!   signature naming someone else is refused.
//! - `req:a-claim-a-result-supports-reads-supported-given-the-model-held-on-purpose`
//!   (gap 2): a claim a documented result supports, resting otherwise only on
//!   premises held on purpose (`held_as` model or principle, #806), reads
//!   `supported` given them, naming each — never proven. One that still rests
//!   on an undischarged supposition, or on an undocumented report, reads as
//!   before.
//!
//! The design is the sinogram trial's cupping in miniature, as the
//! design-a-phenomenon skill lays it out. Every test here failed on main
//! 73b29e4df first, except the refusal of a signature naming someone else,
//! which is the signer rule every signature already meets (pinned here so
//! the ruling stays behind it).

use reflow2_core::DesignGraph;
use reflow2_core::foundation::core::Value;
use reflow2_core::inference::InferenceSpec;
use reflow2_core::intent::Signer;
use reflow2_core::nodes::{Props, edge, node};
use serde_json::Value as Json;

const OWNER: &str = "who:owner";
const OTHER: &str = "who:someone-else";

fn json<T: serde::Serialize>(v: T) -> Json {
    serde_json::to_value(v).expect("serializes")
}

fn type_of(g: &DesignGraph, id: &str) -> String {
    let held = g.node_types_holding(id).expect("types holding");
    assert_eq!(held.len(), 1, "{id}: {held:?}");
    held[0].clone()
}

fn edge_to(g: &mut DesignGraph, from: &str, rel: &str, to: &str, props: Props) {
    let (ft, tt) = (type_of(g, from), type_of(g, to));
    g.create_edge(rel, &ft, from, &tt, to, props)
        .unwrap_or_else(|e| panic!("{from} {rel} {to}: {e}"));
}

fn statement(g: &mut DesignGraph, ty: &str, id: &str, kind: &str, given: bool) {
    let mut props = Props::new()
        .set("name", id)
        .set("statement", format!("FICTIONAL. {id}."))
        .set("statement_kind", kind);
    if given {
        props = props.set("taken_as_given", Value::Bool(true));
    }
    g.upsert_node(ty, id, props)
        .unwrap_or_else(|e| panic!("{id}: {e}"));
}

fn spec(id: &str, premises: &[&str], conclusion: &str) -> InferenceSpec {
    InferenceSpec {
        id: id.to_string(),
        rule: "all".to_string(),
        k: None,
        premises: premises.iter().map(|s| s.to_string()).collect(),
        conclusion: conclusion.to_string(),
        rule_name: None,
        warrant: Some("Stated so a reader can check it.".to_string()),
        name: None,
        confirmed_by: None,
        confirmed_how: None,
        confirmed_at: None,
        assumed: Vec::new(),
        held_as: None,
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
    }
}

fn infer(g: &mut DesignGraph, s: InferenceSpec) {
    let id = s.id.clone();
    g.add_inference(&s).unwrap_or_else(|e| panic!("{id}: {e}"));
}

/// A result: a statement of kind `evidence`, documented (or not) by its data.
fn result(g: &mut DesignGraph, id: &str, documented: bool) {
    g.upsert_node(
        node::CONSTRAINT,
        id,
        Props::new()
            .set("name", id)
            .set("statement", format!("FICTIONAL. Result {id}."))
            .set("statement_kind", "evidence")
            .set("source_reliability", "B")
            .set("information_credibility", "2")
            .set("reported_at", "2026-10-27"),
    )
    .unwrap();
    if documented {
        let art = format!("art:data-{}", id.trim_start_matches("con:"));
        g.upsert_node(
            node::ARTIFACT,
            &art,
            Props::new()
                .set("name", art.as_str())
                .set("artifact_type", "data")
                .set("location", format!("data/{art}.h5"))
                .set("checksum", "sha256:da7a"),
        )
        .unwrap();
        edge_to(g, &art, edge::DOCUMENTS, id, Props::new());
    }
}

/// THE CUPPING, AS THE SKILL LAYS IT OUT: an observation given; a model and
/// two idealizations held as a model; two rival hypotheses, each a
/// supposition discharged into its conditional prediction; the claims each
/// makes about the world; a documented result; the step by which it bears
/// for beam hardening; and a CONTRADICTS from it to the scatter claim.
fn cupping() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("in-memory graph");
    g.add_contributor(OWNER, "The owner", Some("person"), None, None)
        .unwrap();
    g.add_contributor(OTHER, "Someone else", Some("person"), None, None)
        .unwrap();
    statement(
        &mut g,
        node::CONSTRAINT,
        "con:obs-cupping",
        "proposition",
        true,
    );
    for id in [
        "con:model-attenuation",
        "con:idl-monochromatic",
        "con:idl-no-scatter",
    ] {
        statement(&mut g, node::CONSTRAINT, id, "proposition", false);
    }
    for (h, keeps, under, then) in [
        (
            "req:hyp-bh",
            "con:idl-no-scatter",
            "req:pred-bh-under",
            "req:if-bh-then-filtration",
        ),
        (
            "req:hyp-sc",
            "con:idl-monochromatic",
            "req:pred-sc-under",
            "req:if-sc-then-distance",
        ),
    ] {
        statement(&mut g, node::REQUIREMENT, h, "hypothesis", false);
        g.upsert_node(
            node::REQUIREMENT,
            h,
            Props::new().set("variables", Value::List(vec![])),
        )
        .unwrap();
        statement(&mut g, node::REQUIREMENT, under, "proposition", false);
        statement(&mut g, node::REQUIREMENT, then, "proposition", false);
        let short = h.trim_start_matches("req:hyp-");
        let step = format!("inf:{short}-predicts");
        let mut first = spec(&step, &["con:obs-cupping"], under);
        first.assumed = vec!["con:model-attenuation".into(), keeps.into()];
        first.held_as = Some("model".into());
        infer(&mut g, first);
        let mut second = spec(&step, &["con:obs-cupping"], under);
        second.assumed = vec![h.into()];
        infer(&mut g, second);
        let mut cond = spec(&format!("inf:if-{short}"), &[under], then);
        cond.discharges = Some(vec![h.into()]);
        infer(&mut g, cond);
    }
    for c in ["req:claim-bh", "req:claim-sc"] {
        statement(&mut g, node::REQUIREMENT, c, "conjecture", false);
    }
    result(&mut g, "con:result-filtration", true);
    infer(
        &mut g,
        spec(
            "inf:result-bears-for-bh",
            &[
                "con:result-filtration",
                "req:if-bh-then-filtration",
                "req:if-sc-then-distance",
            ],
            "req:claim-bh",
        ),
    );
    edge_to(
        &mut g,
        "con:result-filtration",
        edge::CONTRADICTS,
        "req:claim-sc",
        Props::new().set("alignment", "opposing").set(
            "evidence",
            "scatter predicted a change with distance; none was seen",
        ),
    );
    g
}

/// THE OWNER'S RULING: a Decision concluded by a step from the result and
/// the prediction it missed, conceding the result. `signed` draws the
/// owner's approval; `accepted` sets it accepted.
fn rule(g: &mut DesignGraph, accepted: bool, signed: bool) {
    g.add_decision(
        "dec:ruling-scatter",
        "Ruling: the filtration result refutes scatter",
        "The cupping is not scatter: the result refutes the scatter hypothesis, given the model.",
        Some("The model and both idealizations are kept; the miss is laid on scatter."),
    )
    .unwrap();
    infer(
        g,
        spec(
            "inf:ruling-scatter",
            &["con:result-filtration", "req:if-sc-then-distance"],
            "dec:ruling-scatter",
        ),
    );
    edge_to(
        g,
        "dec:ruling-scatter",
        edge::CONTRADICTS,
        "con:result-filtration",
        Props::new()
            .set("alignment", "conceding")
            .set("evidence", "the owner rules the result refutes scatter"),
    );
    if accepted {
        g.set_decision_status("dec:ruling-scatter", "accepted")
            .unwrap();
    }
    if signed {
        g.authored_by(
            node::DECISION,
            "dec:ruling-scatter",
            OWNER,
            Some("approver"),
            None,
        )
        .unwrap();
    }
}

fn reads(g: &DesignGraph, id: &str) -> (String, String) {
    let r = g
        .statement_reading(id)
        .unwrap()
        .unwrap_or_else(|| panic!("{id} is a statement"));
    (r.reads.to_string(), r.why)
}

fn still_contested(g: &DesignGraph) -> Vec<String> {
    let rep = json(g.reasoning_report_from(None, Some(500), None).unwrap());
    rep["still_contested"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["claim"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_result_alone_contests_the_claim_it_goes_against_and_never_refutes_it() {
    let g = cupping();
    // Nothing derives the rival claim, so on its own it reads open; the
    // read-back lists it as still contested, with the result against it.
    let (sc, why) = reads(&g, "req:claim-sc");
    assert_eq!(sc, "open", "{why}");
    assert_eq!(
        g.premise_truth("req:claim-sc").unwrap().0.as_str(),
        "unknown"
    );
    let o = json(g.objections_to("req:claim-sc").unwrap());
    assert_eq!(o[0]["standing"], "stands", "{o:#}");
    assert!(o[0].get("ruling").is_none(), "{o:#}");
    assert_eq!(still_contested(&g), vec!["req:claim-sc".to_string()]);
    assert_eq!(reads(&g, "req:hyp-sc").0, "discharged");
}

#[test]
fn the_owners_ruling_refutes_it_naming_the_ruling_the_result_and_the_held_model() {
    let mut g = cupping();
    rule(&mut g, true, true);
    let (sc, why) = reads(&g, "req:claim-sc");
    assert_eq!(sc, "refuted", "{why}");
    for named in [
        "the owner's ruling dec:ruling-scatter",
        OWNER,
        "on the result con:result-filtration",
        "con:model-attenuation",
        "con:idl-monochromatic",
        "A result alone only contests",
    ] {
        assert!(why.contains(named), "the reading names {named:?}: {why}");
    }
    // The idealization the scatter prediction dropped is not a condition.
    assert!(!why.contains("con:idl-no-scatter"), "{why}");

    // The objection carries the ruling whole.
    let o = json(g.objections_to("req:claim-sc").unwrap());
    assert_eq!(o[0]["standing"], "conceded", "{o:#}");
    assert_eq!(o[0]["conceded_by"], "dec:ruling-scatter", "{o:#}");
    let r = &o[0]["ruling"];
    assert_eq!(r["ruling"], "dec:ruling-scatter", "{o:#}");
    assert_eq!(r["result"], "con:result-filtration", "{o:#}");
    assert_eq!(r["signed_by"], serde_json::json!([OWNER]), "{o:#}");
    assert_eq!(r["against"], serde_json::json!(["req:claim-sc"]), "{o:#}");
    assert_eq!(r["hypotheses"], serde_json::json!(["req:hyp-sc"]), "{o:#}");
    assert_eq!(
        r["given"],
        serde_json::json!(["con:idl-monochromatic", "con:model-attenuation"]),
        "{o:#}"
    );

    // The hypothesis discharged into the missed prediction reads refuted by
    // the same ruling; its rival does not.
    let (h, hwhy) = reads(&g, "req:hyp-sc");
    assert_eq!(h, "refuted", "{hwhy}");
    assert!(hwhy.contains("dec:ruling-scatter"), "{hwhy}");
    assert!(hwhy.contains("con:model-attenuation"), "{hwhy}");
    assert_eq!(reads(&g, "req:hyp-bh").0, "discharged");

    // The prediction itself, a conditional derived under the model, still
    // holds, and so does the claim the result bears for: refuting scatter
    // takes nothing from them.
    assert_eq!(
        g.premise_truth("req:if-sc-then-distance")
            .unwrap()
            .0
            .as_str(),
        "holds"
    );
    assert_eq!(g.premise_truth("req:claim-bh").unwrap().0.as_str(), "holds");

    // The read-back: no longer still contested, listed as refuted by ruling.
    assert!(still_contested(&g).is_empty());
    let rep = json(g.reasoning_report_from(None, Some(500), None).unwrap());
    let ruled = &rep["refuted_by_ruling"];
    assert_eq!(ruled[0]["ruling"], "dec:ruling-scatter", "{rep:#}");
    assert_eq!(ruled[0]["result"], "con:result-filtration", "{rep:#}");

    // loop_status's reasoning block names it.
    let debt = json(g.reasoning_debt().unwrap().unwrap());
    assert_eq!(debt["rulings"], 1, "{debt:#}");
    assert_eq!(
        debt["rulings_in_force"][0]["given"],
        serde_json::json!(["con:idl-monochromatic", "con:model-attenuation"]),
        "{debt:#}"
    );
}

#[test]
fn a_ruling_withdrawn_leaves_the_result_contesting_again() {
    let mut g = cupping();
    rule(&mut g, true, true);
    assert_eq!(reads(&g, "req:claim-sc").0, "refuted");
    g.set_decision_status("dec:ruling-scatter", "rejected")
        .unwrap();
    // Nothing derives the rival claim, so on its own it reads open; the
    // read-back lists it as still contested, with the result against it.
    let (sc, why) = reads(&g, "req:claim-sc");
    assert_eq!(sc, "open", "{why}");
    assert_eq!(still_contested(&g), vec!["req:claim-sc".to_string()]);
    assert_eq!(reads(&g, "req:hyp-sc").0, "discharged");
    let debt = json(g.reasoning_debt().unwrap().unwrap());
    assert!(debt.get("rulings_in_force").is_none(), "{debt:#}");
}

#[test]
fn a_ruling_nobody_signed_is_no_ruling() {
    let mut g = cupping();
    rule(&mut g, true, false);
    // Nothing derives the rival claim, so on its own it reads open; the
    // read-back lists it as still contested, with the result against it.
    let (sc, why) = reads(&g, "req:claim-sc");
    assert_eq!(sc, "open", "{why}");
    let o = json(g.objections_to("req:claim-sc").unwrap());
    assert_eq!(o[0]["standing"], "stands", "{o:#}");
    assert!(
        o[0]["why"].as_str().unwrap().contains("no approver signed"),
        "{o:#}"
    );
}

#[test]
fn a_ruling_signed_as_someone_else_is_refused_and_the_result_still_contests() {
    let mut g = cupping();
    rule(&mut g, true, false);
    g.begin_signing(Signer::Caller {
        contributor: OWNER.into(),
        how: "named by the test gateway".into(),
    });
    let refused = g.authored_by(
        node::DECISION,
        "dec:ruling-scatter",
        OTHER,
        Some("approver"),
        None,
    );
    assert!(
        refused.is_err(),
        "a signature naming someone else is refused"
    );
    assert_eq!(reads(&g, "req:claim-sc").0, "open");
    assert_eq!(still_contested(&g), vec!["req:claim-sc".to_string()]);
    // Signed as themselves, it rules.
    g.authored_by(
        node::DECISION,
        "dec:ruling-scatter",
        OWNER,
        Some("approver"),
        None,
    )
    .expect("the caller signs as themselves");
    assert_eq!(reads(&g, "req:claim-sc").0, "refuted");
}

#[test]
fn a_what_if_on_the_held_model_or_the_ruling_names_the_ruling_lost() {
    let mut g = cupping();
    rule(&mut g, true, true);
    for (seed, taken) in [
        ("con:idl-monochromatic", "con:idl-monochromatic"),
        ("dec:ruling-scatter", "dec:ruling-scatter"),
    ] {
        let walk = json(g.propagate_from(&[seed], Default::default()).unwrap());
        let lost = &walk["rulings_lost"];
        assert_eq!(lost[0]["ruling"], "dec:ruling-scatter", "{seed}: {walk:#}");
        assert_eq!(
            lost[0]["result"], "con:result-filtration",
            "{seed}: {walk:#}"
        );
        assert_eq!(
            lost[0]["against"],
            serde_json::json!(["req:claim-sc"]),
            "{seed}: {walk:#}"
        );
        assert!(
            lost[0]["taken"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t == taken),
            "{seed}: {walk:#}"
        );
    }
    // The idealization the scatter prediction does not rest on takes nothing.
    let walk = json(
        g.propagate_from(&["con:idl-no-scatter"], Default::default())
            .unwrap(),
    );
    assert!(walk.get("rulings_lost").is_none(), "{walk:#}");
}

#[test]
fn a_claim_a_documented_result_supports_reads_supported_given_the_held_model() {
    let g = cupping();
    let proof = json(g.proof_reading("req:claim-bh").unwrap().unwrap());
    assert_eq!(proof["status"], "supported", "{proof:#}");
    let given = serde_json::json!([
        "con:idl-monochromatic",
        "con:idl-no-scatter",
        "con:model-attenuation"
    ]);
    assert_eq!(proof["given"], given, "{proof:#}");
    let why = proof["why"].as_str().unwrap();
    assert!(why.contains("con:result-filtration"), "{why}");
    assert!(
        why.contains("con:model-attenuation (held as a model)"),
        "{why}"
    );
    assert!(why.contains("never proven"), "{why}");
    assert_eq!(reads(&g, "req:claim-bh").0, "supported");

    let rep = json(
        g.reasoning_report_from(Some("req:claim-bh"), None, None)
            .unwrap(),
    );
    let claim = rep["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "req:claim-bh")
        .unwrap()
        .clone();
    assert_eq!(claim["truth"], "supported", "{claim:#}");
    assert_eq!(claim["given"], given, "{claim:#}");
    assert_eq!(
        claim["rests_on_tests"],
        serde_json::json!(["con:result-filtration"]),
        "{claim:#}"
    );
    // The step reads supported too.
    let step = json(
        g.inference_reading("inf:result-bears-for-bh")
            .unwrap()
            .unwrap(),
    );
    assert_eq!(step["holds_as"], "supported", "{step:#}");
    // The predictions, resting on the model alone, stay conditional.
    let pred = json(
        g.proof_reading("req:if-bh-then-filtration")
            .unwrap()
            .unwrap(),
    );
    assert_eq!(pred["status"], "conditional", "{pred:#}");
}

#[test]
fn an_undischarged_supposition_or_an_undocumented_result_reads_as_before() {
    // An undischarged supposition under the claim.
    let mut g = cupping();
    statement(
        &mut g,
        node::REQUIREMENT,
        "req:supposed",
        "proposition",
        false,
    );
    let mut s = spec("inf:result-bears-for-bh", &[], "req:claim-bh");
    s.premises = vec![
        "con:result-filtration".into(),
        "req:if-bh-then-filtration".into(),
        "req:if-sc-then-distance".into(),
    ];
    s.assumed = vec!["req:supposed".into()];
    infer(&mut g, s);
    let proof = json(g.proof_reading("req:claim-bh").unwrap().unwrap());
    assert_eq!(proof["status"], "conditional", "{proof:#}");
    assert!(proof.get("given").is_none(), "{proof:#}");
    assert_eq!(reads(&g, "req:claim-bh").0, "conditional");

    // A result no pinned source documents.
    let mut g = cupping();
    result(&mut g, "con:result-undocumented", false);
    let s = spec(
        "inf:result-bears-for-bh",
        &[
            "con:result-undocumented",
            "req:if-bh-then-filtration",
            "req:if-sc-then-distance",
        ],
        "req:claim-bh",
    );
    // Revise the step onto the undocumented result alone.
    g.delete_edge(
        edge::PREMISE_OF,
        "con:result-filtration",
        "inf:result-bears-for-bh",
    )
    .unwrap();
    infer(&mut g, s);
    let proof = json(g.proof_reading("req:claim-bh").unwrap().unwrap());
    assert_eq!(proof["status"], "conditional", "{proof:#}");
    assert!(proof.get("given").is_none(), "{proof:#}");
}

/// A DESIGN THAT USES NEITHER READS AS BEFORE: assumptions nobody said are
/// held on purpose read conditional beside evidence, as on main; and a
/// concession of an objection that is not a result, even by a Decision,
/// counts as it always did.
#[test]
fn a_design_using_neither_the_ruling_nor_held_as_reads_as_before() {
    let mut g = DesignGraph::open_in_memory().unwrap();
    statement(&mut g, node::CONSTRAINT, "con:model", "proposition", false);
    statement(&mut g, node::REQUIREMENT, "req:claim", "proposition", false);
    result(&mut g, "con:result", true);
    let mut s = spec("inf:claim", &["con:result"], "req:claim");
    s.assumed = vec!["con:model".into()];
    infer(&mut g, s);
    let proof = json(g.proof_reading("req:claim").unwrap().unwrap());
    assert_eq!(proof["status"], "conditional", "{proof:#}");
    assert!(proof.get("given").is_none(), "{proof:#}");

    // A Decision conceding an objection that is not a result: refutes, as a
    // concession always did, signed or not.
    statement(&mut g, node::REQUIREMENT, "req:target", "conjecture", false);
    statement(
        &mut g,
        node::REQUIREMENT,
        "req:objection",
        "objection",
        false,
    );
    edge_to(
        &mut g,
        "req:objection",
        edge::CONTRADICTS,
        "req:target",
        Props::new().set("alignment", "opposing"),
    );
    g.add_decision("dec:grant", "Grant it", "Granted.", None)
        .unwrap();
    g.set_decision_status("dec:grant", "accepted").unwrap();
    edge_to(
        &mut g,
        "dec:grant",
        edge::CONTRADICTS,
        "req:objection",
        Props::new().set("alignment", "conceding"),
    );
    let (t, why) = reads(&g, "req:target");
    assert_eq!(t, "refuted", "{why}");
    assert!(why.contains("conceded by dec:grant"), "{why}");
    assert!(g.rulings().unwrap().is_empty());
}
