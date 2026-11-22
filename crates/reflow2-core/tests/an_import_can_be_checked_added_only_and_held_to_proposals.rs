//! THREE THINGS AN IMPORT CAN BE ASKED TO BE (`ImportOptions`):
//!
//! - `check_only`: the whole import runs and is then DISCARDED. Nothing is
//!   written, and what it found is a report, faults included.
//! - `add_only`: a record or relation the design already holds is refused.
//! - `proposals_only`: a record that would arrive settled, a held record whose
//!   settling property would move at all, and a signature that approves are
//!   refused.
//!
//! All three are off unless asked for, and they combine with each other and
//! with `merge`. What "settled" means is read off the one table,
//! `reflow2_core::intent::SETTLING`; every test here that names a settling
//! value walks that table, so a row added to it is tested the day it lands.
//!
//! The store's own identity on an EMPTY on-disk store needs a store in a
//! folder: `reflow2-mcp/tests/a_checked_import_writes_nothing.rs`.

use reflow2_core::foundation::core::PropertyType;
use reflow2_core::foundation::core::Value;
use reflow2_core::intent::{SETTLING, SettlingProperty, Signer, more_than_a_proposal};
use reflow2_core::nodes::{Props, edge, node};
use reflow2_core::{DesignGraph, GraphExport, ImportOptions, ImportReport};
use serde_json::json;
use std::collections::HashMap;

const ME: &str = "who:alice";
const OTHER: &str = "who:mallory";

const CHECK: ImportOptions = ImportOptions {
    accept_newer: false,
    merge: false,
    check_only: true,
    add_only: false,
    proposals_only: false,
};
const ADD: ImportOptions = ImportOptions {
    accept_newer: false,
    merge: false,
    check_only: false,
    add_only: true,
    proposals_only: false,
};
const PROPOSE: ImportOptions = ImportOptions {
    accept_newer: false,
    merge: false,
    check_only: false,
    add_only: false,
    proposals_only: true,
};

fn doc(v: serde_json::Value) -> GraphExport {
    serde_json::from_value(v).expect("a document")
}

/// A small design: two contributors, two requirements, one relation between
/// them, one decision approved by the other contributor.
fn design() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("in-memory design");
    g.add_contributor(ME, "Alice", Some("person"), None, None)
        .unwrap();
    g.add_contributor(OTHER, "Mallory", Some("person"), None, None)
        .unwrap();
    g.add_requirement("req:a", "A", "The first.").unwrap();
    g.add_requirement("req:b", "B", "The second.").unwrap();
    g.create_edge(
        edge::DEPENDS_ON,
        node::REQUIREMENT,
        "req:a",
        node::REQUIREMENT,
        "req:b",
        Props::new(),
    )
    .unwrap();
    g.add_decision("dec:e", "Choose", "Use CBOR.", None)
        .unwrap();
    g.authored_by(node::DECISION, "dec:e", OTHER, Some("approver"), None)
        .unwrap();
    g
}

/// Everything a design holds, in the form that shows any change: every node
/// with its properties and its write stamps, every edge, the id, the hash.
fn everything(g: &DesignGraph) -> serde_json::Value {
    let export = g.export_graph().expect("export");
    json!({
        "graph_id": export.graph_id,
        "content_hash": export.content_hash,
        "nodes": serde_json::to_value(&export.nodes).unwrap(),
        "edges": serde_json::to_value(&export.edges).unwrap(),
    })
}

/// The faults a REFUSED real import lists, read back out of its error.
fn listed(err: &reflow2_core::DynoError) -> Vec<String> {
    let text = err.to_string();
    assert!(
        text.contains("NOTHING was written"),
        "a refused import says so: {text}"
    );
    text.split("\n  - ").skip(1).map(str::to_string).collect()
}

