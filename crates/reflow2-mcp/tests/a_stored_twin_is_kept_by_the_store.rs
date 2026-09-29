//! The tool surface keeps a relation stored twice in step
//! (`req:a-relation-stored-in-more-than-one-place-has-one-authoritative-copy-and-no-copy-drifts-unnoticed`).
//!
//! The core tests (`a_stored_twin_has_one_authority.rs`) pin the store. These
//! pin the doors a person or agent actually uses:
//! - the generic `create_node` a skill reaches for gets the derived edge;
//! - the generic edge tools refuse a copy drawn against its property, and say
//!   what to do instead;
//! - `create_edges` refuses the whole batch, and writes nothing.
//!
//! OBSERVED FAILING, 2026-09-29, all four, with the store's keep and the
//! tools' refusal switched off: the generic writer left the finding edgeless,
//! and each hand-drawn or hand-deleted copy was accepted.

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::json;

async fn setup() -> ReflowService {
    let s = ReflowService::in_memory().expect("service");
    for (id, name) in [("cmp:a", "A"), ("cmp:b", "B")] {
        s.add_component(Parameters(
            serde_json::from_value(json!({
                "id": id, "name": name, "description": "a part"
            }))
            .unwrap(),
        ))
        .await
        .expect("component");
    }
    s.create_node(Parameters(
        serde_json::from_value(json!({
            "node_type": "TemporalFact",
            "id": "fact:x",
            "props": {"subject_id": "cmp:a", "statement": "measured something", "basis": "measured"}
        }))
        .unwrap(),
    ))
    .await
    .expect("a finding through the generic writer");
    s
}

async fn edges(s: &ReflowService, edge_type: &str) -> Vec<(String, String)> {
    let out = s
        .export_graph(Parameters(serde_json::from_value(json!({})).unwrap()))
        .await
        .expect("export")
        .structured_content
        .expect("a document");
    let mut v: Vec<(String, String)> = out["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .filter(|e| e["edge_type"] == edge_type)
        .map(|e| {
            (
                e["from_id"].as_str().unwrap().to_string(),
                e["to_id"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    v.sort();
    v
}

fn refusal_text(err: &rmcp::ErrorData) -> String {
    err.message.to_string()
}

/// THE MEASURED DEFECT at the door skills use: the generic `create_node`
/// set `subject_id` and drew no edge.
#[tokio::test]
async fn a_finding_made_with_the_generic_create_node_hangs_from_its_subject() {
    let s = setup().await;
    assert_eq!(
        edges(&s, "HAS_TEMPORAL_FACT").await,
        [("cmp:a".to_string(), "fact:x".to_string())]
    );
}

#[tokio::test]
async fn a_second_subject_drawn_by_hand_is_refused_naming_about_entity() {
    let s = setup().await;
    let err = s
        .create_edge(Parameters(
            serde_json::from_value(json!({
                "edge_type": "HAS_TEMPORAL_FACT", "from_id": "cmp:b", "to_id": "fact:x"
            }))
            .unwrap(),
        ))
        .await
        .expect_err("HAS_TEMPORAL_FACT from a node that is not the subject is refused");
    let text = refusal_text(&err);
    assert!(
        text.contains("subject_id") && text.contains("ABOUT_ENTITY"),
        "{text}"
    );

    // What the refusal names works.
    s.create_edge(Parameters(
        serde_json::from_value(json!({
            "edge_type": "ABOUT_ENTITY", "from_id": "fact:x", "to_id": "cmp:b"
        }))
        .unwrap(),
    ))
    .await
    .expect("ABOUT_ENTITY says the finding also concerns cmp:b");
    assert_eq!(
        edges(&s, "HAS_TEMPORAL_FACT").await,
        [("cmp:a".to_string(), "fact:x".to_string())]
    );
}

#[tokio::test]
async fn a_batch_carrying_one_disagreeing_copy_writes_nothing() {
    let s = setup().await;
    let err = s
        .create_edges(Parameters(
            serde_json::from_value(json!({
                "edges": [
                    {"edge_type": "DEPENDS_ON", "from_id": "cmp:a", "to_id": "cmp:b"},
                    {"edge_type": "HAS_TEMPORAL_FACT", "from_id": "cmp:b", "to_id": "fact:x"}
                ]
            }))
            .unwrap(),
        ))
        .await
        .expect_err("the batch is refused");
    assert!(
        refusal_text(&err).contains("edges[1]"),
        "{}",
        refusal_text(&err)
    );
    assert!(
        edges(&s, "DEPENDS_ON").await.is_empty(),
        "all or nothing: the valid item was not written either"
    );
}

#[tokio::test]
async fn deleting_the_copy_its_property_still_names_is_refused() {
    let s = setup().await;
    let err = s
        .delete_edge(Parameters(
            serde_json::from_value(json!({
                "edge_type": "HAS_TEMPORAL_FACT", "from_id": "cmp:a", "to_id": "fact:x"
            }))
            .unwrap(),
        ))
        .await
        .expect_err("the subject's copy cannot be deleted out from under subject_id");
    assert!(
        refusal_text(&err).contains("change subject_id"),
        "{}",
        refusal_text(&err)
    );
    assert_eq!(
        edges(&s, "HAS_TEMPORAL_FACT").await,
        [("cmp:a".to_string(), "fact:x".to_string())]
    );
}
