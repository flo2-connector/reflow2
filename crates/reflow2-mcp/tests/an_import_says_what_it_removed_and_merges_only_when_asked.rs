//! An import says what it removed or changed on records that already existed,
//! and keeps what a partial node leaves out only when the call asks it to
//! merge (`req:an-import-says-what-it-removed-and-merges-only-when-asked`,
//! governed by `dec:import-merges-or-demands-whole-nodes`, accepted
//! 2026-10-21: whole nodes by default, merge only when asked).
//!
//! WHAT FAILED. A work hub kept a member design in step by importing 454
//! Artifacts as `{name, location}`. The import replaces a node whole, so
//! `checksum`, `checksum_basis` and `status` were dropped on all 454, and the
//! reply said `nodes_written: 454` and nothing else. Reproduced on 0.89.0:
//! `artifact_type` was reset from `config` to the schema default `code` as
//! well, and the reply filed that under `materialized`, the field for a
//! default the document left implicit, as if nothing had been there.
//!
//! THE CLASS, not the instance: for EVERY node a document names that the
//! design already held, whatever its type, what the reply counts as removed
//! and changed is exactly what a read-back shows was removed and changed; a
//! value overwritten is never counted as a default filled in; and with
//! `merge` nothing the document leaves out is lost.

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

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

/// Three node types, each stated whole: what a member design holds before a
/// bulk sync touches it.
fn whole() -> Value {
    json!({"nodes": [
        {"node_type": "Artifact", "node_id": "art:a", "properties": {
            "name": "a", "location": "roles/a/main.yml", "checksum": "sha256:aaa",
            "checksum_basis": "bytes", "status": "realized", "description": "role a",
            "artifact_type": "config"}},
        {"node_type": "Artifact", "node_id": "art:b", "properties": {
            "name": "b", "location": "roles/b/main.yml", "checksum": "sha256:bbb",
            "checksum_basis": "bytes", "status": "verified", "description": "role b",
            "artifact_type": "config"}},
        {"node_type": "Requirement", "node_id": "req:r", "properties": {
            "name": "r", "statement": "The okapi is fed.", "priority": "high",
            "status": "accepted", "kind": "functional"}},
        {"node_type": "Component", "node_id": "cmp:c", "properties": {
            "name": "c", "purpose": "feeds the okapi", "description": "the feeder",
            "status": "realized"}}
    ], "edges": []})
}

/// The same nodes as a partial document: each keeps only what its type
/// requires, and changes one value.
fn partial() -> Value {
    json!({"nodes": [
        {"node_type": "Artifact", "node_id": "art:a", "properties": {
            "name": "a", "location": "roles/a/tasks/main.yml"}},
        {"node_type": "Artifact", "node_id": "art:b", "properties": {
            "name": "b", "location": "roles/b/tasks/main.yml"}},
        {"node_type": "Requirement", "node_id": "req:r", "properties": {
            "name": "r", "statement": "The okapi is fed twice a day."}},
        {"node_type": "Component", "node_id": "cmp:c", "properties": {
            "name": "c", "purpose": "feeds the okapi twice"}}
    ], "edges": []})
}

async fn read(s: &ReflowService, id: &str) -> Map<String, Value> {
    let got = call!(s, get_node, json!({"id": id}));
    got["node"]["properties"]
        .as_object()
        .cloned()
        .unwrap_or_else(|| panic!("{id} is held: {got}"))
}

const IDS: [(&str, &str); 4] = [
    ("Artifact", "art:a"),
    ("Artifact", "art:b"),
    ("Requirement", "req:r"),
    ("Component", "cmp:c"),
];

async fn snapshot(s: &ReflowService) -> BTreeMap<&'static str, Map<String, Value>> {
    let mut out = BTreeMap::new();
    for (_, id) in IDS {
        out.insert(id, read(s, id).await);
    }
    out
}

