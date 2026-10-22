//! THROUGH A HUB'S ADDRESS ON A REGISTRY (flo2.io's shape), ONE COMPOSED CHECK
//! reads the hub's members from their LIVE STORES —
//! `req:one-composed-check-runs-over-a-hub-and-several-of-its-designs`, the
//! hosted half of `tests/one_composed_check_runs_over_a_hub_and_several_of_its_designs.rs`.
//!
//! WHAT IS ASSERTED, over the real binary serving H, A and B under one root
//! behind a trusted gateway:
//!   · `compose_and_analyse {tracked_designs: true}` through H composes H with
//!     every member of the view, each read from its live store, and the
//!     `view_note` says so; a member's ids carry its graph_id prefix;
//!   · a member outside the readable set is named `unreachable` in
//!     `not_composed`, and nothing it holds is revealed;
//!   · `other_graph_ids` naming one member composes that member alone;
//!   · the one-other-design call (`other_graph_id`) through H answers in its
//!     old shape, exactly as it did before (the path does not regress);
//!   · where the members touch is computed from what their LIVE STORES hold
//!     (phase 7 step 4): every pair of the designs composed has a
//!     `link_status`, and a pair with a member outside the readable set is
//!     `not_checked`, never `never_touch`.

use serde_json::json;

use super::a_hubs_address_serves_a_view_over_its_members::{
    GATEWAY, Session, hub_and_members, ok, serve,
};

#[test]
fn through_a_hubs_address_the_composed_check_reads_every_member_live() {
    let (root, h, a, b) = hub_and_members("composed-live");
    let server = serve(&root, Some(GATEWAY));
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];

    let reply = on_h.call(
        "compose_and_analyse",
        json!({"tracked_designs": true}),
        Some(&readable),
        Some(&view),
    );
    let got = ok(&reply, "the composed check through H");
    assert_eq!(got["designs_composed"], json!([h, a, b]), "{got}");
    for m in [&a, &b] {
        assert!(
            got["read_from"][m]
                .as_str()
                .is_some_and(|r| r.contains("live store")),
            "{m} was read from its live store: {got}"
        );
    }
    assert_eq!(got["not_composed"], json!([]), "{got}");
    let note = got["view_note"].as_str().unwrap_or_default();
    assert!(
        note.contains("LIVE STORE") && note.contains(&a) && note.contains(&b),
        "the note says it composed every member, live: {got}"
    );
    for m in [&a, &b] {
        let own = got["by_design"][m]
            .as_array()
            .expect("a member's own findings");
        assert!(
            own.iter().all(|f| f["affected_ids"]
                .as_array()
                .unwrap()
                .iter()
                .all(|i| i.as_str().unwrap().starts_with(&format!("{m}:")))),
            "{m}'s findings name its ids with its prefix: {got}"
        );
    }
    // Every pair of the three designs composed has a status, read from the
    // live stores; none was left unchecked.
    let ls = &got["link_status"];
    let statused = ls["linked"].as_array().map_or(0, Vec::len)
        + ls["candidates_only"].as_array().map_or(0, Vec::len)
        + ls["never_touch"].as_array().map_or(0, Vec::len);
    assert_eq!(statused, 3, "three designs, three pairs: {ls}");
    assert_eq!(ls["not_checked"], json!([]), "{ls}");
    assert!(got["computed_links"].is_array(), "{got}");
}

#[test]
fn a_member_outside_the_readable_set_is_named_unreachable_and_reveals_nothing() {
    let (root, h, a, b) = hub_and_members("composed-unreachable");
    let server = serve(&root, Some(GATEWAY));
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), a.as_str()];
    let view = [a.as_str(), b.as_str()];

    let got = ok(
        &on_h.call(
            "compose_and_analyse",
            json!({"tracked_designs": true}),
            Some(&readable),
            Some(&view),
        ),
        "the composed check with B outside the readable set",
    );
    assert_eq!(got["designs_composed"], json!([h, a]), "{got}");
    let missed = &got["not_composed"][0];
    assert_eq!(missed["design"], json!(b), "{got}");
    assert_eq!(missed["state"], json!("unreachable"), "{got}");
    assert!(
        !got.to_string().contains("only-in-beta"),
        "nothing B holds is revealed: {got}"
    );
    assert_eq!(
        got["link_status"]["not_checked"],
        json!([format!("{h}×{b}"), format!("{a}×{b}")]),
        "a pair with a member not read is not checked, never `never_touch`: {got}"
    );

    // Naming one member composes that one alone.
    let one = ok(
        &on_h.call(
            "compose_and_analyse",
            json!({"other_graph_ids": [a]}),
            Some(&readable),
            Some(&view),
        ),
        "the composed check naming A",
    );
    assert_eq!(one["designs_composed"], json!([h, a]), "{one}");
}

#[test]
fn the_one_other_design_call_through_a_hubs_address_keeps_its_old_shape() {
    let (root, h, a, b) = hub_and_members("composed-old-shape");
    let server = serve(&root, Some(GATEWAY));
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];
    let got = ok(
        &on_h.call(
            "compose_and_analyse",
            json!({"other_graph_id": a}),
            Some(&readable),
            Some(&view),
        ),
        "the one-other-design call through H",
    );
    for key in [
        "imported_graph_id",
        "namespace",
        "imported_nodes",
        "imported_edges",
        "skipped_edges",
        "seam_findings",
        "our_findings",
        "their_findings",
    ] {
        assert!(got.get(key).is_some(), "{key}: {got}");
    }
    assert_eq!(got["imported_graph_id"], json!(a), "{got}");
    assert!(got.get("designs_composed").is_none(), "{got}");
    // ...and gains the link fields, the pair's status among them.
    for key in [
        "declared_links",
        "computed_links",
        "candidates",
        "set_aside",
    ] {
        assert!(got.get(key).is_some(), "{key}: {got}");
    }
    assert!(got["link_status"]["status"].is_string(), "{got}");
}
