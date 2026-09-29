//! Every constructor that draws edges names each one in its reply, as a
//! sentence with its subject first — and names nothing else.
//!
//! # The finding this pins
//!
//! `fact:root-cause-add-decision-discards-the-edge-echo-its-related-to-computed-2026-09-28`,
//! sighted a third time on 2026-09-29
//! (`art:dev-reflow2-two-agent-exercise-feedback-2026-09-29`, item I16): a
//! designer agent recorded seven `choice` decisions with `related_to`, every
//! one landed with NO edges, and the reply said nothing. `add_decision` drew
//! `related_to` only for `kind: exploratory`, and even then discarded the core
//! review's outcome, so no kind named what it drew. `add_capability` built a
//! `drawn` list and discarded it too.
//!
//! # The class, not the instance
//!
//! Each constructor hand-built its reply, with no shared "edges this call
//! drew" block and no stated invariant. So this suite asserts the invariant
//! over EVERY constructor that takes relation targets, by diffing the edge set
//! before and after the call: the reply's `edges_drawn` is exactly the new
//! edges, as `from RELATION to`. A new constructor that draws edges and forgets
//! to say so fails here, not in a field report.

use std::collections::BTreeSet;

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

macro_rules! j {
    ($call:expr) => {
        $call
            .await
            .expect("tool ok")
            .structured_content
            .expect("structured content present")
    };
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("reflow2-edges-drawn-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// The design's whole edge set, as subject-first sentences, read through the
/// export — the one reader no constructor controls.
async fn edges(s: &ReflowService, dir: &std::path::Path) -> BTreeSet<String> {
    let f = dir.join("design.json");
    j!(s.export_graph(Parameters(ExportGraphToReq {
        path: Some(f.display().to_string()),
        overwrite: Some(true),
        accept_divergence: None,
    })));
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(&f).expect("export read"))
        .expect("export parses");
    doc["edges"]
        .as_array()
        .expect("edges array")
        .iter()
        .map(|e| {
            format!(
                "{} {} {}",
                e["from_id"].as_str().unwrap(),
                e["edge_type"].as_str().unwrap(),
                e["to_id"].as_str().unwrap()
            )
        })
        .collect()
}

fn named(reply: &Value, key: &str) -> BTreeSet<String> {
    reply[key]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|v| v.as_str().expect("a sentence").to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// THE INVARIANT: the reply names exactly the edges the call created — no
/// fewer (the silent drop) and no more (a claim about an edge that already
/// existed). `ignore` excludes edge types this suite is not about (the
/// approver signature).
fn assert_names_exactly_what_it_drew(
    before: &BTreeSet<String>,
    after: &BTreeSet<String>,
    reply: &Value,
    what: &str,
) {
    let new: BTreeSet<String> = after.difference(before).cloned().collect();
    let said = named(reply, "edges_drawn");
    assert!(
        !new.is_empty(),
        "{what}: the call was offered targets and drew no edge at all — the silent drop. \
         reply: {reply}"
    );
    assert_eq!(
        said, new,
        "{what}: the reply must name every edge it drew, subject first, and nothing else. \
         reply: {reply}"
    );
}

async fn svc() -> ReflowService {
    let s = ReflowService::in_memory().expect("in-memory service");
    for (id, name, text) in [
        (
            "dec:older-thought",
            "Should the pump run on a timer?",
            "Timers are simple and waste water on rainy days.",
        ),
        (
            "dec:prereq",
            "Which moisture sensor to buy",
            "Capacitive probes last longer in wet soil than resistive ones.",
        ),
    ] {
        j!(s.add_decision(Parameters(
            serde_json::from_value(json!({
                "id": id, "name": name,
                "decision": text,
                "kind": "exploratory",
                "no_relation_note": "fixture; nothing to relate"
            }))
            .unwrap()
        )));
    }
    s
}

fn decision_with_relations(id: &str, kind: Option<&str>) -> Value {
    let mut v = json!({
        "id": id,
        "name": format!("A {} decision that relates to two others", kind.unwrap_or("kindless")),
        "decision": "It relates to two records: one it grew out of, one it needs first.",
        "related_to": [
            {"relation": "EVOLVES_INTO", "other_id": "dec:older-thought", "incoming": true,
             "evidence": "the older thought grew into this"},
            {"relation": "DEPENDS_ON", "other_id": "dec:prereq",
             "evidence": "only worth anything once the prerequisite lands"}
        ]
    });
    if let Some(k) = kind {
        v["kind"] = json!(k);
    }
    v
}

#[tokio::test]
async fn add_decision_names_its_relations_for_every_kind() {
    for kind in [Some("choice"), None, Some("exploratory")] {
        let s = svc().await;
        let dir = scratch(&format!("dec-{}", kind.unwrap_or("none")));
        let before = edges(&s, &dir).await;
        let reply = j!(s.add_decision(Parameters(
            serde_json::from_value(decision_with_relations("dec:new", kind)).unwrap()
        )));
        let after = edges(&s, &dir).await;
        assert_names_exactly_what_it_drew(
            &before,
            &after,
            &reply,
            &format!("add_decision kind={kind:?}"),
        );
        // The direction flag is honoured AND visible: the incoming edge reads
        // with the other node as its subject.
        assert!(
            named(&reply, "edges_drawn").contains("dec:older-thought EVOLVES_INTO dec:new"),
            "kind={kind:?}: the incoming edge must read subject-first: {reply}"
        );
    }
}

#[tokio::test]
async fn add_capability_names_the_thread_it_drew_and_a_resend_draws_nothing_new() {
    let s = svc().await;
    let dir = scratch("cap");
    j!(s.add_requirement(Parameters(
        serde_json::from_value(json!({
            "id": "req:a-need", "name": "A need", "statement": "Something must hold."
        }))
        .unwrap()
    )));
    j!(s.add_component(Parameters(
        serde_json::from_value(json!({
            "id": "cmp:a-part", "name": "A part", "description": "It holds the thing."
        }))
        .unwrap()
    )));
    let before = edges(&s, &dir).await;
    let cap = json!({
        "id": "cap:a-function", "name": "A function",
        "description": "Does the thing the need asks for.",
        "satisfies": "req:a-need", "allocated_to": "cmp:a-part"
    });
    let reply = j!(s.add_capability(Parameters(serde_json::from_value(cap.clone()).unwrap())));
    let after = edges(&s, &dir).await;
    assert_names_exactly_what_it_drew(&before, &after, &reply, "add_capability");

    // A REVISE that re-sends the same targets draws nothing and must not read
    // as having drawn something: the edges are named as already present.
    let again = j!(s.add_capability(Parameters(serde_json::from_value(cap).unwrap())));
    assert_eq!(edges(&s, &dir).await, after, "a resend draws nothing new");
    assert!(
        named(&again, "edges_drawn").is_empty(),
        "a resend names no new edge: {again}"
    );
    assert_eq!(
        named(&again, "edges_already_present"),
        BTreeSet::from([
            "cap:a-function SATISFIES req:a-need".to_string(),
            "cap:a-function ALLOCATED_TO cmp:a-part".to_string(),
        ]),
        "a resend says the edges were already there: {again}"
    );
}

#[tokio::test]
async fn add_verification_add_change_event_and_record_finding_name_theirs_as_sentences() {
    let s = svc().await;
    let dir = scratch("assure");
    j!(s.add_requirement(Parameters(
        serde_json::from_value(json!({
            "id": "req:a-need", "name": "A need", "statement": "Something must hold."
        }))
        .unwrap()
    )));

    let before = edges(&s, &dir).await;
    let reply = j!(s.add_verification(Parameters(
        serde_json::from_value(json!({
            "id": "ver:a-check", "name": "A check", "method": "test",
            "verifies": [{"target_id": "req:a-need"}]
        }))
        .unwrap()
    )));
    let after = edges(&s, &dir).await;
    assert_names_exactly_what_it_drew(&before, &after, &reply, "add_verification");

    let before = after;
    let reply = j!(s.add_change_event(Parameters(
        serde_json::from_value(json!({
            "id": "chg:a-change", "name": "A change", "change_type": "defect_fix",
            "summary": "The need's statement was corrected.", "detected_at": "2026-09-29",
            "affected": [{"node_id": "req:a-need"}]
        }))
        .unwrap()
    )));
    let after = edges(&s, &dir).await;
    assert_names_exactly_what_it_drew(&before, &after, &reply, "add_change_event");

    let before = after;
    let reply = j!(s.record_finding(Parameters(
        serde_json::from_value(json!({
            "id": "fact:a-finding", "subject_id": "req:a-need",
            "name": "The need was measured", "statement": "Measured, and it holds.",
            "basis": "measured", "caused_by": "chg:a-change",
            "cause_evidence": "the change is what made it hold"
        }))
        .unwrap()
    )));
    let after = edges(&s, &dir).await;
    assert_names_exactly_what_it_drew(&before, &after, &reply, "record_finding");
}
