//! A hub-wide read answers within its time budget, and names the designs it
//! did not reach.
//!
//! The class (reflow2's
//! `req:a-hub-wide-read-answers-within-its-time-budget-and-names-the-designs-it-did-not-reach`,
//! answering
//! `fact:root-cause-a-hub-wide-read-after-a-quiet-spell-reopens-and-reindexes-every-member-one-at-a-time-2026-11-22`):
//! a read through a hub's address runs on the hub and then on each member in
//! turn, and a member closed for idleness is reopened inside that turn. On
//! flo2.io a cold search over 29 designs took a median of 13.5 s, four took
//! past the gateway's 30 s door, and each of those lost the WHOLE answer
//! although most designs had been reached.
//!
//! These drive the real binary behind a trusted gateway, H viewing [A, B],
//! with B made slow to open on purpose (`REFLOW2_TEST_OPEN_DELAY`, a seam only
//! a debug build has) and the hub's budget lowered by the operator's own
//! setting (`REFLOW2_HUB_VIEW_BUDGET_MS`). Each asserts, through H:
//!
//! 1. `search_design` answers within the budget, with what H and A returned;
//!    B is named in `not_reached` with how to reach it, `designs_searched`
//!    and the counts (`searched`, `found_in`) cover only what was searched,
//!    `time_budget` says the answer is partial, and the text says so.
//!    Then, once B's open has had its time, the same search reaches B: the
//!    open the budget stopped waiting for finished on its own and left B warm.
//! 2. `loop_status` (a by-design read) answers within the budget too, with
//!    H's and A's replies under `by_design` and B named in `not_reached`.
//!
//! FAILED FIRST on main at 9415f9239 with the open-delay seam alone in place:
//! each call waited out B's open and answered whole, past its budget, naming
//! nothing not reached ("search_design through H took 31.65s: past its
//! budget of 6000 ms"; loop_status the same, 31.67 s).

use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::a_hubs_address_serves_a_view_over_its_members::{
    GATEWAY, Session, hub_and_members, ok, serve_with,
};

/// The hub's budget in these tests, in milliseconds.
const BUDGET_MS: u64 = 6_000;
/// How long B takes to open: far past the budget.
const B_OPENS_IN_MS: u64 = 30_000;
/// What the budget may be overrun by: building and sending the reply.
const SLACK: Duration = Duration::from_secs(6);

fn server_env(b: &str) -> Vec<(String, String)> {
    vec![
        (
            "REFLOW2_HUB_VIEW_BUDGET_MS".to_string(),
            BUDGET_MS.to_string(),
        ),
        (
            "REFLOW2_TEST_OPEN_DELAY".to_string(),
            format!("{b}={B_OPENS_IN_MS}"),
        ),
    ]
}

fn names_b_not_reached(got: &Value, b: &str, what: &str) {
    let not_reached = got["not_reached"]
        .as_array()
        .unwrap_or_else(|| panic!("{what}: names the designs it did not reach: {got}"));
    assert_eq!(
        not_reached.len(),
        1,
        "{what}: B alone was not reached: {got}"
    );
    let entry = &not_reached[0];
    assert_eq!(entry["design"], json!(b), "{what}: names B: {got}");
    assert_eq!(entry["why"], json!("budget"), "{what}: says why: {got}");
    assert!(
        entry["call"]
            .as_str()
            .is_some_and(|c| c.contains("member") && c.contains(b)),
        "{what}: says how to reach B (`member` naming it): {got}"
    );
    let tb = &got["time_budget"];
    assert_eq!(tb["complete"], json!(false), "{what}: partial: {got}");
    assert_eq!(tb["budget_ms"], json!(BUDGET_MS), "{what}: {got}");
    assert_eq!(tb["set_by"], json!("server"), "{what}: {got}");
    assert!(
        tb["note"]
            .as_str()
            .is_some_and(|n| n.contains("not reached in time")),
        "{what}: the budget's note says what was not reached in time: {got}"
    );
}

