//! EVIDENCE AND NATURAL DEDUCTION, over the MCP surface — the fourth PR of
//! phase 6. The core's `evidence_and_natural_deduction.rs` pins the readings;
//! this file pins what a client writes and is told:
//!
//! - `req:evidence-carries-its-source-grade-and-confidence-and-is-not-intent`:
//!   add_requirement / add_constraint take `statement_kind` `evidence` and
//!   `judgement` with `source_reliability`, `information_credibility`,
//!   `reported_at` and `confidence`; a report documented by a pinned source
//!   reads `cited_as_evidence`, and what rests on it `supported`.
//! - `req:natural-deduction-gaps-from-the-logic-rung-are-closed`: add_inference
//!   takes `rule_node`, `establishes`, `conclusion_form` and a premise named
//!   twice, and refuses a rule named as a premise too.
//! - `req:a-write-that-makes-a-claim-read-refuted-says-so-in-its-reply`, the
//!   rest of it: create_edges, draw_edges and add_decision's `related_to` name
//!   what they turn refuted in `now_refuted`, as create_edge and
//!   review_relations have since PR 2.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "evidence-wire-probe".to_string();
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
            Err(e) => Err(e.to_string()),
        }
    }

    async fn ok(&self, tool: &str, args: Value) -> Value {
        let shown = args.to_string();
        self.call(tool, args)
            .await
            .unwrap_or_else(|e| panic!("`{tool}` {shown} was refused: {e}"))
    }

    async fn statement(&self, id: &str, kind: &str, given: bool) {
        self.ok(
            "add_requirement",
            json!({"id": id, "name": format!("statement {id}"), "statement": format!("{id}."),
                   "statement_kind": kind, "taken_as_given": given}),
        )
        .await;
    }

    async fn reading_of(&self, step: &str) -> Value {
        let r = self.ok("reasoning_report", json!({})).await;
        r["claims"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|c| c["support"].as_array().cloned().unwrap_or_default())
            .map(|s| {
                if s.get("reading").is_some() {
                    s["reading"].clone()
                } else {
                    s
                }
            })
            .find(|r| r["inference"] == step)
            .unwrap_or_else(|| panic!("{step} in the read-back: {r:#}"))
    }
}

#[tokio::test]
async fn evidence_is_written_with_its_grade_and_cited_as_evidence() {
    let s = session().await;
    s.ok(
        "add_requirement",
        json!({"id": "req:r2", "name": "R2: emissions at 9.4 GHz",
               "statement": "A FICTIONAL intercept says the radar emits at 9.4 GHz.",
               "statement_kind": "evidence", "source_reliability": "B",
               "information_credibility": "2", "reported_at": "2025",
               "source": "FICTIONAL report R2"}),
    )
    .await;
    let n = s.ok("get_node", json!({"id": "req:r2"})).await;
    assert_eq!(
        n["node"]["statement_reading"]["reads"], "evidence_unverified",
        "{n:#}"
    );
    for (k, v) in [
        ("source_reliability", "B"),
        ("information_credibility", "2"),
        ("reported_at", "2025"),
    ] {
        assert_eq!(n["node"]["properties"][k], v, "{n:#}");
    }
    s.ok(
        "add_artifact",
        json!({"id": "art:r2", "name": "FICTIONAL report R2", "location": "reports/r2.pdf",
               "artifact_type": "document", "status": "realized",
               "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000002"}),
    )
    .await;
    s.ok(
        "create_edge",
        json!({"edge_type": "DOCUMENTS", "from_id": "art:r2", "to_id": "req:r2"}),
    )
    .await;
    let n = s.ok("get_node", json!({"id": "req:r2"})).await;
    assert_eq!(
        n["node"]["statement_reading"]["reads"], "cited_as_evidence",
        "{n:#}"
    );
    s.ok(
        "add_requirement",
        json!({"id": "req:a1", "name": "A1: the radar works in X band",
               "statement": "The radar works in X band.", "statement_kind": "judgement",
               "confidence": "moderate"}),
    )
    .await;
    s.ok(
        "add_inference",
        json!({"id": "inf:a1", "rule": "all", "premises": ["req:r2"], "conclusion": "req:a1"}),
    )
    .await;
    let n = s.ok("get_node", json!({"id": "req:a1"})).await;
    assert_eq!(
        n["node"]["statement_reading"]["reads"], "supported",
        "{n:#}"
    );
    let r = s.ok("reasoning_report", json!({})).await;
    let ev = r["evidence"].as_array().expect("an evidence list");
    let a1 = ev.iter().find(|e| e["id"] == "req:a1").expect("A1 listed");
    assert_eq!(a1["grade"]["confidence"], "moderate", "{r:#}");
    let r2 = ev.iter().find(|e| e["id"] == "req:r2").expect("R2 listed");
    assert_eq!(r2["reads"], "cited_as_evidence", "{r:#}");
    assert_eq!(s.reading_of("inf:a1").await["holds_as"], "supported");
    // A grade outside its scale is refused, naming the scale.
    let bad = s
        .call(
            "add_requirement",
            json!({"id": "req:r9", "name": "R9", "statement": "R9.",
                   "statement_kind": "evidence", "source_reliability": "G"}),
        )
        .await;
    assert!(bad.is_err(), "{bad:?}");
}

