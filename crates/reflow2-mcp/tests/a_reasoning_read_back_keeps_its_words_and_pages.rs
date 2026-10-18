//! The reasoning read-back keeps the words of every claim it returns, and
//! pages instead of blanking the proof.
//!
//! `req:the-reasoning-read-back-keeps-the-words-of-the-claims-it-returns`
//! (Anthony, 2026-10-17: "do 1, 2 and 6 first", fix 2).
//!
//! # The measured case
//!
//! Proof trial rung 2 on flo2.io (design bbcd344a71ff1cbd, 0.86.0;
//! `fact:the-proof-trial-rung-2-ran-on-flo2-io-and-what-counts-holds-while-recording-a-proof-is-still-costly-2026-10-17`,
//! friction items 2 and 7). Its read-back cut every prose field it returned to
//! "…" (152 of them, re-measured the same day) and still returned only 25 of
//! the 60 claims it walked; the statements and warrants, which are the proof,
//! were readable only through scan_nodes. And with a small `limit`, `rests_on`
//! and `stated_foundations` came back `[]`, which reads as "rests on nothing".
//!
//! # What these tests pin
//!
//! - every claim a reply returns is WHOLE: its own words (`says`), each step's
//!   rule and warrant, each premise's name; no text is cut;
//! - the reply pages in proof order from where it starts, with `next_offset`,
//!   and the pages together return every claim of the walk exactly once;
//! - the budget the reply was held to is named in the reply;
//! - a small page never reports what the reasoning rests on as nothing.

use std::collections::{BTreeMap, BTreeSet};

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

/// The trial design, exported read-only from flo2.io on 2026-10-17.
const TRIAL: &str = include_str!("fixtures/proof_trial_rung_2_fta.json");

async fn trial() -> (ReflowService, BTreeMap<String, Value>) {
    let doc: Value = serde_json::from_str(TRIAL).expect("the fixture parses");
    let s = ReflowService::in_memory().expect("in-memory service");
    s.import_graph(Parameters(ImportGraphReq {
        document: doc.as_object().cloned(),
        path: None,
        accept_newer: None,
    }))
    .await
    .expect("the trial imports");
    let props = doc["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| {
            (
                n["node_id"].as_str().unwrap().to_string(),
                n["properties"].clone(),
            )
        })
        .collect();
    (s, props)
}

async fn read(s: &ReflowService, args: Value) -> Value {
    let req: ReasoningReportReq = serde_json::from_value(args).expect("a request");
    s.reasoning_report(Parameters(req))
        .await
        .expect("the read-back answers")
        .structured_content
        .expect("structured")
}

/// Every string in `v` that the budget cut (it ends in the elision mark).
fn cut_strings(v: &Value, at: &str, out: &mut Vec<String>) {
    match v {
        Value::String(s) if s.ends_with('…') => out.push(format!("{at}: {s}")),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .for_each(|(i, x)| cut_strings(x, &format!("{at}[{i}]"), out)),
        Value::Object(o) => o
            .iter()
            .for_each(|(k, x)| cut_strings(x, &format!("{at}.{k}"), out)),
        _ => {}
    }
}

/// The node's own words, as the read-back should carry them.
fn own_words(p: &Value) -> Option<&str> {
    ["statement", "decision", "description", "summary"]
        .iter()
        .find_map(|k| p[*k].as_str())
}

/// ⭐ At the default budget, every claim that comes back is whole.
#[tokio::test]
async fn every_claim_returned_at_the_default_budget_is_whole() {
    let (s, props) = trial().await;
    let r = read(&s, json!({})).await;
    let claims = r["claims"].as_array().expect("claims");
    assert!(!claims.is_empty(), "a page holds at least one claim: {r}");

    let mut cut = Vec::new();
    cut_strings(&r["claims"], "claims", &mut cut);
    assert!(
        cut.is_empty(),
        "no returned claim may have its words cut; {} were:\n{}",
        cut.len(),
        cut.join("\n")
    );

    for c in claims {
        let id = c["id"].as_str().unwrap();
        let stored = &props[id];
        if let Some(words) = own_words(stored) {
            assert_eq!(
                c["says"].as_str(),
                Some(words),
                "{id} must carry its own words"
            );
        }
        for step in c["support"].as_array().into_iter().flatten() {
            let inf = step["inference"].as_str().unwrap();
            assert!(step["rule"].is_string(), "{inf} carries its rule");
            if let Some(w) = props[inf]["warrant"].as_str() {
                assert_eq!(
                    step["warrant"].as_str(),
                    Some(w),
                    "{inf} carries its warrant"
                );
            }
            for p in step["premises"].as_array().unwrap() {
                let pid = p["id"].as_str().unwrap();
                assert_eq!(
                    p["name"], props[pid]["name"],
                    "premise {pid} of {inf} carries its name"
                );
            }
        }
    }
    eprintln!(
        "proof trial at the default budget: {} of {} claims whole on the first page, next_offset {}",
        claims.len(),
        r["total_claims"],
        r["next_offset"]
    );
}

