//! A write unit is all or nothing, however it is nested — the core half of
//! `dec:idea-a-refused-typed-write-stores-nothing`.
//!
//! The served surface stages every write a tool call makes in ONE unit and
//! commits it when the handler answers, or discards it when the handler
//! refuses (`reflow2-mcp`'s `a_refused_write_stores_nothing.rs` asks that of
//! every write tool). The unit is the store's existing atomic batch — the one
//! `import_graph`, the bulk forms and HEAL already ride
//! (`dec:bulk-is-all-or-nothing-with-per-item-findings`) — so these pin the
//! three things a CALL-sized batch needs that a bulk-sized one never did:
//!
//! 1. **A batch opened inside a batch NESTS.** A tool call's unit is open when
//!    its handler reaches a bulk form or an import, which opens its own. The
//!    store used to answer a second `begin_batch` by COMMITTING the first, so
//!    the outer unit's staged writes became durable before anyone had decided
//!    whether the call succeeded. An inner commit must now hand its writes to
//!    the outer unit, and an inner discard must drop only its own.
//! 2. **A read inside a unit sees the unit's own writes, derived scans too.**
//!    The memoised scans (`detect_gaps`, the defect scan, the node-type index,
//!    the adjacency) are keyed on the store's write generation, which only a
//!    COMMITTED write moved. A staged write left a scan computed before it
//!    standing, so a handler that wrote and then asked the design a question
//!    was answered about the design before its own write.
//! 3. **A discarded unit leaves the store exactly as it found it** — node for
//!    node and edge for edge, and the derived scans with it.

use reflow2_core::DesignGraph;
use reflow2_core::Value;
use reflow2_core::bulk::NodeSpec;

fn stored(g: &DesignGraph) -> (serde_json::Value, serde_json::Value) {
    let export = serde_json::to_value(g.export_graph().expect("export")).expect("json");
    (export["nodes"].clone(), export["edges"].clone())
}

fn spec(node_type: &str, id: &str, props: &[(&str, &str)]) -> NodeSpec {
    NodeSpec {
        node_type: node_type.to_string(),
        id: id.to_string(),
        props: props
            .iter()
            .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
            .collect(),
    }
}

fn holds(g: &DesignGraph, ty: &str, id: &str) -> bool {
    g.get_node(ty, id).expect("read").is_some()
}

/// A design with something in it, so "unchanged" is a claim about content.
fn seeded() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_requirement("req:held", "Held", "The design held this before.")
        .expect("seed");
    g
}

// ---- 1. nesting, through the public bulk forms alone --------------------

/// FAILING FIRST ON MAIN. An outer batch writes a node, an inner bulk form
/// opens its own batch and is refused, and the outer one then fails too. The
/// outer write must not survive: nothing was ever committed. The store used to
/// commit the outer batch the moment the inner one began.
#[test]
fn a_batch_inside_a_batch_does_not_commit_the_outer_one() {
    let mut g = seeded();
    let before = stored(&g);
    let report = g
        .atomically(
            &[()],
            |_| "outer".to_string(),
            |g, _| -> Result<(), String> {
                g.add_requirement("req:outer", "Outer", "Written by the outer batch.")
                    .map_err(|e| e.to_string())?;
                // The inner bulk form fails on its one bad item and discards
                // ITS batch — which must leave the outer write staged, not
                // committed and not dropped.
                let inner = g
                    .create_nodes(&[spec("NoSuchType", "x:bad", &[("name", "bad")])])
                    .map_err(|e| e.to_string())?;
                assert!(!inner.applied, "the inner bulk form must refuse");
                assert!(
                    holds(g, "Requirement", "req:outer"),
                    "an inner discard dropped the OUTER batch's staged write"
                );
                Err("the outer item fails after the inner one".to_string())
            },
            false,
        )
        .expect("the bulk call itself");
    assert!(!report.applied);
    assert!(
        !holds(&g, "Requirement", "req:outer"),
        "the outer batch was discarded, so its write must not exist — it was committed early, \
         when the inner batch began"
    );
    assert_eq!(
        stored(&g),
        before,
        "a discarded outer batch changes nothing"
    );
}

/// An inner batch that SUCCEEDS hands its writes to the outer one: they are
/// visible inside it, and they go when the outer batch is discarded.
#[test]
fn an_inner_commit_belongs_to_the_outer_batch() {
    let mut g = seeded();
    let before = stored(&g);
    let report = g
        .atomically(
            &[()],
            |_| "outer".to_string(),
            |g, _| -> Result<(), String> {
                let inner = g
                    .create_nodes(&[spec(
                        "Requirement",
                        "req:inner",
                        &[
                            ("name", "Inner"),
                            ("statement", "Written by the inner batch."),
                        ],
                    )])
                    .map_err(|e| e.to_string())?;
                assert!(inner.applied, "the inner bulk form succeeds");
                assert!(
                    holds(g, "Requirement", "req:inner"),
                    "an inner commit is visible to the outer batch"
                );
                Err("and then the outer batch is refused".to_string())
            },
            false,
        )
        .expect("the bulk call itself");
    assert!(!report.applied);
    assert!(
        !holds(&g, "Requirement", "req:inner"),
        "the inner batch's writes belong to the outer one and are discarded with it"
    );
    assert_eq!(stored(&g), before);
}

