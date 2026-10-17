//! A PROOF COUNTS WHAT IT SHOULD, over the MCP surface — phase 1 of the fixes
//! Anthony accepted on 2026-10-17 after the primes trial on flo2.io
//! (fact:the-primes-trial-rung-1-ran-on-flo2-io-and-its-friction-names-what-counts-2026-10-17).
//! The core's file of the same name pins the readings; this one pins what a
//! client calls and reads:
//!
//! - `add_verification` takes `computed`, and a computed check with no kept
//!   record reads `asserted` as a premise
//!   (req:a-computed-fact-counts-only-through-its-kept-record);
//! - `add_constraint` and `add_requirement` take `taken_as_given`
//!   (req:the-loop-sees-unconfirmed-steps-and-uncited-premises), and
//!   `loop_status` carries `reasoning`;
//! - the snags of req:the-primes-trials-small-tool-snags-are-fixed:
//!   `gaps_to_prompts` and `gap_to_prompt` need only the gap's `id` (F8);
//!   `describe_schema` keeps to `budget_chars` (F3); the `verifies` field
//!   says its shape in its first sentence (F20); a capped `get_instructions`
//!   carries a usable first section (F1).

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "proof-probe".to_string();
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

    async fn axiom(&self, id: &str, given: bool) {
        let mut args =
            json!({"id": id, "name": format!("axiom {id}"), "statement": "A definition."});
        if given {
            args["taken_as_given"] = json!(true);
        }
        self.ok("add_constraint", args).await;
    }

    async fn statement(&self, id: &str) {
        self.ok(
            "add_requirement",
            json!({"id": id, "name": format!("statement {id}"), "statement": "A statement."}),
        )
        .await;
    }

    async fn step(&self, id: &str, premises: &[&str], conclusion: &str) {
        self.ok(
            "add_inference",
            json!({"id": id, "rule": "all", "premises": premises, "conclusion": conclusion}),
        )
        .await;
    }
}

#[tokio::test]
async fn a_computed_check_with_no_kept_record_reads_asserted_over_the_surface() {
    let s = session().await;
    s.axiom("con:def-prime", false).await;
    s.statement("req:counterexample-n40").await;
    s.ok(
        "add_verification",
        json!({
            "id": "ver:calc-n40", "name": "n² + n + 41 at n = 40", "method": "analysis",
            "computed": true, "status": "passing", "findings": "1681 = 41·41",
            "last_run_at": "2026-10-17",
        }),
    )
    .await;
    s.step(
        "inf:counterexample",
        &["ver:calc-n40", "con:def-prime"],
        "req:counterexample-n40",
    )
    .await;
    let node = s.ok("get_node", json!({"id": "inf:counterexample"})).await;
    let premise = node["node"]["inference_reading"]["premises"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "ver:calc-n40")
        .cloned()
        .unwrap();
    assert_eq!(premise["truth"], "unknown", "{premise:#}");
    assert!(
        premise["why"].as_str().unwrap().contains("asserted"),
        "{premise:#}"
    );

    // Its kept record, named in the location: it holds, and it is cited.
    s.ok(
        "add_artifact",
        json!({"id": "art:calc-n40", "name": "n40.calc.json", "artifact_type": "data",
               "location": "n40.calc.json", "checksum": "sha256:b2fa"}),
    )
    .await;
    s.ok(
        "add_verification",
        json!({"id": "ver:calc-n40", "location": "n40.calc.json (art:calc-n40)"}),
    )
    .await;
    let node = s.ok("get_node", json!({"id": "inf:counterexample"})).await;
    assert_eq!(
        node["node"]["inference_reading"]["verdict"], "holds",
        "{node:#}"
    );
    let report = s.ok("reasoning_report", json!({})).await;
    let uncited = report["uncited_premises"].as_array().unwrap();
    assert!(!uncited.contains(&json!("ver:calc-n40")), "{report:#}");
}

#[tokio::test]
async fn taken_as_given_and_the_reasonings_debt_reach_the_surface() {
    let s = session().await;
    s.axiom("con:def-prime", true).await;
    s.axiom("con:axiom-arith", false).await;
    s.statement("req:lemma").await;
    s.step(
        "inf:lemma",
        &["con:def-prime", "con:axiom-arith"],
        "req:lemma",
    )
    .await;
    let status = s.ok("loop_status", json!({})).await;
    let r = &status["reasoning"];
    assert_eq!(r["unconfirmed_steps"], 1, "{status:#}");
    assert_eq!(
        r["uncited_premises"], 1,
        "only the axiom not given: {status:#}"
    );
    assert_eq!(
        r["uncited_premise_ids"],
        json!(["con:axiom-arith"]),
        "{status:#}"
    );
    assert_eq!(r["stated_foundations"], 1, "{status:#}");
    let gaps = s.ok("detect_gaps", json!({})).await;
    let text = gaps.to_string();
    assert!(text.contains("unconfirmed_reasoning"), "{gaps:#}");
    let req = s
        .ok(
            "add_requirement",
            json!({"id": "req:axiom-as-statement", "name": "An axiom stated as a requirement",
                   "statement": "Every n ≥ 2 has a least divisor above 1.", "taken_as_given": true}),
        )
        .await;
    assert_eq!(req["properties"]["taken_as_given"], true, "{req:#}");
}

