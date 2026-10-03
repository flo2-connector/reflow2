//! What a schema change TOOK AWAY, and the migration each one shipped.
//!
//! `dec:idea-stored-data-is-rechecked-against-the-current-schema` (accepted by
//! Anthony, 2026-10-02), piece 2: **a narrowing ships its migration.**
//!
//! # The class this closes
//!
//! A narrowing is a schema change after which the store's write rule refuses
//! something it used to accept: an edge type or node type removed, an endpoint
//! dropped from an edge (a `*` enumerated is the usual case), an enum value
//! removed, a property that became required with no default. Data written
//! BEFORE it is still in every store and every export, and the write rule is
//! checked only on write — so nothing judged that data until an import refused
//! the whole document. Each narrowing so far carried its own repair, by hand,
//! for whatever its author's census found
//! (`fact:root-cause-a-schema-narrowing-ships-its-migration-by-hand-per-pair-and-nothing-rechecks-a-store-2026-10-02`):
//!
//! - **VERIFIES, 2026-08-08.** The census missed two targets carrying one edge
//!   each, and dev_storyflow's committed export — the design that proposed the
//!   change — could not be re-imported for at least four days.
//! - **QualityGate, 2026-09-07.** Cutting the type without listing it as retired
//!   left the provenance guard telling operators their reflow2 was BEHIND.
//! - **REALIZES, 2026-09-23.** Done carefully — and still only for the pairs the
//!   census of the maintainer's machine had seen. musicjug's store held one it
//!   had not, found on 2026-10-01 when the move was attempted.
//!
//! # What this module holds
//!
//! - [`EDGE_REWRITES`] — every stored edge the import and every open REWRITE
//!   rather than refuse. One table, read by both doors and by the recheck, so
//!   "what the import does with an old edge" has one answer.
//! - [`named_replacement`] — the edge the import NAMES when it refuses one,
//!   read by the import's refusal and by `detect_defects`.
//! - [`NARROWINGS`] — every narrowing, with the migration it shipped and the
//!   record that says why. The provenance guard's retired types are read from
//!   here, so a retirement is one entry rather than two edits.
//! - [`Acceptance`] — what a schema accepts, as a reviewable snapshot. The
//!   committed one is the LAST RELEASE's (`schema/accepted-at-last-release.json`);
//!   `tests/a_schema_narrowing_ships_its_migration.rs` diffs today's schema
//!   against it and fails on any narrowing this table does not name.
//!
//! # What is deliberately NOT a narrowing here
//!
//! - **A newly declared OPTIONAL property.** A store could hold an undeclared
//!   property of that name with a value the declaration now refuses, but this
//!   project declares properties constantly, and treating each as a narrowing
//!   would bury the real ones. A newly declared property that is REQUIRED with
//!   no default IS one, because every stored node lacks it.
//! - **A property removed from a type.** The schema is additive: an undeclared
//!   property is accepted, so nothing stored becomes refusable.
//! - **A change of meaning under an unchanged name.** No structural diff can see
//!   it; the provenance module says the same about its stamp.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::foundation::core::{EdgeEndpoint, PropertyDef, PropertyType, Schema};
use crate::nodes::{edge, node};

// =============================================================================
// The two doors' shared rewrite and the import's named replacement
// =============================================================================

/// A stored edge the import and every open REWRITE rather than refuse — the
/// pair had a single right answer when its narrowing shipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeRewrite {
    pub edge_type: &'static str,
    pub from_type: &'static str,
    pub to_type: &'static str,
    /// The edge type it is written as instead. Its properties are dropped: the
    /// old edge's have no declaration on the new one.
    pub becomes: &'static str,
    /// Why, in the words the import's `migrated_edges` report uses.
    pub why: &'static str,
}

/// Every stored edge rewritten on open and on import. Read by
/// `DesignGraph::import_graph_with`, `DesignGraph::migrate_edge_rewrites` (on
/// every open) and the `refused_by_schema` detector, so the three cannot
/// disagree about what an old edge becomes.
pub const EDGE_REWRITES: &[EdgeRewrite] = &[EdgeRewrite {
    // 2026-09-23, when REALIZES stopped accepting any target: the one class of
    // the old wildcard's misuse with a single right answer. A file registered
    // against a check is that check's executable form.
    edge_type: edge::REALIZES,
    from_type: node::ARTIFACT,
    to_type: node::VERIFICATION,
    becomes: edge::IMPLEMENTS,
    why: "a file that is a check implements it",
}];

/// The rewrite for this edge, when it has one.
pub fn edge_rewrite_for(
    edge_type: &str,
    from_type: &str,
    to_type: &str,
) -> Option<&'static EdgeRewrite> {
    EDGE_REWRITES
        .iter()
        .find(|r| r.edge_type == edge_type && r.from_type == from_type && r.to_type == to_type)
}

