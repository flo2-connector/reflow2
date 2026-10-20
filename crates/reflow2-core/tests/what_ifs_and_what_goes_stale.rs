//! WHAT-IFS AND WHAT GOES STALE — the third PR of phase 6
//! (`epoch:planned-p6-argument-gaps`).
//!
//! - **`req:a-what-if-on-an-assumed-principle-shows-what-holds-only-under-it`**:
//!   dropping a principle the steps take up in `assumed` reads every position
//!   on it — which lose all support, which still hold on another line (and
//!   under what) — where it used to say nothing.
//! - **`req:a-what-if-on-a-premise-reaches-only-what-rests-on-it`**: a path
//!   from a statement of the reasoning may climb a DECOMPOSES to its parent,
//!   and never fans back down to the parent's other parts.
//! - **`req:a-what-if-shows-what-a-premise-costs-a-contested-conclusion-and-objections-that-return`**:
//!   a row says what its claim read before (`conclusion_was`), a claim that
//!   was only contested and is left with nothing is carried on, the claim's
//!   routes the change never reaches are named (`untouched_routes`), and an
//!   objection whose reply the change takes away comes back
//!   (`objections_returning`).
//! - **`req:a-check-resting-on-a-claim-that-loses-support-reads-stale`**.
//! - **`req:a-claim-derived-under-a-refuted-supposition-says-so`**.
//! - **`req:what-has-lost-its-reason-now-is-one-read`**.
//! - **`req:a-requirements-read-back-follows-its-reason-to-the-evidence`**.
//!
//! Every test is a CLASS over every fixture that can carry it — the trolley,
//! cogito and capstone designs (the capstone is fictional, exported from
//! flo2.io 2a26dc6044306d27 on 2026-10-20) and the five proof trials — plus
//! the measured shape built by hand. Read as JSON, so the file compiles on
//! main 48f2865; each test FAILED there first.

use std::collections::BTreeSet;

use reflow2_core::DesignGraph;
use reflow2_core::export::GraphExport;
use reflow2_core::foundation::core::Value;
use reflow2_core::nodes::Props;
use reflow2_core::propagate::PropagateOptions;
use serde_json::Value as Json;

const TROLLEY: &str = include_str!("fixtures/argument_trial_trolley.json");
const COGITO: &str = include_str!("fixtures/argument_trial_cogito.json");
const CAPSTONE: &str = include_str!("fixtures/argument_trial_capstone.json");
const PI_E: &str = include_str!("fixtures/proof_trial_pi_plus_e.json");
const ARROW: &str = include_str!("fixtures/proof_trial_arrow.json");
const BROCARD: &str = include_str!("fixtures/proof_trial_brocard.json");
const RUNG_1: &str =
    include_str!("../../reflow2-mcp/tests/fixtures/proof_trial_rung_1_primes.json");
const RUNG_2: &str = include_str!("../../reflow2-mcp/tests/fixtures/proof_trial_rung_2_fta.json");

