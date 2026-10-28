//! The whole design as one portable document, and back again.
//!
//! Three jobs, one mechanism:
//!
//! - **Migration.** Export with the old binary, import with the new one. This is
//!   the general answer to a schema or storage-format change, and a far better
//!   one than bespoke backfill code written per change.
//! - **Backup.** A design graph is small — hundreds to low thousands of nodes —
//!   so keeping every version costs almost nothing.
//! - **Portability.** Move a design between machines, or hand one to somebody.
//!
//! # Deterministic on purpose
//!
//! Everything is sorted: node types, ids, edges, and property keys (which is why
//! the exported types use [`BTreeMap`] rather than the `HashMap` the store
//! hands back). Two exports of an unchanged graph are byte-identical.
//!
//! That is not tidiness. It is what makes the file diffable, so a backup
//! directory under version control shows *what changed in the design* between
//! two points rather than a fresh blob each time. A `HashMap`'s iteration order
//! is seeded per process, so an unsorted export would rewrite itself completely
//! on every run and the history would be worthless.
//!
//! # Not the temporal axis
//!
//! `DesignEpoch` / `Snapshot` / `ChangeEvent` record *why* the design changed,
//! semantically, inside the graph. This records the graph's contents at a point
//! in time. Neither substitutes for the other: the temporal axis cannot recover
//! a corrupted store, and an export cannot explain a requirement's history.

use std::collections::{BTreeMap, BTreeSet};

use crate::foundation::core::{DynoError, Value};
use crate::foundation::store::{StoredEdge, StoredNode};
use serde::{Deserialize, Serialize};

use crate::graph::DesignGraph;
use crate::nodes::{edge, node};
use crate::provenance::GraphStamp;

/// Sorted property bag — `BTreeMap` so the JSON is byte-stable.
pub type Props = BTreeMap<String, Value>;

/// A node, as it appears in an export.
///
/// ON DISK the server's stamps (`server_created_at`, `server_changed_at`,
/// `server_seal`) sit in `properties`, where every older reader already
/// accepts a property it does not know. IN MEMORY they are held apart, in
/// `stamps`, so nothing that reads or compares a node's properties — a diff
/// between two designs, a mirror's refresh, a merge — sees a stamp as content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "NodeOnDisk", into = "NodeOnDisk")]
pub struct ExportedNode {
    pub node_type: String,
    pub node_id: String,
    pub properties: Props,
    /// The server's stamps the node carried where it was exported, as stored.
    /// An import keeps them as stated; their seal decides how they read.
    pub stamps: Props,
}

/// [`ExportedNode`] as its file holds it.
#[derive(Clone, Serialize, Deserialize)]
struct NodeOnDisk {
    node_type: String,
    node_id: String,
    #[serde(default)]
    properties: Props,
}

impl From<NodeOnDisk> for ExportedNode {
    fn from(d: NodeOnDisk) -> Self {
        Self::from_disk(d.node_type, d.node_id, d.properties)
    }
}

impl From<ExportedNode> for NodeOnDisk {
    fn from(n: ExportedNode) -> Self {
        let mut properties = n.properties;
        properties.extend(n.stamps);
        Self {
            node_type: n.node_type,
            node_id: n.node_id,
            properties,
        }
    }
}

impl ExportedNode {
    /// A node from properties as a file holds them: any stamp among them is
    /// moved into `stamps`.
    pub fn from_disk(node_type: String, node_id: String, mut properties: Props) -> Self {
        let mut stamps = Props::new();
        for k in crate::foundation::store::STAMP_KEYS {
            if let Some(v) = properties.remove(k) {
                stamps.insert(k.to_string(), v);
            }
        }
        Self {
            node_type,
            node_id,
            properties,
            stamps,
        }
    }
}

impl From<StoredNode> for ExportedNode {
    fn from(n: StoredNode) -> Self {
        Self {
            node_type: n.node_type,
            node_id: n.node_id,
            properties: n.properties.into_iter().collect(),
            stamps: Props::new(),
        }
    }
}

/// An edge, as it appears in an export.
///
/// Endpoint *types* are not stored: they are recoverable from the nodes in the
/// same document, and duplicating them would let a file disagree with itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportedEdge {
    pub edge_type: String,
    pub from_id: String,
    pub to_id: String,
    #[serde(default)]
    pub properties: Props,
}

impl From<StoredEdge> for ExportedEdge {
    fn from(e: StoredEdge) -> Self {
        Self {
            edge_type: e.edge_type,
            from_id: e.from_id,
            to_id: e.to_id,
            properties: e.properties.into_iter().collect(),
        }
    }
}

/// The working tree an export was TAKEN in — which branch, at which commit,
/// and whether that tree held uncommitted work besides the export itself.
///
/// Recorded once per export by the layer that may read git (the MCP server);
/// the core takes no clock and does no I/O, so this is caller-supplied and
/// absent means nobody said (an export from outside a repository, or from
/// before 2026-09-16). It answers the question the phantom-drift incident
/// asked — *which branch was this export taken on?* — at the one grain where
/// it is true: an export is a property of a working tree, and a node is not
/// (dec:idea-should-a-node-carry-its-git-coordinate, option ②).
///
/// ⚠️ `commit` CANNOT mean "the commit that contains this file": writing the
/// hash into the file changes the file, which changes the commit. It means
/// the commit the tree was AT when the design content was last exported —
/// the parent of whatever commit later carries it. And it is kept, not
/// bumped, while the design content is unchanged, so an export that did not
/// move stays byte-identical across commits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TakenAt {
    /// The branch checked out, or None on a detached HEAD.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// HEAD at the time — the tree the design content was taken from.
    pub commit: String,
    /// Whether the working tree held uncommitted changes OTHER than the
    /// export file itself when the export was taken.
    #[serde(default)]
    pub dirty: bool,
}

/// A whole design graph, portable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphExport {
    /// Where this export was taken — see [`TakenAt`]. Not part of the content
    /// hash: two exports of one design taken on different branches are the
    /// same design.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taken_at: Option<TakenAt>,
    /// Which reflow2 wrote it. Carried so an import can tell whether the file
    /// came from a vocabulary it does not know — the same question
    /// [`crate::provenance`] asks of a graph directory. Optional on the way in,
    /// the sibling rule to `content_hash`: a hand-authored or third-party
    /// document legitimately has no stamp, and its absence is a first-class,
    /// reported state (see [`ImportReport::provenance_note`]), never a reason to
    /// refuse the document (BL-87). Every export reflow2 writes carries one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stamp: Option<GraphStamp>,
    /// Fingerprint of the design content — see [`GraphExport::content_hash`].
    /// Deliberately excludes the stamp and the chain fields, so the same
    /// design hashes identically whichever build wrote it and whatever it
    /// claims about its ancestry. A document whose embedded hash does not
    /// match its own content was edited outside reflow2 or corrupted
    /// (`dec:export-hash-chain`); absence means it predates hashing, which is
    /// reported, never an error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    /// The content hash of the export this one superseded — file lineage,
    /// recorded at the file-write seam when an export replaces an existing
    /// export file with *changed* content (unchanged content keeps the old
    /// chain, so two exports of an unchanged design stay byte-identical).
    /// What lets `compare_designs` answer "does other descend from base?".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_content_hash: Option<String>,
    /// Which design wrote it. **Optional on the way in (BL-138)**, and the
    /// reason is that `import_graph` never reads it: an import loads into the
    /// receiving graph, whose id the server already knows, so demanding it from
    /// a hand author asks the caller to restate the receiver's own identity —
    /// and then ignores the answer. The `adopt` skill's central instruction is
    /// *"build one export document and `import_graph` it once"*, and following
    /// it literally failed on `missing field 'graph_id'`.
    ///
    /// **`mirror_surface` DOES read it and still refuses a document without
    /// one**, because the two operations genuinely differ: mirroring records
    /// where a surface came from (`mirror_of`) and guards against mirroring a
    /// design into itself, neither of which is answerable from an unidentified
    /// document. Distinguishing them is the point — the same shape as [BL-119],
    /// where one requirement was right for a round-tripped export and wrong for
    /// a hand-authored one.
    ///
    /// Empty means unidentified; see [`GraphExport::is_unidentified`]. Every
    /// export reflow2 writes carries a real one.
    #[serde(default)]
    pub graph_id: String,
    /// Defaulted so a document may legitimately carry no nodes — and so that a
    /// hand-authored `{"nodes": [...]}` with no edge list is accepted rather
    /// than refused for omitting an empty array.
    #[serde(default)]
    pub nodes: Vec<ExportedNode>,
    /// See [`GraphExport::nodes`].
    #[serde(default)]
    pub edges: Vec<ExportedEdge>,
}

impl GraphExport {
    /// Whether the document arrived with no stamp — a hand-authored or
    /// third-party document rather than one reflow2 exported (BL-87).
    pub fn is_unstamped(&self) -> bool {
        self.stamp.is_none()
    }

    /// Whether the document names no source design (BL-138). Harmless for
    /// `import_graph`, which never reads it; disqualifying for
    /// `mirror_surface`, which cannot record where a surface came from without
    /// it. Reported, never guessed at.
    pub fn is_unidentified(&self) -> bool {
        self.graph_id.is_empty()
    }

    /// The reflow2 version that wrote this document, or `"unstamped"` when it
    /// carries no stamp — so callers reporting provenance (compare, merge) read
    /// one value and never unwrap an absent stamp.
    pub fn reflow2_version(&self) -> &str {
        self.stamp
            .as_ref()
            .map(|s| s.reflow2_version.as_str())
            .unwrap_or("unstamped")
    }

