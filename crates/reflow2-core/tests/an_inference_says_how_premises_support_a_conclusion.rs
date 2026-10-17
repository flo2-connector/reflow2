//! A design states how its premises support a conclusion, and reflow2 checks
//! that the conclusion's status follows
//! (req:a-design-states-how-premises-support-a-conclusion-and-reflow2-checks-it-follows,
//! shaped by dec:an-inference-is-a-record-between-premises-and-conclusion).
//!
//! An `Inference` sits between its premises (`PREMISE_OF`, premise → Inference)
//! and its one conclusion (`CONCLUDES`, Inference → conclusion), and holds the
//! rule: `all`, `any`, or `k_of_n` with its `k`. The rule is evaluated over
//! three values — holds, fails, unknown (Kleene) — and the check lives in three
//! reads that already exist:
//!
//! - THE ROLL-UP: a requirement concluded by an Inference is delivered when the
//!   rule holds over its premises; every other requirement rolls up as before.
//! - THE CHANGE WALK: from a changed or failing premise, each Inference
//!   downstream reads UNSUPPORTED (the rule no longer holds; since 2026-10-17,
//!   req:unsupported-is-not-refuted, a loss is never read as false), BROKEN
//!   (a premise really fails) or WEAKENED (it still holds on the other
//!   premises), and the walk carries on past a conclusion left unsupported
//!   only.
//! - A FINDING: a conclusion whose claimed status is past what its Inference
//!   supports (`conclusion_does_not_follow`).
//!
//! Written against the generic write path (`create_node` / `create_edge`) and
//! read through the existing reports, serialised, so every test here COMPILES
//! on a build that predates the vocabulary and is observed FAILING there: the
//! node type and the two edge types do not exist, so the first write is refused.
//! The typed constructor `add_inference` is driven over the MCP surface in the
//! reflow2-mcp crate's file of the same name.

use reflow2_core::DesignGraph;
use reflow2_core::nodes::Props;
use serde_json::Value as Json;

const INFERENCE: &str = "Inference";
const PREMISE_OF: &str = "PREMISE_OF";
const CONCLUDES: &str = "CONCLUDES";

fn graph() -> DesignGraph {
    DesignGraph::open_in_memory().expect("in-memory graph")
}

/// The type of an existing node, read the way a caller would.
fn type_of(g: &DesignGraph, id: &str) -> String {
    let held = g.node_types_holding(id).expect("types holding");
    assert_eq!(
        held.len(),
        1,
        "{id} must be held by exactly one type: {held:?}"
    );
    held[0].clone()
}

/// An Inference through the generic write path: the node, one PREMISE_OF per
/// premise, one CONCLUDES. `rule: None` writes no `rule` at all.
fn inference(
    g: &mut DesignGraph,
    id: &str,
    rule: Option<&str>,
    k: Option<i64>,
    premises: &[&str],
    conclusion: &str,
) {
    let mut props = Props::new().set("name", format!("step {id}"));
    if let Some(r) = rule {
        props = props.set("rule", r);
    }
    if let Some(k) = k {
        props = props.set("k", k);
    }
    g.create_node(INFERENCE, id, props)
        .unwrap_or_else(|e| panic!("an Inference node is writable: {e}"));
    for p in premises {
        let ty = type_of(g, p);
        g.create_edge(PREMISE_OF, &ty, p, INFERENCE, id, Props::new())
            .unwrap_or_else(|e| panic!("{p} PREMISE_OF {id} is writable: {e}"));
    }
    let ty = type_of(g, conclusion);
    g.create_edge(CONCLUDES, INFERENCE, id, &ty, conclusion, Props::new())
        .unwrap_or_else(|e| panic!("{id} CONCLUDES {conclusion} is writable: {e}"));
}

/// A check with a recorded result.
fn check(g: &mut DesignGraph, id: &str, status: &str) {
    g.add_verification(id, &format!("check {id}"), None, None, None)
        .unwrap();
    if status != "planned" {
        g.set_verification_status(id, status, Some("2026-10-14"), None)
            .unwrap();
    }
}

fn requirement(g: &mut DesignGraph, id: &str) {
    g.add_requirement(id, &format!("claim {id}"), "A claim the premises support.")
        .unwrap();
}

