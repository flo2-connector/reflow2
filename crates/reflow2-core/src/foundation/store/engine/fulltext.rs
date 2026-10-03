//! `StorageEngine` — public full-text search/reindex. Split out of `engine.rs`; `use super::*`
//! inherits the shared imports and types from the parent `engine` module.
//! Private helper methods live in `engine/mod.rs` (a parent module, so
//! these methods reach them as descendants).

use super::*;

/// One full-text hit: the matched node's id and type, plus its BM25 score.
///
/// Owned by this crate on purpose. `search_fulltext` used to return
/// `crate::foundation::text::TextHit` directly, which leaked a type belonging to the
/// `TextIndex` boundary through the `StorageEngine` boundary: a consumer that
/// never names `dynograph-text` — and cannot, since it arrives only through an
/// optional feature — still broke when that type changed, and no published
/// surface said so. The two boundaries are published separately, so their types
/// are separate too.
///
/// `#[non_exhaustive]`: fields are read, never constructed, outside this crate,
/// so adding one later must not be a breaking change.
///
/// Gated on `fulltext` alongside its only constructor and its only returning
/// method: without the feature nothing can produce one, and public API that no
/// build path can reach is a surface promising something it cannot deliver.
#[cfg(feature = "fulltext")]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FulltextHit {
    /// The matched node's id.
    pub node_id: String,
    /// The matched node's type.
    pub node_type: String,
    /// BM25 relevance score; higher is a better match.
    pub score: f32,
}

#[cfg(feature = "fulltext")]
impl From<crate::foundation::text::TextHit> for FulltextHit {
    fn from(h: crate::foundation::text::TextHit) -> Self {
        Self {
            node_id: h.node_id,
            node_type: h.node_type,
            score: h.score,
        }
    }
}

/// What the full-text index holds for one graph, against what the store holds
/// that the index should.
///
/// THE INDEX IS A DERIVED COPY OF THE STORE, and a copy can lack what it was
/// copied from: a store directory copied without its `fulltext/` subdirectory, a
/// store written by a build without the feature, an index directory lost or
/// restored from a different backup. Each of those opens cleanly — Tantivy
/// creates an empty index where none exists — and then answers every query with
/// "nothing matched". Measured 2026-10-02 through `--call` on a design another
/// server held: `search_design` answered `{"hits": []}` and `topic_report`
/// "NOTHING MATCHED … across 2 node(s)" for a word one of those two nodes held.
/// `covers()` is the one comparison that tells that apart from a true miss.
#[cfg(feature = "fulltext")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FulltextCoverage {
    /// Documents the index holds for the graph: what a search runs over.
    pub indexed: usize,
    /// Nodes the store holds for the graph, of every type that declares a
    /// `fulltext` property: what a search should run over.
    pub searchable: usize,
}

#[cfg(feature = "fulltext")]
impl FulltextCoverage {
    /// Whether the index holds exactly what the store says it should. Every
    /// node write mirrors into the index, so two counts that differ mean the
    /// index was not built from this store — never a matter of timing.
    pub fn covers(&self) -> bool {
        self.indexed == self.searchable
    }
}