/// The replacement the import NAMES when it refuses a stored edge, or `None`
/// when it knows none. Shared by the import's refusal and by `detect_defects`,
/// so both say the same thing about the same edge.
///
/// Each line is the edge a narrowing's own repair actually wrote for that
/// class — `crate::artifact::realizes_target_hint` says so for REALIZES. It is
/// a PROPOSAL: the 2026-10-01 field repair of a file REALIZING a Decision chose
/// GOVERNED_BY where this names DOCUMENTS. So nothing applies it.
pub fn named_replacement(edge_type: &str, _from_type: &str, to_type: &str) -> Option<String> {
    match edge_type {
        t if t == edge::REALIZES => crate::artifact::realizes_target_hint(to_type),
        t if t == edge::VERIFIES => verifies_target_hint(to_type),
        _ => None,
    }
}

/// For a target a check may NOT verify, what the check is evidence for
/// instead — `None` for the types it may.
///
/// ⭐ ADDED BY THE GATE ITSELF, 2026-10-03. Replaying the VERIFIES narrowing of
/// 2026-08-08 against `entry_problems` found eight dropped targets — Project
/// among them, which five checks in dynograph-foundation's export still VERIFY
/// (`fact:dynograph-foundations-export-will-not-import-because-five-checks-verify-its-project`)
/// — that the import refused naming NOTHING, and no edge the schema models
/// for the pair either. A refusal that names nothing is the VERIFIES
/// precedent this whole table exists to stop.
///
/// The list of what a check MAY verify is read from the schema, so this
/// sentence cannot fall behind the next widening.
fn verifies_target_hint(to_type: &str) -> Option<String> {
    let schema = crate::schema::parsed_schema().ok()?;
    let def = schema.edge_types.get(edge::VERIFIES)?;
    if def.to.accepts(to_type) {
        return None;
    }
    let may = endpoint_list(&def.to).join(", ");
    Some(match to_type {
        t if t == node::DECISION => {
            "a check on a ruling is GOVERNED_BY the Decision (governed_by) \
             rather than a VERIFIES of it"
                .to_string()
        }
        t if t == node::PROJECT => format!(
            "a check on the whole design is evidence for something in it — VERIFIES the \
             Requirement or Capability it actually checks (a check verifies only {may})"
        ),
        other => format!(
            "a check verifies only {may}; a check about a {other} is evidence for one of \
             those — VERIFIES the one it actually checks"
        ),
    })
}

/// The edge types the schema MODELS for `from_type -> to_type` — exact fits,
/// those that declare themselves for the pair, and those naming one side
/// exactly while open on the other by design — best first, as the write path's
/// refusal ranks them (`crate::vocabulary::edge_types_between_in_schema`).
/// Edges that accept the pair only through `*` on both sides are left out:
/// they tolerate it and say nothing about meaning it.
pub fn modelled_fits(
    schema: &Schema,
    edge_type: &str,
    from_type: &str,
    to_type: &str,
) -> Vec<String> {
    let Ok(q) = crate::vocabulary::edge_query_in(schema, from_type, to_type) else {
        return Vec::new();
    };
    q.matches
        .iter()
        .filter(|m| {
            m.is_exact()
                || m.declared_for_this_pair
                || m.from_match == crate::vocabulary::EndpointMatch::Exact
                || m.to_match == crate::vocabulary::EndpointMatch::Exact
        })
        .map(|m| m.spec.edge_type.clone())
        .filter(|t| t != edge_type)
        .collect()
}

// =============================================================================
// The table of narrowings
// =============================================================================

/// Which end of an edge a narrowing dropped a type from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    From,
    To,
}

impl Side {
    pub fn as_str(self) -> &'static str {
        match self {
            Side::From => "from",
            Side::To => "to",
        }
    }
}

/// Whether a property belongs to a node type or an edge type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    Node,
    Edge,
}

/// How a property's accepted values narrowed, when it was not an enum value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Facet {
    /// Its type changed to one that refuses a value the old type accepted.
    Type,
    /// It became required with no default — every stored item lacking it.
    Required,
    /// It stopped accepting null while required.
    Null,
    /// Its numeric range tightened, or appeared.
    Range,
}

/// What a narrowing took away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Narrowed {
    NodeType {
        node_type: &'static str,
    },
    EdgeType {
        edge_type: &'static str,
    },
    /// `node_type: "*"` stands for every type that side stopped accepting and
    /// no more specific entry names — the shape of a wildcard enumerated.
    Endpoint {
        edge_type: &'static str,
        side: Side,
        node_type: &'static str,
    },
    EnumValue {
        owner: Owner,
        type_name: &'static str,
        property: &'static str,
        value: &'static str,
    },
    Property {
        owner: Owner,
        type_name: &'static str,
        property: &'static str,
        facet: Facet,
    },
}

