//! One node's edges are read back AS EDGES — every edge, both directions, with
//! the other end named and the edge's own properties, bounded, and with every
//! count a filter or a bound could otherwise hide
//! (`dec:idea-an-edge-reader-returns-one-nodes-edges-and-find-tools-finds-it`).
//!
//! THE CLASS THESE PIN, not the instance: the design's edges had no per-node
//! read
//! (`fact:root-cause-no-served-read-returns-one-nodes-edges-and-the-nearest-is-an-impact-walk-2026-10-02`).
//! The nearest reader, `propagate_from` at depth 1, gave 6 of 7 edges on a
//! real node: it left out `AUTHORED_BY` (not a traceability edge), named an
//! impact direction instead of from → to, and showed no evidence. So the first
//! test builds a node with exactly those edges — an authorship, an incoming
//! and an outgoing traceability edge each with evidence, and a stored twin —
//! and asks for every one of them back as stored.
//!
//! The served half (`get_node` with `include_edges`, its schema, find_tools)
//! is pinned in `crates/reflow2-mcp/tests/get_node_reads_its_edges.rs`.

use reflow2_core::DesignGraph;
use reflow2_core::foundation::core::Value;
use reflow2_core::node_edges::{
    DEFAULT_EDGE_LIMIT, EdgeDirection, EdgeQuery, EdgeRow, EdgeSide, NodeEdges,
};
use reflow2_core::nodes::{Props, edge, node};

const BUDGET: usize = reflow2_core::detect::DEFAULT_REPLY_BUDGET_CHARS;

/// A requirement with one edge of each kind the impact walk mishandled:
///
/// - `cap:c SATISFIES req:r`, with `evidence` (incoming).
/// - `req:r AUTHORED_BY who:a`, with `roles` (outgoing, not traceability).
/// - `req:r DEPENDS_ON req:base`, with a `note` (outgoing).
/// - `req:r HAS_TEMPORAL_FACT fact:f`, the stored twin of `fact:f.subject_id`
///   (outgoing; drawn by the store, never by hand).
fn design() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("in-memory graph");
    g.add_requirement(
        "req:r",
        "Read one node's edges",
        "A node's edges can be read",
    )
    .expect("req");
    g.add_requirement("req:base", "A base need", "Something req:r rests on")
        .expect("req base");
    g.add_capability("cap:c", "An edge reader", "Reads a node's edges", None)
        .expect("cap");
    g.add_contributor("who:a", "A. Person", None, None, None)
        .expect("contributor");
    g.create_edge(
        edge::SATISFIES,
        node::CAPABILITY,
        "cap:c",
        node::REQUIREMENT,
        "req:r",
        Props::new().set("evidence", "the reader returns every edge"),
    )
    .expect("satisfies");
    g.create_edge(
        edge::AUTHORED_BY,
        node::REQUIREMENT,
        "req:r",
        node::CONTRIBUTOR,
        "who:a",
        Props::new().set("roles", Value::List(vec![Value::String("author".into())])),
    )
    .expect("authored_by");
    g.create_edge(
        edge::DEPENDS_ON,
        node::REQUIREMENT,
        "req:r",
        node::REQUIREMENT,
        "req:base",
        Props::new().set("note", "it cannot hold without the base"),
    )
    .expect("depends_on");
    g.create_node(
        node::TEMPORAL_FACT,
        "fact:f",
        Props::new()
            .set("name", "A finding about req:r")
            .set("subject_id", "req:r")
            .set("statement", "measured"),
    )
    .expect("fact");
    g
}

fn read(g: &DesignGraph, ty: &str, id: &str, q: &EdgeQuery) -> NodeEdges {
    g.node_edges(ty, id, q, BUDGET, 0).expect("node_edges")
}

fn row<'a>(e: &'a NodeEdges, edge_type: &str) -> &'a EdgeRow {
    e.items
        .iter()
        .find(|r| r.edge_type == edge_type)
        .unwrap_or_else(|| panic!("no {edge_type} edge in {:#?}", e.items))
}

