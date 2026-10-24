//! A premise says whether it rests on its owner's INTENT or on its DELIVERY,
//! over the MCP surface (dec:a-premise-says-whether-it-rests-on-intent-or-delivery,
//! Anthony 2026-10-24; the core's class tests are
//! crates/reflow2-core/tests/a_premise_says_whether_it_rests_on_intent_or_delivery.rs).
//!
//! The Shop sample trial recorded each design choice as an accepted Decision
//! concluded by one `add_inference` step over accepted requirements, and every
//! chain read unknown because a requirement premise read only its delivery.
//! Over the wire, as a chat meets it: `add_inference` takes `as_intent` and
//! `as_delivery` per premise, `get_node` on the step reads each premise with
//! what it was read as, `reasoning_report` lists a premise held on its
//! owner's word apart from the uncited ones, and `loop_status` no longer
//! counts the choice as having lost its reason.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "intent-probe".to_string();
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

    async fn refused(&self, tool: &str, args: Value) -> String {
        let shown = args.to_string();
        match self.call(tool, args).await {
            Ok(v) => panic!("`{tool}` {shown} was accepted: {v:#}"),
            Err(e) => e,
        }
    }

    /// The Shop sample's shape: an owner, two accepted requirements, an
    /// accepted decision a step concludes.
    async fn a_design_choice(&self) {
        self.ok(
            "add_contributor",
            json!({"id": "who:owner", "kind": "person", "name": "The Owner"}),
        )
        .await;
        for id in ["req:reusable-medium", "req:shipped-beds"] {
            self.ok(
                "add_requirement",
                json!({"id": id, "name": format!("need {id}"), "statement": "The owner's need."}),
            )
            .await;
            self.ok(
                "set_requirement_status",
                json!({"requirement_id": id, "status": "accepted", "approver": "who:owner"}),
            )
            .await;
        }
        self.ok(
            "add_decision",
            json!({"id": "dec:clay-and-coir", "name": "Clay in the fish loop, coir in the fruiting bed",
                   "decision": "Expanded clay in the fish loop; coco coir in the fruiting bed.",
                   "kind": "choice", "status": "accepted", "approver": "who:owner"}),
        )
        .await;
    }
}

fn premise<'a>(step: &'a Value, id: &str) -> &'a Value {
    step["premises"]
        .as_array()
        .and_then(|ps| ps.iter().find(|p| p["id"] == id))
        .unwrap_or_else(|| panic!("{id} is not a premise: {step:#}"))
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

/// A design choice over an accepted requirement reads as intent by default,
/// and a premise marked `as_delivery` reads as it always did; the reply and
/// `get_node` show each premise's `reads_as`.
#[tokio::test]
async fn a_decisions_step_reads_its_needs_as_intent_and_says_so_per_premise() {
    let s = session().await;
    s.a_design_choice().await;
    let reply = s
        .ok(
            "add_inference",
            json!({"id": "inf:why-clay-and-coir", "rule": "all",
                   "premises": ["req:reusable-medium", "req:shipped-beds"],
                   "as_delivery": ["req:shipped-beds"],
                   "conclusion": "dec:clay-and-coir",
                   "warrant": "Fish water returns to the fish, so its medium must shed nothing."}),
        )
        .await;
    let step = &reply["inference_reading"];
    let need = premise(step, "req:reusable-medium");
    assert_eq!(need["truth"], "holds", "{need}");
    assert_eq!(need["reads_as"], "intent", "{need}");
    let shipped = premise(step, "req:shipped-beds");
    assert_eq!(
        shipped["truth"], "unknown",
        "delivery: nothing delivers it yet: {shipped}"
    );
    assert_eq!(shipped["reads_as"], "delivery", "{shipped}");

    // A revise marks the second as intent too: the chain then holds.
    s.ok(
        "add_inference",
        json!({"id": "inf:why-clay-and-coir", "rule": "all", "premises": [],
               "as_intent": ["req:shipped-beds"], "conclusion": "dec:clay-and-coir"}),
    )
    .await;
    let node = s
        .ok("get_node", json!({"id": "inf:why-clay-and-coir"}))
        .await;
    let step = &node["node"]["inference_reading"];
    assert_eq!(step["verdict"], "holds", "{step:#}");
    assert_eq!(premise(step, "req:shipped-beds")["reads_as"], "intent");

    let report = s
        .ok("reasoning_report", json!({"claim": "dec:clay-and-coir"}))
        .await;
    assert!(
        strs(&report["held_on_the_owners_word"]).contains(&"req:reusable-medium".to_string()),
        "{report:#}"
    );
    assert!(
        !strs(&report["uncited_premises"]).contains(&"req:reusable-medium".to_string()),
        "{}",
        report["uncited_premises"]
    );
    let status = s.ok("loop_status", json!({})).await;
    assert!(
        !strs(&status["reasoning"]["decision_without_a_holding_chain_ids"])
            .contains(&"dec:clay-and-coir".to_string()),
        "{:#}",
        status["reasoning"]
    );
}

/// The marker is refused, naming the field, where it cannot apply: an id
/// that is not a premise of the step, an assumed one, a statement (which
/// reads as itself), and one named both ways. Nothing is written.
#[tokio::test]
async fn a_marker_that_cannot_apply_is_refused_naming_why() {
    let s = session().await;
    s.a_design_choice().await;
    s.ok(
        "add_requirement",
        json!({"id": "req:a-proposition", "name": "A proposition", "statement": "P.",
               "statement_kind": "proposition"}),
    )
    .await;
    for (args, says) in [
        (
            json!({"premises": ["req:reusable-medium"], "as_intent": ["req:shipped-beds"]}),
            "not a premise",
        ),
        (
            json!({"premises": ["req:reusable-medium"], "assumed": ["req:shipped-beds"],
                   "as_intent": ["req:shipped-beds"]}),
            "assumed",
        ),
        (
            json!({"premises": ["req:a-proposition"], "as_delivery": ["req:a-proposition"]}),
            "reads as itself",
        ),
        (
            json!({"premises": ["req:reusable-medium"], "as_intent": ["req:reusable-medium"],
                   "as_delivery": ["req:reusable-medium"]}),
            "both",
        ),
    ] {
        let mut args = args;
        let obj = args.as_object_mut().unwrap();
        obj.insert("id".into(), json!("inf:refused"));
        obj.insert("rule".into(), json!("all"));
        obj.insert("conclusion".into(), json!("dec:clay-and-coir"));
        let e = s.refused("add_inference", args).await;
        assert!(e.contains(says), "{says}: {e}");
        let gone = s.ok("get_node", json!({"id": "inf:refused"})).await;
        assert!(gone["node"].is_null(), "{says}: nothing written: {gone}");
    }
}
