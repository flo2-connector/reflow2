//! A store opened without its search index rebuilds it — it never answers
//! "nothing matched" from an index that does not hold what the store holds.
//!
//! FOUND 2026-10-02 (fact:root-cause-a-held-design-answers-search-through-the-door-with-a-false-nothing-matched-2026-10-02):
//! while another process held a design, `reflow2-mcp --call search_design`
//! answered from a copy of the store directory that left out the nested
//! `fulltext/` index. The copy opened cleanly onto an empty index — Tantivy
//! creates one where none exists — and `search_design` said `{"hits": []}`,
//! `topic_report` "NOTHING MATCHED … across 2 node(s)", exit 0, for a word one
//! of those two nodes held.
//!
//! THE CLASS IS NOT THE COPY. Any open of a store whose index was not built
//! from it lands in the same place: a directory copied or restored without its
//! subdirectory, a store written by a build without the `fulltext` feature, an
//! index lost by hand. So the guarantee is a property of the OPEN, and this
//! pins it there rather than on `--call` (whose own test pins the door).

#![cfg(all(feature = "rocksdb", feature = "fulltext"))]

use reflow2_core::DesignGraph;

fn written_and_closed(dir: &std::path::Path) -> String {
    let path = dir.join("graph");
    let path = path.to_str().expect("utf-8 temp path").to_string();
    let mut g = DesignGraph::open_rocksdb(&path).expect("open");
    g.add_project("proj:zoo", "Zoo").expect("project");
    g.add_requirement(
        "req:stripes",
        "The enclosure shows a zebra pattern",
        "Visitors see the zebra pattern from the path.",
    )
    .expect("requirement");
    g.add_requirement(
        "req:water",
        "The enclosure has water",
        "A trough is filled daily.",
    )
    .expect("requirement");
    path
}

#[test]
fn a_store_whose_index_directory_is_gone_finds_its_words_after_reopening() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = written_and_closed(dir.path());

    // The index directory is the only nested directory in the store, and the
    // one a flat copy of it leaves out.
    std::fs::remove_dir_all(std::path::Path::new(&path).join("fulltext"))
        .expect("the store keeps its index in a nested fulltext/ directory");

    let g = DesignGraph::open_rocksdb(&path).expect("reopen");
    let found = g.search_design("zebra pattern", None, 10).expect("search");
    assert_eq!(
        found.hits.first().map(|h| h.node_id.as_str()),
        Some("req:stripes"),
        "the word the store holds is found, not answered with an empty list: {found:?}"
    );
    assert_eq!(
        found.searched, 3,
        "the search ran over every node the store holds that search can see"
    );

    // And the open says what it repaired, rather than repairing in silence.
    let rebuilt = g
        .search_rebuilt_on_open()
        .expect("an open that rebuilt the index says so");
    assert_eq!(rebuilt.indexed_before, 0);
    assert_eq!(rebuilt.searchable, 3);

    let topic = g
        .topic_report("zebra pattern", None, 10_000)
        .expect("topic");
    assert_eq!(topic.count, 1, "{}", topic.not_found);
    assert_eq!(topic.searched, 3);
}

#[test]
fn a_store_whose_index_is_in_step_is_not_rebuilt() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = written_and_closed(dir.path());
    let g = DesignGraph::open_rocksdb(&path).expect("reopen");
    assert!(
        g.search_rebuilt_on_open().is_none(),
        "an index that already holds what the store holds is left alone"
    );
    assert_eq!(
        g.search_design("zebra", None, 10)
            .expect("search")
            .hits
            .len(),
        1
    );
}

/// "Nothing matched" names the population it searched, so an empty answer
/// can never read like a searched absence.
#[test]
fn a_true_miss_says_how_many_nodes_it_searched() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = written_and_closed(dir.path());
    let g = DesignGraph::open_rocksdb(&path).expect("reopen");
    let miss = g.search_design("okapi", None, 10).expect("search");
    assert!(miss.hits.is_empty());
    assert_eq!(miss.searched, 3);
    let topic = g.topic_report("okapi", None, 10_000).expect("topic");
    assert_eq!(topic.count, 0);
    assert!(
        topic.not_found.contains("in the 3 node(s) searched"),
        "the not-found line names the population searched: {}",
        topic.not_found
    );
}
