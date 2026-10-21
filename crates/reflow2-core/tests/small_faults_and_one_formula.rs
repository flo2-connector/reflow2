//! SMALL FAULTS AND ONE FORMULA — the fifth and last PR of phase 6
//! (`epoch:planned-p6-argument-gaps`), the core's half.
//!
//! - **`req:small-read-back-faults-from-the-cogito-trial-are-fixed`**: a
//!   stated foundation is never listed unsupported (1); a generated step name
//!   follows its premises, added or deleted (2, 9); a kept calculator record is
//!   an executable form and a run fed back (11); a setpoint is never a budget
//!   (12); a walk from a check stops at what it checks (14). The MCP half pins
//!   the rest (`small_faults_are_fixed_on_the_wire.rs`).
//! - **`req:natural-deduction-gaps-from-the-logic-rung-are-closed`, item 3**, as
//!   `dec:idea-one-formula-under-several-suppositions-is-one-node-or-one-per-
//!   occurrence` chose (b): each occurrence names its formula
//!   (`same_formula_as`), the read-back groups them, and no truth moves.
//! - **`dec:idea-what-rests-on-unsourced-evidence-reads-supported-or-
//!   conditional`, option b**: what rests on a report no pinned source
//!   documents reads CONDITIONAL on it; on documented reports only, supported.
//!
//! Every test is a class over the fixtures that can carry it, read as JSON so
//! the file compiles on main 9b7b3bd; each FAILED there first.

use std::collections::BTreeSet;

use reflow2_core::DesignGraph;
use reflow2_core::export::GraphExport;
use reflow2_core::nodes::Props;
use serde_json::{Value as Json, json};

const LOGIC: &str = include_str!("fixtures/proof_trial_logic_rung.json");
const CAPSTONE: &str = include_str!("fixtures/argument_trial_capstone.json");
const TROLLEY: &str = include_str!("fixtures/argument_trial_trolley.json");
const COGITO: &str = include_str!("fixtures/argument_trial_cogito.json");
const PI_E: &str = include_str!("fixtures/proof_trial_pi_plus_e.json");
const ARROW: &str = include_str!("fixtures/proof_trial_arrow.json");
const BROCARD: &str = include_str!("fixtures/proof_trial_brocard.json");

const ALL: [(&str, &str); 7] = [
    ("logic", LOGIC),
    ("capstone", CAPSTONE),
    ("trolley", TROLLEY),
    ("cogito", COGITO),
    ("pi+e", PI_E),
    ("arrow", ARROW),
    ("brocard", BROCARD),
];

fn doc(text: &str) -> Json {
    serde_json::from_str(text).expect("fixture parses")
}

fn load_json(d: &Json) -> DesignGraph {
    let d: GraphExport = serde_json::from_value(d.clone()).expect("an export");
    let mut g = DesignGraph::open_in_memory().expect("in-memory graph");
    g.import_graph(&d).expect("fixture imports");
    g
}

fn load(text: &str) -> DesignGraph {
    load_json(&doc(text))
}

fn js<T: serde::Serialize>(v: T) -> Json {
    serde_json::to_value(v).expect("serializes")
}