/// The change walk's rows, as a client reads them off the blast radius.
fn walk(g: &DesignGraph, seeds: &[&str]) -> Vec<Json> {
    let radius = g
        .propagate_from(seeds, reflow2_core::PropagateOptions::default())
        .expect("propagate_from");
    let v = serde_json::to_value(&radius).unwrap();
    v.get("inferences")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default()
}

/// The walk's row for one Inference.
fn row<'a>(rows: &'a [Json], inference: &str) -> &'a Json {
    rows.iter()
        .find(|r| r["inference"] == inference)
        .unwrap_or_else(|| panic!("no walk row for {inference} in {rows:#?}"))
}

/// The open gaps of one source, by the source's serialised name.
fn gaps_of(g: &DesignGraph, source: &str) -> Vec<Json> {
    g.detect_gaps()
        .unwrap()
        .iter()
        .map(|gap| serde_json::to_value(gap).unwrap())
        .filter(|gap| gap["gap_source"] == source)
        .collect()
}

// ─── AND by default ─────────────────────────────────────────────────────────

#[test]
fn all_is_the_default_and_one_failing_premise_leaves_the_conclusion_undelivered_and_broken() {
    let mut g = graph();
    requirement(&mut g, "req:claim");
    check(&mut g, "ver:a", "passing");
    check(&mut g, "ver:b", "passing");
    // No `rule` written: the step reads `all`, which says nothing new.
    inference(
        &mut g,
        "inf:both",
        None,
        None,
        &["ver:a", "ver:b"],
        "req:claim",
    );

    assert!(
        g.requirement_is_delivered("req:claim").unwrap(),
        "every premise holds, so the conclusion is delivered"
    );

    g.set_verification_status("ver:b", "failing", Some("2026-10-14"), None)
        .unwrap();
    assert!(
        !g.requirement_is_delivered("req:claim").unwrap(),
        "`all` with one failing premise does not deliver its conclusion"
    );
    let rows = walk(&g, &["ver:b"]);
    let r = row(&rows, "inf:both");
    assert_eq!(r["impact"], "broken", "{r:#}");
    assert_eq!(r["rule"], "all", "an unstated rule reads `all`: {r:#}");
    assert_eq!(r["conclusion"], "req:claim", "{r:#}");
}

// ─── The requirement's SHOWN WHEN, literally ───────────────────────────────

#[test]
fn redundant_support_marked_any_is_weakened_by_one_loss_and_broken_by_the_last() {
    let mut g = graph();
    requirement(&mut g, "req:altitude-known");
    check(&mut g, "ver:gps", "passing");
    check(&mut g, "ver:baro", "passing");
    inference(
        &mut g,
        "inf:redundant-altitude",
        Some("any"),
        None,
        &["ver:gps", "ver:baro"],
        "req:altitude-known",
    );
    assert!(g.requirement_is_delivered("req:altitude-known").unwrap());

    // Remove one support: WEAKENED, and still standing.
    let rows = walk(&g, &["ver:gps"]);
    let r = row(&rows, "inf:redundant-altitude");
    assert_eq!(r["impact"], "weakened", "{r:#}");
    assert_eq!(
        r["after"], "holds",
        "it still stands on the barometer: {r:#}"
    );
    g.delete_edge(PREMISE_OF, "ver:gps", "inf:redundant-altitude")
        .unwrap();
    assert!(
        g.requirement_is_delivered("req:altitude-known").unwrap(),
        "one support left still delivers an `any` claim"
    );

    // Remove the last: UNSUPPORTED, never false (req:unsupported-is-not-refuted).
    let rows = walk(&g, &["ver:baro"]);
    let r = row(&rows, "inf:redundant-altitude");
    assert_eq!(r["impact"], "unsupported", "{r:#}");
    assert_eq!(r["after"], "unknown", "{r:#}");
    assert_eq!(r["conclusion_reads"], "unsupported", "{r:#}");
    g.set_verification_status("ver:baro", "failing", Some("2026-10-14"), None)
        .unwrap();
    assert!(!g.requirement_is_delivered("req:altitude-known").unwrap());
}

// ─── k of n, at its boundary ───────────────────────────────────────────────

