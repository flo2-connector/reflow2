//! EVIDENCE AND NATURAL DEDUCTION — the fourth PR of phase 6
//! (`epoch:planned-p6-argument-gaps`).
//!
//! - **`req:evidence-carries-its-source-grade-and-confidence-and-is-not-intent`**:
//!   a report is recorded as `evidence` with its source reliability,
//!   information credibility and date, an analytic judgement with its
//!   confidence; both are fields read back; a piece of evidence is CITED AS
//!   EVIDENCE through the source that documents it, and what rests on it reads
//!   supported, never shown or proven; no statement of a line of reasoning is
//!   listed as intent awaiting the owner's word.
//! - **`req:natural-deduction-gaps-from-the-logic-rung-are-closed`**: a case
//!   split can name an assumed disjunction (1); a step's rule is a node of its
//!   own, never a premise (2); a premise can be used twice (4); a line says whether it
//!   establishes ⊨ or ⊢ (5); a what-if says which part of a conjunction or
//!   biconditional stands (6); a step resting on an unverified citation reads
//!   conditional AT THE STEP, and two lines sharing a cited foundation are not
//!   independent (7).
//!
//! Every test is a CLASS over every fixture that can carry it — the logic
//! rung (exported read-only from flo2.io e104c2efbcc0fb6d, 0.88.0), the
//! fictional capstone, the trolley and cogito designs and the proof trials —
//! with the trial's own shape transformed where the new form is what the
//! designer could not write. Read as JSON, so the file compiles on main
//! 4dd766d; each test FAILED there first. Item 3 (one formula under several
//! suppositions as one node) is NOT built: merging the rung's twenty ⊥ into
//! one closes circles through the reductios, which the design refuses; it is
//! the proposed dec:idea-one-formula-under-several-suppositions-is-one-node-or-
//! one-per-occurrence.

use std::collections::{BTreeMap, BTreeSet};

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
    let d: GraphExport = serde_json::from_value(d.clone()).expect("fixture reads as an export");
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

fn nodes_mut(d: &mut Json) -> &mut Vec<Json> {
    d["nodes"].as_array_mut().expect("nodes")
}

fn edges_mut(d: &mut Json) -> &mut Vec<Json> {
    d["edges"].as_array_mut().expect("edges")
}

fn set_prop(d: &mut Json, id: &str, key: &str, value: Json) {
    let n = nodes_mut(d)
        .iter_mut()
        .find(|n| n["node_id"] == id)
        .unwrap_or_else(|| panic!("{id} in the fixture"));
    n["properties"][key] = value;
}

fn props_of<'a>(d: &'a Json, id: &str) -> &'a Json {
    &d["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node_id"] == id)
        .unwrap_or_else(|| panic!("{id} in the fixture"))["properties"]
}

fn steps(g: &DesignGraph) -> Vec<String> {
    let mut v: Vec<String> = g
        .scan_nodes("Inference")
        .unwrap()
        .into_iter()
        .map(|n| n.node_id)
        .collect();
    v.sort();
    v
}

fn reading(g: &DesignGraph, step: &str) -> Json {
    js(g.inference_reading(step).unwrap().expect("a step"))
}

fn proof_status(g: &DesignGraph, id: &str) -> Option<String> {
    g.proof_reading(id)
        .unwrap()
        .map(|p| js(p)["status"].as_str().unwrap().to_string())
}

/// A node's stored properties, as JSON.
fn node_json(g: &DesignGraph, id: &str) -> Option<Json> {
    let held = g.node_types_holding(id).unwrap();
    let ty = held.first()?;
    g.get_node(ty, id).unwrap().map(|n| {
        let mut m = serde_json::Map::new();
        for (k, v) in n.properties {
            if let Some(t) = v.as_str() {
                m.insert(k, json!(t));
            } else if let Some(b) = v.as_bool() {
                m.insert(k, json!(b));
            }
        }
        Json::Object(m)
    })
}