#[test]
fn every_edge_comes_back_as_stored_with_its_other_end_and_its_own_properties() {
    let g = design();
    let e = read(&g, node::REQUIREMENT, "req:r", &EdgeQuery::default());

    assert_eq!(e.total, 4, "four edges are stored on req:r: {:#?}", e.items);
    assert_eq!(e.matched, 4);
    assert_eq!(e.returned, 4);
    assert_eq!(e.omitted, 0);
    assert_eq!(e.next_offset, None);
    assert_eq!(e.capped_by, None);
    assert!(e.empty_because.is_none());

    // The edge the impact walk dropped is here, with its own properties.
    let authored = row(&e, edge::AUTHORED_BY);
    assert_eq!(authored.direction, EdgeSide::Out);
    assert_eq!(
        (authored.from_id.as_str(), authored.to_id.as_str()),
        ("req:r", "who:a")
    );
    assert_eq!(authored.other.node_id, "who:a");
    assert_eq!(authored.other.node_type.as_deref(), Some(node::CONTRIBUTOR));
    assert_eq!(authored.other.name.as_deref(), Some("A. Person"));
    assert!(authored.properties.contains_key("roles"));

    // Incoming: the STORED direction, not an impact label, and the evidence.
    let satisfies = row(&e, edge::SATISFIES);
    assert_eq!(satisfies.direction, EdgeSide::In);
    assert_eq!(
        (satisfies.from_id.as_str(), satisfies.to_id.as_str()),
        ("cap:c", "req:r")
    );
    assert_eq!(satisfies.other.node_type.as_deref(), Some(node::CAPABILITY));
    assert_eq!(satisfies.other.name.as_deref(), Some("An edge reader"));
    assert_eq!(
        satisfies.properties.get("evidence").and_then(Value::as_str),
        Some("the reader returns every edge"),
        "an edge's own evidence must come back with it"
    );
    assert!(
        satisfies.twin_of.is_none(),
        "an ordinary edge is not a twin"
    );

    let depends = row(&e, edge::DEPENDS_ON);
    assert_eq!(depends.direction, EdgeSide::Out);
    assert_eq!(
        depends.properties.get("note").and_then(Value::as_str),
        Some("it cannot hold without the base")
    );

    // The by_type tally covers every edge, by direction.
    assert_eq!(e.by_type[edge::SATISFIES].incoming, 1);
    assert_eq!(e.by_type[edge::SATISFIES].out, 0);
    assert_eq!(e.by_type[edge::AUTHORED_BY].out, 1);
}

#[test]
fn a_stored_twin_is_listed_and_says_which_property_it_copies_from_either_end() {
    let g = design();
    let from_subject = read(&g, node::REQUIREMENT, "req:r", &EdgeQuery::default());
    let twin = row(&from_subject, edge::HAS_TEMPORAL_FACT);
    assert_eq!(twin.direction, EdgeSide::Out);
    assert_eq!(twin.other.node_id, "fact:f");
    assert_eq!(twin.twin_of.as_deref(), Some("TemporalFact.subject_id"));
    assert_eq!(from_subject.twins, 1);
    assert!(
        from_subject.twins_note.is_some(),
        "a reply listing a twin says what a twin is"
    );

    // From the finding's side the property is on the node read.
    let from_fact = read(&g, node::TEMPORAL_FACT, "fact:f", &EdgeQuery::default());
    let twin = row(&from_fact, edge::HAS_TEMPORAL_FACT);
    assert_eq!(twin.direction, EdgeSide::In);
    assert_eq!(twin.other.node_id, "req:r");
    assert_eq!(twin.twin_of.as_deref(), Some("TemporalFact.subject_id"));
}

#[test]
fn a_filter_narrows_the_list_and_never_the_counts() {
    let g = design();
    let only_in = read(
        &g,
        node::REQUIREMENT,
        "req:r",
        &EdgeQuery {
            direction: EdgeDirection::In,
            ..Default::default()
        },
    );
    assert_eq!(only_in.matched, 1);
    assert!(only_in.items.iter().all(|r| r.direction == EdgeSide::In));
    assert_eq!(only_in.total, 4, "the total is every edge, filter or not");
    assert_eq!(only_in.filtered_out, 3);
    assert_eq!(only_in.by_type.len(), 4, "by_type is never filtered");

    let only_out = read(
        &g,
        node::REQUIREMENT,
        "req:r",
        &EdgeQuery {
            direction: EdgeDirection::Out,
            ..Default::default()
        },
    );
    assert_eq!(only_out.matched, 3);
    assert!(only_out.items.iter().all(|r| r.direction == EdgeSide::Out));

    let kept = read(
        &g,
        node::REQUIREMENT,
        "req:r",
        &EdgeQuery {
            edge_types: vec![edge::SATISFIES.into(), edge::AUTHORED_BY.into()],
            ..Default::default()
        },
    );
    let mut types: Vec<&str> = kept.items.iter().map(|r| r.edge_type.as_str()).collect();
    types.sort();
    assert_eq!(types, vec![edge::AUTHORED_BY, edge::SATISFIES]);
    assert_eq!(
        kept.filter.edge_types.as_deref(),
        Some(&[edge::SATISFIES.to_string(), edge::AUTHORED_BY.to_string()][..]),
        "the filter as applied is echoed"
    );

    let dropped = read(
        &g,
        node::REQUIREMENT,
        "req:r",
        &EdgeQuery {
            exclude_edge_types: vec![edge::AUTHORED_BY.into()],
            ..Default::default()
        },
    );
    assert_eq!(dropped.matched, 3);
    assert!(
        dropped
            .items
            .iter()
            .all(|r| r.edge_type != edge::AUTHORED_BY)
    );
    assert_eq!(dropped.by_type[edge::AUTHORED_BY].out, 1);
}

