//! A design's store is copied, consistently, while it stays open
//! (`cap:a-server-checkpoints-every-design-it-serves-on-a-signal-without-stopping`).
//!
//! FOUND 2026-11-23: flo2.io's nightly backup stopped the engine for about two
//! and a half minutes to get a consistent copy, because nothing in the engine
//! could copy a store it held. These pin the two copies a server takes: of a
//! design it holds open (`checkpoint_to`), and of one nothing in the process
//! holds (`checkpoint_closed_store`), and what each refuses.

use reflow2_core::DesignGraph;

fn design_at(dir: &std::path::Path) -> (String, DesignGraph) {
    let path = dir.join("graph").to_str().expect("utf-8").to_string();
    let mut g = DesignGraph::open_rocksdb(&path).expect("open");
    g.add_project("proj:zoo", "Zoo").expect("project");
    g.add_requirement("req:water", "The enclosure has water", "Filled daily.")
        .expect("requirement");
    (path, g)
}

/// The copy at `copy`, opened as a design: its identity file is copied
/// beside it first, as a server's checkpoint set and a restore both do.
fn open_copy(original: &str, copy: &std::path::Path) -> DesignGraph {
    let copy = copy.to_str().unwrap();
    std::fs::copy(
        reflow2_core::identity::identity_path(original),
        reflow2_core::identity::identity_path(copy),
    )
    .expect("the original has an identity file");
    DesignGraph::open_rocksdb(copy).expect("a copy opens as a design")
}

fn holds(g: &DesignGraph, id: &str) -> bool {
    g.get_node("Requirement", id).unwrap().is_some()
}

#[test]
fn an_open_design_is_copied_as_of_the_moment_and_keeps_taking_writes() {
    let dir = tempfile::tempdir().unwrap();
    let (path, mut g) = design_at(dir.path());
    let copy = dir.path().join("copy");

    g.checkpoint_to(&copy).expect("an open design is copied");
    g.add_requirement(
        "req:shade",
        "The enclosure has shade",
        "Written after the copy.",
    )
    .expect("the design still takes writes");

    let c = open_copy(&path, &copy);
    assert!(
        holds(&c, "req:water"),
        "the copy holds what was written before it"
    );
    assert!(!holds(&c, "req:shade"), "and nothing written after it");
    assert!(
        holds(&g, "req:shade"),
        "the design itself kept the later write"
    );
}

#[test]
fn a_store_nothing_holds_is_copied_without_being_opened_as_a_design() {
    let dir = tempfile::tempdir().unwrap();
    let (path, g) = design_at(dir.path());
    drop(g);
    let copy = dir.path().join("copy");

    DesignGraph::checkpoint_closed_store(&path, &copy).expect("a closed store is copied");

    assert!(holds(&open_copy(&path, &copy), "req:water"));
}

#[test]
fn a_copy_refuses_a_place_that_exists_a_store_that_does_not_and_a_design_in_memory() {
    let dir = tempfile::tempdir().unwrap();
    let (path, g) = design_at(dir.path());
    let taken = dir.path().join("taken");
    std::fs::create_dir(&taken).unwrap();
    assert!(
        g.checkpoint_to(&taken).is_err(),
        "a copy never lands on top of something"
    );
    drop(g);

    let nowhere = dir.path().join("nowhere");
    let err =
        DesignGraph::checkpoint_closed_store(nowhere.to_str().unwrap(), &dir.path().join("c"))
            .expect_err("no store there");
    assert!(err.to_string().contains("no RocksDB store"), "{err}");
    assert!(!nowhere.exists(), "refusing creates nothing");

    let memory = DesignGraph::open_in_memory().unwrap();
    let err = memory
        .checkpoint_to(&dir.path().join("m"))
        .expect_err("an in-memory design keeps no files");
    assert!(err.to_string().contains("in memory"), "{err}");
    let _ = path;
}
