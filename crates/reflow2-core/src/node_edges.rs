//! One node's edges, read back AS EDGES — the per-node read the graph had no
//! form of (`dec:idea-an-edge-reader-returns-one-nodes-edges-and-find-tools-finds-it`,
//! accepted by Anthony on 2026-10-02).
//!
//! # Why this exists, measured on 0.77.0
//!
//! The edges are what make a design a graph rather than a list, and no served
//! read handed one node's edges back as edges
//! (`fact:root-cause-no-served-read-returns-one-nodes-edges-and-the-nearest-is-an-impact-walk-2026-10-02`).
//! On `req:a-lesson-is-served-at-the-step-it-concerns`, which holds 7 edges:
//!
//! - `get_node` returned its properties only.
//! - `propagate_from` at depth 1, the nearest reader, gave 6 of the 7. It
//!   dropped `AUTHORED_BY`, because authorship is deliberately not a
//!   traceability edge. It labelled each with an IMPACT direction
//!   (downstream / lateral / causal) rather than the stored from → to, and it
//!   showed none of an edge's own properties, so no evidence.
//! - `topic_report` gave counts per edge type, keyed by words, not by id.
//! - Only `export_graph` was complete, at the cost of the whole design.
//!
//! Each partial reader was built for another job, so none of them is complete
//! for this one. An absence claim made from the node alone was measured wrong
//! on 2026-09-28: the reason a requirement was dropped sat on the decision
//! that retired it, one edge away.
//!
//! # What it returns
//!
//! Every edge on the node, in both directions, including the ones an impact
//! walk leaves out (`AUTHORED_BY`, `CONTAINS`) and the stored twins
//! (`HAS_TEMPORAL_FACT` and the rest, see [`crate::twins`]). Each edge carries:
//!
//! - its type, its stored `from_id` and `to_id`, and `direction` (`out` when
//!   it leaves the node read, `in` when it arrives);
//! - the other end's id, type and `name`, so the reader needs no second call
//!   to know what it is joined to;
//! - the edge's OWN properties, evidence and note included;
//! - `twin_of` when the edge is the stored copy of a node property.
//!
//! # Nothing is dropped silently
//!
//! - `total` and `by_type` count EVERY edge on the node and are never
//!   filtered, so a filter or a bound can shorten the list and never the
//!   count.
//! - A filter is the caller's: direction, edge types to keep, edge types to
//!   drop. Nothing is excluded by default. When one type holds most of what
//!   matched and the list could not show it all (a Release's `INCLUDES`, a
//!   contributor's `AUTHORED_BY`), `dominant` says so and names the filter
//!   that drops it.
//! - The list is bounded by `limit` and by the shared reply budget, and says
//!   which bound stopped it (`capped_by`) and where to resume (`next_offset`).
//!   An edge is returned whole or not at all, because a trimmed evidence string
//!   reads as the whole evidence.
//! - An empty list says which empty it is (`empty_because`): no edges at all,
//!   none matching the filter, a limit of zero, or an offset past the end.
//!
//! # Order
//!
//! Rarest edge type first, then by type name, direction (`out` before `in`)
//! and the other end's id. Deterministic, so paging with `offset` is stable.
//! A dominant type therefore comes LAST, which keeps a bounded reply showing
//! the edges that say the most about the node, and `dominant` says what was
//! pushed past the bound.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::foundation::core::{DynoError, TwinEnd, Value};
use crate::foundation::store::{StoredEdge, StoredNode};
use crate::graph::DesignGraph;
use crate::twins::StoredTwin;

/// How many edges one reply lists when the caller names no `limit`.
///
/// Measured on reflow2's own design (2026-10-02): the median node holds 3
/// edges and the 99th percentile 225, so 50 returns the whole set for nearly
/// every node in one call. The shared reply budget bounds it again by size.
pub const DEFAULT_EDGE_LIMIT: usize = 50;

/// The order the list is in, said in the reply so a door agent that never
/// reads the tool's description still knows how to page it.
pub const EDGE_ORDER: &str = "rarest edge type first, then by edge type, direction (out before \
     in) and the other end's id; stable, so `offset` pages it";

/// Which of a node's edges to read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeDirection {
    /// Edges that arrive at the node (the node is `to_id`).
    In,
    /// Edges that leave the node (the node is `from_id`).
    Out,
    /// Both. The default.
    #[default]
    Both,
}

