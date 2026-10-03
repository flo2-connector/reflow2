//! `get_node` reads a node's EDGES when asked (`include_edges`), and leaves its
//! reply exactly as it was when not
//! (`dec:idea-an-edge-reader-returns-one-nodes-edges-and-find-tools-finds-it`,
//! accepted by Anthony on 2026-10-02).
//!
//! THE CLASS: no served read returned one node's edges as edges
//! (`fact:root-cause-no-served-read-returns-one-nodes-edges-and-the-nearest-is-an-impact-walk-2026-10-02`).
//! Confirmed on main at c4e1cbd before this change: `--call get_node` with
//! `include_edges` was refused as an unknown field, and `propagate_from` at
//! depth 1 on `req:a-lesson-is-served-at-the-step-it-concerns` listed 6 of its
//! 7 stored edges (no `AUTHORED_BY`), with impact directions and no evidence.
//!
//! OBSERVED FAILING on that code, 2026-10-03: every test here that passes
//! `include_edges` failed with `unknown field \`include_edges\``.
//! The core half — directions, filters, twins, the bound, the dominant type,
//! the empty answers — is pinned in
//! `crates/reflow2-core/tests/one_nodes_edges_are_read_back_as_edges.rs`.
//! What is pinned HERE is the served door: the parameter's two shapes, the
//! refusals, the unchanged default reply, and the absent node.

use reflow2_mcp::service::ReflowService;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

async fn get(s: &ReflowService, args: Value) -> Result<Value, String> {
    let req = serde_json::from_value(args).map_err(|e| e.to_string())?;
    s.get_node(Parameters(req))
        .await
        .map(|r| r.structured_content.expect("structured content"))
        .map_err(|e| format!("{e:?}"))
}

async fn node(s: &ReflowService, ty: &str, id: &str, props: Value) {
    s.create_node(Parameters(
        serde_json::from_value(json!({"node_type": ty, "id": id, "props": props})).unwrap(),
    ))
    .await
    .expect("create_node");
}

async fn edge(s: &ReflowService, ty: &str, from: &str, to: &str, props: Value) {
    s.create_edge(Parameters(
        serde_json::from_value(
            json!({"edge_type": ty, "from_id": from, "to_id": to, "props": props}),
        )
        .unwrap(),
    ))
    .await
    .expect("create_edge");
}

/// A requirement with an incoming SATISFIES (with evidence), an outgoing
/// AUTHORED_BY, and a finding about it (the HAS_TEMPORAL_FACT twin).
async fn design() -> ReflowService {
    let s = ReflowService::in_memory().expect("service");
    node(
        &s,
        "Requirement",
        "req:r",
        json!({"name": "Read one node's edges", "statement": "A node's edges can be read."}),
    )
    .await;
    node(
        &s,
        "Capability",
        "cap:c",
        json!({"name": "An edge reader", "description": "Reads a node's edges."}),
    )
    .await;
    node(&s, "Contributor", "who:a", json!({"name": "A. Person"})).await;
    edge(
        &s,
        "SATISFIES",
        "cap:c",
        "req:r",
        json!({"evidence": "the reader returns every edge"}),
    )
    .await;
    edge(
        &s,
        "AUTHORED_BY",
        "req:r",
        "who:a",
        json!({"roles": ["author"]}),
    )
    .await;
    node(
        &s,
        "TemporalFact",
        "fact:f",
        json!({"name": "A finding about req:r", "subject_id": "req:r", "statement": "measured"}),
    )
    .await;
    s
}

fn edge_of<'a>(reply: &'a Value, edge_type: &str) -> &'a Value {
    reply["edges"]["items"]
        .as_array()
        .expect("edges.items")
        .iter()
        .find(|e| e["edge_type"] == edge_type)
        .unwrap_or_else(|| panic!("no {edge_type} edge in {reply:#}"))
}

