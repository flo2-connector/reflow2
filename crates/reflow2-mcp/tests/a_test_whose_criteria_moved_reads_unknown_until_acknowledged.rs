//! A TEST WHOSE CRITERIA MOVED AFTER ITS DATA READS UNKNOWN UNTIL ACKNOWLEDGED —
//! `req:a-test-whose-criteria-moved-after-its-data-reads-unknown-until-acknowledged`
//! (Anthony, 2026-10-20, option (b) of
//! `dec:idea-should-a-criterion-changed-after-the-data-move-the-tests-reading`),
//! the third PR of phase 6.
//!
//! Until this change a criterion moved after the data was REPORTED (the
//! `criteria_changed_after_data` gap and the check's `test_reading`), while the
//! claim resting on the test went on reading `supported`. Now the test supports
//! nothing until the owner acknowledges the change with a reason; once
//! acknowledged it reads as before. Every write still lands; nothing is
//! refused. A change made before the data changes nothing.
//!
//! The record is the real flo2-calc 0.10.0 kept record the criteria-lock tests
//! use: Welch's t-test on Student's sleep data, p = 0.0397 at α = 0.05.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

const WELCH_LESS: &str = include_str!("fixtures/hypothesis_tests/sleep-welch-less.calc.json");

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "hypothesis-probe".to_string();
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
}

/// The `hypothesis_test` block of a kept record's one tested value.
fn block(record: &str) -> Value {
    let rec: Value = serde_json::from_str(record).expect("a calc record");
    rec["values"]
        .as_array()
        .expect("values")
        .iter()
        .find_map(|v| v.get("hypothesis_test").cloned())
        .expect("a value carrying hypothesis_test")
}

/// The study designed up front: the hypothesis, α as a Constraint, the test
/// chosen as a Decision, and the planned check that verifies the one and is
/// governed by the other.
async fn designed_up_front(s: &Session, alpha: f64) {
    s.ok(
        "add_requirement",
        json!({"id": "req:drug-2-adds-more-sleep", "name": "Drug 2 adds more sleep than drug 1",
               "statement": "The mean extra sleep under drug 2 exceeds the mean under drug 1."}),
    )
    .await;
    s.ok(
        "add_constraint",
        json!({"id": "con:significance-level", "name": "Significance level",
               "statement": format!("α = {alpha}, fixed before the data."), "limit": alpha,
               "quantity": "type_i_error"}),
    )
    .await;
    s.ok(
        "add_decision",
        json!({"id": "dec:welch-one-sided", "name": "The test is Welch's, one-sided",
               "decision": "Welch's two-sample t-test, one-sided (drug 2 adds more)."}),
    )
    .await;
    s.ok(
        "add_verification",
        json!({"id": "ver:sleep-welch", "name": "Welch's t-test on the sleep data",
               "method": "analysis",
               "verifies": [{"target_id": "req:drug-2-adds-more-sleep"}]}),
    )
    .await;
    s.ok(
        "governed_by",
        json!({"from_id": "ver:sleep-welch", "to_id": "dec:welch-one-sided"}),
    )
    .await;
    s.ok(
        "add_artifact",
        json!({"id": "art:sleep-welch", "name": "sleep-welch.calc.json", "artifact_type": "data",
               "location": "sleep-welch.calc.json", "checksum": "sha256:9c4a946c6a8b"}),
    )
    .await;
}

/// The data arrive: the kept record is attached with its test's block.
async fn attach(s: &Session, block: Value) -> Value {
    s.ok(
        "add_verification",
        json!({"id": "ver:sleep-welch", "status": "passing", "last_run_at": "2026-10-19",
               "findings": "flo2-calc's kept record sleep-welch.calc.json",
               "location": "sleep-welch.calc.json (art:sleep-welch)",
               "hypothesis_block": block}),
    )
    .await
}

/// The claim the test is evidence for, through one step.
async fn claim_on_the_test(s: &Session) {
    s.ok(
        "add_requirement",
        json!({"id": "req:drug-2-is-the-better-sleeper", "name": "Drug 2 is the better sleeper",
               "statement": "Prescribe drug 2 for extra sleep.", "statement_kind": "proposition"}),
    )
    .await;
    s.ok(
        "add_inference",
        json!({"id": "inf:from-the-sleep-test", "rule": "all", "premises": ["ver:sleep-welch"],
               "conclusion": "req:drug-2-is-the-better-sleeper"}),
    )
    .await;
}

async fn status(s: &Session) -> Value {
    let n = s
        .ok(
            "get_node",
            json!({"id": "req:drug-2-is-the-better-sleeper"}),
        )
        .await;
    n["node"]["proof_status"]["status"].clone()
}

async fn criteria_gap(s: &Session) -> Option<String> {
    let gaps = s.ok("detect_gaps", json!({})).await;
    gaps["gaps"]
        .as_array()
        .or_else(|| gaps["items"].as_array())
        .into_iter()
        .flatten()
        .find(|g| g["gap_source"] == "criteria_changed_after_data")
        .and_then(|g| g["id"].as_str().map(str::to_string))
}

#[tokio::test]
async fn a_criterion_moved_after_the_data_stops_the_test_counting_until_acknowledged() {
    let s = session().await;
    designed_up_front(&s, 0.05).await;
    attach(&s, block(WELCH_LESS)).await;
    claim_on_the_test(&s).await;
    assert_eq!(status(&s).await, "supported");

    // α relaxed after the data: the write lands, and the test stops counting.
    s.ok(
        "add_constraint",
        json!({"id": "con:significance-level", "limit": 0.1,
               "statement": "α = 0.1, relaxed after the data."}),
    )
    .await;
    let con = s
        .ok("get_node", json!({"id": "con:significance-level"}))
        .await;
    assert_eq!(
        con["node"]["properties"]["limit"], 0.1,
        "the write landed: {con:#}"
    );
    assert_ne!(status(&s).await, "supported");
    let step = s
        .ok("get_node", json!({"id": "inf:from-the-sleep-test"}))
        .await;
    let why = step["node"]["inference_reading"]["premises"][0]["why"].to_string();
    assert!(why.contains("criteria moved after its data"), "{step:#}");

    // The owner acknowledges the change with a reason: it reads as before.
    let gap = criteria_gap(&s).await.expect("the criteria gap is raised");
    s.ok(
        "acknowledge_gap",
        json!({"gap_id": gap, "reason": "α was set wrongly in the plan; 0.1 was agreed before unblinding"}),
    )
    .await;
    assert_eq!(status(&s).await, "supported");
}

#[tokio::test]
async fn a_criterion_changed_before_the_data_changes_nothing() {
    let s = session().await;
    designed_up_front(&s, 0.1).await;
    s.ok(
        "add_constraint",
        json!({"id": "con:significance-level", "limit": 0.05,
               "statement": "α = 0.05, fixed before the data."}),
    )
    .await;
    attach(&s, block(WELCH_LESS)).await;
    claim_on_the_test(&s).await;
    assert_eq!(status(&s).await, "supported");
}