/// ⭐ The pages, read on with `next_offset`, return every claim of the walk
/// exactly once, in the order one unbounded read gives them.
#[tokio::test]
async fn the_read_back_pages_in_proof_order_and_reaches_every_claim() {
    let (s, _) = trial().await;
    let whole = read(&s, json!({ "limit": 500, "budget_chars": 10_000_000 })).await;
    let order: Vec<String> = whole["claims"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        whole["total_claims"].as_u64(),
        Some(order.len() as u64),
        "an unbounded read returns the whole walk: {}",
        whole["page"]
    );
    assert!(whole["next_offset"].is_null(), "nothing left to read");

    let mut paged: Vec<String> = Vec::new();
    let mut offset = 0u64;
    let mut pages = 0;
    loop {
        let r = read(&s, json!({ "offset": offset })).await;
        pages += 1;
        let ids: Vec<String> = r["claims"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_str().unwrap().to_string())
            .collect();
        assert!(!ids.is_empty(), "a page is never empty while claims remain");
        assert_eq!(
            r["remaining"].as_u64(),
            Some(order.len() as u64 - offset - ids.len() as u64),
            "the reply says how many claims are left: {}",
            r["page"]
        );
        paged.extend(ids);
        match r["next_offset"].as_u64() {
            Some(n) => {
                assert_eq!(
                    n,
                    paged.len() as u64,
                    "next_offset is where this page stopped"
                );
                offset = n;
            }
            None => break,
        }
        assert!(pages < 100, "paging terminates");
    }
    assert_eq!(
        paged, order,
        "the pages read the walk in order, each claim once"
    );
    let unique: BTreeSet<&String> = paged.iter().collect();
    assert_eq!(unique.len(), paged.len(), "no claim twice");
    eprintln!(
        "proof trial: {} claims in {pages} page(s) at the default budget",
        paged.len()
    );
}

/// The budget the reply was held to is named in the reply, so a budget
/// lowered on the way is visible in the result itself. 27,267 is what flo2.io
/// lowered the trial's 120,000 to on 2026-10-17 (its connector's replies are
/// held to 29,267, less 2,000 for its own wrapper).
#[tokio::test]
async fn the_budget_a_reply_was_held_to_is_named() {
    let (s, _) = trial().await;
    let r = read(&s, json!({ "budget_chars": 27_267 })).await;
    assert_eq!(r["page"]["budget_chars"], 27_267, "{}", r["page"]);
    let reads = r["page"]["reads"].as_str().unwrap_or_default();
    assert!(
        reads.contains("budget_chars 27267"),
        "the page says, in words, what it was held to: {reads}"
    );
    let mut cut = Vec::new();
    cut_strings(&r["claims"], "claims", &mut cut);
    assert!(
        cut.is_empty(),
        "held lower, the claims are still whole: {cut:?}"
    );
    let size = r.to_string().len();
    assert!(size <= 27_267, "and it fits: {size}");
    eprintln!(
        "proof trial at flo2.io's 27,267: {} of {} claims whole, next_offset {}",
        r["claims"].as_array().unwrap().len(),
        r["total_claims"],
        r["next_offset"]
    );
}

/// A small page never reports what the reasoning rests on as nothing: the
/// summary lists cover the whole walk, whatever one page returns.
#[tokio::test]
async fn a_small_page_never_reads_as_resting_on_nothing() {
    let (s, _) = trial().await;
    let r = read(
        &s,
        json!({ "claim": "req:thm-fundamental-theorem-of-arithmetic", "limit": 1 }),
    )
    .await;
    assert_eq!(
        r["claims"].as_array().unwrap().len(),
        1,
        "one claim: {}",
        r["page"]
    );
    for (list, must_hold) in [
        ("rests_on", "con:ax-integer-arithmetic"),
        ("stated_foundations", "con:def-prime"),
    ] {
        let ids: Vec<&str> = r[list]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert!(
            ids.contains(&must_hold),
            "`{list}` must cover the whole walk, not the one claim this page returned: {ids:?}"
        );
    }
    assert_eq!(r["next_offset"], 1, "and it says where to read on");
}
