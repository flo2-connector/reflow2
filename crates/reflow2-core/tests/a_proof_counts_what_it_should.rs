//! A PROOF COUNTS WHAT IT SHOULD — phase 1 of the fixes Anthony accepted on
//! 2026-10-17 after the first proof trial on flo2.io
//! (fact:the-primes-trial-rung-1-ran-on-flo2-io-and-its-friction-names-what-counts-2026-10-17).
//!
//! Each test is one accepted requirement, on the trial's own shapes:
//!
//! - two proofs of "there are infinitely many primes" (Euclid's and
//!   Goldbach's) that share Lemma 5, "every n ≥ 2 has a prime divisor";
//! - the conjecture "n² + n + 41 is prime for every n", refuted by the
//!   computed counterexample at n = 40, with the computed check of n = 0..39
//!   corroborating it (evidence, never a premise);
//! - computed checks with and without their kept calc records.
//!
//! Written against the generic write path, so each compiles on 0.85.0 and
//! fails there on the behaviour it pins (or on the new optional property the
//! schema refuses).

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

fn edge(g: &mut DesignGraph, ty: &str, from: &str, to: &str, props: Props) {
    let (ft, tt) = (type_of(g, from), type_of(g, to));
    g.create_edge(ty, &ft, from, &tt, to, props)
        .unwrap_or_else(|e| panic!("{from} {ty} {to}: {e}"));
}

/// A computed check, set passing with findings, its location naming a
/// `.calc.json` record. With `record`, the record is an Artifact with a
/// checksum, named in the location by id.
fn computed_check(g: &mut DesignGraph, id: &str, file: &str, record: Option<&str>) {
    g.add_verification(
        id,
        &format!("Computation {id}"),
        Some("analysis"),
        None,
        None,
    )
    .unwrap();
    g.set_verification_status(
        id,
        "passing",
        Some("2026-10-17"),
        Some("exact: the value is 41·41"),
    )
    .unwrap();
    let location = match record {
        Some(art) => {
            g.add_artifact(art, file, Some("data"), Some(file)).unwrap();
            g.upsert_node("Artifact", art, Props::new().set("checksum", "sha256:b2fa"))
                .unwrap();
            format!("{file} ({art})")
        }
        None => file.to_string(),
    };
    g.upsert_node("Verification", id, Props::new().set("location", location))
        .unwrap();
}

/// The trial's two proofs of one theorem, sharing Lemma 5 and Lemma 2.
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
        "req:l1-divides-transitive",
        "req:l2-divides-difference",
        "req:l4-least-divisor",
        "req:l5-prime-divisor",
        "req:e3-listed-dont-divide",
        "req:e4-n-has-prime-divisor",
        "req:euclid",
        "req:f1-fermat-odd",
        "req:f5-fermat-distinct-primes",
        "req:theorem",
    ] {
        statement(g, s);
    }
    step(
        g,
        "inf:l1",
        &["con:def-divides", "con:axiom-arith"],
        "req:l1-divides-transitive",
    );
    step(
        g,
        "inf:l2",
        &["con:def-divides", "con:axiom-arith"],
        "req:l2-divides-difference",
    );
    step(
        g,
        "inf:l4",
        &["con:def-divides", "con:axiom-arith"],
        "req:l4-least-divisor",
    );
    step(
        g,
        "inf:l5",
        &[
            "con:def-prime",
            "req:l1-divides-transitive",
            "req:l4-least-divisor",
        ],
        "req:l5-prime-divisor",
    );
    // Euclid's line.
    step(
        g,
        "inf:e3",
        &["con:def-prime", "req:l2-divides-difference"],
        "req:e3-listed-dont-divide",
    );
    step(
        g,
        "inf:e4",
        &["con:def-prime", "req:l5-prime-divisor"],
        "req:e4-n-has-prime-divisor",
    );
    step(
        g,
        "inf:euclid",
        &["req:e3-listed-dont-divide", "req:e4-n-has-prime-divisor"],
        "req:euclid",
    );
    step(
        g,
        "inf:theorem-by-euclid",
        &["req:euclid", "con:def-finite"],
        "req:theorem",
    );
    // Goldbach's line.
    step(
        g,
        "inf:f1",
        &["con:axiom-arith", "req:l2-divides-difference"],
        "req:f1-fermat-odd",
    );
    step(
        g,
        "inf:f5",
        &["req:f1-fermat-odd", "req:l5-prime-divisor"],
        "req:f5-fermat-distinct-primes",
    );
    step(
        g,
        "inf:theorem-by-goldbach",
        &["req:f5-fermat-distinct-primes", "con:def-finite"],
        "req:theorem",
    );
}

