//! `StorageEngine` — write-buffer batching. Split out of `engine.rs`; `use super::*`
//! inherits the shared imports and types from the parent `engine` module.
//! Private helper methods live in `engine/mod.rs` (a parent module, so
//! these methods reach them as descendants).
//!
//! # Batches nest
//!
//! A batch begun while one is open is an INNER batch, held as a savepoint in
//! the one buffer: its commit hands its writes to the batch around it, and its
//! discard drops only what it wrote. Only the OUTERMOST commit reaches the
//! backend. Until 2026-10-02 a second `begin_batch` committed the first, with
//! a warning — harmless while batches were opened only by a bulk form, an
//! import or HEAL, which never sat inside one another. A tool call's write
//! unit (`dec:idea-a-refused-typed-write-stores-nothing`) is the batch every
//! one of those now sits inside, and committing it early would make a refused
//! call's earlier writes durable.

use super::*;

impl StorageEngine {
    /// Begin buffering writes. All subsequent `put()` calls will be buffered
    /// instead of committed immediately. Call `commit_batch()` to write all
    /// buffered operations atomically.
    ///
    /// Inside an open batch this opens an INNER one (see the module note):
    /// nothing is committed, and the matching `commit_batch` / `discard_batch`
    /// settles only the inner batch.
    pub fn begin_batch(&mut self) {
        if let Some(buffer) = &self.write_buffer {
            self.savepoints.push(buffer.len());
            return;
        }
        // Establish a clean full-text baseline before the batch buffers its own
        // ops. `discard_batch` reverts via a writer-global `rollback()`, which is
        // only correct if the writer holds *exactly* this batch's ops. A prior
        // non-batched write whose index commit failed (the node is in RocksDB,
        // its index op left uncommitted) would otherwise be dropped by that
        // rollback — silent, permanent drift. Committing here flushes any such
        // stranded op (it matches a durable RocksDB write, so committing is the
        // correct heal) and guarantees the writer is clean when buffering starts.
        #[cfg(feature = "fulltext")]
        {
            if let Some(ti) = &self.text_index
                && let Err(e) = ti.commit()
            {
                tracing::error!("full-text commit at begin_batch failed: {e}");
            }
            *self.text_batch.lock().expect("text batch lock poisoned") = TextBatch::default();
        }
        self.write_buffer = Some(WriteBuffer::default());
    }

    /// Returns true if write batching is currently active.
    pub fn is_batching(&self) -> bool {
        self.write_buffer.is_some()
    }

    /// How many batches are open: `0` none, `1` the outermost alone, more
    /// when batches are nested inside it.
    pub fn batch_depth(&self) -> usize {
        if self.write_buffer.is_some() {
            1 + self.savepoints.len()
        } else {
            0
        }
    }

    /// Commit all buffered writes as a single atomic operation.
    /// For RocksDB, this uses `WriteBatch` for atomic multi-CF writes.
    /// For in-memory backend, applies writes directly.
    ///
    /// Settling an INNER batch writes nothing: its ops stay in the buffer and
    /// belong to the batch around it from here on. The count returned is the
    /// number of ops the settled batch contributed.
    pub fn commit_batch(&mut self) -> Result<usize, DynoError> {
        if let Some(at) = self.savepoints.pop() {
            let len = self.write_buffer.as_ref().map_or(0, WriteBuffer::len);
            return Ok(len.saturating_sub(at));
        }
        let buffer = match self.write_buffer.take() {
            Some(b) => b,
            None => return Ok(0),
        };
        #[cfg(feature = "fulltext")]
        let text = std::mem::take(&mut *self.text_batch.lock().expect("text batch lock poisoned"));

        let count = buffer.len();
        if count == 0 {
            // Nothing for the backend, but the index may still hold text this
            // batch re-derived after an inner discard: make it current.
            #[cfg(feature = "fulltext")]
            if text.pending
                && let Some(ti) = &self.text_index
            {
                ti.commit().map_err(|e| {
                    DynoError::Storage(format!("full-text batch commit failed: {e}"))
                })?;
            }
            return Ok(0);
        }

        // Invalidate cache before applying the batch so a concurrent
        // reader either sees pre-batch + cache-miss (re-fetches) or
        // post-batch + cache-miss — never stale data.
        {
            let mut cache = self.read_cache.lock().expect("read_cache lock poisoned");
            for op in buffer.ops() {
                match op {
                    BufferedOp::Put { key, .. } | BufferedOp::Delete { key, .. } => {
                        cache.invalidate(key);
                    }
                    BufferedOp::PrefixDelete { prefix, .. } => {
                        cache.invalidate_prefix(prefix);
                    }
                }
            }
        }

        // Apply the buffered ops atomically. The backend owns the
        // all-or-nothing semantics (RocksDB `WriteBatch`; the in-memory
        // backend an in-order loop) and the `PrefixDelete`-supersedes-
        // earlier-puts ordering.
        if let Err(e) = self.backend.commit_batch(buffer.into_ops()) {
            // Nothing landed, so the index must not say otherwise — including
            // any of this batch's text a search inside it already published.
            #[cfg(feature = "fulltext")]
            self.revert_text(&text);
            return Err(e);
        }

        // Make this batch's buffered full-text writes visible now that the
        // authoritative backend write has landed. Only when the batch left
        // text pending: a batch whose text a search already published, and
        // that wrote none since, has nothing left to make visible.
        #[cfg(feature = "fulltext")]
        if text.pending
            && let Some(ti) = &self.text_index
        {
            ti.commit()
                .map_err(|e| DynoError::Storage(format!("full-text batch commit failed: {e}")))?;
        }

        Ok(count)
    }