/// A valid document that adds two records and two relations to [`design`].
fn two_new_records() -> GraphExport {
    doc(json!({
        "nodes": [
            {"node_type": "Requirement", "node_id": "req:new", "properties": {"name": "New", "statement": "A new one."}},
            {"node_type": "Decision", "node_id": "dec:new", "properties": {"name": "New", "decision": "Maybe JSON."}}
        ],
        "edges": [
            {"edge_type": "DEPENDS_ON", "from_id": "req:new", "to_id": "req:a", "properties": {}},
            {"edge_type": "DEPENDS_ON", "from_id": "req:b", "to_id": "req:a", "properties": {}}
        ]
    }))
}

// ---- check_only -----------------------------------------------------------

#[test]
fn a_check_of_a_valid_document_writes_nothing_and_the_same_import_then_matches_it() {
    let mut g = design();
    let before = everything(&g);
    let nodes_before = g.export_graph().unwrap().nodes.len();
    let document = two_new_records();

    let checked = g.import_graph_with(&document, CHECK).expect("a check");
    assert!(checked.checked_only, "{checked:?}");
    assert!(checked.faults.is_empty(), "{checked:?}");
    assert_eq!(
        (checked.nodes_written, checked.edges_written),
        (0, 0),
        "a check wrote nothing and says so"
    );
    let would = checked.would_write.expect("what it would write");
    assert_eq!((would.nodes, would.edges), (2, 2));

    assert_eq!(everything(&g), before, "a check changes nothing");
    assert_eq!(g.export_graph().unwrap().nodes.len(), nodes_before);
    assert!(g.get_node(node::REQUIREMENT, "req:new").unwrap().is_none());

    // The check said "would import"; the same import does, with its counts.
    let written = g.import_graph(&document).expect("the same import");
    assert!(!written.checked_only);
    assert_eq!(written.would_write, None);
    assert_eq!(
        (written.nodes_written, written.edges_written),
        (would.nodes, would.edges)
    );
    assert_eq!(
        g.export_graph().unwrap().nodes.len(),
        nodes_before + would.nodes
    );
}

#[test]
fn a_check_of_an_invalid_document_lists_every_fault_and_writes_nothing() {
    let mut g = design();
    let before = everything(&g);
    let document = doc(json!({
        "nodes": [
            {"node_type": "Requirement", "node_id": "req:fine", "properties": {"name": "Fine", "statement": "Valid."}},
            {"node_type": "Requirement", "node_id": "req:bad-status", "properties": {"name": "Bad", "statement": "S.", "status": "no-such-status"}},
            {"node_type": "NoSuchType", "node_id": "x:1", "properties": {"name": "X"}},
            {"node_type": "Decision", "node_id": "dec:no-body", "properties": {"name": "No body"}}
        ],
        "edges": [
            {"edge_type": "NO_SUCH_EDGE", "from_id": "req:fine", "to_id": "req:a", "properties": {}}
        ]
    }));

    let checked = g
        .import_graph_with(&document, CHECK)
        .expect("a check with faults is an answer, not an error");
    assert!(checked.checked_only);
    assert_eq!(
        checked.would_write.map(|w| (w.nodes, w.edges)),
        Some((0, 0)),
        "an import is all-or-nothing, so a refused one would write nothing"
    );
    assert_eq!(everything(&g), before, "a refused check changes nothing");

    // The same strings, in the same order, a refused real import lists.
    let refused = g.import_graph(&document).expect_err("the real import");
    assert_eq!(checked.faults, listed(&refused));
    assert_eq!(checked.faults.len(), 4, "{:#?}", checked.faults);
    assert_eq!(everything(&g), before);
}

