//! A near-match ACROSS TYPES, or between two records of a LINE OF REASONING,
//! is a note on a successful write, never a refusal.
//!
//! `req:a-near-match-across-types-or-within-a-line-of-reasoning-is-a-note-not-a-refusal`,
//! settled by `dec:idea-a-cross-type-near-match-is-a-note-and-only-a-same-type-one-refuses`
//! (Anthony, 2026-10-17: "Note, never refuse", for both).
//!
//! # The measured case
//!
//! Proof trial rung 2 on flo2.io (design bbcd344a71ff1cbd, the fundamental
//! theorem of arithmetic, on 0.86.0;
//! `fact:the-proof-trial-rung-2-ran-on-flo2-io-and-what-counts-holds-while-recording-a-proof-is-still-costly-2026-10-17`):
//! about 19 creates were refused, often across types (a statement against an
//! Inference, the Project, a definition), and one statement three times
//! against three different nodes. 42 of its 49 statements carry a
//! `distinct_from` list, the longest 18 ids. Every one of those ids names a
//! record of another type, or another statement carrying a `statement_kind`.
//!
//! # What these tests pin: the RULE, as a class
//!
//! - every cross-type near-match, for every constructor that checks: written,
//!   with the match named in `search_first.near_matches` with its type;
//! - a same-type match between two statements of a line of reasoning: written,
//!   with the same report;
//! - every other same-type match: refused, with the three answers;
//! - and the real case: every statement of the trial, re-recorded against the
//!   rest of its design with no `distinct_from`, is written.
//!
//! The hand-kept list of exempt pairs (`PRESCRIBED_LAYER_PAIRS`) is gone; a
//! list could not be pinned as a class, and it grew five times for field
//! reports before this rule replaced it.

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use serde_json::{Value, json};

/// One idea in one vocabulary, long enough for the check to judge (it declines
/// below twelve words), used verbatim on both sides so the words really match.
const SAID: &str = "The read-back keeps the words of every claim it returns and pages through \
                    the rest of the proof in order instead of blanking the prose.";

async fn svc() -> ReflowService {
    ReflowService::in_memory().expect("in-memory service")
}

fn structured(r: CallToolResult) -> Value {
    r.structured_content.expect("a structured reply")
}

/// The prose field each type's text lives in, as the constructors write it.
fn prose_field(node_type: &str) -> &'static str {
    match node_type {
        "Capability" | "Component" | "Project" => "description",
        "Decision" => "decision",
        "ChangeEvent" => "summary",
        "Inference" => "warrant",
        _ => "statement",
    }
}

/// Seed one node of `node_type` saying [`SAID`], through the generic door.
async fn seed(s: &ReflowService, node_type: &str, id: &str, extra: Value) {
    let mut props = json!({ "name": format!("Seeded {node_type}") });
    props[prose_field(node_type)] = json!(SAID);
    if let (Some(p), Some(e)) = (props.as_object_mut(), extra.as_object()) {
        p.extend(e.clone());
    }
    let req: CreateNodeReq = serde_json::from_value(json!({
        "node_type": node_type, "id": id, "props": props,
    }))
    .expect("a create_node request");
    s.create_node(Parameters(req))
        .await
        .unwrap_or_else(|e| panic!("seeding {node_type} {id}: {e:?}"));
}

/// Create one node of a CHECKING constructor's type saying [`SAID`].
async fn create(
    s: &ReflowService,
    node_type: &str,
    id: &str,
    extra: Value,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let mut args = json!({ "id": id, "name": format!("New {node_type}") });
    args[prose_field(node_type)] = json!(SAID);
    if let (Some(a), Some(e)) = (args.as_object_mut(), extra.as_object()) {
        a.extend(e.clone());
    }
    match node_type {
        "Requirement" => {
            s.add_requirement(Parameters(serde_json::from_value(args).unwrap()))
                .await
        }
        "DesignRule" => {
            s.add_design_rule(Parameters(serde_json::from_value(args).unwrap()))
                .await
        }
        "Capability" => {
            s.add_capability(Parameters(serde_json::from_value(args).unwrap()))
                .await
        }
        "Component" => {
            s.add_component(Parameters(serde_json::from_value(args).unwrap()))
                .await
        }
        "Decision" => {
            s.add_decision(Parameters(serde_json::from_value(args).unwrap()))
                .await
        }
        other => panic!("{other} is not a checking constructor"),
    }
}