#[tokio::test]
async fn gaps_to_prompts_and_gap_to_prompt_need_only_the_gaps_id() {
    let s = session().await;
    s.statement("req:a").await;
    let gaps = s.ok("detect_gaps", json!({})).await;
    let id = gaps["items"]
        .as_array()
        .or_else(|| gaps["gaps"].as_array())
        .and_then(|a| a.first())
        .and_then(|g| g["id"].as_str())
        .map(str::to_string)
        .unwrap_or_else(|| panic!("a gap to ask about: {gaps:#}"));
    let many = s
        .ok(
            "gaps_to_prompts",
            json!({"gaps": [{"gap": {"id": id}, "answers": []}]}),
        )
        .await;
    assert_eq!(many["status"], "needs_llm", "{many:#}");
    let one = s.ok("gap_to_prompt", json!({"gap": {"id": id}})).await;
    assert_eq!(one["status"], "needs_llm", "{one:#}");
    let refused = s
        .call(
            "gaps_to_prompts",
            json!({"gaps": [{"gap": {"title": "no id"}, "answers": []}]}),
        )
        .await
        .expect_err("a row with no id is refused");
    assert!(
        refused.contains("`gaps[0].gap` needs the gap's `id`"),
        "{refused}"
    );
}

#[tokio::test]
async fn describe_schema_keeps_to_its_budget() {
    let s = session().await;
    s.statement("req:a").await;
    for args in [
        json!({"budget_chars": 4000}),
        json!({"node_type": "Verification", "budget_chars": 4000}),
        json!({"from": "Requirement", "to": "Requirement", "budget_chars": 4000}),
    ] {
        let reply = s.ok("describe_schema", args.clone()).await;
        let chars = serde_json::to_string(&reply).unwrap().chars().count();
        assert!(chars <= 4000, "{args}: {chars} characters");
        assert!(
            reply["budget"]["compacted"].is_string(),
            "{args}: {reply:#}"
        );
    }
    // Every name survives a shortened list.
    let reply = s.ok("describe_schema", json!({"budget_chars": 4000})).await;
    let names = reply["all_names"]["edge_types"].as_array().unwrap();
    assert!(names.contains(&json!("PREMISE_OF")), "{reply:#}");
}

#[tokio::test]
async fn add_verifications_verifies_field_says_its_shape_first() {
    let s = session().await;
    let tools = s.client.list_all_tools().await.expect("tools/list");
    let tool = tools
        .iter()
        .find(|t| t.name == "add_verification")
        .expect("add_verification is served");
    let schema = serde_json::to_value(tool.input_schema.as_ref()).unwrap();
    let about = schema["properties"]["verifies"]["description"]
        .as_str()
        .unwrap();
    let first_sentence = about.split(". ").next().unwrap();
    assert!(first_sentence.contains("target_id"), "{first_sentence}");
    assert!(schema["properties"]["computed"].is_object(), "{schema:#}");
}

#[tokio::test]
async fn a_capped_get_instructions_carries_its_first_section_whole() {
    let s = session().await;
    let whole = s.ok("get_instructions", json!({})).await;
    let sections = whole["sections"].as_array().unwrap();
    let first = sections[0]["section"].as_str().unwrap().to_string();
    let first_bytes = sections[0]["bytes"].as_u64().unwrap() as usize;
    let total = whole["total_bytes"].as_u64().unwrap() as usize;
    let budget = first_bytes + 6000;
    assert!(budget < total, "the budget withholds the whole document");
    let capped = s
        .ok("get_instructions", json!({"budget_chars": budget}))
        .await;
    assert_eq!(capped["section"], first, "{capped:#}");
    assert_eq!(capped["returned_bytes"], first_bytes, "{capped:#}");
    assert!(capped["instructions"].is_string(), "{capped:#}");
    assert!(capped["sections"].is_array());
    assert_eq!(
        capped["budget"]["detail"], "first_section_only",
        "{capped:#}"
    );
    // A budget too small for even the first section still gets the manifest.
    let tiny = s.ok("get_instructions", json!({"budget_chars": 500})).await;
    assert!(tiny["instructions"].is_null(), "{tiny:#}");
    assert_eq!(tiny["budget"]["detail"], "whole_document_withheld");
}
