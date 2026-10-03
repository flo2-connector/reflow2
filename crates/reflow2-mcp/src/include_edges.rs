//! `get_node`'s `include_edges` parameter: `true`, or a filter.
//!
//! The reader itself is `reflow2_core::node_edges`; this is only the shape a
//! caller writes and the schema `tools/list` publishes for it
//! (`dec:idea-an-edge-reader-returns-one-nodes-edges-and-find-tools-finds-it`).
//!
//! # Why one parameter with two shapes, and how it stays in the checked subset
//!
//! The decision put the reader ON `get_node` rather than in a new tool,
//! because the surface is budgeted (`req:the-mcp-surface-is-sized-for-tokens-per-task`).
//! So the common ask is one word, `"include_edges": true`, and the narrower
//! ask is the same parameter as an object. Off by default, so `get_node`'s
//! reply is byte-identical for every caller that does not ask.
//!
//! The published schema is `"type": ["boolean", "object", "null"]` with the
//! object's `properties` and `additionalProperties: false` beside it — JSON
//! Schema applies those two only when the value IS an object. That keeps it
//! inside the keyword subset every served schema is held to (no `anyOf`),
//! which an untagged serde enum would have left.
//!
//! The deserialiser is written by hand for the same reason the schema is: an
//! untagged enum answers a bad filter with "data did not match any variant",
//! naming nothing. This one hands an object to the filter's own derived
//! deserialiser, so an unknown key or direction is refused in serde's words
//! with the legal names listed.

use std::fmt;

use reflow2_core::node_edges::{EdgeDirection, EdgeQuery};
use schemars::{Schema, SchemaGenerator, json_schema};
use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize};

/// The filter form of `include_edges`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeFilter {
    /// `in`, `out` or `both` (the default).
    #[serde(default)]
    pub direction: Option<EdgeDirection>,
    /// Keep only these edge types.
    #[serde(default)]
    pub edge_types: Option<Vec<String>>,
    /// Drop these edge types.
    #[serde(default)]
    pub exclude_edge_types: Option<Vec<String>>,
    /// How many edges to list.
    #[serde(default)]
    pub limit: Option<usize>,
    /// Where to start in the ordered list.
    #[serde(default)]
    pub offset: Option<usize>,
    /// The reply budget, in characters of JSON.
    #[serde(default)]
    pub budget_chars: Option<usize>,
}

/// `get_node`'s `include_edges`: off (the default, `false` or `null`), every
/// edge (`true`), or a filter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum IncludeEdges {
    #[default]
    Off,
    All,
    Filter(EdgeFilter),
}

impl IncludeEdges {
    /// The core query this asks for, and the reply budget; `None` when off.
    #[must_use]
    pub fn query(&self) -> Option<(EdgeQuery, usize)> {
        let default_budget = crate::reply_budget::DEFAULT_REPLY_BUDGET_CHARS;
        match self {
            IncludeEdges::Off => None,
            IncludeEdges::All => Some((EdgeQuery::default(), default_budget)),
            IncludeEdges::Filter(f) => Some((
                EdgeQuery {
                    direction: f.direction.unwrap_or_default(),
                    edge_types: f.edge_types.clone().unwrap_or_default(),
                    exclude_edge_types: f.exclude_edge_types.clone().unwrap_or_default(),
                    limit: f.limit,
                    offset: f.offset.unwrap_or(0),
                },
                f.budget_chars.unwrap_or(default_budget),
            )),
        }
    }
}

impl<'de> Deserialize<'de> for IncludeEdges {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Shape;
        impl<'de> Visitor<'de> for Shape {
            type Value = IncludeEdges;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(
                    "`true`, `false`, or a filter object {direction, edge_types, \
                     exclude_edge_types, limit, offset, budget_chars}",
                )
            }

            fn visit_bool<E: de::Error>(self, on: bool) -> Result<IncludeEdges, E> {
                Ok(if on {
                    IncludeEdges::All
                } else {
                    IncludeEdges::Off
                })
            }

            fn visit_unit<E: de::Error>(self) -> Result<IncludeEdges, E> {
                Ok(IncludeEdges::Off)
            }

            fn visit_none<E: de::Error>(self) -> Result<IncludeEdges, E> {
                Ok(IncludeEdges::Off)
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<IncludeEdges, A::Error> {
                EdgeFilter::deserialize(de::value::MapAccessDeserializer::new(map))
                    .map(IncludeEdges::Filter)
            }
        }
        d.deserialize_any(Shape)
    }
}

/// The published schema: a boolean, or an object of the filter's fields.
/// Every keyword here is one the argument check reads.
pub fn schema(_: &mut SchemaGenerator) -> Schema {
    json_schema!({
        "type": ["boolean", "object", "null"],
        "properties": {
            "direction": {
                "type": ["string", "null"],
                "enum": ["in", "out", "both", null],
                "description": "Which edges: `in` (they arrive at this node), `out` (they leave it) or `both` (the default)."
            },
            "edge_types": {
                "type": ["array", "null"],
                "items": { "type": "string" },
                "description": "Keep only these edge types, as the schema names them (SATISFIES, AUTHORED_BY …). An unknown name is refused with the nearest one."
            },
            "exclude_edge_types": {
                "type": ["array", "null"],
                "items": { "type": "string" },
                "description": "Drop these edge types — for a type that crowds the rest out, such as a Release's INCLUDES. Nothing is dropped unless you name it here."
            },
            "limit": {
                "type": ["integer", "null"],
                "format": "uint",
                "minimum": 0,
                "description": "How many edges to list (default 50). 0 returns the counts only."
            },
            "offset": {
                "type": ["integer", "null"],
                "format": "uint",
                "minimum": 0,
                "description": "Where to start in the ordered list: pass the reply's `next_offset` to read on."
            },
            "budget_chars": {
                "type": ["integer", "null"],
                "format": "uint",
                "minimum": 0,
                "description": "Characters of JSON the reply may spend (default 30,000). Edges are listed whole, evidence included, until the next would pass it; raise it only if this client has the room."
            }
        },
        "additionalProperties": false
    })
}
