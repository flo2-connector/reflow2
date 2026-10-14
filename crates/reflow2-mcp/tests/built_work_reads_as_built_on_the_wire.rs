//! A Component's `built_reading` reaches the reader, and the owner's
//! awaiting-word list rides `loop_status`'s scoped answer — both ADDITIVE.
//!
//! `fact:root-cause-built-work-stays-open-because-its-record-is-kept-by-hand-and-the-ship-moves-only-the-capability-2026-10-13`:
//! 107 of 109 components on reflow2's own design read `planned` while 88 have
//! built work behind them, and `get_node`/`scan_nodes` said nothing beside the
//! status. The core pins the computation (`built_work_reads_as_built.rs`,
//! `awaiting_the_owners_word.rs`); this file pins the SURFACE: the field is
//! there on a Component and nowhere else, the stored status is untouched, and
//! every key a reply carried before is still carried.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "built-reading-probe".to_string();
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
        let shown = args.to_string();
        let arguments = args.as_object().cloned().unwrap_or_default();
        let r = self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
            .unwrap_or_else(|e| panic!("`{tool}` {shown} failed: {e}"));
        assert!(
            !r.is_error.unwrap_or(false),
            "`{tool}` {shown} answered an error: {:?}",
            r.content
        );
        r.structured_content.unwrap_or(Value::Null)
    }

    async fn node(&self, node_type: &str, id: &str, props: Value) {
        self.ok(
            "create_node",
            json!({"node_type": node_type, "id": id, "props": props}),
        )
        .await;
    }

    async fn edge(&self, edge_type: &str, from: &str, to: &str) {
        self.ok(
            "create_edge",
            json!({"edge_type": edge_type, "from_id": from, "to_id": to}),
        )
        .await;
    }

    /// A planned component hosting one realized capability, and one hosting
    /// nothing built.
    async fn seeded(self) -> Self {
        self.node(
            "Component",
            "cmp:engine",
            json!({"name": "Engine", "purpose": "runs", "status": "planned"}),
        )
        .await;
        self.node(
            "Component",
            "cmp:idle",
            json!({"name": "Idle", "purpose": "waits", "status": "planned"}),
        )
        .await;
        self.node(
            "Capability",
            "cap:run",
            json!({"name": "Run", "description": "runs", "status": "realized"}),
        )
        .await;
        self.node(
            "Capability",
            "cap:wait",
            json!({"name": "Wait", "description": "waits", "status": "planned"}),
        )
        .await;
        self.edge("ALLOCATED_TO", "cap:run", "cmp:engine").await;
        self.edge("ALLOCATED_TO", "cap:wait", "cmp:idle").await;
        self
    }
}

#[tokio::test]
async fn get_node_on_a_component_carries_its_built_reading_beside_the_status() {
    let s = session().await.seeded().await;
    let r = s.ok("get_node", json!({"id": "cmp:engine"})).await;
    let node = &r["node"];
    assert_eq!(node["properties"]["status"], "planned", "untouched: {r}");
    assert_eq!(
        node["discontinued"], false,
        "every key it carried is kept: {r}"
    );
    let reading = &node["built_reading"];
    assert_eq!(reading["reads_as"], "built", "{r}");
    assert_eq!(reading["capabilities_allocated"], 1, "{r}");
    assert_eq!(reading["capabilities_built"], 1, "{r}");
    assert_eq!(reading["files_built"], 0, "{r}");
    assert_eq!(reading["checks_passing"], 0, "{r}");

    let idle = s.ok("get_node", json!({"id": "cmp:idle"})).await;
    assert_eq!(
        idle["node"]["built_reading"]["reads_as"], "not_built",
        "{idle}"
    );

    // Only a Component carries it: a capability's own status is what the ship
    // moves, and it already has its readings.
    let cap = s.ok("get_node", json!({"id": "cap:run"})).await;
    assert!(
        cap["node"].get("built_reading").is_none(),
        "no built_reading on a Capability: {cap}"
    );
}

#[tokio::test]
async fn scan_nodes_carries_it_on_every_component_brief_or_full() {
    let s = session().await.seeded().await;
    for brief in [true, false] {
        let r = s
            .ok(
                "scan_nodes",
                json!({"node_type": "Component", "brief": brief}),
            )
            .await;
        let items = r["items"].as_array().expect("items");
        assert_eq!(items.len(), 2, "{r}");
        for item in items {
            assert!(
                item["built_reading"]["reads_as"].is_string(),
                "brief={brief}: every component carries its reading: {item}"
            );
            assert_eq!(item["discontinued"], false, "{item}");
        }
        let engine = items
            .iter()
            .find(|i| i["node_id"] == "cmp:engine")
            .expect("engine");
        assert_eq!(engine["built_reading"]["reads_as"], "built", "{engine}");
        for key in [
            "count",
            "total",
            "offset",
            "returned",
            "omitted",
            "capped_by",
            "brief",
        ] {
            assert!(r.get(key).is_some(), "the list keeps `{key}`: {r}");
        }
    }
    let caps = s
        .ok(
            "scan_nodes",
            json!({"node_type": "Capability", "brief": true}),
        )
        .await;
    for item in caps["items"].as_array().expect("items") {
        assert!(item.get("built_reading").is_none(), "{item}");
    }
}

#[tokio::test]
async fn loop_status_carries_awaiting_word_scoped_and_an_empty_list_unscoped() {
    let s = session().await.seeded().await;
    s.ok(
        "add_contributor",
        json!({"id": "who:owner", "name": "The Owner"}),
    )
    .await;
    let scoped = s
        .ok("loop_status", json!({"contributor_id": "who:owner"}))
        .await;
    assert!(
        scoped["awaiting_word"].is_array(),
        "the scoped answer lists what awaits an owner's word: {scoped}"
    );
    let unscoped = s.ok("loop_status", json!({})).await;
    assert_eq!(
        unscoped["awaiting_word"],
        json!([]),
        "unscoped it is empty, as gaps_on_owned_ground and questions_put_to_them are: {unscoped}"
    );
    for key in [
        "assigned_decisions",
        "questions_put_to_them",
        "gaps_on_owned_ground",
        "next",
        "clean",
    ] {
        assert!(scoped.get(key).is_some(), "the scoped answer keeps `{key}`");
    }
}
