//! A reply asked at N through a hub's address arrives at N or less, what the
//! hub adds counted (PR B "replies fit", item 2).
//!
//! `fact:root-cause-a-reasoning-read-back-outgrows-the-client-on-repeated-premise-prose-and-text-added-after-the-budget-2026-10-22`:
//! a reasoning_report page asked at 60,000 through the hub reached the client
//! at 63,998 characters. The member held its payload to its budget; the hub
//! then prefixed 287 ids with the member's graph_id (+4,879) and added its
//! view_note (+275), after the budget.
//!
//! The class: any read that takes `budget_chars`, routed to a member, with
//! enough ids in the reply that the prefixes alone pass the budget.

use serde_json::{Value, json};

use super::a_hubs_address_serves_a_view_over_its_members::{
    GATEWAY, Session, mint_holding, ok, serve, tmp_root,
};

fn size(v: &Value) -> usize {
    v.to_string().len()
}

#[test]
fn a_routed_read_asked_at_n_arrives_within_n_with_its_ids_prefixed() {
    let root = tmp_root("fits");
    let hub_dir = root.join("hub");
    let member_dir = root.join("member");
    std::fs::create_dir_all(&hub_dir).unwrap();
    std::fs::create_dir_all(&member_dir).unwrap();
    let h = mint_holding(&hub_dir, &[("req:hub".into(), "the hub".into())]);
    // Many short ids: the prefix is a large share of each row.
    let reqs: Vec<(String, String)> = (0..160)
        .map(|i| (format!("req:r{i:03}"), format!("requirement {i}")))
        .collect();
    let m = mint_holding(&member_dir, &reqs);
    let server = serve(&root, Some(GATEWAY));
    let readable = [h.as_str(), m.as_str()];
    let view = [m.as_str()];

    // The member's own reply, unbudgeted by the hub, sets the scale.
    let mut on_m = Session::open(server.port, &m);
    let whole = ok(
        &on_m.call(
            "scan_nodes",
            json!({"node_type": "Requirement", "brief": true, "budget_chars": 400_000}),
            Some(&readable),
            None,
        ),
        "the member's own scan",
    );
    // Just under the whole: the member gives a little to fit, and the
    // prefixes on 160 ids (17 characters each) would put it far over.
    let budget = size(&whole) - 200;

    // On its own address, the member fits the budget, with room the hub's
    // additions would overrun.
    let direct = ok(
        &on_m.call(
            "scan_nodes",
            json!({"node_type": "Requirement", "brief": true, "budget_chars": budget}),
            Some(&readable),
            None,
        ),
        "the member at the budget",
    );
    assert!(size(&direct) <= budget, "the member holds its own budget");

    let mut on_h = Session::open(server.port, &h);
    let through = ok(
        &on_h.call(
            "scan_nodes",
            json!({"node_type": "Requirement", "brief": true, "budget_chars": budget, "member": m}),
            Some(&readable),
            Some(&view),
        ),
        "the member through the hub",
    );
    let n = size(&through);
    assert!(
        n <= budget,
        "asked at {budget} through the hub, answered {n} (the member alone answered {})",
        size(&direct)
    );
    eprintln!(
        "whole {} budget {budget} direct {} through {n}",
        size(&whole),
        size(&direct)
    );
    let note = through["view_note"].as_str().unwrap_or_default();
    assert!(
        note.contains(&format!("held to budget_chars {budget}")),
        "the reply says it was held, prefixes counted: {note}"
    );
    // Every id still carries its design's prefix, and the count is kept.
    let items = through["items"].as_array().cloned().unwrap_or_default();
    assert!(!items.is_empty(), "{through}");
    for i in &items {
        let id = i["id"]
            .as_str()
            .or(i["node_id"].as_str())
            .unwrap_or_default();
        assert!(id.starts_with(&format!("{m}:")), "{i}");
    }
    for key in ["total", "matched", "count"] {
        if let Some(t) = whole.get(key).filter(|_| key != "count") {
            assert_eq!(&through[key], t, "`{key}` is kept: {through}");
        }
    }

    // A reply that fits is not asked twice: no held sentence.
    let roomy = ok(
        &on_h.call(
            "scan_nodes",
            json!({"node_type": "Requirement", "brief": true, "budget_chars": 400_000, "member": m}),
            Some(&readable),
            Some(&view),
        ),
        "the member through the hub, with room",
    );
    assert!(
        !roomy["view_note"]
            .as_str()
            .unwrap_or_default()
            .contains("held to budget_chars"),
        "{}",
        roomy["view_note"]
    );
}