impl StorageEngine {
    /// BM25 keyword search over the full-text index, scoped to `graph_id` and
    /// optionally one `node_type`. Returns up to `limit` hits, highest score
    /// first. Empty when the schema declares no `fulltext` property. Fails loud
    /// if full-text was enabled on a live engine that opened without an index
    /// (see `fulltext_unavailable`).
    ///
    /// ⭐ AN EMPTY RESULT IS CHECKED BEFORE IT IS RETURNED. "Nothing matched" is
    /// the strongest claim a search makes — a caller deciding whether something
    /// already exists acts on it — so before an empty list goes back, the index
    /// is compared with the store ([`fulltext_coverage`](Self::fulltext_coverage)).
    /// An index that does not hold what the store holds REFUSES, naming both
    /// counts, rather than answer an absence it never searched for. The check
    /// costs one count of the graph's searchable nodes and is paid only on an
    /// empty result; a store's open rebuilds an index that does not cover it
    /// ([`ensure_fulltext_covers`](Self::ensure_fulltext_covers)), so on any
    /// store opened through `DesignGraph` this refusal is the floor under that
    /// guarantee, not the normal path. Skipped inside an open batch, whose
    /// buffered nodes the store reads and the index by design does not yet.
    #[cfg(feature = "fulltext")]
    pub fn search_fulltext(
        &self,
        graph_id: &str,
        query: &str,
        node_type: Option<&str>,
        limit: usize,
    ) -> Result<Vec<FulltextHit>, DynoError> {
        if let Some(ti) = &self.text_index {
            let hits: Vec<FulltextHit> = ti
                .search(graph_id, query, node_type, limit)
                .map(|hits| hits.into_iter().map(FulltextHit::from).collect())
                .map_err(|e| DynoError::Storage(format!("full-text search failed: {e}")))?;
            // Not inside an open batch: there the store already reads the
            // batch's buffered nodes and the index by design does not (they
            // become searchable at commit), so the two counts differ for a
            // reason that is not a missing index.
            if hits.is_empty()
                && !self.is_batching()
                && let Some(coverage) = self.fulltext_coverage(graph_id)?
                && !coverage.covers()
            {
                return Err(DynoError::Storage(format!(
                    "SEARCH REFUSED rather than answer \"nothing matched\": the full-text index \
                     holds {} document(s) for this design and the store holds {} node(s) that \
                     should be in it, so an empty result here would be an absence nobody \
                     searched for. The index is a copy derived from the store and is rebuilt \
                     when the store is opened; reopen it (restart the server, or run the call \
                     again) for a full answer.",
                    coverage.indexed, coverage.searchable
                )));
            }
            return Ok(hits);
        }
        match self.fulltext_unavailable() {
            Some(err) => Err(err),
            None => Ok(Vec::new()),
        }
    }

    /// How many documents the full-text index holds for `graph_id` — the
    /// population a search in that graph runs over. `0` when the schema
    /// genuinely declares no full-text; fails loud when full-text is declared
    /// and the engine opened without an index, like `search_fulltext`.
    #[cfg(feature = "fulltext")]
    pub fn fulltext_indexed(&self, graph_id: &str) -> Result<usize, DynoError> {
        if let Some(ti) = &self.text_index {
            return ti
                .count(graph_id)
                .map_err(|e| DynoError::Storage(format!("full-text count failed: {e}")));
        }
        match self.fulltext_unavailable() {
            Some(err) => Err(err),
            None => Ok(0),
        }
    }

    /// What the index holds for `graph_id` against what the store holds that it
    /// should — see [`FulltextCoverage`]. `None` when the schema declares no
    /// full-text, so there is nothing for an index to cover.
    ///
    /// Cost: one count per full-text node type, which reads every such node's
    /// key and value once. Bounded by graph size; meant for an open and for the
    /// empty-result check, not for every hit.
    #[cfg(feature = "fulltext")]
    pub fn fulltext_coverage(&self, graph_id: &str) -> Result<Option<FulltextCoverage>, DynoError> {
        if !self.schema.has_any_fulltext_properties() {
            return Ok(None);
        }
        let indexed = self.fulltext_indexed(graph_id)?;
        let mut searchable = 0usize;
        for node_type in self.schema.node_types.keys() {
            if self.schema.has_fulltext_properties(node_type) {
                searchable += self.count_nodes(graph_id, node_type)?;
            }
        }
        Ok(Some(FulltextCoverage {
            indexed,
            searchable,
        }))
    }

