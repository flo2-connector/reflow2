//! Declaring a dependency replies with a RECEIPT for that one declaration, and
//! a RE-DECLARE keeps what it was not passed.
//!
//! # The two failures this pins
//!
//! `fact:root-cause-external-dependency-replies-with-the-whole-manifest-because-the-receipt-shapes-only-node-and-edge-records-2026-10-02`
//! (the VS Code `--call` field report, measured on 0.77.0): `external_dependency`
//! replied with the WHOLE dependency manifest as TOML — 486, 636, 786, 936,
//! 1,086 and 1,236 bytes for one to six declarations, +150 per declaration —
//! naming no node and saying nothing about what a re-declare changed. The
//! receipt layer recognises node and edge records, so a write that replied
//! with anything else was never shaped.
//!
//! `fact:re-declaring-a-dependency-drops-the-resources-description-2026-09-23`
//! (eight field occurrences, 0.66 to 0.77): a re-declare wrote through a
//! REPLACING constructor, so the Resource's description (2 to 5 KB of pin
//! history each time), `components`, `features` and the watch were cleared,
//! and the manifest reply hid it.
//!
//! OBSERVED FAILING on main at 293f957, through `--call` on a scratch store:
//! the reply was `{"value": "<TOML>"}` with no `node_id`, and grew from 444 to
//! 969 bytes over one to six declarations (+105 each with these short
//! values); a re-declare without `components`, `features` or `note` left
//! `components` and `features` empty and the description gone.
//!
//! Driven through `call_tool`, where the receipt is made, exactly as a session
//! reaches the tool.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "a-dependency-declaration-replies-with-a-receipt".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

async fn connect() -> Client {
    let service = ReflowService::in_memory().expect("in-memory service");
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = service.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    TestClient
        .serve((client_rx, client_tx))
        .await
        .expect("the in-process handshake")
}

async fn call(c: &Client, tool: &str, args: Value) -> Value {
    let Value::Object(arguments) = args else {
        panic!("arguments for {tool} must be an object");
    };
    let r = c
        .call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments))
        .await
        .unwrap_or_else(|e| panic!("{tool}: {e}"));
    assert_ne!(r.is_error, Some(true), "{tool} refused: {:?}", r.content);
    r.structured_content.unwrap_or(Value::Null)
}

async fn get(c: &Client, id: &str) -> Value {
    call(c, "get_node", json!({"id": id})).await["node"]["properties"].clone()
}

fn chars(v: &Value) -> usize {
    v.to_string().chars().count()
}

#[tokio::test]
async fn a_declaration_replies_with_its_own_record_and_does_not_grow_with_the_others() {
    let c = connect().await;
    let mut sizes = Vec::new();
    for i in 1..=6 {
        let reply = call(
            &c,
            "external_dependency",
            json!({"id": format!("dep:lib{i}"), "name": format!("lib{i}"),
                   "source": "https://example.org/lib.git", "version": "v1.0.0",
                   "components": ["core"], "features": ["fast"]}),
        )
        .await;
        assert_eq!(reply["node_id"], format!("dep:lib{i}"), "{reply:#}");
        assert_eq!(reply["node_type"], "Resource", "{reply:#}");
        assert_eq!(reply["properties"]["version"], "v1.0.0", "{reply:#}");
        for j in 1..i {
            assert!(
                !reply.to_string().contains(&format!("dep:lib{j}")),
                "the receipt for dep:lib{i} carries another declaration (dep:lib{j}): {reply}"
            );
        }
        sizes.push(chars(&reply));
    }
    let (first, last) = (sizes[0], sizes[5]);
    assert!(
        last.abs_diff(first) < 20,
        "a declaration's reply must not grow with how many others exist: {sizes:?}"
    );
    // The whole manifest is still one read away, and that read writes nothing.
    let manifest = call(&c, "reconcile_dependencies", json!({})).await;
    assert!(
        manifest["manifest"]
            .as_str()
            .is_some_and(|m| (1..=6).all(|i| m.contains(&format!("[dependencies.lib{i}]")))),
        "{manifest:#}"
    );
}

#[tokio::test]
async fn a_re_declare_keeps_what_it_was_not_passed_and_says_what_it_changed() {
    let c = connect().await;
    call(
        &c,
        "external_dependency",
        json!({"id": "dep:up", "name": "upstream", "source": "https://example.org/up.git",
               "version": "v1.0.0", "components": ["core", "cli"], "features": ["fast"],
               "declared_in": "Cargo.toml", "graph_id": "upstream",
               "design_export": "/no/such/export.json",
               "note": "Pinned at v1.0.0 because v0.9 dropped the stdio door."}),
    )
    .await;
    // History a person wrote on the Resource itself, as the hub sessions did.
    let history = "PIN HISTORY. ".repeat(40);
    call(
        &c,
        "add_resource",
        json!({"id": "dep:up", "description": history.clone()}),
    )
    .await;

    // The recurring job: move the version, say nothing else.
    let reply = call(
        &c,
        "external_dependency",
        json!({"id": "dep:up", "name": "upstream", "source": "https://example.org/up.git",
               "version": "v1.1.0"}),
    )
    .await;
    let stored = get(&c, "dep:up").await;
    assert_eq!(stored["version"], "v1.1.0");
    assert_eq!(stored["components"], "core,cli", "{stored:#}");
    assert_eq!(stored["features"], "fast", "{stored:#}");
    assert_eq!(stored["declared_in"], "Cargo.toml", "{stored:#}");
    assert_eq!(stored["dependency_graph_id"], "upstream", "{stored:#}");
    assert_eq!(
        stored["description"].as_str(),
        Some(history.as_str()),
        "a re-declare keeps the Resource's description"
    );
    assert_eq!(
        stored["design_export"], "/no/such/export.json",
        "and keeps the watch it was not told to drop"
    );
    let rev = &reply["revision"];
    assert_eq!(rev["changed"], true, "{reply:#}");
    let replaced: Vec<&str> = rev["replaced"]
        .as_array()
        .expect("replaced")
        .iter()
        .filter_map(|r| r["field"].as_str())
        .collect();
    assert_eq!(replaced, ["version"], "only the version moved: {reply:#}");
    assert!(
        chars(&reply) < 3_000,
        "a receipt, not the stored history: {} chars",
        chars(&reply)
    );

    // An explicit empty list clears; an explicit "" stops the watch — and the
    // receipt says what left.
    let reply = call(
        &c,
        "external_dependency",
        json!({"id": "dep:up", "name": "upstream", "source": "https://example.org/up.git",
               "version": "v1.1.0", "features": [], "design_export": ""}),
    )
    .await;
    let stored = get(&c, "dep:up").await;
    assert_eq!(stored["features"], "", "{stored:#}");
    assert!(stored.get("design_export").is_none(), "{stored:#}");
    let removed: Vec<&str> = reply["revision"]["removed"]
        .as_array()
        .expect("a removal is named")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(removed, ["design_export"], "{reply:#}");
    assert_eq!(stored["components"], "core,cli", "still kept: {stored:#}");
}
