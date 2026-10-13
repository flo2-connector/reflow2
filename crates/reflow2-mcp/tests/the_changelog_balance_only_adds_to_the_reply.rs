//! The changelog's balance line ADDS to two replies and changes nothing else.
//!
//! `cap:changelog-view-carries-a-balance-line` (`dec:how-the-changelog-balance-is-counted`,
//! option C) puts two new things on the wire:
//!
//! - `release_includes_all` with `apply: true` keeps the release's status
//!   snapshot, and its reply gains `status_snapshot` saying it was written and
//!   how many entries it holds;
//! - `changelog_view` gains `balance`: what was added beside what was taken
//!   away between the two ends, and the failures that came back.
//!
//! **The MCP surface only grows** (Anthony, 2026-10-05: *"don't break it for
//! how it was originally used/designed as an mcp server"*). So every key either
//! reply carried before this change is asserted here, still present and of the
//! same JSON kind, beside the new one. A client that reads only the old keys
//! reads exactly what it read before.

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

macro_rules! call {
    ($s:expr, $tool:ident, $args:expr) => {
        $s.$tool(Parameters(
            serde_json::from_value($args).expect("arguments fit the tool"),
        ))
        .await
        .expect("tool ok")
        .structured_content
        .expect("structured content present")
    };
}

/// The JSON kind of a value, so a key that changed type is caught as well as
/// one that vanished.
fn kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn assert_keeps(reply: &Value, before: &[(&str, &str)], tool: &str) {
    for (key, was) in before {
        let v = reply
            .get(*key)
            .unwrap_or_else(|| panic!("{tool} lost `{key}`: {reply}"));
        assert!(
            kind(v) == *was || kind(v) == "null" || *was == "null",
            "{tool}'s `{key}` was {was} and is now {}: {reply}",
            kind(v)
        );
    }
}

#[tokio::test]
async fn the_cut_and_the_changelog_keep_every_key_they_had_and_add_one_each() {
    let s = ReflowService::in_memory().expect("in-memory service");
    call!(
        s,
        add_requirement,
        json!({"id": "req:a", "name": "A", "statement": "Something wanted."})
    );
    call!(
        s,
        add_release,
        json!({"id": "rel:one", "name": "one", "version": "1.0.0", "status": "deployed"})
    );

    // ---- release_includes_all ------------------------------------------
    let manifest = call!(
        s,
        release_includes_all,
        json!({"release_id": "rel:one", "apply": true})
    );
    assert_keeps(
        &manifest,
        &[
            ("release_id", "string"),
            ("entries", "array"),
            ("added", "number"),
            ("already_present", "number"),
            ("excluded", "number"),
            ("without_checksum", "array"),
            ("applied", "bool"),
        ],
        "release_includes_all",
    );
    let snap = &manifest["status_snapshot"];
    assert_eq!(snap["outcome"], "written", "{manifest}");
    assert_eq!(snap["stored_as"], "whole");
    assert_eq!(snap["entries"], 1);
    assert_eq!(snap["snapshot_id"], "snap:release-status:rel:one");

    call!(
        s,
        add_epoch,
        json!({"id": "epoch:one", "name": "one", "epoch_type": "release_cut", "sequence": 10})
    );
    call!(
        s,
        pin_at_epoch,
        json!({"node_type": "Release", "node_id": "rel:one", "epoch_id": "epoch:one"})
    );
    call!(
        s,
        add_requirement,
        json!({"id": "req:b", "name": "B", "statement": "Something else wanted."})
    );

    // ---- changelog_view ------------------------------------------------
    let draft = call!(s, changelog_view, json!({}));
    assert_keeps(
        &draft,
        &[
            ("heading", "string"),
            ("from", "string"),
            ("to", "null"),
            ("from_sequence", "number"),
            ("to_sequence", "null"),
            ("entries", "array"),
            ("manifest", "object"),
            ("unmapped", "array"),
            ("is_draft", "bool"),
            ("needs_a_human", "array"),
            ("notes", "array"),
        ],
        "changelog_view",
    );
    let balance = &draft["balance"];
    assert_eq!(balance["counted"], true, "{draft}");
    assert_eq!(balance["from"]["source"], "snapshot");
    assert_eq!(balance["to"]["source"], "live_design");
    let added: Vec<&str> = balance["requirements"]["added"]["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect();
    assert_eq!(added, ["req:b"]);
    assert!(
        balance["requirements"]["added"]["how_counted"]
            .as_str()
            .is_some_and(|h| !h.is_empty())
    );
}