/// `check_only` inside a batch validates and writes nothing — the inner
/// discard is by request — and the outer batch's own write still lands.
#[test]
fn a_check_inside_a_batch_writes_nothing_and_keeps_the_outer_write() {
    let mut g = seeded();
    let report = g
        .atomically(
            &[()],
            |_| "outer".to_string(),
            |g, _| -> Result<(), String> {
                g.add_requirement("req:kept", "Kept", "The outer batch keeps this.")
                    .map_err(|e| e.to_string())?;
                let checked = g
                    .create_nodes_with(
                        &[spec(
                            "Requirement",
                            "req:only-checked",
                            &[
                                ("name", "Checked"),
                                ("statement", "Validated, never written."),
                            ],
                        )],
                        true,
                    )
                    .map_err(|e| e.to_string())?;
                assert!(checked.check_only && checked.failures.is_empty());
                Ok(())
            },
            false,
        )
        .expect("the bulk call itself");
    assert!(report.applied);
    assert!(holds(&g, "Requirement", "req:kept"));
    assert!(
        !holds(&g, "Requirement", "req:only-checked"),
        "a check writes nothing, nested or not"
    );
}

// ---- 2. a derived scan sees the batch's own writes ----------------------

/// FAILING FIRST ON MAIN. Inside a batch, a memoised scan taken before a write
/// must not answer for the design after it. `detect_gaps` is memoised on the
/// write generation; a staged write did not move it, so the second call
/// returned the first call's gaps, missing the new capability entirely.
///
/// The control, outside any batch, is the same two calls around the same
/// write — measured while writing this: there the second call names
/// `cap:fresh` ("Nothing asked for capability"), so the test discriminates.
#[test]
fn a_derived_scan_inside_a_batch_sees_the_batchs_own_writes() {
    fn named(g: &DesignGraph) -> String {
        g.detect_gaps()
            .expect("gaps")
            .iter()
            .flat_map(|gap| gap.affected_ids.clone())
            .collect::<Vec<_>>()
            .join(" ")
    }

    // The control: no batch.
    let mut control = seeded();
    assert!(!named(&control).contains("cap:fresh"));
    control
        .add_capability("cap:fresh", "Fresh", "Nothing asks for this yet.", None)
        .expect("write");
    assert!(
        named(&control).contains("cap:fresh"),
        "the control must show the gap, or this test measures nothing"
    );

    // The same, inside a batch.
    let mut g = seeded();
    let report = g
        .atomically(
            &[()],
            |_| "outer".to_string(),
            |g, _| -> Result<(), String> {
                assert!(!named(g).contains("cap:fresh"));
                g.add_capability("cap:fresh", "Fresh", "Nothing asks for this yet.", None)
                    .map_err(|e| e.to_string())?;
                let after = named(g);
                if after.contains("cap:fresh") {
                    Ok(())
                } else {
                    Err(format!(
                        "detect_gaps inside the batch answered from before the batch's own \
                         write — cap:fresh is asked for by nothing and absent from: {after}"
                    ))
                }
            },
            false,
        )
        .expect("the bulk call itself");
    assert!(
        report.failures.is_empty(),
        "{:?}",
        report.failures.iter().map(|f| &f.error).collect::<Vec<_>>()
    );
}

// ---- 3. the unit API the served surface uses -----------------------------

/// Everything a unit stages — nodes, a revise, edges, and a bulk form inside
/// it — is gone when the unit is discarded, and the store reads exactly as it
/// did before the unit began.
#[test]
fn a_discarded_unit_leaves_the_store_exactly_as_it_found_it() {
    let mut g = seeded();
    g.add_capability("cap:held", "Held", "described before the unit", None)
        .expect("seed");
    let before = stored(&g);
    let gaps_before = g.detect_gaps().expect("gaps").len();

    g.begin_unit();
    assert!(g.in_unit());
    g.add_capability("cap:held", "Held", "rewritten inside the unit", None)
        .expect("revise");
    g.add_requirement("req:staged", "Staged", "Written inside the unit.")
        .expect("create");
    g.satisfies("cap:held", "req:staged").expect("edge");
    let bulk = g
        .create_nodes(&[spec(
            "Requirement",
            "req:bulk",
            &[
                ("name", "Bulk"),
                ("statement", "Written by a bulk form inside the unit."),
            ],
        )])
        .expect("bulk");
    assert!(bulk.applied);
    // Reads inside the unit see all of it.
    assert!(holds(&g, "Requirement", "req:staged") && holds(&g, "Requirement", "req:bulk"));
    assert_eq!(
        g.get_node("Capability", "cap:held")
            .unwrap()
            .unwrap()
            .properties["description"],
        Value::String("rewritten inside the unit".into())
    );
    assert_eq!(g.outgoing("cap:held", Some("SATISFIES")).unwrap().len(), 1);
    g.discard_unit();

    assert!(!g.in_unit());
    assert_eq!(stored(&g), before, "a discarded unit changes nothing");
    assert_eq!(
        g.detect_gaps().expect("gaps").len(),
        gaps_before,
        "and the derived scans answer for the store as it is, not as the unit left it"
    );
}

/// A committed unit lands whole.
#[test]
fn a_committed_unit_lands_whole() {
    let mut g = seeded();
    g.begin_unit();
    g.add_capability("cap:landed", "Landed", "written in a unit", None)
        .expect("create");
    g.satisfies("cap:landed", "req:held").expect("edge");
    let wrote = g.commit_unit().expect("commit");
    assert!(wrote > 0, "the unit reports what it wrote");
    assert!(!g.in_unit());
    assert!(holds(&g, "Capability", "cap:landed"));
    assert_eq!(
        g.outgoing("cap:landed", Some("SATISFIES")).unwrap().len(),
        1
    );
}

/// A unit that wrote nothing commits nothing, and says so.
#[test]
fn an_empty_unit_commits_nothing() {
    let mut g = seeded();
    let before = stored(&g);
    g.begin_unit();
    assert_eq!(g.commit_unit().expect("commit"), 0);
    assert_eq!(stored(&g), before);
}