/// The migration a narrowing shipped — what happens to data written before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Migration {
    /// Opening a store and importing an export REWRITE it: an [`EDGE_REWRITES`]
    /// row, whose new edge the current schema accepts. Endpoints only — there
    /// is no rewrite table for values yet, and a narrowing that needs one
    /// builds it rather than claiming this.
    Rewritten,
    /// REFUSED BY NAME: the import refuses it saying what fits, and
    /// `detect_defects` (`refused_by_schema`) reports it on any store that still
    /// holds it — with the replacement, before the move that would fail. For an
    /// endpoint the gate checks a replacement or a modelled fit exists for
    /// every pair the narrowing dropped; a refusal that names nothing is the
    /// VERIFIES precedent, and is not this.
    RefusedByName,
    /// The type is gone. The provenance guard reads a stamp naming it as one to
    /// MIGRATE, not as a graph from the future (`crate::provenance`), and
    /// `detect_defects` reports a stored edge of a retired edge type.
    Retired,
}

/// One narrowing: what it took away, when, why, and what it shipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Narrowing {
    pub narrowed: Narrowed,
    /// The date the narrowing landed.
    pub on: &'static str,
    /// The design record that says why — a Decision or the finding behind it.
    pub record: &'static str,
    pub migration: Migration,
}

/// EVERY NARROWING THE SCHEMA HAS SHIPPED THAT A RECORD CAN NAME. A narrowing
/// relative to the last release that is missing from here fails
/// `tests/a_schema_narrowing_ships_its_migration.rs`, and so does an entry
/// whose migration does not do what it says.
///
/// The five below predate the gate and are written from their records.
/// Narrowings older than these, if any, are not reconstructed: the gate's
/// baseline is the last release, and inventing a history would be worse than
/// stating where it starts.
pub const NARROWINGS: &[Narrowing] = &[
    Narrowing {
        narrowed: Narrowed::EdgeType {
            edge_type: "ENABLES",
        },
        on: "2026-07-22",
        record: "dec:edge-orthogonality",
        migration: Migration::Retired,
    },
    Narrowing {
        narrowed: Narrowed::EdgeType {
            edge_type: "VALIDATES",
        },
        on: "2026-07-22",
        record: "dec:edge-orthogonality",
        migration: Migration::Retired,
    },
    Narrowing {
        narrowed: Narrowed::Endpoint {
            edge_type: edge::VERIFIES,
            side: Side::To,
            node_type: "*",
        },
        on: "2026-08-08",
        record: "fact:defect-a-schema-narrowing-orphaned-live-edges-and-no-export-is-ever-imported",
        migration: Migration::RefusedByName,
    },
    Narrowing {
        narrowed: Narrowed::NodeType {
            node_type: "QualityGate",
        },
        on: "2026-09-07",
        record: "dec:qualitygate-is-retired-the-phase-gate-dissolved-into-the-detectors",
        migration: Migration::Retired,
    },
    Narrowing {
        narrowed: Narrowed::Endpoint {
            edge_type: edge::REALIZES,
            side: Side::To,
            node_type: node::VERIFICATION,
        },
        on: "2026-09-23",
        record: "dec:realizes-is-restricted-to-capability-component-and-interface",
        migration: Migration::Rewritten,
    },
    Narrowing {
        narrowed: Narrowed::Endpoint {
            edge_type: edge::REALIZES,
            side: Side::To,
            node_type: "*",
        },
        on: "2026-09-23",
        record: "dec:realizes-is-restricted-to-capability-component-and-interface",
        migration: Migration::RefusedByName,
    },
];

/// Node types this reflow2 retired — read by the provenance guard, so a type
/// removed with its [`NARROWINGS`] entry is diagnosed as retired (migrate) and
/// never as a graph from the future. Until 2026-10-03 this was a second list,
/// and the QualityGate removal shipped with only one of the two edits.
pub fn retired_node_types() -> Vec<&'static str> {
    NARROWINGS
        .iter()
        .filter_map(|n| match n.narrowed {
            Narrowed::NodeType { node_type } => Some(node_type),
            _ => None,
        })
        .collect()
}

/// Edge types this reflow2 retired. See [`retired_node_types`].
pub fn retired_edge_types() -> Vec<&'static str> {
    NARROWINGS
        .iter()
        .filter_map(|n| match n.narrowed {
            Narrowed::EdgeType { edge_type } => Some(edge_type),
            _ => None,
        })
        .collect()
}

// =============================================================================
// What a schema accepts, as a reviewable snapshot
// =============================================================================

