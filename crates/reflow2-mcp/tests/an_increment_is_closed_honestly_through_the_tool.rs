//! AN INCREMENT IS CLOSED HONESTLY THROUGH THE TOOL, and through no other door.
//!
//! req:an-increment-that-will-not-be-finished-can-be-closed-honestly: the
//! close's own rules are pinned in reflow2-core's
//! `an_increment_that_will_not_be_finished_is_closed_honestly.rs`. This file
//! pins what a CLIENT meets:
//! - `close_increment` is served, refuses a close with no `approver` (the
//!   owner's word) before anything is written, and declares that it always
//!   settles, so a gateway signing for its caller reads the rule instead of
//!   guessing it;
//! - `set_epoch_status` and `add_epoch` offer only `planned` and `arrived`,
//!   and refuse the three closes, naming `close_increment`;
//! - a deferred increment leaves the list `loop_status` reads out and is
//!   counted apart there.
//!
//! OBSERVED FAILING first: written before the tool existed, on origin/main
//! e2bbdc6f3, where `close_increment` is an unknown tool.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "close-increment-probe".to_string();
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
    let s = Session { client };
    s.ok("add_project", json!({"id": "proj:1", "name": "Thing"}))
        .await;
    s.ok(
        "add_contributor",
        json!({"id": "who:owner", "name": "The Owner", "kind": "person"}),
    )
    .await;
    s.ok(
        "add_epoch",
        json!({"id": "epoch:now", "name": "Now", "epoch_type": "revision", "sequence": 1}),
    )
    .await;
    s
}

impl Session {
    async fn call(&self, tool: &str, args: Value) -> (bool, Value, String) {
        let arguments = args.as_object().cloned().unwrap_or_default();
        match self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
        {
            Ok(r) => {
                let text = r
                    .content
                    .iter()
                    .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                    .collect::<Vec<_>>()
                    .join("\n");
                (
                    !r.is_error.unwrap_or(false),
                    r.structured_content.unwrap_or(Value::Null),
                    text,
                )
            }
            Err(rmcp::service::ServiceError::McpError(e)) => {
                (false, Value::Null, e.message.to_string())
            }
            Err(e) => panic!("transport failure on {tool}: {e}"),
        }
    }

    async fn ok(&self, tool: &str, args: Value) -> Value {
        let shown = args.to_string();
        let (answered, structured, text) = self.call(tool, args).await;
        assert!(answered, "`{tool}` {shown} was refused: {text}");
        structured
    }

    async fn refused(&self, tool: &str, args: Value) -> String {
        let shown = args.to_string();
        let (answered, _, text) = self.call(tool, args).await;
        assert!(!answered, "`{tool}` {shown} was accepted: {text}");
        text
    }

    async fn plan(&self, id: &str) {
        self.ok(
            "plan_epoch",
            json!({"id": id, "name": id, "epoch_type": "milestone", "sequence": 2}),
        )
        .await;
    }

    async fn need(&self, req: &str, epoch: &str) {
        self.ok(
            "add_requirement",
            json!({"id": req, "name": req, "statement": "it must hold"}),
        )
        .await;
        self.ok("schedule_for", json!({"item_id": req, "target_id": epoch}))
            .await;
    }

    async fn epoch_status(&self, id: &str) -> String {
        let n = self.ok("get_node", json!({"id": id})).await;
        n.pointer("/node/properties/status")
            .and_then(Value::as_str)
            .unwrap_or("arrived")
            .to_string()
    }

    async fn tool(&self, name: &str) -> rmcp::model::Tool {
        self.client
            .list_all_tools()
            .await
            .expect("tools")
            .into_iter()
            .find(|t| t.name == name)
            .unwrap_or_else(|| panic!("`{name}` is served"))
    }
}

