//! A RELATION THAT CANNOT HOLD FROM A NODE TO ITSELF IS STATED ONCE, IN THE
//! SCHEMA, AND REFUSED ON EVERY DOOR THAT WRITES AN EDGE.
//!
//! Seen twice, by two different doors:
//!
//! - 2026-10-30, a work hub: `record_finding` with `caused_by` equal to its own
//!   id drew "X CAUSES X", and `create_edge` drew a CAUSES self-loop on any node
//!   (fact:root-cause-a-finding-can-name-itself-as-its-cause-and-create-edge-draws-a-causes-self-loop-2026-10-30).
//! - 2026-11-07, a proof trial: two route alternatives carried "X CONTRADICTS
//!   X" (fact:defect-two-alternatives-in-the-aa-attempts-reflow2-design-carry-a-contradicts-edge-to-themselves-2026-11-07).
//!   Measured: `register_alternative`, called again for an alternative already
//!   registered under the decision, takes every alternative GOVERNED_BY it as
//!   a sibling, itself included, and draws CONTRADICTS to each.
//!
//! THE CLASS: the rule "this relation cannot hold from a node to itself" was
//! kept per door. `review_relations` refused all thirteen of its relations to
//! a node itself, `decomposes` refused DECOMPOSES, the store refused a step
//! that supports itself, a heal merge never re-points an edge into a loop —
//! and `create_edge`, `create_edges`, `record_finding` and
//! `register_alternative` refused nothing. Fixing the door that drew today's
//! would not have stopped the one of 2026-10-30. So the rule is declared once,
//! per edge type, in the schema (`irreflexive: true`), and the graph's one edge
//! write refuses it for every door. A restore keeps what a document already
//! holds and says so, as it does for a dangling reference.

use std::collections::BTreeSet;

use reflow2_core::DesignGraph;
use reflow2_core::export::ExportedEdge;
use reflow2_core::foundation::core::Schema;
use reflow2_core::nodes::{Props, edge, node};

fn schema() -> &'static Schema {
    reflow2_core::schema::parsed_schema().expect("schema parses")
}

/// The relations the schema declares cannot hold from a node to itself
/// (`irreflexive: true`), read as the schema serializes them.
fn declared() -> BTreeSet<String> {
    schema()
        .edge_types
        .iter()
        .filter(|(_, d)| serde_json::to_value(d).unwrap()["irreflexive"] == true)
        .map(|(n, _)| n.clone())
        .collect()
}

/// ⭐ ONE RULE, NOT ONE PER DOOR: every relation some door already refuses to
/// draw to a node itself, and every relation whose own reading says its two
/// ends are not the same, is declared irreflexive in the schema — including
/// the two seen drawn to themselves in the field.
#[test]
fn every_relation_a_door_refuses_to_draw_to_itself_is_declared_so_in_the_schema() {
    let d = declared();
    let mut want: BTreeSet<&str> = [edge::CONTRADICTS, edge::CAUSES].into_iter().collect();
    // review_relations: "'{id}' cannot be related to itself", for each but
    // TRIGGERS. A step of a process may trigger itself (a retry), which a
    // Flow models on purpose (tests/flow.rs,
    // a_self_trigger_is_a_degenerate_cycle_not_a_silent_drop); a review
    // refuses it only because an idea triggering itself means nothing.
    want.extend(
        reflow2_core::relate::REVIEW_RELATIONS
            .iter()
            .copied()
            .filter(|t| *t != edge::TRIGGERS),
    );
    // decomposes: "cannot decompose itself"; a step: "cannot support itself".
    want.extend([edge::DECOMPOSES, edge::PREMISE_OF, edge::CONCLUDES]);
    // A reading that says `¬same-as(a,b)` says the relation is irreflexive.
    for (name, def) in &schema().edge_types {
        let reading = serde_json::to_string(&def.reading).unwrap();
        if reading.contains("¬same-as(a,b)") {
            want.insert(name.as_str());
        }
    }
    assert!(
        !d.contains(edge::TRIGGERS),
        "TRIGGERS is declared irreflexive, which refuses a step that retries itself"
    );
    let missing: Vec<&&str> = want.iter().filter(|t| !d.contains(**t)).collect();
    assert!(
        missing.is_empty(),
        "relations a door already refuses to draw to a node itself, or whose reading says \
         ¬same-as, that the schema does not declare irreflexive: {missing:?} (declared: {d:?})"
    );
}

