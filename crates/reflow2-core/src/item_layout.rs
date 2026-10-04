//! The committed design laid out ONE FILE PER NODE AND ONE PER EDGE, so git's
//! ordinary merge merges it.
//!
//! # Why
//!
//! `req:the-saved-design-merges-correctly-under-gits-ordinary-merge` (accepted
//! 2026-09-22): design and code merge in the same act, on GitHub or anywhere,
//! with no custom merge driver, and disjoint design changes never conflict. The
//! single-file export could not do that, for a measured reason: every export
//! rewrote one shared header (`content_hash`, `prev_content_hash`, `taken_at`),
//! so ALL 46 of 46 concurrently open PR pairs in #640–#668 conflicted on
//! `docs/design/reflow2.json`, 39 of them on nothing else.
//!
//! Settled by Anthony on 2026-10-03 (`dec:how-the-saved-design-is-laid-out-so-git-merges-it`,
//! option a; `dec:the-designs-lineage-is-kept-per-item`, option a):
//!
//! - `nodes/<Type>/<escaped-id>.json` holds one node;
//! - `edges/<2 hex>/<20 hex>.json` holds one edge, named by a stable hash of
//!   its type and endpoints, so an edge keeps its file for life;
//! - `design.json` holds `graph_id`, `schema_version` and, for a design that
//!   was converted, `migrated_from` — nothing a PR rewrites;
//! - every item file carries its own `content_hash` and the `prev_item_hash` of
//!   the version it replaced. The whole-design hash is COMPUTED ON READ and
//!   never stored, so there is no shared field left to conflict on.
//!
//! Two branches then conflict only when they change THE SAME ITEM differently,
//! which is a real conflict and shows up as one small file named after it.
//!
//! # What lives here, and what does not
//!
//! This module is pure: paths, the canonical item body, hashing, rendering,
//! assembling a [`GraphExport`] from files someone else read, and planning a
//! write. It reads no directory and runs no git — the core does no file I/O
//! (`crates/reflow2-mcp/src/saved_design.rs` does both). Keeping the plan here
//! means the one rule that decides what a file holds is written once.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::export::{ExportedEdge, ExportedNode, GraphExport, Props};

/// The small stamp file at the root of the layout.
pub const DESIGN_FILE: &str = "design.json";
/// Where `taken_at` lives now: beside the items, and git-ignored
/// (`dec:item-13-checksums-move-to-change-edges-and-main-converts-in-one-pr`,
/// decision 5). It names the branch an export was taken on, which is a fact
/// about one working tree, never about the design.
pub const TAKEN_AT_FILE: &str = "taken_at.json";
/// The ignore file the layout carries so the sidecar never reaches a commit,
/// whichever project the layout is written into.
pub const GITIGNORE_FILE: &str = ".gitignore";
/// What [`GITIGNORE_FILE`] says.
pub const GITIGNORE_TEXT: &str = "# Written by reflow2. The taken_at sidecar names the working tree an export\n\
     # was taken in; it is local, never part of the design.\n/taken_at.json\n";
/// Directory of node files.
pub const NODES_DIR: &str = "nodes";
/// Directory of edge files.
pub const EDGES_DIR: &str = "edges";

/// The longest file name an escaped id may take before it is shortened. Kept
/// well under every filesystem's 255-byte limit, so a long `snap:` id plus the
/// path above it still fits where Windows-era tooling is in the way.
const MAX_NAME_BYTES: usize = 150;

/// The stamp file's content.
///
/// Deliberately small (decision 5): `reflow2_version` and the vocabulary
/// fingerprint used to ride in the export's stamp, and both changed on PRs
/// that did not touch the design's meaning — a vocabulary edit was 1 of the 27
/// PRs' header conflicts. What stays is what changes only on a schema bump.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignStamp {
    pub graph_id: String,
    pub schema_version: u32,
    /// The whole-design content hash of the single-file export this layout was
    /// converted from. Set once, at conversion, and carried after that — the
    /// link between the two histories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub migrated_from: Option<String>,
}

