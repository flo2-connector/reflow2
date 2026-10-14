//! Built work READS as built: a Component carries a computed build reading
//! beside its stored `status`, and the reading never moves the status.
//!
//! Measured on reflow2's own design 2026-10-13 (main c58305ba8,
//! `fact:root-cause-built-work-stays-open-because-its-record-is-kept-by-hand-and-the-ship-moves-only-the-capability-2026-10-13`):
//! 107 of 109 components read `planned`, and 88 of them have built work behind
//! them — a realized or verified capability allocated to them, or a realized
//! file or a passing check attached directly. `understated_status` reported the
//! lag in one roll-up and nothing else said it: a reader of `get_node` or
//! `scan_nodes` saw `planned` and nothing beside it.
//!
//! The class is a status kept by hand at a not-done default, whose truth the
//! engine already computes. So the truth is computed ONCE
//! ([`DesignGraph::component_build`], in `arrival.rs` beside the shared
//! "has this arrived" predicate), served beside the status, and read by the
//! detector too, so the reading and the finding cannot disagree.
//!
//! The stored `status` is the owner's record and stays exactly as written.

use reflow2_core::arrival::BuildState;
use reflow2_core::detect::GapCandidate;
use reflow2_core::graph::DesignGraph;
use reflow2_core::nodes::{Props, edge, node};

fn graph() -> DesignGraph {
    DesignGraph::open_in_memory().expect("open")
}

fn cap(g: &mut DesignGraph, id: &str, status: &str) {
    g.add_capability(id, id, "does the thing", Some(status))
        .expect("capability");
}

fn component(g: &mut DesignGraph, id: &str, status: Option<&str>) {
    let mut props = Props::new()
        .set("name", id)
        .set("purpose", "hosts the thing");
    if let Some(s) = status {
        props = props.set("status", s);
    }
    g.create_node(node::COMPONENT, id, props)
        .expect("component");
}

fn artifact(g: &mut DesignGraph, id: &str, status: Option<&str>) {
    let mut props = Props::new()
        .set("name", id)
        .set("location", format!("src/{id}.rs"));
    if let Some(s) = status {
        props = props.set("status", s);
    }
    g.create_node(node::ARTIFACT, id, props).expect("artifact");
}

fn check(g: &mut DesignGraph, id: &str, target_type: &str, target: &str, status: &str) {
    g.add_verification(id, id, Some("test"), Some("unit"), None)
        .expect("verification");
    g.verifies(id, target_type, target).expect("verifies");
    g.set_verification_status(id, status, None, None)
        .expect("verification status");
}