fn within_budget(took: Duration, what: &str) {
    assert!(
        took < Duration::from_millis(BUDGET_MS) + SLACK,
        "{what} took {took:?}: past its budget of {BUDGET_MS} ms (B opens in \
         {B_OPENS_IN_MS} ms, so a fan-out that waits for B answers after that)"
    );
}

#[test]
fn a_hub_search_past_its_budget_answers_with_the_designs_it_reached_and_names_the_rest() {
    if !cfg!(debug_assertions) {
        eprintln!("skipped: the open-delay seam exists in a debug build only");
        return;
    }
    let (root, h, a, b) = hub_and_members("view-time-budget-search");
    let env = server_env(&b);
    let env: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let server = serve_with(&root, Some(GATEWAY), &env);
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];

    // A word only A holds, and one only B holds.
    let asked = Instant::now();
    let reply = on_h.call(
        "search_design",
        json!({"query": "aardvark zanzibar"}),
        Some(&readable),
        Some(&view),
    );
    let took = asked.elapsed();
    let got = ok(&reply, "search_design through H, B slow to open");
    within_budget(took, "search_design through H");

    assert_eq!(
        got["found_in"],
        json!({ a.as_str(): 1 }),
        "the hits are the reached designs' own: {got}"
    );
    assert_eq!(
        got["designs_searched"],
        json!([h.as_str(), a.as_str()]),
        "`designs_searched` names what was searched, and only that: {got}"
    );
    assert!(
        !got.to_string().contains("only-in-beta"),
        "nothing of B's is in the reply: {got}"
    );
    names_b_not_reached(&got, &b, "search_design");
    assert!(
        reply.text().contains("not reached in time"),
        "the text block says the answer is partial: {}",
        reply.text()
    );

    // B's open was not abandoned: it finishes on its own, and the next search
    // through H reaches B in time.
    let opened_by = asked + Duration::from_millis(B_OPENS_IN_MS) + Duration::from_secs(3);
    if let Some(rest) = opened_by.checked_duration_since(Instant::now()) {
        std::thread::sleep(rest);
    }
    let asked = Instant::now();
    let reply = on_h.call(
        "search_design",
        json!({"query": "zanzibar"}),
        Some(&readable),
        Some(&view),
    );
    let took = asked.elapsed();
    let got = ok(&reply, "search_design through H once B is open");
    within_budget(took, "the second search_design through H");
    assert_eq!(
        got["found_in"],
        json!({ b.as_str(): 1 }),
        "B, warm now, is searched: {got}"
    );
    assert!(
        got.get("not_reached").is_none() && got.get("time_budget").is_none(),
        "a whole answer carries no partial-answer fields: {got}"
    );
}

#[test]
fn a_by_design_read_past_its_budget_answers_with_the_designs_it_reached_and_names_the_rest() {
    if !cfg!(debug_assertions) {
        eprintln!("skipped: the open-delay seam exists in a debug build only");
        return;
    }
    let (root, h, a, b) = hub_and_members("view-time-budget-loop");
    let env = server_env(&b);
    let env: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let server = serve_with(&root, Some(GATEWAY), &env);
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];

    let asked = Instant::now();
    let reply = on_h.call("loop_status", json!({}), Some(&readable), Some(&view));
    let took = asked.elapsed();
    let got = ok(&reply, "loop_status through H, B slow to open");
    within_budget(took, "loop_status through H");

    let by = got["by_design"]
        .as_object()
        .unwrap_or_else(|| panic!("a fan-out answers by_design: {got}"));
    for id in [&h, &a] {
        assert!(
            by.get(id.as_str())
                .is_some_and(|r| r.get("error").is_none()),
            "{id} was reached and answered: {got}"
        );
    }
    assert!(
        !by.contains_key(b.as_str()),
        "B, not reached, has no reply under by_design: {got}"
    );
    names_b_not_reached(&got, &b, "loop_status");
    assert!(
        reply.text().contains("not reached in time"),
        "the text block says the answer is partial: {}",
        reply.text()
    );
}
