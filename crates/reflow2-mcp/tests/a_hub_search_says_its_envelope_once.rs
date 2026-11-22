//! A search through a hub's address says its envelope once, and takes a budget.
//!
//! ⭐ THE ROOT CAUSE: reflow2:fact:root-cause-a-hub-wide-search-spends-77-percent-
//! on-per-design-lists-and-takes-no-budget-2026-11-19. Through a hub of 29
//! designs a search at limit 2 answered 5,746 characters, 1,314 of them hits:
//! `view` repeats `designs_searched`, `found_by_design` and
//! `searched_by_design` list every design whatever the answer, and
//! search_design took no `budget_chars`, so nothing held the merged reply.
//!
//! The decision (Anthony, 2026-11-20, "Announce, remove next";
//! dec:a-hub-search-reply-announces-its-per-design-lists-and-the-next-release-removes-them):
//! this release the merged reply carries `searched` (one total) and
//! `found_in` (only the designs with hits), announces that `view`,
//! `searched_by_design` and `found_by_design` are going, and holds the whole
//! reply to `budget_chars`. The next release removes the three, and it has: since
//! 2026-11-21 (after 0.102.0 shipped) the merged reply carries none of them and
//! no notice, and this file checks they stay gone.

use serde_json::{Value, json};

use super::a_hubs_address_serves_a_view_over_its_members::{
    GATEWAY, Session, hub_and_members, mint_holding, ok, serve, tmp_root,
};

fn notice_paths(got: &Value) -> Vec<String> {
    got["deprecation_notice"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|d| d["item"]["path"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_hub_search_carries_one_total_and_only_the_designs_with_hits_and_no_per_design_lists() {
    let (root, h, a, b) = hub_and_members("envelope-once");
    let server = serve(&root, Some(GATEWAY));
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];

    let got = ok(
        &on_h.call(
            "search_design",
            json!({"query": "zanzibar"}),
            Some(&readable),
            Some(&view),
        ),
        "search_design through H",
    );

    // Only B holds the word: `found_in` names B alone.
    assert_eq!(
        got["found_in"],
        json!({ b.as_str(): 1 }),
        "only the designs with hits, with their counts: {got}"
    );
    // One total, the key a single design's search answers with.
    // Each design's own search, asked through the hub with `member`, says how many
    // nodes it ran over; the merged `searched` is their sum.
    let total: u64 = [&h, &a, &b]
        .iter()
        .map(|id| {
            let one = ok(
                &on_h.call(
                    "search_design",
                    json!({"query": "zanzibar", "member": id.as_str()}),
                    Some(&readable),
                    Some(&view),
                ),
                "search_design on one member",
            );
            one["searched"].as_u64().unwrap()
        })
        .sum();
    assert_eq!(got["searched"], json!(total), "{got}");

    // REMOVED in the release after 0.102.0 announced them: the three lists are gone,
    // and nothing is announced as going any more.
    for field in ["view", "searched_by_design", "found_by_design"] {
        assert!(
            got.get(field).is_none(),
            "{field} was removed after 0.102.0 announced it: {got}"
        );
    }
    assert!(
        notice_paths(&got).is_empty(),
        "no deprecation notice is left on a hub search: {got}"
    );
}

#[test]
fn a_hub_search_is_held_to_the_budget_it_is_asked_for() {
    let (root, h, a, b) = hub_and_members("envelope-budget");
    let server = serve(&root, Some(GATEWAY));
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];

    let full = ok(
        &on_h.call(
            "search_design",
            json!({"query": "zanzibar aardvark harbour"}),
            Some(&readable),
            Some(&view),
        ),
        "search_design through H, no budget",
    );
    let full_chars = serde_json::to_string(&full).unwrap().len();
    let budget = full_chars * 3 / 4;

    let held = ok(
        &on_h.call(
            "search_design",
            json!({"query": "zanzibar aardvark harbour", "budget_chars": budget}),
            Some(&readable),
            Some(&view),
        ),
        "search_design through H at a budget",
    );
    let chars = serde_json::to_string(&held).unwrap().len();
    assert!(
        chars <= budget,
        "a reply of {full_chars} asked at {budget} answered {chars}: {held}"
    );
    assert!(
        held.get("budget").is_some(),
        "and says what it withheld: {held}"
    );
    assert_eq!(
        held["searched"], full["searched"],
        "counts survive the cut: {held}"
    );
}

#[test]
fn a_single_designs_search_takes_a_budget_and_withholds_whole_hits() {
    let root = tmp_root("search-budget-one");
    let d = root.join("one");
    std::fs::create_dir_all(&d).unwrap();
    // Ten requirements that each name a wombat and otherwise say different
    // things, so none reads as a near-duplicate of another.
    let said = [
        "The wombat gate opens at dawn for the keeper",
        "A wombat scale weighs each animal every Monday",
        "Night cameras record the wombat enclosure in infrared",
        "The wombat pond is drained and refilled each spring",
        "Visitors hear a recorded talk beside the wombat window",
        "The vet checks wombat teeth twice a year",
        "Fresh grass is cut for the wombat feeders at noon",
        "Heating cables keep the wombat den above ten degrees",
        "A wombat keeper logs every burrow collapse",
        "School groups book the wombat tour a week ahead",
    ];
    let reqs: Vec<(String, String)> = said
        .iter()
        .enumerate()
        .map(|(i, s)| (format!("req:wombat-{i}"), (*s).to_string()))
        .collect();
    let id = mint_holding(&d, &reqs);
    let server = serve(&root, None);
    let mut on_one = Session::open(server.port, &id);

    let full = ok(
        &on_one.call("search_design", json!({"query": "wombat"}), None, None),
        "search_design on one design",
    );
    assert_eq!(full["hits"].as_array().map(Vec::len), Some(10), "{full}");
    assert!(
        full.get("found_in").is_none() && full.get("deprecation_notice").is_none(),
        "a single design's search answers as it always has: {full}"
    );
    let full_chars = serde_json::to_string(&full).unwrap().len();
    let budget = full_chars / 2;
    let held = ok(
        &on_one.call(
            "search_design",
            json!({"query": "wombat", "budget_chars": budget}),
            None,
            None,
        ),
        "search_design on one design at half its size",
    );
    let shown = held["hits"].as_array().map(Vec::len).unwrap_or(0);
    assert!(
        shown < 10 && shown > 0,
        "whole hits withheld from the tail: {held}"
    );
    assert!(held.get("budget").is_some(), "the cut is said: {held}");
    for hit in held["hits"].as_array().into_iter().flatten() {
        assert!(
            hit["node_id"].is_string() && hit["name"].is_string(),
            "every hit shown is whole: {held}"
        );
    }
    assert_eq!(
        held["searched"], full["searched"],
        "counts survive the cut: {held}"
    );
}
