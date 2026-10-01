//! A `parks` RULING'S WRITER SAYS WHETHER IT TAKES EFFECT — and says what
//! every parks reader will do with it.
//!
//! Until 2026-09-29 only the READERS asked whether a ruling parks anything
//! (`heal.rs` `is_parked`: an ACCEPTED Decision, nothing else), and
//! `governed_by` stored the edge and replied with success whatever it pointed
//! at: a proposed Decision, or any DesignRule, parked nothing and nothing said
//! so (fact:root-cause-a-parks-ruling-is-validated-only-by-its-readers-and-the-writer-reports-success-either-way-2026-09-29).
//!
//! Every ruling state is driven through the writer and then read back through
//! the readers — detect_defects's `swept.parked` and its `orphan_node` finding,
//! both of which go through `is_parked`, the one predicate every finding in
//! `PARKING_READERS` delegates to (pinned by
//! `every_finding_that_reads_parking_names_it`). The writer's stated effect
//! must be what the readers do.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

const BOSS: &str = "who:boss";

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "a-parks-writer-says-whether-it-takes-effect".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

async fn connect() -> Client {
    let service = ReflowService::in_memory().expect("in-memory service");
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = service.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    TestClient
        .serve((client_rx, client_tx))
        .await
        .expect("the in-process handshake")
}

/// `Ok(structured reply)` or `Err(refusal text)`.
async fn call(c: &Client, tool: &str, args: Value) -> Result<Value, String> {
    let Value::Object(arguments) = args else {
        panic!("arguments for {tool} must be an object");
    };
    match c
        .call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments))
        .await
    {
        Ok(r) if r.is_error == Some(true) => Err(format!("{:?}", r.content)),
        Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
        Err(e) => Err(e.to_string()),
    }
}

async fn defects(c: &Client) -> Value {
    call(c, "detect_defects", json!({}))
        .await
        .expect("detect_defects")
}

fn parked(d: &Value, id: &str) -> bool {
    d["swept"]["parked"]
        .as_array()
        .is_some_and(|a| a.iter().any(|v| v == id))
}

fn flagged_orphan(d: &Value, id: &str) -> bool {
    d["defects"].as_array().is_some_and(|a| {
        a.iter().any(|x| {
            x["category"] == "orphan_node"
                && x["affected_ids"]
                    .as_array()
                    .is_some_and(|ids| ids.iter().any(|v| v == id))
        })
    })
}

#[tokio::test]
async fn the_writer_says_what_every_parks_reader_will_do() {
    let c = connect().await;
    call(
        &c,
        "add_contributor",
        json!({"id": BOSS, "name": "The owner", "kind": "person"}),
    )
    .await
    .expect("seed contributor");
    // (ruling id, how to make it, will it park?)
    let rulings: Vec<(&str, Option<Value>, Option<bool>)> = vec![
        (
            "dec:ruling-proposed",
            Some(
                json!({"tool": "add_decision", "args": {"id": "dec:ruling-proposed", "name": "Park the proposed way", "decision": "a proposed ruling", "kind": "choice"}}),
            ),
            Some(false),
        ),
        (
            "dec:ruling-accepted",
            Some(
                json!({"tool": "add_decision", "args": {"id": "dec:ruling-accepted", "name": "Park the accepted way", "decision": "an accepted ruling", "kind": "choice", "status": "accepted", "approver": BOSS}}),
            ),
            Some(true),
        ),
        (
            "dec:ruling-deferred",
            Some(
                json!({"tool": "add_decision", "args": {"id": "dec:ruling-deferred", "name": "Park the deferred way", "decision": "a deferred ruling", "kind": "choice", "status": "deferred", "approver": BOSS}}),
            ),
            Some(false),
        ),
        (
            "dec:ruling-rejected",
            Some(
                json!({"tool": "add_decision", "args": {"id": "dec:ruling-rejected", "name": "Park the rejected way", "decision": "a rejected ruling", "kind": "choice", "status": "rejected", "approver": BOSS}}),
            ),
            Some(false),
        ),
        (
            "rule:ruling-a-rule",
            Some(
                json!({"tool": "add_design_rule", "args": {"id": "rule:ruling-a-rule", "name": "A rule, not a ruling", "statement": "rules shape; they do not park", "enforced": true, "approver": BOSS}}),
            ),
            None,
        ),
        ("dec:ruling-that-does-not-exist", None, None),
    ];
    let mut problems = Vec::new();
    for (i, (ruling, make, will_park)) in rulings.into_iter().enumerate() {
        if let Some(m) = make {
            call(&c, m["tool"].as_str().expect("tool"), m["args"].clone())
                .await
                .unwrap_or_else(|e| panic!("fixture {ruling}: {e}"));
        }
        let node = format!("art:parked-{i}");
        call(&c, "add_artifact", json!({"id": node, "name": format!("A report parked under {ruling}"), "artifact_type": "document", "location": format!("docs/parked-{i}.md")}))
            .await
            .expect("fixture artifact");
        let reply = call(
            &c,
            "governed_by",
            json!({"from_id": node, "to_id": ruling, "ruling": "parks"}),
        )
        .await;
        let d = defects(&c).await;
        match (will_park, reply) {
            (None, Ok(r)) => problems.push(format!(
                "{ruling}: a parks ruling no reader will EVER honour was recorded as success: {r}"
            )),
            (None, Err(_)) => {
                if parked(&d, &node) {
                    problems.push(format!("{ruling}: refused, yet {node} reads as parked"));
                }
            }
            (Some(_), Err(e)) => {
                problems.push(format!("{ruling}: a Decision ruling was refused: {e}"))
            }
            (Some(want), Ok(r)) => {
                let said = r["parks"]["in_force"].as_bool();
                if said != Some(want) {
                    problems.push(format!(
                        "{ruling}: the writer said in_force={said:?}, the rule says {want}: {r}"
                    ));
                }
                if !want
                    && !r["parks"]["note"]
                        .as_str()
                        .is_some_and(|n| n.contains("PARKS NOTHING YET"))
                {
                    problems.push(format!(
                        "{ruling}: a ruling that parks nothing yet did not say so: {r}"
                    ));
                }
                if parked(&d, &node) != want {
                    problems.push(format!(
                        "{ruling}: the writer said in_force={want}, and swept.parked says {}",
                        parked(&d, &node)
                    ));
                }
                if flagged_orphan(&d, &node) == want {
                    problems.push(format!(
                        "{ruling}: orphan_node reads {node} as {} — the writer said in_force={want}",
                        if want { "still open" } else { "parked" }
                    ));
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
