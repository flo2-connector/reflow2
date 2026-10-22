//! A COMPOSED REPLY HOLDS ITS BUDGET, whatever the composition — the bounded
//! half of `req:one-composed-check-runs-over-a-hub-and-several-of-its-designs`
//! and of `req:designs-are-shown-where-they-already-touch-before-anyone-declares-it`
//! (phase 7, step 4).
//!
//! WHAT FAILED. Measured 2026-10-22 on PR 4's head (c8d6e2e29) through the
//! door, a hub composed with fifteen exports answered 58,505 characters at the
//! default budget of 30,000 (45,091 on PR 3): every list was at its floor of
//! one entry and the reply was still over, because a list's floor is one WHOLE
//! entry — a cut-off cluster naming about 75 ids — one per `by_pair` key, and
//! `link_status` was never sampled. A host that refuses an oversized reply
//! (flo2.io) has no size to suggest that fits.
//!
//! THE CLASS, across budgets (the default, 12,000, 6,000, 60,000) and across
//! the shapes (one other design by `other_graph_ids`, two, every pinned design,
//! and the one-other-design call with `budget_chars`): the compact reply is at
//! most the budget; every count survives whole; whatever was withheld is named
//! (the `budget` block and `withheld_to_fit`); and a reply under its budget is
//! untouched.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "a-composed-reply-holds-its-budget".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

async fn connect(service: ReflowService) -> Client {
    let (server_rx, client_tx) = tokio::io::duplex(1 << 24);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 24);
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

async fn call(c: &Client, tool: &str, args: Value) -> Value {
    let Value::Object(arguments) = args.clone() else {
        panic!("arguments for {tool} must be an object");
    };
    match c
        .call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments))
        .await
    {
        Ok(r) if r.is_error == Some(true) => panic!("{tool} refused: {:?}", r.content),
        Ok(r) => r.structured_content.unwrap_or(Value::Null),
        Err(e) => panic!("{tool} refused: {e}"),
    }
}

const MEMBERS: usize = 10;
const PER_MEMBER: usize = 150;

/// A hub pinning ten member exports, each one large threaded body (a
/// Component and 150 Capabilities allocated to it) that the hub's
/// unwired pin joins only to itself — so each pair hub×member carries a
/// cut-off cluster naming over 150 ids — and every capability an
/// unmotivated-capability gap.
async fn big_hub(tag: &str) -> (Client, Vec<String>, Value) {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-composed-budget-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut ids = Vec::new();
    let mut first_doc = Value::Null;
    let hub = connect(ReflowService::in_memory_as("hub").expect("hub")).await;
    call(
        &hub,
        "add_component",
        json!({"id": "cmp:hub", "name": "hub", "description": "the hub"}),
    )
    .await;
    for m in 0..MEMBERS {
        let gid = format!("member{m:02}");
        let c = connect(ReflowService::in_memory_as(&gid).expect("member")).await;
        let mut nodes = vec![json!({"node_type": "Component", "id": "cmp:body",
                                    "props": {"name": format!("{gid} body"), "purpose": "holds it"}})];
        let mut edges = Vec::new();
        for i in 0..PER_MEMBER {
            nodes.push(
                json!({"node_type": "Capability", "id": format!("cap:c{i:03}"),
                              "props": {"name": format!("{gid} capability number {i}"),
                                        "description": "one of many"}}),
            );
            edges.push(
                json!({"edge_type": "ALLOCATED_TO", "from_id": format!("cap:c{i:03}"),
                              "to_id": "cmp:body"}),
            );
        }
        call(&c, "create_nodes", json!({"nodes": nodes})).await;
        call(&c, "create_edges", json!({"edges": edges})).await;
        let path = dir.join(format!("{gid}.json"));
        call(&c, "export_graph", json!({"path": path.to_string_lossy()})).await;
        if m == 0 {
            first_doc = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        }
        call(
            &hub,
            "external_dependency",
            json!({"id": format!("dep:{gid}"), "name": gid, "version": "v1",
                   "source": format!("https://example.org/{gid}"), "graph_id": gid,
                   "design_export": path.to_string_lossy()}),
        )
        .await;
        ids.push(gid);
    }
    (hub, ids, first_doc)
}