#[tokio::test]
async fn without_include_edges_the_reply_is_exactly_what_it_was() {
    // Regression cover for every consumer that reads `{node}`, flo2's gateway
    // among them: the most-called read must not change shape unasked.
    let s = design().await;
    let plain = get(&s, json!({"id": "req:r"})).await.expect("get_node");
    let keys: Vec<&String> = plain.as_object().expect("object").keys().collect();
    assert!(
        !keys.iter().any(|k| *k == "edges"),
        "no edges block unless asked: {plain:#}"
    );
    let off = get(&s, json!({"id": "req:r", "include_edges": false}))
        .await
        .expect("get_node");
    assert_eq!(off, plain, "`false` is the same as leaving it out");
}

#[tokio::test]
async fn include_edges_true_returns_every_edge_with_its_other_end_and_evidence() {
    let s = design().await;
    let got = get(&s, json!({"id": "req:r", "include_edges": true}))
        .await
        .expect("get_node with include_edges");
    assert_eq!(
        got["node"]["node_id"], "req:r",
        "the node is still there: {got:#}"
    );

    let satisfies = edge_of(&got, "SATISFIES");
    assert_eq!(satisfies["direction"], "in");
    assert_eq!(satisfies["from_id"], "cap:c");
    assert_eq!(satisfies["to_id"], "req:r");
    assert_eq!(satisfies["other"]["node_type"], "Capability");
    assert_eq!(satisfies["other"]["name"], "An edge reader");
    assert_eq!(
        satisfies["properties"]["evidence"],
        "the reader returns every edge"
    );

    let authored = edge_of(&got, "AUTHORED_BY");
    assert_eq!(authored["direction"], "out");
    assert_eq!(authored["other"]["node_id"], "who:a");

    let twin = edge_of(&got, "HAS_TEMPORAL_FACT");
    assert_eq!(twin["twin_of"], "TemporalFact.subject_id");

    let edges = &got["edges"];
    let total = edges["total"].as_u64().expect("total");
    assert_eq!(
        edges["returned"].as_u64(),
        Some(total),
        "everything fits, so everything is returned: {edges:#}"
    );
    assert_eq!(edges["capped_by"], Value::Null);
    let counted: u64 = edges["by_type"]
        .as_object()
        .expect("by_type")
        .values()
        .map(|c| c["out"].as_u64().unwrap_or(0) + c["in"].as_u64().unwrap_or(0))
        .sum();
    assert_eq!(counted, total, "by_type counts every edge: {edges:#}");
}