fn counts(v: &Value) -> BTreeMap<String, u64> {
    v.as_object()
        .map(|o| {
            o.iter()
                .map(|(k, n)| (k.clone(), n.as_u64().unwrap_or(0)))
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn every_property_an_import_removes_or_changes_is_counted_as_a_read_back_shows() {
    let s = ReflowService::in_memory().expect("service");
    call!(s, import_graph, json!({"document": whole()}));
    let before = snapshot(&s).await;

    let reply = call!(s, import_graph, json!({"document": partial()}));
    let after = snapshot(&s).await;

    // What a read-back shows, by Type.property.
    let mut removed: BTreeMap<String, u64> = BTreeMap::new();
    let mut changed: BTreeMap<String, u64> = BTreeMap::new();
    for (node_type, id) in IDS {
        let (b, a) = (&before[id], &after[id]);
        for (k, v) in b {
            match a.get(k) {
                None => *removed.entry(format!("{node_type}.{k}")).or_default() += 1,
                Some(now) if now != v => {
                    *changed.entry(format!("{node_type}.{k}")).or_default() += 1
                }
                Some(_) => {}
            }
        }
    }
    assert!(
        !removed.is_empty(),
        "the default is still whole nodes: a partial document removes what it leaves out"
    );
    assert_eq!(
        counts(&reply["properties_removed"]),
        removed,
        "the reply counts every property removed from a node that existed: {reply}"
    );
    assert_eq!(
        counts(&reply["properties_changed"]),
        changed,
        "the reply counts every value changed on a node that existed: {reply}"
    );
    assert_eq!(
        counts(&reply["properties_changed"]).get("Artifact.artifact_type"),
        Some(&2),
        "config reset to the default code is a change: {reply}"
    );
    // A value overwritten is never reported as a default filled in.
    for key in counts(&reply["materialized"]).keys() {
        let (t, p) = key.split_once('.').expect("Type.property");
        for (node_type, id) in IDS {
            assert!(
                !(node_type == t && before[id].contains_key(p)),
                "`materialized` names {key}, which {id} already held: {reply}"
            );
        }
    }
    // It names a few, so the agent can look before it moves on.
    let named = reply["removed_from"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        named.iter().any(|n| n
            .as_str()
            .is_some_and(|t| t.contains("art:a") && t.contains("checksum"))),
        "the reply names a node and what it lost: {reply}"
    );
}

#[tokio::test]
async fn with_merge_nothing_a_partial_node_leaves_out_is_lost() {
    let s = ReflowService::in_memory().expect("service");
    call!(s, import_graph, json!({"document": whole()}));
    let before = snapshot(&s).await;

    let reply = call!(
        s,
        import_graph,
        json!({"document": partial(), "merge": true})
    );
    let after = snapshot(&s).await;
    let stated = partial();
    for (i, (_, id)) in IDS.iter().enumerate() {
        let said = stated["nodes"][i]["properties"].as_object().unwrap();
        for (k, v) in &before[id] {
            let expected = said.get(k).unwrap_or(v);
            assert_eq!(
                after[id].get(k),
                Some(expected),
                "{id}.{k}: a merge keeps what the document leaves out and takes what it states"
            );
        }
    }
    assert!(
        counts(&reply["properties_removed"]).is_empty(),
        "a merge removes nothing: {reply}"
    );
    assert_eq!(
        counts(&reply["properties_changed"]).get("Artifact.location"),
        Some(&2),
        "a merge still says what it changed: {reply}"
    );
}

#[tokio::test]
async fn a_round_trip_of_the_designs_own_export_reports_nothing_removed_or_changed() {
    let s = ReflowService::in_memory().expect("service");
    call!(s, import_graph, json!({"document": whole()}));
    let exported = call!(s, export_graph, json!({}));
    let doc = exported.get("document").cloned().unwrap_or(exported);
    let reply = call!(s, import_graph, json!({"document": doc}));
    assert!(
        reply.get("properties_removed").is_none() && reply.get("properties_changed").is_none(),
        "re-importing what the design already says changes nothing, and says nothing: {reply}"
    );
}

/// The served description says a partial node is refused ONLY when it lacks a
/// property its type requires; this asserts the refusal half, with or without
/// `merge`, and that nothing is written.
#[tokio::test]
async fn a_node_missing_a_required_property_is_refused_and_nothing_is_written() {
    let s = ReflowService::in_memory().expect("service");
    call!(s, import_graph, json!({"document": whole()}));
    let before = snapshot(&s).await;
    for merge in [false, true] {
        let doc = json!({"nodes": [
            {"node_type": "Requirement", "node_id": "req:r", "properties": {"name": "r"}},
            {"node_type": "Artifact", "node_id": "art:a", "properties": {"name": "a"}}
        ], "edges": []});
        let refused = s
            .import_graph(Parameters(
                serde_json::from_value(json!({"document": doc, "merge": merge})).unwrap(),
            ))
            .await;
        if !merge {
            let message = match refused {
                Err(e) => e.message.to_string(),
                Ok(r) => panic!("a Requirement with no statement is refused: {r:?}"),
            };
            assert!(message.contains("statement"), "{message}");
        }
        assert_eq!(
            snapshot(&s).await,
            before,
            "the design is unchanged (merge: {merge}; a merge completes the node from what it holds)"
        );
    }
}