#[tokio::test]
async fn a_close_moves_and_drops_its_leftovers_on_the_owners_word() {
    let s = session().await;
    s.plan("epoch:a").await;
    s.plan("epoch:b").await;
    s.need("req:one", "epoch:a").await;
    s.need("req:two", "epoch:a").await;

    let args = json!({
        "epoch_id": "epoch:a",
        "how": "dropped",
        "reason": "the work moved on",
        "recorded_in": "epoch:now",
        "acted_at": "2026-11-17",
        "leftovers": [
            {"item_id": "req:one", "fate": "moves", "to": "epoch:b"},
            {"item_id": "req:two", "fate": "dropped", "reason": "not needed"},
        ],
    });
    let unsigned = s.refused("close_increment", args.clone()).await;
    assert!(unsigned.contains("approver"), "{unsigned}");
    assert_eq!(s.epoch_status("epoch:a").await, "planned");

    let mut signed = args.clone();
    signed["approver"] = json!("who:owner");
    let closed = s.ok("close_increment", signed).await;
    assert_eq!(closed["status"], "dropped", "{closed}");
    assert_eq!(closed["leftovers"][0]["fate"], "moves");
    assert_eq!(closed["leftovers"][1]["reason"], "not needed");
    assert_eq!(s.epoch_status("epoch:a").await, "dropped");
}

#[tokio::test]
async fn an_unanswered_leftover_refuses_the_whole_close() {
    let s = session().await;
    s.plan("epoch:a").await;
    s.need("req:one", "epoch:a").await;
    let why = s
        .refused(
            "close_increment",
            json!({"epoch_id": "epoch:a", "how": "deferred", "approver": "who:owner"}),
        )
        .await;
    assert!(why.contains("req:one"), "{why}");
    assert!(why.contains("None is defaulted"), "{why}");
    assert_eq!(s.epoch_status("epoch:a").await, "planned");
}

#[tokio::test]
async fn the_tool_declares_that_every_close_is_the_owners_word() {
    let s = session().await;
    let t = s.tool("close_increment").await;
    let meta = serde_json::to_value(&t.meta).expect("meta");
    let settles = meta
        .get("reflow2/settles")
        .unwrap_or_else(|| panic!("close_increment declares its settle: {meta}"));
    assert_eq!(settles["when"], "always", "{settles}");
    assert_eq!(settles["approver"], "approver", "{settles}");
    assert_eq!(settles["unsigned"], "refused", "{settles}");
}

#[tokio::test]
async fn the_status_doors_offer_only_planned_and_arrived_and_name_the_close() {
    let s = session().await;
    for (tool, pointer) in [
        ("set_epoch_status", "/properties/status/enum"),
        ("add_epoch", "/properties/status/enum"),
    ] {
        let t = s.tool(tool).await;
        let schema = serde_json::to_value(&*t.input_schema).expect("schema");
        let offered: Vec<String> = schema
            .pointer(pointer)
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("{tool} offers a status enum: {schema}"))
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
        assert_eq!(offered, vec!["planned", "arrived"], "{tool}");
    }

    s.plan("epoch:a").await;
    let why = s
        .refused(
            "set_epoch_status",
            json!({"epoch_id": "epoch:a", "status": "dropped"}),
        )
        .await;
    assert!(why.contains("close_increment"), "{why}");
    let why = s
        .refused(
            "add_epoch",
            json!({"id": "epoch:z", "name": "z", "epoch_type": "milestone", "sequence": 3,
                   "status": "deferred"}),
        )
        .await;
    assert!(why.contains("close_increment"), "{why}");
}

#[tokio::test]
async fn a_deferred_increment_is_counted_apart_where_a_session_ends() {
    let s = session().await;
    s.plan("epoch:a").await;
    s.plan("epoch:b").await;
    s.need("req:one", "epoch:a").await;
    s.ok(
        "close_increment",
        json!({"epoch_id": "epoch:a", "how": "deferred", "approver": "who:owner",
               "leftovers": [{"item_id": "req:one", "fate": "stays"}]}),
    )
    .await;
    let status = s.ok("loop_status", json!({})).await;
    let open = &status["open_increments"];
    assert_eq!(open["open"], 1, "{open}");
    assert_eq!(open["deferred"], 1, "{open}");
    assert!(
        open["increments"]
            .as_array()
            .expect("rows")
            .iter()
            .all(|r| r["epoch_id"] != "epoch:a"),
        "{open}"
    );
}