/// Escape an id into a file name that is safe on every filesystem git runs on:
/// lower-case ASCII letters, digits, `.`, `_` and `-` pass through; any other
/// ASCII byte becomes `%XX` (so `:` survives Windows and an upper-case letter
/// cannot collide with its lower-case twin on a case-insensitive disk); other
/// characters pass through unchanged. A name past [`MAX_NAME_BYTES`] keeps its
/// readable start and ends in `~` and sixteen hex digits of the id's hash, so
/// it stays unique and stable.
pub fn escape_id(id: &str) -> String {
    let mut out = String::with_capacity(id.len() + 8);
    for c in id.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-') {
            out.push(c);
        } else if c.is_ascii() {
            out.push_str(&format!("%{:02X}", c as u32));
        } else {
            out.push(c);
        }
    }
    if out.len() <= MAX_NAME_BYTES {
        return out;
    }
    let mut cut = MAX_NAME_BYTES - 17;
    while !out.is_char_boundary(cut) {
        cut -= 1;
    }
    let digest = hex(&sha256(id.as_bytes()));
    format!("{}~{}", &out[..cut], &digest[..16])
}

/// `nodes/<Type>/<escaped-id>.json`, always with forward slashes.
pub fn node_rel_path(node_type: &str, node_id: &str) -> String {
    format!("{NODES_DIR}/{node_type}/{}.json", escape_id(node_id))
}

/// `edges/<2 hex>/<20 hex>.json`: the first 20 hex digits of
/// sha256(`type \0 from \0 to`), fanned out by their first two. An edge's
/// identity in the store is exactly that triple, so the file never moves.
pub fn edge_rel_path(edge_type: &str, from_id: &str, to_id: &str) -> String {
    let key = hex(&sha256(
        format!("{edge_type}\0{from_id}\0{to_id}").as_bytes(),
    ));
    format!("{EDGES_DIR}/{}/{}.json", &key[..2], &key[..20])
}

fn sha256(bytes: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).to_vec()
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// `sha256:` over the canonical compact JSON of a value — sorted keys, compact
/// separators, minimal escaping. The same canonical form as
/// [`GraphExport::compute_content_hash`], so Python's
/// `json.dumps(v, sort_keys=True, ensure_ascii=False, separators=(",", ":"))`
/// recomputes it (`tools/design_io.py`).
pub fn canonical_hash(value: &JsonValue) -> String {
    let text = serde_json::to_string(value).expect("a JSON value always serializes");
    format!("sha256:{}", hex(&sha256(text.as_bytes())))
}

/// One item of the design, borrowed.
#[derive(Debug, Clone, Copy)]
pub enum ItemRef<'a> {
    Node(&'a ExportedNode),
    Edge(&'a ExportedEdge),
}

impl ItemRef<'_> {
    /// Where this item lives inside the layout.
    pub fn rel_path(&self) -> String {
        match self {
            ItemRef::Node(n) => node_rel_path(&n.node_type, &n.node_id),
            ItemRef::Edge(e) => edge_rel_path(&e.edge_type, &e.from_id, &e.to_id),
        }
    }

    /// The item's own content as JSON — exactly what the single-file export
    /// holds for it, and what its hash covers.
    pub fn body(&self) -> JsonValue {
        match self {
            ItemRef::Node(n) => serde_json::to_value(n),
            ItemRef::Edge(e) => serde_json::to_value(e),
        }
        .expect("export items always serialize")
    }

    /// A label for messages: `Type id` or `TYPE from -> to`.
    pub fn label(&self) -> String {
        match self {
            ItemRef::Node(n) => format!("{} {}", n.node_type, n.node_id),
            ItemRef::Edge(e) => format!("{} {} -> {}", e.edge_type, e.from_id, e.to_id),
        }
    }
}

