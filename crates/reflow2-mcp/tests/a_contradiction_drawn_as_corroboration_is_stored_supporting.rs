//! A review link can say what the relation's schema default would otherwise
//! assert for it — first of all, that a CONTRADICTS is a corroboration.
//!
//! `fact:root-cause-review-relations-cannot-mark-a-contradiction-supporting-and-the-default-says-opposing-2026-10-13`.
//! `review_relations` took relation, other_id, evidence, incoming, other_design
//! and other_name, refused anything else, and wrote only `evidence` on the edge.
//! `schema/inference.yaml` declares CONTRADICTS `alignment` with
//! `default: opposing`, and a default materialises on create. So a writer who
//! meant "these two corroborate each other" had no field to say it in, wrote it
//! into the evidence, and the stored edge asserted the opposite. Measured on
//! main c58305ba8: 10 CONTRADICTS edges store `opposing` while their own
//! evidence says "alignment=supporting", and `detect_defects` reported 9 of
//! them as contradictions.
//!
//! Each of these failed before the fields existed: the per-link argument
//! struct denies unknown fields, so `alignment` did not even deserialize.

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

macro_rules! call {
    ($s:expr, $tool:ident, $args:expr) => {
        j!($s.$tool(Parameters(serde_json::from_value($args).unwrap())))
    };
}

async fn three_requirements() -> ReflowService {
    let s = ReflowService::in_memory_as("corroboration").unwrap();
    call!(s, add_project, json!({"id": "proj:p", "name": "P"}));
    for (id, what) in [
        ("req:a", "The aardvark need"),
        ("req:b", "The bandicoot need"),
        ("req:c", "The capybara need"),
    ] {
        call!(
            s,
            add_requirement,
            json!({"id": id, "name": what, "statement": format!("{what}, stated plainly.")})
        );
    }
    s
}

/// The stored properties of the one `edge` from `from` to `to`, or Null.
async fn stored(s: &ReflowService, edge: &str, from: &str, to: &str) -> Value {
    let found = call!(
        s,
        get_node,
        json!({"id": from, "include_edges": {"direction": "out", "edge_types": [edge]}})
    );
    found["edges"]["items"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|e| e["to_id"] == json!(to))
        .map(|e| e["properties"].clone())
        .unwrap_or(Value::Null)
}

/// The review request, deserialized as the server would — which is the step
/// that refused `alignment` outright before this change.
fn review(args: Value) -> ReviewRelationsReq {
    serde_json::from_value(args.clone())
        .unwrap_or_else(|e| panic!("review_relations must accept this argument shape: {e}\n{args}"))
}

async fn contradictions(s: &ReflowService) -> Vec<Vec<String>> {
    let sweep = call!(s, detect_defects, json!({}));
    sweep["defects"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|d| d["category"] == json!("contradiction"))
        .map(|d| {
            let mut ids: Vec<String> = d["affected_ids"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|x| x.as_str().map(String::from))
                .collect();
            ids.sort();
            ids
        })
        .collect()
}

#[tokio::test]
async fn a_supporting_contradiction_drawn_through_review_relations_is_stored_supporting() {
    let s = three_requirements().await;
    let evidence = "Both say the same thing from different sides — corroboration, not conflict.";
    let out = j!(s.review_relations(Parameters(review(json!({
        "node_id": "req:a",
        "links": [{"relation": "CONTRADICTS", "other_id": "req:b",
                   "alignment": "supporting", "evidence": evidence}]
    })))));
    assert_eq!(out["drawn"], json!(["req:a CONTRADICTS req:b"]), "{out}");

    let props = stored(&s, "CONTRADICTS", "req:a", "req:b").await;
    assert_eq!(props["alignment"], json!("supporting"), "{props}");
    assert_eq!(
        props["evidence"],
        json!(evidence),
        "evidence is kept as given"
    );
    assert!(
        contradictions(&s).await.is_empty(),
        "a corroboration is not a contradiction defect"
    );
}

#[tokio::test]
async fn an_omitted_alignment_still_stores_opposing() {
    let s = three_requirements().await;
    let out = j!(s.review_relations(Parameters(review(json!({
        "node_id": "req:a",
        "links": [{"relation": "CONTRADICTS", "other_id": "req:c",
                   "evidence": "Both cannot hold: one forbids what the other needs."}]
    })))));
    assert_eq!(out["drawn"], json!(["req:a CONTRADICTS req:c"]), "{out}");

    let props = stored(&s, "CONTRADICTS", "req:a", "req:c").await;
    assert_eq!(
        props["alignment"],
        json!("opposing"),
        "absent means the schema default, exactly as before: {props}"
    );
    assert_eq!(
        contradictions(&s).await,
        vec![vec!["req:a".to_string(), "req:c".to_string()]],
        "an opposing CONTRADICTS is still reported"
    );
}

