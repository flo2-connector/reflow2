//! A design the server opens again reuses the search index its store kept
//! (`req:a-designs-search-index-is-kept-with-its-store-and-reused-on-open`).
//!
//! FOUND 2026-11-22
//! (`fact:root-cause-a-hub-wide-read-after-a-quiet-spell-reopens-and-reindexes-every-member-one-at-a-time-2026-11-22`):
//! the registry on flo2.io closes a design nobody used for 15 minutes and
//! opens it again through `ReflowService::new_reporting`, which rebuilt the
//! design's full-text index from the store on every open, on the request
//! path. A whole-hub search after a quiet spell paid that once per member, one
//! after another: a median of 13.5 s cold against 0.6 s warm.
//!
//! The core's own test (`a_designs_search_index_is_kept_with_its_store_and_reused_on_open.rs`)
//! pins the match check. This pins the door the registry and the stdio server
//! open through, counting rebuilds (`search_index_rebuilds`), never timing.

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::json;

fn open(store: &str) -> ReflowService {
    ReflowService::new_reporting(store)
        .expect("a design on disk opens")
        .0
}

async fn add(s: &ReflowService, id: &str, name: &str, statement: &str) {
    s.add_requirement(Parameters(
        serde_json::from_value(json!({ "id": id, "name": name, "statement": statement }))
            .unwrap(),
    ))
    .await
    .expect("requirement");
}

async fn found(s: &ReflowService, words: &str) -> Vec<String> {
    let out = s
        .search_design(Parameters(
            serde_json::from_value(json!({ "query": words })).unwrap(),
        ))
        .await
        .expect("search")
        .structured_content
        .expect("a search answers with an object");
    out["hits"]
        .as_array()
        .expect("hits")
        .iter()
        .map(|h| h["node_id"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn a_design_opened_again_reuses_its_index_and_finds_what_was_written_between() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = dir.path().join("graph");
    let store = store.to_str().expect("utf-8").to_string();

    {
        let s = open(&store);
        add(
            &s,
            "req:stripes",
            "The enclosure shows a zebra pattern",
            "Visitors see the zebra pattern from the path.",
        )
        .await;
    }

    // The second open of an unchanged design rebuilds nothing.
    {
        let s = open(&store);
        assert_eq!(
            s.search_index_rebuilds().await,
            0,
            "opening an unchanged design again reuses the index its store kept"
        );
        assert_eq!(found(&s, "zebra").await, vec!["req:stripes".to_string()]);
        // A write between two opens...
        add(
            &s,
            "req:shade",
            "The enclosure has shade",
            "A canopy of acacia covers the north side.",
        )
        .await;
        assert_eq!(found(&s, "acacia").await, vec!["req:shade".to_string()]);
    }

    // ...is found after reopening, still with no rebuild.
    let s = open(&store);
    assert_eq!(found(&s, "acacia").await, vec!["req:shade".to_string()]);
    assert_eq!(found(&s, "zebra").await, vec!["req:stripes".to_string()]);
    assert_eq!(s.search_index_rebuilds().await, 0);
}

#[tokio::test]
async fn a_design_whose_index_is_gone_is_rebuilt_on_open() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = dir.path().join("graph");
    let store = store.to_str().expect("utf-8").to_string();
    {
        let s = open(&store);
        add(
            &s,
            "req:stripes",
            "The enclosure shows a zebra pattern",
            "Visitors see the zebra pattern from the path.",
        )
        .await;
    }
    std::fs::remove_dir_all(std::path::Path::new(&store).join("fulltext")).expect("rm index");

    let s = open(&store);
    assert_eq!(s.search_index_rebuilds().await, 1, "a missing index is rebuilt");
    assert_eq!(found(&s, "zebra").await, vec!["req:stripes".to_string()]);
}