    /// The canonical content fingerprint: sha256 over the compact,
    /// sorted-key JSON of `{"edges", "graph_id", "nodes"}` — the design
    /// content only. Full hash, not truncated like display checksums: this is
    /// tamper evidence, and the chain's identity.
    ///
    /// Canonical means: object keys sorted (serde_json's default map is a
    /// `BTreeMap`), compact separators, minimal escaping — the same form
    /// Python's `json.dumps(…, sort_keys=True, ensure_ascii=False,
    /// separators=(",", ":"))` produces, so the stdlib CI gate can recompute
    /// it; `tools/smoke_mcp.py` pins the two implementations against each
    /// other.
    pub fn compute_content_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let canonical = serde_json::json!({
            "edges": self.edges,
            "graph_id": self.graph_id,
            "nodes": self.nodes,
        });
        let text = serde_json::to_string(&canonical)
            .expect("export content is JSON-serializable by construction");
        let digest = Sha256::digest(text.as_bytes());
        let mut hex = String::with_capacity(7 + 64);
        hex.push_str("sha256:");
        for b in digest {
            use std::fmt::Write as _;
            let _ = write!(hex, "{b:02x}");
        }
        hex
    }

    /// The document's content hash for chain purposes: the embedded one when
    /// present, recomputed otherwise — so a pre-hashing document still has an
    /// identity and the chain can grow from it.
    pub fn effective_content_hash(&self) -> String {
        self.content_hash
            .clone()
            .unwrap_or_else(|| self.compute_content_hash())
    }

    /// Does the embedded `content_hash` match the actual content? `None` when
    /// the document carries no hash (predates hashing) — three-valued on
    /// purpose, because "unhashed" and "tampered" are different facts.
    pub fn verify_content_hash(&self) -> Option<bool> {
        self.content_hash
            .as_ref()
            .map(|h| *h == self.compute_content_hash())
    }

    /// Set the lineage link relative to the export file this document is
    /// about to replace (`dec:export-hash-chain`): content changed → the
    /// chain advances to the predecessor's hash; content unchanged → the
    /// predecessor's own chain is kept, so an unchanged design writes
    /// byte-identical files.
    pub fn chain_after(&mut self, predecessor: &GraphExport) {
        let prev_hash = predecessor.effective_content_hash();
        let own = self
            .content_hash
            .clone()
            .unwrap_or_else(|| self.compute_content_hash());
        self.prev_content_hash = if own == prev_hash {
            predecessor.prev_content_hash.clone()
        } else {
            Some(prev_hash)
        };
    }
}

/// What an import did. Reported rather than assumed — an import that quietly
/// skipped half a design would be the worst kind of success.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportReport {
    pub nodes_written: usize,
    pub edges_written: usize,
    /// Edges whose endpoints were not in the document and not already in the
    /// graph. Named, never dropped silently.
    pub skipped_edges: Vec<String>,
    /// Set when the document's embedded `content_hash` does not match its
    /// actual content — the file was edited outside reflow2 or corrupted.
    /// The import still proceeds (the human may know exactly why — a hand-
    /// resolved git merge, say) but never silently: deciding what a mismatch
    /// means is their call, seeing it is not optional. `None` for a matching
    /// hash and for pre-hashing documents alike — absence of a hash is not
    /// evidence of tampering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrity_note: Option<String>,
    /// Set when the document arrived with no `stamp` — a hand-authored or
    /// third-party document rather than one reflow2 exported. The import
    /// proceeds (the stamp is provenance metadata, not a gate — the sibling
    /// rule to an absent `content_hash`), but never silently: an unstamped
    /// document cannot be checked for an upgrade-direction mismatch, so the
    /// human should know what they loaded (BL-87). `None` for a stamped
    /// document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance_note: Option<String>,
    /// Set when this import ADOPTED the document's design identity — a restore
    /// into an empty store, which takes the design's own name rather than
    /// renaming it to the receiver's (BL-169).
    ///
    /// Reported rather than merely done, because the alternative is exactly the
    /// failure that filed the row: a design silently renamed, committed and
    /// pushed through a fully green pipeline, with `graph_id` sitting inside the
    /// export's content hash. `None` when nothing was adopted — an in-memory
    /// graph, a store that already holds a design, or a document whose name the
    /// store already carries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adopted_identity: Option<String>,
    /// Edges written as something other than they arrived, each named with
    /// what it became — every row of `narrowing::EDGE_REWRITES`. Today one: a
    /// legacy `Artifact REALIZES Verification` becomes IMPLEMENTS (2026-09-23,
    /// when REALIZES stopped accepting any target). Reported, never done in
    /// silence.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub migrated_edges: Vec<String>,
    /// Node-reference properties whose value names a node that is neither in
    /// the document nor already in the graph. Reported, never a refusal.
    ///
    /// ⭐ THE SIBLING OF `skipped_edges`, AND THE SAME QUESTION THAT FIELD
    /// ALREADY ANSWERS: an edge whose endpoint is missing is skipped and named
    /// rather than failing the restore, and a property naming a missing node is
    /// the other shape of exactly that. Answering it differently here would mean
    /// one import refusing what its neighbour two lines up reports.
    ///
    /// 🛑 WHY A RESTORE MUST NOT REFUSE THESE, though `create_node` does. The
    /// write guard exists to stop a NEW record being made about a node the
    /// design does not have. An import asserts nothing new — it reproduces a
    /// state that already existed, and the document is the evidence of it.
    /// Refusing would mean a graph holding one dangling reference has an
    /// UNRESTORABLE BACKUP, and the remedy on offer would be hand-editing the
    /// export, which is the one thing this project tells users never to do.
    /// Measured 2026-08-29: this design's own export carried nine, all written
    /// before the guard existed — five instances of a `prj:`/`proj:` typo and
    /// four capabilities renamed after the fact was written.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub dangling_node_refs: Vec<String>,
    /// What the store now holds that the document did NOT state, counted by
    /// `Type.property` (nodes) and `EDGE_TYPE.property` (edges): the schema
    /// defaults this reflow2 materialised and the migrations it applied (an
    /// AUTHORED_BY `role` becoming `roles`, say).
    ///
    /// On a document written by THIS version these are the defaults the export
    /// left implicit on purpose (`cap:defaults-are-not-serialized`) and the
    /// round trip stays byte-identical. On a document written by a NEWER
    /// reflow2 they are the hazard `ImportOptions::accept_newer` gates: on
    /// 2026-09-14 an 0.59.0 binary imported a 0.60.2 record and 853 AUTHORED_BY
    /// edges gained `role: author`, and no report said so. Now one does.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[serde(default)]
    pub materialized: BTreeMap<String, usize>,
    /// Relations the document held twice and out of step, brought into step
    /// with their authority as the import landed (`crate::twins`), each
    /// named. A document written before the store kept these copies usually
    /// carries some: findings with no `HAS_TEMPORAL_FACT` edge, a finding
    /// hung by hand off a second node, a Flow's stored exit.
    ///
    /// Reported, never refused, for the reason `dangling_node_refs` gives: a
    /// restore reproduces a state that existed, and refusing would leave a
    /// design with an unrestorable backup. It is also what makes the CI gate's
    /// round trip FAIL on a committed export that is out of step. The document
    /// and its restore then differ, and this field says where.
    #[serde(skip_serializing_if = "crate::twins::TwinRepairs::is_empty")]
    #[serde(default)]
    pub twin_repairs: crate::twins::TwinRepairs,
    /// Properties a node ALREADY IN THE DESIGN held before the import and does
    /// not hold after it, counted by `Type.property`. Whole-node import (the
    /// default) removes what a document leaves out; this is where that is
    /// said, so a partial document can never silently wipe data.
    ///
    /// Measured 2026-10-20 in hxm_program: 454 Artifacts imported as
    /// `{name, location}` lost `checksum`, `checksum_basis` and `status`, and
    /// the reply said `nodes_written: 454` and nothing else
    /// (`req:an-import-says-what-it-removed-and-merges-only-when-asked`).
    /// Always empty under `merge`, which removes nothing.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[serde(default)]
    pub properties_removed: BTreeMap<String, usize>,
    /// Properties a node already in the design held with a DIFFERENT value
    /// after the import, counted by `Type.property` — whether the document
    /// stated the new value or the schema's default replaced one the document
    /// left out. A value overwritten is counted here and never under
    /// `materialized`: until 2026-10-21 an `artifact_type` reset from `config`
    /// to the default `code`, and an accepted Requirement's `status` reset to
    /// `proposed`, were reported as defaults filled in.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[serde(default)]
    pub properties_changed: BTreeMap<String, usize>,
    /// A few of the existing nodes that lost properties, each with what it
    /// lost — at most [`REMOVED_FROM_SHOWN`], the count of the rest said in
    /// the last line — so the caller can look before moving on.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub removed_from: Vec<String>,
    /// Said whenever anything was removed: what happened and how to keep what
    /// a partial document leaves out next time.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub removed_note: Option<String>,
}

/// How an import may deviate from the safe default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportOptions {
    /// Import a document stamped by a reflow2 NEWER than this crate. Refused
    /// by default — see [`DesignGraph::import_graph_with`].
    pub accept_newer: bool,
    /// Merge each node the design already holds: the document's properties
    /// overwrite, and every stored property the document leaves out SURVIVES,
    /// as `upsert_node` (create_node, create_nodes) does. Off by default, so a
    /// document is a complete statement of the nodes it names: an export→import
    /// round trip reproduces a design, and a document can remove a property
    /// (`dec:import-merges-or-demands-whole-nodes`, accepted 2026-10-21: whole
    /// nodes unless the call asks to merge).
    pub merge: bool,
}

/// How many existing nodes an import names in `removed_from`, at most.
pub const REMOVED_FROM_SHOWN: usize = 5;