/// What one property accepts — only the facets a narrowing can move.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyAcceptance {
    #[serde(rename = "type")]
    pub prop_type: String,
    /// Enum values, sorted. Absent for a non-enum, and for an enum declaring
    /// no list (which accepts any string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    /// Required with no default: a stored item lacking it is refused.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub required: bool,
    /// Required and not nullable: a stored null is refused.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub refuses_null: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<[f64; 2]>,
}

impl PropertyAcceptance {
    fn of(def: &PropertyDef) -> Self {
        Self {
            prop_type: type_name(&def.prop_type).to_string(),
            values: def.values.as_ref().map(|v| {
                let mut v = v.clone();
                v.sort();
                v
            }),
            required: def.required && def.default.is_none(),
            refuses_null: def.required && !def.nullable,
            range: def.range.map(|(a, b)| [a, b]),
        }
    }
}

fn type_name(t: &PropertyType) -> &'static str {
    match t {
        PropertyType::String => "string",
        PropertyType::Int => "int",
        PropertyType::Float => "float",
        PropertyType::Bool => "bool",
        PropertyType::Datetime => "datetime",
        PropertyType::Enum => "enum",
        PropertyType::ListString => "list:string",
    }
}

/// Does a property of type `new` accept every value one of type `old` did?
/// The write path widens an exact integer into a float property, and an enum's
/// values are strings.
fn type_accepts(old: &str, new: &str) -> bool {
    old == new || (old == "enum" && new == "string") || (old == "int" && new == "float")
}

/// What one edge type accepts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeAcceptance {
    /// Sorted; `["*"]` for a wildcard end.
    pub from: Vec<String>,
    pub to: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, PropertyAcceptance>,
}

/// Everything a schema ACCEPTS, in a form a person can review in a diff and a
/// test can compare — node types and their properties, edge types with their
/// endpoints and properties. Everything sorted, so the file is stable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Acceptance {
    /// The reflow2 whose schema this is.
    pub reflow2_version: String,
    pub node_types: BTreeMap<String, BTreeMap<String, PropertyAcceptance>>,
    pub edge_types: BTreeMap<String, EdgeAcceptance>,
}

fn endpoint_list(e: &EdgeEndpoint) -> Vec<String> {
    let mut v: Vec<String> = match e {
        EdgeEndpoint::Single(t) => vec![t.clone()],
        EdgeEndpoint::Multiple(ts) => ts.clone(),
    };
    if v.iter().any(|t| t == EdgeEndpoint::WILDCARD) {
        return vec![EdgeEndpoint::WILDCARD.to_string()];
    }
    v.sort();
    v.dedup();
    v
}

impl Acceptance {
    /// What `schema` accepts, stamped with this binary's version.
    pub fn of(schema: &Schema) -> Self {
        let props = |p: &std::collections::HashMap<String, PropertyDef>| {
            p.iter()
                .map(|(k, d)| (k.clone(), PropertyAcceptance::of(d)))
                .collect::<BTreeMap<_, _>>()
        };
        Self {
            reflow2_version: env!("CARGO_PKG_VERSION").to_string(),
            node_types: schema
                .node_types
                .iter()
                .map(|(k, d)| (k.clone(), props(&d.properties)))
                .collect(),
            edge_types: schema
                .edge_types
                .iter()
                .map(|(k, d)| {
                    (
                        k.clone(),
                        EdgeAcceptance {
                            from: endpoint_list(&d.from),
                            to: endpoint_list(&d.to),
                            properties: props(&d.properties),
                        },
                    )
                })
                .collect(),
        }
    }

    /// The node types an endpoint list accepts, against this snapshot's types.
    fn accepted(&self, end: &[String]) -> BTreeSet<String> {
        if end.iter().any(|t| t == EdgeEndpoint::WILDCARD) {
            self.node_types.keys().cloned().collect()
        } else {
            end.iter().cloned().collect()
        }
    }

    /// Every narrowing from `self` (the older) to `now` — what `self` accepted
    /// and `now` refuses. Sorted, so a failure lists them stably. A widening
    /// contributes nothing.
    pub fn narrowings_to(&self, now: &Acceptance) -> Vec<FoundNarrowing> {
        let mut out = Vec::new();
        for t in self.node_types.keys() {
            if !now.node_types.contains_key(t) {
                out.push(FoundNarrowing::NodeType {
                    node_type: t.clone(),
                });
            }
        }
        for (e, was) in &self.edge_types {
            let Some(is) = now.edge_types.get(e) else {
                out.push(FoundNarrowing::EdgeType {
                    edge_type: e.clone(),
                });
                continue;
            };
            for (side, old_end, new_end) in [
                (Side::From, &was.from, &is.from),
                (Side::To, &was.to, &is.to),
            ] {
                let kept = now.accepted(new_end);
                for t in self.accepted(old_end) {
                    // A type gone from the schema altogether is ITS narrowing,
                    // reported once above, not once per edge that named it.
                    if !kept.contains(&t) && now.node_types.contains_key(&t) {
                        out.push(FoundNarrowing::Endpoint {
                            edge_type: e.clone(),
                            side,
                            node_type: t,
                        });
                    }
                }
            }
            property_narrowings(Owner::Edge, e, &was.properties, &is.properties, &mut out);
        }
        for (t, was) in &self.node_types {
            if let Some(is) = now.node_types.get(t) {
                property_narrowings(Owner::Node, t, was, is, &mut out);
            }
        }
        out.sort();
        out
    }
}

