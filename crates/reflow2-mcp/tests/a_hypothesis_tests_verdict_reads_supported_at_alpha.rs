//! A HYPOTHESIS TEST'S VERDICT READS "SUPPORTED AT α", NEVER `holds`, `shown`
//! OR `proven` — over the MCP surface.
//!
//! `req:a-hypothesis-tests-verdict-reads-as-supported-at-alpha-never-as-shown`
//! (Anthony, 2026-10-19; option (b) of flo2-calc's
//! `dec:idea-how-a-statistical-hypothesis-test-is-designed-computed-and-read`).
//!
//! The records are REAL flo2-calc 0.10.0 kept records (record schema 10),
//! computed from Student's sleep data (Cushny and Peebles, 1905) on the branch
//! `feat/named-tests-and-power` at 140d14c, and kept byte for byte in
//! `fixtures/hypothesis_tests/`:
//!
//! - `sleep-welch-less.calc.json` — Welch's two-sample t-test, H1 μ1 < μ2,
//!   p = 0.0397 at α = 0.05: rejects H0;
//! - `sleep-welch-two-sided.calc.json` — the same data two-sided, p = 0.0794:
//!   does not reject H0.
//!
//! THE CLASS: whatever shape a claim rests on a significant test in — the one
//! step, a step a person confirmed, a chain through a lemma, a step beside a
//! taken-as-given premise — the claim reads `supported` and its reading names
//! α, the p-value, the test and n; never `holds`, `shown` or `proven`. A
//! non-significant result reads "not rejected at α" and supports nothing; it
//! never reads "H0 true". The test's assumptions are records an objection can
//! attack.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

const WELCH_LESS: &str = include_str!("fixtures/hypothesis_tests/sleep-welch-less.calc.json");
const WELCH_TWO_SIDED: &str =
    include_str!("fixtures/hypothesis_tests/sleep-welch-two-sided.calc.json");

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

/// A study designed up front, then its data: the hypothesis, α fixed as a
/// Constraint, the planned check, and then the kept record attached with the
/// test's block. Returns the reply of the attaching `add_verification`.
async fn study(s: &Session, record: &str) -> Value {
    s.ok(
        "add_requirement",
        json!({"id": "req:drug-2-adds-more-sleep", "name": "Drug 2 adds more sleep than drug 1",
               "statement": "The mean extra sleep under drug 2 exceeds the mean under drug 1."}),
    )
    .await;
    s.ok(
        "add_constraint",
        json!({"id": "con:significance-level", "name": "Significance level",
               "statement": "α = 0.05, fixed before the data.", "limit": 0.05,
               "quantity": "type_i_error"}),
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
        "add_artifact",
        json!({"id": "art:sleep-welch", "name": "sleep-welch.calc.json", "artifact_type": "data",
               "location": "sleep-welch.calc.json", "checksum": "sha256:9c4a946c6a8b"}),
    )
    .await;
    s.ok(
        "add_verification",
        json!({"id": "ver:sleep-welch", "status": "passing", "last_run_at": "2026-10-19",
               "findings": "flo2-calc's kept record sleep-welch.calc.json",
               "location": "sleep-welch.calc.json (art:sleep-welch)",
               "hypothesis_block": block(record)}),
    )
    .await
}

fn reads_never_settled(label: &str, v: &Value) {
    let text = v.to_string();
    for word in ["\"proven\"", "\"shown\""] {
        assert!(!text.contains(word), "{label}: reads {word}: {v:#}");
    }
}

/// The reading names α, the p-value, the test and n.
fn names_the_test(label: &str, why: &str) {
    for part in ["α = 0.05", "p = 0.0397", "Welch", "n = 10, 10"] {
        assert!(why.contains(part), "{label}: `{part}` missing from: {why}");
    }
}

async fn claim_reads_supported(s: &Session, label: &str, claim: &str) {
    let node = s.ok("get_node", json!({"id": claim})).await;
    let ps = &node["node"]["proof_status"];
    assert_eq!(ps["status"], "supported", "{label}: {node:#}");
    names_the_test(label, ps["why"].as_str().unwrap_or_default());
    reads_never_settled(label, ps);
    let report = s.ok("reasoning_report", json!({"claim": claim})).await;
    let row = report["claims"]
        .as_array()
        .and_then(|cs| cs.iter().find(|c| c["id"] == claim))
        .unwrap_or_else(|| panic!("{label}: {claim} not read back: {report:#}"));
    assert_eq!(row["truth"], "supported", "{label}: {row:#}");
    assert_ne!(row["truth"], "holds", "{label}");
    names_the_test(label, row["why"].as_str().unwrap_or_default());
}

