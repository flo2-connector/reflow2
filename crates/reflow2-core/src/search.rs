//! SEARCH — find design nodes by what they say, not by knowing their id.
//!
//! The schema has declared `fulltext:` on `name`/`statement`/`description`
//! properties since it was written, and the foundation implements the index
//! (`dynograph-text`, BM25 over Tantivy, mirrored automatically on every node
//! write) — but until 2026-07-20 nothing in reflow2 enabled the feature or
//! served it, so the only retrieval was `get_node` (know the id) and
//! `scan_nodes` (read a whole type). That made finding-by-content the LLM's
//! job, which is the seat-swap docs/partnership.md forbids: finding and
//! counting belong to the graph.
//!
//! The index is a **derived, rebuildable sidecar** — the node store stays the
//! source of truth. A graph written by a binary built *without* the feature
//! has nodes the index never saw, which is why [`DesignGraph::reindex_search`]
//! exists and is run once at server start: stale silence is worse than the
//! cost of one bounded rebuild.

use crate::foundation::core::DynoError;

use crate::graph::DesignGraph;

/// One search hit, hydrated: the scored id plus the node's `name`, so a caller
/// can show a result list without a `get_node` round trip per hit.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchHit {
    pub node_id: String,
    pub node_type: String,
    /// BM25 relevance — comparable within one result list, not across queries.
    pub score: f32,
    /// The node's `name` property at hit time (empty if it has none).
    pub name: String,
    /// The node's `status` (`accepted`, `proposed`, `realized`, …) when it has
    /// one, and a Decision's `kind` (`exploratory` / `choice`). ADDED 2026-10-06
    /// from the field log: the brainstorm skill says to search accepted intent
    /// before framing a question as open, and with no status on a hit every
    /// candidate cost a `get_node`. Absent, not null, when the node has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// How old this node's claim is, when it carries a date.
    ///
    /// FLATTENED AND SILENT BY DEFAULT: a node with no `valid_from` /
    /// `valid_to` serialises exactly as it did before ages existed, so the
    /// ordinary hit is unchanged and only a dated claim says anything extra.
    /// This is part 3 of `cap:claims-carry-their-age` — a six-week-old
    /// observation must not be read back as though it were current.
    #[serde(flatten)]
    pub age: crate::dates::ClaimAge,
    /// The records directly linked to this hit by a review relation or
    /// DECOMPOSES, so a linked family comes back together (F3 of
    /// `req:the-pieces-of-one-picture-are-found-together`). They are LINKED,
    /// not matched: they did not necessarily contain the query. At most
    /// [`LINKED_PER_HIT`]; absent when nothing is linked.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub linked: Vec<LinkedRecord>,
}

/// One record linked to a search hit.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LinkedRecord {
    pub node_id: String,
    /// The edge type: a review relation (`EVOLVES_INTO`, `DEPENDS_ON`, ...) or
    /// `DECOMPOSES`.
    pub relation: String,
    /// `out` when the hit is the edge's source (*hit RELATION linked*), `in`
    /// when it is the target (*linked RELATION hit*).
    pub direction: &'static str,
}

/// How many linked records a hit carries at most, so a heavily linked node
/// cannot swamp the reply.
pub const LINKED_PER_HIT: usize = 8;

impl DesignGraph {
    /// The records directly linked to `node_id` by a review relation or
    /// DECOMPOSES, outbound first, capped at [`LINKED_PER_HIT`].
    pub fn linked_records(&self, node_id: &str) -> Result<Vec<LinkedRecord>, DynoError> {
        let mut out = Vec::new();
        let kinds = crate::relate::REVIEW_RELATIONS
            .iter()
            .copied()
            .chain(std::iter::once(crate::nodes::edge::DECOMPOSES));
        for kind in kinds {
            for e in self.outgoing(node_id, Some(kind))? {
                out.push(LinkedRecord {
                    node_id: e.to_id,
                    relation: kind.to_string(),
                    direction: "out",
                });
            }
            for e in self.incoming(node_id, Some(kind))? {
                out.push(LinkedRecord {
                    node_id: e.from_id,
                    relation: kind.to_string(),
                    direction: "in",
                });
            }
        }
        out.sort_by(|a, b| {
            (a.direction != "out")
                .cmp(&(b.direction != "out"))
                .then(a.relation.cmp(&b.relation))
                .then(a.node_id.cmp(&b.node_id))
        });
        out.truncate(LINKED_PER_HIT);
        Ok(out)
    }
}

