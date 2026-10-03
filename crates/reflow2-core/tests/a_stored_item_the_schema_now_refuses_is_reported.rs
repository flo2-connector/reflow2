//! A stored node or edge that the CURRENT schema refuses is reported by
//! `detect_defects`, by the same rule the write path and the import apply, and
//! with the replacement the import names when it knows one.
//!
//! `dec:idea-stored-data-is-rechecked-against-the-current-schema`, accepted by
//! Anthony 2026-10-02, piece 1.
//!
//! # The class
//!
//! The schema is checked only when something is WRITTEN. A store minted under
//! an older reflow2 can hold an edge a later release refuses — measured
//! 2026-10-01 on musicjug's design (minted by 0.45.0): `Artifact REALIZES
//! Decision`, which 0.75.0 exported without a word and then refused on import,
//! all-or-nothing, so the design could not move until the edge was repaired by
//! hand (`fact:a-store-keeps-an-edge-a-newer-schema-refuses-and-its-export-cannot-be-imported-2026-10-01`).
//! Opening a store, `export_graph` and `detect_defects` never re-checked what
//! was stored, so the first reader of the new rule was the next import —
//! usually a move, and usually when the author is not there
//! (`fact:root-cause-a-schema-narrowing-ships-its-migration-by-hand-per-pair-and-nothing-rechecks-a-store-2026-10-02`).
//!
//! # How the older store is made
//!
//! `write_under_schema` writes with a WIDER schema — the one an older reflow2
//! had — and hands the store back under today's. The store keeps no vocabulary
//! of its own, so this is the upgrade itself, not a stand-in for it. Each case
//! narrows ONE thing, the way a release does.
//!
//! These tests read the reply as JSON on purpose: they were written against
//! the unfixed code, where the category did not exist, and watched to fail
//! there (2026-10-03).

use reflow2_core::export::GraphExport;
use reflow2_core::foundation::core::{EdgeEndpoint, PropertyDef, PropertyType, Schema};
use reflow2_core::graph::DesignGraph;
use reflow2_core::heal::{HealOptions, HealStrategy};
use reflow2_core::nodes::{Props, edge, node};
use reflow2_core::schema::load_schema;
use serde_json::Value as Json;

const CATEGORY: &str = "refused_by_schema";

/// A small design every case starts from: one of each type the cases touch,
/// all written under today's schema.
fn world() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_project("proj:pump", "Pump").expect("project");
    g.add_capability("cap:impeller", "Impeller", "moves water", None)
        .expect("capability");
    g.create_node(
        node::ARTIFACT,
        "art:palettes",
        Props::new()
            .set("name", "palettes.js")
            .set("location", "src/palettes.js"),
    )
    .expect("artifact");
    g.create_node(
        node::DECISION,
        "dec:styles-are-palettes",
        Props::new()
            .set("name", "Styles are palettes")
            .set("decision", "A style is a palette."),
    )
    .expect("decision");
    g.create_edge(
        edge::REALIZES,
        node::ARTIFACT,
        "art:palettes",
        node::CAPABILITY,
        "cap:impeller",
        Props::new(),
    )
    .expect("a legal REALIZES, so the store is not all refusals");
    g
}

/// Today's schema with REALIZES accepting any target, as it did until
/// 2026-09-23 (`dec:realizes-is-restricted-to-capability-component-and-interface`).
fn realizes_was_a_wildcard() -> Schema {
    let mut s = load_schema().expect("schema");
    s.edge_types.get_mut(edge::REALIZES).expect("REALIZES").to =
        EdgeEndpoint::Single(EdgeEndpoint::WILDCARD.to_string());
    s
}

/// The refused-by-schema findings, read from the reply as JSON.
fn refused(g: &DesignGraph) -> Vec<Json> {
    let sweep = serde_json::to_value(g.detect_defects().expect("detect")).expect("json");
    sweep["defects"]
        .as_array()
        .expect("defects")
        .iter()
        .filter(|d| d["category"] == CATEGORY)
        .cloned()
        .collect()
}

fn text(v: &Json) -> String {
    serde_json::to_string(v).expect("json")
}

