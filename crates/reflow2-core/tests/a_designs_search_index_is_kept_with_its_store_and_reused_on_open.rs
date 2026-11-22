//! A design's search index is kept with its store and reused when the design
//! is opened again (`req:a-designs-search-index-is-kept-with-its-store-and-reused-on-open`).
//!
//! FOUND 2026-11-22
//! (`fact:root-cause-a-hub-wide-read-after-a-quiet-spell-reopens-and-reindexes-every-member-one-at-a-time-2026-11-22`):
//! a hosted server closes a design nobody used for 15 minutes, and every open
//! rebuilt the design's full-text index from the store on the request path,
//! so a whole-hub search after a quiet spell paid one rebuild per member.
//!
//! WHAT THESE PIN, at the open every door goes through
//! (`DesignGraph::open_rocksdb`):
//! - an open of an unchanged design reuses the index the store kept;
//! - a write between two opens is found after reopening, with no rebuild;
//! - a store that moved behind its kept index — rewritten by a build that
//!   keeps no index, or restored from a backup on either side — is NEVER
//!   searched through that index: it is rebuilt, even when the store holds
//!   exactly as many nodes as the index has documents, which is the case a
//!   comparison of counts could not tell from a current one;
//! - a missing or unreadable index is rebuilt rather than failing the open.
//!
//! The observable is the engine's rebuild counter
//! (`DesignGraph::search_index_rebuilds`), never a timing.

use std::path::{Path, PathBuf};

use reflow2_core::DesignGraph;
use reflow2_core::foundation::core::Value;

/// A design with three searchable nodes, written and closed. Returns the
/// store's path.
fn written_and_closed(dir: &Path) -> String {
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

/// The ids a search for `words` returns, best first.
fn found(g: &DesignGraph, words: &str) -> Vec<String> {
    g.search_design(words, None, 10)
        .unwrap_or_else(|e| panic!("search for {words:?} answered: {e}"))
        .hits
        .into_iter()
        .map(|h| h.node_id)
        .collect()
}

/// Rewrite `req:stripes` so no word of the zebra remains in it: the same
/// number of nodes, different words.
fn stripes_become_spots(g: &mut DesignGraph) {
    g.upsert_node(
        "Requirement",
        "req:stripes",
        [
            (
                "name".to_string(),
                Value::String("The enclosure shows an okapi coat".into()),
            ),
            (
                "statement".to_string(),
                Value::String("Visitors see the okapi coat from the path.".into()),
            ),
        ]
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>(),
    )
    .expect("rewrite");
}

/// Copy a directory tree, leaving out the entries `skip` names at its top.
fn copy_tree(from: &Path, to: &Path, skip: &[&str]) {
    std::fs::create_dir_all(to).expect("mkdir");
    for entry in std::fs::read_dir(from).expect("read dir").flatten() {
        let name = entry.file_name();
        if skip.iter().any(|s| name == *s) {
            continue;
        }
        let target = to.join(&name);
        if entry.file_type().expect("type").is_dir() {
            copy_tree(&entry.path(), &target, &[]);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy");
        }
    }
}

/// Empty `dir` of everything but the entries `keep` names.
fn empty_but(dir: &Path, keep: &[&str]) {
    for entry in std::fs::read_dir(dir).expect("read dir").flatten() {
        let name = entry.file_name();
        if keep.iter().any(|k| name == *k) {
            continue;
        }
        if entry.file_type().expect("type").is_dir() {
            std::fs::remove_dir_all(entry.path()).expect("rm dir");
        } else {
            std::fs::remove_file(entry.path()).expect("rm");
        }
    }
}

fn index_dir(store: &str) -> PathBuf {
    Path::new(store).join("fulltext")
}

#[test]
fn a_second_open_of_an_unchanged_design_reuses_its_index() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = written_and_closed(dir.path());

    let g = DesignGraph::open_rocksdb(&path).expect("reopen");
    assert_eq!(
        g.search_index_rebuilds(),
        0,
        "the index the store kept is reused, not rebuilt"
    );
    assert_eq!(found(&g, "zebra pattern"), vec!["req:stripes".to_string()]);
    drop(g);

    // And again: reusing it does not use it up.
    let g = DesignGraph::open_rocksdb(&path).expect("reopen again");
    assert_eq!(g.search_index_rebuilds(), 0);
    assert_eq!(found(&g, "trough"), vec!["req:water".to_string()]);
}

#[test]
fn a_write_between_two_opens_is_found_after_reopening() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = written_and_closed(dir.path());

    {
        let mut g = DesignGraph::open_rocksdb(&path).expect("reopen");
        g.add_requirement(
            "req:shade",
            "The enclosure has shade",
            "A canopy of acacia covers the north side.",
        )
        .expect("a write between two opens");
        stripes_become_spots(&mut g);
        // Found in the session that wrote it, exactly as before.
        assert_eq!(found(&g, "acacia"), vec!["req:shade".to_string()]);
    }

    let g = DesignGraph::open_rocksdb(&path).expect("reopen after the write");
    assert_eq!(found(&g, "acacia"), vec!["req:shade".to_string()]);
    assert_eq!(found(&g, "okapi"), vec!["req:stripes".to_string()]);
    assert!(
        found(&g, "zebra").is_empty(),
        "the words the write replaced are gone"
    );
    assert_eq!(
        g.search_index_rebuilds(),
        0,
        "a session that closed cleanly leaves an index the next open can reuse"
    );
}