#[tokio::test]
async fn a_value_the_schema_does_not_allow_is_refused_naming_the_property_and_its_values() {
    let s = three_requirements().await;
    let err = s
        .review_relations(Parameters(review(json!({
            "node_id": "req:a",
            "links": [
                {"relation": "CONTRADICTS", "other_id": "req:c",
                 "evidence": "drawn first in the same call"},
                {"relation": "CONTRADICTS", "other_id": "req:b",
                 "alignment": "neutral", "evidence": "neither for nor against"}]
        }))))
        .await
        .expect_err("a value CONTRADICTS.alignment does not declare is refused");
    for word in ["alignment", "neutral", "opposing", "supporting"] {
        assert!(
            err.message.contains(word),
            "the refusal names `{word}`: {}",
            err.message
        );
    }
    // Refused before anything was written: the valid link ahead of the bad
    // one was not drawn either.
    assert!(stored(&s, "CONTRADICTS", "req:a", "req:c").await.is_null());
    assert!(stored(&s, "CONTRADICTS", "req:a", "req:b").await.is_null());

    // `basis` is a legal field name for two relations with two different
    // value sets. A DUPLICATES value on a CAUSES link is refused with the
    // CAUSES values.
    let err = s
        .review_relations(Parameters(review(json!({
            "node_id": "req:a",
            "links": [{"relation": "CAUSES", "other_id": "req:b",
                       "basis": "asserted", "evidence": "a"}]
        }))))
        .await
        .expect_err("a DUPLICATES basis is not a CAUSES basis");
    for word in [
        "basis",
        "asserted",
        "observed",
        "correlational",
        "causal",
        "spurious",
    ] {
        assert!(
            err.message.contains(word),
            "the refusal names `{word}`: {}",
            err.message
        );
    }
    assert!(stored(&s, "CAUSES", "req:a", "req:b").await.is_null());
}

#[tokio::test]
async fn a_property_foreign_to_the_relation_is_refused() {
    let s = three_requirements().await;
    let err = s
        .review_relations(Parameters(review(json!({
            "node_id": "req:a",
            "links": [{"relation": "CAUSES", "other_id": "req:b",
                       "alignment": "supporting", "evidence": "a brings b about"}]
        }))))
        .await
        .expect_err("CAUSES declares no alignment");
    for word in ["alignment", "CAUSES", "basis", "validation_status"] {
        assert!(
            err.message.contains(word),
            "the refusal names `{word}`, and what CAUSES does take: {}",
            err.message
        );
    }
    assert!(stored(&s, "CAUSES", "req:a", "req:b").await.is_null());

    let err = s
        .review_relations(Parameters(review(json!({
            "node_id": "req:a",
            "links": [{"relation": "BLOCKS", "other_id": "req:b",
                       "optional": true, "evidence": "a stalls b"}]
        }))))
        .await
        .expect_err("BLOCKS declares no optional");
    assert!(
        err.message.contains("optional") && err.message.contains("BLOCKS"),
        "{}",
        err.message
    );
    assert!(stored(&s, "BLOCKS", "req:a", "req:b").await.is_null());
}

/// A value given for an edge that is already stored is never dropped: the same
/// value reads as already present, a different one is refused and the stored
/// edge is left as it was. Reporting it `already_present` would tell the
/// writer their corroboration was recorded while the edge still says opposing.
#[tokio::test]
async fn a_value_for_an_edge_already_stored_otherwise_is_refused_not_dropped() {
    let s = three_requirements().await;
    let evidence = "Both cannot hold.";
    j!(s.review_relations(Parameters(review(json!({
        "node_id": "req:a",
        "links": [{"relation": "CONTRADICTS", "other_id": "req:b", "evidence": evidence}]
    })))));

    let err = s
        .review_relations(Parameters(review(json!({
            "node_id": "req:a",
            "links": [{"relation": "CONTRADICTS", "other_id": "req:b",
                       "alignment": "supporting", "evidence": evidence}]
        }))))
        .await
        .expect_err("a different value for a stored edge is not silently dropped");
    for word in ["alignment", "opposing", "supporting", "create_edge"] {
        assert!(
            err.message.contains(word),
            "the refusal names `{word}`: {}",
            err.message
        );
    }
    assert_eq!(
        stored(&s, "CONTRADICTS", "req:a", "req:b").await["alignment"],
        json!("opposing")
    );

    let again = j!(s.review_relations(Parameters(review(json!({
        "node_id": "req:a",
        "links": [{"relation": "CONTRADICTS", "other_id": "req:b",
                   "alignment": "opposing", "evidence": evidence}]
    })))));
    assert_eq!(
        again["already_present"],
        json!(["req:a CONTRADICTS req:b"]),
        "{again}"
    );
}

/// `add_decision`'s `related_to` takes the same link shape and draws through
/// the same core call, so it carries the same fields.
#[tokio::test]
async fn a_decision_captured_with_a_supporting_relation_stores_it_supporting() {
    let s = three_requirements().await;
    let args: DecisionReq = serde_json::from_value(json!({
        "id": "dec:idea-x", "name": "OPEN — an idea", "decision": "An idea.",
        "kind": "exploratory",
        "related_to": [{"relation": "CONTRADICTS", "other_id": "req:a",
                        "alignment": "supporting",
                        "evidence": "the idea is the requirement, argued again"}]
    }))
    .unwrap_or_else(|e| panic!("add_decision's related_to takes the link fields: {e}"));
    j!(s.add_decision(Parameters(args)));
    assert_eq!(
        stored(&s, "CONTRADICTS", "dec:idea-x", "req:a").await["alignment"],
        json!("supporting")
    );
}