#[tokio::test]
async fn a_claim_resting_on_a_significant_test_reads_supported_at_alpha_in_every_shape() {
    // 1. The one step: the test is the claim's only premise.
    let s = session().await;
    study(&s, WELCH_LESS).await;
    s.ok(
        "add_inference",
        json!({"id": "inf:drug-2", "rule": "all", "premises": ["ver:sleep-welch"],
               "conclusion": "req:drug-2-adds-more-sleep"}),
    )
    .await;
    claim_reads_supported(&s, "one step", "req:drug-2-adds-more-sleep").await;

    // The step's own reading names the test on its premise, and says it is
    // evidence, not proof.
    let step = s.ok("get_node", json!({"id": "inf:drug-2"})).await;
    let premise = step["node"]["inference_reading"]["premises"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "ver:sleep-welch")
        .cloned()
        .unwrap();
    let why = premise["why"].as_str().unwrap();
    names_the_test("the step's premise", why);
    assert!(why.contains("supported at α = 0.05"), "{why}");
    assert!(why.contains("evidence, not proof"), "{why}");

    // 2. A person confirms the step: a deductive line would read proven; a
    // test's never does.
    s.ok(
        "add_inference",
        json!({"id": "inf:drug-2", "rule": "all", "premises": ["ver:sleep-welch"],
               "conclusion": "req:drug-2-adds-more-sleep", "confirmed_how": "person",
               "confirmed_by": "who:owner", "confirmed_at": "2026-10-19T12:00:00Z"}),
    )
    .await;
    claim_reads_supported(&s, "confirmed step", "req:drug-2-adds-more-sleep").await;

    // 3. A chain: the claim is a lemma, and a theorem rests on it beside a
    // premise taken as given.
    s.ok(
        "add_constraint",
        json!({"id": "con:dose-is-safe", "name": "The dose is safe",
               "statement": "Both doses are within the safe range.", "taken_as_given": true}),
    )
    .await;
    s.ok(
        "add_requirement",
        json!({"id": "req:prescribe-drug-2", "name": "Prescribe drug 2 for sleep",
               "statement": "Drug 2 is the better choice for sleep."}),
    )
    .await;
    s.ok(
        "add_inference",
        json!({"id": "inf:prescribe", "rule": "all",
               "premises": ["req:drug-2-adds-more-sleep", "con:dose-is-safe"],
               "conclusion": "req:prescribe-drug-2", "confirmed_how": "person",
               "confirmed_by": "who:owner", "confirmed_at": "2026-10-19T12:00:00Z"}),
    )
    .await;
    claim_reads_supported(&s, "a chain", "req:prescribe-drug-2").await;

    // 4. A statement that says its kind reads `supported` too.
    s.ok(
        "add_requirement",
        json!({"id": "req:drug-2-adds-more-sleep", "statement_kind": "proposition"}),
    )
    .await;
    let node = s
        .ok("get_node", json!({"id": "req:drug-2-adds-more-sleep"}))
        .await;
    assert_eq!(
        node["node"]["statement_reading"]["reads"], "supported",
        "{node:#}"
    );
}

#[tokio::test]
async fn a_non_significant_test_reads_not_rejected_and_never_h0_true() {
    let s = session().await;
    study(&s, WELCH_TWO_SIDED).await;
    s.ok(
        "add_inference",
        json!({"id": "inf:drug-2", "rule": "all", "premises": ["ver:sleep-welch"],
               "conclusion": "req:drug-2-adds-more-sleep"}),
    )
    .await;
    let step = s.ok("get_node", json!({"id": "inf:drug-2"})).await;
    let premise = &step["node"]["inference_reading"]["premises"][0];
    assert_eq!(premise["truth"], "unknown", "{premise:#}");
    let why = premise["why"].as_str().unwrap();
    assert!(why.contains("not rejected at α = 0.05"), "{why}");
    assert!(why.contains("p = 0.0794"), "{why}");
    assert!(!why.contains("H0 true") || why.contains("never"), "{why}");
    let ver = s.ok("get_node", json!({"id": "ver:sleep-welch"})).await;
    assert_eq!(
        ver["node"]["test_reading"]["reads"], "not_rejected",
        "{ver:#}"
    );
    // The claim it was meant to support is unsupported, never refuted and
    // never "supported".
    let claim = s
        .ok("get_node", json!({"id": "req:drug-2-adds-more-sleep"}))
        .await;
    assert_eq!(
        claim["node"]["proof_status"]["status"], "unproven",
        "{claim:#}"
    );
    reads_never_settled("not rejected", &claim["node"]["proof_status"]);
}

