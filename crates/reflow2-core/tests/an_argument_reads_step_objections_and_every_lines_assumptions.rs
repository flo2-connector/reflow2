//! AN ARGUMENT READS THE OBJECTIONS AIMED AT ITS STEPS, AND EVERY LINE'S
//! ASSUMPTIONS — the first PR of phase 6 (`epoch:planned-p6-argument-gaps`),
//! on Anthony's word of 2026-10-20 ("Start phase 6").
//!
//! - **`req:an-objection-aimed-at-a-step-is-read-against-that-step`** (the
//!   cogito trial's item 1, the trolley trial's item 6). An opposing
//!   CONTRADICTS into an Inference attacks the step's warrant or rule, not a
//!   premise or the conclusion. Until this change it was stored and never
//!   read: the step read `holds`, and reasoning_report did not list it. It now
//!   counts as far as it holds: one that holds DEFEATS the step (it fails and
//!   supports nothing; its conclusion is left unsupported, never refuted), one
//!   that stands CONTESTS it, one that is answered no longer counts and the
//!   step is reinstated. The trolley design holds two such objections
//!   (closeness against the double-effect footbridge step; Greene/Singer's
//!   debunking against the footbridge objection's own step).
//! - **`req:a-claim-with-several-lines-reads-each-lines-assumptions`** (the
//!   trolley trial's item 4). A claim held by several lines holds under ANY
//!   ONE of them and names every line (`holds_under_any_of`), never only the
//!   first: "Driver: steering permissible" is held by five principles and read
//!   "conditional on act consequentialism" alone. A step citing such a claim
//!   inherits only what its own cheapest line needs, never another line's
//!   assumption. And an objection holding only under one line's assumption
//!   contests a claim that also holds outside it, never refutes it.
//!
//! Every test is a CLASS over every fixture where a fixture can carry it — the
//! two argument trials (trolley, 4c35490abf0cefdf; cogito, 989598dd02246a81,
//! exported from flo2.io on 2026-10-20 at 0.89.0) and the five proof trials —
//! plus the hand-built shape it names. Read as JSON through the generic write
//! path, so the file compiles on main 50af571; each test FAILED there first.

use std::collections::BTreeSet;

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
            .set("evidence", "an objection, as the trial recorded it"),
    );
}

