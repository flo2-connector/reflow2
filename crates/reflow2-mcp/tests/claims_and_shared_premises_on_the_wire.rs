//! CLAIMS, SUPERSEDED MIRRORS AND SHARED PREMISES, over the MCP surface — PR
//! A2. The core's `claims_and_shared_premises.rs` pins the readings on the
//! trial fixtures; this file pins what a client writes and reads:
//!
//! - `add_inference` with `independent_of: []` clears a stated independence
//!   (the Navier–Stokes map's finding (b): it did not);
//! - `add_requirement` takes `claimed_by`, and `get_node` reads the claim
//!   `claimed`;
//! - `mirror_surface` refuses a refresh that withdraws a judgment a step rests
//!   on, naming the PREMISE_OF edge, and with `accept_withdrawal` keeps it
//!   marked superseded while `loop_status` counts the decision under
//!   `reasons_lost`.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

const KESTREL_V1: &str =
    include_str!("../../reflow2-core/tests/fixtures/split_capstone_kestrel_assess_v1.surface.json");
const KESTREL_V2: &str =
    include_str!("../../reflow2-core/tests/fixtures/split_capstone_kestrel_assess_v2.surface.json");

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "claims-probe".to_string();
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

fn strs(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn an_empty_independent_of_clears_the_statement() {
    let s = session().await;
    for (id, kind) in [
        ("req:p", "proposition"),
        ("req:a", "proposition"),
        ("req:b", "proposition"),
        ("req:t", "theorem"),
    ] {
        s.statement(id, kind).await;
    }
    s.ok(
        "add_inference",
        json!({"id": "inf:a", "rule": "all", "premises": ["req:p", "req:a"], "conclusion": "req:t"}),
    )
    .await;
    let step = s
        .ok(
            "add_inference",
            json!({"id": "inf:b", "rule": "all", "premises": ["req:p", "req:b"],
                   "independent_of": ["inf:a"], "conclusion": "req:t"}),
        )
        .await;
    let ind = &step["inference_reading"]["independence"][0];
    assert_eq!(ind["holds"], true, "the theorem's own setting: {step:#}");
    assert_eq!(strs(&ind["shared_setting"]), vec!["req:p"], "{step:#}");
    let step = s
        .ok(
            "add_inference",
            json!({"id": "inf:b", "rule": "all", "premises": ["req:p", "req:b"],
                   "independent_of": [], "conclusion": "req:t"}),
        )
        .await;
    assert!(
        step["inference_reading"].get("independence").is_none(),
        "`independent_of: []` clears it: {step:#}"
    );
    let n = s.node("inf:b").await;
    assert!(n["properties"].get("independent_of").is_none(), "{n:#}");
}

#[tokio::test]
async fn a_claim_by_a_named_party_reads_claimed() {
    let s = session().await;
    s.ok(
        "add_requirement",
        json!({"id": "req:c", "name": "(C) breakdown", "statement": "Breakdown happens.",
               "statement_kind": "conjecture", "claimed_by": "OpenAI"}),
    )
    .await;
    let n = s.node("req:c").await;
    assert_eq!(n["properties"]["claimed_by"], "OpenAI", "{n:#}");
    let r = s.ok("reasoning_report", json!({"claim": "req:c"})).await;
    let c = r["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "req:c")
        .cloned()
        .unwrap();
    assert_eq!(c["statement"]["reads"], "claimed", "{c:#}");
    assert_eq!(strs(&c["claimed_by"]), vec!["OpenAI"]);
}

#[tokio::test]
async fn a_refresh_that_withdraws_a_premise_is_refused_then_kept_superseded() {
    let s = session().await;
    s.ok(
        "add_project",
        json!({"id": "proj:acq", "name": "Acquisition (FICTIONAL)"}),
    )
    .await;
    let v1: Value = serde_json::from_str(KESTREL_V1).unwrap();
    let v2: Value = serde_json::from_str(KESTREL_V2).unwrap();
    s.ok(
        "mirror_surface",
        json!({"document": v1, "at": "2026-10-22"}),
    )
    .await;
    s.ok(
        "add_decision",
        json!({"id": "dec:b", "name": "Design for agility", "decision": "Design for agility."}),
    )
    .await;
    s.ok(
        "add_inference",
        json!({"id": "inf:b", "rule": "all",
               "premises": ["req:kj2-frequency-agility-unresolved"], "conclusion": "dec:b"}),
    )
    .await;
    let why = s
        .call(
            "mirror_surface",
            json!({"document": v2.clone(), "at": "2026-10-22"}),
        )
        .await
        .expect_err("withdrawing a premise is refused");
    assert!(
        why.contains("PREMISE_OF req:kj2-frequency-agility-unresolved -> inf:b"),
        "{why}"
    );
    let r = s
        .ok(
            "mirror_surface",
            json!({"document": v2, "at": "2026-10-22", "accept_withdrawal": true}),
        )
        .await;
    assert!(
        strs(&r["superseded"]).contains(&"req:kj2-frequency-agility-unresolved".to_string()),
        "{r:#}"
    );
    let kj2 = s.node("req:kj2-frequency-agility-unresolved").await;
    assert!(
        kj2["properties"].get("superseded_by_surface").is_some(),
        "{kj2:#}"
    );
    let loop_status = s.ok("loop_status", json!({})).await;
    let text = loop_status.to_string();
    assert!(
        text.contains("dec:b"),
        "reason lost is owed: {loop_status:#}"
    );
}
