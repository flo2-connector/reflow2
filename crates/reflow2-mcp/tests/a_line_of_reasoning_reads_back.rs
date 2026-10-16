//! A line of reasoning, recorded with reflow2's own vocabulary and read back
//! (req:a-design-holds-a-line-of-reasoning) — over the MCP surface.
//!
//! An objection counts as far as it holds: one that holds refutes the claim,
//! one that is unknown contests it, one a reply answers stops counting. A
//! dispute kept open on purpose is a Decision set to `contested`, and is not a
//! defect. `reasoning_report` reads the claims, their support, what they rest
//! on, the points still contested and every claim with no support.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "reasoning-probe".to_string();
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

    async fn claim(&self, id: &str) {
        self.ok(
            "add_requirement",
            json!({"id": id, "name": format!("claim {id}"), "statement": "A claim in an argument."}),
        )
        .await;
    }

    async fn check(&self, id: &str, status: &str) {
        self.ok(
            "add_verification",
            json!({"id": id, "name": format!("check {id}"), "description": "A check."}),
        )
        .await;
        self.ok(
            "set_verification_status",
            json!({"verification_id": id, "status": status, "last_run_at": "2026-10-16"}),
        )
        .await;
    }

    async fn step(&self, id: &str, premises: &[&str], conclusion: &str, warrant: &str) {
        self.ok(
            "add_inference",
            json!({
                "id": id,
                "rule": "all",
                "premises": premises,
                "conclusion": conclusion,
                "warrant": warrant
            }),
        )
        .await;
    }

    /// `from` stands against `to`: an objection, or a reply to one.
    async fn objects(&self, from: &str, to: &str) {
        self.ok(
            "create_edge",
            json!({"edge_type": "CONTRADICTS", "from_id": from, "to_id": to}),
        )
        .await;
    }

    async fn proof_status(&self, id: &str) -> Value {
        self.ok("get_node", json!({"id": id})).await["node"]["proof_status"].clone()
    }

    async fn verdict(&self, step: &str) -> Value {
        self.ok("get_node", json!({"id": step})).await["node"]["inference_reading"]["verdict"]
            .clone()
    }
}

fn claim_in<'a>(report: &'a Value, id: &str) -> &'a Value {
    report["claims"]
        .as_array()
        .expect("claims")
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("{id} is not in the read-back: {report:#}"))
}

fn ids(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|x| x.as_str().unwrap_or_default().to_string())
        .collect()
}

#[tokio::test]
async fn an_unestablished_objection_contests_a_claim_rather_than_refuting_it() {
    let s = session().await;
    s.claim("req:claim").await;
    s.check("ver:evidence", "passing").await;
    s.step(
        "inf:support",
        &["ver:evidence"],
        "req:claim",
        "the check measures it",
    )
    .await;
    assert_eq!(s.proof_status("req:claim").await["status"], "shown");

    // Raised, neither established nor answered: a proposed requirement.
    s.claim("req:objection").await;
    s.objects("req:objection", "req:claim").await;

    let ps = s.proof_status("req:claim").await;
    assert_eq!(ps["status"], "contested", "{ps:#}");
    assert!(
        ps["why"].as_str().unwrap().contains("req:objection"),
        "the why names what stands against it: {ps:#}"
    );

    // A step resting on a contested claim is unknown, never broken.
    s.claim("req:downstream").await;
    s.step("inf:uses", &["req:claim"], "req:downstream", "it follows")
        .await;
    assert_eq!(s.verdict("inf:uses").await, "unknown");
}

#[tokio::test]
async fn a_reply_that_holds_answers_the_objection_and_the_claim_stands_again() {
    let s = session().await;
    s.claim("req:claim").await;
    s.check("ver:evidence", "passing").await;
    s.step(
        "inf:support",
        &["ver:evidence"],
        "req:claim",
        "the check measures it",
    )
    .await;
    s.claim("req:objection").await;
    s.objects("req:objection", "req:claim").await;
    assert_eq!(s.proof_status("req:claim").await["status"], "contested");

    s.check("ver:reply", "passing").await;
    s.objects("ver:reply", "req:objection").await;

    assert_eq!(s.proof_status("req:claim").await["status"], "shown");
    let r = s
        .ok("reasoning_report", json!({"claim": "req:claim"}))
        .await;
    let objections = &claim_in(&r, "req:claim")["objections"];
    assert_eq!(objections[0]["id"], "req:objection", "{r:#}");
    assert_eq!(objections[0]["standing"], "answered", "{r:#}");
    let reply = &claim_in(&r, "req:objection")["objections"][0];
    assert_eq!(reply["id"], "ver:reply", "{r:#}");
    assert_eq!(reply["standing"], "refutes", "{r:#}");
    assert!(r["still_contested"].as_array().unwrap().is_empty(), "{r:#}");
}