/// The refuted conjecture: a computed counterexample that holds objects to
/// it; the computed finite check corroborates it.
fn refuted_conjecture(g: &mut DesignGraph) {
    if g.node_types_holding("con:def-prime").unwrap().is_empty() {
        axiom(g, "con:def-prime");
    }
    statement(g, "req:conjecture-n2-n-41");
    statement(g, "req:counterexample-n40");
    statement(g, "req:finite-0-to-39");
    computed_check(g, "ver:calc-n40", "n40.calc.json", Some("art:calc-n40"));
    computed_check(g, "ver:calc-0-39", "n0-39.calc.json", Some("art:calc-0-39"));
    step(
        g,
        "inf:counterexample",
        &["ver:calc-n40", "con:def-prime"],
        "req:counterexample-n40",
    );
    step(
        g,
        "inf:finite",
        &["ver:calc-0-39", "con:def-prime"],
        "req:finite-0-to-39",
    );
    edge(
        g,
        "CONTRADICTS",
        "req:counterexample-n40",
        "req:conjecture-n2-n-41",
        Props::new(),
    );
    edge(
        g,
        "CONTRADICTS",
        "req:finite-0-to-39",
        "req:conjecture-n2-n-41",
        Props::new().set("alignment", "supporting"),
    );
}

fn walk(g: &DesignGraph, seeds: &[&str]) -> Vec<Json> {
    let radius = g
        .propagate_from(seeds, reflow2_core::PropagateOptions::default())
        .expect("propagate_from");
    serde_json::to_value(&radius).unwrap()["inferences"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn row<'a>(rows: &'a [Json], inference: &str) -> &'a Json {
    rows.iter()
        .find(|r| r["inference"] == inference)
        .unwrap_or_else(|| panic!("no walk row for {inference} in {rows:#?}"))
}