#[tokio::test]
async fn a_step_is_written_with_its_rule_node_relation_form_and_a_premise_twice() {
    let s = session().await;
    s.statement("req:a", "axiom", true).await;
    s.statement("req:a-and-a", "lemma", false).await;
    s.ok(
        "add_constraint",
        json!({"id": "con:rule-and-intro", "name": "Rule ∧I", "statement": "From A and B, infer A ∧ B.",
               "statement_kind": "axiom", "taken_as_given": true}),
    )
    .await;
    let refused = s
        .call(
            "add_inference",
            json!({"id": "inf:bad", "rule": "all", "premises": ["req:a", "con:rule-and-intro"],
                   "rule_node": "con:rule-and-intro", "conclusion": "req:a-and-a"}),
        )
        .await;
    assert!(
        refused
            .as_ref()
            .is_err_and(|e| e.contains("never one of its premises")),
        "{refused:?}"
    );
    s.ok(
        "add_inference",
        json!({"id": "inf:and-intro", "rule": "all", "premises": ["req:a", "req:a"],
               "conclusion": "req:a-and-a", "rule_name": "∧I", "rule_node": "con:rule-and-intro",
               "establishes": "derivability", "conclusion_form": "conjunction"}),
    )
    .await;
    let r = s.reading_of("inf:and-intro").await;
    assert_eq!(r["verdict"], "holds", "{r:#}");
    assert_eq!(r["rule_node"]["id"], "con:rule-and-intro", "{r:#}");
    assert_eq!(r["rule_node"]["truth"], "holds", "{r:#}");
    assert_eq!(r["establishes"], "derivability", "{r:#}");
    assert_eq!(r["conclusion_form"], "conjunction", "{r:#}");
    assert_eq!(r["premises"][0]["uses"], 2, "{r:#}");
    // A what-if on the rule reaches the step that uses it.
    let w = s
        .ok(
            "propagate_from",
            json!({"seed_ids": ["con:rule-and-intro"]}),
        )
        .await;
    assert!(w.to_string().contains("inf:and-intro"), "{w:#}");
}

#[tokio::test]
async fn create_edges_says_what_it_turns_refuted() {
    let s = session().await;
    s.statement("req:claim", "proposition", true).await;
    s.statement("req:obj", "objection", true).await;
    let r = s
        .ok(
            "create_edges",
            json!({"edges": [{"edge_type": "CONTRADICTS", "from_id": "req:obj", "to_id": "req:claim",
                              "props": {"evidence": "a counterexample"}}]}),
        )
        .await;
    assert_eq!(r["now_refuted"][0]["id"], "req:claim", "{r:#}");
    assert_eq!(r["now_refuted"][0]["reads"], "refuted", "{r:#}");
}

#[tokio::test]
async fn add_decisions_related_to_says_what_it_turns_refuted() {
    let s = session().await;
    s.statement("req:obj", "objection", true).await;
    let r = s
        .ok(
            "add_decision",
            json!({"id": "dec:idea-push", "name": "Push the one to save the five",
                   "decision": "Push.", "kind": "choice",
                   "related_to": [{"relation": "CONTRADICTS", "other_id": "req:obj", "incoming": true,
                                   "evidence": "the objection holds against it"}]}),
        )
        .await;
    assert_eq!(r["now_refuted"][0]["id"], "dec:idea-push", "{r:#}");
    assert_eq!(r["now_refuted"][0]["reads"], "refuted", "{r:#}");
}

/// A cited counterexample nobody has read reads `cited_unverified`: it holds
/// only if it says what it is recalled to say, so it contests the claim it
/// opposes and does not refute it. Documenting its source through draw_edges
/// makes it a plain citation, which refutes — and the batch's reply says so.
#[tokio::test]
async fn draw_edges_says_what_it_turns_refuted() {
    let s = session().await;
    s.statement("req:claim", "proposition", true).await;
    s.statement("req:counterexample", "theorem", true).await;
    s.ok(
        "create_edge",
        json!({"edge_type": "CONTRADICTS", "from_id": "req:counterexample", "to_id": "req:claim",
               "props": {"evidence": "a published counterexample"}}),
    )
    .await;
    let before = s.ok("get_node", json!({"id": "req:claim"})).await;
    assert_ne!(
        before["node"]["statement_reading"]["reads"], "refuted",
        "{before:#}"
    );
    s.ok(
        "add_artifact",
        json!({"id": "art:paper", "name": "the paper", "location": "papers/counterexample.pdf",
               "artifact_type": "document", "status": "realized",
               "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000003"}),
    )
    .await;
    let r = s
        .ok(
            "draw_edges",
            json!({"edges": [{"tool": "documents",
                              "arguments": {"artifact_id": "art:paper", "target_id": "req:counterexample"}}]}),
        )
        .await;
    let claim = s.ok("get_node", json!({"id": "req:claim"})).await;
    assert_eq!(r["now_refuted"][0]["id"], "req:claim", "{r:#}\n{claim:#}");
    assert_eq!(r["now_refuted"][0]["reads"], "refuted", "{r:#}");
}
