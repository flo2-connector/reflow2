//! A search on a design that pins other designs names every pinned design it
//! did NOT search, and why — so "nothing found here" never reads as nothing
//! found anywhere (the small defect inside
//! `req:one-search-reaches-every-design-a-hub-tracks`; the search that reaches
//! them is phase 7's second step).
//!
//! WHAT FAILED. hxm_program's hub pins fourteen member designs by their
//! committed exports and is reached only through the --call door. Measured
//! 2026-10-21 on 0.89.0: `search_design` for a word only a member holds
//! answered from the hub alone, with `searched: 4` and nothing saying that
//! fourteen designs were never looked in.
//!
//! THE CLASS: every pin that names another design is named in the reply,
//! whatever state its record is in — an export on disk, an export missing,
//! an address, nowhere to read it — each with a reason, and a design that
//! pins nothing answers exactly as before.

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

fn scratch(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("reflow2-search-pins-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

async fn member_export(dir: &std::path::Path, graph_id: &str) -> String {
    let m = ReflowService::in_memory_as(graph_id).expect("member");
    call!(
        m,
        add_component,
        json!({"id": "cmp:role-pdfstream", "name": "pdfstream role",
               "description": "deploys pdfstream servers"})
    );
    let path = dir.join(format!("{graph_id}.json"));
    call!(m, export_graph, json!({"path": path.to_string_lossy()}));
    path.to_string_lossy().into_owned()
}

async fn hub() -> ReflowService {
    let s = ReflowService::in_memory_as("hub").expect("hub");
    call!(s, add_project, json!({"id": "proj:hub", "name": "Hub"}));
    call!(
        s,
        add_requirement,
        json!({"id": "req:firewall", "name": "PDF firewall fix",
               "statement": "The PDF beamline firewall admits the stream."})
    );
    s
}

#[tokio::test]
async fn every_pinned_design_is_named_with_why_it_was_not_searched() {
    let dir = scratch("all-kinds");
    let present = member_export(&dir, "ansible").await;
    let s = hub().await;
    let pins = [
        json!({"id": "dep:ansible", "name": "ansible", "source": "https://example.org/ansible",
               "version": "abc123", "graph_id": "ansible", "design_export": present}),
        json!({"id": "dep:tiled", "name": "tiled plugins", "source": "https://example.org/tiled",
               "version": "def456", "graph_id": "tiled",
               "design_export": dir.join("gone.json").to_string_lossy()}),
        json!({"id": "dep:hosted", "name": "hosted", "source": "https://example.org/hosted",
               "version": "v1", "graph_id": "hosted",
               "design_address": "http://127.0.0.1:9/g/hosted/mcp"}),
        json!({"id": "dep:nowhere", "name": "nowhere", "source": "https://example.org/nowhere",
               "version": "v1", "graph_id": "nowhere"}),
    ];
    for p in &pins {
        call!(s, external_dependency, p.clone());
    }
    let found = call!(s, search_design, json!({"query": "pdfstream"}));
    let named = found["not_searched"]
        .as_array()
        .unwrap_or_else(|| panic!("the pinned designs not searched are named: {found}"));
    for design in ["ansible", "tiled", "hosted", "nowhere"] {
        let entry = named
            .iter()
            .find(|n| n["design"] == json!(design))
            .unwrap_or_else(|| panic!("{design} is named: {found}"));
        assert!(
            entry["why"].as_str().is_some_and(|w| !w.trim().is_empty()),
            "{design} carries a reason: {found}"
        );
    }
    let why = |d: &str| {
        named
            .iter()
            .find(|n| n["design"] == json!(d))
            .and_then(|n| n["why"].as_str())
            .unwrap_or_default()
            .to_string()
    };
    assert!(why("tiled").contains("missing"), "{found}");
    assert!(why("nowhere").contains("nowhere"), "{found}");
    assert!(
        found["hits"].as_array().is_some_and(Vec::is_empty),
        "the member's word is not in the hub: {found}"
    );
    assert!(
        found["not_searched_note"]
            .as_str()
            .is_some_and(|n| n.contains("4")),
        "the reply says plainly how many pinned designs were not looked in: {found}"
    );
}

#[tokio::test]
async fn a_design_that_pins_nothing_answers_as_before() {
    let s = hub().await;
    let found = call!(s, search_design, json!({"query": "firewall"}));
    assert!(found.get("not_searched").is_none(), "{found}");
    assert!(found.get("not_searched_note").is_none(), "{found}");
    // A code dependency that names no design is not a design left unsearched.
    call!(
        s,
        external_dependency,
        json!({"id": "dep:serde", "name": "serde", "source": "crates.io", "version": "1"})
    );
    let found: Value = call!(s, search_design, json!({"query": "firewall"}));
    assert!(found.get("not_searched").is_none(), "{found}");
}
