//! A schema change that takes something away ships its migration, or the build
//! fails.
//!
//! `dec:idea-stored-data-is-rechecked-against-the-current-schema`, accepted by
//! Anthony 2026-10-02, piece 2: "a gate that a schema narrowing ships its
//! migration" — option (d), "CI diffs the schema's edge endpoint lists against
//! the last release".
//!
//! # How it works
//!
//! `schema/accepted-at-last-release.json` is what the LAST RELEASE's schema
//! accepted (`narrowing::Acceptance`). Today's schema is diffed against it, and
//! every narrowing — an endpoint dropped, an enum value removed, a type
//! retired, a property newly required with no default, a type or range
//! tightened — must be named in `narrowing::NARROWINGS` with the migration it
//! shipped. Every entry there is then checked against today's schema: in
//! effect, and its migration doing what it says (a rewrite the import and the
//! open both perform onto an edge the schema accepts; a refusal that names a
//! replacement or a modelled fit for every pair it dropped).
//!
//! # Why the last RELEASE and not the last commit
//!
//! What reaches a consumer's store is what a release shipped. Something added
//! and removed between two releases never left this repository, and a snapshot
//! that had to move with every additive schema edit would make every such PR
//! re-bless a file for nothing. So the snapshot carries the version it was
//! taken at, and moves at a cut: the cut bumps the version, this test then
//! fails until the snapshot is re-blessed, and the bless itself refuses while
//! any narrowing is unaccounted for.
//!
//! # To re-bless (at a cut, and only then)
//!
//! ```text
//! REFLOW2_BLESS_ACCEPTANCE=1 cargo test -p reflow2-core --no-default-features \
//!     --test a_schema_narrowing_ships_its_migration
//! ```

use std::path::PathBuf;

use reflow2_core::foundation::core::{EdgeEndpoint, PropertyDef, PropertyType, Schema};
use reflow2_core::narrowing::{
    Acceptance, FoundNarrowing, Owner, Side, entry_problems, retired_edge_types,
    retired_node_types, uncovered,
};
use reflow2_core::nodes::{edge, node};
use reflow2_core::schema::load_schema;

const BLESS: &str = "REFLOW2_BLESS_ACCEPTANCE";

fn snapshot_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schema/accepted-at-last-release.json")
}

fn write_snapshot(a: &Acceptance) {
    let mut text = serde_json::to_string_pretty(a).expect("serialise");
    text.push('\n');
    std::fs::write(snapshot_path(), text).expect("write the snapshot");
}

