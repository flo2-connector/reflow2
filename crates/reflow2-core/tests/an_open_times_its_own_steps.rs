//! Opening a design on disk times its own steps
//! (`cap:an-open-reports-how-long-each-of-its-steps-took`).
//!
//! FOUND 2026-11-23 (fact:profile-2026-11-23-where-an-open-and-loop-status-spend-their-time-on-reflow2s-own-design):
//! the only way to say where an open of reflow2's own design spent its time was
//! to stop it under a debugger, and a sampled profile is not a split. Nothing
//! in the engine timed an open, so a hosted reopen could not be split at all.
//! These pin that every on-disk open now names each of its steps, in order, and
//! that the steps add up to the whole.

use std::time::Duration;

use reflow2_core::DesignGraph;

/// Every step an open of an existing store runs, in the order it runs them.
const STEPS: [&str; 12] = [
    "schema",
    "identity",
    "store",
    "identity_probe",
    "provenance",
    "identity_write",
    "stamps",
    "type_index",
    "authored_by_roles",
    "edge_rewrites",
    "stored_twins",
    "search_index",
];

fn a_store_with_a_design(dir: &std::path::Path) -> String {
    let path = dir.join("graph");
    let path = path.to_str().expect("utf-8 temp path").to_string();
    let mut g = DesignGraph::open_rocksdb(&path).expect("open");
    g.add_project("proj:zoo", "Zoo").expect("project");
    g.add_requirement(
        "req:water",
        "The enclosure has water",
        "A trough is filled daily.",
    )
    .expect("requirement");
    path
}

#[test]
fn reopening_a_store_names_every_step_in_order_and_they_add_up_to_the_whole() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = a_store_with_a_design(dir.path());

    let g = DesignGraph::open_rocksdb(&path).expect("reopen");
    let timing = g.opened_in().expect("an on-disk open keeps its timing");
    let names: Vec<&str> = timing.steps.iter().map(|s| s.name).collect();
    assert_eq!(names, STEPS, "the steps an open runs, in order");
    let sum: Duration = timing.steps.iter().map(|s| s.elapsed).sum();
    assert_eq!(
        sum, timing.total,
        "each step starts where the one before ended"
    );

    // The repairs say what they changed: a store written by this build holds
    // nothing for them to move.
    for repair in ["authored_by_roles", "edge_rewrites"] {
        let step = timing.steps.iter().find(|s| s.name == repair).unwrap();
        assert_eq!(
            step.changed,
            Some(0),
            "{repair} on a store this build wrote"
        );
    }
    let line = timing.summary();
    assert!(line.starts_with("schema="), "{line}");
    assert!(line.contains(" search_index="), "{line}");
}

#[test]
fn a_one_shot_open_is_timed_the_same_way() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = a_store_with_a_design(dir.path());

    let (g, _) = DesignGraph::open_rocksdb_once(&path).expect("open once");
    let names: Vec<&str> = g
        .opened_in()
        .expect("timed")
        .steps
        .iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, STEPS);
}

#[test]
fn a_graph_in_memory_was_never_opened_from_disk() {
    let g = DesignGraph::open_in_memory().expect("in memory");
    assert!(g.opened_in().is_none());
}
