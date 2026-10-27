//! A BULK WRITE SAYS NO NEAR-MATCH CHECK RAN.
//!
//! Seen in the sinogram trial on flo2.io (reflow2 0.92.0):
//! `fact:create-nodes-runs-no-near-match-check-and-its-reply-does-not-say-so-2026-10-27`.
//! create_nodes wrote nodes with no near-match check, while a later
//! add_decision flagged overlap. Skipping the check on the generic and bulk
//! paths is by design (cap:search-first-at-the-tool: "bulk paths are
//! unaffected"), but neither reply said so, and a reply that says nothing
//! reads as "nothing close" (req:no-silent-fallback).
//!
//! Now the reply of create_nodes and create_node carries `near_match_check`,
//! saying no check ran and naming the reads that do one.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "bulk-check-probe".to_string();
        cfg
    }
}

async fn call(
    client: &rmcp::service::RunningService<rmcp::service::RoleClient, Probe>,
    tool: &str,
    args: Value,
) -> Value {
    let arguments = args.as_object().cloned().unwrap_or_default();
    let r = client
        .call_tool(
            rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
        )
        .await
        .unwrap_or_else(|e| panic!("`{tool}`: {e}"));
    assert!(!r.is_error.unwrap_or(false), "`{tool}` was refused: {r:?}");
    r.structured_content.unwrap_or(Value::Null)
}

fn says_no_check_ran(tool: &str, r: &Value) {
    let said = r["near_match_check"].as_str().unwrap_or_else(|| {
        panic!("{tool}'s reply does not say whether a near-match check ran: {r:#}")
    });
    assert!(said.contains("NO NEAR-MATCH CHECK RAN"), "{tool}: {said}");
    assert!(said.contains("search_design"), "{tool}: {said}");
}

#[tokio::test]
async fn create_nodes_and_create_node_say_no_near_match_check_ran() {
    let svc = ReflowService::in_memory().expect("in-memory service");
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = svc.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    let client = Probe
        .serve((client_rx, client_tx))
        .await
        .expect("in-process handshake");

    let r = call(
        &client,
        "create_nodes",
        json!({"nodes": [
            {"node_type": "Requirement", "id": "req:pred-arcs",
             "props": {"name": "The sinogram shows arcs", "statement": "Arcs."}},
            {"node_type": "Requirement", "id": "req:pred-arcs-too",
             "props": {"name": "The sinogram shows arcs", "statement": "Arcs."}}
        ]}),
    )
    .await;
    assert_eq!(r["applied"], true, "{r:#}");
    says_no_check_ran("create_nodes", &r);

    let r = call(
        &client,
        "create_node",
        json!({"node_type": "Requirement", "id": "req:pred-arcs-three",
               "props": {"name": "The sinogram shows arcs", "statement": "Arcs."}}),
    )
    .await;
    says_no_check_ran("create_node", &r);
}