#[tokio::test]
async fn the_tests_assumptions_are_records_an_objection_can_attack() {
    let s = session().await;
    let attached = study(&s, WELCH_LESS).await;
    let recorded = &attached["hypothesis_test_recorded"];
    let assumptions: Vec<String> = recorded["assumptions"]
        .as_array()
        .unwrap_or_else(|| panic!("the reply names the assumptions: {attached:#}"))
        .iter()
        .map(|a| a["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(assumptions.len(), 3, "{attached:#}");
    let normality = recorded["assumptions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["text"].as_str().unwrap_or_default().contains("normal"))
        .and_then(|a| a["id"].as_str())
        .unwrap()
        .to_string();
    let node = s.ok("get_node", json!({"id": normality})).await;
    assert_eq!(node["node"]["node_type"], "Constraint", "{node:#}");
    assert_eq!(
        node["node"]["properties"]["taken_as_given"], true,
        "{node:#}"
    );

    s.ok(
        "add_inference",
        json!({"id": "inf:drug-2", "rule": "all", "premises": ["ver:sleep-welch"],
               "conclusion": "req:drug-2-adds-more-sleep"}),
    )
    .await;
    let ver = s.ok("get_node", json!({"id": "ver:sleep-welch"})).await;
    let listed: Vec<&str> = ver["node"]["test_reading"]["assumptions"]
        .as_array()
        .unwrap_or_else(|| panic!("{ver:#}"))
        .iter()
        .filter_map(|a| a["id"].as_str())
        .collect();
    for a in &assumptions {
        assert!(listed.contains(&a.as_str()), "{a} not read: {ver:#}");
    }

    // An objection raised against normality, neither established nor
    // answered: the claim is contested, not supported.
    s.ok(
        "add_requirement",
        json!({"id": "req:skewed", "name": "The extra sleep is skewed",
               "statement": "Two patients gain over 3 h; the data look right-skewed."}),
    )
    .await;
    s.ok(
        "create_edge",
        json!({"edge_type": "CONTRADICTS", "from_id": "req:skewed", "to_id": normality}),
    )
    .await;
    let claim = s
        .ok("get_node", json!({"id": "req:drug-2-adds-more-sleep"}))
        .await;
    let ps = &claim["node"]["proof_status"];
    assert_ne!(ps["status"], "supported", "{claim:#}");
    reads_never_settled("objection to an assumption", ps);
    let report = s
        .ok(
            "reasoning_report",
            json!({"claim": "req:drug-2-adds-more-sleep"}),
        )
        .await;
    let row = report["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "req:drug-2-adds-more-sleep")
        .cloned()
        .unwrap();
    assert_eq!(row["truth"], "unknown", "{row:#}");
    // The step says which assumption no longer holds.
    let step = s.ok("get_node", json!({"id": "inf:drug-2"})).await;
    let why = step["node"]["inference_reading"]["premises"][0]["why"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        why.contains(&normality) && why.contains("supports nothing"),
        "the step names the attacked assumption: {why}"
    );
}

#[tokio::test]
async fn a_significant_test_as_an_objection_contests_and_never_refutes() {
    let s = session().await;
    study(&s, WELCH_LESS).await;
    s.ok(
        "add_requirement",
        json!({"id": "req:no-difference", "name": "The drugs give the same extra sleep",
               "statement": "Drug 1 and drug 2 add the same mean sleep.",
               "taken_as_given": true, "distinct_from": ["req:drug-2-adds-more-sleep"]}),
    )
    .await;
    s.ok(
        "create_edge",
        json!({"edge_type": "CONTRADICTS", "from_id": "ver:sleep-welch",
               "to_id": "req:no-difference"}),
    )
    .await;
    let report = s
        .ok("reasoning_report", json!({"claim": "req:no-difference"}))
        .await;
    let row = report["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "req:no-difference")
        .cloned()
        .unwrap_or_else(|| panic!("{report:#}"));
    assert_eq!(row["truth"], "unknown", "contested, never refuted: {row:#}");
    let objection = &row["objections"][0];
    assert_eq!(objection["standing"], "stands", "{row:#}");
    assert!(
        objection["why"].as_str().unwrap().contains("evidence"),
        "{objection:#}"
    );
}

#[tokio::test]
async fn a_block_missing_what_the_reading_needs_is_refused_by_its_path_and_nothing_is_written() {
    let s = session().await;
    let mut b = block(WELCH_LESS);
    b.as_object_mut().unwrap().remove("reject_h0");
    let refused = s
        .call(
            "add_verification",
            json!({"id": "ver:no-verdict", "name": "A test with no verdict",
                   "method": "analysis", "hypothesis_block": b}),
        )
        .await
        .expect_err("a block with no reject_h0 is refused");
    assert!(refused.contains("add_verification"), "{refused}");
    assert!(refused.contains("hypothesis_block.reject_h0"), "{refused}");
    let node = s.ok("get_node", json!({"id": "ver:no-verdict"})).await;
    assert!(node["node"].is_null(), "nothing was written: {node:#}");
}
