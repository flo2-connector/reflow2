//! `derived_report` is served: every declared derived relation, counted over
//! the design by the code path its declaration names.
//!
//! `req:reflow2-declares-its-derived-relations-and-serves-a-read-that-runs-them`
//! (Anthony, 2026-09-29). The field report behind it (item I20 of
//! `art:dev-reflow2-two-agent-exercise-feedback-2026-09-29`): an agent wanted
//! the derived-to-asserted ratio of reflow2's own design and had to write a
//! ~150-line script over the export, because no served read evaluated any
//! derived relation — and reflow2 declared none. The declarations and their
//! check against the code are pinned in reflow2-core's
//! `derived_relations_are_declared_and_checked.rs`; this file pins the
//! SURFACE: the tool exists, reports all 23, counts by the code, narrows, and
//! refuses an id nobody declared.
//!
//! OBSERVED FAILING on origin/main 72917f6 (clean build, under the shared
//! build lock): all three tests failed because no tool named `derived_report`
//! was served.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "derived-relations-probe".to_string();
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

    /// One capability realized by an artifact, one not.
    async fn seeded(self) -> Self {
        for (id, desc) in [("cap:built", "built"), ("cap:idea", "not built")] {
            self.ok(
                "create_node",
                json!({"node_type": "Capability", "id": id, "props": {"name": id, "description": desc}}),
            )
            .await;
        }
        self.ok(
            "create_node",
            json!({"node_type": "Artifact", "id": "art:code", "props": {"name": "the code"}}),
        )
        .await;
        self.ok(
            "create_edge",
            json!({"edge_type": "REALIZES", "from_id": "art:code", "to_id": "cap:built"}),
        )
        .await;
        self
    }
}

fn relation<'a>(report: &'a Value, id: &str) -> &'a Value {
    report["relations"]
        .as_array()
        .expect("relations")
        .iter()
        .find(|r| r["id"] == id)
        .unwrap_or_else(|| panic!("{id} in {report}"))
}

#[tokio::test]
async fn every_declared_relation_is_reported_with_its_reading_and_count() {
    let s = session().await.seeded().await;
    let r = s.ok("derived_report", json!({})).await;
    assert_eq!(r["declared"], 23, "{r}");
    assert_eq!(r["relations"].as_array().unwrap().len(), 23);
    assert_eq!(
        r["counted"], 22,
        "the closure is declared and not counted: {r}"
    );

    let realized = relation(&r, "realized");
    assert_eq!(realized["count"], 1, "{realized}");
    assert_eq!(realized["sample"], json!(["cap:built"]));
    assert_eq!(realized["stated_over_readings"], "full");
    assert!(
        realized["reads_as"]
            .as_str()
            .unwrap()
            .contains("instance-of")
    );
    assert_eq!(realized["inference"], "deduced");

    let impact = relation(&r, "impact");
    assert!(impact.get("count").is_none(), "{impact}");
    assert!(
        impact["not_counted"]
            .as_str()
            .unwrap()
            .contains("propagate_from")
    );

    assert!(r["asserted_facts"].as_u64().unwrap() > 0);
    assert!(r["note"].as_str().unwrap().contains("nothing is stored"));
}

#[tokio::test]
async fn only_narrows_the_read_to_the_named_relations() {
    let s = session().await.seeded().await;
    let r = s
        .ok("derived_report", json!({"only": ["realized", "checked"]}))
        .await;
    let ids: Vec<&str> = r["relations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["checked", "realized"]);
    assert_eq!(relation(&r, "checked")["count"], 0, "nothing is verified");
}

#[tokio::test]
async fn a_relation_nobody_declared_is_refused_and_the_refusal_lists_the_declared_ones() {
    let s = session().await;
    let err = s
        .call("derived_report", json!({"only": ["blast_radius"]}))
        .await
        .expect_err("an undeclared relation is refused, never answered empty");
    assert!(err.contains("blast_radius"), "{err}");
    assert!(err.contains("impact") && err.contains("delivered"), "{err}");
}
