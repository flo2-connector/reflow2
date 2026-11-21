//! EVERY WRITE IS CLEANED OF HIDDEN CHARACTERS, AND SAYS SO; AN IMPORT IS
//! REPORTED ON, NEVER REWRITTEN
//! (`req:every-write-and-every-import-is-cleaned-of-hidden-characters-and-says-what-was-stripped`).
//!
//! # The failure this pins
//!
//! `reflow2_core::sanitize_text` ran only on INGEST extraction. Every write tool
//! stored what it was sent, so on a design several people write to, text holding
//! Unicode tag characters (an invisible alphabet), bidirectional overrides or
//! zero-width formatting reached the next reader's agent unchanged. Found by an
//! outside review of v0.101.0
//! (`fact:root-cause-bl-41s-trigger-for-the-mechanical-half-is-met-on-shared-hosted-designs-and-nothing-re-raised-it-2026-11-20`).
//! Measured on the unfixed build: a write's statement came back with every hidden
//! character in it, and the reply said nothing.
//!
//! # The contract
//!
//! - a write's text is cleaned before it is stored, and the reply's `sanitized`
//!   block names the kinds and the fields;
//! - an id holding hidden characters is REFUSED, never cleaned, and nothing is
//!   stored (the store accepts such ids, so `req:a\u{200B}b` and `req:ab` read
//!   the same and differ);
//! - `import_graph` stores its document as given and reports what it holds under
//!   `hidden_characters` (an export's content is its identity);
//! - a clean write carries no `sanitized` block.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "a-write-is-cleaned-of-hidden-characters".to_string();
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

async fn call(c: &Client, tool: &str, args: Value) -> Result<Value, String> {
    let Value::Object(arguments) = args else {
        panic!("arguments for {tool} must be an object");
    };
    match c
        .call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments))
        .await
    {
        Ok(r) if r.is_error == Some(true) => Err(format!("{:?}", r.content)),
        Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
        Err(e) => Err(e.to_string()),
    }
}

async fn stored_statement(c: &Client, id: &str) -> Option<String> {
    let got = call(c, "get_node", json!({"node_id": id})).await.ok()?;
    got.pointer("/node/properties/statement")
        .and_then(Value::as_str)
        .map(String::from)
}

const HIDDEN: &str = "pump\u{200B} must\u{E0041}\u{E0042} hold\u{202E} pressure";
const CLEAN: &str = "pump must hold pressure";

#[tokio::test]
async fn a_write_stores_clean_text_and_says_what_it_removed() {
    let c = connect().await;
    let reply = call(
        &c,
        "add_requirement",
        json!({"id": "req:pump-holds-pressure", "name": "Pump holds pressure", "statement": HIDDEN}),
    )
    .await
    .expect("the write is accepted");
    assert_eq!(
        stored_statement(&c, "req:pump-holds-pressure")
            .await
            .as_deref(),
        Some(CLEAN),
        "the stored statement must hold none of the hidden characters"
    );
    let block = reply.get("sanitized").unwrap_or_else(|| {
        panic!("the reply must say what was removed, never strip silently: {reply}")
    });
    assert_eq!(block["stripped"]["unicode_tag"], 2, "{block}");
    assert_eq!(block["stripped"]["bidi_control"], 1, "{block}");
    assert_eq!(block["stripped"]["hidden_formatting"], 1, "{block}");
    assert!(
        block["fields"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f == "statement"),
        "the field is named: {block}"
    );
}

#[tokio::test]
async fn an_id_holding_hidden_characters_is_refused_and_nothing_is_stored() {
    let c = connect().await;
    let refused = call(
        &c,
        "add_requirement",
        json!({"id": "req:pump\u{200B}-holds", "name": "Pump", "statement": "fine"}),
    )
    .await
    .expect_err("an id with a hidden character must be refused, not cleaned");
    assert!(refused.contains("never cleaned"), "{refused}");
    assert!(
        stored_statement(&c, "req:pump-holds").await.is_none(),
        "the cleaned id must not have been written"
    );
}

#[tokio::test]
async fn a_clean_write_carries_no_sanitized_block() {
    let c = connect().await;
    let reply = call(
        &c,
        "add_requirement",
        json!({"id": "req:plain", "name": "Plain", "statement": CLEAN}),
    )
    .await
    .expect("accepted");
    assert!(reply.get("sanitized").is_none(), "{reply}");
}

#[tokio::test]
async fn an_import_stores_its_document_as_given_and_reports_what_it_holds() {
    let c = connect().await;
    let doc = json!({
        "nodes": [{
            "node_type": "Requirement",
            "node_id": "req:imported",
            "properties": {"name": "Imported", "statement": HIDDEN, "status": "proposed"}
        }],
        "edges": []
    });
    let reply = call(&c, "import_graph", json!({"document": doc}))
        .await
        .expect("the import is accepted");
    assert_eq!(
        stored_statement(&c, "req:imported").await.as_deref(),
        Some(HIDDEN),
        "an import must store its document exactly as given"
    );
    assert!(
        reply.get("sanitized").is_none(),
        "an import is not cleaned: {reply}"
    );
    let found = reply
        .get("hidden_characters")
        .unwrap_or_else(|| panic!("the import must report what its document holds: {reply}"));
    assert_eq!(found["found"]["unicode_tag"], 2, "{found}");
    assert!(
        found["fields"].as_array().unwrap().iter().any(|f| f
            .as_str()
            .is_some_and(|s| s.contains("req:imported") && s.contains("statement"))),
        "{found}"
    );
}