/// The text of one item file: the body plus `content_hash` and, when the item
/// replaced an earlier version, `prev_item_hash`. Pretty-printed with sorted
/// keys and a trailing newline, so a byte difference always means a content
/// difference and a diff reads one property per line.
pub fn render_item(body: &JsonValue, content_hash: &str, prev_item_hash: Option<&str>) -> String {
    let mut obj = match body {
        JsonValue::Object(m) => m.clone(),
        other => {
            let mut m = serde_json::Map::new();
            m.insert("body".into(), other.clone());
            m
        }
    };
    obj.insert(
        "content_hash".into(),
        JsonValue::String(content_hash.into()),
    );
    if let Some(prev) = prev_item_hash {
        obj.insert("prev_item_hash".into(), JsonValue::String(prev.into()));
    }
    let mut text = serde_json::to_string_pretty(&JsonValue::Object(obj))
        .expect("a JSON value always serializes");
    text.push('\n');
    text
}

/// The stamp file's text.
pub fn render_stamp(stamp: &DesignStamp) -> String {
    let mut text = serde_json::to_string_pretty(
        &serde_json::to_value(stamp).expect("the stamp always serializes"),
    )
    .expect("a JSON value always serializes");
    text.push('\n');
    text
}

/// One item file as found, parsed. `deny_unknown_fields`: a field this reader
/// does not know would otherwise be dropped in silence, and a hand-edited item
/// is exactly where one turns up.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ItemFile {
    #[serde(default)]
    node_type: Option<String>,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    edge_type: Option<String>,
    #[serde(default)]
    from_id: Option<String>,
    #[serde(default)]
    to_id: Option<String>,
    #[serde(default)]
    properties: Props,
    #[serde(default)]
    content_hash: Option<String>,
    #[serde(default)]
    prev_item_hash: Option<String>,
}

/// What one parsed item file holds.
#[derive(Debug, Clone)]
pub enum ParsedBody {
    Node(ExportedNode),
    Edge(ExportedEdge),
}

/// One item file, read and checked against itself.
#[derive(Debug, Clone)]
pub struct ParsedItem {
    /// Its path inside the layout, forward slashes.
    pub rel_path: String,
    pub body: ParsedBody,
    /// The `content_hash` the file states.
    pub stated_hash: Option<String>,
    /// The hash of the body as it actually is.
    pub computed_hash: String,
    /// The `prev_item_hash` the file states.
    pub prev_item_hash: Option<String>,
    /// Whether the file is intact (its stated hash matches its content), so
    /// a write may leave it exactly as it is.
    pub canonical: bool,
}

impl ParsedItem {
    fn item(&self) -> ItemRef<'_> {
        match &self.body {
            ParsedBody::Node(n) => ItemRef::Node(n),
            ParsedBody::Edge(e) => ItemRef::Edge(e),
        }
    }

    /// Where an item with this content BELONGS.
    pub fn expected_path(&self) -> String {
        self.item().rel_path()
    }

    /// The file's own claim agrees with its content.
    pub fn intact(&self) -> bool {
        self.stated_hash.as_deref() == Some(self.computed_hash.as_str())
    }
}

/// Parse one item file. `Err` names the file and what is wrong with it.
pub fn parse_item(rel_path: &str, bytes: &[u8]) -> Result<ParsedItem, String> {
    let file: ItemFile = serde_json::from_slice(bytes)
        .map_err(|e| format!("{rel_path} is not a reflow2 item file: {e}"))?;
    let body = match (
        file.node_type,
        file.node_id,
        file.edge_type,
        file.from_id,
        file.to_id,
    ) {
        (Some(node_type), Some(node_id), None, None, None) => ParsedBody::Node(ExportedNode {
            node_type,
            node_id,
            properties: file.properties,
        }),
        (None, None, Some(edge_type), Some(from_id), Some(to_id)) => {
            ParsedBody::Edge(ExportedEdge {
                edge_type,
                from_id,
                to_id,
                properties: file.properties,
            })
        }
        _ => {
            return Err(format!(
                "{rel_path} is neither a node (node_type + node_id) nor an edge (edge_type + \
                 from_id + to_id)"
            ));
        }
    };
    let mut parsed = ParsedItem {
        rel_path: rel_path.to_string(),
        body,
        stated_hash: file.content_hash,
        computed_hash: String::new(),
        prev_item_hash: file.prev_item_hash,
        canonical: false,
    };
    parsed.computed_hash = canonical_hash(&parsed.item().body());
    // An intact item is taken as written: re-rendering 50,000 files on every
    // read to compare bytes cost more than the read itself, and a file whose
    // content matches its own hash loses nothing by keeping its formatting.
    parsed.canonical = parsed.intact();
    Ok(parsed)
}

