//! A HUB ON ONE MACHINE, REACHED ONLY THROUGH THE `--call` DOOR, runs ONE
//! composed check over itself and the designs it pins — the door's own run of
//! `tests/one_composed_check_runs_over_a_hub_and_several_of_its_designs.rs`
//! (`req:one-composed-check-runs-over-a-hub-and-several-of-its-designs`).
//!
//! A work hub (fifteen designs, organisation policy blocking MCP) has
//! `reflow2 read|write` and nothing else: no registry, no hub server. Measured
//! 2026-10-21 on 0.90.0, `reflow2 read compose_and_analyse` with
//! `tracked_designs` was refused by the schema, and `other_graph_id` is served
//! only by a registry. Here, on the real binary and real stores: a hub pinning
//! two members by their exports, one export that is missing, and one design at
//! an address; the two members' composition yields a finding of their pair.

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
fn a_hub_composes_itself_with_its_pinned_exports_through_the_door_and_writes_nothing() {
    let dir = tempfile::Builder::new()
        .prefix("reflow2-door-composed-")
        .tempdir()
        .unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let hub = dir.path().join("hub/.reflow2/graph");

    // Two members, each a store of its own with its committed export.
    let mut members: Vec<(String, std::path::PathBuf, Vec<u8>)> = Vec::new();
    for name in ["deploy", "storage"] {
        let store = dir.path().join(format!("{name}/.reflow2/graph"));
        let export = dir.path().join(format!("{name}/design.json"));
        for (tool, args) in [
            (
                "add_project",
                json!({"id": format!("proj:{name}"), "name": name}),
            ),
            (
                "add_requirement",
                json!({"id": "req:feed", "name": format!("{name} feed"),
                       "statement": format!("{name} is fed from the stream.")}),
            ),
            (
                "add_component",
                json!({"id": "cmp:store", "name": format!("{name} store"),
                       "description": "the member's own store"}),
            ),
            ("export_graph", json!({"path": export.to_string_lossy()})),
        ] {
            ok(&run(&home, &store, "write", tool, &args), tool);
        }
        let graph_id =
            serde_json::from_slice::<Value>(&std::fs::read(&export).unwrap()).unwrap()["graph_id"]
                .as_str()
                .unwrap()
                .to_string();
        members.push((graph_id, export.clone(), std::fs::read(&export).unwrap()));
    }

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
    let mut pins: Vec<Value> = members
        .iter()
        .enumerate()
        .map(|(i, (graph_id, export, _))| {
            json!({"id": format!("dep:m{i}"), "name": format!("m{i}"),
                   "source": format!("https://example.org/m{i}"), "version": "abc123",
                   "graph_id": graph_id, "design_export": export.to_string_lossy()})
        })
        .collect();
    pins.push(
        json!({"id": "dep:gone", "name": "gone", "source": "https://example.org/gone",
                     "version": "def456", "graph_id": "gone",
                     "design_export": dir.path().join("gone/design.json").to_string_lossy()}),
    );
    pins.push(
        json!({"id": "dep:hosted", "name": "hosted", "source": "https://example.org/hosted",
                     "version": "v1", "graph_id": "hosted",
                     "design_address": "http://127.0.0.1:9/g/hosted/mcp"}),
    );
    for pin in pins {
        ok(
            &run(&home, &hub, "write", "external_dependency", &pin),
            "a pin",
        );
    }
    let hub_hash = |tag: &str| -> String {
        let path = dir.path().join(format!("hub-{tag}.json"));
        ok(
            &run(
                &home,
                &hub,
                "write",
                "export_graph",
                &json!({"path": path.to_string_lossy()}),
            ),
            "the hub's export",
        );
        serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap()["content_hash"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let before = hub_hash("before");

    // ONE READ composes the hub with every design it pins.
    let composed = ok(
        &run(
            &home,
            &hub,
            "read",
            "compose_and_analyse",
            &json!({"tracked_designs": true}),
        ),
        "the composed check",
    );
    let (a, t) = (members[0].0.as_str(), members[1].0.as_str());
    let hub_id = composed["this_design"].as_str().unwrap().to_string();
    assert_eq!(
        composed["designs_composed"],
        json!([hub_id, a, t]),
        "{composed}"
    );
    let pair = composed["by_pair"][format!("{a}×{t}")]
        .as_array()
        .unwrap_or_else(|| panic!("the two members' pair has a finding: {composed}"));
    let states: Vec<(&str, &str)> = composed["not_composed"]
        .as_array()
        .expect("not_composed")
        .iter()
        .map(|n| (n["design"].as_str().unwrap(), n["state"].as_str().unwrap()))
        .collect();
    assert_eq!(
        states,
        [("gone", "missing"), ("hosted", "not_reached")],
        "{composed}"
    );

    // A finding's id opens, from the member's export, through `read`.
    let id = pair[0]["affected_ids"][0].as_str().unwrap();
    let node = ok(
        &run(&home, &hub, "read", "get_node", &json!({"id": id})),
        "a pair finding's id opened",
    );
    assert!(!node["node"].is_null(), "{node}");

    // NOTHING WAS WRITTEN: the hub's content hash, and every member's export.
    assert_eq!(hub_hash("after"), before, "the hub's design is unchanged");
    for (graph_id, export, bytes) in &members {
        assert_eq!(
            &std::fs::read(export).unwrap(),
            bytes,
            "{graph_id}'s export"
        );
    }
}

/// THROUGH THE DOOR, where the pinned designs touch is computed from their
/// exports (`req:designs-are-shown-where-they-already-touch-before-anyone-declares-it`):
/// a hub pinning three members — two in ONE repository sharing a file, one
/// with only a closely named interface — and one whose export is missing.
#[test]
fn through_the_door_a_hub_is_told_where_its_designs_touch_and_what_was_not_checked() {
    let dir = tempfile::Builder::new()
        .prefix("reflow2-door-touch-")
        .tempdir()
        .unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let hub = dir.path().join("hub/.reflow2/graph");
    let mut ids: Vec<String> = Vec::new();
    for (name, extra) in [
        (
            "deploy",
            json!({"node_type": "Artifact", "id": "art:datastream-tasks",
                   "props": {"name": "datastream role tasks",
                             "location": "roles/datastream/tasks/main.yml"}}),
        ),
        (
            "lab",
            json!({"node_type": "Artifact", "id": "art:role",
                   "props": {"name": "the role it runs",
                             "location": "./roles/datastream/tasks/main.yml"}}),
        ),
        (
            "storage",
            json!({"node_type": "Interface", "id": "ifc:stream",
                   "props": {"name": "Storage stream", "medium": "REST"}}),
        ),
    ] {
        let store = dir.path().join(format!("{name}/.reflow2/graph"));
        let export = dir.path().join(format!("{name}/design.json"));
        for (verb, tool, args) in [
            (
                "write",
                "add_component",
                json!({"id": "cmp:role", "name": format!("{name} role"), "description": "it"}),
            ),
            ("write", "create_node", extra.clone()),
            (
                "write",
                "export_graph",
                json!({"path": export.to_string_lossy()}),
            ),
        ] {
            ok(&run(&home, &store, verb, tool, &args), tool);
        }
        ids.push(
            serde_json::from_slice::<Value>(&std::fs::read(&export).unwrap()).unwrap()["graph_id"]
                .as_str()
                .unwrap()
                .to_string(),
        );
    }
    ok(
        &run(
            &home,
            &hub,
            "write",
            "create_node",
            &json!({"node_type": "Interface", "id": "ifc:stream",
                    "props": {"name": "storage streams", "medium": "REST"}}),
        ),
        "the hub's interface",
    );
    let repo = |name: &str| match name {
        "storage" => "https://github.com/example/storage",
        _ => "https://github.com/Example/deploy",
    };
    for (i, name) in ["deploy", "lab", "storage"].iter().enumerate() {
        ok(
            &run(
                &home,
                &hub,
                "write",
                "external_dependency",
                &json!({"id": format!("dep:{name}"), "name": name, "source": repo(name),
                        "version": "v1", "graph_id": ids[i],
                        "design_export": dir.path().join(format!("{name}/design.json")).to_string_lossy()}),
            ),
            "a pin",
        );
    }
    ok(
        &run(
            &home,
            &hub,
            "write",
            "external_dependency",
            &json!({"id": "dep:gone", "name": "gone", "source": "https://example.org/gone",
                    "version": "v1", "graph_id": "gone",
                    "design_export": dir.path().join("gone/design.json").to_string_lossy()}),
        ),
        "the missing pin",
    );
    let got = ok(
        &run(
            &home,
            &hub,
            "read",
            "compose_and_analyse",
            &json!({"tracked_designs": true}),
        ),
        "the composed check",
    );
    let (a, p, t) = (ids[0].as_str(), ids[1].as_str(), ids[2].as_str());
    let hub_id = got["this_design"].as_str().unwrap().to_string();
    // The shared file: computed, drawn, on the location in one repository.
    assert!(
        got["computed_links"].as_array().unwrap().iter().any(|l| {
            l["evidence"] == json!("same_location")
                && l["from_id"] == json!(format!("{a}:art:datastream-tasks"))
                && l["to_id"] == json!(format!("{p}:art:role"))
                && l["drawn"] == json!(true)
        }),
        "{got}"
    );
    let ls = &got["link_status"];
    let entry = |status: &str, pair: String| -> Value {
        ls[status]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["pair"] == json!(pair))
            .cloned()
            .unwrap_or(Value::Null)
    };
    assert_eq!(
        entry("linked", format!("{a}×{p}"))["computed"],
        json!(1),
        "{ls}"
    );
    // The hub's own interface and storage's: a candidate, not a link.
    assert!(
        entry("candidates_only", format!("{hub_id}×{t}")).is_null()
            && !entry("linked", format!("{hub_id}×{t}")).is_null(),
        "the hub pins storage, so that pair is linked by the pin: {ls}"
    );
    assert!(
        got["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["a_design"] == json!(hub_id) && c["b_design"] == json!(t)),
        "{got}"
    );
    assert_eq!(
        ls["never_touch"],
        json!([format!("{a}×{t}"), format!("{p}×{t}")]),
        "{ls}"
    );
    let not_checked: Vec<&str> = ls["not_checked"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(
        not_checked,
        [
            format!("{hub_id}×gone"),
            format!("{a}×gone"),
            format!("{p}×gone"),
            format!("{t}×gone")
        ],
        "{ls}"
    );
}
