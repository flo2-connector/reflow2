//! A budget says how its contributions compose, and its verdict follows what
//! it says; one read reports every budget
//! (req:a-budget-says-whether-its-parts-add-up-or-run-along-a-path).
//!
//! Found by the dev_reflow2 two-agent exercise (art:dev-reflow2-two-agent-exercise-feedback-2026-09-29):
//! - I18: a 50 ms write budget whose truth is 33 ms along its dependency path
//!   read "46 ms, exceeded" against a 40 ms limit, because the verdict is
//!   hard-wired to the plain sum while the path total is computed beside it
//!   (fact:root-cause-a-latency-budget-with-parallel-branches-is-judged-on-the-sum-while-its-worst-path-is-computed-beside-it-2026-09-29).
//! - I17: checking 14 budgets took 14 calls, because budget_report reads one
//!   Constraint and nothing reads them all with their verdicts
//!   (fact:root-cause-budget-report-reads-one-budget-and-its-refusal-never-names-the-all-budgets-sweep-2026-09-29).
//!
//! Driven over the MCP surface, the way a caller meets it, so every test here
//! can be observed failing on a build that predates the fix.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "budget-composition-probe".to_string();
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
    async fn call(&self, tool: &str, args: Value) -> Result<Value, String> {
        let arguments = args.as_object().cloned().unwrap_or_default();
        match self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
        {
            Ok(r) if r.is_error.unwrap_or(false) => Err(r
                .content
                .iter()
                .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                .collect::<Vec<_>>()
                .join("\n")),
            Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
            Err(rmcp::service::ServiceError::McpError(e)) => Err(e.message.to_string()),
            Err(e) => Err(format!("transport failure: {e}")),
        }
    }

    async fn ok(&self, tool: &str, args: Value) -> Value {
        let shown = args.to_string();
        self.call(tool, args)
            .await
            .unwrap_or_else(|e| panic!("`{tool}` {shown} was refused: {e}"))
    }

    async fn description(&self, tool: &str) -> String {
        self.client
            .list_all_tools()
            .await
            .expect("tools/list")
            .into_iter()
            .find(|t| t.name == tool)
            .and_then(|t| t.description.map(|d| d.to_string()))
            .unwrap_or_default()
    }

    /// The owner agent's write path, as measured on 0.74.0: gateway 5 →
    /// router 2 → host 6 → store 20, with the rule delta (13) running beside
    /// the commit. 33 ms along the heaviest path, 46 ms as a sum.
    async fn the_write_path(&self, composition: Option<&str>) {
        let mut args = json!({
            "id": "con:write-p99",
            "name": "Durable write p99",
            "statement": "A confirmed write stays at or under 40 ms at p99.",
            "category": "budget",
            "quantity": "latency_ms",
            "unit": "ms",
            "limit": 40.0,
            "direction": "maximum",
        });
        if let Some(c) = composition {
            args["composition"] = json!(c);
        }
        self.ok("add_constraint", args).await;
        for (id, name, ms) in [
            ("cmp:gw", "Gateway", 5.0),
            ("cmp:router", "Router", 2.0),
            ("cmp:host", "Host", 6.0),
            ("cmp:store", "Store commit", 20.0),
            ("cmp:rules", "Rule delta", 13.0),
        ] {
            self.ok(
                "add_component",
                json!({"id": id, "name": name, "description": "A part the write passes."}),
            )
            .await;
            self.ok(
                "constrains",
                json!({"constraint_id": "con:write-p99", "target_id": id,
                       "contribution": ms, "unit": "ms", "basis": "estimated", "source": "who:owner"}),
            )
            .await;
        }
        for (a, b) in [
            ("cmp:gw", "cmp:router"),
            ("cmp:router", "cmp:host"),
            ("cmp:host", "cmp:store"),
            ("cmp:host", "cmp:rules"),
        ] {
            self.ok("depends_on", json!({"from_id": a, "to_id": b}))
                .await;
        }
    }
}

// ─── The owner's measured case, pinned ──────────────────────────────────────

#[tokio::test]
async fn a_path_budget_is_judged_on_its_path_not_its_sum() {
    let s = session().await;
    s.the_write_path(Some("path")).await;
    let r = s
        .ok("budget_report", json!({"constraint_id": "con:write-p99"}))
        .await;
    assert_eq!(r["total"], json!(46.0), "the sum is still reported: {r:#}");
    assert_eq!(r["worst_path_total"], json!(33.0), "{r:#}");
    assert_eq!(
        r["judged_on"],
        json!("path"),
        "a budget declared `composition: path` is judged on its path: {r:#}"
    );
    assert_eq!(r["judged_total"], json!(33.0), "{r:#}");
    assert_eq!(
        r["verdict"],
        json!("within"),
        "33 ms along the heaviest path is within 40 ms; reading it as the 46 ms sum said \
         `exceeded` about a budget that fits (I18): {r:#}"
    );
}

#[tokio::test]
async fn an_undeclared_budget_keeps_the_sum_and_says_what_it_did_not_read() {
    let s = session().await;
    s.the_write_path(None).await;
    let r = s
        .ok("budget_report", json!({"constraint_id": "con:write-p99"}))
        .await;
    assert_eq!(
        r["verdict"],
        json!("exceeded"),
        "an undeclared budget keeps today's verdict on the sum — inferring the composition \
         would silently change verdicts on designs that never asked: {r:#}"
    );
    assert_eq!(r["judged_on"], json!("sum"), "{r:#}");
    assert!(
        r["composition"].is_null(),
        "absent means nobody said: {r:#}"
    );
    let note = r["composition_note"].as_str().unwrap_or_default();
    assert!(
        note.contains("33") && note.contains("composition"),
        "the path total the verdict did NOT read is named beside it, with the declaration \
         that would make the verdict read it — silence here is how the owner's 33 ms read as \
         46: {r:#}"
    );
}

