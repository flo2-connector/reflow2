//! PROOFS AND ARGUMENTS READ RIGHT, over the MCP surface — PR A1. The core's
//! `proofs_and_arguments_read_right.rs` pins the readings on the trial
//! fixtures; this file pins what a client writes and reads:
//!
//! - `add_requirement` takes `statement_kind` `bridge`, and `get_node` reads
//!   it `bridge`; what rests on it reads conditional, and `reasoning_report`
//!   lists it under `bridges`;
//! - `create_edge` draws `EQUIVALENT_TO` with `by`, neither side's reading
//!   moves, `reasoning_report` names `equivalent_to`, and `propagate_from`
//!   names `equivalents_reached`;
//! - a step whose premise is equivalent to its conclusion reads `circular` on
//!   `get_node`.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "proof-probe".to_string();
        cfg
    }
}

struct Session {
    client: rmcp::service::RunningService<rmcp::service::RoleClient, Probe>,
}

async fn session() -> Session {
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
    Session { client }
}

impl Session {
    async fn call(&self, tool: &str, args: Value) -> Result<Value, String> {
        let arguments = args.as_object().cloned().unwrap_or_default();
        match self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
        {
            Ok(r) if r.is_error.unwrap_or(false) => Err(r
                .content
                .iter()
                .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                .collect::<Vec<_>>()
                .join("\n")),
            Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
            Err(rmcp::service::ServiceError::McpError(e)) => Err(e.message.to_string()),
            Err(e) => Err(format!("transport failure: {e}")),
        }
    }

    async fn ok(&self, tool: &str, args: Value) -> Value {
        let shown = args.to_string();
        self.call(tool, args)
            .await
            .unwrap_or_else(|e| panic!("`{tool}` {shown} was refused: {e}"))
    }

    async fn statement(&self, id: &str, kind: &str) {
        self.ok(
            "add_requirement",
            json!({"id": id, "name": format!("statement {id}"),
                   "statement": format!("The statement {id}."), "statement_kind": kind}),
        )
        .await;
    }

    async fn node(&self, id: &str) -> Value {
        self.ok("get_node", json!({"id": id})).await["node"].clone()
    }
}

#[tokio::test]
async fn a_bridge_is_written_and_read_as_a_bridge() {
    let s = session().await;
    s.ok(
        "add_requirement",
        json!({"id": "req:godel", "name": "Gödel's first incompleteness theorem",
               "statement": "A consistent formal system of arithmetic is incomplete.",
               "statement_kind": "theorem", "taken_as_given": true}),
    )
    .await;
    s.statement("req:bridge", "bridge").await;
    s.statement("req:minds", "proposition").await;
    s.ok(
        "add_inference",
        json!({"id": "inf:carry", "rule": "all", "premises": ["req:godel", "req:bridge"],
               "conclusion": "req:minds"}),
    )
    .await;
    assert_eq!(
        s.node("req:bridge").await["statement_reading"]["reads"],
        "bridge"
    );
    assert_eq!(
        s.node("req:minds").await["proof_status"]["status"],
        "conditional"
    );
    let r = s
        .ok("reasoning_report", json!({"claim": "req:minds"}))
        .await;
    assert_eq!(r["bridges"], json!(["req:bridge"]), "{r:#}");
    let walk = s
        .ok("propagate_from", json!({"seed_ids": ["req:bridge"]}))
        .await;
    let rows = walk["inferences"].as_array().expect("rows");
    assert!(
        rows.iter()
            .any(|r| r["bridges_lost"] == json!(["req:bridge"])),
        "{walk:#}"
    );
}

#[tokio::test]
async fn two_statements_are_drawn_equivalent_and_a_restating_step_reads_circular() {
    let s = session().await;
    s.statement("req:rh", "conjecture").await;
    s.statement("req:robin", "conjecture").await;
    s.statement("req:robin-1984", "theorem").await;
    let before = s.node("req:rh").await["statement_reading"].clone();
    s.ok(
        "create_edge",
        json!({"edge_type": "EQUIVALENT_TO", "from_id": "req:robin", "to_id": "req:rh",
               "props": {"by": "req:robin-1984"}}),
    )
    .await;
    assert_eq!(s.node("req:rh").await["statement_reading"], before);
    let r = s.ok("reasoning_report", json!({"claim": "req:rh"})).await;
    let claims = r["claims"].as_array().expect("claims");
    let rh = claims.iter().find(|c| c["id"] == "req:rh").expect("rh");
    assert_eq!(rh["equivalent_to"][0]["id"], "req:robin", "{r:#}");
    let walk = s
        .ok("propagate_from", json!({"seed_ids": ["req:robin"]}))
        .await;
    assert_eq!(walk["equivalents_reached"][0]["to"], "req:rh", "{walk:#}");

    s.ok(
        "add_inference",
        json!({"id": "inf:closing", "rule": "all", "premises": ["req:robin"],
               "conclusion": "req:rh"}),
    )
    .await;
    let step = s.node("inf:closing").await;
    assert!(
        step["inference_reading"]["circular"]
            .as_str()
            .is_some_and(|w| w.starts_with("circular")),
        "{step:#}"
    );
}