/// A build that keeps no index (one without the `fulltext` feature, or a
/// store opened under a schema that searches nothing) rewrites the store and
/// leaves the index as it was. Same node count, different words.
#[test]
fn a_store_rewritten_by_a_build_that_keeps_no_index_is_never_searched_through_the_old_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = written_and_closed(dir.path());
    let graph_id = DesignGraph::open_rocksdb(&path)
        .expect("reopen")
        .graph_id()
        .to_string();

    {
        let mut schema = reflow2_core::schema::load_schema().expect("schema");
        for def in schema.node_types.values_mut() {
            for prop in def.properties.values_mut() {
                prop.fulltext = false;
            }
        }
        let mut engine = reflow2_core::foundation::store::StorageEngine::new_rocksdb(schema, &path)
            .expect("open the store under a schema that keeps no index");
        let mut node = engine
            .get_node(&graph_id, "Requirement", "req:stripes")
            .expect("read")
            .expect("the node");
        node.properties.insert(
            "name".into(),
            Value::String("The enclosure shows an okapi coat".into()),
        );
        node.properties.insert(
            "statement".into(),
            Value::String("Visitors see the okapi coat from the path.".into()),
        );
        engine
            .replace_node_properties(&graph_id, "Requirement", "req:stripes", node.properties)
            .expect("rewrite behind the index")
            .expect("the node exists");
    }

    let g = DesignGraph::open_rocksdb(&path).expect("reopen");
    assert_eq!(
        found(&g, "okapi"),
        vec!["req:stripes".to_string()],
        "what the store says now is found"
    );
    assert!(
        found(&g, "zebra").is_empty(),
        "what the store no longer says is not found"
    );
    assert_eq!(g.search_index_rebuilds(), 1, "the open rebuilt the index");
}

/// The store restored from a backup taken BEFORE a write, under the index
/// kept AFTER it: the index is ahead of its store.
#[test]
fn a_store_restored_behind_its_kept_index_is_never_searched_through_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = written_and_closed(dir.path());
    let backup = tempfile::tempdir().expect("backup dir");
    copy_tree(Path::new(&path), backup.path(), &[]);

    {
        let mut g = DesignGraph::open_rocksdb(&path).expect("reopen");
        stripes_become_spots(&mut g);
    }
    // Restore the store, keep the newer index.
    empty_but(Path::new(&path), &["fulltext"]);
    copy_tree(backup.path(), Path::new(&path), &["fulltext"]);

    let g = DesignGraph::open_rocksdb(&path).expect("open the restored store");
    assert_eq!(
        found(&g, "zebra pattern"),
        vec!["req:stripes".to_string()],
        "the restored store's words are found"
    );
    assert!(
        found(&g, "okapi").is_empty(),
        "a word only the newer index held is not answered from it"
    );
    assert_eq!(g.search_index_rebuilds(), 1);
}

/// The index restored from a backup taken BEFORE a write, under the store
/// kept AFTER it: the index is behind its store.
#[test]
fn a_kept_index_behind_its_store_is_never_searched() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = written_and_closed(dir.path());
    let backup = tempfile::tempdir().expect("backup dir");
    copy_tree(&index_dir(&path), backup.path(), &[]);

    {
        let mut g = DesignGraph::open_rocksdb(&path).expect("reopen");
        stripes_become_spots(&mut g);
    }
    std::fs::remove_dir_all(index_dir(&path)).expect("rm index");
    copy_tree(backup.path(), &index_dir(&path), &[]);

    let g = DesignGraph::open_rocksdb(&path).expect("open under the older index");
    assert_eq!(found(&g, "okapi"), vec!["req:stripes".to_string()]);
    assert!(found(&g, "zebra").is_empty());
    assert_eq!(g.search_index_rebuilds(), 1);
}

#[test]
fn a_missing_index_is_rebuilt() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = written_and_closed(dir.path());
    std::fs::remove_dir_all(index_dir(&path)).expect("rm index");

    let g = DesignGraph::open_rocksdb(&path).expect("reopen");
    assert_eq!(found(&g, "zebra"), vec!["req:stripes".to_string()]);
    assert_eq!(g.search_index_rebuilds(), 1);
}

/// An index whose own record of its segments cannot be read is a derived
/// copy that cannot be used, not a reason the design cannot be opened.
#[test]
fn a_corrupt_index_is_rebuilt() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = written_and_closed(dir.path());
    std::fs::write(
        index_dir(&path).join("meta.json"),
        b"{ not the index's meta",
    )
    .expect("corrupt the index");

    let g = DesignGraph::open_rocksdb(&path).expect("an unreadable index does not fail the open");
    assert_eq!(found(&g, "zebra"), vec!["req:stripes".to_string()]);
    assert_eq!(g.search_index_rebuilds(), 1);
    drop(g);

    // Rebuilt for good: the next open reuses it.
    let g = DesignGraph::open_rocksdb(&path).expect("reopen");
    assert_eq!(g.search_index_rebuilds(), 0);
    assert_eq!(found(&g, "zebra"), vec!["req:stripes".to_string()]);
}