/// Which side of one edge the node read sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeSide {
    /// The edge leaves the node read: it is `from_id`.
    Out,
    /// The edge arrives at the node read: it is `to_id`.
    In,
}

/// What to read. Every field defaults to "everything".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EdgeQuery {
    pub direction: EdgeDirection,
    /// Keep only these edge types. Empty keeps every type.
    pub edge_types: Vec<String>,
    /// Drop these edge types.
    pub exclude_edge_types: Vec<String>,
    /// How many to list. `None` is [`DEFAULT_EDGE_LIMIT`].
    pub limit: Option<usize>,
    /// Where to start in the ordered list.
    pub offset: usize,
}

/// How many edges of one type leave and arrive at the node.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct DirectionCounts {
    pub out: usize,
    #[serde(rename = "in")]
    pub incoming: usize,
}

/// The filter as it was applied, echoed so a reader can tell a filtered list
/// from a whole one without remembering what it asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppliedFilter {
    pub direction: EdgeDirection,
    /// `null` when every type was kept.
    pub edge_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude_edge_types: Vec<String>,
    pub limit: usize,
    pub offset: usize,
}

/// The node at the other end of one edge.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OtherEnd {
    pub node_id: String,
    /// `null` when no node type holds the id (`missing`) or when more than one
    /// does (`held_by`) — never a guess.
    pub node_type: Option<String>,
    /// The other node's `name`, `null` when it carries none.
    pub name: Option<String>,
    /// True when nothing in the design holds this id: the edge dangles.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub missing: bool,
    /// Every type holding the id, when it is more than one (a broken id
    /// convention). Absent otherwise.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub held_by: Vec<String>,
}

/// One edge, as stored, seen from the node read.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EdgeRow {
    pub edge_type: String,
    pub direction: EdgeSide,
    pub from_id: String,
    pub to_id: String,
    pub other: OtherEnd,
    /// The edge's own properties — evidence, note, coverage, roles — exactly as
    /// stored. `{}` when it carries none.
    pub properties: BTreeMap<String, Value>,
    /// `NodeType.property` when this edge is the stored copy of that property
    /// (see [`crate::twins`]): the property is the authority, and the edge
    /// follows it. Absent on an ordinary edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twin_of: Option<String>,
}

/// One edge type holding most of what matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DominantType {
    pub edge_type: String,
    pub count: usize,
    pub of: usize,
    pub note: String,
}

/// One node's edges, bounded, with every count a filter or a bound could
/// otherwise hide.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NodeEdges {
    /// Every edge on the node, both directions, before any filter. A self-loop
    /// counts once.
    pub total: usize,
    /// Per edge type, how many leave the node (`out`) and arrive at it
    /// (`in`) — over EVERY edge, never filtered. A self-loop counts on both
    /// sides, because it is both.
    pub by_type: BTreeMap<String, DirectionCounts>,
    pub filter: AppliedFilter,
    /// Edges the filter kept.
    pub matched: usize,
    /// Edges the filter left out: `total - matched`.
    pub filtered_out: usize,
    /// How many are in `items`.
    pub returned: usize,
    /// Matching edges not in this reply: `matched - offset - returned`.
    pub omitted: usize,
    /// Where to resume, `null` when this reply reached the end.
    pub next_offset: Option<usize>,
    /// `limit` or `size` when the list stopped early; `null` when it did not.
    pub capped_by: Option<&'static str>,
    /// The reply budget the list was fitted to, in characters of JSON.
    pub budget_chars: usize,
    pub order: &'static str,
    /// How many listed edges are stored twins (`twin_of` set).
    pub twins: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twins_note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dominant: Option<DominantType>,
    pub items: Vec<EdgeRow>,
    /// Said whenever `items` is empty: which empty it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_because: Option<String>,
    /// Said whenever the list stopped early: what stopped it and how to read on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

fn json_len<T: Serialize + ?Sized>(v: &T) -> usize {
    serde_json::to_string(v)
        .map(|s| s.len())
        .unwrap_or(usize::MAX)
}

/// What one id resolved to: the node when exactly one type holds it, else the
/// holders (none, or more than one).
type Resolved = (Option<StoredNode>, Vec<String>);

struct Reader<'g> {
    g: &'g DesignGraph,
    all_types: Vec<String>,
    twins: Vec<StoredTwin>,
    read_type: &'g str,
    read_id: &'g str,
    read_node: Option<StoredNode>,
    cache: HashMap<String, Resolved>,
}

