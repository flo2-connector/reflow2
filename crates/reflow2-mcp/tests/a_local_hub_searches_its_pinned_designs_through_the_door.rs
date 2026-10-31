//! A HUB ON ONE MACHINE, REACHED ONLY THROUGH THE `--call` DOOR, searches the
//! designs it pins and opens a hit in one — the door's own run of
//! `tests/one_search_reaches_every_design_a_hub_tracks.rs`
//! (`cap:search-design-reaches-the-designs-a-hub-tracks`).
//!
//! A work hub (fifteen designs, organisation policy blocking MCP) has
//! `reflow2 read|write` and nothing else: no registry, no hub server. Measured
//! 2026-10-21 on 0.89.0, `reflow2 read search_design` with `tracked_designs`
//! was refused by the schema and `get_node` with `member` was refused off a
//! hub's address. Here, on the real binary and real stores: a hub pinning one
//! member by its export, one export that is missing, and one design at an
//! address; a word only the member holds.

use std::path::Path;
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

fn run(home: &Path, graph: &Path, verb: &str, tool: &str, args: &Value) -> Output {
    Command::new(bin())
        .arg("--graph-path")
        .arg(graph)
        .args([verb, tool, &args.to_string()])
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("REFLOW2_CONFIG_DIR", home.join("reflow2-config"))
        .env("RUST_LOG", "error")
        .env_remove("REFLOW2_CONTENT_POLICY")
        .env_remove("REFLOW2_TRUSTED_GATEWAY")
        .stdin(Stdio::null())
        .output()
        .expect("the binary runs")
}

fn ok(o: &Output, what: &str) -> Value {
    assert_eq!(
        o.status.code(),
        Some(0),
        "{what}: {}\n{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    serde_json::from_slice(&o.stdout).unwrap_or_else(|e| panic!("{what}: not JSON ({e})"))
}

#[test]
fn a_hub_searches_its_pinned_exports_and_opens_a_hit_through_the_door() {
    let dir = tempfile::Builder::new()
        .prefix("reflow2-door-tracked-")
        .tempdir()
        .unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let member = dir.path().join("deploy/.reflow2/graph");
    let hub = dir.path().join("hub/.reflow2/graph");
    let export = dir.path().join("deploy/design.json");

    ok(
        &run(
            &home,
            &member,
            "write",
            "add_component",
            &json!({"id": "cmp:role-datastream", "name": "datastream role",
                    "description": "deploys the datastream servers"}),
        ),
        "the member's component",
    );
    ok(
        &run(
            &home,
            &member,
            "write",
            "export_graph",
            &json!({"path": export.to_string_lossy()}),
        ),
        "the member's export",
    );
    let graph_id =
        serde_json::from_slice::<Value>(&std::fs::read(&export).unwrap()).unwrap()["graph_id"]
            .as_str()
            .unwrap()
            .to_string();
    let exported = std::fs::read(&export).unwrap();

    ok(
        &run(
            &home,
            &hub,
            "write",
            "add_requirement",
            &json!({"id": "req:instrument-firewall", "name": "instrument firewall fix",
                    "statement": "The instrument firewall admits the stream."}),
        ),
        "the hub's requirement",
    );
    for pin in [
        json!({"id": "dep:deploy", "name": "deploy", "source": "https://example.org/deploy",
               "version": "abc123", "graph_id": graph_id,
               "design_export": export.to_string_lossy()}),
        json!({"id": "dep:storage", "name": "storage", "source": "https://example.org/storage",
               "version": "def456", "graph_id": "storage",
               "design_export": dir.path().join("storage/design.json").to_string_lossy()}),
        json!({"id": "dep:hosted", "name": "hosted", "source": "https://example.org/hosted",
               "version": "v1", "graph_id": "hosted",
               "design_address": "http://127.0.0.1:9/g/hosted/mcp"}),
    ] {
        ok(
            &run(&home, &hub, "write", "external_dependency", &pin),
            "a pin",
        );
    }

    // BEFORE (0.90.0): a plain search finds nothing and names the pins.
    let plain = ok(
        &run(
            &home,
            &hub,
            "read",
            "search_design",
            &json!({"query": "datastream"}),
        ),
        "the plain search",
    );
    assert_eq!(plain["hits"], json!([]), "{plain}");

    // AFTER: one read searches the member's export too.
    let found = ok(
        &run(
            &home,
            &hub,
            "read",
            "search_design",
            &json!({"query": "datastream", "tracked_designs": true}),
        ),
        "the tracked search",
    );
    let id = format!("{graph_id}:cmp:role-datastream");
    assert_eq!(found["hits"][0]["node_id"], json!(id), "{found}");
    assert_eq!(found["hits"][0]["design"], json!(graph_id), "{found}");
    let states: Vec<(&str, &str)> = found["not_searched"]
        .as_array()
        .expect("not_searched")
        .iter()
        .map(|n| (n["design"].as_str().unwrap(), n["state"].as_str().unwrap()))
        .collect();
    assert!(
        states.contains(&("storage", "missing")) && states.contains(&("hosted", "not_reached")),
        "{found}"
    );

    // The hit opens, from the export, through `read`.
    let node = ok(
        &run(&home, &hub, "read", "get_node", &json!({"id": id})),
        "the hit opened",
    );
    assert_eq!(
        node["node"]["properties"]["name"],
        json!("datastream role"),
        "{node}"
    );

    // A write aimed at the member is refused, and nothing is written.
    let o = run(
        &home,
        &hub,
        "write",
        "add_requirement",
        &json!({"id": "req:x", "name": "x", "statement": "x.", "member": graph_id}),
    );
    assert_ne!(
        o.status.code(),
        Some(0),
        "a write into an export is refused"
    );
    assert_eq!(
        std::fs::read(&export).unwrap(),
        exported,
        "the export is untouched"
    );
}