#[test]
fn a_check_under_merge_reports_what_would_be_removed_and_removes_nothing() {
    let mut g = design();
    let before = everything(&g);
    // States the requirement whole, without its statement's sibling fields:
    // `status` comes back as the default, the rest is what the document says.
    g.create_node(
        node::REQUIREMENT,
        "req:rich",
        HashMap::from([
            ("name".to_string(), Value::String("Rich".into())),
            ("statement".to_string(), Value::String("S.".into())),
            ("rationale".to_string(), Value::String("Because.".into())),
        ]),
    )
    .unwrap();
    let with_rich = everything(&g);
    assert_ne!(with_rich, before);
    let document = doc(json!({
        "nodes": [{"node_type": "Requirement", "node_id": "req:rich", "properties": {"name": "Rich", "statement": "S."}}],
        "edges": []
    }));

    let whole = g.import_graph_with(&document, CHECK).expect("a check");
    assert_eq!(
        whole.properties_removed.get("Requirement.rationale"),
        Some(&1),
        "{whole:?}"
    );
    let note = whole.removed_note.expect("the removal is said");
    assert!(
        note.contains("WOULD REMOVE") && note.contains("nothing was removed"),
        "a check never says it removed anything: {note}"
    );
    assert_eq!(everything(&g), with_rich);

    let merged = g
        .import_graph_with(
            &document,
            ImportOptions {
                merge: true,
                ..CHECK
            },
        )
        .expect("a check under merge");
    assert!(merged.properties_removed.is_empty(), "{merged:?}");
    assert_eq!(merged.removed_note, None);
    assert_eq!(everything(&g), with_rich);
}

#[test]
fn a_check_of_a_document_from_a_newer_reflow2_answers_with_that_refusal() {
    let mut g = design();
    let before = everything(&g);
    let mut document = g.export_graph().unwrap();
    let mut stamp = document.stamp.clone().expect("an export is stamped");
    stamp.reflow2_version = "99.0.0".into();
    document.stamp = Some(stamp);

    let checked = g.import_graph_with(&document, CHECK).expect("a check");
    assert!(checked.checked_only);
    assert_eq!(checked.faults.len(), 1, "{:#?}", checked.faults);
    let refused = g.import_graph(&document).expect_err("the real import");
    assert_eq!(checked.faults[0], refused.to_string());
    assert!(checked.faults[0].contains("BEHIND"));
    assert_eq!(everything(&g), before);
}

#[test]
fn a_check_credits_nobody_with_the_nodes_it_walked() {
    // A served write credits whoever the call writes for with every node it
    // touched, read off this log when the hold is released. A check touched
    // none, so the log is left exactly as long as it found it.
    let mut g = design();
    g.begin_touch_log();
    g.add_requirement("req:c", "C", "Written before the check.")
        .unwrap();
    let mut document = g.export_graph().unwrap();
    document.content_hash = None;
    g.import_graph_with(&document, CHECK).expect("a check");
    let log = g.take_touch_log();
    assert_eq!(
        log,
        vec![(node::REQUIREMENT.to_string(), "req:c".to_string())],
        "only the write made before the check is credited"
    );
}

// ---- add_only -------------------------------------------------------------

#[test]
fn add_only_refuses_what_the_design_holds_and_writes_what_it_does_not() {
    let mut g = design();
    let before = everything(&g);
    let held = doc(json!({
        "nodes": [
            {"node_type": "Requirement", "node_id": "req:new", "properties": {"name": "New", "statement": "A new one."}},
            // Held, same type.
            {"node_type": "Requirement", "node_id": "req:a", "properties": {"name": "A, renamed", "statement": "Changed."}},
            // Held under ANOTHER type: the id is taken all the same.
            {"node_type": "Decision", "node_id": "req:b", "properties": {"name": "B", "decision": "D."}}
        ],
        "edges": [
            // Held.
            {"edge_type": "DEPENDS_ON", "from_id": "req:a", "to_id": "req:b", "properties": {}},
            // New, between two held nodes, the other way round.
            {"edge_type": "DEPENDS_ON", "from_id": "req:b", "to_id": "req:a", "properties": {}}
        ]
    }));

    let refused = g
        .import_graph_with(&held, ADD)
        .expect_err("add-only refuses a held record");
    assert_eq!(
        listed(&refused),
        vec![
            "nodes[1] req:a: the design already holds this record, and an add-only import \
             never changes one it holds"
                .to_string(),
            "nodes[2] req:b: the design already holds this record, and an add-only import \
             never changes one it holds"
                .to_string(),
            "edges[0] DEPENDS_ON req:a -> req:b: the design already holds this relation, and \
             an add-only import never changes one it holds"
                .to_string(),
        ]
    );
    assert_eq!(everything(&g), before, "all or nothing");

    // A check says the same, as a report.
    let checked = g
        .import_graph_with(
            &held,
            ImportOptions {
                check_only: true,
                ..ADD
            },
        )
        .expect("a check");
    assert_eq!(checked.faults, listed(&refused));
    assert_eq!(everything(&g), before);

    // Without the option the same document is the upsert it always was.
    let mut plain = design();
    plain
        .import_graph(&doc(json!({
            "nodes": [{"node_type": "Requirement", "node_id": "req:a", "properties": {"name": "A, renamed", "statement": "Changed."}}],
            "edges": [{"edge_type": "DEPENDS_ON", "from_id": "req:a", "to_id": "req:b", "properties": {}}]
        })))
        .expect("an ordinary import replaces a held node");

    // New records, a new relation between two held nodes, and one between a
    // held node and a new one: all written.
    let report = g
        .import_graph_with(&two_new_records(), ADD)
        .expect("add-only writes what is new");
    assert_eq!((report.nodes_written, report.edges_written), (2, 2));
    assert!(g.get_node(node::REQUIREMENT, "req:new").unwrap().is_some());
    assert!(
        g.outgoing("req:b", Some(edge::DEPENDS_ON))
            .unwrap()
            .iter()
            .any(|e| e.to_id == "req:a")
    );
    // ...and the same document a second time is now all held.
    let again = g
        .import_graph_with(&two_new_records(), ADD)
        .expect_err("held now");
    assert_eq!(listed(&again).len(), 4, "{again}");
}

