//! A link from a hub record to a record in a pinned design says when that
//! record is gone from the design's saved copy, and is checked to exist when
//! it is drawn (`req:a-link-into-a-pinned-design-says-when-its-record-is-gone`).
//!
//! WHAT FAILED. Measured 2026-10-21 on 0.89.0 through the --call door, where
//! no server holds the member: `review_relations` drew an `xref:` to a node
//! the pinned export did not hold, without a check; after the far node was
//! deleted and the member re-exported, `upstream_status` said only `moved` and
//! `link_far_end_moved`, never that the node was gone. On a registry server
//! the same reference reads `stale` or `unreachable` (#776); a hub on one
//! machine had nothing to read it against but the member's committed export,
//! and nothing read it there.
//!
//! THE CLASS: every link into a pinned design whose record is an export reads
//! as exactly one of present, `stale` (the export no longer holds the far
//! node) or `unreachable` (the export cannot be read) — never silently
//! present — and a link to a node the export does not hold is refused before
//! anything is written. Reading the far node's truth from an export is NOT
//! this requirement.

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
    let dir = std::env::temp_dir().join(format!("reflow2-xref-gone-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Export a member holding `ids` as Components, to `<dir>/<graph_id>.json`.
async fn export_of(dir: &std::path::Path, graph_id: &str, ids: &[&str]) -> String {
    let m = ReflowService::in_memory_as(graph_id).expect("member");
    for id in ids {
        call!(
            m,
            add_component,
            json!({"id": id, "name": id, "description": "an ansible role"})
        );
    }
    let path = dir.join(format!("{graph_id}.json"));
    let _ = std::fs::remove_file(&path);
    call!(m, export_graph, json!({"path": path.to_string_lossy()}));
    path.to_string_lossy().into_owned()
}

async fn hub() -> ReflowService {
    let s = ReflowService::in_memory_as("hub").expect("hub");
    call!(s, add_project, json!({"id": "proj:hub", "name": "Hub"}));
    for id in ["req:fw", "req:stream", "req:other"] {
        call!(
            s,
            add_requirement,
            json!({"id": id, "name": id, "statement": format!("{id} holds.")})
        );
    }
    s
}

async fn pin(s: &ReflowService, graph_id: &str, export: &str) {
    call!(
        s,
        external_dependency,
        json!({"id": format!("dep:{graph_id}"), "name": graph_id,
               "source": "https://example.org/x", "version": "abc123",
               "graph_id": graph_id, "design_export": export, "relation": ["uses"]})
    );
}

async fn link(s: &ReflowService, from: &str, design: &str, far: &str) -> Value {
    call!(
        s,
        review_relations,
        json!({"node_id": from, "links": [{
            "relation": "DEPENDS_ON", "other_design": design, "other_id": far,
            "evidence": "the fix is in this role"}]})
    )
}

fn findings_for<'a>(status: &'a Value, xref: &str) -> Vec<&'a str> {
    status["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["dependency"] == json!(xref))
        .filter_map(|f| f["kind"].as_str())
        .collect()
}

#[tokio::test]
async fn every_link_into_an_exported_design_reads_present_stale_or_unreachable() {
    let dir = scratch("class");
    let ansible = export_of(
        &dir,
        "ansible",
        &["cmp:role-nftables", "cmp:role-pdfstream"],
    )
    .await;
    let tiled = export_of(&dir, "tiled", &["cmp:plugin"]).await;
    let s = hub().await;
    pin(&s, "ansible", &ansible).await;
    pin(&s, "tiled", &tiled).await;
    link(&s, "req:fw", "ansible", "cmp:role-nftables").await;
    link(&s, "req:stream", "ansible", "cmp:role-pdfstream").await;
    link(&s, "req:other", "tiled", "cmp:plugin").await;

    // Upstream, the nftables role is removed and the member re-exported; the
    // tiled plugins' export goes missing.
    export_of(&dir, "ansible", &["cmp:role-pdfstream"]).await;
    std::fs::remove_file(&tiled).expect("remove");

    let status = j!(s.upstream_status(Parameters(UpstreamStatusReq {})));
    let gone = findings_for(&status, "xref:ansible:cmp:role-nftables");
    assert!(
        gone.contains(&"link_far_end_stale"),
        "a far node gone from the export reads stale: {status}"
    );
    let kept = findings_for(&status, "xref:ansible:cmp:role-pdfstream");
    assert!(
        !kept.contains(&"link_far_end_stale") && !kept.contains(&"link_far_end_unreachable"),
        "a far node the export still holds is not stale: {status}"
    );
    let lost = findings_for(&status, "xref:tiled:cmp:plugin");
    assert!(
        lost.contains(&"link_far_end_unreachable"),
        "a link into a design whose export is missing reads unreachable: {status}"
    );
    let detail = status["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["kind"] == json!("link_far_end_stale"))
        .and_then(|f| f["detail"].as_str())
        .unwrap();
    assert!(
        detail.contains("cmp:role-nftables") && detail.contains(&ansible),
        "{detail}"
    );
}

#[tokio::test]
async fn a_link_to_a_node_the_pinned_export_does_not_hold_is_refused_and_nothing_is_written() {
    let dir = scratch("drawn");
    let ansible = export_of(&dir, "ansible", &["cmp:role-pdfstream"]).await;
    let s = hub().await;
    pin(&s, "ansible", &ansible).await;
    let refused = s
        .review_relations(Parameters(
            serde_json::from_value(json!({"node_id": "req:fw", "links": [{
                "relation": "DEPENDS_ON", "other_design": "ansible",
                "other_id": "cmp:role-nftables", "evidence": "the fix is in this role"}]}))
            .unwrap(),
        ))
        .await;
    let message = match refused {
        Err(e) => e.message.to_string(),
        Ok(r) if r.is_error == Some(true) => format!("{r:?}"),
        Ok(r) => panic!("a link to a node the pinned export does not hold is refused: {r:?}"),
    };
    assert!(
        message.contains("cmp:role-nftables") && message.contains(&ansible),
        "the refusal names the node and the export it was checked against: {message}"
    );
    let xref = call!(s, get_node, json!({"id": "xref:ansible:cmp:role-nftables"}));
    assert_eq!(xref["node"], Value::Null, "nothing was written: {xref}");

    // The node the export does hold links as before.
    let ok = link(&s, "req:stream", "ansible", "cmp:role-pdfstream").await;
    assert_eq!(ok["drawn"].as_array().map(Vec::len), Some(1), "{ok}");
}