#[test]
fn k_of_n_holds_at_k_and_not_at_k_minus_one() {
    let mut g = graph();
    requirement(&mut g, "req:quorum");
    check(&mut g, "ver:1", "passing");
    check(&mut g, "ver:2", "passing");
    check(&mut g, "ver:3", "failing");
    inference(
        &mut g,
        "inf:two-of-three",
        Some("k_of_n"),
        Some(2),
        &["ver:1", "ver:2", "ver:3"],
        "req:quorum",
    );
    // k holding: delivered.
    assert!(
        g.requirement_is_delivered("req:quorum").unwrap(),
        "2 of 3 holding meets k = 2"
    );
    // Lose one of the two: k - 1 holding, and the lost one reads
    // unsupported (unknown), never false — so the step no longer holds, and is
    // not thereby broken (req:unsupported-is-not-refuted).
    let rows = walk(&g, &["ver:2"]);
    let r = row(&rows, "inf:two-of-three");
    assert_eq!(r["before"], "holds", "{r:#}");
    assert_eq!(r["after"], "unknown", "{r:#}");
    assert_eq!(r["impact"], "unsupported", "{r:#}");

    g.set_verification_status("ver:2", "failing", Some("2026-10-14"), None)
        .unwrap();
    assert!(
        !g.requirement_is_delivered("req:quorum").unwrap(),
        "1 of 3 holding is k - 1"
    );
}

// ─── Unknown propagates (Kleene) ───────────────────────────────────────────

