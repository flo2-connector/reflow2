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
    /// Whether the index was built over the properties THIS build searches.
    /// `false` when it says a different list, or says none (an index an older
    /// build made) — even with the counts equal, because a `fulltext` flag
    /// added to a type already indexed leaves the count where it was. Measured
    /// 2026-10-12: six prose fields marked searchable reached no node written
    /// before the release that marked them.
    pub built_over_these_fields: bool,
}

#[cfg(feature = "fulltext")]
impl FulltextCoverage {
    /// Whether the index holds exactly what the store says it should, over
    /// the fields this build searches. Every node write mirrors into the
    /// index, so two counts that differ mean the index was not built from this
    /// store — never a matter of timing.
    pub fn covers(&self) -> bool {
        self.indexed == self.searchable && self.built_over_these_fields
    }

    /// What is wrong with an index that does not cover its store, in a
    /// sentence a refusal can carry.
    fn shortfall(&self) -> String {
        if self.indexed == self.searchable {
            format!(
                "the full-text index holds all {} node(s) of this design, but it was built \
                 over a different list of the fields search reads than this build's, so a \
                 word in a field it did not read is not in it",
                self.indexed
            )
        } else {
            format!(
                "the full-text index holds {} document(s) for this design and the store \
                 holds {} node(s) that should be in it",
                self.indexed, self.searchable
            )
        }
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
    /// guarantee, not the normal path.
    ///
    /// INSIDE AN OPEN BATCH it answers for the batch's own writes, as every
    /// other read does: the batch's pending text is published first (see
    /// `TextBatch` for why, and for how a discard then undoes it).
    #[cfg(feature = "fulltext")]
    pub fn search_fulltext(
        &self,
        graph_id: &str,
        query: &str,
        node_type: Option<&str>,
        limit: usize,
    ) -> Result<Vec<FulltextHit>, DynoError> {
        if let Some(ti) = &self.text_index {
            self.publish_text()?;
            let hits: Vec<FulltextHit> = ti
                .search(graph_id, query, node_type, limit)
                .map(|hits| hits.into_iter().map(FulltextHit::from).collect())
                .map_err(|e| DynoError::Storage(format!("full-text search failed: {e}")))?;
            // Inside an open batch too: the batch's pending text was just
            // published, so the index holds the batch's writes exactly as the
            // store's reads do, and the two counts still compare like for like.
            if hits.is_empty()
                && let Some(coverage) = self.fulltext_coverage(graph_id)?
                && !coverage.covers()
            {
                return Err(DynoError::Storage(format!(
                    "SEARCH REFUSED rather than answer \"nothing matched\": {}, so an empty \
                     result here would be an absence nobody searched for. The index is a copy \
                     derived from the store and is rebuilt when the store is opened; reopen it \
                     (restart the server, or run the call again) for a full answer.",
                    coverage.shortfall()
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
        // What the index says it was built under, against what this build
        // searches. An engine with no index has already failed above.
        let built_over_these_fields = self.text_index.as_ref().is_none_or(|ti| {
            ti.built_under(graph_id).as_deref() == Some(self.schema.fulltext_stamp().as_str())
        });
        Ok(Some(FulltextCoverage {
            indexed,
            searchable,
            built_over_these_fields,
        }))
    }

    /// Make the index match the store for `graph_id`: REUSE it when it is
    /// proven to match, and rebuild it otherwise. Returns what was found
    /// BEFORE a rebuild when the index was found short of the store (fewer
    /// documents than nodes, or built over other fields), and `None` when it
    /// was reused, when it held what the store holds and was rebuilt only
    /// because nothing proved it current, or when there is no full-text.
    ///
    /// ⭐ PROVEN, NOT GUESSED (`req:a-designs-search-index-is-kept-with-its-store-and-reused-on-open`).
    /// The index is reused only when the voucher its last commit carries names
    /// this store exactly as it stood when this engine opened it (write
    /// sequence and anchor), this reflow2 version, this index format and the
    /// fields this build searches, and the graph is among those it vouches
    /// for ([`SearchTrust`]). Anything else is rebuilt, including an index
    /// whose document count equals the store's node count: a store restored
    /// from a backup, or rewritten by a build that keeps no index, can hold as
    /// many nodes saying different things, and a count cannot tell. Until
    /// 2026-11-22 the count was all an open compared, and the hosted server
    /// rebuilt on every open regardless, seconds on a large design, on the
    /// request path (`fact:root-cause-a-hub-wide-read-after-a-quiet-spell-reopens-and-reindexes-every-member-one-at-a-time-2026-11-22`).
    ///
    /// Called when a store is opened, because that is the moment a store meets
    /// an index it may not have been written with — a copied directory, an
    /// older build's store, a lost `fulltext/`, an index built over fewer
    /// fields than this build searches — and the one moment a rebuild is
    /// both cheap to justify and safe (nothing else holds the store yet). It
    /// is a property of every open rather than one caller's habit: the copy
    /// `--call` reads while another process holds a design was once opened
    /// through a path that skipped it, and searched an empty index.
    #[cfg(feature = "fulltext")]
    pub fn ensure_fulltext_covers(
        &self,
        graph_id: &str,
    ) -> Result<Option<FulltextCoverage>, DynoError> {
        if self.index_is_proven_for(graph_id) {
            return Ok(None);
        }
        let Some(found) = self.fulltext_coverage(graph_id)? else {
            return Ok(None);
        };
        self.reindex_fulltext(graph_id)?;
        // Vouch for the rebuilt index now, not only when this engine closes:
        // a process stopped without closing (killed, redeployed) then still
        // leaves an index the next open can reuse, as long as nothing was
        // written in between. Never fails the open: an index nobody vouched
        // for is only rebuilt again.
        if let Err(e) = self.vouch_for_search_index() {
            tracing::warn!("the rebuilt search index could not be vouched for: {e}");
        }
        Ok((!found.covers()).then_some(found))
    }

    /// Whether `graph_id`'s documents are KNOWN to match the store under the
    /// fields this build searches: vouched for by the index at open, or
    /// rebuilt since, with nothing since that could have put them out of step.
    #[cfg(feature = "fulltext")]
    fn index_is_proven_for(&self, graph_id: &str) -> bool {
        let Some(ti) = &self.text_index else {
            return false;
        };
        let stamp = self.schema.fulltext_stamp();
        let trust = self
            .search_trust
            .lock()
            .expect("search trust lock poisoned");
        trust.graphs.get(graph_id) == Some(&stamp)
            && ti.built_under(graph_id).as_deref() == Some(stamp.as_str())
            && !self.index_is_suspect()
    }

    /// Whether anything this session did could have left the index out of
    /// step with the store: a failed write to the index, a write to the
    /// store that stopped before it reached the index, or a failure that
    /// could only be logged.
    #[cfg(feature = "fulltext")]
    fn index_is_suspect(&self) -> bool {
        self.unsettled_writes > 0
            || self.text_suspect.load(std::sync::atomic::Ordering::SeqCst)
            || self.text_index.as_ref().is_some_and(|ti| ti.faulted())
    }

    /// Note a full-text failure that could only be logged: nothing vouches
    /// for the index again this session.
    #[cfg(feature = "fulltext")]
    pub(super) fn index_went_suspect(&self) {
        self.text_suspect
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Write a voucher into the index for every graph KNOWN to match the
    /// store (see [`SearchTrust`]), so the next open of this store can reuse
    /// the index instead of rebuilding it. Returns whether a voucher stands
    /// afterwards.
    ///
    /// Writes nothing when nothing can be vouched for: a backend that keeps
    /// no write sequence (in memory), an open batch, an index that may be out
    /// of step ([`index_is_suspect`](Self::index_is_suspect)), or no graph
    /// known to match. Writes nothing either when the voucher found at open,
    /// or written since, still stands: neither side has moved.
    ///
    /// When the store was written since its anchor was last recorded, a
    /// fresh anchor goes into the store first (one write), so a copy of this
    /// store written apart from it can never match this voucher. The voucher
    /// then names the sequence AFTER that write, read back from the store.
    #[cfg(feature = "fulltext")]
    fn vouch_for_search_index(&self) -> Result<bool, DynoError> {
        let Some(ti) = &self.text_index else {
            return Ok(false);
        };
        let Some(sequence_now) = self.backend.write_sequence() else {
            return Ok(false);
        };
        if self.is_batching() || self.index_is_suspect() {
            return Ok(false);
        }
        let stamp = self.schema.fulltext_stamp();
        let mut trust = self
            .search_trust
            .lock()
            .expect("search trust lock poisoned");
        let graphs: Vec<String> = trust
            .graphs
            .iter()
            .filter(|(g, s)| **s == stamp && ti.built_under(g).as_deref() == Some(stamp.as_str()))
            .map(|(g, _)| g.clone())
            .collect();
        if graphs.is_empty() {
            return Ok(false);
        }
        if trust.standing == Some((sequence_now, ti.commits())) {
            return Ok(true);
        }
        let anchor = match &trust.mark {
            Some(mark) if mark.sequence == sequence_now => mark.anchor.clone(),
            _ => {
                let fresh = uuid::Uuid::new_v4().to_string();
                self.backend.set_index_anchor(fresh.as_bytes())?;
                Some(fresh)
            }
        };
        let sequence = self.backend.write_sequence().unwrap_or(sequence_now);
        let voucher = IndexVoucher {
            format: SEARCH_INDEX_FORMAT,
            reflow2: env!("CARGO_PKG_VERSION").to_string(),
            stamp,
            store_sequence: sequence,
            store_anchor: anchor.clone(),
            graphs,
        };
        let payload =
            serde_json::to_string(&voucher).map_err(|e| DynoError::Serialization(e.to_string()))?;
        ti.commit_with_payload(&payload)
            .map_err(|e| DynoError::Storage(format!("full-text voucher commit failed: {e}")))?;
        trust.mark = Some(StoreMark { sequence, anchor });
        trust.standing = Some((sequence, ti.commits()));
        Ok(true)
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
            // Only now, after the commit: the record vouches for documents
            // that exist.
            ti.mark_built_under(graph_id, &self.schema.fulltext_stamp())
                .map_err(|e| DynoError::Storage(e.to_string()))?;
            // And this engine now KNOWS the graph's documents match the
            // store: it just derived them from it, under no open batch.
            self.search_trust
                .lock()
                .expect("search trust lock poisoned")
                .graphs
                .insert(graph_id.to_string(), self.schema.fulltext_stamp());
            Ok(count)
        };
        // Best-effort rollback to drop the uncommitted batch on failure;
        // surface the original rebuild error regardless of the rollback result.
        self.fulltext_rebuilds
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        rebuild().inspect_err(|_| {
            let _ = ti.rollback();
        })
    }

    /// How many times this engine has rebuilt a graph's full-text index from
    /// the store, by any path (an open that could not reuse what it found,
    /// or an explicit `reindex_fulltext`). `0` after an open means the index
    /// the store kept was reused as it was.
    #[cfg(feature = "fulltext")]
    pub fn fulltext_rebuilds(&self) -> u64 {
        self.fulltext_rebuilds
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// The version of how reflow2 derives a search index from a store: which
/// properties become a document, how they are joined, the Tantivy document
/// schema. A voucher written under another number is never trusted. Raise it
/// with any change to the derivation that a version bump alone would not
/// mark (the reflow2 version is checked too).
#[cfg(feature = "fulltext")]
pub(super) const SEARCH_INDEX_FORMAT: u32 = 1;

/// The store's write sequence and anchor at one moment.
#[cfg(feature = "fulltext")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StoreMark {
    pub(super) sequence: u64,
    pub(super) anchor: Option<String>,
}

/// The payload of the index commit that vouches for it: "the documents of
/// `graphs`, committed with this, were derived from the store at write
/// sequence `store_sequence` and anchor `store_anchor`, by reflow2
/// `reflow2`, index format `format`, over the fields `stamp` names."
#[cfg(feature = "fulltext")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct IndexVoucher {
    format: u32,
    reflow2: String,
    stamp: String,
    store_sequence: u64,
    store_anchor: Option<String>,
    graphs: Vec<String>,
}

/// Open the search index kept at `dir`, setting aside and recreating one
/// that cannot be read. The index is a copy derived from the store, so an
/// unreadable one costs a rebuild, never the open. The rebuild that follows
/// is reported (`ensure_fulltext_covers` finds the new index short of the
/// store), so this is not a silent repair. An index ANOTHER WRITER holds is
/// left alone and refuses, as before.
#[cfg(all(feature = "fulltext", feature = "rocksdb"))]
pub(super) fn open_kept_index(
    dir: &std::path::Path,
) -> Result<crate::foundation::text::TextIndex, DynoError> {
    use crate::foundation::text::TextIndex;
    let failed = |e: crate::foundation::text::TextError| {
        DynoError::Storage(format!("full-text index open failed: {e}"))
    };
    match TextIndex::open(dir) {
        Ok(ti) => Ok(ti),
        Err(e) if e.is_held_by_another_writer() || !dir.exists() => Err(failed(e)),
        Err(e) => {
            tracing::warn!(
                "the search index at {} could not be read ({e}); it is a copy derived from the \
                 store, so it is set aside and rebuilt",
                dir.display()
            );
            std::fs::remove_dir_all(dir).map_err(|io| {
                DynoError::Storage(format!(
                    "full-text index open failed: {e}; and the unreadable index at {} could \
                     not be set aside to rebuild it: {io}",
                    dir.display()
                ))
            })?;
            TextIndex::open(dir).map_err(failed)
        }
    }
}

/// What an engine opening `backend` with `text_index` knows before it writes
/// anything: the store's mark, and the graphs the index's voucher proves,
/// when its voucher names this store exactly as it stands.
#[cfg(all(feature = "fulltext", feature = "rocksdb"))]
pub(super) fn trust_at_open(
    backend: &dyn KvBackend,
    text_index: Option<&crate::foundation::text::TextIndex>,
    schema: &Schema,
) -> Result<SearchTrust, DynoError> {
    let Some(sequence) = backend.write_sequence() else {
        return Ok(SearchTrust::default());
    };
    let anchor = backend
        .index_anchor()?
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
    let mark = StoreMark { sequence, anchor };
    let mut trust = SearchTrust {
        mark: Some(mark.clone()),
        ..SearchTrust::default()
    };
    let Some(ti) = text_index else {
        return Ok(trust);
    };
    // A payload that cannot be read, or is not a voucher, vouches for nothing.
    let voucher = ti
        .committed_payload()
        .ok()
        .flatten()
        .and_then(|p| serde_json::from_str::<IndexVoucher>(&p).ok());
    if let Some(v) = voucher
        && v.format == SEARCH_INDEX_FORMAT
        && v.reflow2 == env!("CARGO_PKG_VERSION")
        && v.stamp == schema.fulltext_stamp()
        && v.store_sequence == mark.sequence
        && v.store_anchor == mark.anchor
    {
        trust.graphs = v.graphs.into_iter().map(|g| (g, v.stamp.clone())).collect();
        trust.standing = Some((mark.sequence, ti.commits()));
    }
    Ok(trust)
}

#[cfg(feature = "fulltext")]
impl Drop for StorageEngine {
    /// A store's engine closing VOUCHES FOR ITS SEARCH INDEX, so the next
    /// open can reuse it (`vouch_for_search_index`). Best effort: a failure
    /// is logged, and costs the next open a rebuild. Nothing is vouched for
    /// while a panic unwinds, when a write may have stopped halfway.
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }
        if let Err(e) = self.vouch_for_search_index() {
            tracing::warn!("the search index could not be vouched for on close: {e}");
        }
    }
}