impl DesignGraph {
    /// A node's [`ClaimAge`], INCLUDING whether another record overturned it.
    ///
    /// The property-only [`crate::dates::claim_age`] cannot answer the second
    /// half: supersession lives on an `INVALIDATES` edge, not in the property
    /// bag. Every read surface that reports an age must come through here, or
    /// it will report a refuted finding as current — which is exactly what
    /// `search_design` did until 2026-09-07, while the gap detector reading the
    /// same edge had correctly fallen silent.
    ///
    /// Costs one incoming-edge lookup per node. That is why the clock is still
    /// read once per result by the caller rather than once per node.
    pub fn claim_age_of(
        &self,
        node_id: &str,
        props: &std::collections::HashMap<String, crate::foundation::core::Value>,
        today: &str,
    ) -> Result<crate::dates::ClaimAge, DynoError> {
        let mut age = crate::dates::claim_age(props, today);
        // The FIRST invalidator, by id, so the answer is stable across runs
        // rather than dependent on edge ordering. More than one is legitimate
        // — two records can each overturn a finding — and naming one of them
        // is enough to lead the reader to the rest.
        let mut invalidators: Vec<String> = self
            .incoming(node_id, Some(crate::nodes::edge::INVALIDATES))?
            .into_iter()
            .map(|e| e.from_id)
            .collect();
        invalidators.sort();
        age.superseded_by = invalidators.into_iter().next();
        Ok(age)
    }
}

/// A search result that owns up to what it could not do: hits the index
/// returned whose node no longer exists in the store are reported, never
/// silently dropped — a non-empty `stale` list means the index has drifted
/// and a [`DesignGraph::reindex_search`] is due.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    /// Ids the index returned but the store no longer holds (index drift).
    pub stale: Vec<String>,
    /// The limit that bounded this result — `hits.len() == limit` means there
    /// may be more; this is the no-silent-caps rule made visible.
    pub limit: usize,
    /// How many of this design's nodes the search ran over: the index's own
    /// count. It is what makes an empty `hits` say WHICH empty it is — "nothing
    /// matched in 6,000" and "nothing matched in 0" were the same reply until
    /// 2026-10-02, when a copy of a held design searched an empty index and
    /// answered `{"hits": []}` for a word the design held.
    pub searched: usize,
}

/// What opening a store found wrong with its search index, when the open had
/// to rebuild it: the index held `indexed_before` documents for a store holding
/// `searchable` nodes that belong in it. Kept so a surface can say it — a
/// rebuild that went only to a log would be the silent kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct SearchIndexRebuild {
    pub indexed_before: usize,
    pub searchable: usize,
}

#[cfg(feature = "fulltext")]
impl DesignGraph {
    /// BM25 keyword search over every `fulltext` property in the design,
    /// optionally scoped to one node type. Keyword search, not substring or
    /// regex: "persistence graph" finds nodes whose text carries those terms,
    /// ranked. Empty query or zero limit returns an empty result rather than
    /// everything.
    pub fn search_design(
        &self,
        query: &str,
        node_type: Option<&str>,
        limit: usize,
    ) -> Result<crate::search::SearchResult, DynoError> {
        let raw = self
            .engine()
            .search_fulltext(self.graph_id(), query, node_type, limit)?;
        let searched = self.engine().fulltext_indexed(self.graph_id())?;
        let mut hits = Vec::with_capacity(raw.len());
        let mut stale = Vec::new();
        // Read the clock ONCE for the whole result, not once per hit: two hits
        // in one list must never disagree about what day it is.
        let today = crate::dates::today_utc();
        for h in raw {
            match self.get_node(&h.node_type, &h.node_id)? {
                Some(node) => hits.push(SearchHit {
                    name: node
                        .properties
                        .get("name")
                        .and_then(crate::foundation::core::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    status: node
                        .properties
                        .get("status")
                        .and_then(crate::foundation::core::Value::as_str)
                        .map(str::to_string),
                    kind: node
                        .properties
                        .get("kind")
                        .and_then(crate::foundation::core::Value::as_str)
                        .map(str::to_string),
                    age: self.claim_age_of(&h.node_id, &node.properties, &today)?,
                    linked: self.linked_records(&h.node_id)?,
                    node_id: h.node_id,
                    node_type: h.node_type,
                    score: h.score,
                }),
                None => stale.push(h.node_id),
            }
        }
        Ok(SearchResult {
            hits,
            stale,
            limit,
            searched,
        })
    }

    /// Rebuild the full-text index from the node store. Bounded by graph size
    /// and idempotent; run at server start so a graph written by an older,
    /// index-less binary becomes searchable instead of silently absent.
    /// Returns the number of nodes indexed.
    pub fn reindex_search(&self) -> Result<usize, DynoError> {
        self.engine().reindex_fulltext(self.graph_id())
    }
}

#[cfg(not(feature = "fulltext"))]
impl DesignGraph {
    /// Fails loud without the `fulltext` feature (mirroring the `rocksdb`
    /// contract): a search that silently returns nothing would read as "the
    /// design says nothing about that", which is a lie.
    pub fn search_design(
        &self,
        _query: &str,
        _node_type: Option<&str>,
        _limit: usize,
    ) -> Result<crate::search::SearchResult, DynoError> {
        Err(DynoError::Storage(
            "this reflow2 was built without the `fulltext` feature, so it cannot search the \
             design. Rebuild with:  cargo build -p reflow2-mcp  (the surface crate enables it)"
                .into(),
        ))
    }

    /// Fails loud without the `fulltext` feature; see [`Self::search_design`].
    pub fn reindex_search(&self) -> Result<usize, DynoError> {
        Err(DynoError::Storage(
            "this reflow2 was built without the `fulltext` feature, so there is no search \
             index to rebuild. Rebuild with:  cargo build -p reflow2-mcp"
                .into(),
        ))
    }
}