#[test]
fn add_only_lists_its_faults_beside_every_other_fault() {
    let mut g = design();
    let before = everything(&g);
    let refused = g
        .import_graph_with(
            &doc(json!({
                "nodes": [
                    {"node_type": "Requirement", "node_id": "req:a", "properties": {"name": "A", "statement": "S."}},
                    {"node_type": "Requirement", "node_id": "req:bad", "properties": {"name": "Bad", "statement": "S.", "status": "no-such-status"}}
                ],
                "edges": []
            })),
            ADD,
        )
        .expect_err("two faults");
    let faults = listed(&refused);
    assert_eq!(faults.len(), 2, "{faults:#?}");
    assert!(faults[0].starts_with("nodes[0] req:a: the design already holds this record"));
    assert!(faults[1].starts_with("nodes[1] req:bad: "));
    assert_eq!(everything(&g), before);
}

#[test]
fn add_only_means_held_before_the_document_not_named_twice_in_it() {
    let mut g = design();
    let report = g
        .import_graph_with(
            &doc(json!({
                "nodes": [
                    {"node_type": "Requirement", "node_id": "req:twice", "properties": {"name": "Once", "statement": "S."}},
                    {"node_type": "Requirement", "node_id": "req:twice", "properties": {"name": "Twice", "statement": "S."}}
                ],
                "edges": [
                    {"edge_type": "DEPENDS_ON", "from_id": "req:twice", "to_id": "req:a", "properties": {}},
                    {"edge_type": "DEPENDS_ON", "from_id": "req:twice", "to_id": "req:a", "properties": {}}
                ]
            })),
            ADD,
        )
        .expect("the design held neither before this document");
    assert_eq!(report.nodes_written, 2);
}

// ---- proposals_only, over every row of the one table -----------------------

/// The least a node of this type must state, read off the schema, WITHOUT the
/// row's settling property.
fn least(g: &DesignGraph, row: &SettlingProperty) -> serde_json::Map<String, serde_json::Value> {
    let def = &g.schema().node_types[row.node_type];
    let mut props = serde_json::Map::new();
    for (name, p) in &def.properties {
        if !p.required || name == row.property {
            continue;
        }
        let value = match p.prop_type {
            PropertyType::String | PropertyType::Datetime => json!("stated by the test"),
            PropertyType::Int => json!(0),
            PropertyType::Float => json!(0.0),
            PropertyType::Bool => json!(false),
            PropertyType::Enum => json!(p.values.as_ref().expect("an enum has values")[0]),
            PropertyType::ListString => json!([]),
            ref other => panic!(
                "{}.{name} is a {other:?}: teach this test a value",
                row.node_type
            ),
        };
        props.insert(name.clone(), value);
    }
    props
}

