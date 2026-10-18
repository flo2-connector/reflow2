//! The whole-design `reasoning_report` reaches every claim of the reasoning,
//! refuted ones included.
//!
//! `fact:root-cause-the-reasoning-walk-never-reaches-a-claim-only-an-objection-points-to-2026-10-18`
//! (Anthony, 2026-10-18: "Yes, build the fixes. Do in one reflow2 PR").
//!
//! # The measured case
//!
//! Proof trial rung 2 (design bbcd344a71ff1cbd, 0.86.0): read with no `claim`,
//! the walk started from four roots and read 66 claims, and neither refuted
//! conjecture (`req:conj-h-unique-factorization`, `req:conj-h-euclid`) was among
//! them, although the counterexample refuting each was. Rung 1
//! (d2ea1acfee6090c8): the walk read the Euler counterexample and never
//! `req:conjecture-euler-polynomial`. The walk followed an objection only from
//! the claim it is against, and its roots were "concluded by a step, premise of
//! none" and contested Decisions, so a claim only an objection points to was
//! never read.
//!
//! # What these tests pin
//!
//! - THE CLASS, on both trial designs: the whole-design walk reads every node
//!   that has a `statement_reading` or a `proof_status`, or that is an end of
//!   an opposing CONTRADICTS;
//! - a bare conjecture refuted by a bare counterexample, neither concluded nor
//!   any step's premise, is read, and so is a conjecture nothing touches;
//! - the pages still return every claim of the walk exactly once, and every
//!   page carries the whole walk's summary lists.

use std::collections::BTreeSet;

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

/// Rung 2, the fundamental theorem of arithmetic, exported read-only from
/// flo2.io on 2026-10-17.
const RUNG_2: &str = include_str!("fixtures/proof_trial_rung_2_fta.json");
/// Rung 1, infinitely many primes, exported read-only from flo2.io on
/// 2026-10-18.
const RUNG_1: &str = include_str!("fixtures/proof_trial_rung_1_primes.json");

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "walk-reach-probe".to_string();
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
        let shown = args.to_string();
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
            "`{tool}` {shown} was refused: {:?}",
            r.content
        );
        r.structured_content.unwrap_or(Value::Null)
    }

    /// Every page of the whole-design read-back, `limit` claims at a time.
    async fn pages(&self, limit: u64) -> Vec<Value> {
        let mut pages = Vec::new();
        let mut offset = 0u64;
        loop {
            let r = self
                .ok(
                    "reasoning_report",
                    json!({"limit": limit, "offset": offset, "budget_chars": 1_000_000}),
                )
                .await;
            let next = r["next_offset"].as_u64();
            pages.push(r);
            match next {
                Some(n) => offset = n,
                None => return pages,
            }
        }
    }

    /// The ids the whole-design walk read, each once, from its pages.
    async fn walked(&self, limit: u64) -> BTreeSet<String> {
        let pages = self.pages(limit).await;
        let mut seen = BTreeSet::new();
        for p in &pages {
            assert_eq!(p["capped"], false, "the walk is whole: {p:#}");
            for c in p["claims"].as_array().unwrap() {
                let id = c["id"].as_str().unwrap().to_string();
                assert!(seen.insert(id.clone()), "{id} came back twice");
            }
        }
        let total = pages[0]["total_claims"].as_u64().unwrap();
        assert_eq!(seen.len() as u64, total, "the pages return the whole walk");
        for key in [
            "roots",
            "rests_on",
            "uncited_premises",
            "stated_foundations",
            "unsupported",
            "still_contested",
        ] {
            for p in &pages {
                assert_eq!(
                    p[key], pages[0][key],
                    "`{key}` is the whole walk's on every page"
                );
            }
        }
        seen
    }
}

async fn imported(fixture: &str) -> (Session, Value) {
    let doc: Value = serde_json::from_str(fixture).expect("the fixture parses");
    let s = session().await;
    s.ok("import_graph", json!({"document": doc})).await;
    (s, doc)
}

