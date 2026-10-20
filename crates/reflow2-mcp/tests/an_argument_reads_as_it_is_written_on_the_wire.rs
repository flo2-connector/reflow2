//! AN ARGUMENT READS AS IT IS WRITTEN, over the MCP surface — the second PR
//! of phase 6. The core's `how_an_argument_reads.rs` pins the readings; this
//! file pins what a client writes and is told:
//!
//! - `req:a-write-that-makes-a-claim-read-refuted-says-so-in-its-reply`: a
//!   write that turns a claim refuted, or a step defeated, names it in
//!   `now_refuted`, with how to keep a claim and its objector as an open
//!   dispute instead. Measured in the cogito trial's AI branch: one
//!   CONTRADICTS between rival verdicts made one read refuted, and nothing
//!   said so.
//! - the writes the other requirements need, through the tools a client
//!   calls: `statement_kind` `objection` / `reply` / `unstated_premise` /
//!   `method_rule` on add_requirement and add_constraint, rule `immediate` on
//!   add_inference with no premise, `ruling: held` on governed_by, and a
//!   conceding CONTRADICTS with `moves_to` through create_edge.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "argument-wire-probe".to_string();
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
        match self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
        {
            Ok(r) if r.is_error.unwrap_or(false) => panic!(
                "`{tool}` {shown} was refused: {}",
                r.content
                    .iter()
                    .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            Ok(r) => r.structured_content.unwrap_or(Value::Null),
            Err(e) => panic!("`{tool}` {shown} failed: {e}"),
        }
    }

    async fn statement(&self, id: &str, kind: &str, given: bool) {
        self.ok(
            "add_requirement",
            json!({"id": id, "name": format!("statement {id}"), "statement": format!("{id}."),
                   "statement_kind": kind, "taken_as_given": given}),
        )
        .await;
    }
}

#[tokio::test]
async fn a_write_that_refutes_a_claim_says_so_and_how_to_keep_a_dispute() {
    let s = session().await;
    s.statement("req:functionalist-verdict", "proposition", true)
        .await;
    s.statement("req:cartesian-verdict", "proposition", true)
        .await;
    // One way: the target now reads refuted, and the reply says so.
    let r = s
        .ok(
            "review_relations",
            json!({"node_id": "req:functionalist-verdict", "links": [{"relation": "CONTRADICTS",
                   "other_id": "req:cartesian-verdict", "evidence": "kept as the open dispute this branch is about"}]}),
        )
        .await;
    let now = &r["now_refuted"];
    assert_eq!(now[0]["id"], "req:cartesian-verdict", "{r:#}");
    assert_eq!(now[0]["reads"], "refuted", "{r:#}");
    assert_eq!(now[0]["by"][0], "req:functionalist-verdict", "{r:#}");
    assert!(
        now[0]["keep_as_dispute"]
            .as_str()
            .is_some_and(|k| k.contains("CONTRADICTS back")),
        "{r:#}"
    );
    // The other way too: both contested, nothing newly refuted.
    let r = s
        .ok(
            "review_relations",
            json!({"node_id": "req:cartesian-verdict", "links": [{"relation": "CONTRADICTS",
                   "other_id": "req:functionalist-verdict", "evidence": "the other side of the dispute"}]}),
        )
        .await;
    assert!(r.get("now_refuted").is_none(), "{r:#}");
    let n = s
        .ok("get_node", json!({"id": "req:cartesian-verdict"}))
        .await;
    assert_eq!(
        n["node"]["statement_reading"]["reads"], "contested",
        "{n:#}"
    );
}

#[tokio::test]
async fn an_objection_that_defeats_a_step_is_named_in_the_write() {
    let s = session().await;
    s.statement("req:i-am-thinking", "proposition", true).await;
    s.statement("req:i-exist", "proposition", false).await;
    s.ok(
        "add_inference",
        json!({"id": "inf:cogito", "rule": "all", "premises": ["req:i-am-thinking"],
               "conclusion": "req:i-exist"}),
    )
    .await;
    s.statement("req:obj-begs-the-question", "objection", true)
        .await;
    let r = s
        .ok(
            "create_edge",
            json!({"edge_type": "CONTRADICTS", "from_id": "req:obj-begs-the-question",
                   "to_id": "inf:cogito", "props": {"evidence": "it begs the question"}}),
        )
        .await;
    assert_eq!(r["now_refuted"][0]["id"], "inf:cogito", "{r:#}");
    assert_eq!(r["now_refuted"][0]["reads"], "defeated", "{r:#}");
}

#[tokio::test]
async fn the_argument_forms_are_written_through_the_tools() {
    let s = session().await;
    for (id, kind) in [
        ("req:o", "objection"),
        ("req:r", "reply"),
        ("req:u", "unstated_premise"),
    ] {
        s.statement(id, kind, false).await;
    }
    s.ok(
        "add_constraint",
        json!({"id": "con:method", "name": "Method of doubt",
               "statement": "Accept only what cannot be doubted.", "statement_kind": "method_rule"}),
    )
    .await;
    let n = s.ok("get_node", json!({"id": "con:method"})).await;
    assert_eq!(
        n["node"]["statement_reading"]["reads"], "taken_as_given",
        "{n:#}"
    );
    // Not an inference, on no premise.
    s.statement("req:i-exist", "proposition", false).await;
    s.ok(
        "add_inference",
        json!({"id": "inf:intuition", "rule": "immediate", "premises": [],
               "conclusion": "req:i-exist", "rule_name": "simple intuition of the mind"}),
    )
    .await;
    let n = s.ok("get_node", json!({"id": "inf:intuition"})).await;
    assert_eq!(n["node"]["inference_reading"]["verdict"], "holds", "{n:#}");
    // The owner's chosen reading of a dispute kept open.
    s.ok(
        "add_decision",
        json!({"id": "dec:which-reading", "name": "Which reading", "decision": "Kept open."}),
    )
    .await;
    s.ok(
        "governed_by",
        json!({"from_id": "inf:intuition", "to_id": "dec:which-reading", "ruling": "held"}),
    )
    .await;
    // A concession that moves the argument to another reading.
    s.ok(
        "create_edge",
        json!({"edge_type": "CONTRADICTS", "from_id": "req:r", "to_id": "req:o",
               "props": {"alignment": "conceding", "moves_to": "inf:intuition",
                         "evidence": "granted against the syllogism"}}),
    )
    .await;
}
