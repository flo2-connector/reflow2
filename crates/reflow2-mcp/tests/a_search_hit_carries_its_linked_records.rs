//! F3 of `req:the-pieces-of-one-picture-are-found-together`: a search hit
//! carries the records directly linked to it, so a linked family comes back
//! together.
//!
//! Measured 2026-09-22 and again 2026-10-07: an edge did nothing for
//! `search_design`, which ranked each node by its own text. A family whose
//! pieces were linked still came back as scattered hits, and the agent reading
//! them told the owner nothing tied them together.

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

fn linked_of<'a>(result: &'a Value, hit: &str) -> &'a [Value] {
    result["hits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["node_id"] == json!(hit))
        .unwrap_or_else(|| panic!("{hit} is a hit: {result}"))["linked"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

#[tokio::test]
async fn a_hit_names_the_records_linked_to_it_with_the_relation_and_its_direction() {
    let s = ReflowService::in_memory().unwrap();
    call!(s, add_project, json!({"id": "proj:x", "name": "X"}));
    call!(
        s,
        add_requirement,
        json!({"id": "req:umbrella", "name": "The zebracorn aim",
               "statement": "The whole zebracorn picture."})
    );
    call!(
        s,
        add_requirement,
        json!({"id": "req:piece", "name": "A piece",
               "statement": "One facet of something larger.",
               "distinct_from": ["req:umbrella"]})
    );
    call!(
        s,
        decomposes,
        json!({"from_id": "req:piece", "to_id": "req:umbrella"})
    );
    call!(
        s,
        create_edge,
        json!({"edge_type": "DEPENDS_ON", "from_id": "req:umbrella", "to_id": "req:piece"})
    );

    let found = call!(s, search_design, json!({"query": "zebracorn"}));
    let linked = linked_of(&found, "req:umbrella");
    assert!(
        linked.iter().any(|l| l["node_id"] == json!("req:piece")
            && l["relation"] == json!("DECOMPOSES")
            && l["direction"] == json!("in")),
        "the piece that DECOMPOSES the hit comes back with it, inbound: {found}"
    );
    assert!(
        linked.iter().any(|l| l["node_id"] == json!("req:piece")
            && l["relation"] == json!("DEPENDS_ON")
            && l["direction"] == json!("out")),
        "a review relation drawn from the hit comes back outbound: {found}"
    );
}

#[tokio::test]
async fn a_hit_with_nothing_linked_carries_no_linked_field() {
    let s = ReflowService::in_memory().unwrap();
    call!(s, add_project, json!({"id": "proj:x", "name": "X"}));
    call!(
        s,
        add_requirement,
        json!({"id": "req:alone", "name": "A quokka requirement",
               "statement": "Nothing links to this quokka."})
    );
    let found = call!(s, search_design, json!({"query": "quokka"}));
    assert!(
        linked_of(&found, "req:alone").is_empty(),
        "no links, no field: {found}"
    );
}