impl DesignGraph {
    /// Export the whole graph, deterministically.
    ///
    /// Walks every node type the schema declares, then each node's outgoing
    /// edges — so every edge is visited exactly once, from its source.
    ///
    /// The server's stamps stay in the store and are NOT in it
    /// (`dec:write-stamps-stay-in-the-store-and-travel-in-an-export-only-when-asked-2026-10-28`):
    /// two exports of the same design built apart stay byte-identical, and two
    /// branches making the same change to one item still merge. Ask for them
    /// with [`export_graph_with_stamps`](Self::export_graph_with_stamps).
    pub fn export_graph(&self) -> Result<GraphExport, DynoError> {
        self.export_graph_carrying(false)
    }

    /// [`export_graph`](Self::export_graph) carrying each node's server stamps,
    /// for a backup or a move that should keep them. An import keeps them as
    /// stated, and they read as recorded here only in the store whose key
    /// sealed them; anywhere else they read "imported, not recorded here".
    pub fn export_graph_with_stamps(&self) -> Result<GraphExport, DynoError> {
        self.export_graph_carrying(true)
    }

    fn export_graph_carrying(&self, stamps: bool) -> Result<GraphExport, DynoError> {
        let mut node_types: Vec<&String> = self.schema().node_types.keys().collect();
        node_types.sort();

        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for t in node_types {
            let mut found: Vec<StoredNode> = self.scan_nodes(t)?;
            found.sort_by(|a, b| a.node_id.cmp(&b.node_id));
            for n in found {
                for e in self.outgoing(&n.node_id, None)? {
                    edges.push(ExportedEdge::from(e));
                }
                // Asked for, the server's stamps travel with the node (in
                // its properties on disk, where every older reader already
                // accepts them); an import keeps them as stated and their seal
                // decides how they read there.
                let carried = if stamps {
                    self.stamp_properties(&n.node_type, &n.node_id)?
                } else {
                    Vec::new()
                };
                let mut exported = ExportedNode::from(n);
                exported.stamps.extend(carried);
                nodes.push(exported);
            }
        }
        edges.sort_by(|a, b| {
            a.edge_type
                .cmp(&b.edge_type)
                .then(a.from_id.cmp(&b.from_id))
                .then(a.to_id.cmp(&b.to_id))
        });

        // A CHECKSUM THE RECORD ALREADY HOLDS ON ITS ACCEPTING CHANGE IS NOT
        // WRITTEN TWICE. Since 2026-10-03 an accept puts the checksum on the
        // accepting change's CHANGED edge (`checksum_after`), and the Artifact's
        // own `checksum` is the current one of those — derived, not a second
        // record. Kept on the node it would be ONE value every pull request that
        // edits the file rewrites, which is exactly the conflict decision 3 of
        // dec:item-13-checksums-move-to-change-edges-and-main-converts-in-one-pr
        // removes. Elided only while it EQUALS what the edges derive; a node
        // value that disagrees (a direct write) is kept, so nothing is lost, and
        // the import puts the derived value back.
        let current = current_acceptances(&edges);
        for n in nodes.iter_mut().filter(|n| n.node_type == node::ARTIFACT) {
            let Some((checksum, basis)) = current.get(&n.node_id) else {
                continue;
            };
            let stated = n.properties.get("checksum").and_then(Value::as_str);
            if stated != Some(checksum.as_str()) {
                continue;
            }
            n.properties.remove("checksum");
            if basis.is_some()
                && n.properties.get("checksum_basis").and_then(Value::as_str) == basis.as_deref()
            {
                n.properties.remove("checksum_basis");
            }
        }

        let mut export = GraphExport {
            taken_at: None,
            stamp: Some(GraphStamp::current(self.schema())),
            content_hash: None,
            prev_content_hash: None,
            graph_id: self.graph_id().to_string(),
            nodes,
            edges,
        };
        export.content_hash = Some(export.compute_content_hash());
        Ok(export)
    }

    /// Load an exported design into this graph, atomically.
    ///
    /// Upsert, not replace: a node id already present is overwritten, and
    /// anything already in the graph and absent from the document is left
    /// alone. Clearing first is the caller's decision, not a side effect of
    /// importing.
    ///
    /// Everything lands in one batch, so a document that fails validation
    /// half-way leaves the graph untouched rather than half-loaded.
    pub fn import_graph(&mut self, doc: &GraphExport) -> Result<ImportReport, DynoError> {
        self.import_graph_with(doc, ImportOptions::default())
    }