#[tokio::test]
async fn an_established_objection_refutes_the_claim() {
    let s = session().await;
    s.claim("req:claim").await;
    s.check("ver:evidence", "passing").await;
    s.step(
        "inf:support",
        &["ver:evidence"],
        "req:claim",
        "the check measures it",
    )
    .await;
    s.check("ver:counterexample", "passing").await;
    s.objects("ver:counterexample", "req:claim").await;

    let ps = s.proof_status("req:claim").await;
    assert_eq!(ps["status"], "unproven", "{ps:#}");
    let r = s
        .ok("reasoning_report", json!({"claim": "req:claim"}))
        .await;
    let c = claim_in(&r, "req:claim");
    assert_eq!(c["truth"], "fails", "{r:#}");
    assert_eq!(c["objections"][0]["standing"], "refutes", "{r:#}");
}

#[tokio::test]
async fn two_claims_that_object_to_each_other_are_both_contested() {
    let s = session().await;
    s.claim("req:one").await;
    s.claim("req:two").await;
    s.check("ver:for-one", "passing").await;
    s.check("ver:for-two", "passing").await;
    s.step("inf:one", &["ver:for-one"], "req:one", "measured")
        .await;
    s.step("inf:two", &["ver:for-two"], "req:two", "measured")
        .await;
    s.objects("req:one", "req:two").await;
    s.objects("req:two", "req:one").await;

    assert_eq!(s.proof_status("req:one").await["status"], "contested");
    assert_eq!(s.proof_status("req:two").await["status"], "contested");
}

#[tokio::test]
async fn a_contested_decision_keeps_a_dispute_open_and_it_is_not_a_defect() {
    let s = session().await;
    s.claim("req:side-one").await;
    s.claim("req:side-two").await;
    s.objects("req:side-one", "req:side-two").await;
    let before = s.ok("detect_defects", json!({})).await;
    assert!(
        before.to_string().contains("contradict each other"),
        "an unsettled disagreement is reported: {before:#}"
    );

    s.ok(
        "add_decision",
        json!({
            "id": "dec:keep-both",
            "name": "Both readings are kept",
            "decision": "Neither side is chosen; both stay on the record."
        }),
    )
    .await;
    s.ok(
        "set_decision_status",
        json!({"decision_id": "dec:keep-both", "status": "contested"}),
    )
    .await;
    for side in ["req:side-one", "req:side-two"] {
        s.ok(
            "governed_by",
            json!({"from_id": side, "to_id": "dec:keep-both"}),
        )
        .await;
    }

    let after = s.ok("detect_defects", json!({})).await;
    assert!(
        !after.to_string().contains("contradict each other"),
        "a dispute kept open on purpose is not a defect: {after:#}"
    );
    assert_eq!(
        after
            .pointer("/swept/suppressed_by_held_open/contradiction")
            .cloned(),
        Some(json!(1)),
        "it is counted as held open: {after:#}"
    );

    let r = s.ok("reasoning_report", json!({})).await;
    assert_eq!(ids(&r["roots"]), ["dec:keep-both"], "{r:#}");
    let held = &r["contested_by_design"][0];
    assert_eq!(held["decision"], "dec:keep-both", "{r:#}");
    assert_eq!(ids(&held["sides"]), ["req:side-one", "req:side-two"]);
    assert!(
        r["still_contested"].as_array().unwrap().is_empty(),
        "kept open on purpose is not still contested: {r:#}"
    );
    assert_eq!(claim_in(&r, "dec:keep-both")["truth"], "unknown");
}

