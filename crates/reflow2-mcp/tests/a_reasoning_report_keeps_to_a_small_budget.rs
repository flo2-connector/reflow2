//! A REASONING REPORT KEEPS TO A SMALL BUDGET.
//!
//! Seen with the Shop stall on flo2.io
//! (`fact:root-cause-the-shop-stall-was-reasoning-evaluation-that-grew-past-the-gateways-30-s-and-nothing-stops-or-reports-it-honestly-2026-10-25`):
//! reasoning_report asked with `budget_chars: 1000` answered 34,450
//! characters. The claims were whole, as promised, and small (1,390
//! characters); the rest was the summary lists — what the reasoning rests on,
//! the uncited premises, the unsupported claims, the assumptions — which the
//! fallback trimmed of prose but never shortened. They are now shortened by
//! the characters each costs, each named in `budget.lists_shortened` with
//! what it shows of its total; and the budget block's own labels are counted
//! before the reply is said to fit. Measured on the Shop's export: 34,026
//! characters before, 5,045 after (one whole claim, one entry of each list and
//! the note saying so are the floor).

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "small-budget-probe".to_string();
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
    async fn ok(&self, tool: &str, args: Value) -> Value {
        let arguments = args.as_object().cloned().unwrap_or_default();
        let r = self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
            .unwrap_or_else(|e| panic!("`{tool}`: {e}"));
        assert!(!r.is_error.unwrap_or(false), "`{tool}` was refused: {r:?}");
        r.structured_content.unwrap_or(Value::Null)
    }
}

fn chars(v: &Value) -> usize {
    serde_json::to_string(v).unwrap().chars().count()
}

/// Forty steps resting on three premises each that cite nothing: the summary lists
/// (what the reasoning rests on, which premises cite no source, which claims
/// nothing supports) carry an entry per premise, and they, not the claims,
/// are what outgrew the budget. They were trimmed of prose only, never
/// shortened, so the reply kept every entry of every list at any budget.
#[tokio::test]
async fn a_small_budget_shortens_the_summary_lists_and_names_them() {
    const PREMISES: usize = 120;
    let s = session().await;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for g in 0..PREMISES / 3 {
        let (goal, step) = (format!("req:goal-{g:02}"), format!("inf:why-{g:02}"));
        nodes.push(json!({"node_type": "Requirement", "id": goal,
            "props": {"name": format!("goal {g}"), "statement": "The goal holds."}}));
        nodes.push(json!({"node_type": "Inference", "id": step,
            "props": {"name": format!("why goal {g} holds"), "rule": "all"}}));
        edges.push(json!({"edge_type": "CONCLUDES", "from_id": step, "to_id": goal}));
        for k in 0..3 {
            let id = format!("req:a-premise-that-cites-no-source-number-{:03}", 3 * g + k);
            nodes.push(json!({"node_type": "Requirement", "id": id,
                "props": {"name": format!("premise {}", 3 * g + k), "statement": "It is so."}}));
            edges.push(json!({"edge_type": "PREMISE_OF", "from_id": id, "to_id": step}));
        }
    }
    s.ok("create_nodes", json!({"nodes": nodes})).await;
    s.ok("create_edges", json!({"edges": edges})).await;

    let whole = s
        .ok("reasoning_report", json!({"budget_chars": 200_000}))
        .await;
    assert_eq!(
        whole["rests_on"].as_array().map(Vec::len),
        Some(PREMISES),
        "{whole:#}"
    );
    let r = s
        .ok("reasoning_report", json!({"budget_chars": 4_000}))
        .await;
    let n = chars(&r);
    assert!(
        n <= 4_000,
        "budget_chars 4,000 answered {n} characters (the whole read-back is {}): {:#}",
        chars(&whole),
        r["budget"]
    );
    let shortened = r["budget"]["lists_shortened"]
        .as_array()
        .unwrap_or_else(|| {
            panic!(
                "the budget block names no shortened list: {:#}",
                r["budget"]
            )
        });
    let rests_on = shortened
        .iter()
        .find(|l| l["path"] == "rests_on")
        .unwrap_or_else(|| panic!("`rests_on` is not named as shortened: {shortened:#?}"));
    assert_eq!(rests_on["total"], PREMISES, "{rests_on}");
    assert!(
        rests_on["shown"].as_u64().unwrap() < PREMISES as u64,
        "{rests_on}"
    );
    // Every claim on the page is still whole.
    for c in r["claims"].as_array().unwrap() {
        let id = c["id"].as_str().unwrap();
        let w = whole["claims"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["id"] == id)
            .unwrap();
        assert_eq!(c["support"], w["support"], "{id}");
    }
}