    /// [`import_graph`](Self::import_graph) with options.
    ///
    /// 🛑 A DOCUMENT WRITTEN BY A NEWER reflow2 IS REFUSED unless
    /// `accept_newer`. The vocabulary may read fine — that is not the hazard.
    /// The hazard is what THIS binary writes: every property the newer reflow2
    /// left implicit gets this binary's default, which may be a default the
    /// newer one removed. Measured 2026-09-14: an 0.59.0 server imported a
    /// 0.60.2 record and 853 AUTHORED_BY edges gained `role: author`, silently
    /// (`fact:root-cause-the-853-is-an-older-binary-writing-the-old-role-default-
    /// onto-a-newer-document-not-the-export-leaving-a-default-implicit`). The
    /// import compared no versions; the store's own open-time guard compares
    /// vocabulary, which was identical. This is the mirror of the stale-export
    /// refusal: a record is written by code that is on disk, and read by code
    /// that is at least as new as what wrote it.
    pub fn import_graph_with(
        &mut self,
        doc: &GraphExport,
        options: ImportOptions,
    ) -> Result<ImportReport, DynoError> {
        if let Some(stamp) = &doc.stamp
            && crate::provenance::is_newer(&stamp.reflow2_version, env!("CARGO_PKG_VERSION"))
            && !options.accept_newer
        {
            return Err(DynoError::Validation {
                node_type: "GraphExport".into(),
                property: "stamp.reflow2_version".into(),
                message: format!(
                    "REFUSED: this document was written by reflow2 {}, and you are running {}, \
                     which is BEHIND it. Nothing was written. {} Update reflow2 and import \
                     again — or pass accept_newer: true if you mean to read it with this \
                     binary, and read the report's `materialized` field for exactly what it \
                     wrote that the document did not state.",
                    stamp.reflow2_version,
                    env!("CARGO_PKG_VERSION"),
                    crate::provenance::BEHIND_HAZARD
                ),
            });
        }
        // WHOSE DESIGN IS THIS? Answered before a single write, because the id
        // namespaces every stored key and the import writes under whatever name
        // the graph currently carries (BL-169).
        //
        // Restoring a design into an EMPTY store takes the document's name.
        // Anything else renames it: `graph_id` is inside the export's content
        // hash, so a round trip that renamed would not come back byte-identical,
        // and on 2026-08-02 exactly that shipped — an export replayed through a
        // temp graph came back as `05a6fbe860bf7a23` where the design has been
        // `reflow2` since its first commit, and it was committed and pushed with
        // both CI jobs and the coherence gate green.
        //
        // This rule already existed and lived in the WRONG PLACE: `main.rs`
        // applied it on the CLI `--import` path only, so the command and the
        // tool disagreed about what a restore means. It belongs in the operation
        // — one predicate, every caller, including any future one.
        //
        // `adopt_on_import` is the shared predicate and it is conservative in
        // the direction that matters: a store already holding a design keeps its
        // own name, because layering an export onto a live design is an upsert,
        // not a restore, and taking the incoming name there would rename a
        // design to whatever was last imported into it.
        let mut adopted_identity = None;
        if let Some(path) = self.store_path.clone() {
            let holds = self.holds_a_design();
            if let Some(adopted) = crate::identity::adopt_on_import(&path, &doc.graph_id, holds)? {
                self.graph_id = adopted.graph_id.clone();
                adopted_identity = Some(adopted.graph_id);
            }
        }

        // Endpoint types come from the document's own nodes, falling back to
        // what is already in the graph — so an export can be layered onto a
        // design it references without carrying it.
        let mut types: BTreeMap<&str, &str> = BTreeMap::new();
        for n in &doc.nodes {
            types.insert(n.node_id.as_str(), n.node_type.as_str());
        }
        let existing = self.node_type_index()?;

        self.begin_batch();
        let result = (|| -> Result<ImportReport, DynoError> {
            // EVERY FAULT IN ONE RESPONSE, NOT ONE PER ROUND TRIP (BL-118).
            // This used to be `create_node(..)?`, so validation stopped at the
            // first violation: an external adopt pass over a hand-authored
            // 9,000-line document took FOUR consecutive imports to learn four
            // faults — a missing stamp field, then three different enum
            // violations — each attempt a full edit-retry cycle for one error.
            //
            // THE ATOMICITY IS UNTOUCHED AND MUST STAY THAT WAY: the same
            // session named "nothing half-loaded across four failures" as one
            // of the things that worked notably well. This is exactly
            // `dec:bulk-is-all-or-nothing-with-per-item-findings` — every item
            // attempted so you learn every failure at once, and if any failed
            // nothing is written — which BL-153 settled for the bulk tools and
            // whose own row noted that `import_graph` files the opposite
            // defect. The shape is borrowed rather than reinvented.
            //
            // It still returns Err rather than an Ok-with-failures report, and
            // that is deliberate: a caller that treated a rejected import as
            // success would be the silent-failure this crate's first principle
            // forbids. The error carries the whole list.
            let mut faults: Vec<String> = Vec::new();
            let mut materialized: BTreeMap<String, usize> = BTreeMap::new();
            let mut pending_refs: Vec<(usize, &str, &str, String, String)> = Vec::new();
            // The checksum `export_graph` left off an Artifact because its
            // accepting change already carries it comes back here — derived
            // from the document's own edges, so it is not "materialised": the
            // document stated it, once, where it belongs.
            let accepted = current_acceptances(&doc.edges);
            // WHAT AN EXISTING NODE LOSES OR HAS CHANGED is measured against
            // the node as it stood before this document touched it, by the
            // document's own type (`existing` maps an id to the type holding
            // it). Read before the write, so a node named twice in one
            // document is compared with the store, not with itself.
            let mut properties_removed: BTreeMap<String, usize> = BTreeMap::new();
            let mut properties_changed: BTreeMap<String, usize> = BTreeMap::new();
            let mut removed_from: Vec<String> = Vec::new();
            let mut nodes_losing = 0usize;
            for (index, n) in doc.nodes.iter().enumerate() {
                let mut stated = n.properties.clone();
                fill_accepted_checksum(&n.node_type, &n.node_id, &mut stated, &accepted);
                let before: Option<std::collections::HashMap<String, Value>> =
                    if existing.get(&n.node_id).map(String::as_str) == Some(n.node_type.as_str()) {
                        self.get_node(&n.node_type, &n.node_id)?
                            .map(|stored| stored.properties)
                    } else {
                        None
                    };
                let mut props: std::collections::HashMap<String, Value> =
                    stated.clone().into_iter().collect();
                if options.merge
                    && let Some(before) = &before
                {
                    for (k, v) in before {
                        props.entry(k.clone()).or_insert_with(|| v.clone());
                    }
                }
                for (prop, target) in self.declared_node_refs(&n.node_type, &props) {
                    pending_refs.push((
                        index,
                        n.node_id.as_str(),
                        n.node_type.as_str(),
                        prop,
                        target,
                    ));
                }
                // The stamps the document carries go in as it states them.
                props.extend(n.stamps.iter().map(|(k, v)| (k.clone(), v.clone())));
                match self.create_node_refs_checked_later(&n.node_type, &n.node_id, props) {
                    Ok(stored) => {
                        for key in stored.properties.keys() {
                            // A default filled in is a key neither the
                            // document nor the node held. One the node
                            // already held is its own value kept (merge) or
                            // replaced — counted below, never as a fill.
                            if !stated.contains_key(key)
                                && !before.as_ref().is_some_and(|b| b.contains_key(key))
                            {
                                *materialized
                                    .entry(format!("{}.{key}", n.node_type))
                                    .or_insert(0) += 1;
                            }
                        }
                        if let Some(before) = &before {
                            let mut lost: Vec<&str> = Vec::new();
                            for (key, was) in before {
                                match stored.properties.get(key) {
                                    None => {
                                        lost.push(key);
                                        *properties_removed
                                            .entry(format!("{}.{key}", n.node_type))
                                            .or_insert(0) += 1;
                                    }
                                    Some(now) if now != was => {
                                        *properties_changed
                                            .entry(format!("{}.{key}", n.node_type))
                                            .or_insert(0) += 1;
                                    }
                                    Some(_) => {}
                                }
                            }
                            if !lost.is_empty() {
                                nodes_losing += 1;
                                if removed_from.len() < REMOVED_FROM_SHOWN {
                                    lost.sort_unstable();
                                    removed_from.push(format!(
                                        "{}: {}",
                                        n.node_id,
                                        lost.join(", ")
                                    ));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // A type this reflow2 RETIRED is named as retired, with the
                        // way through, rather than left as an unknown name.
                        let hint = crate::narrowing::retired_node_type_hint(&n.node_type)
                            .map(|h| format!(" — {h}"))
                            .unwrap_or_default();
                        faults.push(format!("nodes[{index}] {}: {e}{hint}", n.node_id))
                    }
                }
            }

            // A NODE REFERENCE RESOLVES AGAINST THE WHOLE DOCUMENT, NOT AGAINST
            // HOW FAR THE WALK HAS GOT.
            //
            // `create_node` refuses a property that NAMES a node which does not
            // exist, and on an interactive write that is exactly right. On an
            // import it is not, because "exists" there would mean "appears
            // earlier in this file" — and an export is sorted by node type, so
            // the order is the alphabet rather than the dependencies.
            //
            // MEASURED 2026-08-29 on this design's own export: 23 faults, of
            // which nine were real dangling references and four were nodes
            // present in the very same document, refused only for sorting after
            // their referrer. Nothing was written, so the export could not be
            // restored at all.
            //
            // `types` is every node id the document carries and `existing` is
            // every id already in the graph, so this asks the question against
            // the state the import would actually leave behind. Resolving
            // against `types` also stops one refused node cascading a second,
            // derived fault onto whoever referenced it — the earlier fault is
            // the one worth reading.
            //
            // ⚠️ IT STILL CANNOT SEE A RENAME. Four of the nine real dangles
            // were valid when written and broke later when the target was
            // renamed. No check on the write or on the import reaches those;
            // they need detection over the graph as it stands, which is a
            // different cause with a different fix.
            let mut dangling_node_refs: Vec<String> = Vec::new();
            for (_index, node_id, _node_type, prop, target) in pending_refs {
                if types.contains_key(target.as_str()) || existing.contains_key(&target) {
                    continue;
                }
                dangling_node_refs.push(format!(
                    "{node_id}.{prop} -> {target} (names no node in the document or the graph)"
                ));
            }
            let mut edges_written = 0;
            let mut skipped_edges = Vec::new();
            let mut migrated_edges: Vec<String> = Vec::new();
            for (index, e) in doc.edges.iter().enumerate() {
                let from = types
                    .get(e.from_id.as_str())
                    .copied()
                    .or_else(|| existing.get(&e.from_id).map(String::as_str));
                let to = types
                    .get(e.to_id.as_str())
                    .copied()
                    .or_else(|| existing.get(&e.to_id).map(String::as_str));
                match (from, to) {
                    (Some(ft), Some(tt)) => {
                        let mut props: std::collections::HashMap<String, Value> =
                            e.properties.clone().into_iter().collect();
                        // A legacy export carries single `role` (+ `acted_at`);
                        // the store holds the set shape. Normalised on the way
                        // in so one design never holds both.
                        if e.edge_type == edge::AUTHORED_BY {
                            crate::graph::normalize_authored_by_props(&mut props);
                        }
                        // An edge a narrowing gave a single right answer
                        // arrives as that answer and is REPORTED, so a design
                        // written before the narrowing still restores — today
                        // a legacy `Artifact REALIZES Verification` as the
                        // IMPLEMENTS it always meant. One table
                        // (`narrowing::EDGE_REWRITES`), read here, on every
                        // open, and by the recheck. Its old properties have
                        // no home on the new edge.
                        let mut edge_type = e.edge_type.as_str();
                        if let Some(r) = crate::narrowing::edge_rewrite_for(edge_type, ft, tt) {
                            edge_type = r.becomes;
                            props.clear();
                            migrated_edges.push(format!(
                                "{} {} -> {} became {} ({})",
                                e.edge_type, e.from_id, e.to_id, r.becomes, r.why
                            ));
                        }
                        // An edge property naming a node resolves against
                        // the whole document too, and a dangling one is
                        // reported beside the nodes', never refused.
                        for (prop, target) in self.declared_edge_node_refs(edge_type, &props) {
                            if !types.contains_key(target.as_str())
                                && !existing.contains_key(&target)
                            {
                                dangling_node_refs.push(format!(
                                    "{} {} -> {}.{prop} -> {target} (names no node in the \
                                     document or the graph)",
                                    edge_type, e.from_id, e.to_id
                                ));
                            }
                        }
                        match self.create_edge_refs_checked_later(
                            edge_type, ft, &e.from_id, tt, &e.to_id, props,
                        ) {
                            Ok(stored) => {
                                edges_written += 1;
                                for key in stored.properties.keys() {
                                    if !e.properties.contains_key(key) {
                                        *materialized
                                            .entry(format!("{}.{key}", e.edge_type))
                                            .or_insert(0) += 1;
                                    }
                                }
                            }
                            Err(err) => {
                                // The same replacement `detect_defects`
                                // names for this edge on a store that holds it.
                                let hint =
                                    crate::narrowing::named_replacement(&e.edge_type, ft, tt)
                                        .map(|h| format!(" — {h}"))
                                        .unwrap_or_default();
                                faults.push(format!(
                                    "edges[{index}] {} {} -> {}: {err}{hint}",
                                    e.edge_type, e.from_id, e.to_id
                                ))
                            }
                        }
                    }
                    _ => skipped_edges.push(format!(
                        "{} {} -> {} (endpoint not in the document or the graph)",
                        e.edge_type, e.from_id, e.to_id
                    )),
                }
            }
            if !faults.is_empty() {
                let n = faults.len();
                return Err(DynoError::Validation {
                    node_type: "GraphExport".into(),
                    property: "nodes/edges".into(),
                    message: format!(
                        "{n} item(s) in this document are invalid, and NOTHING was written — \
                         the import is all-or-nothing. Every one is listed so you can fix them \
                         in one edit rather than learning them one import at a time:\n  - {}",
                        faults.join("\n  - ")
                    ),
                });
            }
            // Inside the batch, so the repair lands with the document or not at all.
            let twin_repairs = self.repair_stored_twins()?;
            Ok(ImportReport {
                nodes_written: doc.nodes.len(),
                edges_written,
                skipped_edges,
                integrity_note: match doc.verify_content_hash() {
                    Some(false) => Some(
                        "the document's content_hash does not match its content — it was \
                         edited outside reflow2 or corrupted since it was exported"
                            .to_string(),
                    ),
                    _ => None,
                },
                provenance_note: doc.is_unstamped().then(|| {
                    "the document had no stamp — imported as unstamped (a hand-authored or \
                     third-party document, not one reflow2 exported); provenance and the \
                     upgrade-direction check cannot be run on it"
                        .to_string()
                }),
                adopted_identity: adopted_identity.clone(),
                migrated_edges,
                dangling_node_refs,
                materialized,
                twin_repairs,
                removed_note: removed_note(&properties_removed, nodes_losing),
                properties_removed,
                properties_changed,
                removed_from: {
                    if nodes_losing > removed_from.len() {
                        removed_from.push(format!(
                            "… and {} more node(s); `properties_removed` counts them all",
                            nodes_losing - removed_from.len()
                        ));
                    }
                    removed_from
                },
            })
        })();

        match result {
            Ok(report) => {
                self.commit_batch()?;
                Ok(report)
            }
            Err(e) => {
                // Nothing half-written: the batch is dropped, not partially kept.
                self.discard_batch();
                Err(e)
            }
        }
    }
}

/// The sentence an import says when it removed properties from nodes the
/// design already held. `None` when it removed none.
fn removed_note(removed: &BTreeMap<String, usize>, nodes: usize) -> Option<String> {
    if removed.is_empty() {
        return None;
    }
    let total: usize = removed.values().sum();
    Some(format!(
        "This import REMOVED {total} stored value(s) from {nodes} node(s) the design already \
         held. A document states each node whole, so what it leaves out is removed: that is how \
         an export→import round trip reproduces a design and how a document drops a property. \
         `properties_removed` counts them by Type.property and `removed_from` names a few. If \
         the document was meant to EDIT those nodes, restore them from your last export, then \
         import with `merge: true` (or write the edits with create_nodes), which keeps what a \
         call leaves out."
    ))
}

/// Put back the `checksum` (and its `checksum_basis`) an export left off an
/// Artifact because its current acceptance carries it — the IMPORT'S rule,
/// written once so every reader of a document applies the same one: the
/// import, and `compare_designs`, which must read a record written before the
/// checksum moved (on the node) and one written after (on the change) as the
/// same design. A value the node states is never overridden. `accepted` is
/// [`current_acceptances`] of the same document. Returns whether it filled.
pub fn fill_accepted_checksum(
    node_type: &str,
    node_id: &str,
    props: &mut Props,
    accepted: &BTreeMap<String, (String, Option<String>)>,
) -> bool {
    if node_type != node::ARTIFACT || props.contains_key("checksum") {
        return false;
    }
    let Some((checksum, basis)) = accepted.get(node_id) else {
        return false;
    };
    props.insert("checksum".into(), Value::String(checksum.clone()));
    if let Some(basis) = basis
        && !props.contains_key("checksum_basis")
    {
        props.insert("checksum_basis".into(), Value::String(basis.clone()));
    }
    true
}

/// For each artifact, the checksum its CURRENT acceptance carries, and that
/// acceptance's basis: among the `CHANGED` edges into it with `checksum_after`,
/// the highest `accepted_seq`, ties to the smaller change id. The one rule
/// [`DesignGraph::current_acceptance`] applies to a store, applied to a
/// document, so an export's elision and an import's derivation cannot disagree.
pub fn current_acceptances(edges: &[ExportedEdge]) -> BTreeMap<String, (String, Option<String>)> {
    let mut best: BTreeMap<String, (i64, &str, String, Option<String>)> = BTreeMap::new();
    for e in edges.iter().filter(|e| e.edge_type == edge::CHANGED) {
        let Some(checksum) = e.properties.get("checksum_after").and_then(Value::as_str) else {
            continue;
        };
        let seq = e
            .properties
            .get("accepted_seq")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let basis = e
            .properties
            .get("checksum_basis")
            .and_then(Value::as_str)
            .map(str::to_string);
        let better = match best.get(&e.to_id) {
            None => true,
            Some((s, from, _, _)) => seq > *s || (seq == *s && e.from_id.as_str() < *from),
        };
        if better {
            best.insert(
                e.to_id.clone(),
                (seq, e.from_id.as_str(), checksum.to_string(), basis),
            );
        }
    }
    best.into_iter()
        .map(|(id, (_, _, checksum, basis))| (id, (checksum, basis)))
        .collect()
}

/// What a published-surface export contains, and what it deliberately does not.
///
/// A node this surface kept whose container it withheld.
///
/// Reported so a recipient can tell a genuinely top-level part from one this
/// filtering orphaned — the two are indistinguishable in the document itself.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SeveredContainment {
    /// A node this surface KEPT.
    pub node_id: String,
    /// A container it has in the full design, which this surface WITHHELD — so
    /// in the document the recipient holds, the node has no parent.
    pub withheld_parent: String,
}

/// The report is the load-bearing half. A surface document is a *partial* design
/// by construction, so shipping one without saying what was held back would be
/// the silent drop rule 6 forbids — and worse than usual here, because the
/// recipient cannot tell a small design from a heavily-filtered one.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SurfaceExport {
    /// The document: published Interfaces, what specifies them, and the parts on
    /// each side. Importable like any export.
    pub document: GraphExport,
    /// The CHECKS this surface carries, sorted — every Verification whose
    /// target the owner published.
    ///
    /// ⚠️ NAMED RATHER THAN MERELY COUNTED BECAUSE THEY DISCLOSE. A check
    /// travels AS STORED, and `Verification.findings` is free text about what a
    /// run found. Measured on this design 2026-09-22, the three checks that
    /// would travel carried internal build paths (`target/debug/...`), internal
    /// ChangeEvent ids and internal node counts in exactly that field. That is
    /// a real disclosure, it is bounded (only checks on published targets
    /// travel), and an owner should be able to READ WHICH ONES LEFT rather than
    /// discover it in somebody else's graph. Reported, never repaired —
    /// trimming a node would make it import as a lie.
    pub evidence: Vec<String>,
    /// The published boundaries this surface is *about*, sorted.
    pub published: Vec<String>,
    /// Design nodes kept back as internal.
    pub withheld_nodes: usize,
    /// Edges dropped because at least one endpoint was withheld.
    pub withheld_edges: usize,
    /// Nodes this surface KEPT whose container it WITHHELD, each naming the
    /// container that went — so a recipient can tell a genuinely top-level part
    /// from one this filtering orphaned.
    ///
    /// **Reported, never repaired.** Carrying the ancestry would disclose the
    /// internals a surface exists to withhold; re-parenting the orphan to the
    /// Project would assert a `CONTAINS` nobody drew, which is the fabrication
    /// `req:a-repair-suggestion-never-proposes-fabrication` forbids; dropping the
    /// child would delete the provider of a published contract. So the document
    /// is unchanged and the severance is stated.
    ///
    /// Found 2026-08-12 on the first real cross-design trial: four `subsystem`
    /// components travelled without the `system` that contains them, and
    /// `hierarchy_issues` in the receiving graph went 0 → 4 `orphan_level` —
    /// findings that were FALSE about the source design and CORRECT about the
    /// document it was handed. The detector was innocent; the document lied by
    /// omission.
    pub severed_containment: Vec<SeveredContainment>,
    /// Said in words, because a count alone does not tell the recipient what
    /// kind of document they are holding.
    pub note: String,
}

impl DesignGraph {
    /// Export **only the published surface** — the contracts others are entitled
    /// to rely on, and nothing internal.
    ///
    /// The first piece of `req:design-composes` that every architecture answer
    /// needs: whatever composes, it composes through a published boundary rather
    /// than by reaching into another system's internals. Also the openness half
    /// of `req:key-interfaces` ("a design that publishes a surface should be able
    /// to export exactly that surface").
    ///
    /// **What goes in**: every Interface designated `published`, the artifacts
    /// that `SPECIFIES` or `REALIZES` it (the machine-readable contract — an
    /// OpenAPI or protobuf file is the real ICD; both edges are honoured because
    /// `link_artifact` writes REALIZES while the schema's intent for a contract
    /// artifact is SPECIFIES), the Components that provide or consume it, and the
    /// Project for provenance. Nodes are exported **as stored** — nothing is
    /// trimmed or rewritten, because a fabricated node would import as a lie.
    /// What leaks internals is the graph *beneath* a component, and that is what
    /// is excluded.
    ///
    /// Plus the **evidence for what was published**: any Verification whose
    /// `VERIFIES` edge lands on a published Interface or a published
    /// Requirement. Derived from the target's designation, never from a
    /// designation of its own — see the carve-out below.
    ///
    /// **What stays home**: capabilities, decisions, history, provenance, every
    /// internal component and contract, every requirement not designated
    /// `published`, and every check whose target was not published. All
    /// counted, never silently dropped.
    ///
    /// **It does not join the file's hash chain.** `prev_content_hash` is left
    /// unset: this is a derived view, not a record of the design, and letting it
    /// advance the chain would make `compare_designs` treat a published surface
    /// as an ancestor of the full design (`dec:export-hash-chain`).
    pub fn export_surface(&self) -> Result<SurfaceExport, DynoError> {
        let published = self.published_interfaces()?;
        let mut keep: BTreeSet<String> = published.clone();

        // The parts on each side of a published contract: PROVIDES/CONSUMES run
        // from the Component to the Interface, so the sides are its incoming.
        for ifc in &published {
            for e in self.incoming(ifc, None)? {
                if e.edge_type == edge::PROVIDES || e.edge_type == edge::CONSUMES {
                    keep.insert(e.from_id.clone());
                }
            }
            // The contract artifacts. SPECIFIES is the modelled intent — "an
            // OpenAPI / protobuf / IDL file SPECIFIES an Interface (its
            // authoritative contract)" — but `link_artifact` writes REALIZES, so
            // in practice the ICD arrives on either edge. Both belong on a
            // published surface: one is the contract as written, the other as
            // built, and a recipient needs whichever exists.
            for e in self.incoming(ifc, None)? {
                if e.edge_type == edge::SPECIFIES || e.edge_type == edge::REALIZES {
                    keep.insert(e.from_id.clone());
                }
            }
        }
        // The promises this design publishes (`req:publishable-promise`). A
        // structural surface says what the boundaries ARE and cannot say what any
        // of them undertakes to DO — "fails loud rather than falling back",
        // "ordering is preserved" — because a behavioural commitment lives in a
        // Requirement and every Requirement was withheld. Found by a real trial,
        // where the promise ended up asserted in a comment in the CONSUMER's
        // build file, on the wrong side of the seam.
        //
        // Opt-in, exactly like the boundaries: only requirements the owner
        // deliberately designated `published` travel. Everything else is still
        // withheld and still counted.
        let promises = self.published_promises()?;
        keep.extend(promises.iter().cloned());

        // THE EVIDENCE FOR WHAT WAS PUBLISHED. A promise without its check is a
        // claim nothing can re-run: delivery is computed as SATISFIED and
        // REALIZED and A PASSING CHECK, so a consumer holding the promise alone
        // cannot tell whether it currently HOLDS. Withholding the check
        // reproduces, at the seam, the defect this design removed from its own
        // insides on 2026-09-22 — 241 capabilities reading "verified" against 21
        // with a check anything could re-run.
        //
        // 🛑 DERIVED, NOT A THIRD OPT-IN, and that choice is measured rather
        // than aesthetic. Interfaces and Requirements each travel on an explicit
        // `designation`; a third gate on Verification would reproduce the
        // failure measured the same day, when this design held 262 requirements
        // and had marked ZERO `published` because the vocabulary existed and
        // nothing asked. THE OWNER ALREADY OPTED IN WHEN THEY PUBLISHED THE
        // TARGET. So a check travels exactly when it verifies something the
        // owner deliberately published — the same derived rule that already
        // carries the parts either side of a contract and the artifacts that
        // specify it.
        //
        // SCOPED TO THE COMMITMENTS, not to everything on the surface. A
        // Component travels because a published contract needs its sides, not
        // because anyone undertook anything about it, so a check on that
        // component is internal evidence about an internal part and stays home.
        //
        // ⚠️ AND IT DISCLOSES WHAT THE CHECK SAYS. `Verification.findings` is
        // free text about what a run found, and a surface exports nodes AS
        // STORED — trimming one would make it import as a lie. A findings field
        // holding internal paths or ids therefore leaves with the check. Bounded
        // (only checks on published targets travel) and real; an owner should
        // know it rather than discover it.
        let commitments: BTreeSet<String> =
            published.iter().chain(promises.iter()).cloned().collect();
        let mut evidence: BTreeSet<String> = BTreeSet::new();
        for v in self.scan_nodes(node::VERIFICATION)? {
            let travels = self
                .outgoing(&v.node_id, Some(edge::VERIFIES))?
                .iter()
                .any(|e| commitments.contains(&e.to_id));
            if travels {
                evidence.insert(v.node_id.clone());
                keep.insert(v.node_id);
            }
        }

        // The Project, so a recipient knows whose surface this is.
        for p in self.scan_nodes(node::PROJECT)? {
            keep.insert(p.node_id);
        }

        let full = self.export_graph()?;
        let total_nodes = full.nodes.len();
        let total_edges = full.edges.len();
        let nodes: Vec<ExportedNode> = full
            .nodes
            .into_iter()
            .filter(|n| keep.contains(&n.node_id))
            .collect();
        let edges: Vec<ExportedEdge> = full
            .edges
            .into_iter()
            .filter(|e| keep.contains(&e.from_id) && keep.contains(&e.to_id))
            .collect();

        let withheld_nodes = total_nodes - nodes.len();
        let withheld_edges = total_edges - edges.len();

        // What the withholding did to what we KEPT. A node whose every CONTAINS
        // parent was filtered out arrives with no place in the hierarchy, and
        // the recipient cannot tell that from a genuinely top-level part.
        let mut severed_containment: Vec<SeveredContainment> = Vec::new();
        for n in &nodes {
            let mut lost: Vec<String> = Vec::new();
            let mut survived = false;
            for e in self.incoming(&n.node_id, Some(edge::CONTAINS))? {
                if keep.contains(&e.from_id) {
                    survived = true;
                    break;
                }
                lost.push(e.from_id.clone());
            }
            if !survived {
                lost.sort();
                for parent in lost {
                    severed_containment.push(SeveredContainment {
                        node_id: n.node_id.clone(),
                        withheld_parent: parent,
                    });
                }
            }
        }
        severed_containment.sort_by(|a, b| {
            (&a.node_id, &a.withheld_parent).cmp(&(&b.node_id, &b.withheld_parent))
        });
        let severance_note = if severed_containment.is_empty() {
            String::new()
        } else {
            format!(
                " ⚠️ {} kept node(s) LOST THEIR CONTAINER to this filtering and arrive with no                  parent: {}. They are not top-level in the source design — read them as                  structurally partial, and do not diagnose their hierarchy from this document.",
                severed_containment.len(),
                severed_containment
                    .iter()
                    .map(|s| format!("{} (was in {})", s.node_id, s.withheld_parent))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        // Said plainly whichever way it comes out, because "no promises" and
        // "promises you cannot see" must never look alike (`req:publishable-promise`).
        let promise_note = if promises.is_empty() {
            " NO BEHAVIOURAL PROMISES are published: this surface says what the boundaries ARE \
             and nothing about what they undertake to DO. Read that as \"none stated\", never as \
             \"none exist\" — designate a Requirement `published` to commit to one."
                .to_string()
        } else {
            format!(
                " {} behavioural promise(s) published alongside the structure — commitments a \
                 consumer is entitled to rely on, not just contracts it may call.",
                promises.len()
            )
        };
        let note = if published.is_empty() {
            // The dangerous case: an empty surface looks exactly like a design
            // with nothing to share, and someone could publish it believing they
            // had shared something. Say so unmistakably rather than refusing —
            // "prove I publish nothing" is a legitimate question.
            format!(
                "EMPTY SURFACE: no Interface is designated `published`, so this document exposes \
                 nothing. {withheld_nodes} node(s) and {withheld_edges} edge(s) were withheld as \
                 internal. If you meant to publish a boundary, designate it first \
                 (set_interface_designation) — an empty surface is indistinguishable from a design \
                 with nothing in it.{promise_note}"
            )
        } else {
            format!(
                "Published surface: {} boundary(ies), {} node(s) exposed. WITHHELD as internal: \
                 {withheld_nodes} node(s), {withheld_edges} edge(s) — undesignated requirements, \
                 capabilities, decisions, verifications and history stay home. This is a partial \
                 design by design; it is not a backup and cannot be imported as one.                 {promise_note}{severance_note}",
                published.len(),
                nodes.len()
            )
        };

        let mut document = GraphExport {
            taken_at: None,
            stamp: Some(GraphStamp::current(self.schema())),
            content_hash: None,
            // Deliberately not chained — see the doc comment.
            prev_content_hash: None,
            graph_id: self.graph_id().to_string(),
            nodes,
            edges,
        };
        document.content_hash = Some(document.compute_content_hash());

        Ok(SurfaceExport {
            document,
            evidence: evidence.into_iter().collect(),
            published: published.into_iter().collect(),
            withheld_nodes,
            withheld_edges,
            severed_containment,
            note,
        })
    }
}

/// What mirroring another design's published surface did — and, critically, what
/// it refused to do.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MirrorReport {
    /// The `graph_id` of the design mirrored.
    pub mirror_of: String,
    /// The surface document's content hash: the coordinate this mirror is pinned
    /// to, and what makes staleness computable later.
    pub mirror_content_hash: Option<String>,
    /// Nodes brought in as foreign mirrors.
    pub mirrored_nodes: usize,
    /// Edges brought in (only those wholly inside the mirrored set).
    pub mirrored_edges: usize,
    /// Ids in the surface that ALREADY EXIST here and were left untouched.
    ///
    /// The hazard this exists for: `import_graph` is an upsert, so mirroring a
    /// foreign surface whose ids collide with local ones would silently overwrite
    /// your own design with somebody else's nodes. A collision is reported and
    /// skipped, never resolved by guessing — and a non-empty list means the two
    /// designs are using the same id for different things, which is a naming
    /// conversation between two owners, not a merge.
    pub collisions: Vec<String>,
    /// True when this replaced a mirror of the SAME design rather than adding a
    /// new one — the two are different operations and only one of them may
    /// remove nodes.
    pub refreshed: bool,
    /// Ids the far side NO LONGER PUBLISHES. Since PR A2 a refresh never
    /// deletes one: each is kept and marked superseded (`superseded`), with a
    /// snapshot and a change event. Empty on a first mirror.
    pub withdrawn: Vec<String>,
    /// THE WITHDRAWN NODES THIS REFRESH KEPT AND MARKED SUPERSEDED
    /// (`superseded_by_surface` names the surface that withdrew them), so
    /// what rested on them reads reason-lost instead of silently losing the
    /// premise (req:a-superseded-mirrored-judgment-is-kept-and-what-rested-
    /// on-it-reads-reason-lost, part 1). A node already marked by an earlier
    /// refresh is not counted again.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub superseded: Vec<String>,
    /// EVERY LOCAL EDGE touching a withdrawn node, each kept: `TYPE from ->
    /// to` (part 2). The split-capstone refresh dropped every one of them
    /// silently — PREMISE_OF, CONSUMES, CALIBRATED_AGAINST, DOCUMENTS.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub kept_edges: Vec<String>,
    /// The snapshots holding each superseded node's state before it was
    /// marked.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub snapshots: Vec<String>,
    /// The ChangeEvent recording the withdrawal, when anything was superseded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_event: Option<String>,
    /// Said in words, because the counts alone do not tell you whether to worry.
    pub note: String,
}

impl DesignGraph {
    /// Mirror another design's published surface into this graph as **foreign**
    /// nodes carrying the coordinate that says whose they are.
    ///
    /// The first rung of `dec:nested-graphs` option (c): designs are separate
    /// graphs at ownership boundaries and link by mirroring, because an edge
    /// cannot cross a store — the schema validates both endpoints. After
    /// mirroring, your own components `provides`/`consumes` the mirrored
    /// Interface with **ordinary local edges**, so the golden thread, propagate,
    /// and every detector work unchanged. Foreignness is a property of the node,
    /// never of the link.
    ///
    /// The mirrored Project node carries the coordinate — `mirror_of`,
    /// `mirror_content_hash`, `mirrored_at` — which is what makes a later
    /// surface with a different hash detectable as staleness rather than assumed
    /// current (`req:design-composes`, obligation 3).
    ///
    /// **Collisions are refused, not merged.** An id present here already is left
    /// exactly as it is and reported: mirroring must never overwrite your design
    /// with someone else's node, and two designs using one id for different
    /// things is a conversation between owners rather than something a tool may
    /// silently resolve (`dec:ask-not-repair`).
    pub fn mirror_surface(
        &mut self,
        doc: &GraphExport,
        at: Option<&str>,
    ) -> Result<MirrorReport, DynoError> {
        self.mirror_surface_with(doc, at, false)
    }

