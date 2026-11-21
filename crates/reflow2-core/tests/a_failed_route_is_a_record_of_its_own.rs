//! A FAILED ROUTE IS A RECORD OF ITS OWN, READ BACK WITH THE REASONING —
//! `req:a-failed-route-is-a-record-of-its-own-read-back-with-the-reasoning`
//! (2026-10-30, option (a) of
//! `dec:idea-the-preserved-search-is-a-deliverable-of-working-a-proof-hole`).
//!
//! The encoding (`reflow2_core::failed_route`): a Requirement with
//! `statement_kind: failed_route`, whose `statement` says what was tried,
//! `route_to` the claim it was a route to, `fails_at` / `fails_at_step` the
//! step at which it fails, `fails_because` why, and `route_rests_on` what it
//! rests on. These tests pin:
//!
//! 1. it reads `failed_route`, never as an open or unproven claim, and its
//!    truth never holds;
//! 2. it is read whole under the claim it was a route to, in reasoning_report,
//!    and tallied in the report's `failed_routes`;
//! 3. NOTHING ELSE MOVES: on the committed Riemann map, every gap, every
//!    defect, the reasoning's debt and every claim of the read-back read the
//!    same with a failed route recorded as without it;
//! 4. it supports, concludes and objects to nothing: each such link is
//!    refused, where it is written;
//! 5. a what-if never reaches it — the heat rung's k < 0 finding, drawn with
//!    DEPENDS_ON to its failing steps, was reached by every what-if on them.

use reflow2_core::DesignGraph;
use reflow2_core::export::GraphExport;
use reflow2_core::foundation::core::Value;
use reflow2_core::inference::{InferenceSpec, Truth};
use reflow2_core::nodes::Props;
use reflow2_core::propagate::PropagateOptions;
use serde_json::Value as Json;
use std::collections::BTreeMap;

const RIEMANN: &str = include_str!("fixtures/proof_map_riemann.json");
const HEAT: &str = include_str!("fixtures/proof_trial_heat_equation.json");

const RH: &str = "req:riemann-hypothesis";
const ROBIN_1984: &str = "req:cite-robin-1984-equivalence";
const ROBIN_ALL_N: &str = "req:robin-inequality-all-n";
const ROUTE: &str = "req:route-robin-on-colossally-abundant-numbers";

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

fn list(ids: &[&str]) -> Value {
    Value::List(ids.iter().map(|s| Value::String(s.to_string())).collect())
}

fn statement(g: &mut DesignGraph, id: &str, kind: &str) {
    g.add_requirement(id, &format!("statement {id}"), "A statement.")
        .unwrap();
    g.upsert_node("Requirement", id, Props::new().set("statement_kind", kind))
        .unwrap();
}

fn step(g: &mut DesignGraph, id: &str, premises: &[&str], conclusion: &str) {
    g.add_inference(&InferenceSpec {
        id: id.to_string(),
        rule: "all".to_string(),
        premises: premises.iter().map(|s| s.to_string()).collect(),
        conclusion: conclusion.to_string(),
        ..Default::default()
    })
    .unwrap_or_else(|e| panic!("{id}: {e}"));
}

/// A failed route written the way the skill teaches it.
#[allow(clippy::too_many_arguments)]
fn route(
    g: &mut DesignGraph,
    id: &str,
    tried: &str,
    to: &str,
    fails_at: Option<&str>,
    fails_at_step: Option<&str>,
    because: Option<&str>,
    rests_on: &[&str],
) {
    g.add_requirement(id, &format!("Failed route: {id}"), tried)
        .unwrap();
    let mut p = Props::new()
        .set("statement_kind", "failed_route")
        .set("route_to", to);
    if let Some(at) = fails_at {
        p = p.set("fails_at", at);
    }
    if let Some(s) = fails_at_step {
        p = p.set("fails_at_step", s);
    }
    if let Some(b) = because {
        p = p.set("fails_because", b);
    }
    if !rests_on.is_empty() {
        p = p.set("route_rests_on", list(rests_on));
    }
    g.upsert_node("Requirement", id, p)
        .unwrap_or_else(|e| panic!("{id}: {e}"));
}