#[tokio::test]
async fn a_declared_path_with_no_path_drawn_reaches_no_numeric_verdict() {
    let s = session().await;
    s.ok(
        "add_constraint",
        json!({"id": "con:lat", "name": "Latency", "statement": "Under 40 ms.",
               "category": "budget", "quantity": "latency_ms", "unit": "ms",
               "limit": 40.0, "composition": "path"}),
    )
    .await;
    for (id, ms) in [("cmp:a", 30.0), ("cmp:b", 30.0)] {
        s.ok(
            "add_component",
            json!({"id": id, "name": id, "description": "A part."}),
        )
        .await;
        s.ok(
            "constrains",
            json!({"constraint_id": "con:lat", "target_id": id, "contribution": ms,
                   "unit": "ms", "source": "who:owner"}),
        )
        .await;
    }
    let r = s
        .ok("budget_report", json!({"constraint_id": "con:lat"}))
        .await;
    assert_eq!(
        r["verdict"],
        json!("incomplete"),
        "a path budget whose contributors are joined by no dependency cannot be judged on a \
         path: reading the parts as parallel would quietly take the max (30) and under-count \
         a maximum, the dangerous direction: {r:#}"
    );
    let note = r["composition_note"].as_str().unwrap_or_default();
    assert!(
        note.contains("depends_on") || note.contains("DEPENDS_ON"),
        "the note names what would make the path computable: {r:#}"
    );
}

#[tokio::test]
async fn a_composition_outside_the_declared_set_is_refused() {
    let s = session().await;
    let refusal = s
        .call(
            "add_constraint",
            json!({"id": "con:x", "name": "X", "statement": "A budget.", "category": "budget",
                   "quantity": "latency_ms", "limit": 1.0, "composition": "average"}),
        )
        .await
        .expect_err("`average` is not a composition");
    assert!(
        refusal.contains("sum") && refusal.contains("path"),
        "the refusal names the compositions that exist: {refusal}"
    );
}

// ─── I17: one read reports every budget ─────────────────────────────────────

#[tokio::test]
async fn budget_report_with_no_constraint_reads_every_budget() {
    let s = session().await;
    s.the_write_path(Some("path")).await;
    s.ok(
        "add_constraint",
        json!({"id": "con:mass", "name": "Mass", "statement": "At most 10 kg.",
               "category": "budget", "quantity": "mass_kg", "unit": "kg", "limit": 10.0}),
    )
    .await;
    s.ok(
        "add_constraint",
        json!({"id": "con:no-pii", "name": "No PII leaves the device",
               "statement": "No personal data leaves the device."}),
    )
    .await;
    let r = s
        .call("budget_report", json!({}))
        .await
        .unwrap_or_else(|e| {
            panic!("with no constraint_id, budget_report reports every budget (I17); refused: {e}")
        });
    let budgets = r["budgets"].as_array().cloned().unwrap_or_default();
    let ids: Vec<&str> = budgets
        .iter()
        .filter_map(|b| b["constraint_id"].as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["con:mass", "con:write-p99"],
        "every numeric budget, sorted — and not the prohibition, which has no quantity: {r:#}"
    );
    let write = budgets
        .iter()
        .find(|b| b["constraint_id"] == "con:write-p99")
        .expect("the write budget");
    assert_eq!(write["verdict"], json!("within"), "{r:#}");
    assert_eq!(write["judged_on"], json!("path"), "{r:#}");
    assert_eq!(
        r["not_budgets"],
        json!(1),
        "the prohibition is counted, not hidden: {r:#}"
    );
    assert!(
        r["by_verdict"]["within"].as_u64().is_some(),
        "the sweep says how many budgets reached each verdict: {r:#}"
    );
}

#[tokio::test]
async fn the_every_budget_read_is_bounded() {
    let s = session().await;
    for i in 0..40 {
        s.ok(
            "add_constraint",
            json!({"id": format!("con:b{i:02}"), "name": format!("Budget {i} {}", "x".repeat(300)),
                   "statement": "A budget with a long name.", "category": "budget",
                   "quantity": "cost_usd", "unit": "USD", "limit": 100.0}),
        )
        .await;
    }
    let r = s.ok("budget_report", json!({"budget_chars": 4000})).await;
    assert_eq!(
        r["budget"]["applied"],
        json!(true),
        "an every-budget reply that outgrows its budget is bounded and says so: {r:#}"
    );
    assert_eq!(
        r["swept"],
        json!(40),
        "counts are never budgeted away: {r:#}"
    );
}

// ─── The instruction reaches the moment ─────────────────────────────────────

#[tokio::test]
async fn constrains_says_what_makes_the_path_total_computable() {
    let s = session().await;
    let d = s.description("constrains").await;
    assert!(
        d.contains("DEPENDS_ON") && d.contains("composition"),
        "the designer spent 50 calls on contributions and never learned that drawing \
         dependencies among them is what yields a path total, or that `composition: path` \
         makes the verdict read it: {d}"
    );
}
