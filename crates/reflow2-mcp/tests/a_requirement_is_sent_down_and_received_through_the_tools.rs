//! Slice 3 through the served tools: `receive_from_design` in the part,
//! `send_to_design` in the parent, and `loop_status` naming what waits on the
//! part's owner. A move without the owner's word is refused.

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

macro_rules! j {
    ($call:expr) => {
        $call
            .await
            .expect("tool ok")
            .structured_content
            .expect("structured content present")
    };
}

macro_rules! call {
    ($s:expr, $tool:ident, $args:expr) => {
        j!($s.$tool(Parameters(serde_json::from_value($args).unwrap())))
    };
}

async fn design(id: &str) -> ReflowService {
    let s = ReflowService::in_memory_as(id).unwrap();
    call!(
        s,
        add_project,
        json!({"id": format!("proj:{id}"), "name": id})
    );
    call!(
        s,
        add_contributor,
        json!({"id": "who:ajs", "name": "Anthony", "kind": "person"})
    );
    s
}

fn reply_of(r: Result<rmcp::model::CallToolResult, rmcp::ErrorData>) -> Value {
    match r {
        Ok(ok) => ok.structured_content.unwrap_or(Value::Null),
        Err(e) => json!({"error": e.message}),
    }
}

#[tokio::test]
async fn a_requirement_moves_down_and_waits_on_the_parts_owner() {
    let p = design("design-p").await;
    let a = design("design-a").await;
    call!(
        p,
        add_requirement,
        json!({"id": "req:p-log", "name": "Logging", "statement": "Keep an audit log."})
    );

    let got = call!(
        a,
        receive_from_design,
        json!({"id": "req:a-log", "name": "Audit log",
               "statement": "This part keeps an audit log.",
               "from_design": "design-p", "from_node_id": "req:p-log",
               "kind": "moved", "sender": "who:ajs"})
    );
    assert_eq!(got["status"], json!("proposed"));

    let refused = reply_of(
        p.send_to_design(Parameters(
            serde_json::from_value(json!({"node_id": "req:p-log", "to_design": "design-a",
                                          "to_node_id": "req:a-log", "kind": "moved"}))
            .unwrap(),
        ))
        .await,
    );
    assert!(
        refused["error"]
            .as_str()
            .is_some_and(|e| e.contains("approver")),
        "a move needs the owner's word: {refused}"
    );
    let sent = call!(
        p,
        send_to_design,
        json!({"node_id": "req:p-log", "to_design": "design-a", "to_node_id": "req:a-log",
               "kind": "moved", "approver": "who:ajs"})
    );
    assert_eq!(sent["reference"], json!("xref:design-a:req:a-log"));

    let status = j!(a.loop_status(Parameters(LoopScopeReq::default())));
    let waiting = status["received_waiting"]
        .as_array()
        .unwrap_or_else(|| panic!("loop_status names what waits: {status}"));
    assert!(
        waiting[0].as_str().is_some_and(|t| t.contains("req:a-log")),
        "{waiting:?}"
    );
}