/// A statement by its id's prefix, as the trials record them: a `con:` case
/// or principle is a Constraint, anything else a Requirement.
fn statement(g: &mut DesignGraph, id: &str, given: bool) {
    let ty = if id.starts_with("con:") {
        g.add_constraint(
            id,
            &format!("statement {id}"),
            "A case or principle.",
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        "Constraint"
    } else {
        g.add_requirement(id, &format!("statement {id}"), "A position.")
            .unwrap();
        "Requirement"
    };
    let mut props = Props::new().set("statement_kind", "proposition");
    if given {
        props = props.set("taken_as_given", Value::Bool(true));
    }
    g.upsert_node(ty, id, props).unwrap();
}

/// A position, case or principle taken as given: it holds.
fn given(g: &mut DesignGraph, id: &str) {
    statement(g, id, true);
}

/// A claim nothing establishes yet (a `req:` one is unknown on its own), or a
/// principle a step takes up as an assumption.
fn claim_node(g: &mut DesignGraph, id: &str) {
    statement(g, id, false);
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

fn report(g: &DesignGraph, id: &str) -> Json {
    json(g.reasoning_report(Some(id), Some(500)).unwrap())
}

fn claim(g: &DesignGraph, id: &str) -> Json {
    report(g, id)["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .cloned()
        .unwrap_or_else(|| panic!("{id} not in its own read-back"))
}

/// Every claim of the whole design's read-back, by id: one walk, so a class
/// over every claim reads each once.
fn claims_of(g: &DesignGraph) -> std::collections::BTreeMap<String, Json> {
    let r = json(g.reasoning_report(None, Some(500)).unwrap());
    assert_eq!(r["capped"], false, "the fixture fits one walk");
    r["claims"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["id"].as_str().unwrap().to_string(), c.clone()))
        .collect()
}

fn truth(g: &DesignGraph, id: &str) -> String {
    g.premise_truth(id).unwrap().0.as_str().to_string()
}

/// Every opposing CONTRADICTS whose target is an Inference: (objector, step).
fn step_objections(g: &DesignGraph) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for st in g.scan_nodes("Inference").unwrap() {
        for e in g.incoming(&st.node_id, Some("CONTRADICTS")).unwrap() {
            if e.properties.get("alignment").and_then(Value::as_str) != Some("supporting") {
                out.push((e.from_id, st.node_id.clone()));
            }
        }
    }
    out
}

// ─── 1. An objection aimed at a step ─────────────────────────────────────────

/// THE CLASS, over every objection aimed at a step in every fixture: the step's
/// reading lists it with how it stands; the step reads `defeated` (and fails)
/// when one refutes it, `contested` (and does not hold) when one stands, and
/// `answered` when every one is answered; the read-back of the claim the step
/// concludes walks the objection and lists the step under `steps_objected`;
/// and the claim itself is never refuted by an objection to one of its steps.
/// On main the trolley design's two step objections were stored and unread:
/// the steps read `holds` with no `objections`.
#[test]
fn every_objection_aimed_at_a_step_is_read_against_that_step() {
    let mut seen = 0;
    for (name, doc) in ALL {
        let g = load(doc);
        for (objector, st) in step_objections(&g) {
            seen += 1;
            let r = reading(&g, &st);
            let listed: Vec<(String, String)> = r["objections"]
                .as_array()
                .unwrap_or_else(|| {
                    panic!(
                        "{name}: {objector} objects to the step {st}, which lists nothing: {r:#}"
                    )
                })
                .iter()
                .map(|o| {
                    (
                        o["id"].as_str().unwrap().to_string(),
                        o["standing"].as_str().unwrap().to_string(),
                    )
                })
                .collect();
            let standing = listed
                .iter()
                .find(|(id, _)| *id == objector)
                .map(|(_, s)| s.clone())
                .unwrap_or_else(|| panic!("{name}: {st} does not list {objector}: {r:#}"));
            assert!(
                ["refutes", "stands", "answered"].contains(&standing.as_str()),
                "{name}: {st}: {standing}"
            );
            let all: Vec<&str> = listed.iter().map(|(_, s)| s.as_str()).collect();
            let (verdict, objected) = (r["verdict"].as_str().unwrap(), &r["objected"]);
            if all.contains(&"refutes") {
                assert_eq!(verdict, "fails", "{name}: a defeated step fails: {r:#}");
                assert_eq!(objected, "defeated", "{name}: {r:#}");
            } else if all.contains(&"stands") {
                assert_ne!(
                    verdict, "holds",
                    "{name}: a contested step does not hold: {r:#}"
                );
                assert_eq!(objected, "contested", "{name}: {r:#}");
            } else {
                assert_eq!(objected, "answered", "{name}: {r:#}");
            }
            let Some(conclusion) = r["conclusion"].as_str() else {
                continue;
            };
            // The read-back of the claim the step concludes walks the
            // objection and lists the step with it.
            let rep = report(&g, conclusion);
            let walked: Vec<&str> = rep["claims"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|c| c["id"].as_str())
                .collect();
            assert!(
                walked.contains(&objector.as_str()),
                "{name}: the read-back of {conclusion} does not walk {objector}, which objects to \
                 its step {st}: {walked:?}"
            );
            let c = claim(&g, conclusion);
            let support = c["support"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["inference"] == st.as_str())
                .unwrap_or_else(|| panic!("{name}: {conclusion} lists no support {st}: {c:#}"));
            assert_eq!(support["objected"], *objected, "{name}: {support:#}");
            if objected == "defeated" || objected == "contested" {
                let listed: Vec<&str> = rep["steps_objected"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|s| s["step"].as_str()).collect())
                    .unwrap_or_default();
                assert!(listed.contains(&st.as_str()), "{name}: {st}: {rep:#}");
            }
            // An objection to a STEP never refutes its conclusion.
            let own_refutes = c["objections"]
                .as_array()
                .is_some_and(|a| a.iter().any(|o| o["standing"] == "refutes"));
            if !own_refutes {
                assert_ne!(
                    truth(&g, conclusion),
                    "fails",
                    "{name}: {conclusion}: {c:#}"
                );
            }
        }
    }
    assert!(
        seen >= 2,
        "the class is not vacuous: {seen} step objection(s)"
    );
}

