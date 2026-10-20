//! A CHANGE TO A TEST'S CRITERIA AFTER ITS DATA ARRIVED IS REPORTED — the
//! criteria lock is CHECKED, not only recorded, over the MCP surface.
//!
//! `req:a-change-to-a-tests-criteria-after-its-data-arrived-is-reported`
//! (Anthony, 2026-10-19; option (c) of flo2-calc's
//! `dec:idea-how-a-statistical-hypothesis-test-is-designed-computed-and-read`).
//!
//! WHAT "WHEN" MEANS HERE. flo2-calc's record carries no time, and reflow2's
//! core takes no clock. The one ordering reflow2 can vouch for is its own:
//! the write that attaches the data (`add_verification` with the record's
//! `hypothesis_test`) LOCKS the criteria as they stand at that write — the α
//! node the record names, what the check VERIFIES (the hypothesis), the
//! Decisions it is GOVERNED_BY (the chosen test), and the record's own test,
//! alternative, α, H0 and H1. A criterion that differs from its lock later is
//! reported by name with both values; when the change was made is read from
//! the ChangeEvents that CHANGED it, with their dates where they carry one.
//!
//! THE CLASS: every kind of criterion — α, the hypothesis, the chosen test,
//! and the record re-attached at another α — changed after the data is
//! reported, as a gap and on the check's reading; the same change made BEFORE
//! the data is not.
//!
//! The record is a REAL flo2-calc 0.10.0 kept record (record schema 10):
//! Welch's t-test on Student's sleep data, H1 μ1 < μ2, p = 0.0397 at α = 0.05.

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

/// The rows the check's reading reports, and the gap's text.
async fn reported(s: &Session) -> (Vec<Value>, String) {
    let ver = s.ok("get_node", json!({"id": "ver:sleep-welch"})).await;
    let rows = ver["node"]["test_reading"]["criteria_changed"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let gaps = s.ok("detect_gaps", json!({})).await;
    let text = gaps
        .to_string()
        .split("criteria_changed_after_data")
        .skip(1)
        .collect::<Vec<_>>()
        .join("criteria_changed_after_data");
    (rows, text)
}

fn row<'a>(rows: &'a [Value], criterion: &str, field: &str) -> &'a Value {
    rows.iter()
        .find(|r| r["criterion"] == criterion && r["field"] == field)
        .unwrap_or_else(|| panic!("no row for {criterion} {field}: {rows:#?}"))
}

#[tokio::test]
async fn every_kind_of_criterion_changed_after_the_data_is_reported_with_both_values() {
    // α, moved after the data: 0.05 -> 0.1, so p = 0.0397 would pass either way,
    // but the goalpost moved and the design must say so.
    let s = session().await;
    designed_up_front(&s, 0.05).await;
    attach(&s, block(WELCH_LESS)).await;
    let (rows, gap) = reported(&s).await;
    assert!(rows.is_empty(), "nothing changed yet: {rows:#?}");
    assert!(gap.is_empty(), "no gap before a change: {gap}");

    s.ok(
        "add_constraint",
        json!({"id": "con:significance-level", "limit": 0.1,
               "statement": "α = 0.1, fixed before the data."}),
    )
    .await;
    // The hypothesis, reworded after the data (HARKing).
    s.ok(
        "add_requirement",
        json!({"id": "req:drug-2-adds-more-sleep",
               "statement": "Drug 2 changes the mean extra sleep."}),
    )
    .await;
    // The chosen test, swapped after the data.
    s.ok(
        "add_decision",
        json!({"id": "dec:welch-one-sided",
               "decision": "Student's pooled t-test, one-sided."}),
    )
    .await;
    // The record itself, re-run at another α and attached again.
    let mut rerun = block(WELCH_LESS);
    rerun["alpha"]["value"] = json!("0.1");
    rerun["decision"] = json!("reject H0 at α = 0.1");
    attach(&s, rerun).await;

    let (rows, gap) = reported(&s).await;
    let alpha = row(&rows, "con:significance-level", "limit");
    assert_eq!(alpha["at_data"], "0.05", "{alpha:#}");
    assert_eq!(alpha["now"], "0.1", "{alpha:#}");
    assert_eq!(alpha["attached_at"], "2026-10-19", "{alpha:#}");
    let h = row(&rows, "req:drug-2-adds-more-sleep", "statement");
    assert!(h["at_data"].as_str().unwrap().contains("exceeds"), "{h:#}");
    assert!(h["now"].as_str().unwrap().contains("changes"), "{h:#}");
    let t = row(&rows, "dec:welch-one-sided", "decision");
    assert!(t["now"].as_str().unwrap().contains("pooled"), "{t:#}");
    let rec = row(&rows, "ver:sleep-welch", "hypothesis_test.alpha");
    assert_eq!(rec["at_data"], "0.05", "{rec:#}");
    assert_eq!(rec["now"], "0.1", "{rec:#}");

    for named in [
        "con:significance-level",
        "req:drug-2-adds-more-sleep",
        "dec:welch-one-sided",
        "0.05",
        "0.1",
    ] {
        assert!(gap.contains(named), "the gap names {named}: {gap}");
    }
    // Reported, never refused: every write above landed.
    let con = s
        .ok("get_node", json!({"id": "con:significance-level"}))
        .await;
    assert_eq!(con["node"]["properties"]["limit"], 0.1, "{con:#}");
}

#[tokio::test]
async fn a_change_made_before_the_data_is_not_reported() {
    let s = session().await;
    designed_up_front(&s, 0.1).await;
    // Settled at 0.05 before any data: that is designing up front.
    s.ok(
        "add_constraint",
        json!({"id": "con:significance-level", "limit": 0.05,
               "statement": "α = 0.05, fixed before the data."}),
    )
    .await;
    attach(&s, block(WELCH_LESS)).await;
    let (rows, gap) = reported(&s).await;
    assert!(rows.is_empty(), "{rows:#?}");
    assert!(gap.is_empty(), "{gap}");
}

#[tokio::test]
async fn when_a_change_was_made_is_read_from_the_change_that_recorded_it() {
    let s = session().await;
    designed_up_front(&s, 0.05).await;
    attach(&s, block(WELCH_LESS)).await;
    s.ok(
        "add_change_event",
        json!({"id": "chg:alpha-relaxed", "name": "α relaxed to 0.1",
               "change_type": "constraint_change", "detected_at": "2026-10-21",
               "summary": "The significance level was relaxed after the data.",
               "affected": [{"node_id": "con:significance-level", "action": "modified"}]}),
    )
    .await;
    s.ok(
        "add_constraint",
        json!({"id": "con:significance-level", "limit": 0.1}),
    )
    .await;
    let (rows, _) = reported(&s).await;
    let alpha = row(&rows, "con:significance-level", "limit");
    let when = alpha["changed_when"].to_string();
    assert!(when.contains("chg:alpha-relaxed"), "{alpha:#}");
    assert!(when.contains("2026-10-21"), "{alpha:#}");
}