#[tokio::test]
async fn the_reasoning_report_reads_an_argument_end_to_end() {
    let s = session().await;
    // The thesis, supported by one step over three premises.
    s.claim("req:thesis").await;
    s.ok(
        "add_constraint",
        json!({
            "id": "con:premise",
            "name": "A premise taken as given",
            "statement": "Taken as given in this argument."
        }),
    )
    .await;
    s.check("ver:observation", "passing").await;
    s.ok(
        "add_artifact",
        json!({"id": "art:source", "name": "The cited study", "location": "docs/study.pdf"}),
    )
    .await;
    s.ok(
        "create_edge",
        json!({"edge_type": "DOCUMENTS", "from_id": "art:source", "to_id": "ver:observation"}),
    )
    .await;
    s.claim("req:bare-claim").await;
    s.step(
        "inf:main",
        &["con:premise", "ver:observation", "req:bare-claim"],
        "req:thesis",
        "the premise and the observation together, with the bare claim, give the thesis",
    )
    .await;
    // An objection to the thesis, answered by a reply that holds.
    s.claim("req:objection-a").await;
    s.objects("req:objection-a", "req:thesis").await;
    s.check("ver:reply-a", "passing").await;
    s.objects("ver:reply-a", "req:objection-a").await;
    // An objection to the bare claim, left standing.
    s.claim("req:objection-b").await;
    s.objects("req:objection-b", "req:bare-claim").await;

    let r = s.ok("reasoning_report", json!({})).await;
    assert_eq!(ids(&r["roots"]), ["req:thesis"], "{r:#}");
    assert_eq!(
        ids(&r["rests_on"]),
        ["con:premise", "req:bare-claim", "ver:observation"],
        "{r:#}"
    );
    assert_eq!(
        ids(&r["uncited_premises"]),
        ["con:premise", "req:bare-claim"],
        "the observation cites its source: {r:#}"
    );
    let thesis = claim_in(&r, "req:thesis");
    assert_eq!(
        thesis["support"][0]["warrant"],
        "the premise and the observation together, with the bare claim, give the thesis"
    );
    assert_eq!(thesis["objections"][0]["standing"], "answered", "{r:#}");
    let observation = claim_in(&r, "ver:observation");
    assert_eq!(observation["sources"][0]["id"], "art:source", "{r:#}");
    assert_eq!(observation["sources"][0]["location"], "docs/study.pdf");

    let contested = r["still_contested"].as_array().unwrap();
    assert_eq!(contested.len(), 1, "{r:#}");
    assert_eq!(contested[0]["claim"], "req:bare-claim");
    assert_eq!(ids(&contested[0]["against"]), ["req:objection-b"]);

    let unsupported: Vec<String> = r["unsupported"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        unsupported,
        ["req:bare-claim", "req:objection-b"],
        "no step, no source, unknown on its own: {r:#}"
    );

    let means = r["what_this_means"].as_str().unwrap();
    assert!(means.contains("STRUCTURE IS NOT TRUTH"), "{means}");
    assert!(means.contains("legal"), "{means}");
}

#[tokio::test]
async fn one_claim_reads_only_what_is_under_it() {
    let s = session().await;
    s.claim("req:thesis").await;
    s.claim("req:bare-claim").await;
    s.step("inf:main", &["req:bare-claim"], "req:thesis", "it follows")
        .await;
    s.claim("req:objection-b").await;
    s.objects("req:objection-b", "req:bare-claim").await;

    let r = s
        .ok("reasoning_report", json!({"claim": "req:bare-claim"}))
        .await;
    assert_eq!(ids(&r["roots"]), ["req:bare-claim"]);
    let read: Vec<&str> = r["claims"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(read, ["req:bare-claim", "req:objection-b"], "{r:#}");
}

#[tokio::test]
async fn a_design_with_no_reasoning_says_so_and_a_missing_claim_is_refused() {
    let s = session().await;
    s.claim("req:alone").await;
    let r = s.ok("reasoning_report", json!({})).await;
    assert!(r["claims"].as_array().unwrap().is_empty(), "{r:#}");
    assert!(
        r["note"].as_str().unwrap().contains("add_inference"),
        "it says how reasoning is recorded: {r:#}"
    );
    let e = s
        .refused("reasoning_report", json!({"claim": "req:nowhere"}))
        .await;
    assert!(e.contains("no single node"), "{e}");
}
