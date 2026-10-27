//! THE BLAST RADIUS OF A RECORDED CHANGE FITS ITS BUDGET AND IS READ WHOLE IN
//! PAGES — through `propagate_change` and `propagate_from` alike, by one
//! bounded serializer.
//!
//! `fact:propagate-change-full-has-no-budget-or-paging-so-the-door-withholds-it-whole-2026-10-26`.
//! On a sample apartment building design (reflow2 0.92.0) `propagate_change`
//! `{full: true}` on a change touching 35 nodes answered 38,395 characters,
//! and flo2.io's door, which holds replies to about 29,000, withheld it whole:
//! `PropagateChangeReq` took no `budget_chars` and no paging, while its sibling
//! `propagate_from` had been given a budget on 2026-10-23. And the gate that
//! should have caught it (tools/replies_are_bounded.py) only called tools with
//! no required argument, so `propagate_change` was never measured.
//!
//! OBSERVED FAILING FIRST on main 02e2739c8: `propagate_change` refused
//! `budget_chars` as an unknown field, and neither tool took `offset`.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "radius-pages-probe".to_string();
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
        let shown: String = args.to_string().chars().take(200).collect();
        self.call(tool, args)
            .await
            .unwrap_or_else(|e| panic!("`{tool}` {shown} was refused: {e}"))
    }
}

const PARTS: usize = 90;

/// A core part ninety others depend on, each with a long name, and a change
/// that touched the core.
async fn a_core_with_ninety_dependents() -> Session {
    let s = session().await;
    s.ok(
        "add_project",
        json!({"id": "proj:block", "name": "An apartment block (fixture)"}),
    )
    .await;
    s.ok(
        "add_component",
        json!({"id": "cmp:core", "name": "Structural core", "description": "Walls and slab."}),
    )
    .await;
    for i in 0..PARTS {
        let id = format!("cmp:part-{i:03}");
        // create_node: the capture tools refuse near-duplicate names, and
        // these are alike on purpose (long rows, like a real radius's).
        s.ok(
            "create_node",
            json!({"node_type": "Component", "id": id,
                   "props": {"name": format!("Part {i}: {}", "a unit fit-out resting on the core ".repeat(6)),
                             "purpose": "A part."}}),
        )
        .await;
        s.ok(
            "depends_on",
            json!({"from_id": id, "from_type": "Component",
                   "to_id": "cmp:core", "to_type": "Component"}),
        )
        .await;
    }
    s.ok(
        "add_change_event",
        json!({"id": "chg:core-moves", "name": "The core moves", "change_type": "scope_change",
               "summary": "The core moves.", "rationale": "Two basement units go.",
               "affected": [{"node_id": "cmp:core"}]}),
    )
    .await;
    s
}

fn chars(v: &Value) -> usize {
    v.to_string().len()
}

fn impacted_ids(r: &Value) -> Vec<String> {
    r["impacted"]
        .as_array()
        .unwrap_or_else(|| panic!("a full radius lists impacted: {r}"))
        .iter()
        .map(|n| {
            n["node_id"]
                .as_str()
                .expect("a row keeps its id")
                .to_string()
        })
        .collect()
}

/// Read every page of `tool`'s full radius at `budget`, and return the ids.
async fn read_in_pages(s: &Session, tool: &str, args: Value, budget: usize) -> Vec<String> {
    let mut seen = Vec::new();
    let mut offset = 0usize;
    for _ in 0..200 {
        let mut a = args.clone();
        a["full"] = json!(true);
        a["budget_chars"] = json!(budget);
        a["offset"] = json!(offset);
        let r = s.ok(tool, a).await;
        assert!(
            chars(&r) <= budget,
            "{tool} at offset {offset} fits budget_chars {budget}: {} chars, {}",
            chars(&r),
            r["budget"]
        );
        let page = &r["impacted_page"];
        assert_eq!(page["offset"], offset, "{page}");
        assert_eq!(
            page["total"], PARTS,
            "the total is never budgeted away: {page}"
        );
        let ids = impacted_ids(&r);
        assert_eq!(page["shown"], ids.len(), "{page}");
        assert!(!ids.is_empty(), "a page shows at least one row: {page}");
        seen.extend(ids);
        match page["next_offset"].as_u64() {
            Some(n) => {
                assert_eq!(
                    n as usize,
                    offset + page["shown"].as_u64().unwrap() as usize
                );
                offset = n as usize;
            }
            None => return seen,
        }
    }
    panic!("{tool} never reached its last page");
}