/// Every state a document can put the row's settling property in: each value
/// the schema allows, and (`None`) not stating it at all.
fn states(g: &DesignGraph, row: &SettlingProperty) -> Vec<Option<serde_json::Value>> {
    let p = &g.schema().node_types[row.node_type].properties[row.property];
    let mut states: Vec<Option<serde_json::Value>> = match p.prop_type {
        PropertyType::Enum => p
            .values
            .as_ref()
            .expect("an enum has values")
            .iter()
            .map(|v| Some(json!(v)))
            .collect(),
        PropertyType::Bool => vec![Some(json!(true)), Some(json!(false))],
        ref other => panic!(
            "{}.{} is a {other:?}: teach this test its values",
            row.node_type, row.property
        ),
    };
    states.push(None);
    states
}

fn one_node(
    g: &DesignGraph,
    row: &SettlingProperty,
    id: &str,
    state: &Option<serde_json::Value>,
) -> GraphExport {
    let mut props = least(g, row);
    if let Some(v) = state {
        props.insert(row.property.to_string(), v.clone());
    }
    doc(json!({
        "nodes": [{"node_type": row.node_type, "node_id": id, "properties": props}],
        "edges": []
    }))
}

/// The row's settling property as the design holds it (`None`: not held).
fn held(g: &DesignGraph, row: &SettlingProperty, id: &str) -> Option<Value> {
    g.get_node(row.node_type, id)
        .unwrap()
        .expect("the node")
        .properties
        .get(row.property)
        .filter(|v| !v.is_null())
        .cloned()
}

fn said(v: &Option<Value>) -> String {
    match v {
        None => "not stated".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(other) => format!("{other:?}"),
    }
}

#[test]
fn the_table_is_what_this_file_walks() {
    assert!(
        SETTLING.len() >= 4,
        "the table lost a row: {SETTLING:?} — every test below iterates it"
    );
}

#[test]
fn proposals_only_refuses_a_new_record_that_would_arrive_settled_for_every_row() {
    let mut refused_some = 0;
    for row in SETTLING {
        let probe = DesignGraph::open_in_memory().unwrap();
        for state in states(&probe, row) {
            let what = format!("{} with {} {state:?}", row.node_type, row.property);
            let document = one_node(&probe, row, "x:new", &state);

            // What an ordinary import stores is what the option is judged on,
            // schema defaults included.
            let mut plain = DesignGraph::open_in_memory().unwrap();
            plain.import_graph(&document).expect(&what);
            let stored = held(&plain, row, "x:new");
            let settles = row.when.settles(stored.as_ref());
            if state.is_none() {
                // A DEFAULT THAT SETTLED would settle a record whose document
                // never stated it. No row has one today; were one added, the
                // record would be refused like any other that arrives settled
                // (asserted below), and this line says it happened.
                assert!(
                    !settles,
                    "{what}: the schema's default for {}.{} now SETTLES ({stored:?}), so a \
                     proposals-only import refuses every {} that does not state it",
                    row.node_type, row.property, row.node_type
                );
            }

            let mut g = DesignGraph::open_in_memory().unwrap();
            let outcome = g.import_graph_with(&document, PROPOSE);
            if settles {
                refused_some += 1;
                let refused = outcome.expect_err(&what);
                assert_eq!(
                    listed(&refused),
                    vec![format!(
                        "nodes[0] x:new: this import takes proposals only, and this record \
                         arrives settled ({} x:new {}={}): settling a record is its owner's \
                         act on the design, never a file's",
                        row.node_type,
                        row.property,
                        said(&stored)
                    )],
                    "{what}"
                );
                assert!(g.get_node(row.node_type, "x:new").unwrap().is_none());
            } else {
                outcome.expect(&what);
                assert_eq!(held(&g, row, "x:new"), stored, "{what}");
            }
        }
    }
    assert!(
        refused_some >= SETTLING.len(),
        "every row refused something"
    );
}