#[tokio::test]
async fn a_filter_object_narrows_and_bounds_the_list() {
    let s = design().await;
    let got = get(
        &s,
        json!({"id": "req:r", "include_edges": {"direction": "in"}}),
    )
    .await
    .expect("direction filter");
    let items = got["edges"]["items"].as_array().expect("items");
    assert!(!items.is_empty());
    assert!(items.iter().all(|e| e["direction"] == "in"), "{got:#}");

    let got = get(
        &s,
        json!({"id": "req:r", "include_edges": {"edge_types": ["AUTHORED_BY"]}}),
    )
    .await
    .expect("edge_types filter");
    let items = got["edges"]["items"].as_array().expect("items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["edge_type"], "AUTHORED_BY");

    let got = get(&s, json!({"id": "req:r", "include_edges": {"limit": 1}}))
        .await
        .expect("limit");
    assert_eq!(got["edges"]["returned"], 1);
    assert_eq!(got["edges"]["capped_by"], "limit");
    assert_eq!(got["edges"]["next_offset"], 1);

    let got = get(
        &s,
        json!({"id": "req:r", "include_edges": {"exclude_edge_types": ["SATISFIES"], "offset": 0, "budget_chars": 30000}}),
    )
    .await
    .expect("exclude");
    assert!(
        got["edges"]["items"]
            .as_array()
            .expect("items")
            .iter()
            .all(|e| e["edge_type"] != "SATISFIES")
    );
}

#[tokio::test]
async fn a_bad_filter_is_refused_naming_what_was_wrong() {
    let s = design().await;
    let e = get(
        &s,
        json!({"id": "req:r", "include_edges": {"direction": "up"}}),
    )
    .await
    .expect_err("an unknown direction is refused");
    assert!(e.contains("up") && e.contains("both"), "{e}");

    let e = get(
        &s,
        json!({"id": "req:r", "include_edges": {"edge_type": ["SATISFIES"]}}),
    )
    .await
    .expect_err("an unknown filter key is refused, not ignored");
    assert!(e.contains("edge_type") && e.contains("edge_types"), "{e}");

    let e = get(
        &s,
        json!({"id": "req:r", "include_edges": {"edge_types": ["satisfies"]}}),
    )
    .await
    .expect_err("an unknown edge type is refused, not matched against nothing");
    assert!(e.contains("Did you mean SATISFIES"), "{e}");

    let e = get(&s, json!({"id": "req:r", "include_edges": "yes"}))
        .await
        .expect_err("a string is neither true nor a filter");
    assert!(e.contains("include_edges") || e.contains("filter"), "{e}");
}

#[tokio::test]
async fn an_absent_node_says_so_and_reads_no_edges() {
    let s = design().await;
    for args in [
        json!({"id": "req:nope", "include_edges": true}),
        json!({"id": "req:nope", "node_type": "Requirement", "include_edges": true}),
    ] {
        let got = get(&s, args.clone()).await.expect("absent is not an error");
        assert!(got["node"].is_null(), "{got:#}");
        assert!(
            got["edges"].is_null(),
            "no edges block for no node: {got:#}"
        );
        assert!(
            got["empty_because"]
                .as_str()
                .is_some_and(|w| w.contains("req:nope")),
            "the empty answer says which empty: {args} -> {got:#}"
        );
    }
}

/// An absent node says which empty it is on EVERY path, `include_edges` or
/// not, typed or not
/// (`fact:get-node-given-a-node-type-answers-an-absent-id-with-a-bare-null-2026-10-03`).
/// OBSERVED FAILING on main at 293f957: the typed read without
/// `include_edges` answered exactly `{"node": null}`. A typed read of an id
/// held under ANOTHER type names that type, because that is the commonest
/// reason a typed read of a real id comes back empty.
#[tokio::test]
async fn an_absent_node_says_which_empty_on_every_path() {
    let s = design().await;
    for args in [
        json!({"id": "req:nope"}),
        json!({"id": "req:nope", "node_type": "Requirement"}),
        json!({"id": "req:nope", "include_edges": true}),
        json!({"id": "req:nope", "node_type": "Requirement", "include_edges": true}),
    ] {
        let got = get(&s, args.clone()).await.expect("absent is not an error");
        assert!(got["node"].is_null(), "{args} -> {got:#}");
        assert!(
            got["empty_because"]
                .as_str()
                .is_some_and(|w| w.contains("req:nope")),
            "an absent node is never a bare null: {args} -> {got:#}"
        );
    }
    let wrong_type = get(&s, json!({"id": "req:r", "node_type": "Capability"}))
        .await
        .expect("absent under that type is not an error");
    assert!(wrong_type["node"].is_null(), "{wrong_type:#}");
    assert!(
        wrong_type["empty_because"]
            .as_str()
            .is_some_and(|w| w.contains("Requirement")),
        "names the type that does hold the id: {wrong_type:#}"
    );
}

#[tokio::test]
async fn a_node_with_no_edges_says_which_empty() {
    let s = design().await;
    node(
        &s,
        "Requirement",
        "req:alone",
        json!({"name": "Alone", "statement": "No links."}),
    )
    .await;
    let got = get(&s, json!({"id": "req:alone", "include_edges": true}))
        .await
        .expect("get_node");
    let edges = &got["edges"];
    // The in-memory service may credit the write to an acting contributor; a
    // node with nothing at all must then say "no edges at all", and one with
    // only that credit must list it. Either way the empty case is explicit.
    if edges["total"] == 0 {
        assert!(
            edges["empty_because"]
                .as_str()
                .is_some_and(|w| w.contains("no edges at all")),
            "{edges:#}"
        );
    } else {
        assert!(!edges["items"].as_array().expect("items").is_empty());
    }
    let filtered = get(
        &s,
        json!({"id": "req:alone", "include_edges": {"edge_types": ["INCLUDES"]}}),
    )
    .await
    .expect("get_node");
    assert!(
        filtered["edges"]["empty_because"].as_str().is_some(),
        "a filtered-away list says why: {filtered:#}"
    );
}