/// Everything a step rests on, transitively: its premises, and what every step
/// concluding one of them rests on.
fn closure(g: &DesignGraph, step: &str) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut steps_seen = BTreeSet::new();
    let mut queue = vec![step.to_string()];
    while let Some(s) = queue.pop() {
        if !steps_seen.insert(s.clone()) {
            continue;
        }
        for p in g.premises_of(&s).unwrap() {
            if seen.insert(p.clone()) {
                queue.extend(g.inferences_concluding(&p).unwrap());
            }
        }
    }
    seen
}

fn concluded(g: &DesignGraph) -> BTreeSet<String> {
    steps(g)
        .iter()
        .filter_map(|s| g.conclusion_of(s).unwrap())
        .collect()
}

// ─── The capstone's reports, recorded as the requirement now allows ───────

/// The capstone's five reports and their grades, as the brief gave them, and
/// its assessments' confidence as their names state it.
const REPORTS: [(&str, &str, &str, &str); 5] = [
    ("req:rpt-r1-humint-defector-x-band", "C", "3", "2019"),
    ("req:rpt-r2-sigint-9-4ghz-pulse-doppler", "B", "2", "2025"),
    (
        "req:rpt-r3-osint-brochure-kestrel-e-agile",
        "C",
        "3",
        "2024",
    ),
    ("req:rpt-r4-imint-antenna-1-2m", "B", "2", "2025"),
    (
        "req:rpt-u1-sigint-2026-ku-band-export-not-fielded",
        "A",
        "1",
        "2026",
    ),
];