    /// Make the index cover the store for `graph_id`, rebuilding it when it
    /// does not. Returns what was found BEFORE a rebuild when one ran, and
    /// `None` when the index already covered the store (or there is no
    /// full-text to cover).
    ///
    /// Called when a store is opened, because that is the moment a store meets
    /// an index it may not have been written with — a copied directory, an
    /// older build's store, a lost `fulltext/` — and the one moment a rebuild is
    /// both cheap to justify and safe (nothing else holds the store yet). This
    /// is what the live server already did at start (`reindex_search`), made a
    /// property of every open instead of one caller's habit: the copy `--call`
    /// reads while another process holds a design was opened through a path
    /// that skipped it, and searched an empty index.
    #[cfg(feature = "fulltext")]
    pub fn ensure_fulltext_covers(
        &self,
        graph_id: &str,
    ) -> Result<Option<FulltextCoverage>, DynoError> {
        match self.fulltext_coverage(graph_id)? {
            Some(found) if !found.covers() => {
                self.reindex_fulltext(graph_id)?;
                Ok(Some(found))
            }
            _ => Ok(None),
        }
    }

    /// Rebuild the full-text index for `graph_id` from the authoritative node
    /// store: drop the graph's existing documents, then re-index every
    /// `fulltext` node. Use to recover from drift. Returns the number of nodes
    /// indexed; `Ok(0)` only when the schema genuinely declares no full-text.
    ///
    /// Fails loud (not `Ok(0)`) if full-text was enabled on a live engine that
    /// opened without an index — reopen the engine to build it first.
    ///
    /// Cost: materializes every full-text node and rebuilds in a single pass
    /// under the caller's lock (the service holds the per-graph write lock), so
    /// a reindex of a large graph blocks its reads and writes for the duration.
    /// An incremental / double-buffered rebuild is tracked as a follow-up.
    #[cfg(feature = "fulltext")]
    pub fn reindex_fulltext(&self, graph_id: &str) -> Result<usize, DynoError> {
        // A rebuild can't run inside an open batch: `scan_nodes` would see
        // uncommitted node state, and the final `ti.commit()` would flush the
        // batch's buffered text ops, breaking its all-or-nothing semantics.
        if self.is_batching() {
            return Err(DynoError::Storage(
                "reindex_fulltext cannot run inside an open batch".to_string(),
            ));
        }
        let ti = match &self.text_index {
            Some(ti) => ti,
            None => {
                return match self.fulltext_unavailable() {
                    Some(err) => Err(err),
                    None => Ok(0),
                };
            }
        };
        // Clear-then-rebuild buffers a `delete_graph` plus a series of
        // `upsert`s into the Tantivy writer before the final `commit()`. If any
        // step fails partway through, the half-built batch must be rolled back
        // before returning — otherwise those ops stay queued in the shared
        // writer and a later unrelated `commit()` (e.g. from a node write)
        // would flush a *partial* rebuild, silently dropping the graph's prior
        // index contents and leaving hard-to-debug drift.
        let rebuild = || -> Result<usize, DynoError> {
            ti.delete_graph(graph_id)
                .map_err(|e| DynoError::Storage(format!("full-text reindex clear failed: {e}")))?;
            // Snapshot the fulltext node types first — `scan_nodes` borrows &self.
            let node_types: Vec<String> = self
                .schema
                .node_types
                .keys()
                .filter(|nt| self.schema.has_fulltext_properties(nt))
                .cloned()
                .collect();
            let mut count = 0usize;
            for node_type in node_types {
                for node in self.scan_nodes(graph_id, &node_type)? {
                    let fields = self.fulltext_fields(&node_type, &node.properties);
                    ti.upsert(graph_id, &node_type, &node.node_id, &fields)
                        .map_err(|e| {
                            DynoError::Storage(format!("full-text reindex upsert failed: {e}"))
                        })?;
                    count += 1;
                }
            }
            ti.commit()
                .map_err(|e| DynoError::Storage(format!("full-text reindex commit failed: {e}")))?;
            Ok(count)
        };
        // Best-effort rollback to drop the uncommitted batch on failure;
        // surface the original rebuild error regardless of the rollback result.
        rebuild().inspect_err(|_| {
            let _ = ti.rollback();
        })
    }
}