fn stored_status(g: &DesignGraph, id: &str) -> Option<String> {
    g.get_node(node::COMPONENT, id)
        .expect("read")
        .and_then(|n| {
            n.properties
                .get("status")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
}

fn understated_components(g: &DesignGraph) -> Vec<String> {
    let gaps: Vec<GapCandidate> = g
        .detect_gaps()
        .expect("detect")
        .into_iter()
        .filter(|x| x.gap_source.as_str() == "understated_status")
        .collect();
    let mut ids: Vec<String> = gaps
        .iter()
        .flat_map(|x| x.affected_ids.iter().cloned())
        .filter(|id| id.starts_with("cmp:"))
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

#[test]
fn a_component_whose_capabilities_are_all_realized_reads_built_and_its_status_is_unchanged() {
    let mut g = graph();
    component(&mut g, "cmp:engine", Some("planned"));
    cap(&mut g, "cap:a", "realized");
    cap(&mut g, "cap:b", "verified");
    g.allocate("cap:a", "cmp:engine").expect("allocate");
    g.allocate("cap:b", "cmp:engine").expect("allocate");

    let build = g.component_build("cmp:engine").expect("reading");
    assert_eq!(build.state(), BuildState::Built, "{build:?}");
    assert_eq!(build.capabilities_allocated, ["cap:a", "cap:b"]);
    assert_eq!(build.capabilities_built, ["cap:a", "cap:b"]);

    let reading = build.reading();
    assert_eq!(reading.reads_as, BuildState::Built);
    assert_eq!(reading.capabilities_allocated, 2);
    assert_eq!(reading.capabilities_built, 2);
    assert_eq!(reading.files_built, 0);
    assert_eq!(reading.checks_passing, 0);
    let wire = serde_json::to_value(reading).expect("serialises");
    assert_eq!(wire["reads_as"], "built", "snake_case on the wire: {wire}");

    assert_eq!(
        stored_status(&g, "cmp:engine").as_deref(),
        Some("planned"),
        "the reading is computed beside the stored status and never writes it"
    );
}

#[test]
fn a_component_with_no_built_evidence_reads_not_built() {
    let mut g = graph();
    component(&mut g, "cmp:idle", None);
    cap(&mut g, "cap:planned", "planned");
    cap(&mut g, "cap:going", "in_progress");
    g.allocate("cap:planned", "cmp:idle").expect("allocate");
    g.allocate("cap:going", "cmp:idle").expect("allocate");
    // A file nobody has said exists, and a check that does not pass, are not
    // evidence of a build — the same narrow reading `understated_status` keeps.
    artifact(&mut g, "art:stub", Some("planned"));
    g.realizes("art:stub", node::COMPONENT, "cmp:idle", None, None)
        .expect("realizes");
    check(&mut g, "ver:red", node::COMPONENT, "cmp:idle", "failing");

    let before = stored_status(&g, "cmp:idle");
    let build = g.component_build("cmp:idle").expect("reading");
    assert_eq!(build.state(), BuildState::NotBuilt, "{build:?}");
    assert_eq!(build.capabilities_allocated.len(), 2);
    assert!(build.capabilities_built.is_empty());
    assert!(build.files_built.is_empty());
    assert!(build.checks_passing.is_empty());
    assert_eq!(
        stored_status(&g, "cmp:idle"),
        before,
        "the status is as it was"
    );

    // And a component with nothing at all.
    component(&mut g, "cmp:empty", Some("planned"));
    assert_eq!(
        g.component_build("cmp:empty").expect("reading").state(),
        BuildState::NotBuilt
    );
}

#[test]
fn a_component_with_some_of_its_function_built_reads_partly_built() {
    let mut g = graph();
    component(&mut g, "cmp:half", Some("planned"));
    cap(&mut g, "cap:done", "realized");
    cap(&mut g, "cap:todo", "planned");
    g.allocate("cap:done", "cmp:half").expect("allocate");
    g.allocate("cap:todo", "cmp:half").expect("allocate");
    let build = g.component_build("cmp:half").expect("reading");
    assert_eq!(build.state(), BuildState::PartlyBuilt, "{build:?}");
    assert_eq!(build.reading().capabilities_built, 1);
    assert_eq!(build.reading().capabilities_allocated, 2);

    // A realized file on the part itself, while a function it hosts is still
    // unbuilt, is part of it built — not all of it.
    component(&mut g, "cmp:file", Some("planned"));
    cap(&mut g, "cap:unbuilt", "planned");
    g.allocate("cap:unbuilt", "cmp:file").expect("allocate");
    artifact(&mut g, "art:main", Some("realized"));
    g.realizes("art:main", node::COMPONENT, "cmp:file", None, None)
        .expect("realizes");
    let build = g.component_build("cmp:file").expect("reading");
    assert_eq!(build.state(), BuildState::PartlyBuilt, "{build:?}");
    assert_eq!(build.files_built, ["art:main"]);

    // A part that hosts no function at all and has a passing check of its own
    // is built: nothing it hosts is waiting.
    component(&mut g, "cmp:checked", Some("planned"));
    check(
        &mut g,
        "ver:green",
        node::COMPONENT,
        "cmp:checked",
        "passing",
    );
    let build = g.component_build("cmp:checked").expect("reading");
    assert_eq!(build.state(), BuildState::Built, "{build:?}");
    assert_eq!(build.checks_passing, ["ver:green"]);
}

#[test]
fn a_withdrawn_capability_is_neither_built_nor_waiting() {
    let mut g = graph();
    component(&mut g, "cmp:x", Some("planned"));
    cap(&mut g, "cap:kept", "realized");
    cap(&mut g, "cap:withdrawn", "planned");
    g.allocate("cap:kept", "cmp:x").expect("allocate");
    g.allocate("cap:withdrawn", "cmp:x").expect("allocate");
    g.create_node(
        node::DECISION,
        "dec:drop-it",
        Props::new()
            .set("name", "drop it")
            .set("decision", "withdraw cap:withdrawn")
            .set("status", "accepted"),
    )
    .expect("decision");
    g.create_edge(
        edge::OBSOLETES,
        node::DECISION,
        "dec:drop-it",
        node::CAPABILITY,
        "cap:withdrawn",
        Props::new(),
    )
    .expect("obsoletes");
    let build = g.component_build("cmp:x").expect("reading");
    assert_eq!(
        build.capabilities_allocated,
        ["cap:kept"],
        "a capability an accepted decision withdrew is not the part's function any more"
    );
    assert_eq!(build.state(), BuildState::Built, "{build:?}");
}

#[test]
fn understated_status_agrees_with_the_reading() {
    let mut g = graph();
    // Built, partly built, not built — all still `planned`.
    component(&mut g, "cmp:built", Some("planned"));
    cap(&mut g, "cap:1", "realized");
    g.allocate("cap:1", "cmp:built").expect("allocate");

    component(&mut g, "cmp:partly", None);
    cap(&mut g, "cap:2", "verified");
    cap(&mut g, "cap:3", "planned");
    g.allocate("cap:2", "cmp:partly").expect("allocate");
    g.allocate("cap:3", "cmp:partly").expect("allocate");

    component(&mut g, "cmp:direct", Some("planned"));
    artifact(&mut g, "art:direct", Some("verified"));
    g.realizes("art:direct", node::COMPONENT, "cmp:direct", None, None)
        .expect("realizes");

    component(&mut g, "cmp:none", Some("planned"));
    cap(&mut g, "cap:4", "planned");
    g.allocate("cap:4", "cmp:none").expect("allocate");

    // Built by evidence, and its status already says so: not understated.
    component(&mut g, "cmp:moved", Some("realized"));
    cap(&mut g, "cap:5", "realized");
    g.allocate("cap:5", "cmp:moved").expect("allocate");

    let mut read_as_built: Vec<String> = Vec::new();
    for id in [
        "cmp:built",
        "cmp:partly",
        "cmp:direct",
        "cmp:none",
        "cmp:moved",
    ] {
        let state = g.component_build(id).expect("reading").state();
        let planned = matches!(stored_status(&g, id).as_deref(), None | Some("planned"));
        if planned && state != BuildState::NotBuilt {
            read_as_built.push(id.to_string());
        }
    }
    read_as_built.sort();
    assert_eq!(
        read_as_built,
        ["cmp:built", "cmp:direct", "cmp:partly"],
        "fixture check"
    );
    assert_eq!(
        understated_components(&g),
        read_as_built,
        "the understated finding names exactly the planned components whose reading is not \
         `not_built` — one computation, read by both"
    );
    assert_eq!(
        g.component_build("cmp:moved").expect("reading").state(),
        BuildState::Built,
        "a moved status changes nothing about the reading: it is evidence, not the record"
    );
}