/// A CALLER CHOOSING AN EDGE IS TOLD: `describe_schema` asked about a pair
/// (`edge_types_between`) says `irreflexive` on the relations it applies to,
/// and on no other.
#[test]
fn describe_schema_says_which_relations_cannot_hold_to_a_node_itself() {
    let g = DesignGraph::open_in_memory().unwrap();
    let q = g
        .edge_types_between(node::REQUIREMENT, node::REQUIREMENT)
        .unwrap();
    let said = |t: &str| -> Option<bool> {
        q.matches
            .iter()
            .find(|m| m.spec.edge_type == t)
            .map(|m| serde_json::to_value(&m.spec).unwrap()["irreflexive"] == true)
    };
    assert_eq!(said(edge::CONTRADICTS), Some(true), "{:?}", q.matches.len());
    assert_eq!(said(edge::CAUSES), Some(true));
    assert_eq!(said(edge::TRIGGERS), Some(false));
}

/// A node of a type both ends of `edge_type` accept, created for the test.
fn endpoint_type(edge_type: &str) -> &'static str {
    let def = &schema().edge_types[edge_type];
    for t in [node::REQUIREMENT, node::INFERENCE, node::DESIGN_RULE] {
        if def.from.accepts(t) && def.to.accepts(t) {
            return t;
        }
    }
    panic!("no test node type is accepted at both ends of {edge_type}");
}

fn graph_with_two_of_each() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().unwrap();
    for id in ["a", "b"] {
        g.create_node(
            node::REQUIREMENT,
            &format!("req:{id}"),
            Props::new()
                .set("name", format!("Requirement {id}"))
                .set("statement", "Stated."),
        )
        .unwrap();
        g.create_node(node::INFERENCE, &format!("inf:{id}"), Props::new())
            .unwrap();
        g.create_node(
            node::DESIGN_RULE,
            &format!("rule:{id}"),
            Props::new()
                .set("name", format!("Rule {id}"))
                .set("statement", "Stated."),
        )
        .unwrap();
    }
    g
}

fn id_of(node_type: &str, which: &str) -> String {
    match node_type {
        t if t == node::REQUIREMENT => format!("req:{which}"),
        t if t == node::INFERENCE => format!("inf:{which}"),
        _ => format!("rule:{which}"),
    }
}

/// THE STORE'S ONE EDGE WRITE refuses a self-loop on every relation the schema
/// declares irreflexive, names the relation, and writes nothing; the same
/// relation between two nodes is still written.
#[test]
fn a_self_loop_on_an_irreflexive_relation_is_refused_and_nothing_is_written() {
    let d = declared();
    assert!(
        d.contains(edge::CONTRADICTS) && d.contains(edge::CAUSES),
        "the schema declares neither CONTRADICTS nor CAUSES irreflexive: {d:?}"
    );
    for t in &d {
        let mut g = graph_with_two_of_each();
        let ty = endpoint_type(t);
        let (a, b) = (id_of(ty, "a"), id_of(ty, "b"));
        let err = g
            .create_edge(t, ty, &a, ty, &a, Props::new())
            .expect_err(&format!("{t} from {a} to itself was written"));
        let said = err.to_string();
        assert!(
            said.contains(t.as_str()) && said.contains("itself"),
            "the refusal of {t} names neither the relation nor why: {said}"
        );
        assert!(
            g.outgoing(&a, Some(t)).unwrap().is_empty(),
            "{t}: the refused self-loop was stored"
        );
        // Not every link of a step can be drawn between two bare nodes; the
        // two that can are the point: a relation, not a node, is refused.
        if t != edge::PREMISE_OF && t != edge::CONCLUDES {
            g.create_edge(t, ty, &a, ty, &b, Props::new())
                .unwrap_or_else(|e| panic!("{t} between two nodes was refused: {e}"));
        }
    }
}