#[test]
fn a_settling_value_the_document_never_stated_is_judged_as_stored() {
    // No schema default settles today, so the walk above cannot show this
    // through an import. The judgement itself is on WHAT WOULD BE STORED: a
    // value the write filled in counts exactly as one the document stated.
    for row in SETTLING {
        let probe = DesignGraph::open_in_memory().unwrap();
        for state in states(&probe, row).into_iter().flatten() {
            let value: Value = serde_json::from_value(state).unwrap();
            let stored = HashMap::from([(row.property.to_string(), value.clone())]);
            let fault = more_than_a_proposal(row.node_type, "x:new", None, &stored);
            assert_eq!(
                fault.is_some(),
                row.when.settles(Some(&value)),
                "{} {}={value:?}",
                row.node_type,
                row.property
            );
        }
    }
}

#[test]
fn proposals_only_refuses_any_move_of_a_held_records_settling_property_for_every_row() {
    // Into a settled value, between two, out of one, and between two values
    // that do not settle: each is a move, and each is refused. Stating the
    // value the record already holds is not.
    let (mut into, mut between, mut out_of, mut neither, mut same) = (0, 0, 0, 0, 0);
    for row in SETTLING {
        let probe = DesignGraph::open_in_memory().unwrap();
        let all = states(&probe, row);
        for from in &all {
            for to in &all {
                for merge in [false, true] {
                    let what = format!(
                        "{} {} {from:?} -> {to:?} (merge: {merge})",
                        row.node_type, row.property
                    );
                    let mut g = DesignGraph::open_in_memory().unwrap();
                    g.import_graph(&one_node(&probe, row, "x:held", from))
                        .expect(&what);
                    let was = held(&g, row, "x:held");
                    let before = everything(&g);
                    let document = one_node(&probe, row, "x:held", to);

                    // What the same import stores without the option.
                    let mut plain = DesignGraph::open_in_memory().unwrap();
                    plain
                        .import_graph(&one_node(&probe, row, "x:held", from))
                        .unwrap();
                    plain
                        .import_graph_with(
                            &document,
                            ImportOptions {
                                merge,
                                ..Default::default()
                            },
                        )
                        .expect(&what);
                    let becomes = held(&plain, row, "x:held");
                    if merge && to.is_none() {
                        assert_eq!(becomes, was, "{what}: a merge keeps what is left out");
                    }

                    let outcome =
                        g.import_graph_with(&document, ImportOptions { merge, ..PROPOSE });
                    if becomes == was {
                        same += 1;
                        outcome.expect(&what);
                        continue;
                    }
                    match (
                        row.when.settles(was.as_ref()),
                        row.when.settles(becomes.as_ref()),
                    ) {
                        (false, true) => into += 1,
                        (true, true) => between += 1,
                        (true, false) => out_of += 1,
                        (false, false) => neither += 1,
                    }
                    let refused = outcome.expect_err(&what);
                    assert_eq!(
                        listed(&refused),
                        vec![format!(
                            "nodes[0] x:held: this import takes proposals only, and it would \
                             move this record's {} from {} to {}: a record the design holds \
                             is moved by its owner on the design, never by a file",
                            row.property,
                            said(&was),
                            said(&becomes)
                        )],
                        "{what}"
                    );
                    assert_eq!(everything(&g), before, "{what}: nothing moved");
                }
            }
        }
    }
    assert!(
        into > 0 && between > 0 && out_of > 0 && same > 0,
        "into {into}, between {between}, out of {out_of}, neither {neither}, same {same}"
    );
}