/// The capstone with its reports as EVIDENCE (graded, R2 and U1 documented
/// by a pinned source) and its assessments as JUDGEMENTS with their stated
/// confidence: what the designer could not write on 0.88.0.
fn capstone_with_evidence() -> Json {
    let mut d = doc(CAPSTONE);
    for (id, rel, cred, date) in REPORTS {
        set_prop(&mut d, id, "statement_kind", json!("evidence"));
        set_prop(&mut d, id, "taken_as_given", Json::Null);
        set_prop(&mut d, id, "source_reliability", json!(rel));
        set_prop(&mut d, id, "information_credibility", json!(cred));
        set_prop(&mut d, id, "reported_at", json!(date));
    }
    // A Null property is no property.
    for n in nodes_mut(&mut d) {
        if let Some(p) = n["properties"].as_object_mut() {
            p.retain(|_, v| !v.is_null());
        }
    }
    let ids: Vec<(String, String)> = d["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["node_id"].as_str().unwrap().starts_with("req:asmt-"))
        .map(|n| {
            let id = n["node_id"].as_str().unwrap().to_string();
            let name = n["properties"]["name"]
                .as_str()
                .unwrap_or("")
                .to_lowercase();
            let c = if name.contains("high") {
                "high"
            } else if name.contains("low conf") {
                "low"
            } else {
                "moderate"
            };
            (id, c.to_string())
        })
        .collect();
    for (id, c) in ids {
        set_prop(&mut d, &id, "statement_kind", json!("judgement"));
        set_prop(&mut d, &id, "confidence", json!(c));
    }
    for (report, art) in [
        (
            "req:rpt-r2-sigint-9-4ghz-pulse-doppler",
            "art:report-r2-intercept-summary",
        ),
        (
            "req:rpt-u1-sigint-2026-ku-band-export-not-fielded",
            "art:report-u1-update",
        ),
    ] {
        nodes_mut(&mut d).push(json!({
            "node_id": art,
            "node_type": "Artifact",
            "properties": {
                "name": format!("FICTIONAL source document for {report}"),
                "artifact_type": "document",
                "location": format!("reports/{art}.pdf"),
                "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000001",
                "status": "realized"
            }
        }));
        edges_mut(&mut d).push(json!({
            "edge_type": "DOCUMENTS",
            "from_id": art,
            "to_id": report,
            "properties": {}
        }));
    }
    d
}

/// THE CLASS, over every node of the capstone recorded as the requirement
/// allows: a piece of EVIDENCE reads `cited_as_evidence` exactly when a pinned
/// source documents it (else `evidence_unverified`); EVERY claim whose
/// support rests on evidence reads `supported`, never `shown` or `proven`, and
/// its step says so AT THE STEP (`holds_as: supported`); and at least one
/// does, so the class is not vacuous. On main `evidence` was no kind at all,
/// and the reports had to be propositions taken as given, read `proven`.
#[test]
fn evidence_is_cited_as_evidence_and_what_rests_on_it_is_supported_never_proven() {
    let d = capstone_with_evidence();
    let g = load_json(&d);
    for (id, ..) in REPORTS {
        let r = js(g.statement_reading(id).unwrap().expect("a statement"));
        let pinned = id.contains("-r2-") || id.contains("-u1-");
        assert_eq!(
            r["reads"],
            if pinned {
                "cited_as_evidence"
            } else {
                "evidence_unverified"
            },
            "{id}: {r:#}"
        );
        assert_eq!(r["kind"], "evidence", "{id}: {r:#}");
    }
    let mut supported = 0;
    for c in concluded(&g) {
        let tests = g.tests_under(&c).unwrap();
        let Some(status) = proof_status(&g, &c) else {
            continue;
        };
        if tests.is_empty() {
            continue;
        }
        assert!(
            !matches!(status.as_str(), "shown" | "proven"),
            "{c} rests on evidence {tests:?} and reads {status}"
        );
        if status == "supported" {
            supported += 1;
        }
    }
    assert!(supported > 0, "no claim reads supported by evidence");
    for s in steps(&g) {
        let r = reading(&g, &s);
        if r["verdict"] == "holds"
            && r.get("rests_on_tests").is_some()
            && r.get("assumes").is_none()
            && r.get("rests_on_unverified").is_none()
        {
            assert_eq!(r["holds_as"], "supported", "{s}: {r:#}");
        }
    }
}

/// THE CLASS, over every fixture and the capstone recorded with evidence: no
/// statement of a line of reasoning — anything carrying a `statement_kind`,
/// a report, a judgement, a lemma — is listed by loop_status as delivered
/// intent awaiting the owner's word. Measured 2026-10-19: the capstone listed
/// 7 analytic claims there, the logic rung 19 lemmas.
#[test]
fn no_statement_of_a_line_of_reasoning_awaits_the_owners_word() {
    let mut designs: Vec<(String, Json)> =
        ALL.iter().map(|(n, t)| (n.to_string(), doc(t))).collect();
    designs.push(("capstone+evidence".into(), capstone_with_evidence()));
    let mut statements_seen = 0;
    for (name, d) in &designs {
        let g = load_json(d);
        for a in g.awaiting_word().unwrap() {
            let a = js(a);
            let id = a["node_id"].as_str().unwrap();
            let Some(p) = node_json(&g, id) else {
                continue;
            };
            assert!(
                p.get("statement_kind").is_none(),
                "{name}: {id} is a statement of the reasoning, listed as awaiting the owner's word: {a:#}"
            );
        }
        statements_seen += d["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["properties"].get("statement_kind").is_some())
            .count();
    }
    assert!(statements_seen > 100, "{statements_seen}");
}

/// THE GRADES ARE FIELDS, READ BACK: every graded node of the capstone carries
/// its grade on its claim reading, and the whole-design read-back lists every
/// piece of evidence and every judgement with how it reads and its grade.
#[test]
fn grades_and_confidence_are_fields_the_read_back_shows() {
    let d = capstone_with_evidence();
    let g = load_json(&d);
    let r = js(g.reasoning_report(None, Some(500)).unwrap());
    let listed: BTreeMap<String, Json> = r["evidence"]
        .as_array()
        .expect("an evidence list")
        .iter()
        .map(|e| (e["id"].as_str().unwrap().to_string(), e.clone()))
        .collect();
    for n in d["nodes"].as_array().unwrap() {
        let id = n["node_id"].as_str().unwrap();
        let p = &n["properties"];
        let kind = p["statement_kind"].as_str();
        if !matches!(kind, Some("evidence" | "judgement")) {
            continue;
        }
        let e = listed
            .get(id)
            .unwrap_or_else(|| panic!("{id} ({kind:?}) not in the read-back's evidence list"));
        for key in [
            "source_reliability",
            "information_credibility",
            "reported_at",
            "confidence",
        ] {
            assert_eq!(e["grade"][key], p[key], "{id} {key}: {e:#}");
        }
        assert_eq!(e["kind"], *kind.as_ref().unwrap(), "{id}");
    }
    let r2 = &listed["req:rpt-r2-sigint-9-4ghz-pulse-doppler"];
    assert_eq!(r2["reads"], "cited_as_evidence", "{r2:#}");
    assert_eq!(r2["grade"]["source_reliability"], "B", "{r2:#}");
}

// ─── Natural deduction, on the logic rung ─────────────────────────────────

/// The logic rung with each rule Constraint moved OUT of the premises into the
/// step's `rule_node`, as the designer wanted to record it.
fn logic_with_rule_nodes(d: &Json) -> (Json, BTreeMap<String, Vec<String>>) {
    let mut d = d.clone();
    let rules: BTreeSet<String> = d["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["node_id"].as_str().unwrap().to_string())
        .filter(|id| id.starts_with("con:rule-"))
        .collect();
    let mut ruled: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut moves: Vec<(String, String)> = Vec::new();
    edges_mut(&mut d).retain(|e| {
        let from = e["from_id"].as_str().unwrap();
        if e["edge_type"] == "PREMISE_OF" && rules.contains(from) {
            moves.push((e["to_id"].as_str().unwrap().to_string(), from.to_string()));
            false
        } else {
            true
        }
    });
    for (step, rule) in moves {
        assert!(
            props_of(&d, &step).get("rule_node").is_none(),
            "{step} has two rule premises"
        );
        set_prop(&mut d, &step, "rule_node", json!(rule));
        ruled.entry(rule).or_default().push(step);
    }
    (d, ruled)
}

/// What a what-if on `seed` leaves unsupported: the conclusions of its rows
/// that no longer hold.
fn lost_by(g: &DesignGraph, seed: &str) -> BTreeSet<String> {
    js(g.inference_walk(&[seed]).unwrap())
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["conclusion_after"] != "holds")
        .filter_map(|r| r["conclusion"].as_str().map(str::to_string))
        .collect()
}

/// ITEM 2, THE CLASS over every rule of the logic rung: with each rule moved
/// out of the premises into `rule_node`, a what-if on the rule leaves exactly
/// what it left as a premise, every step reads its rule node and holds as it
/// did, and no rule is listed among what the reasoning rests on, its uncited
/// premises or its stated foundations. On main a rule was only a label, so
/// the designer listed the 14 rule Constraints as premises.
#[test]
fn a_steps_rule_is_a_node_of_its_own_never_a_premise() {
    let before = doc(LOGIC);
    let (after, ruled) = logic_with_rule_nodes(&before);
    assert!(ruled.len() >= 10, "{ruled:?}");
    let g0 = load_json(&before);
    let g1 = load_json(&after);
    for (rule, steps_of) in &ruled {
        assert_eq!(
            lost_by(&g1, rule),
            lost_by(&g0, rule),
            "a what-if on {rule} as the steps' rule leaves what it left as their premise"
        );
        for s in steps_of {
            let r = reading(&g1, s);
            assert_eq!(r["rule_node"]["id"], rule.as_str(), "{s}: {r:#}");
            assert_eq!(r["verdict"], reading(&g0, s)["verdict"], "{s}: {r:#}");
            assert!(
                !r["premises"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["id"] == rule.as_str()),
                "{s}: {r:#}"
            );
        }
    }
    let r = js(g1.reasoning_report(None, Some(500)).unwrap());
    for list in ["rests_on", "uncited_premises", "stated_foundations"] {
        for id in r[list].as_array().unwrap() {
            assert!(
                !ruled.contains_key(id.as_str().unwrap()),
                "{list} lists the rule {id}"
            );
        }
    }
    // A rule that does not hold leaves its steps without support, never
    // refuted: drop RAA, and excluded middle by deduction is unsupported.
    let raa = lost_by(&g1, "con:rule-raa");
    assert!(
        !raa.is_empty() && raa.iter().all(|c| !c.is_empty()),
        "{raa:?}"
    );
}

/// ITEM 4: a premise named twice is used twice — A, A ⊢ A∧A — one edge
/// carrying `uses: 2`, read once by the rule. On main the second mention was
/// dropped without a word, and the step read "all of 1 premise(s)".
#[test]
fn a_premise_can_be_used_twice() {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_requirement("req:a", "A", "A.").unwrap();
    g.upsert_node(
        "Requirement",
        "req:a",
        Props::new().set("statement_kind", "axiom").set(
            "taken_as_given",
            reflow2_core::foundation::core::Value::Bool(true),
        ),
    )
    .unwrap();
    g.add_requirement("req:a-and-a", "A ∧ A", "A and A.")
        .unwrap();
    g.add_inference(&reflow2_core::inference::InferenceSpec {
        id: "inf:and-intro".into(),
        rule: "all".into(),
        premises: vec!["req:a".into(), "req:a".into()],
        conclusion: "req:a-and-a".into(),
        rule_name: Some("∧I".into()),
        ..Default::default()
    })
    .unwrap();
    let r = reading(&g, "inf:and-intro");
    assert_eq!(r["verdict"], "holds", "{r:#}");
    let p = &r["premises"].as_array().unwrap()[0];
    assert_eq!(p["id"], "req:a", "{r:#}");
    assert_eq!(p["uses"], 2, "{r:#}");
    assert_eq!(r["premises"].as_array().unwrap().len(), 1, "{r:#}");
}

/// ITEM 1: a case split names a disjunction ASSUMED in the same subproof as
/// its exhaustiveness: it is read as assumed there, the split holds under it,
/// and the →-introduction that discharges it holds outright. On main the
/// disjunction was drawn as an ordinary premise, read by its standing status
/// (a hypothesis, not established), and the discharge was refused as
/// "not assumed anywhere".
#[test]
fn a_case_split_can_name_an_assumed_disjunction() {
    let mut g = DesignGraph::open_in_memory().unwrap();
    for (id, kind) in [
        ("req:a-or-b", "hypothesis"),
        ("req:a", "hypothesis"),
        ("req:b", "hypothesis"),
        ("req:c-under-a", "lemma"),
        ("req:c-under-b", "lemma"),
        ("req:c", "lemma"),
        ("req:a-or-b-implies-c", "theorem"),
        ("req:a-implies-c", "axiom"),
        ("req:b-implies-c", "axiom"),
    ] {
        g.add_requirement(id, id, id).unwrap();
        g.upsert_node("Requirement", id, Props::new().set("statement_kind", kind))
            .unwrap();
    }
    let spec = |id: &str, premises: &[&str], assumed: &[&str], conclusion: &str| {
        reflow2_core::inference::InferenceSpec {
            id: id.into(),
            rule: "all".into(),
            premises: premises.iter().map(|s| s.to_string()).collect(),
            assumed: assumed.iter().map(|s| s.to_string()).collect(),
            conclusion: conclusion.into(),
            ..Default::default()
        }
    };
    g.add_inference(&spec(
        "inf:case-a",
        &["req:a-implies-c"],
        &["req:a"],
        "req:c-under-a",
    ))
    .unwrap();
    g.add_inference(&spec(
        "inf:case-b",
        &["req:b-implies-c"],
        &["req:b"],
        "req:c-under-b",
    ))
    .unwrap();
    let mut split = spec(
        "inf:or-elim",
        &["req:c-under-a", "req:c-under-b"],
        &[],
        "req:c",
    );
    split.cases = Some(vec!["req:a".into(), "req:b".into()]);
    split.cases_exhaustive = Some("req:a-or-b".into());
    g.add_inference(&split).unwrap();
    let mut imp = spec("inf:imp-intro", &["req:c"], &[], "req:a-or-b-implies-c");
    imp.discharges = Some(vec!["req:a-or-b".into()]);
    g.add_inference(&imp)
        .expect("the disjunction is assumed beneath →I");

    let r = reading(&g, "inf:or-elim");
    assert_eq!(r["verdict"], "holds", "{r:#}");
    assert_eq!(r["cases"]["truth"], "holds", "{r:#}");
    assert_eq!(r["assumes"], json!(["req:a-or-b"]), "{r:#}");
    assert_eq!(r["holds_as"], "conditional", "{r:#}");
    let t = reading(&g, "inf:imp-intro");
    assert_eq!(t["verdict"], "holds", "{t:#}");
    assert!(t.get("assumes").is_none(), "{t:#}");
    assert!(t.get("holds_as").is_none(), "{t:#}");
}

/// ITEM 5: a line says what relation it establishes, and reads it back: on
/// the logic rung every truth-table step is `entailment` (⊨) and every
/// deduction `derivability` (⊢).
#[test]
fn a_line_says_whether_it_shows_entailment_or_derivability() {
    let mut d = doc(LOGIC);
    let ids: Vec<(String, bool)> = d["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["node_type"] == "Inference")
        .map(|n| {
            let name = n["properties"]["rule_name"].as_str().unwrap_or("");
            (
                n["node_id"].as_str().unwrap().to_string(),
                name.contains("truth table"),
            )
        })
        .collect();
    for (id, table) in &ids {
        set_prop(
            &mut d,
            id,
            "establishes",
            json!(if *table { "entailment" } else { "derivability" }),
        );
    }
    let g = load_json(&d);
    let mut tables = 0;
    for (id, table) in &ids {
        let r = reading(&g, id);
        assert_eq!(
            r["establishes"],
            if *table { "entailment" } else { "derivability" },
            "{id}: {r:#}"
        );
        tables += usize::from(*table);
    }
    assert!(tables >= 3, "{tables}");
}

/// ITEM 6, THE CLASS over every what-if on a rule of the logic rung: with
/// each ↔I and ∧I step marked as making its conclusion of its premises, every
/// row for such a step lists its parts, each reading as its premise now
/// reads; and dropping RAA leaves at least one biconditional with one
/// direction standing while the whole loses its support.
#[test]
fn a_what_if_says_which_part_of_a_conjunction_or_biconditional_stands() {
    let mut d = doc(LOGIC);
    let forms: Vec<(String, &str)> = d["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["node_type"] == "Inference")
        .filter_map(|n| {
            let name = n["properties"]["rule_name"].as_str().unwrap_or("");
            let form = if name.starts_with("↔I") {
                "biconditional"
            } else if name.starts_with("∧I") {
                "conjunction"
            } else {
                return None;
            };
            Some((n["node_id"].as_str().unwrap().to_string(), form))
        })
        .collect();
    assert!(forms.len() >= 5, "{forms:?}");
    for (id, form) in &forms {
        set_prop(&mut d, id, "conclusion_form", json!(form));
    }
    let g = load_json(&d);
    let formed: BTreeSet<&str> = forms.iter().map(|(s, _)| s.as_str()).collect();
    let mut partly = 0;
    for rule in [
        "con:rule-raa",
        "con:rule-not-intro",
        "con:rule-or-elim",
        "con:def-nand",
    ] {
        for row in js(g.inference_walk(&[rule]).unwrap()).as_array().unwrap() {
            let s = row["inference"].as_str().unwrap();
            if !formed.contains(s) {
                assert!(row.get("parts").is_none(), "{s}: {row:#}");
                continue;
            }
            let parts = row["parts"].as_array().expect("parts");
            let premises = row["premises"].as_array().unwrap();
            assert_eq!(parts.len(), premises.len(), "{s}: {row:#}");
            for (part, p) in parts.iter().zip(premises) {
                let expect = match p["truth"].as_str().unwrap() {
                    "holds" => "stands",
                    "fails" => "refuted",
                    _ => "unsupported",
                };
                assert_eq!(part["reads"], expect, "{s}: {row:#}");
            }
            if row["conclusion_after"] != "holds" && parts.iter().any(|p| p["reads"] == "stands") {
                partly += 1;
            }
        }
    }
    assert!(partly > 0, "no whole lost with a part standing");
}

/// ITEM 7, THE CLASS over every step of every fixture: a step that holds
/// while resting on an open assumption or an unverified citation reads
/// `holds_as: conditional` AT THE STEP, one resting only on evidence reads
/// `supported`, and one resting on neither carries no `holds_as`. On the
/// logic rung, the steps under the recalled citations read "holds" on main,
/// and only their claims read conditional.
#[test]
fn a_step_resting_on_an_unverified_citation_reads_conditional_at_the_step() {
    let mut conditional = 0;
    for (name, text) in ALL {
        let g = load(text);
        for s in steps(&g) {
            let r = reading(&g, &s);
            if r["verdict"] != "holds" {
                assert!(r.get("holds_as").is_none(), "{name} {s}: {r:#}");
                continue;
            }
            let expect = if r.get("assumes").is_some() || r.get("rests_on_unverified").is_some() {
                Some("conditional")
            } else if r.get("rests_on_tests").is_some() {
                Some("supported")
            } else {
                None
            };
            assert_eq!(r["holds_as"].as_str(), expect, "{name} {s}: {r:#}");
            if r.get("rests_on_unverified").is_some() {
                conditional += 1;
            }
        }
    }
    assert!(conditional > 0, "no step rests on an unverified citation");
}

/// ITEM 7, THE CLASS over every stated independence in every fixture: two
/// lines stated independent that both rest on one cited result (or one piece
/// of evidence) no `independent_except` names are NOT independent, and the
/// reading names it; one that holds shares none. On the logic rung, the
/// truth-table and deduction lines of each law both rest on the cited
/// soundness and completeness of natural deduction, and passed on main.
#[test]
fn two_lines_through_one_cited_result_are_not_independent() {
    let mut shared = 0;
    for (name, text) in ALL {
        let g = load(text);
        for s in steps(&g) {
            for i in g.independence_of(&s).unwrap() {
                let i = js(i);
                let mine = closure(&g, &s);
                let theirs = closure(&g, i["of"].as_str().unwrap());
                let except: BTreeSet<String> = i["except"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect();
                let cited: Vec<String> = mine
                    .intersection(&theirs)
                    .filter(|f| !except.contains(*f))
                    .filter(|f| {
                        node_json(&g, f).is_some_and(|p| {
                            let kind = p["statement_kind"].as_str().unwrap_or("");
                            (matches!(kind, "theorem" | "lemma" | "corollary")
                                && p["taken_as_given"] == true)
                                || kind == "evidence"
                        })
                    })
                    .cloned()
                    .collect();
                if !cited.is_empty() {
                    shared += 1;
                    assert_eq!(i["holds"], false, "{name} {s}: {i:#}");
                    assert_eq!(i["shared_citations"], json!(cited), "{name} {s}: {i:#}");
                }
            }
        }
    }
    assert!(
        shared > 0,
        "no stated independence shares a cited foundation"
    );
}
