//! A PREDICTION READS WHETHER IT WAS STATED BEFORE THE RESULT IT IS TESTED BY —
//! `req:a-prediction-reads-whether-it-was-stated-before-the-result-it-is-tested-by`
//! (accepted 2026-10-28), on the stamps of
//! `req:reflow2-stamps-the-server-time-on-each-node-creation-and-content-change`,
//! over the MCP surface. Gap 3 of the design-a-phenomenon skill (#807):
//! nothing checked that a prediction came before its result.
//!
//! The order is read from the times the server recorded and SAID in
//! `reasoning_report.prediction_order`; it is never enforced, and no reading
//! moves because of it. A caller-typed `reported_at` cannot change it, and a
//! design with no results reads exactly as before.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use reflow2_core::foundation::store::{StampClock, format_stamp_time};
use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "prediction-order-probe".to_string();
        cfg
    }
}

struct Session {
    client: rmcp::service::RunningService<rmcp::service::RoleClient, Probe>,
}

/// A design whose clock moves one second per write, from 2026-10-28T00:00:00Z.
async fn session() -> Session {
    let at = Arc::new(AtomicI64::new(1_793_145_600));
    let clock: StampClock =
        Arc::new(move || format_stamp_time(at.fetch_add(1, Ordering::SeqCst), 0));
    serve(ReflowService::in_memory_on_clock(clock).expect("in-memory service")).await
}