#[test]
fn proposals_only_refuses_a_signature_that_approves() {
    let approval = |roles: serde_json::Value| {
        doc(json!({
            "nodes": [{"node_type": "Decision", "node_id": "dec:new", "properties": {"name": "New", "decision": "Maybe."}}],
            "edges": [{"edge_type": "AUTHORED_BY", "from_id": "dec:new", "to_id": ME, "properties": roles}]
        }))
    };
    const CARRIES: &str = "edges[0] AUTHORED_BY dec:new -> who:alice: this import takes proposals \
                           only, and this signature carries the approver role: an approval is its \
                           owner's own act on the design, never carried in by a file";

    for props in [
        json!({"roles": ["approver"]}),
        json!({"roles": ["author", "approver"]}),
        // The shape an older export carries.
        json!({"role": "approver"}),
    ] {
        let mut g = design();
        let before = everything(&g);
        let refused = g
            .import_graph_with(&approval(props.clone()), PROPOSE)
            .expect_err("an approval is never a file's");
        assert_eq!(listed(&refused), vec![CARRIES.to_string()], "{props}");
        assert_eq!(everything(&g), before);
    }

    // A signature that only says who wrote it is a proposal like any other.
    let mut g = design();
    g.import_graph_with(&approval(json!({"roles": ["author"]})), PROPOSE)
        .expect("an author signature proposes");
    assert_eq!(
        g.outgoing("dec:new", Some(edge::AUTHORED_BY))
            .unwrap()
            .len(),
        1
    );

    // An approver role ADDED to a signature the design holds is refused...
    let before = everything(&g);
    let refused = g
        .import_graph_with(&approval(json!({"roles": ["author", "approver"]})), PROPOSE)
        .expect_err("adding the role is approving");
    assert_eq!(listed(&refused), vec![CARRIES.to_string()]);
    assert_eq!(everything(&g), before);
}

#[test]
fn proposals_only_refuses_changing_an_approval_the_design_holds_and_accepts_it_restated() {
    // `dec:e` is approved by `who:mallory` in the design.
    let mut g = design();
    let before = everything(&g);

    // The design's own export restates everything it holds, the approval
    // included, and changes nothing: it only proposes.
    let mut own = g.export_graph().unwrap();
    own.content_hash = None;
    g.import_graph_with(&own, PROPOSE)
        .expect("restating what the design holds settles nothing");

    // Taking the approval away is as much the owner's act as giving it.
    let stripped = doc(json!({
        "nodes": [],
        "edges": [{"edge_type": "AUTHORED_BY", "from_id": "dec:e", "to_id": OTHER, "properties": {"roles": ["author"]}}]
    }));
    let mut g = design();
    let outcome = g.import_graph_with(
        &stripped,
        ImportOptions {
            check_only: true,
            ..PROPOSE
        },
    );
    let checked = outcome.expect("a check");
    // Whatever the store does with the roles of a signature written twice
    // (keep the approval, or replace it), the import is refused: the edge it
    // would store either still carries the approval or has lost it.
    assert_eq!(checked.faults.len(), 1, "{:#?}", checked.faults);
    assert!(
        checked.faults[0].starts_with(
            "edges[0] AUTHORED_BY dec:e -> who:mallory: this import takes proposals only, and "
        ),
        "{:#?}",
        checked.faults
    );
    assert_eq!(everything(&g), before);
}

#[test]
fn the_three_options_combine() {
    let mut g = design();
    let before = everything(&g);
    let document = doc(json!({
        "nodes": [
            {"node_type": "Requirement", "node_id": "req:a", "properties": {"name": "A", "statement": "S."}},
            {"node_type": "Requirement", "node_id": "req:settled", "properties": {"name": "S", "statement": "S.", "status": "accepted"}},
            {"node_type": "Requirement", "node_id": "req:fine", "properties": {"name": "F", "statement": "S."}}
        ],
        "edges": [
            {"edge_type": "AUTHORED_BY", "from_id": "req:fine", "to_id": ME, "properties": {"roles": ["approver"]}}
        ]
    }));
    let all = ImportOptions {
        accept_newer: false,
        merge: true,
        check_only: true,
        add_only: true,
        proposals_only: true,
    };
    let checked = g.import_graph_with(&document, all).expect("a check");
    assert_eq!(checked.faults.len(), 3, "{:#?}", checked.faults);
    assert!(checked.faults[0].contains("add-only import"));
    assert!(checked.faults[1].contains("arrives settled"));
    assert!(checked.faults[2].contains("carries the approver role"));
    assert_eq!(everything(&g), before);

    let refused = g
        .import_graph_with(
            &document,
            ImportOptions {
                check_only: false,
                ..all
            },
        )
        .expect_err("the real one");
    assert_eq!(checked.faults, listed(&refused));
    assert_eq!(everything(&g), before);
}

