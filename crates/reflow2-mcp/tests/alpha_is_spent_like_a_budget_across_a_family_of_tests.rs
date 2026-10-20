//! α IS SPENT LIKE A BUDGET ACROSS A FAMILY OF TESTS — over the MCP surface.
//!
//! `req:alpha-is-spent-like-a-budget-across-a-family-of-tests` (Anthony,
//! 2026-10-19; option (d) of flo2-calc's
//! `dec:idea-how-a-statistical-hypothesis-test-is-designed-computed-and-read`).
//!
//! MOSTLY A RECIPE OVER WHAT EXISTS, as the requirement says. The family's
//! error rate is a Constraint with a `limit`; each test spends its α through
//! a CONSTRAINS `contribution`, and `budget_report` sums them. What the
//! rollup cannot do by itself, and the code adds:
//!
//! - a test recorded from its kept record SPENDS what it was tested at: when
//!   the record's α names the family's Constraint (`alpha.source.design_node`),
//!   `add_verification` draws that CONSTRAINS with the record's α, so a test
//!   added later is in the sum and visibly over budget, never forgotten;
//! - a test TESTED at more than it spends (a share drawn by hand at α/k, a
//!   record run at α) breaks the family's control, and its reading says so.
//!
//! THE CLASS: a family of k tests at Bonferroni shares stays within; any test
//! added later is in the sum; any test tested above its share reads unknown.
//!
//! The records are REAL flo2-calc 0.10.0 kept records (record schema 10): the
//! paired t-test on Student's sleep data at α = 0.0166 (a third of 0.05,
//! rounded DOWN, its source the family's Constraint), and Welch's test at
//! α = 0.05.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

const PAIRED: &str = include_str!("fixtures/hypothesis_tests/sleep-paired-less.calc.json");
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

async fn family(s: &Session) {
    s.ok(
        "add_constraint",
        json!({"id": "con:family-alpha", "name": "Family-wise error rate",
               "statement": "Across the family of tests, the chance of any false rejection is at most 0.05 (Bonferroni).",
               "limit": 0.05, "quantity": "family_wise_error_rate", "composition": "sum"}),
    )
    .await;
}

/// One test of the family, attached from its kept record.
async fn test(s: &Session, n: usize, block: Value) -> Value {
    let id = format!("ver:family-test-{n}");
    let art = format!("art:family-test-{n}");
    s.ok(
        "add_artifact",
        json!({"id": art, "name": format!("family-test-{n}.calc.json"), "artifact_type": "data",
               "location": format!("family-test-{n}.calc.json"), "checksum": "sha256:b80a0280"}),
    )
    .await;
    s.ok(
        "add_verification",
        json!({"id": id, "name": format!("Test {n} of the family"), "method": "analysis",
               "status": "passing", "last_run_at": "2026-10-19",
               "findings": "flo2-calc's kept record",
               "location": format!("family-test-{n}.calc.json ({art})"),
               "hypothesis_block": block}),
    )
    .await
}

#[tokio::test]
async fn a_family_of_three_at_bonferroni_shares_is_within_and_a_fourth_is_over() {
    let s = session().await;
    family(&s).await;
    for n in 1..=3 {
        let reply = test(&s, n, block(PAIRED)).await;
        let spent = &reply["hypothesis_test_recorded"]["alpha_spent"];
        assert_eq!(spent["constraint_id"], "con:family-alpha", "{reply:#}");
        assert_eq!(spent["contribution"], 0.0166, "{reply:#}");
    }
    let r = s
        .ok(
            "budget_report",
            json!({"constraint_id": "con:family-alpha"}),
        )
        .await;
    assert_eq!(r["verdict"], "within", "{r:#}");
    let total = r["total"].as_f64().unwrap();
    assert!((total - 0.0498).abs() < 1e-12, "{r:#}");

    // A test added later spends too, and the family is over.
    test(&s, 4, block(PAIRED)).await;
    let r = s
        .ok(
            "budget_report",
            json!({"constraint_id": "con:family-alpha"}),
        )
        .await;
    assert_eq!(r["verdict"], "exceeded", "{r:#}");
    assert!(
        r.to_string().contains("ver:family-test-4"),
        "the report names the test that overspent: {r:#}"
    );
}

#[tokio::test]
async fn a_test_tested_above_its_share_reads_unknown_and_says_why() {
    let s = session().await;
    family(&s).await;
    // The share was drawn by hand at a third of the family's 0.05 ...
    s.ok(
        "add_verification",
        json!({"id": "ver:family-test-1", "name": "Test 1 of the family", "method": "analysis"}),
    )
    .await;
    s.ok(
        "constrains",
        json!({"constraint_id": "con:family-alpha", "target_id": "ver:family-test-1",
               "contribution": 0.0166, "basis": "estimated", "source": "who:owner"}),
    )
    .await;
    // ... and the test was run at the whole 0.05.
    let mut at_whole = block(WELCH_LESS);
    at_whole["alpha"]["source"] = json!({"design_node": "con:family-alpha"});
    let reply = test(&s, 1, at_whole).await;
    assert_eq!(
        reply["hypothesis_test_recorded"]["alpha_spent"]["contribution"], 0.0166,
        "a share drawn by hand is kept, never overwritten: {reply:#}"
    );
    s.ok(
        "add_requirement",
        json!({"id": "req:claim", "name": "The claim the test supports",
               "statement": "Drug 2 adds more sleep than drug 1."}),
    )
    .await;
    s.ok(
        "add_inference",
        json!({"id": "inf:claim", "rule": "all", "premises": ["ver:family-test-1"],
               "conclusion": "req:claim"}),
    )
    .await;
    let step = s.ok("get_node", json!({"id": "inf:claim"})).await;
    let premise = &step["node"]["inference_reading"]["premises"][0];
    assert_eq!(premise["truth"], "unknown", "{premise:#}");
    let why = premise["why"].as_str().unwrap();
    assert!(why.contains("0.05") && why.contains("0.0166"), "{why}");
    assert!(why.contains("con:family-alpha"), "{why}");
}

#[tokio::test]
async fn the_served_recipe_says_how_alpha_is_spent() {
    let s = session().await;
    let skill = s.ok("get_skill", json!({"name": "design-a-proof"})).await;
    let text = skill.to_string();
    for part in [
        "Bonferroni",
        "Holm",
        "Benjamini",
        "budget_report",
        "Round each share DOWN",
        "hypothesis_test",
    ] {
        assert!(text.contains(part), "the recipe names `{part}`");
    }
}
