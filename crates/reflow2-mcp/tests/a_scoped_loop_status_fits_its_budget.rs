//! The owner-scoped `loop_status`, and `scan_nodes`, fit the reply budget a
//! client holds them to — and say what they withheld.
//!
//! MEASURED 2026-10-14 on main's design copy (ea6329305), compact JSON: the
//! scoped `loop_status` grew from 10,249 characters to 38,939 when it began
//! listing what awaits the owner's word (`awaiting_word`, 27,951 of them), and
//! it took no `budget_chars`. flo2.io's gateway holds every engine reply to
//! 30,000 characters and REPLACES a larger one with `reply_too_large`; it asks
//! a tool for less only when the tool's schema takes `budget_chars`. So "what
//! needs me" on a hosted design would have gone from an answer to a refusal.
//! `scan_nodes` was already over on main for every large type (37,645 to
//! 40,110 characters non-brief), because its own cap was 40,000 bytes of items.
//!
//! The rule both now follow is `reply_budget`'s: a shorter answer is never a
//! quieter one. Counts and ids are never budgeted away; the awaiting items'
//! prose (long names, the evidence sentence) goes first, then rows from the
//! tail, and the reply's `budget` block says what was withheld and how to see
//! the rest.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "reply-budget-probe".to_string();
        cfg
    }
}

struct Session {
    client: rmcp::service::RunningService<rmcp::service::RoleClient, Probe>,
}