/// THE FIELD CASE, shape for shape: a file REALIZES a Decision, written while
/// REALIZES was a wildcard.
fn musicjug_shape() -> DesignGraph {
    let mut g = world();
    g.write_under_schema(realizes_was_a_wildcard(), |g| {
        g.create_edge(
            edge::REALIZES,
            node::ARTIFACT,
            "art:palettes",
            node::DECISION,
            "dec:styles-are-palettes",
            Props::new(),
        )
    })
    .expect("the older schema accepted it");
    g
}

#[test]
fn a_stored_edge_the_schema_now_refuses_is_reported_with_the_replacement_the_import_names() {
    let g = musicjug_shape();
    let found = refused(&g);
    assert_eq!(
        found.len(),
        1,
        "exactly the one refused edge, and not the legal REALIZES beside it: {found:#?}"
    );
    let f = &found[0];
    let mut affected: Vec<&str> = f["affected_ids"]
        .as_array()
        .expect("affected")
        .iter()
        .map(|v| v.as_str().expect("id"))
        .collect();
    affected.sort();
    assert_eq!(affected, ["art:palettes", "dec:styles-are-palettes"]);
    let r = &f["refusal"];
    assert_eq!(r["item"], "edge", "{}", text(f));
    assert_eq!(r["edge_type"], "REALIZES");
    assert_eq!(r["from_type"], "Artifact");
    assert_eq!(r["to_type"], "Decision");
    assert_eq!(
        r["refused_by"][0]["rule"],
        "endpoint_pair",
        "WHICH rule refuses it: {}",
        text(r)
    );
    assert!(
        r["replacement"]
            .as_str()
            .is_some_and(|s| s.contains("DOCUMENTS")),
        "the replacement the import already names for a file and a Decision: {}",
        text(r)
    );
    assert!(
        f["message"].as_str().is_some_and(|m| m.contains("import")),
        "the message says what it costs — the export cannot be imported: {}",
        text(f)
    );
    assert_eq!(f["severity"], "critical", "{}", text(f));
}

#[test]
fn a_stored_enum_value_the_schema_no_longer_declares_is_reported() {
    let mut g = world();
    let mut older = load_schema().expect("schema");
    older
        .node_types
        .get_mut(node::DECISION)
        .expect("Decision")
        .properties
        .get_mut("status")
        .expect("status")
        .values
        .as_mut()
        .expect("an enum")
        .push("obsolete".into());
    g.write_under_schema(older, |g| {
        g.create_node(
            node::DECISION,
            "dec:old-status",
            Props::new()
                .set("name", "Old")
                .set("decision", "Old.")
                .set("status", "obsolete"),
        )
    })
    .expect("the older schema accepted it");
    let found = refused(&g);
    assert_eq!(found.len(), 1, "{found:#?}");
    let r = &found[0]["refusal"];
    assert_eq!(r["item"], "node");
    assert_eq!(r["node_id"], "dec:old-status");
    assert_eq!(r["refused_by"][0]["rule"], "property");
    assert_eq!(r["refused_by"][0]["property"], "status");
    assert!(
        text(r).contains("obsolete"),
        "the refusal names the value: {}",
        text(r)
    );
}

#[test]
fn a_property_required_since_the_node_was_written_is_reported() {
    let mut g = world();
    let mut older = load_schema().expect("schema");
    let purpose: &mut PropertyDef = older
        .node_types
        .get_mut(node::COMPONENT)
        .expect("Component")
        .properties
        .get_mut("purpose")
        .expect("purpose");
    assert!(
        purpose.required && purpose.default.is_none(),
        "the case needs a required property with no default to narrow onto"
    );
    purpose.required = false;
    g.write_under_schema(older, |g| {
        g.create_node(node::COMPONENT, "cmp:old", Props::new().set("name", "Old"))
    })
    .expect("the older schema accepted it");
    let found = refused(&g);
    assert_eq!(found.len(), 1, "{found:#?}");
    let r = &found[0]["refusal"];
    assert_eq!(r["node_id"], "cmp:old");
    assert_eq!(r["refused_by"][0]["property"], "purpose");
}

