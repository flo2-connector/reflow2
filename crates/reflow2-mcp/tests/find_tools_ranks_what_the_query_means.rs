//! `find_tools` ranks the tool that ANSWERS a query in a user's words above
//! tools that merely contain more of its words.
//!
//! # Why these six
//!
//! Each was a MISS on 2026-09-11 (absent from the top 10) in the 180-query
//! criterion-1 corpus, and each is found by the corrected scorer — measured
//! over the whole corpus, not predicted: 53 misses → 42, with zero tools
//! regressing. They are the served-surface half of the fix pinned in
//! `service.rs::find_tools_scoring_invariants`.
//!
//! ⚠️ TWO FIXTURES WERE DROPPED, AND WHY IS THE HONEST PART. A Python replica
//! of the scorer predicted `satisfies` and `get_skill` would flip; the real
//! server, re-measured after the fix, still misses both. `get_skill` ←
//! "playbook" is a near-vocabulary-gap (the word is nowhere on the tool), and
//! `satisfies` loses to `add_capability`, whose description genuinely names
//! satisfying. The replica overstated the stopword half throughout (it
//! dropped a hand list; the real fix zero-weights only terms present in EVERY
//! entry). A fixture is a claim about the served surface, so it carries only
//! what the served surface does. The twelve zero-overlap misses
//! (`add_requirement`, `graph_report`, `get_node`…) are not here either —
//! no scoring change can reach them
//! (`fact:find-tools-misses-split-into-a-vocabulary-gap…`).
//!
//! Top 5 is the bar because 5 is what a consumer sees by default.

use reflow2_mcp::service::ReflowService;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

async fn top5(s: &ReflowService, query: &str) -> Vec<String> {
    let v: Value = s
        .find_tools(Parameters(
            serde_json::from_value(json!({"query": query, "limit": 5})).unwrap(),
        ))
        .await
        .expect("find_tools")
        .structured_content
        .expect("structured");
    v["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter_map(|i| i["tool"].as_str().map(String::from))
        .collect()
}

const FIXTURES: &[(&str, &str)] = &[
    (
        "realizes",
        "record that this artifact implements that capability",
    ),
    ("delete_edge", "remove a link between two items"),
    (
        "allocate",
        "say which component is responsible for this capability",
    ),
    (
        "deploy_to",
        "record that this release runs in that environment",
    ),
    (
        "part_of_flow",
        "add this capability as a step in that process",
    ),
    ("contains", "attach this item under the project"),
];

#[tokio::test]
async fn a_query_in_a_users_words_finds_the_tool_that_answers_it() {
    let s = ReflowService::in_memory().expect("service");
    let mut missed = Vec::new();
    for (tool, query) in FIXTURES {
        let got = top5(&s, query).await;
        if !got.iter().any(|t| t == tool) {
            missed.push(format!("{tool:<14} ← {query:?}\n      top5: {got:?}"));
        }
    }
    assert!(
        missed.is_empty(),
        "{} of {} fixtures not in the top 5:\n  {}",
        missed.len(),
        FIXTURES.len(),
        missed.join("\n  ")
    );
}

/// THE READER OF ONE NODE AND ITS EDGES ranks FIRST, not merely top 5
/// (`dec:idea-an-edge-reader-returns-one-nodes-edges-and-find-tools-finds-it`).
///
/// Measured on 0.77.0 before `include_edges` existed
/// (`fact:find-tools-ranks-get-node-eighth-for-a-node-with-its-edges-because-nothing-served-returns-both-2026-10-02`):
/// for the field report's own words, "read one node by id with its properties
/// and edges", get_node ranked 8th, below five edge WRITERS, and so left the
/// default five. The ranking was honest — nothing served returned both — so
/// the fix is the reader, and its description saying so in the asker's words.
/// OBSERVED FAILING before the description changed, 2026-10-03: 7 of the 11
/// queries first written here did not put get_node first.
///
/// The first query is the report's; the rest are paraphrases written from
/// other angles (connections, neighbours), plus the plain reads get_node
/// already led, which must not slip.
const GET_NODE_FIRST: &[&str] = &[
    "read one node by id with its properties and edges",
    "get a node and its edges",
    "show a node's edges",
    "what is this node connected to",
    "show a node's neighbours",
    "read one node by id",
    "fetch a node by id",
    "pull up the full record for a requirement",
];

/// Paraphrases where get_node is FOUND (top 5, the default a caller sees) but
/// cannot rank first: another tool's NAME carries the query's word, which
/// scores five times a description hit and which no wording can outweigh (the
/// rank ratchet's own finding). `create_edges` owns "edges" in "a node and its
/// edges"; `list_skills` and `create_edges` own "list" and "edges" in "list
/// the edges of a node". Two more were measured at rank 5 and are left out as
/// too close to call: "which nodes link to this one" (`create_nodes`,
/// `link_artifact`, `scan_nodes` and `linking_report` carry "nodes" or "link")
/// and "what links to this item and what does it link to".
const GET_NODE_FOUND: &[&str] = &["a node and its edges", "list the edges of a node"];

#[tokio::test]
async fn reading_one_node_with_its_edges_finds_get_node_first() {
    let s = ReflowService::in_memory().expect("service");
    let mut wrong = Vec::new();
    for query in GET_NODE_FIRST {
        let got = top5(&s, query).await;
        if got.first().map(String::as_str) != Some("get_node") {
            wrong.push(format!("first  {query:?}\n      top5: {got:?}"));
        }
    }
    for query in GET_NODE_FOUND {
        let got = top5(&s, query).await;
        if !got.iter().any(|t| t == "get_node") {
            wrong.push(format!("top 5  {query:?}\n      top5: {got:?}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "get_node is not where it must be for {} of {} queries:\n  {}",
        wrong.len(),
        GET_NODE_FIRST.len() + GET_NODE_FOUND.len(),
        wrong.join("\n  ")
    );
}