fn property_narrowings(
    owner: Owner,
    type_name: &str,
    was: &BTreeMap<String, PropertyAcceptance>,
    is: &BTreeMap<String, PropertyAcceptance>,
    out: &mut Vec<FoundNarrowing>,
) {
    let mut facet = |property: &str, facet: Facet| {
        out.push(FoundNarrowing::Property {
            owner,
            type_name: type_name.to_string(),
            property: property.to_string(),
            facet,
        })
    };
    for (p, now) in is {
        let Some(old) = was.get(p) else {
            // Newly declared: only a requirement is a narrowing (module docs).
            if now.required {
                facet(p, Facet::Required);
            }
            continue;
        };
        if !type_accepts(&old.prop_type, &now.prop_type) {
            facet(p, Facet::Type);
        }
        if now.required && !old.required {
            facet(p, Facet::Required);
        }
        if now.refuses_null && !old.refuses_null {
            facet(p, Facet::Null);
        }
        let tighter = match (old.range, now.range) {
            (_, None) => false,
            (None, Some(_)) => true,
            (Some([a, b]), Some([c, d])) => c > a || d < b,
        };
        if tighter {
            facet(p, Facet::Range);
        }
    }
    for (p, old) in was {
        let (Some(old_values), Some(now)) = (&old.values, is.get(p)) else {
            continue;
        };
        // A list that became unrestricted, or a type no longer an enum that
        // accepts strings, removed nothing; a type change is reported above.
        let Some(now_values) = &now.values else {
            continue;
        };
        for v in old_values.iter().filter(|v| !now_values.contains(v)) {
            out.push(FoundNarrowing::EnumValue {
                owner,
                type_name: type_name.to_string(),
                property: p.clone(),
                value: v.clone(),
            });
        }
    }
}

/// A narrowing found by diffing two snapshots — the owned twin of
/// [`Narrowed`], which names a table entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FoundNarrowing {
    NodeType {
        node_type: String,
    },
    EdgeType {
        edge_type: String,
    },
    Endpoint {
        edge_type: String,
        side: Side,
        node_type: String,
    },
    EnumValue {
        owner: Owner,
        type_name: String,
        property: String,
        value: String,
    },
    Property {
        owner: Owner,
        type_name: String,
        property: String,
        facet: Facet,
    },
}

impl FoundNarrowing {
    /// Whether a table entry accounts for this narrowing. An endpoint entry
    /// naming `"*"` covers every type that side dropped.
    pub fn is_covered_by(&self, n: &Narrowed) -> bool {
        match (self, n) {
            (Self::NodeType { node_type: a }, Narrowed::NodeType { node_type: b }) => a == b,
            (Self::EdgeType { edge_type: a }, Narrowed::EdgeType { edge_type: b }) => a == b,
            (
                Self::Endpoint {
                    edge_type,
                    side,
                    node_type,
                },
                Narrowed::Endpoint {
                    edge_type: e,
                    side: s,
                    node_type: t,
                },
            ) => edge_type == e && side == s && (node_type == t || *t == "*"),
            (
                Self::EnumValue {
                    owner,
                    type_name,
                    property,
                    value,
                },
                Narrowed::EnumValue {
                    owner: o,
                    type_name: t,
                    property: p,
                    value: v,
                },
            ) => owner == o && type_name == t && property == p && value == v,
            (
                Self::Property {
                    owner,
                    type_name,
                    property,
                    facet,
                },
                Narrowed::Property {
                    owner: o,
                    type_name: t,
                    property: p,
                    facet: f,
                },
            ) => owner == o && type_name == t && property == p && facet == f,
            _ => false,
        }
    }
}

/// Narrowings no [`NARROWINGS`] entry accounts for.
pub fn uncovered(found: &[FoundNarrowing]) -> Vec<FoundNarrowing> {
    found
        .iter()
        .filter(|f| !NARROWINGS.iter().any(|n| f.is_covered_by(&n.narrowed)))
        .cloned()
        .collect()
}