/// The route tried toward the Riemann Hypothesis through Robin's inequality
/// on the colossally abundant numbers: the speculation the owner discarded on
/// 2026-10-22 as circular, recorded here as what it was — a route that fails.
fn robin_route(g: &mut DesignGraph) {
    route(
        g,
        ROUTE,
        "Prove Robin's inequality for every n > 5040 by proving it on the colossally abundant \
         numbers, where σ(n)/n is largest, and so prove the Riemann Hypothesis.",
        RH,
        Some("the last step, from Robin's inequality for every n > 5040 to the hypothesis"),
        Some(ROBIN_ALL_N),
        Some(
            "Robin's inequality for every n > 5040 is equivalent to the hypothesis (Robin 1984), \
             so the step assumes what it sets out to prove: circular",
        ),
        &[ROBIN_1984, ROBIN_ALL_N],
    );
}

fn report(g: &DesignGraph, claim: Option<&str>) -> Json {
    json(g.reasoning_report_from(claim, Some(500), None).unwrap())
}

fn claim_in<'a>(r: &'a Json, id: &str) -> &'a Json {
    r["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("{id} is not read: {r:#}"))
}

/// The read-back with every `failed_routes` removed: what the reasoning
/// reads, apart from the routes.
fn without_routes(mut v: Json) -> Json {
    fn strip(v: &mut Json) {
        match v {
            Json::Object(o) => {
                o.remove("failed_routes");
                for x in o.values_mut() {
                    strip(x);
                }
            }
            Json::Array(a) => a.iter_mut().for_each(strip),
            _ => {}
        }
    }
    strip(&mut v);
    v
}

fn gap_kinds(g: &DesignGraph) -> BTreeMap<String, Vec<Vec<String>>> {
    let mut out: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();
    for gap in g.detect_gaps().expect("gaps") {
        let mut ids = gap.affected_ids.clone();
        ids.sort();
        out.entry(gap.gap_source.as_str().to_string())
            .or_default()
            .push(ids);
    }
    for v in out.values_mut() {
        v.sort();
    }
    out
}

fn reads(g: &DesignGraph, id: &str) -> String {
    json(g.statement_reading(id).unwrap())["reads"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

// ─── 1. It reads as a failed route, never as a hole ───────────────────────

#[test]
fn a_failed_route_reads_failed_route_and_never_holds() {
    let mut g = load(RIEMANN);
    robin_route(&mut g);
    let s = json(g.statement_reading(ROUTE).unwrap().unwrap());
    assert_eq!(s["kind"], "failed_route", "{s:#}");
    assert_eq!(s["reads"], "failed_route", "{s:#}");
    let why = s["why"].as_str().unwrap();
    assert!(why.contains(&format!("toward {RH}")), "{why}");
    assert!(why.contains("circular"), "the reason is said: {why}");
    assert!(
        why.contains("supports nothing and refutes nothing"),
        "{why}"
    );
    let (truth, _) = g.premise_truth(ROUTE).unwrap();
    assert_eq!(truth, Truth::Unknown, "a failed route never holds");
    // The hole it was a route to reads exactly as it did.
    assert_eq!(reads(&g, RH), "open");
}

#[test]
fn a_failed_route_says_which_of_its_parts_it_does_not_record() {
    let mut g = load(RIEMANN);
    route(
        &mut g,
        "req:route-bare",
        "Tried something and it did not work.",
        RH,
        None,
        None,
        None,
        &[],
    );
    let r = json(g.failed_route_reading("req:route-bare").unwrap().unwrap());
    assert_eq!(
        r["missing"],
        serde_json::json!(["fails_at", "fails_because", "route_rests_on"]),
        "{r:#}"
    );
    assert_eq!(r["names_step_and_reason"], false);
    robin_route(&mut g);
    let r = json(g.failed_route_reading(ROUTE).unwrap().unwrap());
    assert!(r.get("missing").is_none(), "{r:#}");
    assert_eq!(r["names_step_and_reason"], true);
    assert!(r["tried"].as_str().unwrap().contains("colossally"));
    assert_eq!(r["route_to"]["id"], RH);
    assert_eq!(r["route_to"]["name"], "The Riemann Hypothesis (OPEN)");
    assert_eq!(r["fails_at_step"]["id"], ROBIN_ALL_N);
    let rests: Vec<&str> = r["rests_on"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["id"].as_str().unwrap())
        .collect();
    assert_eq!(rests, vec![ROBIN_1984, ROBIN_ALL_N]);
    // The tally counts the complete one only.
    let rep = report(&g, Some(RH));
    let tally = &rep["failed_routes"];
    assert_eq!(tally["total"], 2, "{tally:#}");
    assert_eq!(tally["with_step_and_reason"], 1, "{tally:#}");
    assert_eq!(
        tally["without_step_or_reason"],
        serde_json::json!(["req:route-bare"])
    );
}

/// `req:a-failed-route-records-the-natural-condition-under-which-it-would-hold`:
/// a route may say the natural condition under which it WOULD go through
/// (`would_hold_if`). It is read back with the route and moves no reading:
/// the route still fails for its claim, and the condition is neither a
/// hypothesis of that claim nor a result shown.
#[test]
fn a_route_says_the_condition_under_which_it_would_go_through_and_nothing_moves() {
    const CONDITION: &str = "restricted to the colossally abundant numbers below 10^13, where it is checked by computation";
    let mut plain = load(RIEMANN);
    robin_route(&mut plain);
    let mut g = load(RIEMANN);
    robin_route(&mut g);
    g.upsert_node(
        "Requirement",
        ROUTE,
        Props::new().set("would_hold_if", CONDITION),
    )
    .unwrap();

    // Read back with the route: on its own, and under its claim.
    let r = json(g.failed_route_reading(ROUTE).unwrap().unwrap());
    assert_eq!(r["would_hold_if"], CONDITION, "{r:#}");
    assert!(
        r.get("missing").is_none(),
        "an optional field is never missing: {r:#}"
    );
    let rep = report(&g, Some(RH));
    let under = claim_in(&rep, RH)["failed_routes"]
        .as_array()
        .expect("routes under the claim")
        .iter()
        .find(|x| x["id"] == ROUTE)
        .cloned()
        .expect("the route is read under its claim");
    assert_eq!(under["would_hold_if"], CONDITION, "{under:#}");
    let s = json(g.statement_reading(ROUTE).unwrap().unwrap());
    assert_eq!(s["reads"], "failed_route");
    assert!(
        s["why"]
            .as_str()
            .unwrap()
            .contains(&format!("would go through {CONDITION}")),
        "{s:#}"
    );

    // It moves nothing: the route still fails for its claim, and no other
    // reading, gap or debt changes.
    assert_eq!(g.premise_truth(ROUTE).unwrap().0, Truth::Unknown);
    for claim in [None, Some(RH), Some(ROBIN_ALL_N)] {
        assert_eq!(
            without_routes(report(&g, claim)),
            without_routes(report(&plain, claim)),
            "the read-back under {claim:?} moved"
        );
    }
    assert_eq!(
        gap_kinds(&g),
        gap_kinds(&plain),
        "the condition raised a gap"
    );
    assert_eq!(
        json(g.reasoning_debt().unwrap()),
        json(plain.reasoning_debt().unwrap()),
        "the condition added to what the reasoning owes"
    );
    for id in [RH, ROBIN_ALL_N] {
        assert_eq!(reads(&g, id), reads(&plain, id), "{id}");
    }
}

// ─── 2. It is read back under the claim it was a route to ─────────────────

#[test]
fn a_failed_route_is_read_whole_under_the_claim_it_was_a_route_to() {
    let mut g = load(RIEMANN);
    robin_route(&mut g);
    for whole in [false, true] {
        let r = report(&g, (!whole).then_some(RH));
        let rh = claim_in(&r, RH);
        let routes = rh["failed_routes"].as_array().expect("routes under RH");
        assert_eq!(routes.len(), 1, "{rh:#}");
        assert_eq!(routes[0]["id"], ROUTE);
        assert_eq!(routes[0]["reads"], "failed_route");
        assert!(
            routes[0]["fails_because"]
                .as_str()
                .unwrap()
                .contains("equivalent to the hypothesis")
        );
        let tally = &r["failed_routes"];
        assert_eq!(tally["total"], 1, "{tally:#}");
        assert_eq!(tally["toward"][0]["claim"], RH, "{tally:#}");
        assert_eq!(tally["toward"][0]["routes"], serde_json::json!([ROUTE]));
        assert!(tally.get("read_here").is_none(), "{tally:#}");
        // NEVER A CLAIM OF ITS OWN, AND NEVER UNSUPPORTED: it is not a hole.
        let claims = r["claims"].as_array().unwrap();
        assert!(!claims.iter().any(|c| c["id"] == ROUTE), "whole={whole}");
        let unsupported = r["unsupported"].as_array().unwrap();
        assert!(!unsupported.iter().any(|c| c["id"] == ROUTE), "{r:#}");
        // What it rests on is not what the reasoning rests on.
        let rests: Vec<&str> = r["rests_on"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect();
        assert!(!rests.contains(&ROUTE));
    }
}

#[test]
fn a_route_to_a_step_is_read_under_the_claim_the_step_concludes() {
    let mut g = graph();
    statement(&mut g, "req:def", "definition");
    statement(&mut g, "req:goal", "theorem");
    step(&mut g, "inf:goal", &["req:def"], "req:goal");
    route(
        &mut g,
        "req:route-via-step",
        "Close the step by compactness.",
        "inf:goal",
        Some("the compactness argument"),
        None,
        Some("the space is not compact"),
        &["req:def"],
    );
    let r = report(&g, Some("req:goal"));
    let goal = claim_in(&r, "req:goal");
    assert_eq!(
        goal["failed_routes"][0]["id"], "req:route-via-step",
        "{r:#}"
    );
    assert_eq!(goal["failed_routes"][0]["route_to"]["id"], "inf:goal");
    assert_eq!(goal["failed_routes"][0]["read_under"], "req:goal");
}

#[test]
fn a_route_read_on_its_own_comes_back_whole_and_not_as_unsupported() {
    let mut g = load(RIEMANN);
    robin_route(&mut g);
    let r = report(&g, Some(ROUTE));
    let unsupported = r["unsupported"].as_array().unwrap();
    assert!(unsupported.is_empty(), "{r:#}");
    let here = r["failed_routes"]["read_here"].as_array().unwrap();
    assert_eq!(here.len(), 1, "{r:#}");
    assert_eq!(here[0]["id"], ROUTE);
    assert_eq!(claim_in(&r, ROUTE)["statement"]["reads"], "failed_route");
}

#[test]
fn a_design_with_no_failed_route_reads_exactly_as_before() {
    let g = load(RIEMANN);
    for claim in [None, Some(RH)] {
        let r = report(&g, claim);
        assert!(r.get("failed_routes").is_none(), "{r:#}");
        for c in r["claims"].as_array().unwrap() {
            assert!(c.get("failed_routes").is_none(), "{c:#}");
        }
    }
}

// ─── 3. Nothing else moves ────────────────────────────────────────────────

#[test]
fn on_the_riemann_map_a_failed_route_moves_no_reading_gap_defect_or_debt() {
    let plain = load(RIEMANN);
    let mut g = load(RIEMANN);
    robin_route(&mut g);
    route(
        &mut g,
        "req:route-incomplete",
        "A route recorded with nothing but its words.",
        ROBIN_ALL_N,
        None,
        None,
        None,
        &[],
    );
    for claim in [None, Some(RH), Some(ROBIN_ALL_N), Some(ROBIN_1984)] {
        assert_eq!(
            without_routes(report(&g, claim)),
            without_routes(report(&plain, claim)),
            "the read-back under {claim:?} moved"
        );
    }
    assert_eq!(
        gap_kinds(&g),
        gap_kinds(&plain),
        "a failed route raised a gap"
    );
    // The defects, and what any of them was suppressed by, are the same. The
    // `swept` block counts the populations each rule examined, so it counts
    // the two routes as two more nodes: that is the report being honest about
    // what it walked, and it is checked to be exactly that.
    let (with, without) = (
        json(g.detect_defects().unwrap()),
        json(plain.detect_defects().unwrap()),
    );
    assert_eq!(
        with["defects"], without["defects"],
        "a failed route raised a defect"
    );
    for k in [
        "suppressed_by_held_open",
        "suppressed_by_parked_idea",
        "suppressed_by_resolution",
    ] {
        assert_eq!(with["swept"][k], without["swept"][k], "{k}");
    }
    assert_eq!(
        with["swept"]["nodes"].as_u64().unwrap(),
        without["swept"]["nodes"].as_u64().unwrap() + 2,
        "the sweep counts the two routes and nothing else"
    );
    assert_eq!(
        json(g.reasoning_debt().unwrap()),
        json(plain.reasoning_debt().unwrap()),
        "a failed route added to what the reasoning owes"
    );
    for id in [RH, ROBIN_ALL_N, ROBIN_1984] {
        assert_eq!(reads(&g, id), reads(&plain, id), "{id}");
        assert_eq!(
            g.premise_truth(id).unwrap().0,
            plain.premise_truth(id).unwrap().0,
            "{id}"
        );
    }
}

// ─── 4. It supports, concludes and objects to nothing ─────────────────────

#[test]
fn a_failed_route_is_refused_as_a_premise_a_conclusion_or_an_objection() {
    let mut g = graph();
    statement(&mut g, "req:def", "definition");
    statement(&mut g, "req:goal", "conjecture");
    route(
        &mut g,
        "req:route",
        "An approach that fails.",
        "req:goal",
        Some("step 2"),
        None,
        Some("the bound is too weak"),
        &["req:def"],
    );
    for (premises, conclusion, field) in [
        (vec!["req:route"], "req:goal", "premises"),
        (vec!["req:def"], "req:route", "conclusion"),
    ] {
        let e = g
            .add_inference(&InferenceSpec {
                id: "inf:bad".to_string(),
                rule: "all".to_string(),
                premises: premises.iter().map(|s| s.to_string()).collect(),
                conclusion: conclusion.to_string(),
                ..Default::default()
            })
            .expect_err("refused")
            .to_string();
        assert!(e.contains("is a failed route"), "{field}: {e}");
        assert!(
            g.get_node("Inference", "inf:bad").unwrap().is_none(),
            "{field}: nothing is written"
        );
    }
    let e = g
        .add_inference(&InferenceSpec {
            id: "inf:assumes".to_string(),
            rule: "all".to_string(),
            premises: vec!["req:def".to_string()],
            assumed: vec!["req:route".to_string()],
            conclusion: "req:goal".to_string(),
            ..Default::default()
        })
        .expect_err("refused as an assumption")
        .to_string();
    assert!(e.contains("is a failed route"), "{e}");
    // EVERY DOOR: the links themselves are refused where they are written.
    step(&mut g, "inf:ok", &["req:def"], "req:goal");
    let e = g
        .create_edge(
            "PREMISE_OF",
            "Requirement",
            "req:route",
            "Inference",
            "inf:ok",
            Props::new(),
        )
        .expect_err("a premise link is refused")
        .to_string();
    assert!(e.contains("is a failed route"), "{e}");
    let e = g
        .create_edge(
            "CONTRADICTS",
            "Requirement",
            "req:route",
            "Requirement",
            "req:goal",
            Props::new(),
        )
        .expect_err("an objection from it is refused")
        .to_string();
    assert!(e.contains("never an objection"), "{e}");
    // Corroboration is only listed, never counted: allowed.
    g.create_edge(
        "CONTRADICTS",
        "Requirement",
        "req:route",
        "Requirement",
        "req:goal",
        Props::new().set("alignment", "supporting"),
    )
    .unwrap();
    assert_eq!(reads(&g, "req:goal"), "shown");
}

#[test]
fn an_objection_to_the_record_is_read_on_the_record_and_moves_nothing_else() {
    let mut g = graph();
    statement(&mut g, "req:def", "definition");
    statement(&mut g, "req:goal", "conjecture");
    route(
        &mut g,
        "req:route",
        "An approach that fails.",
        "req:goal",
        Some("step 2"),
        None,
        Some("the bound is too weak"),
        &["req:def"],
    );
    let before = reads(&g, "req:goal");
    // "The route does not fail there after all": an objection to the record.
    statement(&mut g, "req:the-bound-suffices", "objection");
    g.create_edge(
        "CONTRADICTS",
        "Requirement",
        "req:the-bound-suffices",
        "Requirement",
        "req:route",
        Props::new(),
    )
    .unwrap();
    let s = json(g.statement_reading("req:route").unwrap().unwrap());
    assert_eq!(s["reads"], "failed_route", "{s:#}");
    assert!(
        s["why"]
            .as_str()
            .unwrap()
            .contains("An objection to this record stands"),
        "{s:#}"
    );
    assert_eq!(reads(&g, "req:goal"), before, "the claim does not move");
}

#[test]
fn a_record_that_takes_part_in_a_step_is_told_why_it_cannot_become_a_failed_route() {
    let mut g = graph();
    statement(&mut g, "req:def", "definition");
    statement(&mut g, "req:lemma", "lemma");
    statement(&mut g, "req:goal", "theorem");
    step(&mut g, "inf:goal", &["req:lemma"], "req:goal");
    let why = g
        .failed_route_conflict("req:lemma")
        .unwrap()
        .expect("a premise cannot become one");
    assert!(why.contains("a premise of inf:goal"), "{why}");
    assert!(g.failed_route_conflict("req:def").unwrap().is_none());
}

// ─── 5. A what-if never reaches it ────────────────────────────────────────

/// THE HEAT RUNG'S k < 0 CASE (fixture exported from flo2.io on
/// 2026-10-23): the finding that neither proof reaches k < 0 was drawn
/// DEPENDS_ON its two failing steps, for want of a record, and every what-if
/// on either step reached it. Recorded as two failed routes, the what-if on
/// those steps reaches neither, and each route names its step.
#[test]
fn the_heat_rungs_failed_cases_recorded_as_routes_are_reached_by_no_what_if() {
    const ENERGY_STEP: &str = "inf:E-prime-nonpositive";
    const MP_STEP: &str = "inf:mp-strict";
    const FINDING: &str = "req:finding-k-negative-proofs-fail";
    const CLAIM: &str = "req:prop-uniqueness-k-negative";
    let impacted = |g: &DesignGraph, from: &str| -> Vec<String> {
        json(
            g.propagate_from(&[from], PropagateOptions::default())
                .unwrap(),
        )["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["node_id"].as_str().unwrap().to_string())
            .collect()
    };
    let mut g = load(HEAT);
    // MEASURED BEFORE: the finding sits in the blast radius of each step.
    assert!(impacted(&g, ENERGY_STEP).contains(&FINDING.to_string()));
    assert!(impacted(&g, MP_STEP).contains(&FINDING.to_string()));
    route(
        &mut g,
        "req:route-energy-for-k-negative",
        "Prove uniqueness for k < 0 by the energy argument.",
        CLAIM,
        Some("the step that E(t) is nonincreasing"),
        Some(ENERGY_STEP),
        Some(
            "E'(t) = −2k ∫ w_x² dx ≥ 0 when k < 0, so E is nondecreasing and E(0) = 0 bounds nothing",
        ),
        &["con:def-energy-E"],
    );
    route(
        &mut g,
        "req:route-max-principle-for-k-negative",
        "Prove uniqueness for k < 0 by the maximum principle.",
        CLAIM,
        Some("the strict maximum principle step"),
        Some(MP_STEP),
        Some("for k < 0 the sign the strict inequality needs is reversed"),
        &[],
    );
    for (step, r) in [
        (ENERGY_STEP, "req:route-energy-for-k-negative"),
        (MP_STEP, "req:route-max-principle-for-k-negative"),
    ] {
        let hit = impacted(&g, step);
        assert!(!hit.contains(&r.to_string()), "{step} reached {r}");
        let read = json(g.failed_route_reading(r).unwrap().unwrap());
        assert_eq!(read["fails_at_step"]["id"], step);
    }
    assert!(
        !impacted(&g, "con:def-energy-E").contains(&"req:route-energy-for-k-negative".to_string())
    );
    let r = report(&g, Some(CLAIM));
    let routes = claim_in(&r, CLAIM)["failed_routes"].as_array().unwrap();
    assert_eq!(routes.len(), 2, "{r:#}");
    assert_eq!(r["failed_routes"]["with_step_and_reason"], 2);
}

// ─── 6. The skill says how ────────────────────────────────────────────────

const PROOF_SKILL: &str = include_str!("../../../getting-started/skills/design-a-proof/SKILL.md");

/// The served design-a-proof skill teaches the encoding these tests pin, and
/// how a turn in the search names its cause
/// (req:a-turning-point-in-a-proof-search-names-the-source-that-caused-it).
#[test]
fn the_proof_skill_says_how_to_record_a_failed_route_and_a_turn() {
    for word in [
        "`failed_route`",
        "`route_to`",
        "`fails_at`",
        "`fails_at_step`",
        "`fails_because`",
        "`route_rests_on`",
        "`failed_routes`",
        "`with_step_and_reason`",
        "Never draw DEPENDS_ON from it to the step that fails",
        "**A turn in the search**",
        "`CAUSES`",
    ] {
        assert!(PROOF_SKILL.contains(word), "design-a-proof lacks {word}");
    }
}