/// A whole layout, assembled into the one-document shape every reader takes.
#[derive(Debug, Clone)]
pub struct Assembled {
    /// The design, sorted exactly as `export_graph` sorts it, with
    /// `content_hash` COMPUTED (it is never stored) and no chain fields.
    pub export: GraphExport,
    /// The stamp file.
    pub stamp: DesignStamp,
    /// Item files whose stated `content_hash` does not match their content:
    /// edited outside reflow2, or corrupted. Named, never fatal here — the
    /// gate fails on them, an import reports them.
    pub tampered: Vec<String>,
    /// Item files at a path other than the one their content belongs at.
    pub misplaced: Vec<String>,
}

impl Assembled {
    /// One sentence for an import report, or None when every item is intact.
    pub fn integrity_note(&self) -> Option<String> {
        if self.tampered.is_empty() && self.misplaced.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        if !self.tampered.is_empty() {
            parts.push(format!(
                "{} item file(s) do not match their own content_hash — edited outside reflow2 \
                 or corrupted: {}",
                self.tampered.len(),
                sample(&self.tampered)
            ));
        }
        if !self.misplaced.is_empty() {
            parts.push(format!(
                "{} item file(s) sit at a path their content does not belong at: {}",
                self.misplaced.len(),
                sample(&self.misplaced)
            ));
        }
        Some(parts.join("; "))
    }
}

fn sample(items: &[String]) -> String {
    let shown: Vec<&str> = items.iter().take(5).map(String::as_str).collect();
    let more = items.len().saturating_sub(5);
    if more > 0 {
        format!("{} (+{more} more)", shown.join(", "))
    } else {
        shown.join(", ")
    }
}

/// Assemble parsed items into one document. `Err` when two files hold the same
/// item — a merge that kept both copies, or a hand copy — because choosing one
/// silently would drop the other.
pub fn assemble(stamp: DesignStamp, items: Vec<ParsedItem>) -> Result<Assembled, String> {
    let mut tampered = Vec::new();
    let mut misplaced = Vec::new();
    let mut nodes: BTreeMap<(String, String), ExportedNode> = BTreeMap::new();
    let mut edges: BTreeMap<(String, String, String), ExportedEdge> = BTreeMap::new();
    let mut seen_at: BTreeMap<String, String> = BTreeMap::new();
    let mut duplicates = Vec::new();
    for item in items {
        if !item.intact() {
            tampered.push(item.rel_path.clone());
        }
        let expected = item.expected_path();
        if expected != item.rel_path {
            misplaced.push(format!("{} (belongs at {expected})", item.rel_path));
        }
        let label = item.item().label();
        if let Some(first) = seen_at.insert(label.clone(), item.rel_path.clone()) {
            duplicates.push(format!("{label} in {first} and {}", item.rel_path));
            continue;
        }
        match item.body {
            ParsedBody::Node(n) => {
                nodes.insert((n.node_type.clone(), n.node_id.clone()), n);
            }
            ParsedBody::Edge(e) => {
                edges.insert((e.edge_type.clone(), e.from_id.clone(), e.to_id.clone()), e);
            }
        }
    }
    if !duplicates.is_empty() {
        return Err(format!(
            "{} item(s) are held by two files, and choosing one would drop the other: {}",
            duplicates.len(),
            sample(&duplicates)
        ));
    }
    tampered.sort();
    misplaced.sort();
    let mut export = GraphExport {
        taken_at: None,
        stamp: None,
        content_hash: None,
        prev_content_hash: None,
        graph_id: stamp.graph_id.clone(),
        nodes: nodes.into_values().collect(),
        edges: edges.into_values().collect(),
    };
    export.content_hash = Some(export.compute_content_hash());
    Ok(Assembled {
        export,
        stamp,
        tampered,
        misplaced,
    })
}

