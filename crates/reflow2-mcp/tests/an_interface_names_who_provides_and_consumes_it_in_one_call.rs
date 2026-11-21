//! An interface names who provides and who consumes it in the call that
//! creates it.
//!
//! On flo2.io `consumes` and `provides` were run next to each other 31 of 34
//! and 33 of 35 times across the 09-24 to 11-20 counts, and never alone
//! (reflow2:fact:the-tool-counts-re-run-on-0-101-0-three-weeks-later-2026-11-20).
//! A seam is recorded as an interface and its two sides, so the two sides are
//! now arguments of add_interface (`provided_by`, `consumed_by`), drawn
//! through the same bodies `provides` and `consumes` run. Those tools stay
//! served for every other use
//! (req:mint-seat-and-consumes-provides-are-listed-under-their-process-tool-now,
//! Anthony 2026-11-21).

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "an-interface-names-who-provides-and-consumes-it".to_string();
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

async fn component(c: &Client, id: &str, name: &str) {
    call(
        c,
        "add_component",
        json!({"id": id, "name": name, "description": format!("{name} in the test design")}),
    )
    .await
    .unwrap_or_else(|e| panic!("add_component {id}: {e}"));
}

/// Every edge of `edge_type` touching `id`, as (from, to).
async fn edges(c: &Client, id: &str, edge_type: &str) -> Vec<(String, String)> {
    let got = call(
        c,
        "get_node",
        json!({"id": id, "include_edges": {"edge_types": [edge_type]}}),
    )
    .await
    .unwrap_or_else(|e| panic!("get_node {id}: {e}"));
    got["edges"]["items"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|e| {
            (
                e["from_id"].as_str().unwrap_or_default().to_string(),
                e["to_id"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}

#[tokio::test]
async fn one_call_writes_the_interface_and_both_its_sides() {
    let c = connect().await;
    component(&c, "cmp:pump-controller", "Pump controller").await;
    component(&c, "cmp:dashboard", "Dashboard").await;
    component(&c, "cmp:alarm-panel", "Alarm panel").await;

    let got = call(
        &c,
        "add_interface",
        json!({
            "id": "ifc:pump-status",
            "name": "Pump status feed",
            "description": "The controller publishes flow and pressure once a second.",
            "provided_by": ["cmp:pump-controller"],
            "consumed_by": ["cmp:dashboard", "cmp:alarm-panel"],
        }),
    )
    .await
    .expect("add_interface with both sides");

    assert_eq!(
        edges(&c, "ifc:pump-status", "PROVIDES").await,
        vec![(
            "cmp:pump-controller".to_string(),
            "ifc:pump-status".to_string()
        )],
        "the provider is drawn: {got}"
    );
    let mut consumers = edges(&c, "ifc:pump-status", "CONSUMES").await;
    consumers.sort();
    assert_eq!(
        consumers,
        vec![
            ("cmp:alarm-panel".to_string(), "ifc:pump-status".to_string()),
            ("cmp:dashboard".to_string(), "ifc:pump-status".to_string()),
        ],
        "both consumers are drawn: {got}"
    );
    let said = got["edges_drawn"].to_string();
    for s in [
        "cmp:pump-controller PROVIDES ifc:pump-status",
        "cmp:dashboard CONSUMES ifc:pump-status",
        "cmp:alarm-panel CONSUMES ifc:pump-status",
    ] {
        assert!(said.contains(s), "the reply names `{s}`: {got}");
    }
}

#[tokio::test]
async fn an_edge_drawn_here_reads_as_the_edge_tool_draws_it() {
    let c = connect().await;
    component(&c, "cmp:a", "A").await;
    component(&c, "cmp:b", "B").await;
    call(
        &c,
        "add_interface",
        json!({"id": "ifc:one", "name": "One", "provided_by": ["cmp:a"]}),
    )
    .await
    .expect("in the call");
    call(&c, "add_interface", json!({"id": "ifc:two", "name": "Two"}))
        .await
        .expect("bare");
    call(
        &c,
        "provides",
        json!({"from_id": "cmp:a", "to_id": "ifc:two"}),
    )
    .await
    .expect("by the edge tool");
    call(
        &c,
        "consumes",
        json!({"from_id": "cmp:b", "to_id": "ifc:two"}),
    )
    .await
    .expect("by the edge tool");
    call(
        &c,
        "add_interface",
        json!({"id": "ifc:one", "consumed_by": ["cmp:b"]}),
    )
    .await
    .expect("a revise adds a side");

    // The seam reads the same however it was recorded: the same edges, with
    // the same stored properties.
    let props = |v: Value| -> Vec<String> {
        let mut keys: Vec<String> = v["edges"]["items"][0]["properties"]
            .as_object()
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default();
        keys.sort();
        keys
    };
    for edge in ["PROVIDES", "CONSUMES"] {
        let one = call(
            &c,
            "get_node",
            json!({"id": "ifc:one", "include_edges": {"edge_types": [edge]}}),
        )
        .await
        .expect("get_node");
        let two = call(
            &c,
            "get_node",
            json!({"id": "ifc:two", "include_edges": {"edge_types": [edge]}}),
        )
        .await
        .expect("get_node");
        assert_eq!(
            props(one.clone()),
            props(two.clone()),
            "{edge}: {one} / {two}"
        );
    }
    assert_eq!(edges(&c, "ifc:one", "PROVIDES").await.len(), 1);
    assert_eq!(edges(&c, "ifc:one", "CONSUMES").await.len(), 1);
}

#[tokio::test]
async fn a_side_naming_nothing_is_refused_and_nothing_is_written() {
    let c = connect().await;
    component(&c, "cmp:real", "Real").await;
    let refused = call(
        &c,
        "add_interface",
        json!({
            "id": "ifc:half",
            "name": "Half a seam",
            "provided_by": ["cmp:real"],
            "consumed_by": ["cmp:typo"],
        }),
    )
    .await
    .expect_err("a consumer that does not exist is refused");
    assert!(
        refused.contains("cmp:typo"),
        "the refusal names the id: {refused}"
    );
    let back = call(&c, "get_node", json!({"id": "ifc:half"}))
        .await
        .expect("get_node");
    assert!(
        back["node"].is_null(),
        "no interface was left behind: {back}"
    );
    assert!(
        edges(&c, "cmp:real", "PROVIDES").await.is_empty(),
        "and no edge to it"
    );
}