/// THE GATE.
#[test]
fn every_narrowing_since_the_last_release_ships_its_migration() {
    let now = Acceptance::of(&load_schema().expect("schema"));
    let bless = std::env::var_os(BLESS).is_some();
    let Ok(text) = std::fs::read_to_string(snapshot_path()) else {
        assert!(
            bless,
            "no {} — create it from the schema the last release shipped with \
             {BLESS}=1 (see this file's header)",
            snapshot_path().display()
        );
        write_snapshot(&now);
        return;
    };
    let was: Acceptance = serde_json::from_str(&text).expect("the snapshot parses");
    let found = was.narrowings_to(&now);
    let missing = uncovered(&found);
    assert!(
        missing.is_empty(),
        "THE SCHEMA TOOK SOMETHING AWAY THAT reflow2 {} ACCEPTED, AND NOTHING SAYS WHAT \
         HAPPENS TO DATA WRITTEN BEFORE IT. Every store and export made under that release can \
         hold it; the import refuses the whole document on the first one. For each item below, \
         add a `Narrowing` to `narrowing::NARROWINGS` naming its migration — `Rewritten` (an \
         `EDGE_REWRITES` row the import and every open apply), `RefusedByName` (the import and \
         `detect_defects` name what fits — add the replacement to `named_replacement` if the \
         schema models none), or `Retired` for a removed type — and the record that says why.\n  \
         {}",
        was.reflow2_version,
        missing
            .iter()
            .map(|m| serde_json::to_string(m).expect("json"))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
    if bless {
        assert_ne!(
            was.reflow2_version, now.reflow2_version,
            "the snapshot is already {}'s. It is the LAST RELEASE's, and moves at a cut — when \
             the version moves — not with each schema edit",
            now.reflow2_version
        );
        write_snapshot(&now);
        return;
    }
    assert_eq!(
        was.reflow2_version,
        env!("CARGO_PKG_VERSION"),
        "the version moved, so this is a cut: re-bless the snapshot as this release's with \
         {BLESS}=1 (see this file's header). The bless refuses while any narrowing since {} is \
         unaccounted for.",
        was.reflow2_version
    );
}

/// Every entry is true of today's schema, and its migration does what it says.
#[test]
fn every_narrowing_entry_holds_today() {
    let problems = entry_problems(&load_schema().expect("schema"));
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The provenance guard's retired types ARE the table's — one list. The
/// QualityGate removal shipped with only one of the two edits it needed.
#[test]
fn the_provenance_guards_retired_types_are_the_tables() {
    assert_eq!(retired_node_types(), ["QualityGate"]);
    let mut edges = retired_edge_types();
    edges.sort();
    assert_eq!(edges, ["ENABLES", "VALIDATES"]);
}

// ---- the diff itself, on schemas narrowed on purpose -------------------------

fn wildcard() -> EdgeEndpoint {
    EdgeEndpoint::Single(EdgeEndpoint::WILDCARD.to_string())
}

fn diff(was: &Schema, now: &Schema) -> Vec<FoundNarrowing> {
    Acceptance::of(was).narrowings_to(&Acceptance::of(now))
}

/// The REALIZES narrowing of 2026-09-23, replayed: every target it dropped is a
/// narrowing, the table accounts for all of them, and Verification — the one
/// with a single right answer — is the rewritten one.
#[test]
fn the_realizes_narrowing_replayed_is_found_whole_and_accounted_for() {
    let now = load_schema().expect("schema");
    let mut was = now.clone();
    was.edge_types.get_mut(edge::REALIZES).expect("REALIZES").to = wildcard();
    let found = diff(&was, &now);
    let dropped: Vec<&str> = found
        .iter()
        .filter_map(|f| match f {
            FoundNarrowing::Endpoint {
                edge_type,
                side: Side::To,
                node_type,
            } if edge_type == edge::REALIZES => Some(node_type.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(dropped.len(), found.len(), "only REALIZES moved: {found:?}");
    assert_eq!(
        dropped.len(),
        now.node_types.len() - 3,
        "every type but Capability, Component and Interface"
    );
    assert!(dropped.contains(&node::DECISION) && dropped.contains(&node::VERIFICATION));
    assert!(uncovered(&found).is_empty(), "{:?}", uncovered(&found));
}

/// The VERIFIES narrowing of 2026-08-08 — the one whose census missed a long
/// tail and left a design unimportable for days — replayed against the table.
#[test]
fn the_verifies_narrowing_replayed_is_accounted_for() {
    let now = load_schema().expect("schema");
    let mut was = now.clone();
    was.edge_types.get_mut(edge::VERIFIES).expect("VERIFIES").to = wildcard();
    let found = diff(&was, &now);
    assert!(!found.is_empty());
    assert!(uncovered(&found).is_empty(), "{:?}", uncovered(&found));
}

/// A narrowing the table does NOT name is reported, which is the whole gate:
/// an endpoint dropped from an edge nobody has narrowed before.
#[test]
fn an_unrecorded_endpoint_narrowing_is_uncovered() {
    let was = load_schema().expect("schema");
    let mut now = was.clone();
    let changed = now.edge_types.get_mut(edge::CHANGED).expect("CHANGED");
    changed.to = EdgeEndpoint::Multiple(vec![node::ARTIFACT.into(), node::CAPABILITY.into()]);
    let found = diff(&was, &now);
    assert!(
        found.iter().any(|f| matches!(f,
            FoundNarrowing::Endpoint { edge_type, side: Side::To, node_type }
                if edge_type == edge::CHANGED && node_type == node::DECISION)),
        "{found:?}"
    );
    assert_eq!(uncovered(&found).len(), found.len(), "nothing names it yet");
}

#[test]
fn an_enum_value_removed_is_a_narrowing() {
    let now = load_schema().expect("schema");
    let mut was = now.clone();
    was.node_types
        .get_mut(node::DECISION)
        .expect("Decision")
        .properties
        .get_mut("status")
        .expect("status")
        .values
        .as_mut()
        .expect("enum")
        .push("obsolete".into());
    assert_eq!(
        diff(&was, &now),
        [FoundNarrowing::EnumValue {
            owner: Owner::Node,
            type_name: node::DECISION.into(),
            property: "status".into(),
            value: "obsolete".into(),
        }]
    );
}

#[test]
fn a_property_newly_required_with_no_default_is_a_narrowing() {
    let was = load_schema().expect("schema");
    let mut now = was.clone();
    now.node_types
        .get_mut(node::COMPONENT)
        .expect("Component")
        .properties
        .insert(
            "owner_team".into(),
            PropertyDef {
                prop_type: PropertyType::String,
                required: true,
                ..Default::default()
            },
        );
    let found = diff(&was, &now);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        matches!(&found[0], FoundNarrowing::Property { property, .. } if property == "owner_team")
    );
}

#[test]
fn a_type_change_and_a_tighter_range_are_narrowings() {
    let was = load_schema().expect("schema");
    let mut now = was.clone();
    let props = &mut now
        .node_types
        .get_mut(node::REQUIREMENT)
        .expect("Requirement")
        .properties;
    // A string that became an int refuses every stored statement.
    props.insert(
        "statement".into(),
        PropertyDef {
            prop_type: PropertyType::Int,
            required: true,
            ..Default::default()
        },
    );
    let found = diff(&was, &now);
    assert!(
        found.iter().any(
            |f| matches!(f, FoundNarrowing::Property { property, .. } if property == "statement")
        ),
        "{found:?}"
    );
    let mut ranged_was = was.clone();
    let mut ranged_now = was.clone();
    for (s, (lo, hi)) in [(&mut ranged_was, (0.0, 1.0)), (&mut ranged_now, (0.0, 0.5))] {
        s.node_types
            .get_mut(node::REQUIREMENT)
            .expect("Requirement")
            .properties
            .insert(
                "weight".into(),
                PropertyDef {
                    prop_type: PropertyType::Float,
                    range: Some((lo, hi)),
                    ..Default::default()
                },
            );
    }
    assert_eq!(diff(&ranged_was, &ranged_now).len(), 1);
}

/// What the gate must NOT fire on: anything that accepts more than before.
#[test]
fn a_widening_is_not_a_narrowing() {
    let was = load_schema().expect("schema");
    let mut now = was.clone();
    now.edge_types.get_mut(edge::REALIZES).expect("REALIZES").to = wildcard();
    now.node_types
        .get_mut(node::DECISION)
        .expect("Decision")
        .properties
        .get_mut("status")
        .expect("status")
        .values
        .as_mut()
        .expect("enum")
        .push("brand-new".into());
    now.node_types
        .get_mut(node::COMPONENT)
        .expect("Component")
        .properties
        .insert("optional_note".into(), PropertyDef::default());
    now.node_types
        .get_mut(node::COMPONENT)
        .expect("Component")
        .properties
        .get_mut("purpose")
        .expect("purpose")
        .required = false;
    assert!(diff(&was, &now).is_empty(), "{:?}", diff(&was, &now));
}

/// A removed type is reported ONCE, as the type — not once more for every edge
/// that named it — and the retired types are accounted for.
#[test]
fn a_retired_type_is_one_narrowing_and_the_table_names_it() {
    let now = load_schema().expect("schema");
    let mut was = now.clone();
    was.node_types
        .insert("QualityGate".into(), Default::default());
    was.edge_types.insert(
        "VALIDATES".into(),
        reflow2_core::foundation::core::EdgeTypeDef {
            from: EdgeEndpoint::Single("QualityGate".into()),
            to: wildcard(),
            ..Default::default()
        },
    );
    let found = diff(&was, &now);
    assert_eq!(
        found,
        [
            FoundNarrowing::NodeType {
                node_type: "QualityGate".into()
            },
            FoundNarrowing::EdgeType {
                edge_type: "VALIDATES".into()
            },
        ]
    );
    assert!(uncovered(&found).is_empty());
}
