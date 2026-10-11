//! A revise of a ChangeEvent's `affected` list keeps what other tools put on the
//! CHANGED edges it already drew.
//!
//! fact:revising-a-change-events-affected-list-resets-the-acceptances-already-on-its-
//! changed-edges-2026-10-10, measured on 0.81.1 (hosted) and reproduced on 0.82.0
//! on a scratch store with no gateway: `add_change_event` called again with one
//! more `affected` entry re-created every CHANGED edge with `{action}` alone, so the
//! acceptance `set_artifact_checksums` had written on two of them
//! (`accepted_baseline`, `accepted_seq`, `checksum_after`) was gone, and the design
//! gate failed on files accepted minutes earlier. The handler asked
//! `DrawnEdges::classify` whether each edge was there, reported them as
//! `edges_already_present`, and wrote over them anyway.
//!
//! The contract "a revise keeps what it was not passed"
//! (`a_revise_keeps_what_it_was_not_passed.rs`) is held for node properties by
//! `upsert_node`; an edge another tool decorates is the same promise one level
//! down, and `create_edge` replaces. So: an edge already present keeps every
//! property the call did not pass, and `action` — the one thing the call does
//! say — moves when it changes.

use reflow2_mcp::service::ReflowService;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

macro_rules! call {
    ($s:expr, $tool:ident, $args:expr) => {
        $s.$tool(Parameters(serde_json::from_value($args).expect("request")))
            .await
            .expect("tool ok")
            .structured_content
            .expect("structured content present")
    };
}

async fn seeded() -> ReflowService {
    let s = ReflowService::in_memory().expect("in-memory service");
    call!(s, add_project, json!({ "id": "proj:x", "name": "X" }));
    call!(
        s,
        add_capability,
        json!({ "id": "cap:x", "name": "x", "description": "x", "status": "realized" })
    );
    for (id, loc) in [("art:a", "src/a.rs"), ("art:b", "src/b.rs")] {
        // An in-memory service holds no tree, so the first baseline is asserted.
        call!(
            s,
            link_artifact,
            json!({ "artifact_id": id, "target_id": "cap:x", "name": id, "location": loc,
                    "artifact_type": "code", "checksum": "sha256:v1" })
        );
    }
    call!(
        s,
        add_change_event,
        json!({ "id": "chg:one", "name": "one", "summary": "s", "rationale": "r",
                "change_type": "new_feature", "affected": [{ "node_id": "art:a" }] })
    );
    let accepted = call!(
        s,
        set_artifact_checksums,
        json!({ "accepts": [{ "artifact_id": "art:a", "checksum": "sha256:v2",
                              "disposition": "design_updated", "change_type": "new_feature",
                              "design_change_event_id": "chg:one" }] })
    );
    assert_eq!(
        accepted["applied"],
        json!(true),
        "the acceptance applies: {accepted}"
    );
    s
}

/// The CHANGED edge from `chg:one` to `to`, as the store holds it.
async fn changed_edge(s: &ReflowService, to: &str) -> Value {
    let out = call!(
        s,
        get_node,
        json!({ "id": "chg:one", "include_edges": { "edge_types": ["CHANGED"] } })
    );
    out["edges"]["items"]
        .as_array()
        .expect("edges listed")
        .iter()
        .find(|e| e["to_id"] == to)
        .cloned()
        .unwrap_or_else(|| panic!("no CHANGED edge to {to} in {out}"))
}

#[tokio::test]
async fn the_acceptance_is_there_before_the_revise() {
    let s = seeded().await;
    let p = &changed_edge(&s, "art:a").await["properties"];
    assert_eq!(p["accepted_baseline"], json!(true), "{p}");
    assert_eq!(p["checksum_after"], json!("sha256:v2"), "{p}");
    assert_eq!(p["accepted_seq"], json!(1), "{p}");
}

#[tokio::test]
async fn adding_an_affected_entry_keeps_the_acceptance_on_the_edges_already_drawn() {
    let s = seeded().await;
    let out = call!(
        s,
        add_change_event,
        json!({ "id": "chg:one", "affected": [{ "node_id": "art:a" }, { "node_id": "art:b" }] })
    );
    assert_eq!(
        out["edges_already_present"],
        json!(["chg:one CHANGED art:a"]),
        "the edge that was there is reported as such: {out}"
    );
    assert_eq!(
        out["edges_drawn"],
        json!(["chg:one CHANGED art:b"]),
        "{out}"
    );

    let a = &changed_edge(&s, "art:a").await["properties"];
    assert_eq!(
        a["accepted_baseline"],
        json!(true),
        "the acceptance survives the revise: {a}"
    );
    assert_eq!(a["checksum_after"], json!("sha256:v2"), "{a}");
    assert_eq!(a["accepted_seq"], json!(1), "{a}");
    assert_eq!(a["action"], json!("modified"), "{a}");

    let b = &changed_edge(&s, "art:b").await["properties"];
    assert_eq!(
        b["action"],
        json!("modified"),
        "the new edge is an ordinary one: {b}"
    );
    assert_eq!(b["accepted_baseline"], json!(false), "{b}");
}

#[tokio::test]
async fn a_changed_action_moves_and_the_acceptance_still_stays() {
    let s = seeded().await;
    call!(
        s,
        add_change_event,
        json!({ "id": "chg:one", "affected": [{ "node_id": "art:a", "action": "added" }] })
    );
    let a = &changed_edge(&s, "art:a").await["properties"];
    assert_eq!(
        a["action"],
        json!("added"),
        "the call said a new action, so it moves: {a}"
    );
    assert_eq!(a["accepted_baseline"], json!(true), "{a}");
    assert_eq!(a["checksum_after"], json!("sha256:v2"), "{a}");
}