const ALL: [(&str, &str); 8] = [
    ("trolley", TROLLEY),
    ("cogito", COGITO),
    ("capstone", CAPSTONE),
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

fn statement(g: &mut DesignGraph, id: &str, kind: &str, given: bool) {
    g.add_requirement(id, &format!("statement {id}"), "A statement.")
        .unwrap();
    let mut props = Props::new().set("statement_kind", kind);
    if given {
        props = props.set("taken_as_given", Value::Bool(true));
    }
    g.upsert_node("Requirement", id, props).unwrap();
}

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

fn what_if(g: &DesignGraph, seed: &str) -> Json {
    json(
        g.propagate_from(&[seed], PropagateOptions::default())
            .unwrap()
            .summarize(),
    )
}

fn rows(w: &Json) -> Vec<Json> {
    w["inferences"].as_array().cloned().unwrap_or_default()
}

fn claim(g: &DesignGraph, id: &str) -> Json {
    json(g.reasoning_report(Some(id), Some(500)).unwrap())["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .cloned()
        .unwrap_or_else(|| panic!("{id} not in its own read-back"))
}

/// Every line a claim reading holds on: `holds_under_any_of`, else its own
/// `assumes`.
fn line_assumptions(c: &Json) -> Vec<BTreeSet<String>> {
    match c["holds_under_any_of"].as_array() {
        Some(a) if !a.is_empty() => a
            .iter()
            .map(|l| strs(&l["assumes"]).into_iter().collect())
            .collect(),
        _ => vec![strs(&c["assumes"]).into_iter().collect()],
    }
}

// ─── 1. A what-if on an assumed principle ───────────────────────────────────

/// THE CLASS, over every node a step takes up in `assumed`, in every fixture:
/// a what-if dropping it has a row for every step that takes it up; a claim
/// that held conditionally and every one of whose lines rests on it reads no
/// longer supported; one with a line that does not rests on that line, and
/// reads `still_supported` with `still_holds_under` naming no line under the
/// dropped principle. On main a what-if on an assumed principle had no rows.
#[test]
fn a_what_if_on_an_assumed_principle_reads_every_position_on_it() {
    let mut asked = 0;
    for (name, doc) in ALL {
        let g = load(doc);
        let mut assumed: BTreeSet<String> = BTreeSet::new();
        for st in g.scan_nodes("Inference").unwrap() {
            for e in g.incoming(&st.node_id, Some("PREMISE_OF")).unwrap() {
                if e.properties.get("role").and_then(Value::as_str) == Some("assumed") {
                    assumed.insert(e.from_id);
                }
            }
        }
        // Every claim's reading, from one whole-design walk.
        let index: std::collections::BTreeMap<String, Json> =
            json(g.reasoning_report(None, Some(500)).unwrap())["claims"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| (c["id"].as_str().unwrap().to_string(), c.clone()))
                .collect();
        for p in assumed {
            // The walk itself: the rows propagate_from carries whole.
            let rows: Vec<Json> = g
                .inference_walk(&[p.as_str()])
                .unwrap()
                .into_iter()
                .map(json)
                .collect();
            let reached: BTreeSet<String> = rows
                .iter()
                .filter_map(|r| r["inference"].as_str().map(str::to_string))
                .collect();
            for e in g.outgoing(&p, Some("PREMISE_OF")).unwrap() {
                assert!(
                    reached.contains(&e.to_id),
                    "{name}: dropping {p} has no row for {}, which takes it up",
                    e.to_id
                );
            }
            for r in &rows {
                let Some(c) = r["conclusion"].as_str() else {
                    continue;
                };
                if r["conclusion_was"] != "conditional" {
                    continue;
                }
                let before = index.get(c).cloned().unwrap_or_else(|| claim(&g, c));
                let lines = line_assumptions(&before);
                let all_on_it = lines.iter().all(|l| l.contains(&p));
                asked += 1;
                if all_on_it {
                    assert_ne!(
                        r["conclusion_reads"], "still_supported",
                        "{name}: dropping {p}, {c} held only under it: {r:#}"
                    );
                } else {
                    assert_eq!(
                        r["conclusion_reads"], "still_supported",
                        "{name}: dropping {p}, {c} has a line not under it: {r:#}"
                    );
                    for l in r["still_holds_under"].as_array().unwrap() {
                        assert!(
                            !strs(&l["assumes"]).contains(&p),
                            "{name}: {c} still holds under {p}: {r:#}"
                        );
                    }
                }
            }
        }
    }
    assert!(asked >= 10, "the class is not vacuous: {asked}");
}

/// THE MEASURED SHAPE, the trolley observer's fall-out worked by hand:
/// dropping act consequentialism leaves the footbridge "push" verdict
/// unsupported, and the bystander "turn" verdict standing on double effect.
#[test]
fn dropping_act_consequentialism_on_the_trolley_design() {
    let g = load(TROLLEY);
    let w = what_if(&g, "con:pr-act-consequentialism");
    let row = |c: &str| {
        rows(&w)
            .into_iter()
            .find(|r| r["conclusion"] == c)
            .unwrap_or_else(|| panic!("no row for {c}: {w:#}"))
    };
    let push = row("req:v-footbridge-push-permissible");
    assert_eq!(push["conclusion_reads"], "unsupported", "{push:#}");
    let turn = row("req:v-switch-turn-permissible");
    assert_eq!(turn["conclusion_reads"], "still_supported", "{turn:#}");
    assert!(
        strs(&turn["still_supported_by"]).contains(&"inf:switch-by-double-effect".to_string()),
        "{turn:#}"
    );
    let under: Vec<String> = turn["still_holds_under"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|l| strs(&l["assumes"]))
        .collect();
    assert!(
        under.contains(&"con:pr-double-effect".to_string()),
        "{turn:#}"
    );
}

// ─── 2. A what-if on a premise reaches what rests on it ──────────────────────

/// THE CLASS, over every statement that takes part in reasoning in every
/// fixture, and the stage-5 shape built by hand: no path of its what-if climbs
/// a DECOMPOSES to a parent and then turns back down another. On the hand-built
/// shape, two premises of one deliverable reach different totals. On main each
/// premise of the stage-5 deliverable reached the same 66 nodes.
#[test]
fn a_what_if_on_a_premise_reaches_only_what_rests_on_it() {
    let check = |name: &str, g: &DesignGraph, seed: &str| {
        let r = json(
            g.propagate_from(&[seed], PropagateOptions::default())
                .unwrap(),
        );
        for n in r["impacted"].as_array().unwrap() {
            let mut climbed = false;
            for hop in n["via"].as_array().unwrap() {
                if hop["edge_type"] != "DECOMPOSES" {
                    continue;
                }
                if hop["direction"] == "upstream" {
                    climbed = true;
                } else if climbed {
                    panic!(
                        "{name}: a what-if on {seed} reached {} down a sibling DECOMPOSES: {n:#}",
                        n["node_id"]
                    );
                }
            }
        }
        r["impacted"].as_array().unwrap().len()
    };
    for (name, doc) in ALL {
        let g = load(doc);
        // A sample of each fixture's premises: a full radius per premise is
        // seconds each on a debug build.
        for st in g.scan_nodes("Inference").unwrap().into_iter().take(4) {
            if let Some(p) = g.premises_of(&st.node_id).unwrap().into_iter().next() {
                check(name, &g, &p);
            }
        }
    }
    // The stage-5 shape: three premises decompose the deliverable; each rests
    // its own steps.
    let mut g = graph();
    statement(
        &mut g,
        "req:argument-is-the-deliverable",
        "proposition",
        false,
    );
    for p in ["req:premise-2", "req:premise-5", "req:premise-6"] {
        statement(&mut g, p, "proposition", true);
        edge(
            &mut g,
            p,
            "DECOMPOSES",
            "req:argument-is-the-deliverable",
            Props::new(),
        );
    }
    for (i, c) in ["req:b1", "req:b2", "req:b3", "req:b4"].iter().enumerate() {
        statement(&mut g, c, "proposition", false);
        step(&mut g, &format!("inf:p5-{i}"), &["req:premise-5"], &[], c);
    }
    statement(&mut g, "req:c2", "proposition", false);
    step(&mut g, "inf:p2", &["req:premise-2"], &[], "req:c2");
    statement(&mut g, "req:c6", "proposition", false);
    step(&mut g, "inf:p6", &["req:premise-6"], &[], "req:c6");
    let p2 = check("stage-5", &g, "req:premise-2");
    let p5 = check("stage-5", &g, "req:premise-5");
    assert_ne!(p2, p5, "two premises' reach can be told apart");
    let r = json(
        g.propagate_from(&["req:premise-2"], PropagateOptions::default())
            .unwrap(),
    );
    let reached: Vec<&str> = r["impacted"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n["node_id"].as_str())
        .collect();
    assert!(!reached.contains(&"req:c6"), "{reached:?}");
    assert!(!reached.contains(&"req:b1"), "{reached:?}");
}

// ─── 3. What a premise costs a contested conclusion ─────────────────────────

/// A claim that was only contested is left with nothing; its other route the
/// change never reaches is named; and an objection whose reply the change
/// takes away comes back.
#[test]
fn a_what_if_says_what_a_contested_claim_loses_and_which_objections_come_back() {
    let mut g = graph();
    // "I exist", by route A (thinking, contested) and route B (evidence,
    // itself unsupported).
    statement(&mut g, "req:def-thought", "proposition", true);
    statement(&mut g, "req:i-am-thinking", "proposition", false);
    step(
        &mut g,
        "inf:thinking",
        &["req:def-thought"],
        &[],
        "req:i-am-thinking",
    );
    statement(&mut g, "req:i-exist", "proposition", false);
    step(
        &mut g,
        "inf:route-a",
        &["req:i-am-thinking"],
        &[],
        "req:i-exist",
    );
    statement(&mut g, "req:obj-lichtenberg", "objection", false);
    objects(&mut g, "req:obj-lichtenberg", "inf:route-a");
    statement(&mut g, "req:encounter-evidence", "proposition", false);
    step(
        &mut g,
        "inf:route-b",
        &["req:encounter-evidence"],
        &[],
        "req:i-exist",
    );
    assert_eq!(
        json(g.proof_reading("req:i-exist").unwrap())["status"],
        "contested"
    );
    let w = what_if(&g, "req:def-thought");
    let row = rows(&w)
        .into_iter()
        .find(|r| r["inference"] == "inf:route-a")
        .unwrap_or_else(|| panic!("route A is reached: {w:#}"));
    assert_eq!(row["conclusion_was"], "contested", "{row:#}");
    assert_eq!(row["conclusion_reads"], "unsupported", "{row:#}");
    assert_eq!(
        strs(&row["untouched_routes"]),
        vec!["inf:route-b".to_string()],
        "{row:#}"
    );

    // The walking objection, answered by a reply resting on one premise.
    statement(&mut g, "req:sensation-is-thought", "proposition", true);
    statement(&mut g, "req:reply-walking", "reply", false);
    step(
        &mut g,
        "inf:reply-walking",
        &["req:sensation-is-thought"],
        &[],
        "req:reply-walking",
    );
    statement(&mut g, "req:obj-walking", "objection", true);
    objects(&mut g, "req:obj-walking", "req:i-exist");
    objects(&mut g, "req:reply-walking", "req:obj-walking");
    let w = what_if(&g, "req:sensation-is-thought");
    let back = &w["objections_returning"];
    assert_eq!(back[0]["objection"], "req:obj-walking", "{w:#}");
    assert_eq!(back[0]["answered_by"], "req:reply-walking", "{w:#}");
    assert_eq!(strs(&back[0]["against"]), vec!["req:i-exist".to_string()]);
}

/// THE CLASS, over every fixture: a what-if on every premise of a step under a
/// contested claim has a row saying what the claim read before.
#[test]
fn every_row_says_what_its_claim_read_before() {
    let mut seen = 0;
    for (name, doc) in [
        ("cogito", COGITO),
        ("trolley", TROLLEY),
        ("capstone", CAPSTONE),
    ] {
        let g = load(doc);
        for st in g.scan_nodes("Inference").unwrap() {
            for p in g.premises_of(&st.node_id).unwrap() {
                for r in g
                    .inference_walk(&[p.as_str()])
                    .unwrap()
                    .into_iter()
                    .map(json)
                {
                    seen += 1;
                    assert!(
                        [
                            "holds",
                            "conditional",
                            "contested",
                            "unsupported",
                            "refuted"
                        ]
                        .contains(&r["conclusion_was"].as_str().unwrap_or("absent")),
                        "{name}: {r:#}"
                    );
                }
            }
        }
    }
    assert!(seen > 0);
}

// ─── 4. A check resting on claims that lose support reads stale ─────────────

/// THE CLASS, over every check a reasoned claim constrains in every fixture:
/// it reads stale exactly when one of its conditions does not hold outright,
/// and the reasoning debt counts it. Plus the measured shape: the capstone's
/// threat-emulator test, two of whose five settings lost their derivation
/// after U1, reads stale naming them (on main it stayed `planned` with no
/// marker).
#[test]
fn a_check_resting_on_claims_that_lose_support_reads_stale() {
    let mut seen = 0;
    for (name, doc) in ALL {
        let g = load(doc);
        let debt = json(g.reasoning_debt().unwrap());
        for v in g.scan_nodes("Verification").unwrap() {
            let c = claim(&g, &v.node_id)["check_conditions"].clone();
            if c.is_null() {
                continue;
            }
            seen += 1;
            let failing: Vec<&Json> = c["conditions"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|x| x["reads"] != "holds")
                .collect();
            assert_eq!(c["stale"], !failing.is_empty(), "{name}: {c:#}");
            if !failing.is_empty() {
                assert!(
                    strs(&debt["stale_check_ids"]).contains(&v.node_id),
                    "{name}: {} is stale and the debt does not count it",
                    v.node_id
                );
            }
        }
    }
    assert!(seen >= 1, "the class is not vacuous");
    let g = load(CAPSTONE);
    let c = claim(&g, "ver:kestrel-threat-emulator-test")["check_conditions"].clone();
    assert_eq!(c["stale"], true, "{c:#}");
    let failing: Vec<String> = c["conditions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|x| x["reads"] != "holds")
        .filter_map(|x| x["id"].as_str().map(str::to_string))
        .collect();
    assert!(
        failing.contains(&"con:emu-mode-b-agile-1000-hops-per-s".to_string()),
        "{c:#}"
    );

    // Hand-built: a PASSING check run against a setting concluded from an
    // assessment; a what-if on the assessment names it stale, and losing the
    // assessment for real makes the check read unknown as a premise.
    let mut g = graph();
    statement(&mut g, "req:assessment", "proposition", true);
    g.add_constraint(
        "con:setting",
        "Emulator setting",
        "Mode B: agile.",
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    g.upsert_node(
        "Constraint",
        "con:setting",
        Props::new().set("statement_kind", "proposition"),
    )
    .unwrap();
    step(
        &mut g,
        "inf:setting",
        &["req:assessment"],
        &[],
        "con:setting",
    );
    g.upsert_node(
        "Verification",
        "ver:test",
        Props::new().set("name", "test").set("status", "passing"),
    )
    .unwrap();
    edge(
        &mut g,
        "con:setting",
        "CONSTRAINS",
        "ver:test",
        Props::new(),
    );
    assert_eq!(g.premise_truth("ver:test").unwrap().0.as_str(), "holds");
    let w = what_if(&g, "req:assessment");
    assert_eq!(w["checks_gone_stale"][0]["check"], "ver:test", "{w:#}");
    statement(&mut g, "req:new-report", "proposition", true);
    objects(&mut g, "req:new-report", "req:assessment");
    let (t, why) = g.premise_truth("ver:test").unwrap();
    assert_eq!(t.as_str(), "unknown", "{why}");
    assert!(why.contains("stale"), "{why}");
}

// ─── 5. A claim derived under a refuted supposition ─────────────────────────

/// THE CLASS, over every step in every fixture taking up a supposition no step
/// discharges that reads refuted: the step does not hold and names it. Plus
/// the capstone's H-agile, derived under S2 after U1 refuted S2 (on main it
/// read `holds`), and the hand-built rule that an objection derived under a
/// refuted supposition no longer counts.
#[test]
fn a_claim_derived_under_a_refuted_supposition_says_so() {
    let mut seen = 0;
    for (name, doc) in ALL {
        let g = load(doc);
        let mut discharged: BTreeSet<String> = BTreeSet::new();
        for st in g.scan_nodes("Inference").unwrap() {
            for key in ["discharges", "cases"] {
                if let Some(Value::List(items)) = st.properties.get(key) {
                    discharged.extend(items.iter().filter_map(|v| v.as_str().map(str::to_string)));
                }
            }
        }
        for st in g.scan_nodes("Inference").unwrap() {
            for e in g.incoming(&st.node_id, Some("PREMISE_OF")).unwrap() {
                if e.properties.get("role").and_then(Value::as_str) != Some("assumed")
                    || discharged.contains(&e.from_id)
                    || g.premise_truth(&e.from_id).unwrap().0.as_str() != "fails"
                {
                    continue;
                }
                seen += 1;
                let r = json(g.inference_reading(&st.node_id).unwrap());
                assert_ne!(r["verdict"], "holds", "{name}: {r:#}");
                assert!(
                    strs(&r["rests_on_refuted_suppositions"]).contains(&e.from_id),
                    "{name}: {r:#}"
                );
            }
        }
    }
    assert!(seen >= 1, "the class is not vacuous");

    let mut g = graph();
    statement(&mut g, "req:s", "hypothesis", false);
    statement(&mut g, "req:report", "proposition", true);
    statement(&mut g, "req:obj-under-s", "objection", false);
    step(
        &mut g,
        "inf:under-s",
        &["req:report"],
        &["req:s"],
        "req:obj-under-s",
    );
    statement(&mut g, "req:target", "proposition", true);
    objects(&mut g, "req:obj-under-s", "req:target");
    statement(&mut g, "req:refutes-s", "proposition", true);
    objects(&mut g, "req:refutes-s", "req:s");
    assert_eq!(g.premise_truth("req:s").unwrap().0.as_str(), "fails");
    let (t, why) = g.premise_truth("req:target").unwrap();
    assert_eq!(
        t.as_str(),
        "holds",
        "an objection under a refuted supposition no longer counts: {why}"
    );
    let c = claim(&g, "req:obj-under-s");
    assert!(
        c["why"].as_str().unwrap().contains("refuted supposition"),
        "{c:#}"
    );
}

// ─── 6. What has lost its reason now ─────────────────────────────────────────

/// THE CLASS, over every fixture: the reasoning debt lists every requirement
/// whose reason to exist reads lost or stands on acceptance alone, every
/// accepted Decision no line of whose chain holds, and every stale check —
/// the state, read whole. The capstone's 10–12 GHz requirement, whose
/// Decision U1 rejected, is listed (on main only get_node on it said so).
#[test]
fn what_has_lost_its_reason_now_is_one_read() {
    let mut seen = 0;
    for (name, doc) in ALL {
        let g = load(doc);
        let Some(debt) = g.reasoning_debt().unwrap() else {
            continue;
        };
        let debt = json(debt);
        for r in g.scan_nodes("Requirement").unwrap() {
            let Some(rte) = g.reason_to_exist(&r.node_id).unwrap() else {
                continue;
            };
            if matches!(rte.reads, "lost" | "stands_on_acceptance_alone") {
                seen += 1;
                assert!(
                    strs(&debt["reason_lost_ids"]).contains(&r.node_id)
                        || debt["reasons_lost"].as_u64().unwrap() > 20,
                    "{name}: {} lost its reason and the read does not list it",
                    r.node_id
                );
            }
        }
    }
    assert!(seen >= 1, "the class is not vacuous");
    let g = load(CAPSTONE);
    let status = json(g.loop_status().unwrap());
    assert!(
        status.to_string().contains("WHAT HAS LOST ITS REASON NOW"),
        "loop_status names it"
    );
    assert!(
        strs(&status["reasoning"]["reason_lost_ids"])
            .contains(&"req:rwr-covers-10-to-12-ghz".to_string()),
        "{}",
        status["reasoning"]
    );
}

// ─── 7. A requirement's read-back follows its reason to the evidence ────────

/// THE CLASS, over every requirement GOVERNED_BY a Decision a chain concludes,
/// in every fixture: its read-back walks the Decision, its `rests_on` holds
/// every terminal premise of the Decision's chain, and its reason to exist
/// names them. And a claim another replaces feeds no chain. On main the
/// capstone's requirements walked 1 claim and rested on nothing, and A2 kept
/// a decision's chain holding after A2b replaced it.
#[test]
fn a_requirements_read_back_follows_its_reason_to_the_evidence() {
    let mut seen = 0;
    for (name, doc) in ALL {
        let g = load(doc);
        for r in g.scan_nodes("Requirement").unwrap() {
            for e in g.outgoing(&r.node_id, Some("GOVERNED_BY")).unwrap() {
                if type_of(&g, &e.to_id) != "Decision"
                    || g.inferences_concluding(&e.to_id).unwrap().is_empty()
                {
                    continue;
                }
                seen += 1;
                let rep = json(g.reasoning_report(Some(&r.node_id), Some(500)).unwrap());
                let walked: Vec<&str> = rep["claims"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|c| c["id"].as_str())
                    .collect();
                assert!(walked.contains(&e.to_id.as_str()), "{name}: {}", r.node_id);
                let rests_on = strs(&rep["rests_on"]);
                let rte = json(g.reason_to_exist(&r.node_id).unwrap());
                let decision = rte["governed_by"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|d| d["decision"] == e.to_id.as_str())
                    .cloned()
                    .unwrap();
                for f in g.foundations_of(&e.to_id).unwrap() {
                    if type_of(&g, &f.id) == "Inference" {
                        continue;
                    }
                    assert!(
                        rests_on.contains(&f.id),
                        "{name}: {} rests_on misses {}",
                        r.node_id,
                        f.id
                    );
                    assert!(
                        decision["chain_rests_on"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|x| x["id"] == f.id.as_str()),
                        "{name}: {} reason_to_exist misses {}",
                        r.node_id,
                        f.id
                    );
                }
            }
        }
    }
    assert!(seen >= 3, "the class is not vacuous: {seen}");
    let g = load(CAPSTONE);
    let (t, why) = g.premise_truth("req:asmt-a2-agility-unresolved").unwrap();
    assert_eq!(t.as_str(), "unknown", "{why}");
    assert!(why.contains("replaced by"), "{why}");
}