impl Reader<'_> {
    /// Resolve the other end of an edge by asking EVERY node type, because
    /// the store keys an edge's ends by id alone and most edge types are open
    /// (`*`) at one end. More than one holder is reported, never guessed
    /// between — the rule `get_node` keeps for the node it reads.
    fn resolve(&mut self, id: &str) -> Result<Resolved, DynoError> {
        if let Some(hit) = self.cache.get(id) {
            return Ok(hit.clone());
        }
        let mut found = self.holders(id)?;
        let resolved = if found.len() == 1 {
            (found.pop().map(|(_, n)| n), Vec::new())
        } else {
            (None, found.into_iter().map(|(t, _)| t).collect())
        };
        self.cache.insert(id.to_string(), resolved.clone());
        Ok(resolved)
    }

    fn holders(&self, id: &str) -> Result<Vec<(String, StoredNode)>, DynoError> {
        let mut out = Vec::new();
        for t in &self.all_types {
            if let Some(n) = self.g.get_node(t, id)? {
                out.push((t.clone(), n));
            }
        }
        Ok(out)
    }

    /// `NodeType.property` when `e` is the stored copy of a twin property that
    /// names its far end; `None` for an ordinary edge.
    fn twin_of(&self, e: &StoredEdge, other: Option<&StoredNode>) -> Option<String> {
        for st in self.twins.iter().filter(|st| st.twin.edge == e.edge_type) {
            let (owner_id, far) = match st.twin.names {
                TwinEnd::Source => (&e.to_id, &e.from_id),
                TwinEnd::Target => (&e.from_id, &e.to_id),
            };
            let owner = if owner_id == self.read_id {
                self.read_node
                    .as_ref()
                    .filter(|n| n.node_type == self.read_type)
            } else {
                other.filter(|n| &n.node_id == owner_id)
            };
            let Some(owner) = owner else { continue };
            if owner.node_type != st.node_type {
                continue;
            }
            if owner.properties.get(&st.property).and_then(Value::as_str) == Some(far.as_str()) {
                return Some(format!("{}.{}", st.node_type, st.property));
            }
        }
        None
    }

    fn row(&mut self, side: EdgeSide, e: &StoredEdge) -> Result<EdgeRow, DynoError> {
        let other_id = match side {
            EdgeSide::Out => &e.to_id,
            EdgeSide::In => &e.from_id,
        };
        let (node, held_by) = self.resolve(other_id)?;
        let twin_of = self.twin_of(e, node.as_ref());
        let other = OtherEnd {
            node_id: other_id.clone(),
            node_type: node.as_ref().map(|n| n.node_type.clone()),
            name: node
                .as_ref()
                .and_then(|n| n.properties.get("name"))
                .and_then(Value::as_str)
                .map(str::to_string),
            missing: node.is_none() && held_by.is_empty(),
            held_by,
        };
        Ok(EdgeRow {
            edge_type: e.edge_type.clone(),
            direction: side,
            from_id: e.from_id.clone(),
            to_id: e.to_id.clone(),
            other,
            properties: e
                .properties
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            twin_of,
        })
    }
}

