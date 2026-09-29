//! A write records the AGENT it went through beside the contributor it was
//! for — the ACTS_FOR rung (`reflow2_core::acting`).
//!
//! `req:a-write-and-an-approval-record-the-agent-and-the-person-it-acts-for`
//! (accepted, Anthony 2026-09-29). The core half: while an agent is named
//! (`begin_acting`), every AUTHORED_BY role written — through the typed
//! `authored_by` or through the generic `create_edge` — carries the agent in
//! that role's `*_via` set, and the first such record draws
//! `agent ACTS_FOR contributor`. Nothing is recorded when no agent is named,
//! an agent is never recorded as acting for itself, and the agent is never
//! made an approver.

use reflow2_core::DesignGraph;
use reflow2_core::acting::Acting;
use reflow2_core::foundation::core::Value;
use reflow2_core::graph::{authored_roles, edge_has_role, role_via};
use reflow2_core::nodes::{Props, edge, node};

fn design() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_project("prj:ikg", "Ideal knowledge graph").unwrap();
    g.add_contributor("who:anthony", "Anthony", Some("person"), None, None)
        .unwrap();
    g.add_contributor(
        "who:owner-agent",
        "The owner agent",
        Some("automated_agent"),
        Some("owner-agent"),
        None,
    )
    .unwrap();
    g.add_requirement("req:fast", "It is fast", "Reads under 20 ms")
        .unwrap();
    g
}

fn acting(route: &str) -> Acting {
    Acting {
        agent: "who:owner-agent".into(),
        route: route.into(),
    }
}

fn edge_to(g: &DesignGraph, from: &str, to: &str) -> reflow2_core::foundation::store::StoredEdge {
    g.outgoing(from, Some(edge::AUTHORED_BY))
        .unwrap()
        .into_iter()
        .find(|e| e.to_id == to)
        .unwrap_or_else(|| panic!("no AUTHORED_BY {from} -> {to}"))
}

fn acts_for(g: &DesignGraph) -> Vec<(String, String, Option<String>)> {
    g.outgoing("who:owner-agent", Some(edge::ACTS_FOR))
        .unwrap()
        .into_iter()
        .map(|e| {
            (
                e.from_id.clone(),
                e.to_id.clone(),
                e.properties
                    .get("route")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            )
        })
        .collect()
}

#[test]
fn an_approval_recorded_through_an_agent_names_both() {
    let mut g = design();
    g.begin_acting(acting("request")).unwrap();
    g.authored_by(
        node::REQUIREMENT,
        "req:fast",
        "who:anthony",
        Some("approver"),
        Some("2026-09-29"),
    )
    .unwrap();
    g.end_acting();
    let e = edge_to(&g, "req:fast", "who:anthony");
    assert!(
        edge_has_role(&e, "approver"),
        "the person is the approver of record"
    );
    assert_eq!(
        role_via(&e, "approver"),
        vec!["who:owner-agent".to_string()]
    );
    assert_eq!(
        acts_for(&g),
        vec![(
            "who:owner-agent".into(),
            "who:anthony".into(),
            Some("request".into())
        )]
    );
    // The agent is never made an approver.
    assert!(
        g.outgoing("req:fast", Some(edge::AUTHORED_BY))
            .unwrap()
            .iter()
            .all(|e| e.to_id != "who:owner-agent")
    );
}

#[test]
fn only_the_role_this_act_wrote_is_stamped() {
    let mut g = design();
    // Written by the person, with no agent...
    g.authored_by(node::REQUIREMENT, "req:fast", "who:anthony", None, None)
        .unwrap();
    // ...then approved through the agent.
    g.begin_acting(acting("session")).unwrap();
    g.authored_by(
        node::REQUIREMENT,
        "req:fast",
        "who:anthony",
        Some("approver"),
        Some("2026-09-29"),
    )
    .unwrap();
    g.end_acting();
    let e = edge_to(&g, "req:fast", "who:anthony");
    assert_eq!(
        authored_roles(&e),
        vec!["author".to_string(), "approver".to_string()]
    );
    assert!(
        role_via(&e, "author").is_empty(),
        "the authorship had no agent"
    );
    assert_eq!(
        role_via(&e, "approver"),
        vec!["who:owner-agent".to_string()]
    );
}