fn size(v: &Value) -> usize {
    v.to_string().len()
}

#[tokio::test]
async fn every_shape_of_the_composed_reply_holds_every_budget_and_keeps_its_counts() {
    let (hub, ids, doc) = big_hub("class").await;
    let shapes: Vec<(&str, Value)> = vec![
        ("one other design", json!({"other_graph_ids": [ids[0]]})),
        (
            "two other designs",
            json!({"other_graph_ids": [ids[0], ids[1]]}),
        ),
        ("every pinned design", json!({"tracked_designs": true})),
    ];
    for (shape, args) in &shapes {
        let mut whole_args = args.clone();
        whole_args["budget_chars"] = json!(100_000_000);
        let whole = call(&hub, "compose_and_analyse", whole_args).await;
        for budget in [None, Some(12_000usize), Some(6_000), Some(60_000)] {
            let mut a = args.clone();
            if let Some(b) = budget {
                a["budget_chars"] = json!(b);
            }
            let limit = budget.unwrap_or(30_000);
            let got = call(&hub, "compose_and_analyse", a).await;
            assert!(
                size(&got) <= limit,
                "{shape} at {limit}: {} characters (whole {}): budget block {}",
                size(&got),
                size(&whole),
                got["budget"]
            );
            assert_eq!(
                got["counts"], whole["counts"],
                "{shape} at {limit}: every count"
            );
            if size(&whole) <= limit {
                assert_eq!(
                    got, whole,
                    "{shape} at {limit}: a reply that fits is untouched"
                );
            } else {
                assert_eq!(
                    got["budget"]["applied"],
                    json!(true),
                    "{shape} at {limit}: says it was bounded: {}",
                    got["budget"]
                );
            }
            assert_eq!(
                got["designs_composed"], whole["designs_composed"],
                "{shape} at {limit}: what was composed stays whole"
            );
        }
    }
    // The one-other-design call, bounded when it asks.
    let whole_one = call(
        &hub,
        "compose_and_analyse",
        json!({"design": doc, "namespace": ids[0]}),
    )
    .await;
    for budget in [12_000usize, 6_000] {
        let got = call(
            &hub,
            "compose_and_analyse",
            json!({"design": doc, "namespace": ids[0], "budget_chars": budget}),
        )
        .await;
        assert!(
            size(&got) <= budget,
            "the one-other-design call at {budget}: {} characters (whole {})",
            size(&got),
            size(&whole_one)
        );
        assert_eq!(
            got["link_status"]["status"],
            whole_one["link_status"]["status"]
        );
    }
}

#[tokio::test]
async fn a_finding_too_big_for_its_budget_says_how_many_ids_it_names() {
    let (hub, _ids, _) = big_hub("ids").await;
    let got = call(
        &hub,
        "compose_and_analyse",
        json!({"tracked_designs": true, "budget_chars": 6_000}),
    )
    .await;
    let withheld = &got["withheld_to_fit"];
    assert!(
        withheld.is_object(),
        "what was withheld beyond whole list entries is named: {got}"
    );
    // Wherever a finding's ids were cut, it says how many it names.
    let mut each = |f: &Value| {
        if let Some(total) = f.get("affected_ids_total") {
            assert!(
                total.as_u64().unwrap() as usize > f["affected_ids"].as_array().unwrap().len(),
                "{f}"
            );
        }
    };
    for list in got["by_pair"].as_object().unwrap().values() {
        list.as_array().unwrap().iter().for_each(&mut each);
    }
    for list in got["by_design"].as_object().unwrap().values() {
        list.as_array().unwrap().iter().for_each(&mut each);
    }
}