/// Every way a [`NARROWINGS`] entry fails to hold against `schema`: an entry
/// whose narrowing is not in effect (it was reverted, or never happened), a
/// migration the kind of narrowing cannot have, or a migration that does not
/// do what it claims. Empty means every entry is true today.
pub fn entry_problems(schema: &Schema) -> Vec<String> {
    let mut problems = Vec::new();
    for n in NARROWINGS {
        let label = format!("{:?} ({}, {})", n.narrowed, n.on, n.record);
        if n.record.trim().is_empty() || !n.record.contains(':') {
            problems.push(format!("{label}: names no record that says why"));
        }
        match (n.narrowed, n.migration) {
            (Narrowed::NodeType { node_type }, Migration::Retired) => {
                if schema.node_types.contains_key(node_type) {
                    problems.push(format!("{label}: the schema still declares {node_type}"));
                }
            }
            (Narrowed::EdgeType { edge_type }, Migration::Retired) => {
                if schema.edge_types.contains_key(edge_type) {
                    problems.push(format!("{label}: the schema still declares {edge_type}"));
                }
            }
            (Narrowed::NodeType { .. } | Narrowed::EdgeType { .. }, m) => problems.push(format!(
                "{label}: a removed TYPE is `Retired`, not {m:?} — the provenance guard reads \
                 retired types from this table"
            )),
            (Narrowed::Endpoint { .. }, Migration::Retired) => problems.push(format!(
                "{label}: an endpoint is `Rewritten` or `RefusedByName`, never `Retired`"
            )),
            (
                Narrowed::Endpoint {
                    edge_type,
                    side,
                    node_type,
                },
                m,
            ) => endpoint_problems(schema, &label, edge_type, side, node_type, m, &mut problems),
            (Narrowed::EnumValue { .. } | Narrowed::Property { .. }, Migration::Rewritten) => {
                problems.push(format!(
                    "{label}: nothing rewrites a stored VALUE yet — there is no value table \
                     beside EDGE_REWRITES. Build one (open + import, reported like \
                     `migrated_edges`) before claiming this, or ship it `RefusedByName`"
                ))
            }
            (Narrowed::EnumValue { .. } | Narrowed::Property { .. }, Migration::Retired) => {
                problems.push(format!("{label}: only a removed type is `Retired`"))
            }
            (
                Narrowed::EnumValue {
                    owner,
                    type_name,
                    property,
                    value,
                },
                Migration::RefusedByName,
            ) => match declared_property(schema, owner, type_name, property) {
                Some(def)
                    if def
                        .values
                        .as_ref()
                        .is_some_and(|v| !v.iter().any(|x| x == value)) => {}
                _ => problems.push(format!(
                    "{label}: {type_name}.{property} accepts `{value}` today, so this narrowing \
                     is not in effect"
                )),
            },
            (
                Narrowed::Property {
                    owner,
                    type_name,
                    property,
                    facet,
                },
                Migration::RefusedByName,
            ) => {
                let in_effect =
                    declared_property(schema, owner, type_name, property).is_some_and(|d| {
                        match facet {
                            Facet::Required => d.required && d.default.is_none(),
                            Facet::Null => d.required && !d.nullable,
                            Facet::Range => d.range.is_some(),
                            Facet::Type => true,
                        }
                    });
                if !in_effect {
                    problems.push(format!(
                        "{label}: {type_name}.{property} does not narrow that way today"
                    ));
                }
            }
        }
    }
    problems
}

fn declared_property<'a>(
    schema: &'a Schema,
    owner: Owner,
    type_name: &str,
    property: &str,
) -> Option<&'a PropertyDef> {
    match owner {
        Owner::Node => schema.node_types.get(type_name)?.properties.get(property),
        Owner::Edge => schema.edge_types.get(type_name)?.properties.get(property),
    }
}

fn endpoint_types(schema: &Schema, e: &EdgeEndpoint) -> Vec<String> {
    let mut v: Vec<String> = schema
        .node_types
        .keys()
        .filter(|t| e.accepts(t))
        .cloned()
        .collect();
    v.sort();
    v
}