#[test]
fn an_unknown_premise_makes_the_rule_unknown_unless_another_premise_decides_it() {
    let mut g = graph();
    requirement(&mut g, "req:all-unknown");
    requirement(&mut g, "req:any-holds");
    requirement(&mut g, "req:all-fails");
    check(&mut g, "ver:passing", "passing");
    check(&mut g, "ver:failing", "failing");
    // `planned`: the check has not run, so nothing is known either way.
    check(&mut g, "ver:not-run", "planned");
    // A type with no predicate of its own is unknown, never guessed.
    g.add_component(
        "cmp:observed",
        "A part nobody has checked",
        "Stands in for a premise with no predicate.",
        None,
    )
    .unwrap();
    inference(
        &mut g,
        "inf:all-unknown",
        Some("all"),
        None,
        &["ver:passing", "ver:not-run", "cmp:observed"],
        "req:all-unknown",
    );
    inference(
        &mut g,
        "inf:any-holds",
        Some("any"),
        None,
        &["ver:passing", "ver:not-run"],
        "req:any-holds",
    );
    inference(
        &mut g,
        "inf:all-fails",
        Some("all"),
        None,
        &["ver:failing", "ver:not-run"],
        "req:all-fails",
    );

    // all over {holds, unknown} is unknown, and unknown does not deliver.
    assert!(!g.requirement_is_delivered("req:all-unknown").unwrap());
    // any over {holds, unknown} holds.
    assert!(g.requirement_is_delivered("req:any-holds").unwrap());
    // all over {fails, unknown} fails: one failing premise decides it.
    assert!(!g.requirement_is_delivered("req:all-fails").unwrap());

    let rows = walk(&g, &["ver:passing", "ver:failing"]);
    assert_eq!(row(&rows, "inf:all-unknown")["before"], "unknown");
    assert_eq!(row(&rows, "inf:any-holds")["before"], "holds");
    assert_eq!(row(&rows, "inf:all-fails")["before"], "fails");
    // Losing the one holding premise of an `any` whose other premise is
    // unknown leaves it unsupported — neither weakened nor broken.
    assert_eq!(row(&rows, "inf:any-holds")["after"], "unknown");
    assert_eq!(row(&rows, "inf:any-holds")["impact"], "unsupported");
    // A step that did not hold before the loss reads unknown.
    assert_eq!(row(&rows, "inf:all-unknown")["impact"], "unknown");
    // The premise with no predicate says why it is unknown.
    let premises = row(&rows, "inf:all-unknown")["premises"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let fact = premises
        .iter()
        .find(|p| p["id"] == "cmp:observed")
        .expect("the premise is listed");
    assert_eq!(fact["truth"], "unknown", "{fact:#}");
}

// ─── Modus ponens ──────────────────────────────────────────────────────────

#[test]
fn modus_ponens_is_all_over_p_and_p_implies_q_and_q_breaks_when_p_fails() {
    let mut g = graph();
    // P: it rained, measured.
    check(&mut g, "ver:it-rained", "passing");
    // P → Q: an accepted ruling that rain wets the pitch.
    g.add_decision(
        "dec:rain-wets-the-pitch",
        "If it rains, the pitch is wet",
        "Rain on the pitch makes it wet.",
        None,
    )
    .unwrap();
    g.create_node(
        "Decision",
        "dec:rain-wets-the-pitch",
        Props::new()
            .set("name", "If it rains, the pitch is wet")
            .set("decision", "Rain on the pitch makes it wet.")
            .set("status", "accepted"),
    )
    .unwrap();
    // Q.
    requirement(&mut g, "req:the-pitch-is-wet");
    inference(
        &mut g,
        "inf:modus-ponens",
        Some("all"),
        None,
        &["ver:it-rained", "dec:rain-wets-the-pitch"],
        "req:the-pitch-is-wet",
    );
    let stored = g.get_node(INFERENCE, "inf:modus-ponens").unwrap().unwrap();
    let mut props = Props::new();
    for (k, v) in &stored.properties {
        props = props.set(k, v.clone());
    }
    g.create_node(
        INFERENCE,
        "inf:modus-ponens",
        props.set("rule_name", "modus ponens"),
    )
    .unwrap();

    assert!(
        g.requirement_is_delivered("req:the-pitch-is-wet").unwrap(),
        "P and P → Q hold, so Q follows"
    );

    g.set_verification_status("ver:it-rained", "failing", Some("2026-10-14"), None)
        .unwrap();
    assert!(!g.requirement_is_delivered("req:the-pitch-is-wet").unwrap());
    let rows = walk(&g, &["ver:it-rained"]);
    let r = row(&rows, "inf:modus-ponens");
    assert_eq!(r["impact"], "broken", "{r:#}");
    assert_eq!(r["rule_name"], "modus ponens", "{r:#}");
}

// ─── The finding ───────────────────────────────────────────────────────────

#[test]
fn a_claimed_met_that_does_not_follow_is_reported_and_one_that_follows_is_not() {
    let mut g = graph();
    requirement(&mut g, "req:follows");
    requirement(&mut g, "req:does-not-follow");
    check(&mut g, "ver:ok", "passing");
    check(&mut g, "ver:bad", "failing");
    inference(
        &mut g,
        "inf:ok",
        Some("all"),
        None,
        &["ver:ok"],
        "req:follows",
    );
    inference(
        &mut g,
        "inf:bad",
        Some("all"),
        None,
        &["ver:ok", "ver:bad"],
        "req:does-not-follow",
    );
    g.set_requirement_status("req:follows", "met").unwrap();
    g.set_requirement_status("req:does-not-follow", "met")
        .unwrap();

    let found = gaps_of(&g, "conclusion_does_not_follow");
    let about: Vec<String> = found
        .iter()
        .flat_map(|gap| gap["affected_ids"].as_array().cloned().unwrap_or_default())
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    assert!(
        about.iter().any(|a| a == "req:does-not-follow"),
        "a `met` whose `all` has a failing premise is reported: {found:#?}"
    );
    assert!(
        !about.iter().any(|a| a == "req:follows"),
        "a `met` that follows raises nothing: {found:#?}"
    );
    // The finding is a report, never a repair: the status is left as claimed.
    let stored = g
        .get_node("Requirement", "req:does-not-follow")
        .unwrap()
        .unwrap();
    assert_eq!(
        stored.properties.get("status").and_then(|v| v.as_str()),
        Some("met")
    );
    // And the older status check does not ALSO ask "nothing satisfies it" of a
    // conclusion whose support is stated by its Inference.
    let contradictions = gaps_of(&g, "status_contradiction");
    assert!(
        contradictions
            .iter()
            .all(|gap| !gap["affected_ids"].to_string().contains("req:follows")),
        "{contradictions:#?}"
    );
}

// ─── Circular reasoning ────────────────────────────────────────────────────

#[test]
fn a_cycle_through_premise_of_and_concludes_is_refused_at_write() {
    let mut g = graph();
    requirement(&mut g, "req:a");
    requirement(&mut g, "req:b");
    check(&mut g, "ver:seed", "passing");
    // a supports b.
    inference(&mut g, "inf:a-to-b", Some("all"), None, &["req:a"], "req:b");
    // b supporting a would close the circle.
    g.create_node(
        INFERENCE,
        "inf:b-to-a",
        Props::new().set("name", "b to a").set("rule", "all"),
    )
    .unwrap();
    g.create_edge(
        PREMISE_OF,
        "Requirement",
        "req:b",
        INFERENCE,
        "inf:b-to-a",
        Props::new(),
    )
    .unwrap();
    let err = g
        .create_edge(
            CONCLUDES,
            INFERENCE,
            "inf:b-to-a",
            "Requirement",
            "req:a",
            Props::new(),
        )
        .expect_err("a conclusion that supports its own premise is circular reasoning");
    assert!(
        err.to_string().contains("circular"),
        "the refusal says what is wrong: {err}"
    );
    // Nothing was written.
    assert!(
        g.outgoing("inf:b-to-a", Some(CONCLUDES))
            .unwrap()
            .is_empty(),
        "the refused edge is not stored"
    );
    // A step cannot be its own premise either.
    let err = g
        .create_edge(
            PREMISE_OF,
            INFERENCE,
            "inf:a-to-b",
            INFERENCE,
            "inf:a-to-b",
            Props::new(),
        )
        .expect_err("an Inference resting on itself is circular");
    assert!(err.to_string().contains("circular"), "{err}");
    // And one Inference concludes exactly one node.
    let err = g
        .create_edge(
            CONCLUDES,
            INFERENCE,
            "inf:a-to-b",
            "Verification",
            "ver:seed",
            Props::new(),
        )
        .expect_err("a second conclusion is refused");
    assert!(err.to_string().contains("concludes"), "{err}");
}

// ─── A chain, and alternative derivations ──────────────────────────────────

#[test]
fn a_chain_breaks_through_a_broken_conclusion_and_stops_at_a_weakened_one() {
    let mut g = graph();
    check(&mut g, "ver:root", "passing");
    check(&mut g, "ver:spare", "passing");
    requirement(&mut g, "req:middle");
    requirement(&mut g, "req:top");
    requirement(&mut g, "req:redundant");
    requirement(&mut g, "req:above-redundant");
    // root → middle → top: a chain of `all` steps.
    inference(
        &mut g,
        "inf:root-middle",
        Some("all"),
        None,
        &["ver:root"],
        "req:middle",
    );
    inference(
        &mut g,
        "inf:middle-top",
        Some("all"),
        None,
        &["req:middle"],
        "req:top",
    );
    // root or spare → redundant → above-redundant.
    inference(
        &mut g,
        "inf:either",
        Some("any"),
        None,
        &["ver:root", "ver:spare"],
        "req:redundant",
    );
    inference(
        &mut g,
        "inf:above",
        Some("all"),
        None,
        &["req:redundant"],
        "req:above-redundant",
    );
    assert!(g.requirement_is_delivered("req:top").unwrap());
    assert!(g.requirement_is_delivered("req:above-redundant").unwrap());

    let rows = walk(&g, &["ver:root"]);
    // The walk continues past a conclusion left unsupported…
    assert_eq!(row(&rows, "inf:root-middle")["impact"], "unsupported");
    assert_eq!(row(&rows, "inf:middle-top")["impact"], "unsupported");
    assert_eq!(
        row(&rows, "inf:middle-top")["conclusion_reads"],
        "unsupported"
    );
    // …and reports a weakened one without making its dependents lose support.
    assert_eq!(row(&rows, "inf:either")["impact"], "weakened");
    assert!(
        rows.iter().all(|r| r["inference"] != "inf:above"),
        "a weakened conclusion's dependents are not walked as broken: {rows:#?}"
    );

    g.set_verification_status("ver:root", "failing", Some("2026-10-14"), None)
        .unwrap();
    assert!(!g.requirement_is_delivered("req:top").unwrap());
    assert!(g.requirement_is_delivered("req:above-redundant").unwrap());
}

#[test]
fn a_node_concluded_twice_stands_while_either_derivation_holds() {
    let mut g = graph();
    requirement(&mut g, "req:twice");
    check(&mut g, "ver:first", "passing");
    check(&mut g, "ver:second", "failing");
    inference(
        &mut g,
        "inf:first",
        Some("all"),
        None,
        &["ver:first"],
        "req:twice",
    );
    inference(
        &mut g,
        "inf:second",
        Some("all"),
        None,
        &["ver:second"],
        "req:twice",
    );
    assert!(
        g.requirement_is_delivered("req:twice").unwrap(),
        "two Inferences concluding one node are alternative derivations: ANY of them"
    );
    let rows = walk(&g, &["ver:first"]);
    let r = row(&rows, "inf:first");
    assert_eq!(r["impact"], "unsupported", "{r:#}");
    assert_eq!(
        r["conclusion_after"], "unknown",
        "with the first lost and the second failing, nothing derives it — and it is \
         unsupported, never refuted: {r:#}"
    );
    assert_eq!(r["conclusion_reads"], "unsupported", "{r:#}");
    g.set_verification_status("ver:second", "passing", Some("2026-10-14"), None)
        .unwrap();
    let rows = walk(&g, &["ver:first"]);
    let r = row(&rows, "inf:first");
    assert_eq!(r["impact"], "unsupported", "{r:#}");
    assert_eq!(
        r["conclusion_after"], "holds",
        "the other derivation still holds: {r:#}"
    );
    assert_eq!(r["conclusion_reads"], "still_supported", "{r:#}");
    assert_eq!(
        r["still_supported_by"],
        serde_json::json!(["inf:second"]),
        "{r:#}"
    );
}

// ─── Nothing changes for a design that uses none of this ───────────────────

/// A design with no Inference: requirements decomposed, satisfied, checked and
/// failing, a budget, a decision. Its reports are compared byte for byte with
/// what main produced (`tests/fixtures/reports_without_an_inference.json`,
/// recorded on origin/main b19b49d with REFLOW2_BLESS_NO_INFERENCE=1).
fn a_design_with_no_inference() -> DesignGraph {
    let mut g = graph();
    g.add_project("proj:p", "A project").unwrap();
    for (id, name) in [
        ("req:parent", "Checkout"),
        ("req:card", "Enter a card"),
        ("req:receipt", "Get a receipt"),
        ("req:orphan", "Nothing meets this"),
    ] {
        g.add_requirement(id, name, "A stated need.").unwrap();
        g.contains("proj:p", "Requirement", id).unwrap();
    }
    g.decomposes("req:card", "req:parent").unwrap();
    g.decomposes("req:receipt", "req:parent").unwrap();
    g.add_capability("cap:card", "Takes a card", "Card entry.", Some("realized"))
        .unwrap();
    g.add_capability(
        "cap:receipt",
        "Prints a receipt",
        "Receipts.",
        Some("realized"),
    )
    .unwrap();
    g.satisfies("cap:card", "req:card").unwrap();
    g.satisfies("cap:receipt", "req:receipt").unwrap();
    check(&mut g, "ver:card", "passing");
    check(&mut g, "ver:receipt", "failing");
    g.verifies("ver:card", "Capability", "cap:card").unwrap();
    g.verifies("ver:receipt", "Capability", "cap:receipt")
        .unwrap();
    g.add_decision(
        "dec:card-first",
        "Cards first",
        "Cards before receipts.",
        None,
    )
    .unwrap();
    g.governed_by(
        "Capability",
        "cap:card",
        "Decision",
        "dec:card-first",
        None,
        None,
    )
    .unwrap();
    g.set_requirement_status("req:orphan", "met").unwrap();
    g
}

#[test]
fn a_design_with_no_inference_reports_exactly_as_it_did() {
    let g = a_design_with_no_inference();
    let delivered: std::collections::BTreeMap<String, Option<Vec<String>>> =
        ["req:parent", "req:card", "req:receipt", "req:orphan"]
            .iter()
            .map(|r| (r.to_string(), g.requirement_delivered_by(r).unwrap()))
            .collect();
    let reports = serde_json::json!({
        "delivery": g.delivery_coverage().unwrap(),
        "delivered": delivered,
        "gaps": g.detect_gaps().unwrap(),
        "defects": g.detect_defects().unwrap(),
        "radius": g
            .propagate_from(&["ver:receipt", "req:card"], reflow2_core::PropagateOptions::default())
            .unwrap(),
        "graph_report": g.graph_report().unwrap(),
    });
    let text = serde_json::to_string_pretty(&reports).unwrap() + "\n";
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/reports_without_an_inference.json");
    if std::env::var_os("REFLOW2_BLESS_NO_INFERENCE").is_some() {
        std::fs::write(&path, &text).unwrap();
        return;
    }
    let golden = std::fs::read_to_string(&path).expect("the fixture recorded on main");
    assert!(
        text == golden,
        "a design with no Inference must report byte for byte as main did; \
         diff {} against this build's output",
        path.display()
    );
}
