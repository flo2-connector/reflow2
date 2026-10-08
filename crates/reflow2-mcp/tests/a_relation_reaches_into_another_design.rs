//! `other_design` on a relation link, through the served tools: the link is
//! recorded as a typed reference, search shows which design the linked node
//! lives in, and `loop_status` names a link nothing here can check
//! (`req:a-relation-between-nodes-in-two-designs-is-recorded-in-the-design-that-makes-it`).
//! Before it, `other_design` was refused as an unknown field and a link into
//! another design could only be prose.

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

async fn design_a() -> ReflowService {
    let s = ReflowService::in_memory_as("design-a").unwrap();
    call!(s, add_project, json!({"id": "proj:a", "name": "A"}));
    call!(
        s,
        add_requirement,
        json!({"id": "req:here", "name": "The okapi need",
               "statement": "The okapi need, as design A states it."})
    );
    s
}

fn linked(found: &Value) -> &[Value] {
    found["hits"][0]["linked"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

#[tokio::test]
async fn review_relations_links_into_another_design_and_search_names_it() {
    let s = design_a().await;
    call!(
        s,
        review_relations,
        json!({"node_id": "req:here", "links": [{
            "relation": "DUPLICATES", "other_design": "design-b", "other_id": "req:there",
            "other_type": "Requirement", "other_name": "The okapi need, in B",
            "evidence": "B states the same need for its own reason"}]})
    );
    let found = call!(s, search_design, json!({"query": "okapi"}));
    let l = linked(&found);
    assert!(
        l.iter().any(|x| x["design"] == json!("design-b")
            && x["design_node_id"] == json!("req:there")
            && x["relation"] == json!("DUPLICATES")),
        "{found}"
    );
}

#[tokio::test]
async fn a_decision_can_relate_to_a_node_in_another_design() {
    let s = design_a().await;
    call!(
        s,
        add_decision,
        json!({"id": "dec:idea-x", "name": "OPEN — an idea", "decision": "An idea.",
               "kind": "exploratory",
               "related_to": [{"relation": "ANTICIPATES", "other_design": "design-b",
                               "other_id": "req:there",
                               "evidence": "B's requirement is where this idea leads"}]})
    );
    let status = j!(s.upstream_status(Parameters(UpstreamStatusReq {})));
    let refs = status["references"].as_array().expect("references listed");
    assert_eq!(refs[0]["design"], json!("design-b"));
    assert_eq!(refs[0]["links"][0]["node_id"], json!("dec:idea-x"));
}

#[tokio::test]
async fn loop_status_names_a_link_into_an_undeclared_design() {
    let s = design_a().await;
    call!(
        s,
        review_relations,
        json!({"node_id": "req:here", "links": [{
            "relation": "DUPLICATES", "other_design": "design-b", "other_id": "req:there",
            "evidence": "the same need"}]})
    );
    let status = j!(s.loop_status(Parameters(LoopScopeReq::default())));
    let links = status["cross_design_links"]
        .as_array()
        .unwrap_or_else(|| panic!("loop_status names the unchecked link: {status}"));
    assert!(
        links[0].as_str().is_some_and(|t| t.contains("design-b")),
        "{links:?}"
    );
}