#[test]
fn a_filter_naming_an_edge_type_that_does_not_exist_is_refused_with_the_right_name() {
    let g = design();
    for q in [
        EdgeQuery {
            edge_types: vec!["satisfies".into()],
            ..Default::default()
        },
        EdgeQuery {
            exclude_edge_types: vec!["satisfies".into()],
            ..Default::default()
        },
    ] {
        let err = g
            .node_edges(node::REQUIREMENT, "req:r", &q, BUDGET, 0)
            .expect_err("an unknown edge type must be refused, not matched against nothing");
        let msg = err.to_string();
        assert!(msg.contains("\"satisfies\""), "names what was given: {msg}");
        assert!(
            msg.contains("Did you mean SATISFIES"),
            "names the near one: {msg}"
        );
    }
}

#[test]
fn an_empty_list_says_which_empty_it_is() {
    let mut g = design();
    g.add_requirement("req:alone", "Alone", "Nothing links here")
        .expect("req");

    let none = read(&g, node::REQUIREMENT, "req:alone", &EdgeQuery::default());
    assert_eq!((none.total, none.returned), (0, 0));
    let why = none.empty_because.expect("an empty list must say why");
    assert!(why.contains("no edges at all"), "{why}");

    let filtered = read(
        &g,
        node::REQUIREMENT,
        "req:r",
        &EdgeQuery {
            edge_types: vec![edge::INCLUDES.into()],
            ..Default::default()
        },
    );
    let why = filtered
        .empty_because
        .expect("a filtered-away list must say why");
    assert!(why.contains("None of this node's 4 edge(s) match"), "{why}");

    let counts_only = read(
        &g,
        node::REQUIREMENT,
        "req:r",
        &EdgeQuery {
            limit: Some(0),
            ..Default::default()
        },
    );
    assert_eq!(counts_only.matched, 4);
    assert_eq!(counts_only.capped_by, Some("limit"));
    assert!(
        counts_only
            .empty_because
            .expect("limit 0 says so")
            .contains("counts only")
    );

    let past = read(
        &g,
        node::REQUIREMENT,
        "req:r",
        &EdgeQuery {
            offset: 9,
            ..Default::default()
        },
    );
    assert!(
        past.empty_because
            .expect("past the end says so")
            .contains("past the end")
    );
}

/// A release that INCLUDES many artifacts, plus two rarer edges.
fn release_design(artifacts: usize) -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("in-memory graph");
    g.create_node(
        node::RELEASE,
        "rel:big",
        Props::new().set("name", "A big release"),
    )
    .expect("release");
    g.add_contributor("who:a", "A. Person", None, None, None)
        .expect("contributor");
    g.create_edge(
        edge::AUTHORED_BY,
        node::RELEASE,
        "rel:big",
        node::CONTRIBUTOR,
        "who:a",
        Props::new(),
    )
    .expect("authored");
    for i in 0..artifacts {
        let id = format!("art:{i:03}");
        g.create_node(
            node::ARTIFACT,
            &id,
            Props::new().set("name", format!("artifact {i}")),
        )
        .expect("artifact");
        g.create_edge(
            edge::INCLUDES,
            node::RELEASE,
            "rel:big",
            node::ARTIFACT,
            &id,
            Props::new().set("note", "x".repeat(200)),
        )
        .expect("includes");
    }
    g
}

#[test]
fn the_limit_bounds_the_list_and_says_where_to_resume() {
    let g = release_design(30);
    let first = read(
        &g,
        node::RELEASE,
        "rel:big",
        &EdgeQuery {
            limit: Some(10),
            ..Default::default()
        },
    );
    assert_eq!(first.total, 31);
    assert_eq!(first.returned, 10);
    assert_eq!(first.omitted, 21);
    assert_eq!(first.capped_by, Some("limit"));
    assert_eq!(first.next_offset, Some(10));
    assert!(
        first
            .note
            .as_deref()
            .unwrap_or("")
            .contains("\"offset\": 10")
    );

    // Paging is stable and covers every edge exactly once.
    let mut seen: Vec<String> = Vec::new();
    let mut offset = 0;
    loop {
        let page = read(
            &g,
            node::RELEASE,
            "rel:big",
            &EdgeQuery {
                limit: Some(10),
                offset,
                ..Default::default()
            },
        );
        seen.extend(
            page.items
                .iter()
                .map(|r| format!("{} {}", r.edge_type, r.other.node_id)),
        );
        match page.next_offset {
            Some(n) => offset = n,
            None => break,
        }
    }
    let mut unique = seen.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(seen.len(), 31, "every edge, once: {seen:?}");
    assert_eq!(unique.len(), 31);

    assert_eq!(
        DEFAULT_EDGE_LIMIT, 50,
        "the default bound is stated in the docs"
    );
}