/// The pairs an endpoint entry stands for today, and what each must have.
fn endpoint_problems(
    schema: &Schema,
    label: &str,
    edge_type: &str,
    side: Side,
    node_type: &str,
    migration: Migration,
    problems: &mut Vec<String>,
) {
    let Some(def) = schema.edge_types.get(edge_type) else {
        problems.push(format!(
            "{label}: the schema no longer declares {edge_type} — retire the edge type instead"
        ));
        return;
    };
    let (narrowed_end, other_end) = match side {
        Side::From => (&def.from, &def.to),
        Side::To => (&def.to, &def.from),
    };
    // The dropped types this entry speaks for: one, or every type the end now
    // refuses that no more specific entry names.
    let dropped: Vec<String> = if node_type == "*" {
        if endpoint_list(narrowed_end) == [EdgeEndpoint::WILDCARD] {
            problems.push(format!(
                "{label}: {edge_type}.{} is still `*`, so nothing was dropped",
                side.as_str()
            ));
            return;
        }
        let specific: BTreeSet<&str> = NARROWINGS
            .iter()
            .filter_map(|n| match n.narrowed {
                Narrowed::Endpoint {
                    edge_type: e,
                    side: s,
                    node_type: t,
                } if e == edge_type && s == side && t != "*" => Some(t),
                _ => None,
            })
            .collect();
        schema
            .node_types
            .keys()
            .filter(|t| !narrowed_end.accepts(t) && !specific.contains(t.as_str()))
            .cloned()
            .collect()
    } else {
        if !schema.node_types.contains_key(node_type) {
            problems.push(format!("{label}: {node_type} is not a node type today"));
            return;
        }
        if narrowed_end.accepts(node_type) {
            problems.push(format!(
                "{label}: {edge_type}.{} accepts {node_type} today, so this narrowing is not in \
                 effect",
                side.as_str()
            ));
            return;
        }
        vec![node_type.to_string()]
    };
    for dropped_type in dropped {
        for other in endpoint_types(schema, other_end) {
            let (from, to) = match side {
                Side::From => (dropped_type.as_str(), other.as_str()),
                Side::To => (other.as_str(), dropped_type.as_str()),
            };
            match migration {
                Migration::Rewritten => match edge_rewrite_for(edge_type, from, to) {
                    Some(r) => {
                        if schema.validate_edge(r.becomes, from, to).is_err() {
                            problems.push(format!(
                                "{label}: {edge_type} {from} -> {to} is rewritten to {}, which \
                                 the schema refuses for that pair",
                                r.becomes
                            ));
                        }
                    }
                    None => problems.push(format!(
                        "{label}: no EDGE_REWRITES row rewrites {edge_type} {from} -> {to}"
                    )),
                },
                Migration::RefusedByName => {
                    if named_replacement(edge_type, from, to).is_none()
                        && modelled_fits(schema, edge_type, from, to).is_empty()
                    {
                        problems.push(format!(
                            "{label}: {edge_type} {from} -> {to} is refused naming NOTHING — no \
                             replacement in `named_replacement` and no edge the schema models \
                             for the pair. Name what fits, or rewrite it"
                        ));
                    }
                }
                Migration::Retired => unreachable!("refused above"),
            }
        }
    }
}

// =============================================================================
// The recheck: what the import would refuse, asked of the store as it stands
// =============================================================================

/// The rule that refused a stored item, in a word a reader can act on.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RefusedBy {
    /// `endpoint_pair` (the edge type does not accept these two node types),
    /// `edge_type` (the edge type is not declared at all), `property` (a value,
    /// or a required property's absence), `node_type`.
    pub rule: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
    /// The refusal exactly as the write path — and so the import — words it.
    pub refusal: String,
}

impl RefusedBy {
    fn from_error(e: &crate::foundation::core::DynoError) -> Self {
        use crate::foundation::core::DynoError as E;
        let (rule, property) = match e {
            E::InvalidEdge { .. } => ("endpoint_pair", None),
            E::UnknownEdgeType(_) => ("edge_type", None),
            E::UnknownNodeType(_) => ("node_type", None),
            E::Validation { property, .. } | E::EdgeValidation { property, .. } => {
                ("property", Some(property.clone()))
            }
            _ => ("other", None),
        };
        Self {
            rule,
            property,
            refusal: e.to_string(),
        }
    }
}

/// Which stored item a refusal is about.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "item", rename_all = "snake_case")]
pub enum RefusedItem {
    Node {
        node_type: String,
        node_id: String,
    },
    Edge {
        edge_type: String,
        from_type: String,
        from_id: String,
        to_type: String,
        to_id: String,
    },
}

/// One stored node or edge the current schema refuses — what the import would
/// refuse, found before the import is attempted.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StoredRefusal {
    #[serde(flatten)]
    pub item: RefusedItem,
    /// Every rule that refuses it, not only the first: one repair, not one per
    /// import attempt (BL-118's lesson, at the store).
    pub refused_by: Vec<RefusedBy>,
    /// The replacement the import names for this edge ([`named_replacement`]).
    /// A proposal — never applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
    /// The edge types the schema models for the pair ([`modelled_fits`]), for a
    /// refused endpoint pair. A list to choose from, not a choice.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub modelled_fits: Vec<String>,
}

/// What a recheck walked and what it found.
#[derive(Debug, Clone, Default)]
pub struct StoredRecheck {
    pub refusals: Vec<StoredRefusal>,
    pub examined_nodes: usize,
    pub examined_edges: usize,
}