    /// Discard all buffered writes without committing.
    ///
    /// Discarding an INNER batch drops only the ops it buffered, back to the
    /// savepoint it began at; the batch around it keeps its own.
    pub fn discard_batch(&mut self) {
        if let Some(at) = self.savepoints.pop() {
            if let Some(buffer) = self.write_buffer.as_mut() {
                buffer.truncate(at);
            }
            self.view_moved();
            #[cfg(feature = "fulltext")]
            self.rederive_text_after_inner_discard();
            return;
        }
        if self.write_buffer.take().is_none() {
            return;
        }
        self.view_moved();
        // Revert the batch's buffered full-text writes too. Best-effort: a
        // rollback failure can't be surfaced through this infallible signature,
        // and the index is rebuildable via `reindex_fulltext` regardless.
        #[cfg(feature = "fulltext")]
        {
            let text =
                std::mem::take(&mut *self.text_batch.lock().expect("text batch lock poisoned"));
            self.revert_text(&text);
        }
    }

    /// Make the open batch's pending text searchable — see `TextBatch`. A
    /// no-op outside a batch, and when nothing is pending.
    #[cfg(feature = "fulltext")]
    pub(super) fn publish_text(&self) -> Result<(), DynoError> {
        if self.write_buffer.is_none() {
            return Ok(());
        }
        let Some(ti) = &self.text_index else {
            return Ok(());
        };
        let mut tb = self.text_batch.lock().expect("text batch lock poisoned");
        if tb.pending {
            ti.commit().map_err(|e| {
                DynoError::Storage(format!("full-text commit inside a batch failed: {e}"))
            })?;
            tb.pending = false;
            tb.published = true;
        }
        Ok(())
    }

    /// Put the index back as the store holds it after the outermost batch was
    /// discarded, or failed to commit. Best effort, for the reason
    /// `discard_batch` gives; a failure is logged and `reindex_fulltext`
    /// (run at every server start) repairs it.
    #[cfg(feature = "fulltext")]
    fn revert_text(&self, text: &TextBatch) {
        let Some(ti) = &self.text_index else {
            return;
        };
        if let Err(e) = ti.rollback() {
            tracing::error!("full-text rollback after discard_batch failed: {e}");
        }
        if !text.published {
            // None of the batch's text was ever committed, so the rollback
            // alone restored the index.
            return;
        }
        let restored = self.rederive_text(text).and_then(|()| {
            ti.commit()
                .map_err(|e| DynoError::Storage(format!("full-text commit failed: {e}")))
        });
        if let Err(e) = restored {
            tracing::error!(
                "full-text restore after a discarded batch failed: {e} — the index may name \
                 nodes that were never written until reindex_fulltext runs"
            );
        }
    }

    /// After an inner discard the writer's uncommitted ops still include the
    /// discarded ones, and anything a search published may name them too. So
    /// roll the writer back and re-derive every document the batch touched
    /// from the view that remains (the outer batch over the backend).
    #[cfg(feature = "fulltext")]
    fn rederive_text_after_inner_discard(&self) {
        let Some(ti) = &self.text_index else {
            return;
        };
        let mut tb = self.text_batch.lock().expect("text batch lock poisoned");
        if tb.touched.is_empty() && tb.cleared.is_empty() {
            return;
        }
        if let Err(e) = ti.rollback() {
            tracing::error!("full-text rollback after an inner discard failed: {e}");
        }
        if let Err(e) = self.rederive_text(&tb) {
            tracing::error!(
                "full-text re-derive after an inner discard failed: {e} — reindex_fulltext \
                 repairs the index"
            );
        }
        tb.pending = true;
    }

    /// Write every document `text` touched into the index as the store's
    /// CURRENT view holds it: present nodes upserted, absent ones deleted, and
    /// a cleared graph rebuilt from its nodes. Leaves the writer uncommitted.
    #[cfg(feature = "fulltext")]
    fn rederive_text(&self, text: &TextBatch) -> Result<(), DynoError> {
        let Some(ti) = &self.text_index else {
            return Ok(());
        };
        let storage = |e: crate::foundation::text::TextError| {
            DynoError::Storage(format!("full-text re-derive failed: {e}"))
        };
        for graph_id in &text.cleared {
            ti.delete_graph(graph_id).map_err(storage)?;
            let node_types: Vec<String> = self
                .schema
                .node_types
                .keys()
                .filter(|nt| self.schema.has_fulltext_properties(nt))
                .cloned()
                .collect();
            for node_type in node_types {
                for node in self.scan_nodes(graph_id, &node_type)? {
                    let fields = self.fulltext_fields(&node_type, &node.properties);
                    ti.upsert(graph_id, &node_type, &node.node_id, &fields)
                        .map_err(storage)?;
                }
            }
        }
        for ((graph_id, node_id), node_type) in &text.touched {
            if text.cleared.contains(graph_id) {
                continue;
            }
            match self.get_node(graph_id, node_type, node_id)? {
                Some(node) => {
                    let fields = self.fulltext_fields(node_type, &node.properties);
                    ti.upsert(graph_id, node_type, node_id, &fields)
                        .map_err(storage)?;
                }
                None => ti.delete(graph_id, node_id).map_err(storage)?,
            }
        }
        Ok(())
    }
}