fn set_prop(d: &mut Json, id: &str, key: &str, value: Json) {
    let n = d["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["node_id"] == id)
        .unwrap_or_else(|| panic!("{id} in the fixture"));
    n["properties"][key] = value;
}

fn step_name(g: &DesignGraph, id: &str) -> String {
    g.get_node("Inference", id)
        .unwrap()
        .and_then(|n| {
            n.properties
                .get("name")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default()
}

// ─── 1. A stated foundation is not unsupported ────────────────────────────

/// ITEM 1, THE CLASS over every fixture: no claim the read-back lists under
/// `unsupported` is taken as given — a stated foundation is under
/// `stated_foundations`, never "no step concludes it". On main the cogito's
/// contested foundations were listed in both.
#[test]
fn a_stated_foundation_is_never_listed_unsupported() {
    let mut checked = 0;
    for (name, text) in ALL {
        let g = load(text);
        let r = js(g.reasoning_report(None, Some(500)).unwrap());
        let given: BTreeSet<String> = r["claims"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["taken_as_given"] == true)
            .map(|c| c["id"].as_str().unwrap().to_string())
            .collect();
        for u in r["unsupported"].as_array().unwrap() {
            let id = u["id"].as_str().unwrap();
            assert!(
                !given.contains(id),
                "{name}: {id} is a stated foundation: {u:#}"
            );
        }
        checked += given.len();
    }
    assert!(checked > 20, "{checked}");
}

// ─── 2 and 9. A generated step name follows its premises ──────────────────

/// ITEMS 2 AND 9: a step whose name reflow2 generated says how many premises
/// it has after one is added and after one is deleted; a name somebody wrote
/// is never touched. On main the cogito step read "all of 1 premise(s)" after
/// a second was added, and the trolley step kept its count after a deletion.
#[test]
fn a_generated_step_name_follows_its_premises() {
    let mut g = DesignGraph::open_in_memory().unwrap();
    for id in ["req:a", "req:b", "req:c", "req:d", "req:e"] {
        g.add_requirement(id, id, id).unwrap();
    }
    let spec = |id: &str, premises: &[&str], conclusion: &str, name: Option<&str>| {
        reflow2_core::inference::InferenceSpec {
            id: id.into(),
            rule: "all".into(),
            premises: premises.iter().map(|s| s.to_string()).collect(),
            conclusion: conclusion.into(),
            name: name.map(str::to_string),
            ..Default::default()
        }
    };
    g.add_inference(&spec("inf:gen", &["req:a"], "req:c", None))
        .unwrap();
    assert_eq!(
        step_name(&g, "inf:gen"),
        "all of 1 premise(s) support req:c"
    );
    g.add_inference(&spec("inf:gen", &["req:b"], "req:c", None))
        .unwrap();
    assert_eq!(
        step_name(&g, "inf:gen"),
        "all of 2 premise(s) support req:c"
    );
    g.delete_edge("PREMISE_OF", "req:a", "inf:gen").unwrap();
    assert_eq!(
        step_name(&g, "inf:gen"),
        "all of 1 premise(s) support req:c"
    );

    g.add_inference(&spec(
        "inf:named",
        &["req:a"],
        "req:e",
        Some("modus ponens on A"),
    ))
    .unwrap();
    g.add_inference(&spec("inf:named", &["req:d"], "req:e", None))
        .unwrap();
    g.delete_edge("PREMISE_OF", "req:a", "inf:named").unwrap();
    assert_eq!(step_name(&g, "inf:named"), "modus ponens on A");
}

// ─── 11. A kept calculator record is a run that came back ─────────────────

/// ITEM 11, THE CLASS over every check of every fixture: a computed check
/// whose record is kept (a pinned `.calc.json`) has an executable form, and
/// the loop counts it as compared to a run — the logic rung's four truth
/// tables read "never closed" and "no executable form" on main.
#[test]
fn a_kept_calculator_record_is_an_executable_form_and_a_run_fed_back() {
    let mut kept = 0;
    for (name, text) in ALL {
        let d = doc(text);
        // Read from the export, as a reader would: a check that says it is
        // computed, documented by an Artifact that a checksum pins.
        let pinned: BTreeSet<&str> = d["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["node_type"] == "Artifact" && n["properties"]["checksum"].is_string())
            .map(|n| n["node_id"].as_str().unwrap())
            .collect();
        let kept_checks: BTreeSet<&str> = d["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| {
                n["node_type"] == "Verification"
                    && (n["properties"]["computed"] == true
                        || n["properties"]["location"]
                            .as_str()
                            .is_some_and(|l| l.ends_with(".calc.json")))
            })
            .map(|n| n["node_id"].as_str().unwrap())
            .filter(|v| {
                d["edges"].as_array().unwrap().iter().any(|e| {
                    e["edge_type"] == "DOCUMENTS"
                        && e["to_id"] == *v
                        && pinned.contains(e["from_id"].as_str().unwrap())
                })
            })
            .collect();
        let g = load_json(&d);
        let recency = js(g.verification_recency().unwrap());
        for v in recency.as_array().unwrap() {
            let id = v["verification_id"].as_str().unwrap();
            if kept_checks.contains(id) {
                kept += 1;
                assert_eq!(v["has_executable_form"], true, "{name} {id}: {v:#}");
            }
        }
        let closure = js(g.loop_closure().unwrap());
        if name == "logic" {
            assert_ne!(closure["state"], "never_closed", "{closure:#}");
            assert!(
                closure["compared_to_a_run"].as_u64().unwrap() >= 4,
                "{closure:#}"
            );
        }
    }
    assert!(kept >= 4, "{kept}");
}

// ─── 12. A setpoint is not a budget ───────────────────────────────────────

/// ITEM 12: a test parameter recorded as a setpoint ("carrier 9.4 GHz") is a
/// value held, never a maximum: what it constrains cannot exceed it, and it
/// holds. A constraint given a limit and no direction still reads as a
/// maximum, as it always has. On main `setpoint` was no direction at all.
#[test]
fn a_setpoint_is_a_value_held_never_a_budget() {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_constraint(
        "con:carrier",
        "Emulator carrier 9.4 GHz",
        "The emulator transmits at 9.4 GHz.",
        None,
        Some("frequency"),
        Some(9.4),
        None,
        Some("setpoint"),
    )
    .unwrap();
    g.add_constraint(
        "con:mass",
        "Mass under 9.4 kg",
        "Mass stays under 9.4 kg.",
        None,
        Some("mass"),
        Some(9.4),
        None,
        None,
    )
    .unwrap();
    g.add_requirement("req:emulator", "The emulator", "The emulator.")
        .unwrap();
    for c in ["con:carrier", "con:mass"] {
        g.create_edge(
            "CONSTRAINS",
            "Constraint",
            c,
            "Requirement",
            "req:emulator",
            Props::new().set("contribution", 12.0),
        )
        .unwrap();
    }
    let carrier = js(g.budget_report("con:carrier").unwrap());
    assert_eq!(carrier["verdict"], "ungated", "{carrier:#}");
    assert_eq!(g.premise_truth("con:carrier").unwrap().0.as_str(), "holds");
    let mass = js(g.budget_report("con:mass").unwrap());
    assert_eq!(mass["verdict"], "exceeded", "{mass:#}");
    assert_eq!(mass["direction"], "maximum", "{mass:#}");
}

// ─── 14. A walk from a check stops at what it checks ──────────────────────

/// ITEM 14, THE CLASS over every check of every fixture: a blast radius
/// seeded at a check climbs VERIFIES to what the check checks (and on to what
/// that delivers), and never crosses a GOVERNED_BY after it — no Decision is
/// changed by a test, nor the requirements it governs. On main, from the
/// capstone's threat test, the walk climbed VERIFIES and GOVERNED_BY and
/// listed requirements the test never touches at distance 4–5.
#[test]
fn a_walk_from_a_check_never_crosses_a_governed_by() {
    let mut crossed_before = 0;
    for (name, text) in ALL {
        let g = load(text);
        for v in g.scan_nodes("Verification").unwrap() {
            let r = js(g
                .propagate_from(
                    &[v.node_id.as_str()],
                    reflow2_core::propagate::PropagateOptions::default(),
                )
                .unwrap());
            for n in r["impacted"].as_array().unwrap() {
                let via = n["via"].as_array().unwrap();
                let climbed = via
                    .iter()
                    .position(|h| h["edge_type"] == "VERIFIES" && h["direction"] == "upstream");
                if let Some(at) = climbed {
                    assert!(
                        !via[at..].iter().any(|h| h["edge_type"] == "GOVERNED_BY"),
                        "{name} {}: reached {} across a GOVERNED_BY after a VERIFIES climb: {n:#}",
                        v.node_id,
                        n["node_id"]
                    );
                }
            }
            crossed_before += 1;
        }
    }
    assert!(crossed_before > 5, "{crossed_before}");
}

// ─── ND item 3: one formula, one record per occurrence ────────────────────

/// ITEM 3, as decision 1 chose (b): the logic rung's twenty-one ⊥ each name
/// the first as the formula they are occurrences of; the read-back groups
/// them as one formula, and every claim reads exactly as before. On main
/// `same_formula_as` was no property and nothing grouped.
#[test]
fn one_formula_is_one_record_per_occurrence_and_the_read_back_groups_them() {
    let before = doc(LOGIC);
    let mut after = before.clone();
    let falsums: Vec<String> = before["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["properties"]["statement_kind"] == "falsum")
        .map(|n| n["node_id"].as_str().unwrap().to_string())
        .collect();
    assert!(falsums.len() >= 20);
    let first = falsums[0].clone();
    for f in &falsums[1..] {
        set_prop(&mut after, f, "same_formula_as", json!(first));
    }
    let g0 = load_json(&before);
    let g1 = load_json(&after);
    let r = js(g1.reasoning_report(None, Some(500)).unwrap());
    let formulas = r["formulas"].as_array().expect("a formulas list");
    assert_eq!(formulas.len(), 1, "{formulas:#?}");
    let occ: BTreeSet<String> = formulas[0]["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o.as_str().unwrap().to_string())
        .collect();
    assert_eq!(formulas[0]["formula"], first.as_str());
    assert_eq!(occ, falsums.iter().cloned().collect::<BTreeSet<_>>());
    for st in g0.scan_nodes("Inference").unwrap() {
        let Some(c) = g0.conclusion_of(&st.node_id).unwrap() else {
            continue;
        };
        assert_eq!(
            g1.premise_truth(&c).unwrap().0,
            g0.premise_truth(&c).unwrap().0,
            "{c}"
        );
    }
}

// ─── Decision 2(b): undocumented evidence leaves what rests on it conditional

const REPORTS: [(&str, bool); 5] = [
    ("req:rpt-r1-humint-defector-x-band", false),
    ("req:rpt-r2-sigint-9-4ghz-pulse-doppler", true),
    ("req:rpt-r3-osint-brochure-kestrel-e-agile", false),
    ("req:rpt-r4-imint-antenna-1-2m", false),
    ("req:rpt-u1-sigint-2026-ku-band-export-not-fielded", true),
];

/// The capstone with its reports as evidence and R2 and U1 documented by a
/// pinned source, as `evidence_and_natural_deduction.rs` records it.
fn capstone_with_evidence() -> Json {
    let mut d = doc(CAPSTONE);
    for (id, documented) in REPORTS {
        set_prop(&mut d, id, "statement_kind", json!("evidence"));
        let n = d["nodes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|n| n["node_id"] == id)
            .unwrap();
        n["properties"]
            .as_object_mut()
            .unwrap()
            .remove("taken_as_given");
        if documented {
            let art = format!("art:source-of-{}", &id[4..]);
            d["nodes"].as_array_mut().unwrap().push(json!({
                "node_id": art,
                "node_type": "Artifact",
                "properties": {"name": format!("FICTIONAL source of {id}"), "artifact_type": "document",
                               "location": format!("reports/{id}.pdf"), "status": "realized",
                               "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000001"}
            }));
            d["edges"].as_array_mut().unwrap().push(json!({
                "edge_type": "DOCUMENTS", "from_id": art, "to_id": id, "properties": {}
            }));
        }
    }
    d
}

/// DECISION 2(b), THE CLASS over every claim of the capstone that rests on
/// evidence: one resting on a report no pinned source documents never reads
/// `supported`, `shown` or `proven` — it holds only conditionally on that
/// report, and its read-back names it among the unverified; one resting only
/// on documented reports and nothing else unverified reads `supported`. Both
/// kinds occur. #782 read every one of them supported.
#[test]
fn what_rests_on_an_undocumented_report_reads_conditional_on_it() {
    let g = load_json(&capstone_with_evidence());
    let undocumented: BTreeSet<&str> = REPORTS
        .iter()
        .filter(|(_, d)| !d)
        .map(|(id, _)| *id)
        .collect();
    let r = js(g.reasoning_report(None, Some(500)).unwrap());
    let (mut conditional, mut supported) = (0, 0);
    for c in r["claims"].as_array().unwrap() {
        let id = c["id"].as_str().unwrap();
        let tests: Vec<String> = g.tests_under(id).unwrap();
        if tests.is_empty() || undocumented.contains(id) {
            continue;
        }
        let status = c["proof_status"].as_str().unwrap_or("");
        let unverified: BTreeSet<String> = c["rests_on_unverified"]
            .as_array()
            .map(|a| a.iter().map(|v| v.as_str().unwrap().to_string()).collect())
            .unwrap_or_default();
        let on_undocumented = tests.iter().any(|t| undocumented.contains(t.as_str()));
        if on_undocumented && c["truth"] != "fails" && c["truth"] != "unknown" {
            assert!(
                !matches!(status, "supported" | "shown" | "proven"),
                "{id} rests on an undocumented report and reads {status}: {c:#}"
            );
            assert!(
                tests
                    .iter()
                    .filter(|t| undocumented.contains(t.as_str()))
                    .all(|t| unverified.contains(t)),
                "{id}: {c:#}"
            );
            if status == "conditional" {
                conditional += 1;
            }
        }
        if !on_undocumented && unverified.is_empty() && status == "supported" {
            supported += 1;
        }
    }
    assert!(
        conditional > 0,
        "no claim reads conditional on an undocumented report"
    );
    assert!(
        supported > 0,
        "no claim reads supported on documented reports alone"
    );
}
