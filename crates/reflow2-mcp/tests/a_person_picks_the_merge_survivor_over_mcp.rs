//! A PERSON PICKS WHICH NODE A MERGE KEEPS — over the served path, the way a
//! session (Alex's, over MCP) reaches `propose_heal` and `apply_heal`.
//!
//! The core tests (`the_engine_proposes_a_merge_and_a_person_picks_the_survivor`)
//! pin the rule; this pins that the pick survives the JSON round trip through
//! `call_tool`: the proposal's `survivor` block is served, a swapped operation
//! or a `chosen_survivor` is accepted, and a tie applied with no pick is
//! refused with the instruction in the refusal.
//!
//! Anthony, 2026-10-13, on item 23 of the judgement pass: "merge survivor: the
//! engine proposes and a person picks, after a root-cause pass".

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};
use std::time::Duration;

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "a-person-picks-the-merge-survivor".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

async fn connect() -> Client {
    let service = ReflowService::in_memory()
        .expect("in-memory service")
        .without_reaching_out();
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
    let answer = tokio::time::timeout(
        Duration::from_secs(30),
        c.call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments)),
    )
    .await
    .unwrap_or_else(|_| panic!("{tool} did not answer within 30 s"));
    match answer {
        Ok(r) if r.is_error == Some(true) => Err(format!("{:?}", r.content)),
        Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
        Err(e) => Err(e.to_string()),
    }
}

async fn ok(c: &Client, tool: &str, args: Value) -> Value {
    call(c, tool, args)
        .await
        .unwrap_or_else(|e| panic!("{tool} refused: {e}"))
}

async fn exists(c: &Client, id: &str) -> bool {
    !ok(c, "get_node", json!({ "id": id })).await["node"].is_null()
}

/// Two capabilities a person asserted are one thing. With `extra`, `cap:a`
/// also satisfies a requirement, so the engine has a real difference to
/// recommend on; without it the pair differs in nothing but the ids.
async fn pair(c: &Client, extra: bool) {
    ok(c, "add_project", json!({"id": "proj:x", "name": "X"})).await;
    ok(
        c,
        "add_component",
        json!({"id": "cmp:c", "name": "C", "description": "part c"}),
    )
    .await;
    for (id, name) in [("cap:a", "Cap A"), ("cap:b", "Cap B")] {
        ok(
            c,
            "add_capability",
            json!({"id": id, "name": name, "description": "does a"}),
        )
        .await;
        ok(c, "allocate", json!({"from_id": id, "to_id": "cmp:c"})).await;
    }
    if extra {
        ok(
            c,
            "add_requirement",
            json!({"id": "req:r", "name": "R", "statement": "need r"}),
        )
        .await;
        ok(
            c,
            "satisfies",
            json!({"from_id": "cap:a", "to_id": "req:r"}),
        )
        .await;
    }
    ok(
        c,
        "create_edge",
        json!({
            "edge_type": "DUPLICATES",
            "from_type": "Capability", "from_id": "cap:a",
            "to_type": "Capability", "to_id": "cap:b",
            "props": {"basis": "asserted"}
        }),
    )
    .await;
}

fn merges(p: &Value) -> Vec<Value> {
    p["operations"]
        .as_array()
        .expect("operations")
        .iter()
        .filter(|o| o["op"].get("Merge").is_some())
        .cloned()
        .collect()
}

#[tokio::test]
async fn a_person_swaps_the_recommended_survivor_and_keeps_their_choice() {
    let c = connect().await;
    pair(&c, true).await;
    let mut p = ok(&c, "propose_heal", json!({})).await;
    let ops = merges(&p);
    assert_eq!(ops.len(), 1, "{p:#}");
    assert_eq!(
        ops[0]["survivor"]["recommendation"], "cap:a",
        "{:#}",
        ops[0]
    );
    assert_eq!(ops[0]["op"]["Merge"]["keep_id"], "cap:a");

    // The person keeps cap:b: swap keep and remove in the proposal's merge.
    let i = p["operations"]
        .as_array()
        .unwrap()
        .iter()
        .position(|o| o["op"].get("Merge").is_some())
        .unwrap();
    let m = &mut p["operations"][i]["op"]["Merge"];
    m["keep_id"] = json!("cap:b");
    m["remove_id"] = json!("cap:a");

    let report = ok(&c, "apply_heal", json!({ "proposal": p })).await;
    assert_eq!(report["verified"], true, "{report:#}");
    assert!(exists(&c, "cap:b").await, "the person's choice survives");
    assert!(!exists(&c, "cap:a").await);
}

#[tokio::test]
async fn a_tie_applied_without_a_pick_is_refused_and_a_pick_applies() {
    let c = connect().await;
    pair(&c, false).await;
    let p = ok(&c, "propose_heal", json!({})).await;
    let ops = merges(&p);
    assert_eq!(ops.len(), 1, "{p:#}");
    assert_eq!(ops[0]["survivor"]["needs_choice"], true, "{:#}", ops[0]);
    assert!(ops[0]["survivor"]["recommendation"].is_null());

    let refused = call(&c, "apply_heal", json!({ "proposal": p.clone() }))
        .await
        .expect_err("a merge that needs a person's pick is not applied without one");
    assert!(refused.contains("a person must pick"), "{refused}");
    assert!(refused.contains("chosen_survivor"), "{refused}");
    assert!(exists(&c, "cap:a").await && exists(&c, "cap:b").await);

    let mut picked = p;
    let i = picked["operations"]
        .as_array()
        .unwrap()
        .iter()
        .position(|o| o["op"].get("Merge").is_some())
        .unwrap();
    picked["operations"][i]["chosen_survivor"] = json!("cap:b");
    let report = ok(&c, "apply_heal", json!({ "proposal": picked })).await;
    assert_eq!(report["verified"], true, "{report:#}");
    assert!(exists(&c, "cap:b").await, "the person's pick survives");
    assert!(!exists(&c, "cap:a").await);
}
