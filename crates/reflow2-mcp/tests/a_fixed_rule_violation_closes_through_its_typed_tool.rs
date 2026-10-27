//! A RULE VIOLATION RULED MUST-FIX CLOSES THROUGH THE TYPED TOOL THAT RULED IT.
//!
//! `fact:root-cause-the-open-breach-reader-prescribes-resolved-at-that-no-typed-tool-writes-2026-10-26`.
//! The open-breach reader (detect.rs, `breaches_not_closed`) counts a
//! VIOLATES_RULE edge ruled `rejected` as open until it carries `resolved_at`,
//! and its repair text told the agent to "record `resolved_at` on the
//! VIOLATES_RULE edge". No typed tool wrote it: `set_violation_status` took
//! element, rule, status and rationale, and `violates_rule` took no
//! `detected_at` either, so a designer on a sample shop design closed one with
//! the generic `create_edge`, which bypasses the triage tool's checks.
//!
//! OBSERVED FAILING FIRST on main 02e2739c8: `set_violation_status` refused
//! `resolved_at` as an unknown field, and `violates_rule` refused
//! `detected_at`.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "violation-close-probe".to_string();
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

    /// The open breaches `loop_status` lists on the barn.
    async fn open_on_the_barn(&self) -> Vec<Value> {
        let ls = self.ok("loop_status", json!({})).await;
        ls["breaches"]
            .as_array()
            .unwrap_or_else(|| panic!("loop_status lists breaches: {ls}"))
            .iter()
            .filter(|b| b["subject_id"] == "cmp:barn-lean-to")
            .cloned()
            .collect()
    }
}

/// The shop sample's shape with none of its data: a barn lean-to 12 ft from a
/// well, against a 100 ft setback, flagged by a check on a known date.
async fn the_barn_by_the_well() -> Session {
    let s = session().await;
    s.ok(
        "add_project",
        json!({"id": "proj:shop", "name": "A shop (fixture)"}),
    )
    .await;
    s.ok(
        "add_component",
        json!({"id": "cmp:barn-lean-to", "name": "Barn lean-to", "description": "A part."}),
    )
    .await;
    s.ok(
        "add_environment_rule",
        json!({"id": "erule:well-setback", "name": "Livestock barns stay 100 ft from a well",
               "statement": "A barn stands at least 100 ft from a well head.",
               "rule_type": "regulatory"}),
    )
    .await;
    s.ok(
        "violates_rule",
        json!({"element_id": "cmp:barn-lean-to", "rule_id": "erule:well-setback",
               "proposer": "check", "severity": "critical",
               "evidence": "12 ft from the well head", "detected_at": "2026-10-20"}),
    )
    .await;
    s
}

#[tokio::test]
async fn a_must_fix_violation_closed_with_set_violation_status_leaves_the_open_breaches() {
    let s = the_barn_by_the_well().await;
    s.ok(
        "set_violation_status",
        json!({"element_id": "cmp:barn-lean-to", "rule_id": "erule:well-setback",
               "status": "rejected", "rationale": "the lean-to must move"}),
    )
    .await;
    let open = s.open_on_the_barn().await;
    assert_eq!(open.len(), 1, "ruled must-fix, it is open: {open:?}");
    assert_eq!(
        open[0]["recorded_on"], "2026-10-20",
        "the date violates_rule recorded is the breach's date: {open:?}"
    );

    // The gap that asks about it names the typed tool, not a raw edge write.
    let gaps = s.ok("detect_gaps", json!({"budget_chars": 200000})).await;
    let text = gaps.to_string();
    let breach = gaps["items"]
        .as_array()
        .and_then(|g| g.iter().find(|x| x["gap_source"] == "open_breach"))
        .unwrap_or_else(|| panic!("an open_breach gap is raised: {text}"));
    let said = breach.to_string();
    assert!(
        said.contains("set_violation_status") && said.contains("resolved_at"),
        "the repair names the tool and its argument: {said}"
    );
    assert!(
        !said.contains("on the VIOLATES_RULE edge"),
        "no raw edge write is prescribed: {said}"
    );

    // Fixed: closed through the same typed tool, kept for audit.
    let r = s
        .ok(
            "set_violation_status",
            json!({"element_id": "cmp:barn-lean-to", "rule_id": "erule:well-setback",
                   "status": "rejected", "resolved_at": "2026-10-26",
                   "rationale": "the lean-to moved to 140 ft"}),
        )
        .await;
    assert_eq!(
        r["resolved_at"], "2026-10-26",
        "the reply says what it recorded: {r}"
    );
    assert!(
        s.open_on_the_barn().await.is_empty(),
        "a violation fixed through set_violation_status leaves the open breaches"
    );
    let edge = s
        .ok(
            "get_node",
            json!({"id": "cmp:barn-lean-to",
                   "include_edges": {"edge_types": ["VIOLATES_RULE"]}}),
        )
        .await;
    let e = edge.to_string();
    assert!(
        e.contains("2026-10-20")
            && e.contains("2026-10-26")
            && e.contains("12 ft from the well head"),
        "the edge is kept, with when it was found, when it was fixed and its evidence: {e}"
    );
}

#[tokio::test]
async fn resolved_at_is_refused_on_anything_but_a_must_fix_ruling_and_on_a_pair_with_no_edge() {
    let s = the_barn_by_the_well().await;
    let refused = s
        .call(
            "set_violation_status",
            json!({"element_id": "cmp:barn-lean-to", "rule_id": "erule:well-setback",
                   "status": "confirmed", "resolved_at": "2026-10-26"}),
        )
        .await
        .expect_err("a waiver is not a fix");
    assert!(
        refused.contains("resolved_at") && refused.contains("rejected"),
        "it says why and what would work: {refused}"
    );
    let refused = s
        .call(
            "set_violation_status",
            json!({"element_id": "cmp:barn-lean-to", "rule_id": "erule:no-such-rule",
                   "status": "rejected", "resolved_at": "2026-10-26"}),
        )
        .await
        .expect_err("no violation to close");
    assert!(refused.contains("erule:no-such-rule"), "{refused}");
}