/// THE MEASURED SHAPE, the trolley design's: Greene and Singer's debunking
/// argument is aimed at the step concluding the footbridge objection against
/// act consequentialism, and closeness at the double-effect footbridge step.
/// Neither is established or answered, so both steps read contested, and the
/// read-back of the footbridge position names them.
#[test]
fn the_trolley_designs_step_objections_contest_their_steps() {
    let g = load(TROLLEY);
    for (st, objector) in [
        (
            "inf:footbridge-by-double-effect",
            "req:obj-closeness-dde-footbridge",
        ),
        (
            "inf:obj-footbridge-against-act-consequentialism",
            "req:obj-debunking-footbridge-judgment",
        ),
    ] {
        let r = reading(&g, st);
        assert_eq!(r["objected"], "contested", "{st}: {r:#}");
        assert_ne!(r["verdict"], "holds", "{st}: {r:#}");
        assert!(
            strs(&json(
                r["objections"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|o| o["id"].clone())
                    .collect::<Vec<_>>()
            ))
            .contains(&objector.to_string()),
            "{st}: {r:#}"
        );
        assert!(
            r["note"].as_str().unwrap_or_default().contains(objector),
            "{st}: the note says what contests it: {r:#}"
        );
    }
}

/// THE HAND-BUILT SHAPE, the cogito's syllogism reading: an objection to the
/// step that holds DEFEATS it (the step fails; "I exist" is left unsupported,
/// not refuted); a reply that holds answers it and REINSTATES the step; an
/// objection nothing establishes or answers CONTESTS it.
#[test]
fn an_objection_to_a_step_defeats_contests_and_a_reply_reinstates_it() {
    let mut g = graph();
    given(&mut g, "req:i-am-thinking");
    given(&mut g, "req:whatever-thinks-exists");
    claim_node(&mut g, "req:i-exist");
    step(
        &mut g,
        "inf:cogito-as-syllogism",
        &["req:i-am-thinking", "req:whatever-thinks-exists"],
        &[],
        "req:i-exist",
    );
    assert_eq!(reading(&g, "inf:cogito-as-syllogism")["verdict"], "holds");
    assert_eq!(truth(&g, "req:i-exist"), "holds");

    // Begging the question, taken as given: it holds, so it defeats the step.
    given(&mut g, "req:obj-begs-the-question");
    objects(
        &mut g,
        "req:obj-begs-the-question",
        "inf:cogito-as-syllogism",
    );
    let r = reading(&g, "inf:cogito-as-syllogism");
    assert_eq!(r["verdict"], "fails", "{r:#}");
    assert_eq!(r["objected"], "defeated", "{r:#}");
    assert_eq!(r["objections"][0]["standing"], "refutes", "{r:#}");
    let (t, why) = g.premise_truth("req:i-exist").unwrap();
    assert_eq!(t.as_str(), "unknown", "unsupported, never refuted: {why}");
    assert!(
        g.premise_truth("inf:cogito-as-syllogism")
            .unwrap()
            .1
            .contains("defeated"),
        "the step's own truth says it is defeated"
    );

    // A reply that holds answers the objection: the step is reinstated.
    given(&mut g, "req:reply-not-a-syllogism");
    objects(
        &mut g,
        "req:reply-not-a-syllogism",
        "req:obj-begs-the-question",
    );
    let r = reading(&g, "inf:cogito-as-syllogism");
    assert_eq!(r["verdict"], "holds", "{r:#}");
    assert_eq!(r["objected"], "answered", "{r:#}");
    assert_eq!(truth(&g, "req:i-exist"), "holds");

    // An objection nobody establishes or answers contests it.
    claim_node(&mut g, "req:obj-lichtenberg");
    objects(&mut g, "req:obj-lichtenberg", "inf:cogito-as-syllogism");
    let r = reading(&g, "inf:cogito-as-syllogism");
    assert_eq!(r["verdict"], "unknown", "{r:#}");
    assert_eq!(r["objected"], "contested", "{r:#}");
    assert_eq!(truth(&g, "req:i-exist"), "unknown");
    let rep = report(&g, "req:i-exist");
    assert_eq!(
        rep["steps_objected"][0]["step"], "inf:cogito-as-syllogism",
        "{rep:#}"
    );
    assert_eq!(
        strs(&rep["steps_objected"][0]["against"]),
        vec!["req:obj-lichtenberg".to_string()],
        "only what still stands is against it: {rep:#}"
    );
}

