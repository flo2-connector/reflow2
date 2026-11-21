//! NO DOOR DRAWS AN EDGE FROM A NODE TO ITSELF WHERE THAT MEANS NOTHING — on
//! the wire, through the tools a session calls.
//!
//! The two field reports, by the doors that drew them:
//!
//! - `record_finding` with `caused_by` equal to its own id drew "X CAUSES X",
//!   and `create_edge` drew a CAUSES self-loop on any node
//!   (fact:root-cause-a-finding-can-name-itself-as-its-cause-and-create-edge-draws-a-causes-self-loop-2026-10-30);
//! - `register_alternative`, registering an alternative again, drew "X
//!   CONTRADICTS X" in a proof trial
//!   (fact:defect-two-alternatives-in-the-aa-attempts-reflow2-design-carry-a-contradicts-edge-to-themselves-2026-11-07).
//!
//! The rule is the schema's (`irreflexive: true`), applied at the store's one
//! edge write, so each door below refuses for the same reason; the core half
//! is `crates/reflow2-core/tests/a_relation_that_cannot_hold_to_itself_is_refused_on_every_door.rs`.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "self-loop-probe".to_string();
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
            Err(e) => Err(e.to_string()),
            Ok(r) if r.is_error.unwrap_or(false) => Err(format!("{r:?}")),
            Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
        }
    }

    async fn ok(&self, tool: &str, args: Value) -> Value {
        self.call(tool, args.clone())
            .await
            .unwrap_or_else(|e| panic!("`{tool}` {args} was refused: {e}"))
    }

    async fn refused(&self, tool: &str, args: Value) -> String {
        match self.call(tool, args.clone()).await {
            Ok(v) => panic!("`{tool}` {args} was ACCEPTED: {v:#}"),
            Err(e) => e,
        }
    }

    /// The edges of `edge_type` from `id` to itself, as get_node reads them.
    async fn self_loops(&self, id: &str, edge_type: &str) -> usize {
        let r = self
            .ok(
                "get_node",
                json!({"id": id, "include_edges": {"edge_types": [edge_type]}}),
            )
            .await;
        r["edges"]["items"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter(|e| e["from_id"] == id && e["to_id"] == id)
                    .count()
            })
            .unwrap_or(0)
    }
}

async fn a_requirement(s: &Session, id: &str) {
    s.ok(
        "add_requirement",
        json!({"id": id, "name": format!("Requirement {id}"),
               "statement": format!("{id} is stated.")}),
    )
    .await;
}

/// ⭐ register_alternative, called again for an alternative already
/// registered under the decision, is an update: no CONTRADICTS to itself, its
/// siblings' kept both ways.
#[tokio::test]
async fn registering_an_alternative_again_draws_no_edge_to_itself() {
    let s = session().await;
    s.ok(
        "add_decision",
        json!({"id": "dec:road", "name": "Which road",
               "decision": "Which road to take", "status": "proposed"}),
    )
    .await;
    for (id, loc) in [
        ("alt:a", "a.json"),
        ("alt:b", "b.json"),
        ("alt:a", "a-v2.json"),
    ] {
        s.ok(
            "register_alternative",
            json!({"decision_id": "dec:road", "artifact_id": id,
                   "name": format!("Road {id}"), "location": loc}),
        )
        .await;
    }
    assert_eq!(s.self_loops("alt:a", "CONTRADICTS").await, 0);
    let a = s
        .ok(
            "get_node",
            json!({"id": "alt:a", "include_edges": {"edge_types": ["CONTRADICTS"]}}),
        )
        .await;
    assert_eq!(
        a["edges"]["by_type"]["CONTRADICTS"],
        json!({"in": 1, "out": 1}),
        "alt:a and alt:b must still contradict each other, both ways: {:#}",
        a["edges"]
    );
    assert_eq!(a["node"]["properties"]["location"], "a-v2.json");
}

/// record_finding naming the finding itself as its cause is refused BEFORE
/// anything is written: no finding, no edge.
#[tokio::test]
async fn a_finding_cannot_name_itself_as_its_cause_and_nothing_is_written() {
    let s = session().await;
    a_requirement(&s, "req:a").await;
    let why = s
        .refused(
            "record_finding",
            json!({"id": "fact:self", "subject_id": "req:a",
                   "statement": "Something was observed.",
                   "caused_by": "fact:self", "cause_evidence": "It caused itself."}),
        )
        .await;
    assert!(why.contains("itself"), "{why}");
    let got = s.ok("get_node", json!({"id": "fact:self"})).await;
    assert!(got["node"].is_null(), "the finding was written: {got:#}");
}

/// create_edge and create_edges refuse the two relations seen drawn to a node
/// itself, and still draw them between two nodes.
#[tokio::test]
async fn the_generic_edge_doors_refuse_a_self_loop_where_it_means_nothing() {
    let s = session().await;
    a_requirement(&s, "req:a").await;
    a_requirement(&s, "req:b").await;
    for relation in ["CAUSES", "CONTRADICTS"] {
        let one = |to: &str| {
            json!({"edge_type": relation, "from_type": "Requirement", "from_id": "req:a",
                   "to_type": "Requirement", "to_id": to})
        };
        let why = s.refused("create_edge", one("req:a")).await;
        assert!(why.contains(relation) && why.contains("itself"), "{why}");
        let why = s
            .refused(
                "create_edges",
                json!({"edges": [one("req:b"), one("req:a")]}),
            )
            .await;
        assert!(why.contains("itself"), "{why}");
        assert_eq!(s.self_loops("req:a", relation).await, 0, "{relation}");
        s.ok("create_edge", one("req:b")).await;
    }
}