async fn serve(svc: ReflowService) -> Session {
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

    async fn prediction(&self, id: &str, statement: &str) {
        self.ok(
            "add_requirement",
            json!({"id": id, "name": id, "statement": statement,
                   "statement_kind": "proposition"}),
        )
        .await;
    }

    async fn claim(&self, id: &str) {
        self.ok(
            "add_requirement",
            json!({"id": id, "name": id, "statement": format!("FICTIONAL. {id}."),
                   "statement_kind": "conjecture"}),
        )
        .await;
    }

    async fn result(&self, id: &str, reported_at: &str) {
        self.ok(
            "add_constraint",
            json!({"id": id, "name": id,
                   "statement": "FICTIONAL. Cupping fell from 7.6% to 2.1% with 2 mm Al added.",
                   "statement_kind": "evidence", "source_reliability": "B",
                   "information_credibility": "2", "reported_at": reported_at}),
        )
        .await;
    }

    async fn weigh(&self, step: &str, result: &str, prediction: &str, claim: &str) {
        self.ok(
            "add_inference",
            json!({"id": step, "premises": [result, prediction], "conclusion": claim,
                   "rule": "all", "rule_name": "the discriminating result",
                   "warrant": "Predicted a fall with filtration; a fall was seen."}),
        )
        .await;
    }

    async fn order(&self, claim: &str) -> Vec<Value> {
        let rep = self.ok("reasoning_report", json!({"claim": claim})).await;
        rep["prediction_order"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    async fn claim_reading(&self, claim: &str) -> Value {
        let rep = self.ok("reasoning_report", json!({"claim": claim})).await;
        rep["claims"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == claim)
            .cloned()
            .expect("the claim in its own read-back")
    }
}

#[tokio::test]
async fn a_prediction_recorded_before_its_result_reads_stated_before() {
    let s = session().await;
    s.prediction(
        "req:if-bh-then-falls",
        "If beam hardening, cupping falls with filtration.",
    )
    .await;
    s.claim("req:claim-bh").await;
    s.result("con:result", "2026-10-27").await;
    s.weigh(
        "inf:bears",
        "con:result",
        "req:if-bh-then-falls",
        "req:claim-bh",
    )
    .await;

    let order = s.order("req:claim-bh").await;
    assert_eq!(order.len(), 1, "{order:#?}");
    let o = &order[0];
    assert_eq!(o["prediction"], "req:if-bh-then-falls");
    assert_eq!(o["result"], "con:result");
    assert_eq!(o["bears"], "supports");
    assert_eq!(o["step"], "inf:bears");
    assert_eq!(o["order"], "stated_before_the_result", "{o:#}");
    assert_eq!(o["prediction_stated_at"], "2026-10-28T00:00:00.000000Z");
    assert!(
        o["result_recorded_at"].as_str().unwrap() > o["prediction_stated_at"].as_str().unwrap(),
        "{o:#}"
    );
    assert!(
        o["reads"]
            .as_str()
            .unwrap()
            .starts_with("stated before the result"),
        "{o:#}"
    );
}

#[tokio::test]
async fn a_prediction_recorded_after_its_result_reads_post_hoc_and_its_support_is_unchanged() {
    let early = session().await;
    early
        .prediction(
            "req:if-bh-then-falls",
            "If beam hardening, cupping falls with filtration.",
        )
        .await;
    early.claim("req:claim-bh").await;
    early.result("con:result", "2026-10-27").await;
    early
        .weigh(
            "inf:bears",
            "con:result",
            "req:if-bh-then-falls",
            "req:claim-bh",
        )
        .await;

    let late = session().await;
    late.result("con:result", "2026-10-27").await;
    late.prediction(
        "req:if-bh-then-falls",
        "If beam hardening, cupping falls with filtration.",
    )
    .await;
    late.claim("req:claim-bh").await;
    late.weigh(
        "inf:bears",
        "con:result",
        "req:if-bh-then-falls",
        "req:claim-bh",
    )
    .await;

    let order = late.order("req:claim-bh").await;
    assert_eq!(order.len(), 1, "{order:#?}");
    assert_eq!(order[0]["order"], "stated_after_the_result", "{order:#?}");
    assert!(
        order[0]["reads"]
            .as_str()
            .unwrap()
            .starts_with("stated after the result (post hoc)"),
        "{order:#?}"
    );

    // Said, never enforced: the late prediction was not refused, and the
    // claim it supports reads exactly as the early one does.
    let (a, b) = (
        early.claim_reading("req:claim-bh").await,
        late.claim_reading("req:claim-bh").await,
    );
    for field in ["truth", "proof_status", "rests_on_tests", "rests_on"] {
        assert_eq!(
            a[field], b[field],
            "{field} moved with the order: {a:#} vs {b:#}"
        );
    }
}

#[tokio::test]
async fn a_prediction_revised_after_its_result_reads_revised_after_and_keeps_its_old_wording() {
    let s = session().await;
    s.prediction(
        "req:if-bh-then-falls",
        "If beam hardening, cupping falls with filtration.",
    )
    .await;
    s.claim("req:claim-bh").await;
    s.result("con:result", "2026-10-27").await;
    s.weigh(
        "inf:bears",
        "con:result",
        "req:if-bh-then-falls",
        "req:claim-bh",
    )
    .await;
    s.ok(
        "replace_text",
        json!({"node_id": "req:if-bh-then-falls", "field": "statement",
               "old": "falls with filtration",
               "new": "falls by a third with 2 mm Al"}),
    )
    .await;

    let order = s.order("req:claim-bh").await;
    assert_eq!(order.len(), 1, "{order:#?}");
    let o = &order[0];
    assert_eq!(o["order"], "revised_after_the_result", "{o:#}");
    assert_eq!(o["prediction_stated_at"], "2026-10-28T00:00:00.000000Z");
    assert!(
        o["prediction_changed_at"].as_str().unwrap() > o["result_recorded_at"].as_str().unwrap(),
        "{o:#}"
    );
    // The wording it had when the result was recorded stays readable.
    let snap = o["earlier_wording_in"]
        .as_str()
        .unwrap_or_else(|| panic!("no snapshot named: {o:#}"));
    let kept = s.ok("get_node", json!({"id": snap})).await;
    assert!(
        kept.to_string()
            .contains("If beam hardening, cupping falls with filtration."),
        "{kept:#}"
    );
}

#[tokio::test]
async fn a_caller_typed_date_cannot_move_the_order() {
    // The result claims to be reported long before the prediction existed;
    // the server recorded it after.
    let s = session().await;
    s.prediction(
        "req:if-bh-then-falls",
        "If beam hardening, cupping falls with filtration.",
    )
    .await;
    s.claim("req:claim-bh").await;
    s.result("con:result", "1999-01-01").await;
    s.weigh(
        "inf:bears",
        "con:result",
        "req:if-bh-then-falls",
        "req:claim-bh",
    )
    .await;
    assert_eq!(
        s.order("req:claim-bh").await[0]["order"],
        "stated_before_the_result"
    );

    // And a prediction made after the result cannot date itself earlier.
    let t = session().await;
    t.result("con:result", "2999-01-01").await;
    t.ok(
        "add_requirement",
        json!({"id": "req:if-bh-then-falls", "name": "p",
               "statement": "If beam hardening, cupping falls with filtration.",
               "statement_kind": "proposition", "reported_at": "1999-01-01",
               "acted_at": "1999-01-01"}),
    )
    .await;
    t.claim("req:claim-bh").await;
    t.weigh(
        "inf:bears",
        "con:result",
        "req:if-bh-then-falls",
        "req:claim-bh",
    )
    .await;
    assert_eq!(
        t.order("req:claim-bh").await[0]["order"],
        "stated_after_the_result"
    );

    // The stamps themselves cannot be written, by any write.
    for (tool, args) in [
        (
            "create_node",
            json!({"node_type": "Requirement", "id": "req:forged",
                   "properties": {"name": "f", "statement": "f",
                                  "server_created_at": "1999-01-01T00:00:00.000000Z"}}),
        ),
        (
            "create_nodes",
            json!({"nodes": [{"node_type": "Requirement", "id": "req:forged",
                              "properties": {"name": "f", "statement": "f",
                                             "server_changed_at": "1999-01-01T00:00:00.000000Z"}}]}),
        ),
    ] {
        let refused = t.call(tool, args).await;
        let said = format!("{refused:?}");
        assert!(
            refused.is_err() || said.contains("REFUSED"),
            "{tool} accepted a stamp: {said}"
        );
        assert!(said.contains("server_"), "{tool}: {said}");
        assert!(
            t.call("get_node", json!({"id": "req:forged"}))
                .await
                .unwrap()["node"]
                .is_null(),
            "{tool} wrote the node"
        );
    }
}

#[tokio::test]
async fn a_result_that_contests_reads_the_order_of_the_claim_it_goes_against() {
    let s = session().await;
    s.result("con:result", "2026-10-27").await;
    s.claim("req:claim-sc").await;
    s.ok(
        "review_relations",
        json!({"node_id": "con:result", "links": [{"relation": "CONTRADICTS",
               "other_id": "req:claim-sc",
               "evidence": "scatter predicted a change with distance; none was seen"}]}),
    )
    .await;
    let whole = s.ok("reasoning_report", json!({})).await;
    let order = whole["prediction_order"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let o = order
        .iter()
        .find(|o| o["prediction"] == "req:claim-sc")
        .unwrap_or_else(|| panic!("{whole:#}"));
    assert_eq!(o["bears"], "contests");
    assert_eq!(o["order"], "stated_after_the_result", "{o:#}");
    // Still contested, never refuted, whatever the order.
    assert!(
        whole["still_contested"]
            .as_array()
            .is_some_and(|a| a.iter().any(|c| c["claim"] == "req:claim-sc")),
        "{whole:#}"
    );
}

#[tokio::test]
async fn a_result_carried_in_by_an_import_reads_imported_not_recorded_here() {
    let s = session().await;
    s.prediction(
        "req:if-bh-then-falls",
        "If beam hardening, cupping falls with filtration.",
    )
    .await;
    s.claim("req:claim-bh").await;
    s.result("con:result", "2026-10-27").await;
    s.weigh(
        "inf:bears",
        "con:result",
        "req:if-bh-then-falls",
        "req:claim-bh",
    )
    .await;
    let plain = s.ok("export_graph", json!({})).await;
    assert!(
        !plain.to_string().contains("server_seal"),
        "a plain export leaves the stamps in the store"
    );
    let dir = std::env::temp_dir().join(format!("stamps-layout-{}/", std::process::id()));
    let refused = s
        .call(
            "export_graph",
            json!({"path": dir.to_string_lossy(), "stamps": true}),
        )
        .await;
    assert!(
        refused.as_ref().is_err_and(|e| e.contains("item layout")),
        "the item layout never carries them: {refused:?}"
    );
    assert!(!dir.exists(), "nothing was written");
    let exported = s.ok("export_graph", json!({"stamps": true})).await;
    let document = exported
        .get("document")
        .cloned()
        .unwrap_or(exported.clone());
    assert!(
        document.to_string().contains("server_seal"),
        "the stamps travel in the export: {document:#}"
    );

    let copy = session().await;
    copy.ok("import_graph", json!({"document": document})).await;
    let order = copy.order("req:claim-bh").await;
    assert_eq!(order.len(), 1, "{order:#?}");
    assert_eq!(
        order[0]["order"], "imported_not_recorded_here",
        "{order:#?}"
    );
    assert!(order[0].get("result_recorded_at").is_none(), "{order:#?}");
}

#[tokio::test]
async fn a_design_with_no_results_reads_exactly_as_before() {
    let s = session().await;
    s.prediction("req:p", "FICTIONAL. p.").await;
    s.claim("req:q").await;
    s.ok(
        "add_inference",
        json!({"id": "inf:pq", "premises": ["req:p"], "conclusion": "req:q", "rule": "all",
               "warrant": "q follows from p."}),
    )
    .await;
    for args in [json!({"claim": "req:q"}), json!({})] {
        let rep = s.ok("reasoning_report", args).await;
        assert!(
            rep.get("prediction_order").is_none(),
            "a design with no results gains no field: {rep:#}"
        );
    }
    let node = s.ok("get_node", json!({"id": "req:p"})).await;
    assert!(
        !node.to_string().contains("server_"),
        "a node read carries no stamp: {node:#}"
    );
}