// ─── 2. Every line's assumptions ─────────────────────────────────────────────

/// A line as the readings serve it: (assumes, rests_on_unverified, rests_on_tests).
type Line = (Vec<String>, Vec<String>, Vec<String>);

fn line_of(v: &Json) -> Line {
    (
        strs(&v["assumes"]),
        strs(&v["rests_on_unverified"]),
        strs(&v["rests_on_tests"]),
    )
}

/// What a reading holds under, line by line: `holds_under_any_of` when it
/// names several, else the one line its own fields say.
fn lines_of(v: &Json) -> BTreeSet<Line> {
    match v["holds_under_any_of"].as_array() {
        Some(a) if !a.is_empty() => a.iter().map(line_of).collect(),
        _ => BTreeSet::from([line_of(v)]),
    }
}

fn within(a: &Line, b: &Line) -> bool {
    let sub = |x: &[String], y: &[String]| x.iter().all(|i| y.contains(i));
    sub(&a.0, &b.0) && sub(&a.1, &b.1) && sub(&a.2, &b.2)
}

fn union(a: &Line, b: &Line) -> Line {
    let u = |x: &[String], y: &[String]| -> Vec<String> {
        x.iter()
            .chain(y)
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    };
    (u(&a.0, &b.0), u(&a.1, &b.1), u(&a.2, &b.2))
}

/// The minimal lines of `lines`: none needs everything another needs.
fn minimal(lines: impl IntoIterator<Item = Line>) -> BTreeSet<Line> {
    let all: BTreeSet<Line> = lines.into_iter().collect();
    all.iter()
        .filter(|l| !all.iter().any(|m| m != *l && within(m, l)))
        .cloned()
        .collect()
}

/// THE CLASS, over every claim of every fixture that holds on its steps: it
/// names EVERY minimal line its holding steps give it — in
/// `holds_under_any_of` on its read-back and on its foundations when there
/// are several, its proof status naming each line's assumption when it is
/// conditional — and never only the first. On main the trolley design's
/// Driver position, held by five principles, read "only under
/// con:pr-act-consequentialism" and named no other line.
#[test]
fn a_claim_with_several_lines_names_every_lines_assumptions() {
    let mut several = 0;
    for (name, doc) in ALL {
        let g = load(doc);
        let mut concluded: BTreeSet<String> = BTreeSet::new();
        for st in g.scan_nodes("Inference").unwrap() {
            if let Some(c) = g.conclusion_of(&st.node_id).unwrap() {
                concluded.insert(c);
            }
        }
        let index = claims_of(&g);
        for c in concluded {
            let read = index.get(&c).cloned().unwrap_or_else(|| claim(&g, &c));
            if !["holds", "conditional", "supported"].contains(&read["truth"].as_str().unwrap()) {
                continue;
            }
            if read["why"]
                .as_str()
                .unwrap_or_default()
                .contains("delivered directly")
            {
                continue;
            }
            let expected = minimal(
                read["support"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|s| s["verdict"] == "holds")
                    .flat_map(lines_of),
            );
            if expected.is_empty() {
                continue;
            }
            assert_eq!(
                lines_of(&read),
                expected,
                "{name}: {c} names every line its holding steps give it: {read:#}"
            );
            if expected.len() > 1 {
                several += 1;
                let f = json(g.foundations_reading(&c).unwrap());
                assert_eq!(lines_of(&f), expected, "{name}: {c}'s foundations: {f:#}");
                let p = json(g.proof_reading(&c).unwrap());
                if p["status"] == "conditional" {
                    for (assumes, _, _) in &expected {
                        for a in assumes {
                            assert!(
                                p["why"].as_str().unwrap().contains(a.as_str()),
                                "{name}: {c}'s proof status names {a}: {p:#}"
                            );
                        }
                    }
                }
            }
        }
    }
    assert!(
        several >= 1,
        "the class is not vacuous: {several} claim(s) with several lines"
    );
}