fn gap_sources(g: &DesignGraph) -> Vec<String> {
    g.detect_gaps()
        .unwrap()
        .iter()
        .map(|gap| {
            serde_json::to_value(gap).unwrap()["gap_source"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

// ─── 1. A computed fact counts only through its kept record ────────────────

#[test]
fn a_computed_check_with_no_kept_record_reads_asserted_not_passing() {
    let mut g = graph();
    axiom(&mut g, "con:def-prime");
    statement(&mut g, "req:counterexample-n40");
    // Set passing by hand, its location naming a calc record nothing keeps.
    computed_check(&mut g, "ver:calc-n40", "n40.calc.json", None);
    step(
        &mut g,
        "inf:counterexample",
        &["ver:calc-n40", "con:def-prime"],
        "req:counterexample-n40",
    );

    let (truth, why) = g.premise_truth("ver:calc-n40").unwrap();
    assert_eq!(truth, Truth::Unknown, "{why}");
    assert!(why.contains("asserted"), "the why names it asserted: {why}");
    assert!(
        why.contains("record"),
        "the why names the missing record: {why}"
    );
    let step = g.inference_reading("inf:counterexample").unwrap().unwrap();
    assert_eq!(step.verdict, Truth::Unknown);
    assert_eq!(
        g.premise_truth("req:counterexample-n40").unwrap().0,
        Truth::Unknown
    );
}

#[test]
fn a_computed_check_holds_through_a_kept_record_that_states_the_result() {
    let mut g = graph();
    computed_check(
        &mut g,
        "ver:calc-n40",
        "n40.calc.json",
        Some("art:calc-n40"),
    );
    let (truth, why) = g.premise_truth("ver:calc-n40").unwrap();
    assert_eq!(truth, Truth::Holds, "{why}");
    assert!(why.contains("art:calc-n40"), "{why}");

    // The same check flagged `computed`, named by DOCUMENTS alone, with a
    // plain location: still held up by its record.
    let mut g = graph();
    g.add_verification("ver:x", "Computation x", Some("analysis"), None, None)
        .unwrap();
    g.set_verification_status("ver:x", "passing", Some("2026-10-17"), Some("x = 3"))
        .unwrap();
    g.upsert_node("Verification", "ver:x", Props::new().set("computed", true))
        .expect("Verification.computed is a declared property");
    assert_eq!(
        g.premise_truth("ver:x").unwrap().0,
        Truth::Unknown,
        "no record yet"
    );
    g.add_artifact("art:x", "x record", Some("data"), Some("x.json"))
        .unwrap();
    g.documents("art:x", "Verification", "ver:x", None).unwrap();
    assert_eq!(
        g.premise_truth("ver:x").unwrap().0,
        Truth::Unknown,
        "a record with no checksum is not kept"
    );
    g.upsert_node(
        "Artifact",
        "art:x",
        Props::new().set("checksum", "sha256:11"),
    )
    .unwrap();
    assert_eq!(g.premise_truth("ver:x").unwrap().0, Truth::Holds);

    // The record says the run failed: the check fails, whatever its status.
    let (ft, tt) = ("Verification", "Artifact");
    g.create_edge(
        "PRODUCES",
        ft,
        "ver:x",
        tt,
        "art:x",
        Props::new().set("outcome", "fail"),
    )
    .unwrap();
    assert_eq!(g.premise_truth("ver:x").unwrap().0, Truth::Fails);
}

#[test]
fn a_record_that_drifted_from_its_checksum_is_not_kept() {
    let mut g = graph();
    computed_check(
        &mut g,
        "ver:calc-n40",
        "n40.calc.json",
        Some("art:calc-n40"),
    );
    g.create_node(
        "DriftEvent",
        "drift:n40",
        Props::new()
            .set("name", "n40 record changed")
            .set("drift_type", "checksum_change")
            .set("resolved", false),
    )
    .unwrap();
    g.create_edge(
        "DEPENDS_ON",
        "DriftEvent",
        "drift:n40",
        "Artifact",
        "art:calc-n40",
        Props::new(),
    )
    .unwrap();
    let (truth, why) = g.premise_truth("ver:calc-n40").unwrap();
    assert_eq!(truth, Truth::Unknown, "{why}");
    assert!(why.contains("drifted"), "{why}");
}

#[test]
fn a_check_that_is_not_computed_reads_as_it_did_in_0_85_0() {
    let mut g = graph();
    g.add_verification("ver:unit", "a unit test", Some("test"), None, None)
        .unwrap();
    g.set_verification_status("ver:unit", "passing", Some("2026-10-17"), None)
        .unwrap();
    g.upsert_node(
        "Verification",
        "ver:unit",
        Props::new().set("location", "tests/a.rs"),
    )
    .unwrap();
    assert_eq!(
        g.premise_truth("ver:unit").unwrap(),
        (Truth::Holds, "the check is passing".to_string())
    );
}

#[test]
fn a_check_whose_location_names_a_kept_record_cites_it() {
    let mut g = graph();
    axiom(&mut g, "con:def-prime");
    statement(&mut g, "req:counterexample-n40");
    // No DOCUMENTS edge: the record is named in the location only.
    computed_check(
        &mut g,
        "ver:calc-n40",
        "n40.calc.json",
        Some("art:calc-n40"),
    );
    step(
        &mut g,
        "inf:counterexample",
        &["ver:calc-n40", "con:def-prime"],
        "req:counterexample-n40",
    );
    let r = serde_json::to_value(g.reasoning_report(None, None).unwrap()).unwrap();
    let check = r["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "ver:calc-n40")
        .unwrap();
    assert_eq!(check["sources"][0]["id"], "art:calc-n40", "{check:#}");
    let uncited: Vec<&str> = r["uncited_premises"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(!uncited.contains(&"ver:calc-n40"), "{uncited:?}");
}

// ─── 2. Unsupported is not refuted ─────────────────────────────────────────

#[test]
fn losing_a_premise_one_line_uses_leaves_the_theorem_on_the_other_line() {
    let mut g = graph();
    two_proofs(&mut g);
    // E3 is in Euclid's line only.
    let rows = walk(&g, &["req:e3-listed-dont-divide"]);
    let euclid = row(&rows, "inf:theorem-by-euclid");
    assert_eq!(euclid["impact"], "unsupported", "{euclid:#}");
    assert_eq!(euclid["conclusion_after"], "holds", "{euclid:#}");
    assert_eq!(euclid["conclusion_reads"], "still_supported", "{euclid:#}");
    assert_eq!(
        euclid["still_supported_by"],
        serde_json::json!(["inf:theorem-by-goldbach"])
    );
    for r in &rows {
        assert_ne!(
            r["impact"], "broken",
            "a loss alone never breaks a step: {r:#}"
        );
        assert_ne!(
            r["conclusion_after"], "fails",
            "unsupported is not refuted: {r:#}"
        );
    }
}

#[test]
fn losing_the_shared_lemma_leaves_the_theorem_unsupported_never_false() {
    let mut g = graph();
    two_proofs(&mut g);
    let rows = walk(&g, &["req:l5-prime-divisor"]);
    for s in ["inf:theorem-by-euclid", "inf:theorem-by-goldbach"] {
        let r = row(&rows, s);
        assert_eq!(r["conclusion_after"], "unknown", "{r:#}");
        assert_eq!(r["conclusion_reads"], "unsupported", "{r:#}");
        assert_eq!(r["impact"], "unsupported", "{r:#}");
    }
}

#[test]
fn a_step_whose_premise_really_fails_leaves_its_claim_unsupported_not_refuted() {
    let mut g = graph();
    axiom(&mut g, "con:def-prime");
    statement(&mut g, "req:counterexample-n40");
    g.add_verification("ver:n40", "n40", Some("test"), None, None)
        .unwrap();
    g.set_verification_status("ver:n40", "failing", Some("2026-10-17"), None)
        .unwrap();
    step(
        &mut g,
        "inf:c",
        &["ver:n40", "con:def-prime"],
        "req:counterexample-n40",
    );
    assert_eq!(
        g.inference_reading("inf:c").unwrap().unwrap().verdict,
        Truth::Fails
    );
    let (truth, why) = g.premise_truth("req:counterexample-n40").unwrap();
    assert_eq!(truth, Truth::Unknown, "{why}");
    assert!(why.starts_with("unsupported"), "{why}");
}

#[test]
fn a_refuted_claim_reads_refuted_in_the_what_if() {
    let mut g = graph();
    refuted_conjecture(&mut g);
    // A step resting on the refuted conjecture.
    statement(&mut g, "req:euler-primes-forever");
    step(
        &mut g,
        "inf:uses-conjecture",
        &["req:conjecture-n2-n-41"],
        "req:euler-primes-forever",
    );
    assert_eq!(
        g.premise_truth("req:conjecture-n2-n-41").unwrap().0,
        Truth::Fails
    );
    // The conjecture is refuted; what rests on it is unsupported, not refuted.
    assert_eq!(
        g.premise_truth("req:euler-primes-forever").unwrap().0,
        Truth::Unknown
    );
}

// ─── 3. A refutation reads as refuted, not as a defect ─────────────────────

#[test]
fn a_counterexample_that_holds_is_not_a_contradiction_defect() {
    let mut g = graph();
    refuted_conjecture(&mut g);
    assert_eq!(
        g.premise_truth("req:counterexample-n40").unwrap().0,
        Truth::Holds
    );
    assert_eq!(
        g.premise_truth("req:conjecture-n2-n-41").unwrap().0,
        Truth::Fails
    );
    let sweep = g.detect_defects().unwrap();
    let contradictions: Vec<_> = sweep
        .defects
        .iter()
        .filter(|d| d.category.as_str() == "contradiction")
        .collect();
    assert!(contradictions.is_empty(), "{contradictions:#?}");
    let swept = serde_json::to_value(&sweep.swept).unwrap();
    assert_eq!(
        swept["suppressed_by_refutation"]["contradiction"], 1,
        "counted as a refutation, never silenced: {swept:#}"
    );
    let status = g.loop_status().unwrap();
    assert_eq!(status.structural_defects, g.open_defects().unwrap().len());
    assert!(
        g.open_defects()
            .unwrap()
            .iter()
            .all(|d| d.category.as_str() != "contradiction")
    );
}

#[test]
fn a_contradiction_between_two_unknown_claims_is_still_reported() {
    let mut g = graph();
    axiom(&mut g, "con:def-prime");
    statement(&mut g, "req:a");
    statement(&mut g, "req:b");
    statement(&mut g, "req:c");
    // req:a is concluded (so it is in the reasoning) but unknown: its premise
    // is a requirement nobody has shown.
    step(&mut g, "inf:a", &["req:c"], "req:a");
    edge(&mut g, "CONTRADICTS", "req:a", "req:b", Props::new());
    assert_eq!(g.premise_truth("req:a").unwrap().0, Truth::Unknown);
    let sweep = g.detect_defects().unwrap();
    assert!(
        sweep
            .defects
            .iter()
            .any(|d| d.category.as_str() == "contradiction"),
        "{:#?}",
        sweep.defects
    );
}

// ─── 4. The loop sees unconfirmed steps and uncited premises ──────────────

#[test]
fn loop_status_and_detect_gaps_count_what_the_reasoning_owes() {
    let mut g = graph();
    two_proofs(&mut g);
    let status = serde_json::to_value(g.loop_status().unwrap()).unwrap();
    let r = &status["reasoning"];
    assert_eq!(r["steps"], 11, "{r:#}");
    assert_eq!(r["unconfirmed_steps"], 11, "{r:#}");
    assert_eq!(r["uncited_premises"], 4, "the four axioms: {r:#}");
    assert!(r["shown_not_proven"].as_u64().unwrap() >= 1, "{r:#}");
    assert!(
        status["next"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l.as_str().unwrap().contains("unconfirmed step")),
        "{status:#}"
    );
    assert!(gap_sources(&g).contains(&"unconfirmed_reasoning".to_string()));

    // An axiom marked TAKEN AS GIVEN is a stated foundation, not uncited.
    for a in ["con:def-prime", "con:def-divides"] {
        g.upsert_node("Constraint", a, Props::new().set("taken_as_given", true))
            .expect("Constraint.taken_as_given is a declared property");
    }
    let status = serde_json::to_value(g.loop_status().unwrap()).unwrap();
    assert_eq!(status["reasoning"]["uncited_premises"], 2, "{status:#}");
    assert_eq!(status["reasoning"]["stated_foundations"], 2, "{status:#}");
    let (truth, why) = g.premise_truth("con:def-prime").unwrap();
    assert_eq!(truth, Truth::Holds);
    assert!(why.contains("taken as given"), "{why}");
    let report = serde_json::to_value(g.reasoning_report(None, None).unwrap()).unwrap();
    assert!(
        report["stated_foundations"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("con:def-prime"))
    );
}

#[test]
fn a_design_with_no_inference_owes_no_reasoning() {
    let mut g = graph();
    statement(&mut g, "req:plain");
    let status = serde_json::to_value(g.loop_status().unwrap()).unwrap();
    assert!(status.get("reasoning").is_none(), "{status:#}");
    assert!(!gap_sources(&g).contains(&"unconfirmed_reasoning".to_string()));
}

// ─── 5. The gaps fit a proof ───────────────────────────────────────────────

#[test]
fn software_shaped_gaps_are_not_asked_of_a_proof() {
    let mut g = graph();
    two_proofs(&mut g);
    refuted_conjecture(&mut g);
    let sources = gap_sources(&g);
    for s in [
        "concept_without_design",
        "quality_target_unstated",
        "unlinked_intent",
    ] {
        assert!(
            !sources.contains(&s.to_string()),
            "{s} fired on a proof: {sources:?}"
        );
    }
    // One statement outside the reasoning makes it an ordinary design again.
    statement(&mut g, "req:a-thing-to-build");
    let sources = gap_sources(&g);
    assert!(
        sources.contains(&"concept_without_design".to_string()),
        "{sources:?}"
    );
    assert!(
        sources.contains(&"quality_target_unstated".to_string()),
        "{sources:?}"
    );
}

#[test]
fn a_lemma_both_lines_rest_on_is_a_single_point_of_failure() {
    let mut g = graph();
    two_proofs(&mut g);
    let spofs: Vec<Vec<String>> = g
        .open_defects()
        .unwrap()
        .into_iter()
        .filter(|d| d.category.as_str() == "single_point_of_failure")
        .map(|d| d.affected_ids)
        .collect();
    let named: Vec<&str> = spofs.iter().map(|a| a[0].as_str()).collect();
    assert!(named.contains(&"req:l5-prime-divisor"), "{spofs:?}");
    assert!(named.contains(&"req:l2-divides-difference"), "{spofs:?}");
    // A lemma under Lemma 5 falls with it, and an axiom is the foundation.
    for not in [
        "req:l1-divides-transitive",
        "req:l4-least-divisor",
        "con:def-prime",
    ] {
        assert!(!named.contains(&not), "{not} named: {spofs:?}");
    }
    assert!(spofs.iter().all(|a| a[1] == "req:theorem"), "{spofs:?}");
}

#[test]
fn a_what_if_stays_inside_the_reasoning_it_affects() {
    let mut g = graph();
    two_proofs(&mut g);
    refuted_conjecture(&mut g);
    let radius = g
        .propagate_from(
            &["req:e3-listed-dont-divide"],
            reflow2_core::PropagateOptions::default(),
        )
        .unwrap();
    let reached: Vec<&str> = radius.impacted.iter().map(|n| n.node_id.as_str()).collect();
    // What rests on it, and the grounds it rests on, are listed…
    for yes in [
        "inf:euclid",
        "req:euclid",
        "req:theorem",
        "inf:e3",
        "con:def-prime",
    ] {
        assert!(reached.contains(&yes), "{yes} missing: {reached:?}");
    }
    // …and what merely shares a definition with it is not.
    for no in [
        "req:conjecture-n2-n-41",
        "req:counterexample-n40",
        "req:l5-prime-divisor",
        "inf:l1",
    ] {
        assert!(!reached.contains(&no), "{no} reached: {reached:?}");
    }
    let summary = radius.summarize();
    assert!(
        summary
            .risk_crossings
            .iter()
            .all(|n| n.node_id != "req:conjecture-n2-n-41"),
        "{:?}",
        summary.risk_crossings
    );
}