#[test]
fn a_dominant_type_is_listed_last_and_named_with_the_filter_that_drops_it() {
    let g = release_design(30);
    let e = read(
        &g,
        node::RELEASE,
        "rel:big",
        &EdgeQuery {
            limit: Some(5),
            ..Default::default()
        },
    );
    assert_eq!(
        e.items[0].edge_type,
        edge::AUTHORED_BY,
        "the rarest type comes first, so a bounded reply shows it"
    );
    let dominant = e
        .dominant
        .expect("INCLUDES holds 30 of 31 and the list was cut");
    assert_eq!(dominant.edge_type, edge::INCLUDES);
    assert_eq!((dominant.count, dominant.of), (30, 31));
    assert!(
        dominant.note.contains("exclude_edge_types"),
        "{}",
        dominant.note
    );

    // The filter it names does what it says.
    let without = read(
        &g,
        node::RELEASE,
        "rel:big",
        &EdgeQuery {
            exclude_edge_types: vec![edge::INCLUDES.into()],
            ..Default::default()
        },
    );
    assert_eq!(without.matched, 1);
    assert!(without.dominant.is_none());
    assert_eq!(
        without.by_type[edge::INCLUDES].out,
        30,
        "and still counts it"
    );

    // A list that showed everything has nothing to warn about.
    let whole = read(&g, node::RELEASE, "rel:big", &EdgeQuery::default());
    assert!(whole.dominant.is_none());
}

#[test]
fn the_reply_budget_bounds_the_list_by_size_and_lists_each_edge_whole() {
    let g = release_design(30);
    let small = g
        .node_edges(node::RELEASE, "rel:big", &EdgeQuery::default(), 4_000, 0)
        .expect("node_edges");
    assert_eq!(small.capped_by, Some("size"));
    assert!(small.returned >= 1, "at least one edge is always listed");
    assert!(small.returned < 31);
    assert_eq!(small.omitted, 31 - small.returned);
    assert_eq!(small.next_offset, Some(small.returned));
    assert!(
        small
            .items
            .iter()
            .filter(|r| r.edge_type == edge::INCLUDES)
            .all(|r| r.properties.get("note").and_then(Value::as_str) == Some(&"x".repeat(200))),
        "an edge is listed whole, never with its prose cut"
    );
    let chars = serde_json::to_string(&small).expect("json").len();
    assert!(
        chars <= 4_000,
        "the reply fits the budget it was given: {chars} characters"
    );

    // Even a budget smaller than one edge lists one.
    let tiny = g
        .node_edges(node::RELEASE, "rel:big", &EdgeQuery::default(), 10, 0)
        .expect("node_edges");
    assert_eq!(tiny.returned, 1);
    assert_eq!(tiny.capped_by, Some("size"));
}

#[test]
fn a_self_loop_is_one_edge() {
    let mut g = design();
    g.create_edge(
        edge::DEPENDS_ON,
        node::REQUIREMENT,
        "req:base",
        node::REQUIREMENT,
        "req:base",
        Props::new(),
    )
    .expect("self loop");
    let e = read(&g, node::REQUIREMENT, "req:base", &EdgeQuery::default());
    // req:r DEPENDS_ON req:base (in) + the loop.
    assert_eq!(e.total, 2, "{:#?}", e.items);
    assert_eq!(e.matched, 2);
    let only_in = read(
        &g,
        node::REQUIREMENT,
        "req:base",
        &EdgeQuery {
            direction: EdgeDirection::In,
            ..Default::default()
        },
    );
    assert_eq!(only_in.matched, 2, "the loop also arrives at the node");
}

#[test]
fn what_the_caller_already_sends_comes_out_of_the_same_budget() {
    // get_node sends the node whole beside the edges, so the edges get what
    // is left: a node that has spent most of the budget leaves room for fewer.
    let g = release_design(30);
    let roomy = g
        .node_edges(node::RELEASE, "rel:big", &EdgeQuery::default(), 12_000, 0)
        .expect("node_edges");
    let crowded = g
        .node_edges(
            node::RELEASE,
            "rel:big",
            &EdgeQuery::default(),
            12_000,
            8_000,
        )
        .expect("node_edges");
    assert!(
        crowded.returned < roomy.returned,
        "{} edges with nothing spent, {} with 8,000 of 12,000 spent",
        roomy.returned,
        crowded.returned
    );
    assert_eq!(
        crowded.budget_chars, 12_000,
        "the budget reported is the one asked for"
    );
    let chars = serde_json::to_string(&crowded).expect("json").len();
    assert!(
        chars + 8_000 <= 12_000,
        "together they fit: {chars} + 8,000"
    );
}
