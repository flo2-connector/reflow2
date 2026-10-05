use super::*;
use crate::foundation::core::Schema;
use crate::props;

/// A schema with one full-text node type (`Document` with `title`/`body`
/// fulltext) and one without (`Tag`).
fn ft_schema() -> Schema {
    Schema::from_yaml(
        r#"
schema:
  name: ft
  version: 1
  node_types:
    Document:
      properties:
        title: { type: string, fulltext: true }
        body:  { type: string, fulltext: true }
        author: { type: string, indexed: true }
    Tag:
      properties:
        name: { type: string, indexed: true }
  edge_types: {}
"#,
    )
    .unwrap()
}

/// RocksDB engine over a fresh temp dir (leaked — the engine holds it open
/// for the test, mirroring `test_engine`'s rocksdb arm).
#[cfg(feature = "rocksdb")]
fn rocks_engine(schema: Schema) -> StorageEngine {
    let dir = tempfile::tempdir().expect("temp dir").keep();
    let path = dir.to_str().expect("utf-8 temp path");
    StorageEngine::new_rocksdb(schema, path).expect("open rocksdb engine")
}

#[test]
fn no_index_built_when_schema_has_no_fulltext() {
    // Schema with no fulltext property → search is a clean empty, never an
    // error, and no index is constructed.
    let schema = Schema::from_yaml(
        r#"
schema:
  name: plain
  version: 1
  node_types:
    Tag:
      properties:
        name: { type: string, indexed: true }
  edge_types: {}
"#,
    )
    .unwrap();
    let engine = StorageEngine::new_in_memory(schema);
    assert!(
        engine
            .search_fulltext("g1", "anything", None, 10)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn create_indexes_and_delete_clears_in_memory() {
    let mut engine = StorageEngine::new_in_memory(ft_schema());
    engine
        .create_node(
            "g1",
            "Document",
            "n1",
            props! { "title" => "Rust Graphs", "body" => "embedded full text search" },
        )
        .unwrap();

    // Findable by a token from either fulltext field.
    let hits = engine.search_fulltext("g1", "graphs", None, 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].node_id, "n1");
    assert_eq!(hits[0].node_type, "Document");
    assert_eq!(
        engine
            .search_fulltext("g1", "search", None, 10)
            .unwrap()
            .len(),
        1
    );

    // Delete clears the document.
    engine.delete_node("g1", "Document", "n1").unwrap();
    assert!(
        engine
            .search_fulltext("g1", "graphs", None, 10)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn replace_properties_reindexes() {
    let mut engine = StorageEngine::new_in_memory(ft_schema());
    engine
        .create_node(
            "g1",
            "Document",
            "n1",
            props! { "title" => "alpha", "body" => "first" },
        )
        .unwrap();
    assert_eq!(
        engine
            .search_fulltext("g1", "alpha", None, 10)
            .unwrap()
            .len(),
        1
    );

    engine
        .replace_node_properties(
            "g1",
            "Document",
            "n1",
            props! { "title" => "beta", "body" => "second" },
        )
        .unwrap();
    // Old token gone, new token present — replace semantics held.
    assert!(
        engine
            .search_fulltext("g1", "alpha", None, 10)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        engine
            .search_fulltext("g1", "beta", None, 10)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn node_type_with_no_fulltext_is_not_searchable() {
    let mut engine = StorageEngine::new_in_memory(ft_schema());
    engine
        .create_node("g1", "Tag", "t1", props! { "name" => "important" })
        .unwrap();
    // Tag has no fulltext property → never indexed.
    assert!(
        engine
            .search_fulltext("g1", "important", None, 10)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn batch_commit_makes_writes_visible_and_discard_rolls_back() {
    let mut engine = StorageEngine::new_in_memory(ft_schema());

    // Committed batch → searchable.
    engine.begin_batch();
    engine
        .create_node(
            "g1",
            "Document",
            "n1",
            props! { "title" => "committed", "body" => "x" },
        )
        .unwrap();
    // Inside the batch a search answers for the batch's own writes, as every
    // other read does. It used to be empty here — Tantivy's uncommitted text
    // is invisible — which was harmless until a tool call's write unit made
    // the duplicate guard search for the node its own call had just written
    // (dec:idea-a-refused-typed-write-stores-nothing).
    assert_eq!(
        engine
            .search_fulltext("g1", "committed", None, 10)
            .unwrap()
            .len(),
        1,
        "read-your-own-writes holds for search inside a batch"
    );
    engine.commit_batch().unwrap();
    assert_eq!(
        engine
            .search_fulltext("g1", "committed", None, 10)
            .unwrap()
            .len(),
        1
    );

    // Discarded batch → rolled back, never visible.
    engine.begin_batch();
    engine
        .create_node(
            "g1",
            "Document",
            "n2",
            props! { "title" => "transient", "body" => "y" },
        )
        .unwrap();
    engine.discard_batch();
    assert!(
        engine
            .search_fulltext("g1", "transient", None, 10)
            .unwrap()
            .is_empty()
    );
    // The earlier committed doc is untouched by the rollback.
    assert_eq!(
        engine
            .search_fulltext("g1", "committed", None, 10)
            .unwrap()
            .len(),
        1
    );
}

fn found(engine: &StorageEngine, word: &str) -> Vec<String> {
    let mut ids: Vec<String> = engine
        .search_fulltext("g1", word, None, 10)
        .unwrap()
        .into_iter()
        .map(|h| h.node_id)
        .collect();
    ids.sort();
    ids
}

/// A search inside a batch PUBLISHES the batch's text so far; discarding the
/// batch afterwards must take it back out, and put back what a revise inside
/// the batch replaced. Rolling the writer back alone cannot: part of the
/// batch's text is already committed to the index.
#[test]
fn a_discard_after_a_search_inside_the_batch_restores_the_index() {
    let mut engine = StorageEngine::new_in_memory(ft_schema());
    engine
        .create_node(
            "g1",
            "Document",
            "kept",
            props! { "title" => "original", "body" => "before" },
        )
        .unwrap();

    engine.begin_batch();
    engine
        .create_node(
            "g1",
            "Document",
            "staged",
            props! { "title" => "transient", "body" => "y" },
        )
        .unwrap();
    engine
        .create_node(
            "g1",
            "Document",
            "kept",
            props! { "title" => "rewritten", "body" => "inside" },
        )
        .unwrap();
    // The search publishes both.
    assert_eq!(found(&engine, "transient"), vec!["staged"]);
    assert_eq!(found(&engine, "rewritten"), vec!["kept"]);
    assert!(found(&engine, "original").is_empty());
    // Written after the publish: still pending when the batch is discarded.
    engine
        .create_node(
            "g1",
            "Document",
            "late",
            props! { "title" => "latecomer", "body" => "z" },
        )
        .unwrap();
    engine.discard_batch();

    assert!(
        found(&engine, "transient").is_empty(),
        "a discarded create is unindexed"
    );
    assert!(found(&engine, "latecomer").is_empty());
    assert!(
        found(&engine, "rewritten").is_empty(),
        "a discarded revise is unindexed"
    );
    assert_eq!(
        found(&engine, "original"),
        vec!["kept"],
        "and what it replaced is searchable again"
    );
}

/// An inner discard after a publish drops the inner batch's text and keeps the
/// outer batch's, which then commits with it.
#[test]
fn an_inner_discard_after_a_search_keeps_the_outer_batchs_text() {
    let mut engine = StorageEngine::new_in_memory(ft_schema());
    engine.begin_batch();
    engine
        .create_node(
            "g1",
            "Document",
            "outer",
            props! { "title" => "outerword", "body" => "a" },
        )
        .unwrap();
    engine.begin_batch();
    engine
        .create_node(
            "g1",
            "Document",
            "inner",
            props! { "title" => "innerword", "body" => "b" },
        )
        .unwrap();
    assert_eq!(found(&engine, "innerword"), vec!["inner"]);
    engine.discard_batch();
    assert!(
        found(&engine, "innerword").is_empty(),
        "the inner discard takes its published text back out"
    );
    assert_eq!(found(&engine, "outerword"), vec!["outer"]);
    engine.commit_batch().unwrap();
    assert_eq!(found(&engine, "outerword"), vec!["outer"]);
    assert!(found(&engine, "innerword").is_empty());
}

#[cfg(feature = "rocksdb")]
#[test]
fn reindex_rebuilds_and_survives_rocksdb_reopen() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().to_str().expect("utf-8 temp path").to_string();

    {
        let mut engine =
            StorageEngine::new_rocksdb(ft_schema(), &path).expect("open rocksdb engine");
        engine
            .create_node(
                "g1",
                "Document",
                "n1",
                props! { "title" => "persistent", "body" => "z" },
            )
            .unwrap();
        assert_eq!(
            engine
                .search_fulltext("g1", "persistent", None, 10)
                .unwrap()
                .len(),
            1
        );
    }

    // Reopen the same dir: the on-disk Tantivy index reloads and the doc is
    // still searchable without re-indexing.
    let engine = StorageEngine::new_rocksdb(ft_schema(), &path).expect("reopen rocksdb engine");
    assert_eq!(
        engine
            .search_fulltext("g1", "persistent", None, 10)
            .unwrap()
            .len(),
        1
    );

    // reindex_fulltext is idempotent: rebuild from RocksDB, still one hit.
    let n = engine.reindex_fulltext("g1").unwrap();
    assert_eq!(n, 1);
    assert_eq!(
        engine
            .search_fulltext("g1", "persistent", None, 10)
            .unwrap()
            .len(),
        1
    );
}

// Uses `rocks_engine`, which only exists with the `rocksdb` feature. Without
// this gate the test module fails to COMPILE under
// `--no-default-features --features fulltext`, so a fulltext-only build could
// not run its own tests. CI never caught it because CI builds with default
// features, where `rocksdb` is on.
#[cfg(feature = "rocksdb")]
#[test]
fn scoped_by_graph_and_node_type() {
    let mut engine = rocks_engine(ft_schema());
    engine
        .create_node(
            "g1",
            "Document",
            "d1",
            props! { "title" => "common", "body" => "a" },
        )
        .unwrap();
    engine
        .create_node(
            "g2",
            "Document",
            "d2",
            props! { "title" => "common", "body" => "b" },
        )
        .unwrap();
    engine
        .create_node("g1", "Tag", "t1", props! { "name" => "common" })
        .unwrap();

    // Graph scoping.
    let g1 = engine.search_fulltext("g1", "common", None, 10).unwrap();
    assert_eq!(g1.len(), 1);
    assert_eq!(g1[0].node_id, "d1");
    // node_type filter (Tag isn't indexed anyway, so only Document matches).
    let docs = engine
        .search_fulltext("g1", "common", Some("Document"), 10)
        .unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].node_id, "d1");
}

#[test]
fn clear_graph_drops_fulltext_documents() {
    let mut engine = StorageEngine::new_in_memory(ft_schema());
    engine
        .create_node(
            "g1",
            "Document",
            "n1",
            props! { "title" => "scrubme", "body" => "a" },
        )
        .unwrap();
    assert_eq!(
        engine
            .search_fulltext("g1", "scrubme", None, 10)
            .unwrap()
            .len(),
        1
    );
    engine.clear_graph("g1").unwrap();
    assert!(
        engine
            .search_fulltext("g1", "scrubme", None, 10)
            .unwrap()
            .is_empty()
    );
}

/// #1 regression guard: a discarded batch must not drop a prior committed
/// full-text document (the writer is clean at begin_batch, so the rollback
/// only reverts the batch's own ops).
#[test]
fn discard_batch_preserves_prior_committed_fulltext() {
    let mut engine = StorageEngine::new_in_memory(ft_schema());
    engine
        .create_node(
            "g1",
            "Document",
            "n1",
            props! { "title" => "keepme", "body" => "a" },
        )
        .unwrap();
    assert_eq!(
        engine
            .search_fulltext("g1", "keepme", None, 10)
            .unwrap()
            .len(),
        1
    );

    engine.begin_batch();
    engine
        .create_node(
            "g1",
            "Document",
            "n2",
            props! { "title" => "dropme", "body" => "b" },
        )
        .unwrap();
    engine.discard_batch();

    // Prior committed doc survives; the discarded batch's doc never appears.
    assert_eq!(
        engine
            .search_fulltext("g1", "keepme", None, 10)
            .unwrap()
            .len(),
        1
    );
    assert!(
        engine
            .search_fulltext("g1", "dropme", None, 10)
            .unwrap()
            .is_empty()
    );
}

/// #2: enabling full-text on a live engine (via replace_schema) doesn't
/// build an index, so search/reindex fail loud instead of silently empty /
/// Ok(0).
#[test]
fn fulltext_enabled_at_runtime_fails_loud_until_reopen() {
    let plain = Schema::from_yaml(
        r#"
schema:
  name: p
  version: 1
  node_types:
    Document:
      properties:
        title: { type: string }
  edge_types: {}
"#,
    )
    .unwrap();
    let mut engine = StorageEngine::new_in_memory(plain);
    // Genuinely no full-text → clean empty, no error.
    assert!(
        engine
            .search_fulltext("g1", "x", None, 10)
            .unwrap()
            .is_empty()
    );
    assert_eq!(engine.reindex_fulltext("g1").unwrap(), 0);

    // Enable full-text at runtime; no index is built for the live engine.
    engine.replace_schema(ft_schema());
    assert!(engine.search_fulltext("g1", "x", None, 10).is_err());
    assert!(engine.reindex_fulltext("g1").is_err());
}

/// #6: a rebuild can't run inside an open batch.
#[test]
fn reindex_inside_batch_errors() {
    let mut engine = StorageEngine::new_in_memory(ft_schema());
    engine.begin_batch();
    let err = engine.reindex_fulltext("g1").unwrap_err();
    engine.discard_batch();
    match err {
        DynoError::Storage(msg) => assert!(msg.contains("batch"), "got: {msg}"),
        other => panic!("expected Storage error, got {other:?}"),
    }
}

/// THE CLASS, at the engine: an index that does not hold what the store holds
/// must never answer "nothing matched". Measured 2026-10-02 through `--call` on
/// a held design: the copy it read had an index rebuilt empty, and search said
/// `{"hits": []}` for a word the store held. Here the index loses its documents
/// while the store keeps its nodes — the same shape, built in memory.
#[test]
fn an_index_that_does_not_cover_the_store_refuses_an_empty_answer() {
    let mut engine = StorageEngine::new_in_memory(ft_schema());
    engine
        .create_node(
            "g1",
            "Document",
            "n1",
            props! { "title" => "zebra pattern", "body" => "stripes" },
        )
        .unwrap();
    engine
        .create_node("g1", "Tag", "t1", props! { "name" => "zebra" })
        .unwrap();
    // In step: one searchable node (Tag declares no fulltext), one document.
    assert_eq!(
        engine.fulltext_coverage("g1").unwrap(),
        Some(FulltextCoverage {
            indexed: 1,
            searchable: 1
        })
    );
    assert_eq!(engine.fulltext_indexed("g1").unwrap(), 1);

    // The index loses the graph; the store does not.
    let ti = engine.text_index.as_ref().expect("an index");
    ti.delete_graph("g1").unwrap();
    ti.commit().unwrap();

    let err = engine
        .search_fulltext("g1", "zebra", None, 10)
        .expect_err("an empty answer from an index that holds nothing of the store is refused");
    let msg = err.to_string();
    assert!(
        msg.contains("SEARCH REFUSED") && msg.contains("holds 0") && msg.contains("holds 1"),
        "the refusal names what the index holds and what the store holds: {msg}"
    );

    // The open-time repair rebuilds it, says what it found, and the word is found.
    let found = engine.ensure_fulltext_covers("g1").unwrap();
    assert_eq!(
        found,
        Some(FulltextCoverage {
            indexed: 0,
            searchable: 1
        })
    );
    assert_eq!(
        engine
            .search_fulltext("g1", "zebra", None, 10)
            .unwrap()
            .len(),
        1
    );
    // Covered now, so nothing more to do — and a true miss stays an ordinary empty.
    assert_eq!(engine.ensure_fulltext_covers("g1").unwrap(), None);
    assert!(
        engine
            .search_fulltext("g1", "okapi", None, 10)
            .unwrap()
            .is_empty()
    );
}

/// On disk: a store directory without its `fulltext/` subdirectory — exactly
/// what `--call`'s snapshot copy was — opens onto an EMPTY index. Before
/// 2026-10-02 that answered every query "nothing matched"; now an empty answer
/// refuses until the index is rebuilt from the store.
#[cfg(feature = "rocksdb")]
#[test]
fn a_store_reopened_without_its_index_directory_refuses_until_rebuilt() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().to_str().expect("utf-8 temp path").to_string();
    {
        let mut engine =
            StorageEngine::new_rocksdb(ft_schema(), &path).expect("open rocksdb engine");
        engine
            .create_node(
                "g1",
                "Document",
                "n1",
                props! { "title" => "zebra pattern", "body" => "z" },
            )
            .unwrap();
    }
    std::fs::remove_dir_all(dir.path().join("fulltext")).expect("the index directory exists");

    let engine = StorageEngine::new_rocksdb(ft_schema(), &path).expect("reopen rocksdb engine");
    assert_eq!(
        engine.fulltext_coverage("g1").unwrap(),
        Some(FulltextCoverage {
            indexed: 0,
            searchable: 1
        })
    );
    assert!(engine.search_fulltext("g1", "zebra", None, 10).is_err());
    assert!(engine.ensure_fulltext_covers("g1").unwrap().is_some());
    assert_eq!(
        engine
            .search_fulltext("g1", "zebra", None, 10)
            .unwrap()
            .len(),
        1
    );
}
