//! A REASONING SUMMARY KEEPS ITS ROOM, AND THE REPLY KEEPS TO ITS BUDGET.
//!
//! Seen on flo2.io (reflow2 0.92.0):
//! `fact:reasoning-report-at-6000-blanked-its-summary-and-answered-9529-characters-2026-10-27`,
//! after `fact:the-zero-rung-blanks-every-name-over-80-characters-so-a-reasoning-summary-reads-as-dots-2026-10-25`.
//! reasoning_report at budget_chars 6,000 kept its one claim whole, gave the
//! summary 0 room so every name in it read "…", and still answered 9,529
//! characters. The claims page took the budget first, and the last prose rung
//! cut every string over 80 characters to nothing.
//!
//! Now the summary's room is reserved first, and its names are never cut below
//! 80 characters. When one claim alone will not fit beside it, the reply
//! returns no claim, the summary, `next_offset` at that claim, and the budget
//! the claim needs; it stays within the budget asked
//! (req:no-reply-grows-with-the-design-each-fills-its-budget-and-says-how-to-read-on).

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "summary-room-probe".to_string();
        cfg
    }
}

struct Session {
    client: rmcp::service::RunningService<rmcp::service::RoleClient, Probe>,
}

async fn session() -> Session {
    let svc = ReflowService::in_memory().expect("in-memory service");
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
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
        let arguments = args.as_object().cloned().unwrap_or_default();
        let r = self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
            .unwrap_or_else(|e| panic!("`{tool}`: {e}"));
        assert!(!r.is_error.unwrap_or(false), "`{tool}` was refused: {r:?}");
        r.structured_content.unwrap_or(Value::Null)
    }
}

fn chars(v: &Value) -> usize {
    serde_json::to_string(v).unwrap().len()
}

/// Every string outside `claims` and `page` that the budget cut.
fn cut_outside_claims(v: &Value, at: &str, out: &mut Vec<String>) {
    match v {
        Value::String(s) if s.ends_with('…') => out.push(format!("{at}: {s}")),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .for_each(|(i, x)| cut_outside_claims(x, &format!("{at}[{i}]"), out)),
        Value::Object(o) => o
            .iter()
            .filter(|(k, _)| !(at.is_empty() && matches!(k.as_str(), "claims" | "page")))
            .for_each(|(k, x)| cut_outside_claims(x, &format!("{at}.{k}"), out)),
        _ => {}
    }
}

/// The sinogram's shape: one prediction whose words alone are longer than the
/// budget, resting on idealizations whose names run past 80 characters.
async fn one_long_claim(s: &Session) {
    let long_name = |i: usize| {
        format!(
            "Assumption {i}: the beam is monochromatic, parallel and free of scatter across \
             the whole field of view, as the model states"
        )
    };
    // Written in bulk: six idealizations alike in their words are what a
    // model states, and the typed constructor would ask whether they are one.
    let nodes: Vec<Value> = (0..6)
        .map(|i| {
            json!({"node_type": "Requirement", "id": format!("req:assume-{i}"),
                   "props": {"name": long_name(i),
                             "statement": "Held on purpose by the model."}})
        })
        .collect();
    s.ok("create_nodes", json!({"nodes": nodes})).await;
    s.ok(
        "add_requirement",
        json!({"id": "req:pred-arcs", "name": "The sinogram shows arcs",
               "statement": "Each point of the sample traces a sine arc. ".repeat(160)}),
    )
    .await;
    s.ok(
        "add_inference",
        json!({"id": "inf:pred-arcs", "rule": "all", "premises": [],
               "assumed": (0..6).map(|i| format!("req:assume-{i}")).collect::<Vec<_>>(),
               "warrant": "The forward model maps each point to a sinusoid.",
               "conclusion": "req:pred-arcs"}),
    )
    .await;
}

/// ⭐ The trial's numbers: a 6,000 budget and a claim of about 7,000. The
/// reply fits, returns no claim, keeps the summary's names to at least 80
/// characters, and says where to read on and what budget the claim needs.
#[tokio::test]
async fn a_claim_bigger_than_the_budget_leaves_the_summary_and_says_what_it_needs() {
    let s = session().await;
    one_long_claim(&s).await;
    const BUDGET: usize = 6_000;
    let r = s
        .ok("reasoning_report", json!({"budget_chars": BUDGET}))
        .await;
    let n = chars(&r);
    assert!(
        n <= BUDGET,
        "budget_chars {BUDGET} answered {n} characters: {:#}",
        r["budget"]
    );
    assert_eq!(
        r["claims"].as_array().map(Vec::len),
        Some(0),
        "a claim was returned at a budget it cannot fit in: {:#}",
        r["page"]
    );
    assert_eq!(r["next_offset"], 0, "{:#}", r["page"]);
    let needs = r["page"]["claim_needs_budget_chars"]
        .as_u64()
        .unwrap_or_else(|| panic!("the page names no budget the claim needs: {:#}", r["page"]))
        as usize;
    assert!(needs > BUDGET, "{needs}");
    assert!(
        r["page"]["reads"]
            .as_str()
            .unwrap()
            .contains(&format!("budget_chars {needs}")),
        "{:#}",
        r["page"]
    );
    let mut cut = Vec::new();
    cut_outside_claims(&r, "", &mut cut);
    let blanked: Vec<&String> = cut
        .iter()
        .filter(|c| c.rsplit(": ").next().unwrap().chars().count() < 81)
        .collect();
    assert!(
        blanked.is_empty(),
        "names cut below 80 characters: {blanked:#?}"
    );
    let a = r["assumptions"].as_array().expect("assumptions listed");
    assert!(!a.is_empty(), "{r:#}");

    // At the budget it names, the claim comes back whole.
    let r = s
        .ok("reasoning_report", json!({"budget_chars": needs}))
        .await;
    assert!(chars(&r) <= needs, "{} > {needs}", chars(&r));
    assert_eq!(r["claims"][0]["id"], "req:pred-arcs", "{:#}", r["page"]);
    assert!(
        !r["claims"][0]["says"].as_str().unwrap().ends_with('…'),
        "the claim was cut"
    );
}