#[test]
fn nothing_named_means_nothing_recorded() {
    let mut g = design();
    g.authored_by(
        node::REQUIREMENT,
        "req:fast",
        "who:anthony",
        Some("approver"),
        Some("2026-09-29"),
    )
    .unwrap();
    let e = edge_to(&g, "req:fast", "who:anthony");
    assert!(role_via(&e, "approver").is_empty());
    assert!(acts_for(&g).is_empty());
}

#[test]
fn the_generic_edge_write_is_stamped_too() {
    let mut g = design();
    g.begin_acting(acting("request")).unwrap();
    g.create_edge(
        edge::AUTHORED_BY,
        node::REQUIREMENT,
        "req:fast",
        node::CONTRIBUTOR,
        "who:anthony",
        Props::new().set("roles", Value::List(vec![Value::String("reviewer".into())])),
    )
    .unwrap();
    g.end_acting();
    let e = edge_to(&g, "req:fast", "who:anthony");
    assert_eq!(
        role_via(&e, "reviewer"),
        vec!["who:owner-agent".to_string()]
    );
    assert_eq!(acts_for(&g).len(), 1);
}

#[test]
fn an_agent_is_never_recorded_as_acting_for_itself() {
    let mut g = design();
    g.begin_acting(acting("client")).unwrap();
    g.authored_by(node::REQUIREMENT, "req:fast", "who:owner-agent", None, None)
        .unwrap();
    g.end_acting();
    let e = edge_to(&g, "req:fast", "who:owner-agent");
    assert!(role_via(&e, "author").is_empty());
    assert!(acts_for(&g).is_empty());
}

#[test]
fn a_person_or_a_stranger_cannot_be_named_the_acting_agent() {
    let mut g = design();
    let person = g
        .begin_acting(Acting {
            agent: "who:anthony".into(),
            route: "session".into(),
        })
        .unwrap_err()
        .to_string();
    assert!(person.contains("automated_agent"), "{person}");
    let stranger = g
        .begin_acting(Acting {
            agent: "who:nobody".into(),
            route: "session".into(),
        })
        .unwrap_err()
        .to_string();
    assert!(stranger.contains("add_contributor"), "{stranger}");
    assert!(g.acting().is_none());
}

#[test]
fn a_client_name_finds_a_declared_agent_and_never_mints_one() {
    let mut g = design();
    assert_eq!(
        g.agent_for_client("owner-agent").unwrap(),
        Some("who:owner-agent".to_string())
    );
    assert_eq!(g.agent_for_client("claude-code").unwrap(), None);
    // Two agents claiming one client name: neither is recorded.
    g.add_contributor(
        "who:other-agent",
        "Another agent",
        Some("automated_agent"),
        Some("owner-agent"),
        None,
    )
    .unwrap();
    assert_eq!(g.agent_for_client("owner-agent").unwrap(), None);
    assert!(
        g.get_node(node::CONTRIBUTOR, "who:claude-code")
            .unwrap()
            .is_none()
    );
}

#[test]
fn the_record_survives_export_and_import() {
    let mut g = design();
    g.begin_acting(acting("request")).unwrap();
    g.authored_by(
        node::REQUIREMENT,
        "req:fast",
        "who:anthony",
        Some("approver"),
        Some("2026-09-29"),
    )
    .unwrap();
    g.end_acting();
    let doc = g.export_graph().unwrap();
    let mut h = DesignGraph::open_in_memory().unwrap();
    h.import_graph(&doc).unwrap();
    let e = edge_to(&h, "req:fast", "who:anthony");
    assert_eq!(
        role_via(&e, "approver"),
        vec!["who:owner-agent".to_string()]
    );
    assert_eq!(acts_for(&h).len(), 1);
}
