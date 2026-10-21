//! Through a hub's address, a search names the designs the hub PINS that its
//! view did not reach, and never a design it did search — the view half of
//! `a_search_names_every_pinned_design_it_did_not_search.rs` (the small defect
//! inside `req:one-search-reaches-every-design-a-hub-tracks`).
//!
//! H pins A, which is in its view and searched, and a design outside the
//! view, which is not. The merged reply names only the second, and says which
//! design pins it.

use serde_json::json;

use super::a_hubs_address_serves_a_view_over_its_members::{
    GATEWAY, Session, hub_and_members, ok, serve,
};

#[test]
fn a_merged_search_names_only_the_pinned_designs_its_view_did_not_search() {
    let (root, h, a, b) = hub_and_members("view-not-searched");
    let server = serve(&root, Some(GATEWAY));
    let mut on_h = Session::open(server.port, &h);
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];
    for (id, graph_id) in [
        ("dep:alpha", a.as_str()),
        ("dep:outside", "0utside0design00"),
    ] {
        let reply = on_h.call(
            "external_dependency",
            json!({"id": id, "name": id, "source": "https://example.org/x", "version": "v1",
                   "graph_id": graph_id, "relation": ["part_of"]}),
            Some(&readable),
            None,
        );
        ok(&reply, "declaring a pin on H");
    }

    let reply = on_h.call(
        "search_design",
        json!({"query": "zanzibar"}),
        Some(&readable),
        Some(&view),
    );
    let got = ok(&reply, "search_design through H");
    let named = got["not_searched"]
        .as_array()
        .unwrap_or_else(|| panic!("a pinned design outside the view is named: {got}"));
    assert_eq!(
        named.len(),
        1,
        "only the design the view did not search: {got}"
    );
    assert_eq!(named[0]["design"], json!("0utside0design00"), "{got}");
    assert_eq!(named[0]["pinned_by"], json!(h), "{got}");
    assert!(
        got["not_searched_note"]
            .as_str()
            .is_some_and(|n| n.contains("designs_searched")),
        "{got}"
    );
}