/// An item file already on disk, as the planner needs to know it.
#[derive(Debug, Clone)]
pub struct OnDisk {
    /// The hash of its body as it actually is.
    pub computed_hash: String,
    /// What it says it replaced.
    pub prev_item_hash: Option<String>,
    /// Intact: its stated hash matches its content.
    pub canonical: bool,
}

impl From<&ParsedItem> for OnDisk {
    fn from(p: &ParsedItem) -> Self {
        Self {
            computed_hash: p.computed_hash.clone(),
            prev_item_hash: p.prev_item_hash.clone(),
            canonical: p.canonical,
        }
    }
}

/// An item as the committed anchor holds it: its hash and what it replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchored {
    pub content_hash: String,
    pub prev_item_hash: Option<String>,
}

/// Where per-item lineage is anchored.
pub enum Anchor<'a> {
    /// The committed record at the merge-base with the default branch. Called
    /// ONCE with the paths of every item this write changes, and answers with
    /// those that exist there. An item it does not return is new on this
    /// branch, so its `prev_item_hash` is absent.
    Committed(&'a mut dyn FnMut(&[String]) -> BTreeMap<String, Anchored>),
    /// No committed record to anchor at (outside git, or before the first
    /// commit): an item chains from the file it replaces on disk.
    Disk,
}

/// What a write will do. The caller performs it.
#[derive(Debug, Clone, Default)]
pub struct WritePlan {
    /// `(rel_path, text)` for every file to create or replace.
    pub writes: Vec<(String, String)>,
    /// Item files on disk that hold an item the design no longer has.
    pub deletes: Vec<String>,
    /// Item files left exactly as they were.
    pub unchanged: usize,
    /// Items whose content changed (created or modified), not counting files
    /// only reformatted.
    pub changed_items: usize,
    /// Items whose content did NOT change but whose `prev_item_hash` did not
    /// name their hash at the anchor, rewritten so it does — what a merge of
    /// the default branch leaves behind when it was resolved by an export
    /// anchored before the merge (see [`plan_write_rechecking`]).
    pub relinked: usize,
}

/// Plan writing `export` over a layout whose item files are `on_disk`.
///
/// THE LINEAGE RULE (`dec:the-designs-lineage-is-kept-per-item`): an item whose
/// content did not change keeps its file byte for byte, whatever exported it. A
/// changed item's `prev_item_hash` is ITS hash at the anchor — so any number of
/// exports on a branch, before or during a merge of main, chain every changed
/// item from the same committed version, and a squash-merge lands each one
/// exactly one hop. An item changed back to what the anchor holds gets the
/// anchor's file back exactly, so it leaves no diff. An item the anchor lacks is
/// new on this branch and carries no `prev_item_hash`.
pub fn plan_write(
    export: &GraphExport,
    on_disk: &BTreeMap<String, OnDisk>,
    anchor: Anchor<'_>,
) -> WritePlan {
    plan_write_rechecking(export, on_disk, anchor, &BTreeSet::new())
}

