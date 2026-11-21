//! A constructor draws, in the same call, the edges it is nearly always
//! followed by.
//!
//! Measured on flo2.io, 10-30 to 11-20
//! (reflow2:fact:the-tool-counts-re-run-on-0-101-0-three-weeks-later-2026-11-20):
//! - authored_by with add_decision, 45 of 51 runs, never alone;
//! - documents with add_decision, 27 of 42;
//! - constrains right after add_constraint, 13 of 17.
//!
//! So add_decision takes `authored_by` and `documented_by`, and
//! add_constraint takes `constrains`. Each entry runs the edge tool's own
//! body, and the edge tools stay served
//! (req:a-constructor-draws-the-edges-it-is-nearly-always-followed-by-in-the-same-call,
//! Anthony 2026-11-20).

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "a-constructor-draws-its-usual-edges".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

async fn connect() -> Client {
    let service = ReflowService::in_memory().expect("in-memory service");
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = service.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    TestClient
        .serve((client_rx, client_tx))
        .await
        .expect("the in-process handshake")
}

async fn call(c: &Client, tool: &str, args: Value) -> Result<Value, String> {
    let Value::Object(arguments) = args else {
        panic!("arguments for {tool} must be an object");
    };
    match c
        .call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments))
        .await
    {
        Ok(r) if r.is_error == Some(true) => Err(format!("{:?}", r.content)),
        Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
        Err(e) => Err(e.to_string()),
    }
}

async fn ok(c: &Client, tool: &str, args: Value) -> Value {
    call(c, tool, args)
        .await
        .unwrap_or_else(|e| panic!("{tool} must answer: {e}"))
}