#[test]
fn a_stored_edge_of_a_retired_type_is_reported() {
    let mut g = world();
    let mut older = load_schema().expect("schema");
    older.edge_types.insert(
        "VALIDATES".into(),
        reflow2_core::foundation::core::EdgeTypeDef::default(),
    );
    g.write_under_schema(older, |g| {
        g.create_edge(
            "VALIDATES",
            node::ARTIFACT,
            "art:palettes",
            node::CAPABILITY,
            "cap:impeller",
            Props::new(),
        )
    })
    .expect("the older schema had the edge type");
    let found = refused(&g);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0]["refusal"]["refused_by"][0]["rule"], "edge_type");
}

/// The detector reports exactly what the import refuses — the same items, by
/// the same rule. A second copy of the rule could drift from the first; this
/// is the test that would see it.
#[test]
fn the_detector_and_the_import_refuse_the_same_items() {
    let mut g = musicjug_shape();
    let mut older = load_schema().expect("schema");
    older
        .node_types
        .get_mut(node::COMPONENT)
        .expect("Component")
        .properties
        .get_mut("purpose")
        .expect("purpose")
        .required = false;
    g.write_under_schema(older, |g| {
        g.create_node(node::COMPONENT, "cmp:old", Props::new().set("name", "Old"))
    })
    .expect("older");
    let found = refused(&g);
    assert_eq!(found.len(), 2, "{found:#?}");

    let doc: GraphExport = g
        .export_graph()
        .expect("export does not validate, so it succeeds");
    let mut fresh = DesignGraph::open_in_memory().expect("open");
    let err = fresh
        .import_graph(&doc)
        .expect_err("the import refuses what the detector reported");
    let msg = format!("{err}");
    let listed = msg.matches("\n  - ").count();
    assert_eq!(
        listed,
        found.len(),
        "one import fault per finding, no more and no fewer:\n{msg}"
    );
    assert!(
        msg.contains("art:palettes -> dec:styles-are-palettes") && msg.contains("cmp:old"),
        "{msg}"
    );
}

#[test]
fn a_clean_design_reports_none_and_says_what_it_examined() {
    let g = world();
    assert!(refused(&g).is_empty());
    let sweep = serde_json::to_value(g.detect_defects().expect("detect")).expect("json");
    assert!(
        sweep["swept"]["rules"]
            .as_array()
            .expect("rules")
            .iter()
            .any(|r| r == CATEGORY),
        "the rule says it ran: {}",
        text(&sweep["swept"]["rules"])
    );
    let population = sweep["swept"]["rule_populations"]
        .as_array()
        .expect("populations")
        .iter()
        .find(|p| p["rule"] == CATEGORY)
        .cloned()
        .expect("the rule says what it walked");
    // 4 nodes + 1 edge, plus whatever the constructors drew on their own.
    assert!(
        population["examined"].as_u64().expect("examined") >= 5,
        "{}",
        text(&population)
    );
}

/// An edge the import REWRITES is not one it refuses: `Artifact REALIZES
/// Verification` arrives as IMPLEMENTS, and opening a store does the same.
#[test]
fn an_edge_the_import_rewrites_is_not_reported_as_refused() {
    let mut g = world();
    g.add_verification("ver:flow", "flow test", None, None, None)
        .expect("ver");
    g.write_under_schema(realizes_was_a_wildcard(), |g| {
        g.create_edge(
            edge::REALIZES,
            node::ARTIFACT,
            "art:palettes",
            node::VERIFICATION,
            "ver:flow",
            Props::new(),
        )
    })
    .expect("older");
    assert!(refused(&g).is_empty(), "{:#?}", refused(&g));
}