    /// [`mirror_surface`](Self::mirror_surface), saying whether a refresh may
    /// go on although this design still rests on something the new surface
    /// WITHDRAWS (`accept_withdrawal`). Without it such a refresh is refused
    /// and names every local edge involved; with it the withdrawn nodes are
    /// kept, marked superseded, and every edge stays, so what rested on them
    /// reads reason-lost until the owner re-grounds it or lets it go
    /// (req:a-superseded-mirrored-judgment-is-kept-and-what-rested-on-it-
    /// reads-reason-lost).
    pub fn mirror_surface_with(
        &mut self,
        doc: &GraphExport,
        at: Option<&str>,
        accept_withdrawal: bool,
    ) -> Result<MirrorReport, DynoError> {
        let source = doc.graph_id.clone();
        // `graph_id` became optional for import (BL-138) because import never
        // reads it. Mirroring does: it records `mirror_of` and guards against
        // mirroring a design into itself, and neither question has an answer
        // here. Refused by name rather than mirrored under an empty provenance,
        // which would record a claim nobody made.
        if doc.is_unidentified() {
            return Err(DynoError::Validation {
                node_type: node::PROJECT.into(),
                property: "mirror_of".into(),
                message: "this surface document carries no `graph_id`, so there is no way to \
                          record WHERE it came from or to check it is not this design. \
                          import_graph accepts an unidentified document because it never needs \
                          the answer; mirroring does. Add the source design's `graph_id`, or \
                          use import_graph if you meant to load it as your own."
                    .into(),
            });
        }
        if source == self.graph_id() {
            return Err(DynoError::Validation {
                node_type: node::PROJECT.into(),
                property: "mirror_of".into(),
                message: format!(
                    "this surface came from '{source}', which is this graph — mirroring a design \
                     into itself would overwrite your own nodes with a filtered copy of them. \
                     Import a surface from ANOTHER design, or use import_graph if you meant to \
                     restore a backup."
                ),
            });
        }

        // IS THIS A REFRESH? A Project already carrying this `mirror_of` means we
        // hold a copy of this design, so re-mirroring is "replace what I hold of
        // theirs, at a new pin" — NOT "add a stranger and refuse clashes". Before
        // this, the second mirror of one design collided with its own first
        // (12 of 13 ids, measured 2026-08-12) and left the pin reporting the OLD
        // hash, so the staleness register read FRESH after a failed refresh.
        let held: Vec<MirrorRef> = self
            .mirrors()?
            .into_iter()
            .filter(|m| m.mirror_of == source)
            .collect();
        let refreshed = !held.is_empty();

        // What we currently hold of theirs. Recorded on the mirrored Project at
        // mirror time, because it cannot be recovered afterwards: `provenance:
        // imported` is also set by corpus ingest, and the project's CONTAINS
        // edges do not span the mirrored set (measured: 5 of 13).
        let mut previously: BTreeSet<String> = BTreeSet::new();
        // The edges the last surface brought, `TYPE|from|to` (recorded since
        // PR A2; an older mirror recorded none, and then none is removed).
        let mut previous_edges: BTreeSet<String> = BTreeSet::new();
        for m in &held {
            if let Some(p) = self.get_node(node::PROJECT, &m.project_id)? {
                if let Some(list) = p.properties.get("mirror_nodes").and_then(Value::as_str) {
                    previously.extend(
                        list.split(',')
                            .filter(|s| !s.is_empty())
                            .map(str::to_string),
                    );
                }
                if let Some(list) = p.properties.get("mirror_edges").and_then(Value::as_str) {
                    previous_edges.extend(
                        list.split(',')
                            .filter(|s| !s.is_empty())
                            .map(str::to_string),
                    );
                }
            }
            previously.insert(m.project_id.clone());
        }

        // WOULD THIS WITHDRAW SOMETHING WE REST ON? A partner dropping a
        // contract you depend on is a conversation, not a cleanup
        // (dec:ask-not-repair), so this refuses BEFORE writing and names both
        // the withdrawn node and every edge of ours touching it — in EITHER
        // direction. Until PR A2 only edges INTO a withdrawn node counted
        // (DEPENDS_ON, ASKS_ABOUT), so the seven PREMISE_OF edges the
        // split-capstone decisions rested on, which leave the judgment, were
        // missed (part 4). An edge to bookkeeping (a snapshot, a change, a
        // signature) is no dependency.
        let arriving: BTreeSet<String> = doc.nodes.iter().map(|n| n.node_id.clone()).collect();
        let withdrawn: Vec<String> = previously.difference(&arriving).cloned().collect();
        let mut blocked: Vec<String> = Vec::new();
        for gone in &withdrawn {
            for e in self.incoming(gone, None)? {
                if !previously.contains(&e.from_id) {
                    blocked.push(format!("{} {} -> {}", e.edge_type, e.from_id, gone));
                }
            }
            for e in self.outgoing(gone, None)? {
                if previously.contains(&e.to_id) || e.edge_type == edge::AUTHORED_BY {
                    continue;
                }
                let to_type = self.type_holding(&e.to_id)?;
                if to_type
                    .as_deref()
                    .is_some_and(|t| crate::temporal::BOOKKEEPING_TYPES.contains(&t))
                {
                    continue;
                }
                blocked.push(format!("{} {} -> {}", e.edge_type, gone, e.to_id));
            }
        }
        blocked.sort();
        blocked.dedup();
        if !blocked.is_empty() && !accept_withdrawal {
            return Err(DynoError::Validation {
                node_type: node::PROJECT.into(),
                property: "mirror_of".into(),
                message: format!(
                    "REFUSED: this surface from '{source}' no longer publishes {} node(s) \
                     that YOUR design still rests on or points at, and nothing was changed. \
                     Withdrawn: {}. Your edges: {}.\n\nA partner withdrawing something you \
                     reason from is a conversation between the two owners, not a cleanup this \
                     tool may do for you. Either re-point your own edges first, or pass \
                     `accept_withdrawal: true` to refresh anyway: nothing is deleted — each \
                     withdrawn node is KEPT, marked superseded with a snapshot and a change \
                     event, every edge above stays, and what rested on it reads reason-lost \
                     (loop_status, reasoning_report) until you re-ground it or let it go.",
                    withdrawn.len(),
                    withdrawn.join(", "),
                    blocked.join(", "),
                ),
            });
        }

        let existing = self.node_type_index()?;
        let mut collisions = Vec::new();
        let mut mirrored: BTreeSet<String> = BTreeSet::new();
        let mut superseded: Vec<String> = Vec::new();
        let mut snapshots: Vec<String> = Vec::new();
        let mut change_event: Option<String> = None;
        let new_hash = doc
            .content_hash
            .clone()
            .unwrap_or_else(|| "an unhashed surface".to_string());

        self.begin_batch();
        let result = (|| -> Result<(usize, usize), DynoError> {
            let mut nodes_written = 0;
            for n in &doc.nodes {
                if let Some(held_type) = existing.get(&n.node_id) {
                    // OURS TO REFRESH: a node this mirror brought before (in
                    // the last surface, or kept superseded from an earlier
                    // one and published again). Refreshed IN PLACE, so every
                    // local edge into it survives — a refresh used to delete
                    // and re-create, dropping them all.
                    let theirs = previously.contains(&n.node_id)
                        || self
                            .get_node(held_type, &n.node_id)?
                            .and_then(|x| {
                                x.properties
                                    .get("mirrored_from")
                                    .and_then(Value::as_str)
                                    .map(|v| v == source)
                            })
                            .unwrap_or(false);
                    if !theirs {
                        collisions.push(n.node_id.clone());
                        continue;
                    }
                    if held_type != &n.node_type {
                        // A node that changed type is a different node: the
                        // old one cannot carry the new one's edges.
                        self.delete_node(held_type, &n.node_id)?;
                    }
                }
                let mut props: std::collections::HashMap<String, Value> =
                    n.properties.clone().into_iter().collect();
                // Every mirrored node says how it got here. `imported` is the
                // existing provenance value for exactly this (BL-45's "imported
                // reference nodes aren't marked foreign" — now they are).
                props.insert("provenance".into(), Value::from("imported"));
                // Every other mirrored node says WHICH design it came from, so a
                // blast radius that reaches it can name that design and carry
                // the ripple on there (cap:a-cross-design-ripple-follows-each-
                // members-relation). Not `mirror_of`: that key marks the
                // mirrored Project, and readers of it expect only Projects.
                if n.node_type != node::PROJECT {
                    props.insert("mirrored_from".into(), Value::from(source.as_str()));
                }
                if n.node_type == node::PROJECT {
                    props.insert("mirror_of".into(), Value::from(source.as_str()));
                    if let Some(hash) = &doc.content_hash {
                        props.insert("mirror_content_hash".into(), Value::from(hash.as_str()));
                    }
                    if let Some(at) = at {
                        props.insert("mirrored_at".into(), Value::from(at));
                    }
                }
                self.create_node(&n.node_type, &n.node_id, props)?;
                mirrored.insert(n.node_id.clone());
                nodes_written += 1;
            }
            let mut edges_written = 0;
            let mut edge_keys: BTreeSet<String> = BTreeSet::new();
            for e in &doc.edges {
                // Only edges wholly inside what we actually mirrored. An edge
                // touching a collided id is dropped rather than rewired, because
                // pointing their edge at OUR same-named node would fabricate a
                // relationship neither design asserted.
                if mirrored.contains(&e.from_id) && mirrored.contains(&e.to_id) {
                    let props: std::collections::HashMap<String, Value> =
                        e.properties.clone().into_iter().collect();
                    let (from_type, to_type) = (
                        doc.nodes
                            .iter()
                            .find(|n| n.node_id == e.from_id)
                            .map(|n| n.node_type.as_str())
                            .unwrap_or_default(),
                        doc.nodes
                            .iter()
                            .find(|n| n.node_id == e.to_id)
                            .map(|n| n.node_type.as_str())
                            .unwrap_or_default(),
                    );
                    self.create_edge(
                        &e.edge_type,
                        from_type,
                        &e.from_id,
                        to_type,
                        &e.to_id,
                        props,
                    )?;
                    edge_keys.insert(format!("{}|{}|{}", e.edge_type, e.from_id, e.to_id));
                    edges_written += 1;
                }
            }
            // An edge the LAST surface brought between two nodes this one
            // still publishes, and this one no longer carries, is theirs to
            // retract. Edges touching a withdrawn node stay with it.
            for key in previous_edges.difference(&edge_keys) {
                let mut parts = key.splitn(3, '|');
                if let (Some(t), Some(f), Some(to)) = (parts.next(), parts.next(), parts.next())
                    && mirrored.contains(f)
                    && mirrored.contains(to)
                {
                    self.delete_edge(t, f, to)?;
                }
            }
            // WITHDRAWN: KEPT AND MARKED SUPERSEDED, never deleted (part 1).
            // Its state before the mark goes into a snapshot, and one change
            // event records the withdrawal and names each node.
            let index = self.node_type_index()?;
            let mut marked: Vec<(String, String)> = Vec::new();
            for gone in &withdrawn {
                let Some(ty) = index.get(gone).cloned() else {
                    continue;
                };
                let Some(stored) = self.get_node(&ty, gone)? else {
                    continue;
                };
                if stored.properties.contains_key("superseded_by_surface") {
                    continue;
                }
                if self
                    .get_node(node::DESIGN_EPOCH, crate::temporal::PRESERVE_EPOCH_ID)?
                    .is_none()
                {
                    self.add_epoch(
                        crate::temporal::PRESERVE_EPOCH_ID,
                        "Preserved on write",
                        crate::temporal::EpochType::Revision,
                        0,
                    )?;
                }
                let snap = self.snapshot_node(crate::temporal::PRESERVE_EPOCH_ID, &ty, gone)?;
                snapshots.push(snap.node_id);
                let mut props = stored.properties;
                props.insert(
                    "superseded_by_surface".into(),
                    Value::from(new_hash.as_str()),
                );
                if let Some(at) = at {
                    props.insert("superseded_at".into(), Value::from(at));
                }
                self.create_node(&ty, gone, props)?;
                marked.push((ty, gone.clone()));
            }
            if !marked.is_empty() {
                let short: String = new_hash
                    .trim_start_matches("sha256:")
                    .chars()
                    .take(12)
                    .collect();
                let id = format!(
                    "chg:mirror-of-{source}-withdrew-{}-at-{short}",
                    marked.len()
                );
                let names: Vec<&str> = marked.iter().map(|(_, id)| id.as_str()).collect();
                self.add_change_event(
                    &id,
                    &format!(
                        "'{source}' withdrew {} mirrored node(s) at surface {new_hash}",
                        marked.len()
                    ),
                    crate::temporal::ChangeType::Deprecation,
                    Some(crate::temporal::ChangeSubject::System),
                    Some(&format!(
                        "The surface of '{source}' at {new_hash} no longer publishes {}. Each is \
                         KEPT here, marked superseded (`superseded_by_surface`), its prior \
                         state in a snapshot; every local edge touching it stays.",
                        names.join(", ")
                    )),
                    Some(
                        "A refreshed mirror never deletes what the far side withdraws: what \
                         this design rested on it must read reason-lost until its owner \
                         re-grounds it or lets it go (req:a-superseded-mirrored-judgment-is-\
                         kept-and-what-rested-on-it-reads-reason-lost).",
                    ),
                    at,
                )?;
                for (ty, gone) in &marked {
                    self.changed(&id, ty, gone, crate::temporal::ChangeAction::Modified)?;
                }
                superseded = names.iter().map(|s| s.to_string()).collect();
                change_event = Some(id);
            }
            // Recorded here so the next refresh knows which edges were theirs.
            let joined = edge_keys.iter().cloned().collect::<Vec<_>>().join(",");
            if let Some(proj) = doc
                .nodes
                .iter()
                .find(|n| n.node_type == node::PROJECT && mirrored.contains(&n.node_id))
                && let Some(stored) = self.get_node(node::PROJECT, &proj.node_id)?
            {
                let mut props: std::collections::HashMap<String, Value> =
                    stored.properties.into_iter().collect();
                props.insert("mirror_edges".into(), Value::from(joined.as_str()));
                self.create_node(node::PROJECT, &proj.node_id, props)?;
            }
            Ok((nodes_written, edges_written))
        })();
        let kept_edges: Vec<String> = if superseded.is_empty() {
            Vec::new()
        } else {
            blocked.clone()
        };
        match result {
            Ok((mirrored_nodes, mirrored_edges)) => {
                self.commit_batch()?;
                let mut note = if collisions.is_empty() {
                    format!(
                        "Mirrored {mirrored_nodes} node(s) and {mirrored_edges} edge(s) from \
                         '{source}', pinned to that surface's content hash. They are marked \
                         `imported` and are not yours to edit: link to them with your own \
                         provides/consumes edges instead. A newer surface with a different hash \
                         means this mirror is stale."
                    )
                } else {
                    format!(
                        "Mirrored {mirrored_nodes} node(s) and {mirrored_edges} edge(s) from \
                         '{source}'. REFUSED {} id(s) that already exist here and were left \
                         untouched: {}. Two designs using one id for different things is a naming \
                         conversation between their owners — nothing was overwritten and nothing \
                         was guessed.",
                        collisions.len(),
                        collisions.join(", ")
                    )
                };
                if !superseded.is_empty() {
                    note.push_str(&format!(
                        " WITHDRAWN BY THE NEW SURFACE, KEPT AND MARKED SUPERSEDED: {}. \
                         {} local edge(s) still touch them, each kept; what rested on them now \
                         reads reason-lost in loop_status and reasoning_report until you \
                         re-ground it or let it go.",
                        superseded.join(", "),
                        kept_edges.len()
                    ));
                }
                // WHAT WE HOLD OF THEIRS, recorded on their mirrored Project so a
                // later refresh can replace exactly this set. It cannot be
                // recovered afterwards — `provenance: imported` is also set by
                // corpus ingest, and the project's CONTAINS edges do not span the
                // mirrored set (5 of 13, measured). A comma-joined id list follows
                // the schema's existing precedent (`pinned` / `swept` on VERIFIES).
                if let Some(proj) = doc
                    .nodes
                    .iter()
                    .find(|n| n.node_type == node::PROJECT && mirrored.contains(&n.node_id))
                {
                    let list = mirrored.iter().cloned().collect::<Vec<_>>().join(",");
                    // Re-written whole: core `create_node` validates the entire
                    // node, so a partial props object is refused here even though
                    // the MCP layer merges.
                    if let Some(stored) = self.get_node(node::PROJECT, &proj.node_id)? {
                        let mut props: std::collections::HashMap<String, Value> =
                            stored.properties.into_iter().collect();
                        props.insert("mirror_nodes".into(), Value::from(list.as_str()));
                        self.create_node(node::PROJECT, &proj.node_id, props)?;
                    }
                }

                Ok(MirrorReport {
                    mirror_of: source,
                    mirror_content_hash: doc.content_hash.clone(),
                    refreshed,
                    withdrawn: withdrawn.clone(),
                    superseded,
                    kept_edges,
                    snapshots,
                    change_event,
                    mirrored_nodes,
                    mirrored_edges,
                    collisions,
                    note,
                })
            }
            Err(e) => {
                self.discard_batch();
                Err(e)
            }
        }
    }

    /// The designs this graph mirrors: `(project_id, mirror_of, content_hash,
    /// mirrored_at)`, sorted — who we are composed with, and at what version.
    pub fn mirrors(&self) -> Result<Vec<MirrorRef>, DynoError> {
        let mut out = Vec::new();
        for p in self.scan_nodes(node::PROJECT)? {
            let Some(mirror_of) = p.properties.get("mirror_of").and_then(Value::as_str) else {
                continue;
            };
            out.push(MirrorRef {
                project_id: p.node_id.clone(),
                mirror_of: mirror_of.to_string(),
                mirror_content_hash: p
                    .properties
                    .get("mirror_content_hash")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                mirrored_at: p
                    .properties
                    .get("mirrored_at")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }
        out.sort_by(|a, b| a.project_id.cmp(&b.project_id));
        Ok(out)
    }
}

/// One design this graph is composed with, and the version it was pinned to.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MirrorRef {
    pub project_id: String,
    pub mirror_of: String,
    pub mirror_content_hash: Option<String>,
    pub mirrored_at: Option<String>,
}