async fn session() -> Session {
    let svc = ReflowService::in_memory().expect("in-memory service");
    let (server_rx, client_tx) = tokio::io::duplex(1 << 23);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 23);
    tokio::spawn(async move {
        if let Ok(running) = svc.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    let client = Probe
        .serve((client_rx, client_tx))
        .await
        .expect("in-process handshake");
    Session { client }
}

impl Session {
    async fn ok(&self, tool: &str, args: Value) -> Value {
        let shown: String = args.to_string().chars().take(200).collect();
        let arguments = args.as_object().cloned().unwrap_or_default();
        let r = self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
            .unwrap_or_else(|e| panic!("`{tool}` {shown} failed: {e}"));
        assert!(
            !r.is_error.unwrap_or(false),
            "`{tool}` {shown} answered an error: {:?}",
            r.content
        );
        r.structured_content.unwrap_or(Value::Null)
    }

    async fn node(&self, node_type: &str, id: &str, props: Value) {
        self.ok(
            "create_node",
            json!({"node_type": node_type, "id": id, "props": props}),
        )
        .await;
    }

    async fn edge(&self, edge_type: &str, from: &str, to: &str) {
        self.ok(
            "create_edge",
            json!({"edge_type": edge_type, "from_id": from, "to_id": to}),
        )
        .await;
    }
}

/// A long name, as the ideas and requirements a real design holds have.
fn long_name(i: usize) -> String {
    format!(
        "Requirement {i}: {}",
        "the owner wants this part built, checked and kept honest across every release "
            .repeat(7)
    )
}

const AWAITING: usize = 80;

/// `AWAITING` proposed requirements with long names, every one delivered by
/// one realized capability with a passing check, and a contributor to ask for.
async fn many_awaiting() -> Session {
    let s = session().await;
    s.ok("add_contributor", json!({"id": "who:owner", "name": "The Owner"}))
        .await;
    s.node(
        "Capability",
        "cap:delivers",
        json!({"name": "Delivers", "description": "does it", "status": "realized"}),
    )
    .await;
    s.node(
        "Artifact",
        "art:code",
        json!({"name": "code", "location": "src/it.rs", "status": "realized"}),
    )
    .await;
    s.edge("REALIZES", "art:code", "cap:delivers").await;
    s.ok(
        "add_verification",
        json!({"id": "ver:green", "name": "green", "status": "passing",
               "verifies": [{"target_id": "cap:delivers"}]}),
    )
    .await;
    for i in 0..AWAITING {
        let id = format!("req:shipped-{i:03}");
        s.node(
            "Requirement",
            &id,
            json!({"name": long_name(i), "statement": "it must hold", "status": "proposed"}),
        )
        .await;
        s.edge("SATISFIES", "cap:delivers", &id).await;
    }
    s
}

fn chars(v: &Value) -> usize {
    v.to_string().len()
}

fn awaiting_ids(r: &Value) -> Vec<String> {
    r["awaiting_word"]
        .as_array()
        .unwrap_or_else(|| panic!("awaiting_word is a list: {r}"))
        .iter()
        .map(|i| i["node_id"].as_str().expect("every item keeps its id").to_string())
        .collect()
}

fn says_the_count(r: &Value) -> bool {
    r["next"].as_array().is_some_and(|n| {
        n.iter().any(|l| {
            l.as_str()
                .is_some_and(|l| l.starts_with(&format!("{AWAITING} built item(s)")))
        })
    })
}

#[tokio::test]
async fn a_scoped_loop_status_with_many_awaiting_items_fits_the_default_budget_and_keeps_every_id()
{
    let s = many_awaiting().await;
    let r = s
        .ok("loop_status", json!({"contributor_id": "who:owner"}))
        .await;
    assert!(
        chars(&r) <= 30_000,
        "the scoped answer fits the 30,000-character default a gateway holds it to: {} chars",
        chars(&r)
    );
    let ids = awaiting_ids(&r);
    assert_eq!(
        ids.len(),
        AWAITING,
        "prose goes before rows: trimming names and evidence is enough here, so every item \
         stays: {:?}",
        r["budget"]
    );
    for item in r["awaiting_word"].as_array().unwrap() {
        assert!(
            item["closed_by"]
                .as_str()
                .is_some_and(|c| c.starts_with("set_requirement_status")),
            "every item keeps the call that closes it: {item}"
        );
        assert_eq!(item["evidence_ids"], json!(["cap:delivers"]), "{item}");
    }
    assert!(says_the_count(&r), "the count is never budgeted: {:?}", r["next"]);
    assert_eq!(r["budget"]["applied"], true, "and it says it trimmed: {}", r["budget"]);
    assert!(
        r["budget"]["note"].as_str().is_some_and(|n| n.contains("awaiting_word")),
        "naming where: {}",
        r["budget"]
    );

    // The every-session call is untouched: it fits, so it carries no block.
    let unscoped = s.ok("loop_status", json!({})).await;
    assert!(unscoped.get("budget").is_none(), "{unscoped}");
    assert_eq!(unscoped["awaiting_word"], json!([]));
}

#[tokio::test]
async fn an_explicit_small_budget_is_honoured_and_says_what_it_withheld() {
    let s = many_awaiting().await;
    let budget = 9_000;
    let r = s
        .ok(
            "loop_status",
            json!({"contributor_id": "who:owner", "budget_chars": budget}),
        )
        .await;
    assert!(
        chars(&r) <= budget,
        "budget_chars {budget} is honoured: {} chars, {}",
        chars(&r),
        r["budget"]
    );
    let shortened = r["budget"]["lists_shortened"]
        .as_array()
        .unwrap_or_else(|| panic!("rows went, and the block names the list: {}", r["budget"]));
    let row = shortened
        .iter()
        .find(|l| l["path"] == "awaiting_word")
        .unwrap_or_else(|| panic!("awaiting_word is the list shortened: {shortened:?}"));
    assert_eq!(row["total"], AWAITING, "{row}");
    let shown = row["shown"].as_u64().expect("shown") as usize;
    assert_eq!(awaiting_ids(&r).len(), shown, "`shown` is what the list holds");
    assert!(shown >= 1 && shown < AWAITING, "{row}");
    assert!(says_the_count(&r), "the count stays whole: {:?}", r["next"]);
    for item in r["awaiting_word"].as_array().unwrap() {
        assert!(item["node_id"].as_str().is_some() && item["closed_by"].as_str().is_some());
    }
}

#[tokio::test]
async fn scan_nodes_fits_the_default_budget_whole_reply_and_honours_a_smaller_one() {
    let s = session().await;
    let total = 120;
    for i in 0..total {
        s.node(
            "Requirement",
            &format!("req:r{i:03}"),
            json!({"name": format!("r{i}"),
                   "statement": "a long statement a person wrote down, ".repeat(12)}),
        )
        .await;
    }
    let r = s.ok("scan_nodes", json!({"node_type": "Requirement"})).await;
    assert!(
        chars(&r) <= 30_000,
        "the whole reply, not only its items, fits the 30,000 default: {} chars",
        chars(&r)
    );
    assert_eq!(r["total"], total);
    assert_eq!(r["capped_by"], "size", "{}", r["capped_by"]);
    assert!(r["next_offset"].is_number());

    let small = s
        .ok(
            "scan_nodes",
            json!({"node_type": "Requirement", "budget_chars": 5_000}),
        )
        .await;
    assert!(chars(&small) <= 5_000, "{} chars", chars(&small));
    assert_eq!(small["total"], total);
    assert_eq!(small["capped_by"], "size");
    assert_eq!(
        small["omitted"].as_u64().unwrap() + small["returned"].as_u64().unwrap(),
        total as u64,
        "what came back and what did not add up to the total"
    );
}