/// THE TRIGGER OF 2026-11-07: registering an alternative again (to move where
/// its export lives, say) draws no CONTRADICTS to itself, keeps its siblings',
/// and records the new location.
#[test]
fn registering_an_alternative_again_draws_no_contradiction_to_itself() {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.create_node(
        node::DECISION,
        "dec:road",
        Props::new()
            .set("name", "Which road")
            .set("decision", "Which road to take")
            .set("status", "proposed"),
    )
    .unwrap();
    g.register_alternative("dec:road", "alt:a", "Road A", "a.json")
        .unwrap();
    g.register_alternative("dec:road", "alt:b", "Road B", "b.json")
        .unwrap();
    g.register_alternative("dec:road", "alt:a", "Road A", "a-v2.json")
        .expect("registering an alternative again is an update");
    let contradicts = |id: &str| -> Vec<String> {
        let mut v: Vec<String> = g
            .outgoing(id, Some(edge::CONTRADICTS))
            .unwrap()
            .into_iter()
            .map(|e| e.to_id)
            .collect();
        v.sort();
        v
    };
    assert_eq!(contradicts("alt:a"), vec!["alt:b".to_string()]);
    assert_eq!(contradicts("alt:b"), vec!["alt:a".to_string()]);
    let alts = g.alternatives_for("dec:road").unwrap();
    let a = alts.iter().find(|x| x.id == "alt:a").unwrap();
    assert_eq!(a.location.as_deref(), Some("a-v2.json"));
}

/// A RESTORE KEEPS WHAT THE DOCUMENT HOLDS, AND SAYS SO: a design written
/// before the rule imports whole, its self-loop named in the import report;
/// `detect_defects` says what it is and how to remove it, instead of "'X' and
/// 'X' contradict each other".
#[test]
fn a_design_written_before_the_rule_restores_whole_and_names_its_self_loop() {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.create_node(
        node::REQUIREMENT,
        "req:x",
        Props::new().set("name", "X").set("statement", "Stated."),
    )
    .unwrap();
    let mut doc = g.export_graph().unwrap();
    doc.edges.push(ExportedEdge {
        edge_type: edge::CONTRADICTS.to_string(),
        from_id: "req:x".to_string(),
        to_id: "req:x".to_string(),
        properties: Default::default(),
    });
    let mut fresh = DesignGraph::open_in_memory().unwrap();
    let report = fresh
        .import_graph(&doc)
        .expect("a document holding a self-loop still restores");
    let kept = serde_json::to_value(&report).unwrap()["self_loops_kept"].clone();
    assert!(
        kept.as_array().is_some_and(|k| k.iter().any(|s| s
            .as_str()
            .is_some_and(|s| s.contains("CONTRADICTS") && s.contains("req:x")))),
        "the restore kept the self-loop and did not say so: {report:?}"
    );
    assert_eq!(
        fresh
            .outgoing("req:x", Some(edge::CONTRADICTS))
            .unwrap()
            .len(),
        1,
        "a restore must reproduce what the document holds"
    );
    let sweep = fresh.detect_defects().unwrap();
    let issue = sweep
        .defects
        .iter()
        .find(|i| i.affected_ids.iter().all(|a| a == "req:x"))
        .expect("the stored self-loop is reported");
    assert!(
        issue.message.contains("itself") && issue.message.contains("delete_edge"),
        "{}",
        issue.message
    );
}
