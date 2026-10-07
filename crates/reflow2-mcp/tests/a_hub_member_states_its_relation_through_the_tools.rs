//! A member's relation through the served tools: `external_dependency` records
//! it and keeps it on a re-declare that leaves it out, `upstream_status` reads
//! it back, and `loop_status` says when a member is linked by nothing a
//! cross-design ripple could follow — even when nothing is watched, because a
//! design hosted on flo2.io cannot watch another at all
//! (`cap:a-hub-member-records-its-relation-and-the-interfaces-it-crosses`).

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

async fn hub() -> ReflowService {
    let s = ReflowService::in_memory().expect("in-memory service");
    j!(s.add_project(Parameters(
        serde_json::from_value(json!({"id": "proj:hub", "name": "Hub"})).unwrap()
    )));
    s
}

async fn declare(s: &ReflowService, args: Value) -> Value {
    j!(s.external_dependency(Parameters(serde_json::from_value(args).unwrap())))
}

fn member_args() -> Value {
    json!({
        "id": "dep:calc", "name": "calc", "source": "https://example.org/calc",
        "version": "v0.7.0", "graph_id": "0bee0c00b35845f6"
    })
}

#[tokio::test]
async fn a_re_declare_that_leaves_the_relation_out_keeps_it() {
    let s = hub().await;
    let mut args = member_args();
    args["relation"] = json!(["uses"]);
    args["interfaces"] = json!(["ifc:calc-api"]);
    declare(&s, args).await;
    // The version moves; the relation is not mentioned.
    let mut again = member_args();
    again["version"] = json!("v0.8.0");
    declare(&s, again).await;

    let status = j!(s.upstream_status(Parameters(UpstreamStatusReq {})));
    let m = &status["members"][0];
    assert_eq!(m["relation"], json!(["uses"]), "{status}");
    assert_eq!(m["interfaces"], json!(["ifc:calc-api"]), "{status}");
}

#[tokio::test]
async fn loop_status_names_a_member_linked_by_nothing_even_when_nothing_is_watched() {
    let s = hub().await;
    declare(&s, member_args()).await;
    let status = j!(s.loop_status(Parameters(LoopScopeReq::default())));
    let unlinked = status["members_unlinked"]
        .as_array()
        .unwrap_or_else(|| panic!("loop_status must name the unlinked member: {status}"));
    assert!(
        unlinked[0].as_str().is_some_and(|t| t.contains("'calc'")),
        "{unlinked:?}"
    );
    assert!(
        status.get("upstream_moved").is_none(),
        "an unstated relation is not an upstream that moved: {status}"
    );

    let mut stated = member_args();
    stated["relation"] = json!(["part_of"]);
    declare(&s, stated).await;
    let status = j!(s.loop_status(Parameters(LoopScopeReq::default())));
    assert!(
        status.get("members_unlinked").is_none(),
        "once the relation is stated nothing is owed: {status}"
    );
}