/// THE INVARIANT, over a whole design: every node with a statement reading or
/// a proof status, and every end of an opposing CONTRADICTS, is walked.
async fn the_walk_reaches_every_claim(s: &Session, doc: &Value, limit: u64) -> BTreeSet<String> {
    let walked = s.walked(limit).await;
    let mut owed = BTreeSet::new();
    for n in doc["nodes"].as_array().unwrap() {
        let id = n["node_id"].as_str().unwrap();
        let node = s.ok("get_node", json!({"id": id})).await["node"].clone();
        if node.get("statement_reading").is_some() || node.get("proof_status").is_some() {
            owed.insert(id.to_string());
        }
    }
    for e in doc["edges"].as_array().unwrap() {
        if e["edge_type"] == "CONTRADICTS" && e["properties"]["alignment"] != "supporting" {
            owed.insert(e["from_id"].as_str().unwrap().to_string());
            owed.insert(e["to_id"].as_str().unwrap().to_string());
        }
    }
    let missed: Vec<&String> = owed.difference(&walked).collect();
    assert!(
        missed.is_empty(),
        "the whole-design walk never read {} of the {} claims of the reasoning: {missed:?}",
        missed.len(),
        owed.len()
    );
    walked
}

#[tokio::test]
async fn the_walk_of_rung_2_reaches_every_claim_and_both_refuted_conjectures() {
    let (s, doc) = imported(RUNG_2).await;
    let walked = the_walk_reaches_every_claim(&s, &doc, 500).await;
    for conj in ["req:conj-h-unique-factorization", "req:conj-h-euclid"] {
        assert!(walked.contains(conj), "{conj} is walked");
    }
    // The same walk, in small pages: every claim once, the summaries whole.
    assert_eq!(s.walked(25).await, walked);
}

#[tokio::test]
async fn the_walk_of_rung_1_reaches_every_claim_and_the_euler_conjecture() {
    let (s, doc) = imported(RUNG_1).await;
    let walked = the_walk_reaches_every_claim(&s, &doc, 500).await;
    assert!(walked.contains("req:conjecture-euler-polynomial"));
    assert_eq!(s.walked(7).await, walked);
}

#[tokio::test]
async fn a_bare_conjecture_refuted_by_a_bare_counterexample_is_walked() {
    let s = session().await;
    s.ok(
        "add_requirement",
        json!({
            "id": "req:conjecture",
            "name": "Every odd number is prime",
            "statement": "Every odd number greater than 1 is prime.",
            "statement_kind": "conjecture"
        }),
    )
    .await;
    s.ok(
        "add_requirement",
        json!({
            "id": "req:open",
            "name": "An open conjecture nothing touches",
            "statement": "Every even number greater than 2 is the sum of two primes.",
            "statement_kind": "conjecture"
        }),
    )
    .await;
    s.ok(
        "add_verification",
        json!({"id": "ver:nine", "name": "9 = 3 × 3", "description": "Computed."}),
    )
    .await;
    s.ok(
        "set_verification_status",
        json!({"verification_id": "ver:nine", "status": "passing", "last_run_at": "2026-10-18"}),
    )
    .await;
    s.ok(
        "create_edge",
        json!({"edge_type": "CONTRADICTS", "from_id": "ver:nine", "to_id": "req:conjecture"}),
    )
    .await;

    let r = s.ok("reasoning_report", json!({})).await;
    let read: BTreeSet<&str> = r["claims"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        read,
        BTreeSet::from(["req:conjecture", "req:open", "ver:nine"]),
        "{r:#}"
    );
    let roots: Vec<&str> = r["roots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(
        roots,
        ["req:conjecture", "req:open"],
        "the walk starts from the refuted claim, not from what refutes it: {r:#}"
    );
    let conj = r["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "req:conjecture")
        .unwrap();
    assert_eq!(conj["statement"]["reads"], "refuted", "{conj:#}");
    assert!(r["note"].is_null(), "there is reasoning to read: {r:#}");
}