#[tokio::test]
async fn propagate_change_full_fits_its_budget_and_pages_to_the_whole_radius() {
    let s = a_core_with_ninety_dependents().await;
    let whole = s
        .ok(
            "propagate_change",
            json!({"change_event_id": "chg:core-moves", "full": true, "budget_chars": 10_000_000}),
        )
        .await;
    let all: BTreeSet<String> = impacted_ids(&whole).into_iter().collect();
    assert_eq!(all.len(), PARTS, "{}", whole["impacted_page"]);
    assert!(
        chars(&whole) > 8_000,
        "the case is large enough to need pages: {}",
        chars(&whole)
    );

    // The default holds it to the budget a gateway holds every reply to.
    let held = s
        .ok(
            "propagate_change",
            json!({"change_event_id": "chg:core-moves", "full": true}),
        )
        .await;
    assert!(chars(&held) <= 30_000, "{}", chars(&held));

    let paged = read_in_pages(
        &s,
        "propagate_change",
        json!({"change_event_id": "chg:core-moves"}),
        6_000,
    )
    .await;
    assert_eq!(paged.len(), PARTS, "no row is read twice");
    assert_eq!(
        paged.into_iter().collect::<BTreeSet<_>>(),
        all,
        "every row is read once"
    );
}

#[tokio::test]
async fn propagate_from_pages_by_the_same_serializer() {
    let s = a_core_with_ninety_dependents().await;
    let paged = read_in_pages(
        &s,
        "propagate_from",
        json!({"seed_ids": ["cmp:core"]}),
        6_000,
    )
    .await;
    assert_eq!(paged.len(), PARTS);
    assert_eq!(paged.into_iter().collect::<BTreeSet<_>>().len(), PARTS);
}

#[tokio::test]
async fn an_offset_without_full_is_refused_by_name() {
    let s = a_core_with_ninety_dependents().await;
    let refused = s
        .call(
            "propagate_change",
            json!({"change_event_id": "chg:core-moves", "offset": 10}),
        )
        .await
        .expect_err("the summary has no rows to page");
    assert!(
        refused.contains("offset") && refused.contains("full"),
        "it names the argument and what would work: {refused}"
    );
}

/// A CHANGE TOUCHING MANY NODES FITS TOO. Measured 2026-10-26 on reflow2's own
/// design (tools/replies_are_bounded.py, a 579-seed change): with the radius
/// held to 30,000, the reply was still 80,151 characters, because every seed
/// with no prior snapshot carried its own copy of the edge-prose coverage note
/// (541 entries, 55,599 characters with their prose cut, in a map that cannot
/// give rows) and the 579 seed ids were a list nothing shortened. The silent
/// seeds are now named together under one note, and the seed lists give rows.
#[tokio::test]
async fn a_change_touching_every_part_fits_and_names_its_silent_seeds_once() {
    let s = a_core_with_ninety_dependents().await;
    let affected: Vec<Value> = (0..PARTS)
        .map(|i| json!({"node_id": format!("cmp:part-{i:03}")}))
        .collect();
    s.ok(
        "add_change_event",
        json!({"id": "chg:every-unit-refits", "name": "Every unit is refitted",
               "change_type": "scope_change", "summary": "Every unit is refitted.",
               "rationale": "A new fit-out standard.", "affected": affected}),
    )
    .await;
    let whole = s
        .ok(
            "propagate_change",
            json!({"change_event_id": "chg:every-unit-refits", "budget_chars": 10_000_000}),
        )
        .await;
    let silent = &whole["stale_edge_evidence_silent_on"];
    assert_eq!(silent["count"], PARTS, "{silent}");
    assert_eq!(silent["seeds"].as_array().map_or(0, Vec::len), PARTS);
    assert!(
        silent["coverage_note"]
            .as_str()
            .is_some_and(|n| n.contains("snapshot_before_change")),
        "{silent}"
    );
    assert!(
        whole.get("stale_edge_evidence").is_none(),
        "no per-seed copy of the note: {}",
        whole["stale_edge_evidence"]
    );
    for full in [false, true] {
        let r = s
            .ok(
                "propagate_change",
                json!({"change_event_id": "chg:every-unit-refits", "full": full,
                       "budget_chars": 4_000}),
            )
            .await;
        assert!(
            chars(&r) <= 4_000,
            "full={full}: {} chars, {}",
            chars(&r),
            r["budget"]
        );
        assert_eq!(
            r["stale_edge_evidence_silent_on"]["count"], PARTS,
            "the count stays"
        );
    }
}