#[test]
fn every_option_is_off_unless_asked_for() {
    let off = ImportOptions::default();
    assert!(!off.check_only && !off.add_only && !off.proposals_only);
    // The ordinary import settles, replaces and approves as it always has.
    let mut g = design();
    let report: ImportReport = g
        .import_graph(&doc(json!({
            "nodes": [
                {"node_type": "Requirement", "node_id": "req:a", "properties": {"name": "A", "statement": "S.", "status": "accepted"}}
            ],
            "edges": [
                {"edge_type": "AUTHORED_BY", "from_id": "req:a", "to_id": ME, "properties": {"roles": ["approver"]}}
            ]
        })))
        .expect("an ordinary import");
    assert_eq!((report.nodes_written, report.edges_written), (1, 1));
    let said = serde_json::to_value(&report).unwrap();
    for absent in [
        "checked_only",
        "faults",
        "would_write",
        "would_adopt_identity",
    ] {
        assert!(
            said.get(absent).is_none(),
            "an ordinary import's report is the one it always was, without `{absent}`: {said}"
        );
    }
}

// ---- under an established caller (an engine served for others) -------------

fn as_caller(g: &mut DesignGraph) {
    g.begin_signing(Signer::Caller {
        contributor: ME.into(),
        how: "named by the test gateway".into(),
    });
}

#[test]
fn a_check_under_an_established_caller_lists_a_signature_naming_someone_else() {
    let mut g = design();
    let before = everything(&g);
    let document = doc(json!({
        "nodes": [{"node_type": "Decision", "node_id": "dec:imported", "properties": {"name": "I", "decision": "D.", "status": "proposed"}}],
        "edges": [{"edge_type": "AUTHORED_BY", "from_id": "dec:imported", "to_id": OTHER, "properties": {"roles": ["author"]}}]
    }));
    as_caller(&mut g);

    let checked = g.import_graph_with(&document, CHECK).expect("a check");
    assert_eq!(checked.faults.len(), 1, "{:#?}", checked.faults);
    assert!(
        checked.faults[0].starts_with("edges[0] AUTHORED_BY dec:imported -> who:mallory: ")
            && checked.faults[0].contains(ME),
        "the caller's own policy refuses it, and the check lists that refusal: {:#?}",
        checked.faults
    );
    assert!(
        g.get_node(node::DECISION, "dec:imported")
            .unwrap()
            .is_none()
    );

    // The real import refuses with the same line and writes nothing.
    let refused = g.import_graph(&document).expect_err("the real import");
    assert_eq!(checked.faults, listed(&refused));
    g.end_signing();
    assert_eq!(everything(&g), before);
}

#[test]
fn under_an_established_caller_a_plain_import_writes_a_settled_record_nobody_signed() {
    // WHAT THIS RECORDS, BY A RUN: an established caller's signing policy
    // holds a signature to the caller's own name; it does not ask who settled
    // a record. So a plain import under it WRITES a requirement that arrives
    // accepted with no signature at all. `proposals_only` is what refuses it.
    let document = doc(json!({
        "nodes": [{"node_type": "Requirement", "node_id": "req:settled", "properties": {"name": "S", "statement": "S.", "status": "accepted"}}],
        "edges": []
    }));

    let mut g = design();
    as_caller(&mut g);
    g.import_graph(&document)
        .expect("a plain import under an established caller writes it");
    assert_eq!(
        held(&g, &SETTLING[0], "req:settled"),
        Some(Value::String("accepted".into()))
    );
    assert!(
        g.outgoing("req:settled", Some(edge::AUTHORED_BY))
            .unwrap()
            .is_empty(),
        "and nobody signed it"
    );

    let mut g = design();
    as_caller(&mut g);
    let refused = g
        .import_graph_with(&document, PROPOSE)
        .expect_err("proposals only refuses it");
    assert!(listed(&refused)[0].contains("arrives settled"), "{refused}");
    assert!(
        g.get_node(node::REQUIREMENT, "req:settled")
            .unwrap()
            .is_none()
    );
}