/// The replacement is a PROPOSAL. A file and a Decision were repaired to
/// GOVERNED_BY in the field while the import named DOCUMENTS — so no heal may
/// apply one, and the proposal must still name the item for a person.
#[test]
fn propose_heal_proposes_the_replacement_and_applies_nothing() {
    let mut g = musicjug_shape();
    let id = refused(&g)[0]["id"].as_str().expect("id").to_string();
    let proposal = g
        .propose_heal(HealOptions {
            strategy: HealStrategy::Aggressive,
            max_operations: None,
        })
        .expect("propose");
    let p = serde_json::to_value(&proposal).expect("json");
    assert!(
        p["operations"]
            .as_array()
            .expect("operations")
            .iter()
            .all(|o| o["issue_id"] != id.as_str()),
        "no operation may carry it: {}",
        text(&p["operations"])
    );
    assert!(
        p["generated_content"]
            .as_array()
            .expect("generated_content")
            .iter()
            .any(|s| s["for_issue"] == id.as_str()
                && s["description"]
                    .as_str()
                    .is_some_and(|d| d.contains("DOCUMENTS"))),
        "it is proposed, for a person, naming the replacement: {}",
        text(&p["generated_content"])
    );
    assert_eq!(p["requires_human_review"], true);
    g.apply_heal(&proposal).expect("apply");
    assert!(
        g.outgoing("art:palettes", Some(edge::REALIZES))
            .expect("edges")
            .iter()
            .any(|e| e.to_id == "dec:styles-are-palettes"),
        "the refused edge is still there — nothing repaired it behind the owner's back"
    );
}

/// A finding about a stored item can be accepted like any other, and the
/// acknowledgement keys to the item — a SECOND refused edge between the same
/// two nodes is a different finding, not a duplicate of the first.
#[test]
fn two_refused_edges_between_one_pair_are_two_findings() {
    let mut g = musicjug_shape();
    let mut older = realizes_was_a_wildcard();
    older
        .edge_types
        .get_mut(edge::CHANGED)
        .expect("CHANGED")
        .from = EdgeEndpoint::Single(EdgeEndpoint::WILDCARD.to_string());
    g.write_under_schema(older, |g| {
        g.create_edge(
            edge::CHANGED,
            node::ARTIFACT,
            "art:palettes",
            node::DECISION,
            "dec:styles-are-palettes",
            Props::new(),
        )
    })
    .expect("older");
    let found = refused(&g);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert_ne!(found[0]["id"], found[1]["id"], "{found:#?}");
}

#[test]
fn the_older_schema_is_put_back() {
    let mut g = world();
    g.write_under_schema(realizes_was_a_wildcard(), |_| Ok(()))
        .expect("nothing");
    assert!(
        g.create_edge(
            edge::REALIZES,
            node::ARTIFACT,
            "art:palettes",
            node::DECISION,
            "dec:styles-are-palettes",
            Props::new(),
        )
        .is_err(),
        "today's rule applies again the moment the write returns"
    );
    let _ = PropertyType::String;
}

/// The VERIFIES precedent: a check that VERIFIES a Project was refused on import
/// naming NOTHING (dynograph-foundation's export, five such checks). The
/// narrowing gate found it on its first run; both the import and the detector
/// now say what the check is evidence for instead.
#[test]
fn a_refused_verifies_names_what_the_check_is_evidence_for_in_both_places() {
    let mut g = world();
    g.add_verification("ver:whole", "whole-design smoke", None, None, None)
        .expect("ver");
    let mut older = load_schema().expect("schema");
    older
        .edge_types
        .get_mut(edge::VERIFIES)
        .expect("VERIFIES")
        .to = EdgeEndpoint::Single(EdgeEndpoint::WILDCARD.to_string());
    g.write_under_schema(older, |g| {
        g.create_edge(
            edge::VERIFIES,
            node::VERIFICATION,
            "ver:whole",
            node::PROJECT,
            "proj:pump",
            Props::new(),
        )
    })
    .expect("older");
    let found = refused(&g);
    assert_eq!(found.len(), 1, "{found:#?}");
    let replacement = found[0]["refusal"]["replacement"]
        .as_str()
        .expect("a replacement is named")
        .to_string();
    assert!(
        replacement.contains("Requirement or Capability"),
        "{replacement}"
    );
    let doc = g.export_graph().expect("export");
    let err = DesignGraph::open_in_memory()
        .expect("open")
        .import_graph(&doc)
        .expect_err("refused");
    assert!(
        format!("{err}").contains(&replacement),
        "the import names the same replacement: {err}"
    );
}