impl DesignGraph {
    /// Refuse an edge-type name the schema does not declare. A filter naming
    /// an unknown type would otherwise match nothing and answer "no such
    /// edge" for what is really "no such TYPE" — the same two facts `get_node`
    /// refuses to merge for a node type.
    fn check_edge_type_names(&self, field: &str, names: &[String]) -> Result<(), DynoError> {
        let declared = &self.schema().edge_types;
        for name in names {
            if declared.contains_key(name) {
                continue;
            }
            let mut known: Vec<&String> = declared.keys().collect();
            known.sort();
            let near: Vec<&str> = known
                .iter()
                .filter(|k| k.eq_ignore_ascii_case(name))
                .map(|k| k.as_str())
                .collect();
            let hint = if near.is_empty() {
                String::new()
            } else {
                format!(" Did you mean {}?", near.join(" or "))
            };
            return Err(DynoError::UnknownEdgeType(format!(
                "{name:?} in include_edges.{field} is not an edge type in this schema, so \
                 filtering on it would answer \"no such edge\" for what is really \"no such \
                 type\".{hint} Known edge types: {}.",
                known
                    .iter()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        Ok(())
    }

    /// One node's edges, as edges: both directions, each with its type,
    /// direction, the other end's id, type and name, its own properties, and
    /// whether it is a stored twin. Filtered by `q`, ordered as [`EDGE_ORDER`]
    /// says, and bounded by `q.limit` and by `budget_chars` characters of JSON
    /// for the whole reply, of which `spent_chars` are already taken by what
    /// the caller sends beside the edges (`get_node` sends the node). At least
    /// one matching edge is always listed when any remains, so an edge larger
    /// than the whole budget is still readable.
    ///
    /// The node is named by type and id because the caller has resolved it;
    /// an id no node holds simply has no edges, and the caller says so.
    pub fn node_edges(
        &self,
        node_type: &str,
        id: &str,
        q: &EdgeQuery,
        budget_chars: usize,
        spent_chars: usize,
    ) -> Result<NodeEdges, DynoError> {
        self.check_edge_type_names("edge_types", &q.edge_types)?;
        self.check_edge_type_names("exclude_edge_types", &q.exclude_edge_types)?;

        let outgoing = self.outgoing(id, None)?;
        let incoming = self.incoming(id, None)?;

        // Every edge, counted before any filter. A self-loop is in both lists
        // and is one edge.
        let mut by_type: BTreeMap<String, DirectionCounts> = BTreeMap::new();
        for e in &outgoing {
            by_type.entry(e.edge_type.clone()).or_default().out += 1;
        }
        for e in &incoming {
            by_type.entry(e.edge_type.clone()).or_default().incoming += 1;
        }
        let self_loops = outgoing.iter().filter(|e| e.to_id == id).count();
        let total = outgoing.len() + incoming.len() - self_loops;

        let wanted = |e: &StoredEdge| {
            (q.edge_types.is_empty() || q.edge_types.contains(&e.edge_type))
                && !q.exclude_edge_types.contains(&e.edge_type)
        };
        let mut kept: Vec<(EdgeSide, &StoredEdge)> = Vec::new();
        if q.direction != EdgeDirection::In {
            kept.extend(
                outgoing
                    .iter()
                    .filter(|e| wanted(e))
                    .map(|e| (EdgeSide::Out, e)),
            );
        }
        if q.direction != EdgeDirection::Out {
            kept.extend(
                incoming
                    .iter()
                    // Listed once: with both directions asked, a self-loop is
                    // already in as `out`.
                    .filter(|e| !(q.direction == EdgeDirection::Both && e.from_id == id))
                    .filter(|e| wanted(e))
                    .map(|e| (EdgeSide::In, e)),
            );
        }
        let matched = kept.len();

        let mut kept_per_type: HashMap<&str, usize> = HashMap::new();
        for (_, e) in &kept {
            *kept_per_type.entry(e.edge_type.as_str()).or_default() += 1;
        }
        kept.sort_by(|(sa, a), (sb, b)| {
            let other = |s: &EdgeSide, e: &StoredEdge| match s {
                EdgeSide::Out => e.to_id.clone(),
                EdgeSide::In => e.from_id.clone(),
            };
            kept_per_type[a.edge_type.as_str()]
                .cmp(&kept_per_type[b.edge_type.as_str()])
                .then_with(|| a.edge_type.cmp(&b.edge_type))
                .then_with(|| sa.cmp(sb))
                .then_with(|| other(sa, a).cmp(&other(sb, b)))
        });

        let limit = q.limit.unwrap_or(DEFAULT_EDGE_LIMIT);
        let offset = q.offset.min(matched);
        let filter = AppliedFilter {
            direction: q.direction,
            edge_types: (!q.edge_types.is_empty()).then(|| q.edge_types.clone()),
            exclude_edge_types: q.exclude_edge_types.clone(),
            limit,
            offset: q.offset,
        };

        let mut all_types: Vec<String> = self.schema().node_types.keys().cloned().collect();
        all_types.sort();
        let mut reader = Reader {
            g: self,
            all_types,
            twins: self.stored_twins(),
            read_type: node_type,
            read_id: id,
            read_node: self.get_node(node_type, id)?,
            cache: HashMap::new(),
        };

        let mut out = NodeEdges {
            total,
            by_type,
            filter,
            matched,
            filtered_out: total.saturating_sub(matched),
            returned: 0,
            omitted: 0,
            next_offset: None,
            capped_by: None,
            budget_chars,
            order: EDGE_ORDER,
            twins: 0,
            twins_note: None,
            dominant: None,
            items: Vec::new(),
            empty_because: None,
            note: None,
        };
        // What the reply costs before any edge is listed, so the edges are
        // fitted to what is left of the budget rather than to all of it. The
        // slack covers the sentences added after the list is cut (`note`,
        // `dominant`, `twins_note`), each a few hundred characters.
        let overhead = json_len(&out) + 1_200;
        let room = budget_chars.saturating_sub(spent_chars + overhead);

        let mut items: Vec<EdgeRow> = Vec::new();
        let mut spent = 0usize;
        for (side, e) in kept.iter().skip(offset) {
            if items.len() >= limit {
                out.capped_by = Some("limit");
                break;
            }
            let row = reader.row(*side, e)?;
            let size = json_len(&row) + 1;
            if !items.is_empty() && spent + size > room {
                out.capped_by = Some("size");
                break;
            }
            spent += size;
            items.push(row);
        }

        let returned = items.len();
        let next = offset + returned;
        out.returned = returned;
        out.omitted = matched - next;
        out.next_offset = (next < matched).then_some(next);
        out.twins = items.iter().filter(|r| r.twin_of.is_some()).count();
        if out.twins > 0 {
            out.twins_note = Some(
                "An edge with `twin_of` is the stored COPY of that node property: the property \
                 is the authority and the store keeps the edge in step with it. Change the \
                 property to move the relation; the edge tools refuse to draw one that \
                 disagrees with it or to delete one it still names."
                    .to_string(),
            );
        }

        // A type that holds most of what matched, when the list could not show
        // it all: say so, and name the filter that drops it.
        if (next < matched || offset > 0)
            && kept_per_type.len() > 1
            && let Some((ty, count)) = kept_per_type
                .iter()
                .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
            && count * 2 > matched
        {
            out.dominant = Some(DominantType {
                edge_type: ty.to_string(),
                count: *count,
                of: matched,
                note: format!(
                    "{ty} is {count} of the {matched} matching edges, so it is listed last, \
                     after every rarer type, and this reply could not show them all. Pass \
                     include_edges {{\"exclude_edge_types\": [\"{ty}\"]}} to drop it, or \
                     {{\"edge_types\": [\"{ty}\"]}} to page through it alone."
                ),
            });
        }

        out.note = match out.capped_by {
            Some("limit") => Some(format!(
                "Listed {returned} of {matched} matching edge(s), stopped by `limit` {limit}. \
                 Resume with include_edges {{\"offset\": {next}}}, raise `limit`, or narrow \
                 with `direction`, `edge_types` or `exclude_edge_types`. `total` and `by_type` \
                 count every edge."
            )),
            Some(_) => Some(format!(
                "Listed {returned} of {matched} matching edge(s): the next would pass the \
                 reply budget of {budget_chars} characters. Each edge is listed whole, with \
                 its evidence, or not at all. Resume with include_edges {{\"offset\": \
                 {next}}}, raise `budget_chars` if this client has the room, or narrow with \
                 `direction`, `edge_types` or `exclude_edge_types`. `total` and `by_type` \
                 count every edge."
            )),
            None => None,
        };

        if items.is_empty() {
            out.empty_because = Some(if total == 0 {
                "This node has no edges at all, in either direction: nothing links to it and \
                 it links to nothing. That is the whole answer, not a filter or a bound."
                    .to_string()
            } else if matched == 0 {
                format!(
                    "None of this node's {total} edge(s) match the filter (direction {}, {}{}). \
                     `by_type` lists every edge it has.",
                    match q.direction {
                        EdgeDirection::In => "in",
                        EdgeDirection::Out => "out",
                        EdgeDirection::Both => "both",
                    },
                    if q.edge_types.is_empty() {
                        "every edge type".to_string()
                    } else {
                        format!("edge_types {:?}", q.edge_types)
                    },
                    if q.exclude_edge_types.is_empty() {
                        String::new()
                    } else {
                        format!(", excluding {:?}", q.exclude_edge_types)
                    }
                )
            } else if limit == 0 {
                format!(
                    "`limit` 0 asks for counts only: {matched} edge(s) match, and `by_type` \
                     counts every edge."
                )
            } else {
                format!(
                    "`offset` {} is past the end: {matched} edge(s) match. Start at 0.",
                    q.offset
                )
            });
        }

        out.items = items;
        Ok(out)
    }
}