/// The five constructors that check for a near-match before they write.
const CHECKING: [&str; 5] = [
    "Requirement",
    "DesignRule",
    "Capability",
    "Component",
    "Decision",
];

/// What each type needs beyond a name and its prose, to be seeded at all.
fn seed_extra(node_type: &str) -> Value {
    match node_type {
        "Decision" => json!({ "kind": "choice", "status": "proposed" }),
        "ChangeEvent" => json!({ "change_type": "new_feature" }),
        "TemporalFact" => {
            json!({ "fact_type": "finding", "valid_from": "2026-10-17", "subject_id": "proj:x" })
        }
        "Inference" => json!({ "rule": "all" }),
        "Component" => json!({ "purpose": SAID }),
        _ => json!({}),
    }
}

/// Every type a near-match can name: the five checking ones, and the records
/// they most often met in the field (a change, a dated finding, a premise, a
/// step, the project itself).
const SEEDS: [&str; 10] = [
    "Requirement",
    "DesignRule",
    "Capability",
    "Component",
    "Decision",
    "ChangeEvent",
    "TemporalFact",
    "Constraint",
    "Inference",
    "Project",
];

fn near_match_types(reply: &Value) -> Vec<(String, String)> {
    reply["search_first"]["near_matches"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|m| {
                    (
                        m["node_id"].as_str().unwrap_or_default().to_string(),
                        m["node_type"].as_str().unwrap_or_default().to_string(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// ⭐ THE CLASS: every cross-type pair, for every checking constructor, is
/// written, and the match is reported by id and type.
#[tokio::test]
async fn every_cross_type_near_match_is_a_note_on_a_successful_write() {
    let mut refused = Vec::new();
    let mut unreported = Vec::new();
    for creating in CHECKING {
        for seeded in SEEDS.iter().filter(|t| **t != creating) {
            let s = svc().await;
            let seed_id = format!("seed:{}", seeded.to_lowercase());
            if *seeded == "TemporalFact" {
                // A dated finding is about something: a subject that says
                // nothing near the words under test.
                let subject: CreateNodeReq = serde_json::from_value(json!({
                    "node_type": "Project", "id": "proj:x", "props": { "name": "X" },
                }))
                .unwrap();
                s.create_node(Parameters(subject)).await.expect("a subject");
            }
            seed(&s, seeded, &seed_id, seed_extra(seeded)).await;
            let new_id = format!("new:{}", creating.to_lowercase());
            let extra = if creating == "Decision" {
                json!({ "kind": "choice" })
            } else {
                json!({})
            };
            match create(&s, creating, &new_id, extra).await {
                Err(e) => refused.push(format!("{creating} against {seeded}: {}", e.message)),
                Ok(r) => {
                    let found = near_match_types(&structured(r));
                    if !found.contains(&(seed_id.clone(), seeded.to_string())) {
                        unreported.push(format!("{creating} against {seeded}: {found:?}"));
                    }
                }
            }
        }
    }
    assert!(
        refused.is_empty(),
        "a cross-type near-match must never refuse; {} did:\n{}",
        refused.len(),
        refused.join("\n")
    );
    assert!(
        unreported.is_empty(),
        "the written node must still REPORT the match, naming it and its type; {} did not:\n{}",
        unreported.len(),
        unreported.join("\n")
    );
}

/// Two statements of one line of reasoning (each carrying a `statement_kind`)
/// are written beside each other, and the match is reported.
#[tokio::test]
async fn a_same_type_near_match_between_two_statements_of_a_proof_is_a_note() {
    let s = svc().await;
    create(
        &s,
        "Requirement",
        "req:step-one",
        json!({ "statement_kind": "proposition" }),
    )
    .await
    .expect("the first statement lands");
    let r = create(
        &s,
        "Requirement",
        "req:step-two",
        json!({ "statement_kind": "lemma" }),
    )
    .await
    .expect("a near-match between two statements of a proof is a note, never a refusal");
    let found = near_match_types(&structured(r));
    assert!(
        found.contains(&("req:step-one".into(), "Requirement".into())),
        "the match is still reported: {found:?}"
    );
}

/// 🛑 THE COUNTERWEIGHT: a same-type match outside a line of reasoning still
/// refuses with the three answers, and so does one where only ONE side is a
/// statement of a proof.
#[tokio::test]
async fn a_same_type_near_match_outside_a_line_of_reasoning_still_refuses() {
    for (first, second) in [
        (json!({}), json!({})),
        (json!({ "statement_kind": "lemma" }), json!({})),
        (json!({}), json!({ "statement_kind": "lemma" })),
    ] {
        let s = svc().await;
        create(&s, "Requirement", "req:first", first.clone())
            .await
            .expect("the first requirement lands");
        let err = create(&s, "Requirement", "req:second", second.clone())
            .await
            .expect_err("a same-type near duplicate outside a proof is still refused");
        let msg = err.message.to_string();
        for route in ["SHARPEN", "distinct_from", "replaces"] {
            assert!(
                msg.contains(route),
                "the refusal names all three answers ({route} missing, {first} then {second}): \
                 {msg}"
            );
        }
        assert!(msg.contains("req:first"), "it names the match: {msg}");
    }
    // The same holds for the other checking constructors.
    for creating in ["DesignRule", "Capability", "Component", "Decision"] {
        let s = svc().await;
        let extra = if creating == "Decision" {
            json!({ "kind": "choice" })
        } else {
            json!({})
        };
        create(&s, creating, "x:first", extra.clone())
            .await
            .expect("the first lands");
        create(&s, creating, "x:second", extra)
            .await
            .expect_err("a same-type near duplicate is still refused");
    }
}

/// The trial design, exported read-only from flo2.io on 2026-10-17.
const TRIAL: &str = include_str!("fixtures/proof_trial_rung_2_fta.json");

/// ⭐ THE MEASURED CASE: each of the trial's 49 statements, re-recorded with
/// `add_requirement` against the rest of its design and with NO
/// `distinct_from`, is written.
#[tokio::test]
async fn every_statement_of_the_proof_trial_is_written_with_no_distinct_from() {
    let doc: Value = serde_json::from_str(TRIAL).expect("the fixture parses");
    let statements: Vec<Value> = doc["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["node_type"] == "Requirement")
        .cloned()
        .collect();
    assert_eq!(statements.len(), 49, "the trial records 49 statements");
    let mut refused = Vec::new();
    let mut reported = 0usize;
    for st in &statements {
        let id = st["node_id"].as_str().unwrap();
        // The design without this one statement, and without its edges.
        let mut rest = doc.clone();
        rest["nodes"]
            .as_array_mut()
            .unwrap()
            .retain(|n| n["node_id"] != id);
        rest["edges"]
            .as_array_mut()
            .unwrap()
            .retain(|e| e["from_id"] != id && e["to_id"] != id);
        let s = svc().await;
        s.import_graph(Parameters(ImportGraphReq {
            document: rest.as_object().cloned(),
            path: None,
            accept_newer: None,
        }))
        .await
        .expect("the trial imports");
        let p = &st["properties"];
        let req: RequirementReq = serde_json::from_value(json!({
            "id": id,
            "name": p["name"],
            "statement": p["statement"],
            "statement_kind": p["statement_kind"],
        }))
        .unwrap();
        match s.add_requirement(Parameters(req)).await {
            Err(e) => refused.push(format!("{id}: {}", e.message.lines().next().unwrap_or(""))),
            Ok(r) => {
                if !near_match_types(&structured(r)).is_empty() {
                    reported += 1;
                }
            }
        }
    }
    assert!(
        refused.is_empty(),
        "recording the trial's proof must need no distinct_from; {} of 49 statements were \
         refused:\n{}",
        refused.len(),
        refused.join("\n")
    );
    // Not a silent miss: the matches are still reported where they exist.
    assert!(
        reported > 0,
        "the near-match report must survive on the successful writes"
    );
    eprintln!("proof trial: 49 of 49 written with no distinct_from; {reported} carried a report");
}