/// The edges of `edge_type` leaving or arriving at `id`, each with its
/// stored properties.
async fn edges(c: &Client, id: &str, edge_type: &str) -> Vec<Value> {
    let got = ok(
        c,
        "get_node",
        json!({"id": id, "include_edges": {"edge_types": [edge_type]}}),
    )
    .await;
    got["edges"]["items"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

async fn fixtures(c: &Client) {
    ok(
        c,
        "add_contributor",
        json!({"id": "who:sam", "name": "Sam", "kind": "person"}),
    )
    .await;
    ok(
        c,
        "add_artifact",
        json!({
            "id": "art:pump-sizing-note",
            "name": "Pump sizing note",
            "location": "docs/pump-sizing.md",
            "artifact_type": "document",
            "checksum": format!("sha256:{}", "0".repeat(64)),
        }),
    )
    .await;
    ok(
        c,
        "add_component",
        json!({"id": "cmp:pump", "name": "Pump", "description": "Moves water to the tank."}),
    )
    .await;
}

#[tokio::test]
async fn add_decision_draws_its_authors_and_its_documents_in_the_same_call() {
    let c = connect().await;
    fixtures(&c).await;
    let got = ok(
        &c,
        "add_decision",
        json!({
            "id": "dec:a-two-stage-pump",
            "name": "A two-stage pump",
            "decision": "Use a two-stage pump for the hill run.",
            "rationale": "One stage cannot lift to the tank at the flow needed.",
            "authored_by": [{"contributor_id": "who:sam", "role": "reviewer"}],
            "documented_by": [{"artifact_id": "art:pump-sizing-note", "doc_kind": "design_doc"}],
        }),
    )
    .await;
    let said = got["edges_drawn"].to_string();
    assert!(
        said.contains("dec:a-two-stage-pump AUTHORED_BY who:sam"),
        "the author is named in the reply: {got}"
    );
    assert!(
        said.contains("art:pump-sizing-note DOCUMENTS dec:a-two-stage-pump"),
        "the document is named in the reply: {got}"
    );
    let authors = edges(&c, "dec:a-two-stage-pump", "AUTHORED_BY").await;
    assert!(
        authors
            .iter()
            .any(|e| e["to_id"] == "who:sam" && e["properties"].to_string().contains("reviewer")),
        "AUTHORED_BY who:sam, role reviewer, is stored: {authors:?}"
    );
    let docs = edges(&c, "dec:a-two-stage-pump", "DOCUMENTS").await;
    assert!(
        docs.iter().any(|e| e["from_id"] == "art:pump-sizing-note"),
        "DOCUMENTS from the note is stored: {docs:?}"
    );
}

#[tokio::test]
async fn add_constraint_draws_what_it_constrains_as_constrains_does() {
    let c = connect().await;
    fixtures(&c).await;
    ok(
        &c,
        "add_component",
        json!({"id": "cmp:tank", "name": "Tank", "description": "Holds the day's water."}),
    )
    .await;
    let got = ok(
        &c,
        "add_constraint",
        json!({
            "id": "con:the-pad-carries-900-kg",
            "name": "The pad carries 900 kg",
            "statement": "The concrete pad carries at most 900 kg.",
            "quantity": "mass",
            "limit": 900.0,
            "unit": "kg",
            "constrains": [{"target_id": "cmp:pump", "contribution": 120.0, "unit": "kg", "basis": "estimated"}],
        }),
    )
    .await;
    assert!(
        got["edges_drawn"]
            .to_string()
            .contains("con:the-pad-carries-900-kg CONSTRAINS cmp:pump"),
        "the target is named in the reply: {got}"
    );
    ok(
        &c,
        "constrains",
        json!({"constraint_id": "con:the-pad-carries-900-kg", "target_id": "cmp:tank", "contribution": 600.0, "unit": "kg", "basis": "estimated"}),
    )
    .await;
    // Drawn in the call or by the edge tool, the edge stores the same fields.
    let by_call = edges(&c, "cmp:pump", "CONSTRAINS").await;
    let by_tool = edges(&c, "cmp:tank", "CONSTRAINS").await;
    let keys = |e: &[Value]| -> Vec<String> {
        let mut k: Vec<String> = e[0]["properties"]
            .as_object()
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default();
        k.sort();
        k
    };
    assert_eq!(keys(&by_call), keys(&by_tool), "{by_call:?} / {by_tool:?}");
    assert_eq!(by_call[0]["properties"]["contribution"], json!(120.0));
    let budget = ok(
        &c,
        "budget_report",
        json!({"constraint_id": "con:the-pad-carries-900-kg"}),
    )
    .await;
    assert!(
        budget.to_string().contains("720"),
        "both contributions roll into the total: {budget}"
    );
}

#[tokio::test]
async fn a_named_edge_that_cannot_be_drawn_refuses_the_whole_call() {
    let c = connect().await;
    fixtures(&c).await;
    let refused = call(
        &c,
        "add_decision",
        json!({
            "id": "dec:half-recorded",
            "name": "Half recorded",
            "decision": "Something.",
            "rationale": "Because.",
            "authored_by": [{"contributor_id": "who:sam"}],
            "documented_by": [{"artifact_id": "art:no-such-note"}],
        }),
    )
    .await
    .expect_err("a document that does not exist is refused");
    assert!(
        refused.contains("art:no-such-note"),
        "the refusal names it: {refused}"
    );
    let back = ok(&c, "get_node", json!({"id": "dec:half-recorded"})).await;
    assert!(
        back["node"].is_null(),
        "no decision was left behind: {back}"
    );

    let refused = call(
        &c,
        "add_constraint",
        json!({
            "id": "con:half-a-limit",
            "name": "Half a limit",
            "statement": "At most 10 kg.",
            "quantity": "mass",
            "limit": 10.0,
            "unit": "kg",
            "constrains": [{"target_id": "cmp:no-such-part"}],
        }),
    )
    .await
    .expect_err("a target that does not exist is refused");
    assert!(
        refused.contains("cmp:no-such-part"),
        "the refusal names it: {refused}"
    );
    let back = ok(&c, "get_node", json!({"id": "con:half-a-limit"})).await;
    assert!(
        back["node"].is_null(),
        "no constraint was left behind: {back}"
    );
}