/// The edges a recheck judges — exactly those an export carries and its import
/// would write: both endpoints are nodes of a declared type. An edge onto a
/// node that is not there is the import's `skipped_edges`, not a refusal.
/// Shared by the recheck and its population count, so the count is of what
/// the rule actually walked.
pub(crate) fn rechecked_edges<'a>(
    adjacency: &'a crate::graph::Adjacency,
    index: &'a std::collections::HashMap<String, String>,
) -> impl Iterator<Item = (&'a crate::foundation::store::StoredEdge, &'a str, &'a str)> + 'a {
    adjacency.out.values().flatten().filter_map(move |e| {
        let ft = index.get(&e.from_id)?;
        let tt = index.get(&e.to_id)?;
        Some((e, ft.as_str(), tt.as_str()))
    })
}

impl crate::graph::DesignGraph {
    /// Every stored node and edge the current schema refuses, judged by THE
    /// RULE THE STORE'S WRITE POINT APPLIES (`Schema::node_refusals`,
    /// `Schema::edge_refusals`) after the same preparation the import gives an
    /// item: integers widened into float properties, a legacy AUTHORED_BY
    /// normalised, an [`EDGE_REWRITES`] edge rewritten. So what this reports is
    /// what an import of this store's export would refuse — no more, no less —
    /// and `tests/a_stored_item_the_schema_now_refuses_is_reported.rs` holds the
    /// two to that.
    pub(crate) fn recheck_stored_items(
        &self,
        index: &std::collections::HashMap<String, String>,
    ) -> Result<StoredRecheck, crate::foundation::core::DynoError> {
        let schema = self.schema();
        let mut out = StoredRecheck::default();

        let mut ids: Vec<(&String, &String)> = index.iter().collect();
        ids.sort();
        for (node_id, node_type) in ids {
            let Some(n) = self.get_node(node_type, node_id)? else {
                continue;
            };
            out.examined_nodes += 1;
            let mut props = n.properties.clone();
            if let Some(def) = schema.node_types.get(node_type.as_str()) {
                crate::graph::widen_ints_for_float_props(&def.properties, &mut props);
            }
            let refusals = schema.node_refusals(node_type, &props);
            if refusals.is_empty() {
                continue;
            }
            out.refusals.push(StoredRefusal {
                item: RefusedItem::Node {
                    node_type: node_type.clone(),
                    node_id: node_id.clone(),
                },
                refused_by: refusals.iter().map(RefusedBy::from_error).collect(),
                replacement: None,
                modelled_fits: Vec::new(),
            });
        }

        let adjacency = self.adjacency()?;
        let mut edges: Vec<_> = rechecked_edges(&adjacency, index).collect();
        edges.sort_by(|a, b| {
            (&a.0.edge_type, &a.0.from_id, &a.0.to_id).cmp(&(
                &b.0.edge_type,
                &b.0.from_id,
                &b.0.to_id,
            ))
        });
        for (e, ft, tt) in edges {
            out.examined_edges += 1;
            let mut props = e.properties.clone();
            if e.edge_type == crate::nodes::edge::AUTHORED_BY {
                crate::graph::normalize_authored_by_props(&mut props);
            }
            let mut edge_type = e.edge_type.as_str();
            if let Some(r) = edge_rewrite_for(edge_type, ft, tt) {
                edge_type = r.becomes;
                props.clear();
            }
            if let Some(def) = schema.edge_types.get(edge_type) {
                crate::graph::widen_ints_for_float_props(&def.properties, &mut props);
            }
            let refusals = schema.edge_refusals(edge_type, ft, tt, &props);
            if refusals.is_empty() {
                continue;
            }
            let pair_refused = refusals
                .iter()
                .any(|r| matches!(r, crate::foundation::core::DynoError::InvalidEdge { .. }));
            out.refusals.push(StoredRefusal {
                item: RefusedItem::Edge {
                    edge_type: e.edge_type.clone(),
                    from_type: ft.to_string(),
                    from_id: e.from_id.clone(),
                    to_type: tt.to_string(),
                    to_id: e.to_id.clone(),
                },
                refused_by: refusals.iter().map(RefusedBy::from_error).collect(),
                replacement: named_replacement(&e.edge_type, ft, tt),
                modelled_fits: if pair_refused || !schema.edge_types.contains_key(edge_type) {
                    modelled_fits(schema, edge_type, ft, tt)
                } else {
                    Vec::new()
                },
            });
        }
        Ok(out)
    }

    /// How many stored items a recheck judges — the population
    /// `refused_by_schema` walks, from the same enumeration the recheck uses.
    pub(crate) fn recheck_population(
        &self,
        index: &std::collections::HashMap<String, String>,
    ) -> Result<usize, crate::foundation::core::DynoError> {
        let adjacency = self.adjacency()?;
        Ok(index.len() + rechecked_edges(&adjacency, index).count())
    }
}