/// [`plan_write`], and also RE-CHECK the lineage of the items in `recheck` —
/// the item files that already differ from the anchor in the working tree,
/// which is what the gate judges — even when this write leaves their content
/// alone. Only with [`Anchor::Committed`]; outside git there is no anchor to
/// check against.
///
/// Why a write must do this, measured 2026-10-04 on #672: an item both sides
/// changed is resolved DURING a merge of the default branch, and an export
/// anchored before that merge chained it from the version main had already
/// replaced. Once the merge was committed the gate failed LINEAGE on it, and
/// every later export left the file alone because its content had not moved —
/// so the remedy the gate names, "re-export from the graph", did nothing. The
/// lineage is the anchor's to state, not the file's: a changed item whose
/// `prev_item_hash` does not name its hash at the anchor is rewritten so it
/// does, and an item whose content is back to the anchor's gets the anchor's
/// lineage back, so it leaves no diff.
pub fn plan_write_rechecking(
    export: &GraphExport,
    on_disk: &BTreeMap<String, OnDisk>,
    anchor: Anchor<'_>,
    recheck: &BTreeSet<String>,
) -> WritePlan {
    let mut plan = WritePlan::default();
    let mut wanted: BTreeSet<String> = BTreeSet::new();
    let from_disk = matches!(anchor, Anchor::Disk);
    // (rel, body, hash) for items whose content differs from disk.
    let mut changed: Vec<(String, JsonValue, String)> = Vec::new();
    // (rel, body, hash, prev on disk, canonical) for items whose content is
    // what disk holds but whose lineage the anchor is asked to confirm.
    let mut rechecked: Vec<(String, JsonValue, String, Option<String>, bool)> = Vec::new();
    let items = export
        .nodes
        .iter()
        .map(ItemRef::Node)
        .chain(export.edges.iter().map(ItemRef::Edge));
    for item in items {
        let rel = item.rel_path();
        let body = item.body();
        let hash = canonical_hash(&body);
        wanted.insert(rel.clone());
        match on_disk.get(&rel) {
            Some(d) if d.computed_hash == hash && !from_disk && recheck.contains(&rel) => {
                rechecked.push((rel, body, hash, d.prev_item_hash.clone(), d.canonical));
            }
            Some(d) if d.computed_hash == hash && d.canonical => plan.unchanged += 1,
            Some(d) if d.computed_hash == hash => {
                // Same content, non-canonical bytes (a hand reformat, or a
                // stated hash that no longer matches). Rewritten in place with
                // the lineage it already carries — content did not move.
                let text = render_item(&body, &hash, d.prev_item_hash.as_deref());
                plan.writes.push((rel, text));
            }
            _ => changed.push((rel, body, hash)),
        }
    }
    let anchored: BTreeMap<String, Anchored> = match anchor {
        Anchor::Committed(lookup) if !changed.is_empty() || !rechecked.is_empty() => {
            let rels: Vec<String> = changed
                .iter()
                .map(|(r, _, _)| r.clone())
                .chain(rechecked.iter().map(|(r, ..)| r.clone()))
                .collect();
            lookup(&rels)
        }
        _ => BTreeMap::new(),
    };
    // An item's lineage at the anchor: its hash there, or — when its content
    // is back to exactly the anchor's — the anchor's own predecessor, so the
    // file comes back byte for byte.
    let at_anchor = |rel: &str, hash: &str| -> Option<String> {
        match anchored.get(rel) {
            Some(a) if a.content_hash == hash => a.prev_item_hash.clone(),
            Some(a) => Some(a.content_hash.clone()),
            None => None,
        }
    };
    for (rel, body, hash) in changed {
        let prev: Option<String> = if from_disk {
            // Outside git: chain from the version this replaces on disk.
            on_disk.get(&rel).map(|d| d.computed_hash.clone())
        } else {
            at_anchor(&rel, &hash)
        };
        plan.changed_items += 1;
        let text = render_item(&body, &hash, prev.as_deref());
        plan.writes.push((rel, text));
    }
    for (rel, body, hash, prev_on_disk, canonical) in rechecked {
        let prev = at_anchor(&rel, &hash);
        if prev == prev_on_disk && canonical {
            plan.unchanged += 1;
            continue;
        }
        if prev != prev_on_disk {
            plan.relinked += 1;
        }
        let text = render_item(&body, &hash, prev.as_deref());
        plan.writes.push((rel, text));
    }
    for rel in on_disk.keys() {
        if !wanted.contains(rel) {
            plan.deletes.push(rel.clone());
        }
    }
    plan
}