/// THE MEASURED SHAPE: Driver, held by five principles, holds under any one
/// of the five, and its read-back, foundations and proof status name all five.
#[test]
fn driver_holds_under_any_one_of_its_five_principles() {
    let g = load(TROLLEY);
    let principles: BTreeSet<String> = [
        "con:pr-act-consequentialism",
        "con:pr-double-effect",
        "con:pr-negative-over-positive-duties",
        "con:pr-redirect-threat",
        "con:pr-what-you-would-not-pay-yourself",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let c = claim(&g, "req:v-driver-steer-permissible");
    let named: BTreeSet<String> = lines_of(&c).into_iter().flat_map(|l| l.0).collect();
    assert_eq!(named, principles, "{c:#}");
    assert_eq!(c["truth"], "conditional", "{c:#}");
    let p = json(g.proof_reading("req:v-driver-steer-permissible").unwrap());
    for pr in &principles {
        assert!(p["why"].as_str().unwrap().contains(pr.as_str()), "{p:#}");
    }
}

/// The lines a premise gives a step: an assumed one is itself; any other is
/// what it holds under, line by line.
fn premise_lines(
    g: &DesignGraph,
    index: &std::collections::BTreeMap<String, Json>,
    p: &Json,
) -> BTreeSet<Line> {
    let id = p["id"].as_str().unwrap();
    if p["role"] == "assumed" {
        return BTreeSet::from([(vec![id.to_string()], Vec::new(), Vec::new())]);
    }
    if type_of(g, id) == "Inference" {
        return lines_of(&reading(g, id));
    }
    match index.get(id) {
        Some(c) => lines_of(c),
        None => lines_of(&claim(g, id)),
    }
}

/// THE CLASS, over every holding `all` step of every fixture that discharges
/// nothing and splits on no cases: the step holds under exactly the minimal
/// ways of taking one line from each premise together. So its lightest line
/// is never heavier than its premises' cheapest combination, and a step
/// citing a claim held on several lines never inherits a line it does not
/// need. Plus the measured shape, built by hand.
#[test]
fn a_step_inherits_only_what_its_chosen_line_needs() {
    let mut asked = 0;
    for (name, doc) in ALL {
        let g = load(doc);
        let index = claims_of(&g);
        for st in g.scan_nodes("Inference").unwrap() {
            let has =
                |k: &str| matches!(st.properties.get(k), Some(Value::List(v)) if !v.is_empty());
            if has("discharges") || has("cases") {
                continue;
            }
            if st
                .properties
                .get("rule")
                .and_then(Value::as_str)
                .unwrap_or("all")
                != "all"
            {
                continue;
            }
            let r = reading(&g, &st.node_id);
            if r["verdict"] != "holds" {
                continue;
            }
            let mut acc: BTreeSet<Line> = BTreeSet::from([Default::default()]);
            for p in r["premises"].as_array().unwrap() {
                let lines = premise_lines(&g, &index, p);
                acc = minimal(
                    acc.iter()
                        .flat_map(|a| lines.iter().map(move |l| union(a, l))),
                );
            }
            asked += 1;
            assert_eq!(
                lines_of(&r),
                acc,
                "{name}: {} holds under the minimal combinations of its premises' lines: {r:#}",
                st.node_id
            );
        }
    }
    assert!(asked > 0, "the class is not vacuous");

    // The measured shape: a Thomson-2008 step cites the shared Switch
    // position, held by Foot's line and by Thomson 2008's. It needs only
    // Thomson 2008's line, and inherited Foot's on 0.87.0.
    let mut g = graph();
    for p in [
        "con:pr-negative-over-positive-duties",
        "con:pr-what-you-would-not-pay-yourself",
    ] {
        claim_node(&mut g, p);
    }
    given(&mut g, "con:case-switch");
    given(&mut g, "con:case-three-options");
    claim_node(&mut g, "req:v-switch-turn-impermissible");
    claim_node(&mut g, "req:v-three-options-turn-impermissible");
    step(
        &mut g,
        "inf:switch-by-negative-duties",
        &["con:case-switch"],
        &["con:pr-negative-over-positive-duties"],
        "req:v-switch-turn-impermissible",
    );
    step(
        &mut g,
        "inf:switch-by-what-you-would-not-pay",
        &["con:case-switch"],
        &["con:pr-what-you-would-not-pay-yourself"],
        "req:v-switch-turn-impermissible",
    );
    step(
        &mut g,
        "inf:three-options-from-switch",
        &["con:case-three-options", "req:v-switch-turn-impermissible"],
        &["con:pr-what-you-would-not-pay-yourself"],
        "req:v-three-options-turn-impermissible",
    );
    let r = reading(&g, "inf:three-options-from-switch");
    assert_eq!(
        strs(&r["assumes"]),
        vec!["con:pr-what-you-would-not-pay-yourself".to_string()],
        "it needs only its own line's principle, never Foot's: {r:#}"
    );
    let c = claim(&g, "req:v-switch-turn-impermissible");
    assert_eq!(
        lines_of(&c).len(),
        2,
        "the shared position holds on two lines: {c:#}"
    );
}

/// AN OBJECTION INSIDE ONE LINE CONTESTS A CLAIM ANOTHER LINE HOLDS OUTSIDE IT.
/// A claim held under A or under B is not inside A, so an objection holding
/// only under A establishes "if A, then not this" and refutes nothing. On main
/// the claim's first line ({A}) was taken as its whole scope, and the
/// objection refuted it.
#[test]
fn an_objection_inside_one_line_contests_a_claim_another_line_holds_outside_it() {
    let mut g = graph();
    claim_node(&mut g, "con:pr-a");
    claim_node(&mut g, "con:pr-b");
    given(&mut g, "con:case");
    claim_node(&mut g, "req:position");
    claim_node(&mut g, "req:objection-under-a");
    step(
        &mut g,
        "inf:by-a",
        &["con:case"],
        &["con:pr-a"],
        "req:position",
    );
    step(
        &mut g,
        "inf:by-b",
        &["con:case"],
        &["con:pr-b"],
        "req:position",
    );
    step(
        &mut g,
        "inf:objection-by-a",
        &["con:case"],
        &["con:pr-a"],
        "req:objection-under-a",
    );
    objects(&mut g, "req:objection-under-a", "req:position");
    let (t, why) = g.premise_truth("req:position").unwrap();
    assert_eq!(t.as_str(), "unknown", "contested, never refuted: {why}");
    let c = claim(&g, "req:position");
    assert_eq!(c["objections"][0]["standing"], "stands", "{c:#}");
    assert_eq!(
        strs(&c["objections"][0]["holds_only_under"]),
        vec!["con:pr-a".to_string()],
        "{c:#}"
    );
}

/// RIVAL POSITIONS KEPT AS AN OPEN DISPUTE, the form the argument skill
/// teaches: a CONTRADICTS each way between them. Both read contested, neither
/// refuted, whichever way the edges were drawn first. A one-way CONTRADICTS
/// refutes its target when the source holds outright — which is why the skill
/// never draws one between rival positions.
#[test]
fn rival_positions_with_mutual_objections_both_read_contested() {
    let mut g = graph();
    given(&mut g, "con:case-footbridge");
    given(&mut g, "con:pr-act-consequentialism");
    given(&mut g, "con:pr-double-effect");
    claim_node(&mut g, "req:v-footbridge-push-permissible");
    claim_node(&mut g, "req:v-footbridge-push-impermissible");
    step(
        &mut g,
        "inf:footbridge-by-act-consequentialism",
        &["con:case-footbridge", "con:pr-act-consequentialism"],
        &[],
        "req:v-footbridge-push-permissible",
    );
    step(
        &mut g,
        "inf:footbridge-by-double-effect",
        &["con:case-footbridge", "con:pr-double-effect"],
        &[],
        "req:v-footbridge-push-impermissible",
    );
    objects(
        &mut g,
        "req:v-footbridge-push-permissible",
        "req:v-footbridge-push-impermissible",
    );
    // One way only: the target reads refuted.
    assert_eq!(truth(&g, "req:v-footbridge-push-impermissible"), "fails");
    // Both ways: an open dispute.
    objects(
        &mut g,
        "req:v-footbridge-push-impermissible",
        "req:v-footbridge-push-permissible",
    );
    for side in [
        "req:v-footbridge-push-permissible",
        "req:v-footbridge-push-impermissible",
    ] {
        assert_eq!(truth(&g, side), "unknown", "{side}");
        let p = json(g.proof_reading(side).unwrap());
        assert_eq!(p["status"], "contested", "{side}: {p:#}");
    }
}
