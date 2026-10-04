//! The one place a saved design is READ from a path — the single-file export
//! or the per-item layout — and the one place the item layout is WRITTEN.
//!
//! # Why one reader
//!
//! Until 2026-10-03 every reader parsed the export with its own
//! `serde_json::from_str::<GraphExport>`: the import tool, `--import`,
//! `--diff`, `--merge`, `--merge-apply`, `--merge-driver`, the upstream watch,
//! the stale-seat check, the write-through's hand-edit guard. Ten copies of one
//! line is harmless while the format is one file and a trap the day it is not
//! (`dec:how-the-saved-design-is-laid-out-so-git-merges-it`): the reader that
//! is missed reads a directory as "cannot read" and a design looks gone. So
//! every reader comes through [`read_design`], which takes either form for the
//! one-release overlap the decision grants.
//!
//! # Which form a path means
//!
//! A path that IS a directory, or ends in `/`, names the item layout; anything
//! else (`docs/design/reflow2.json`) names the single file, as it always has.
//! So a first export into a place that does not exist yet lands in the form
//! the caller spelled: `docs/design/reflow2/` for the layout.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use reflow2_core::GraphExport;
use reflow2_core::item_layout::{
    self, Anchor, Anchored, DESIGN_FILE, DesignStamp, EDGES_DIR, GITIGNORE_FILE, GITIGNORE_TEXT,
    NODES_DIR, OnDisk, ParsedItem, TAKEN_AT_FILE,
};

use crate::export_write::{WriteRefusal, Written};

/// Whether `path` names the per-item layout (true) or the single file.
pub fn is_item_layout(path: &Path) -> bool {
    if path.is_dir() {
        return true;
    }
    let text = path.to_string_lossy();
    text.ends_with('/') || text.ends_with(std::path::MAIN_SEPARATOR)
}

/// What reading the item layout found besides the design itself.
#[derive(Debug, Clone)]
pub struct ItemsRead {
    pub stamp: DesignStamp,
    /// Item files that do not match their own `content_hash`, or sit at a path
    /// their content does not belong at — None when every one is intact.
    pub integrity_note: Option<String>,
    /// How many item files were read.
    pub files: usize,
}

/// A saved design, read.
#[derive(Debug, Clone)]
pub struct ReadDesign {
    pub export: GraphExport,
    /// Set when the design came from the item layout.
    pub items: Option<ItemsRead>,
}

impl ReadDesign {
    /// The note an import should carry about this document's provenance, in
    /// place of the single file's "unstamped" note: the item layout's stamp is
    /// cut to `graph_id` and `schema_version` by decision
    /// (`dec:item-13-checksums-move-to-change-edges-and-main-converts-in-one-pr`,
    /// decision 5), so it never says which reflow2 wrote it.
    pub fn layout_note(&self) -> Option<String> {
        self.items.as_ref().map(|i| {
            format!(
                "read from the per-item layout (schema_version {}, {} item files): its stamp \
                 carries graph_id and schema_version only, so which reflow2 version wrote it is \
                 not recorded — `materialized` names anything this build filled in",
                i.stamp.schema_version, i.files
            )
        })
    }
}

/// The key a path is recorded under in the sync sidecar: an item layout's
/// path without its trailing separator, so `docs/design/reflow2/` and
/// `docs/design/reflow2` are one target, not two.
pub fn sync_key(path: &str) -> String {
    if is_item_layout(Path::new(path)) {
        let trimmed = path.trim_end_matches(['/', std::path::MAIN_SEPARATOR]);
        if trimmed.is_empty() {
            path.to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        path.to_string()
    }
}

/// THE ITEM LAYOUT'S VERSION GUARD. Its stamp says which `schema_version`
/// wrote it and nothing finer (decision 5), so that is the guard it gets: a
/// layout written under a NEWER schema version than this binary knows is
/// refused unless `accept_newer`, the same opt-in the single file's
/// reflow2-version guard takes. Nothing to check for the single file here —
/// the import itself checks its stamp.
pub fn refuse_newer_schema(read: &ReadDesign, accept_newer: bool) -> Result<(), String> {
    let Some(items) = &read.items else {
        return Ok(());
    };
    let ours = reflow2_core::load_schema()
        .map(|s| s.version)
        .unwrap_or(items.stamp.schema_version);
    if items.stamp.schema_version > ours && !accept_newer {
        return Err(format!(
            "REFUSED: this design was saved under schema version {}, and this reflow2 ({}) knows \
             version {ours}, which is BEHIND it. Nothing was written. Update reflow2 and import \
             again — or pass accept_newer to read it with this binary, and read the report's \
             `materialized` field for exactly what it wrote that the design did not state.",
            items.stamp.schema_version,
            env!("CARGO_PKG_VERSION"),
        ));
    }
    Ok(())
}

/// Put what reading the item layout found onto an import's report: the
/// per-item integrity finding, and the layout's provenance in place of the
/// single file's "unstamped" note (the layout has a stamp — a smaller one).
pub fn annotate_import(read: &ReadDesign, report: &mut reflow2_core::ImportReport) {
    let Some(items) = &read.items else {
        return;
    };
    if let Some(note) = &items.integrity_note {
        report.integrity_note = Some(note.clone());
    }
    report.provenance_note = read.layout_note();
}

/// Read a saved design from `path`, in whichever form the path names.
/// `Err` is a sentence naming the path and what is wrong.
pub fn read_design(path: &str) -> Result<ReadDesign, String> {
    let p = Path::new(path);
    if is_item_layout(p) {
        let (stamp, items) = read_items(p)?;
        let Some(stamp) = stamp else {
            return Err(format!(
                "{path} is not a reflow2 design: there is no {DESIGN_FILE} in it (an item layout \
                 always has one)"
            ));
        };
        let files = items.len();
        let assembled = item_layout::assemble(stamp, items)
            .map_err(|e| format!("{path} cannot be read as one design: {e}"))?;
        let integrity_note = assembled.integrity_note();
        return Ok(ReadDesign {
            export: assembled.export,
            items: Some(ItemsRead {
                stamp: assembled.stamp,
                integrity_note,
                files,
            }),
        });
    }
    let raw = std::fs::read_to_string(p)
        .map_err(|e| format!("failed to read the design from {path}: {e}"))?;
    let export: GraphExport = serde_json::from_str(&raw)
        .map_err(|e| format!("{path} is not a reflow2 export document: {e}"))?;
    Ok(ReadDesign {
        export,
        items: None,
    })
}

/// Read just the document (the common case).
pub fn read_export(path: &str) -> Result<GraphExport, String> {
    read_design(path).map(|r| r.export)
}

/// Read every item file under `dir`, and its stamp when there is one.
fn read_items(dir: &Path) -> Result<(Option<DesignStamp>, Vec<ParsedItem>), String> {
    let stamp_path = dir.join(DESIGN_FILE);
    let stamp = if stamp_path.exists() {
        let raw = std::fs::read(&stamp_path)
            .map_err(|e| format!("cannot read {}: {e}", stamp_path.display()))?;
        Some(serde_json::from_slice::<DesignStamp>(&raw).map_err(|e| {
            format!(
                "{} is not a reflow2 design stamp: {e}",
                stamp_path.display()
            )
        })?)
    } else {
        None
    };
    let mut items = Vec::new();
    for top in [NODES_DIR, EDGES_DIR] {
        let root = dir.join(top);
        if !root.exists() {
            continue;
        }
        walk(&root, &mut |file: &Path| -> Result<(), String> {
            let rel = file
                .strip_prefix(dir)
                .map_err(|_| format!("{} escaped {}", file.display(), dir.display()))?
                .to_string_lossy()
                .replace('\\', "/");
            if !rel.ends_with(".json") {
                return Err(format!(
                    "{} holds {rel}, which is not an item file — a reflow2 item layout holds \
                     only .json items under {NODES_DIR}/ and {EDGES_DIR}/",
                    dir.display()
                ));
            }
            let bytes =
                std::fs::read(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
            items.push(item_layout::parse_item(&rel, &bytes)?);
            Ok(())
        })?;
    }
    Ok((stamp, items))
}

/// Depth-first over regular files; hidden entries (a `.DS_Store`, an editor's
/// swap file) are skipped, everything else is handed to `each`.
fn walk(dir: &Path, each: &mut dyn FnMut(&Path) -> Result<(), String>) -> Result<(), String> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("cannot list {}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            !p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('.'))
        })
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            walk(&p, each)?;
        } else {
            each(&p)?;
        }
    }
    Ok(())
}

/// A cheap fingerprint of an item layout for "could it have moved?" checks:
/// the newest modification time among the stamp file, the layout's directory
/// and every directory under `nodes/` and `edges/`, plus how many directories
/// that is. Every write reflow2 and git make creates or renames an entry, which
/// moves its directory's time, so this sees them at the cost of a few hundred
/// stats rather than tens of thousands. An editor that rewrites one item IN
/// PLACE is the case it cannot see; the full read still does.
pub fn layout_stat(dir: &Path) -> Option<(u64, i64)> {
    fn mtime(p: &Path) -> Option<i64> {
        std::fs::metadata(p)
            .ok()?
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_nanos() as i64)
    }
    let mut newest = mtime(dir)?;
    let mut count = 1u64;
    if let Some(t) = mtime(&dir.join(DESIGN_FILE)) {
        newest = newest.max(t);
    }
    for top in [NODES_DIR, EDGES_DIR] {
        let root = dir.join(top);
        let Some(t) = mtime(&root) else { continue };
        newest = newest.max(t);
        count += 1;
        if let Ok(rd) = std::fs::read_dir(&root) {
            for e in rd.flatten() {
                if let Some(t) = mtime(&e.path()) {
                    newest = newest.max(t);
                    count += 1;
                }
            }
        }
    }
    Some((count, newest))
}

/// Write `contents` to `path` through a sibling temporary file and a rename,
/// so a reader never sees half an item and the directory's time moves.
fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

/// Remove `rel`'s file and any directory it leaves empty under the layout.
fn remove_item(dir: &Path, rel: &str) -> std::io::Result<()> {
    let path = dir.join(rel);
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let mut parent = path.parent().map(Path::to_path_buf);
    while let Some(p) = parent {
        if p == dir || !p.starts_with(dir) {
            break;
        }
        if std::fs::remove_dir(&p).is_err() {
            break; // not empty, or not ours to remove
        }
        parent = p.parent().map(Path::to_path_buf);
    }
    Ok(())
}

/// The entries a directory may already hold for reflow2 to write a layout
/// into it without asking.
fn is_layout_entry(name: &str) -> bool {
    matches!(
        name,
        DESIGN_FILE | NODES_DIR | EDGES_DIR | GITIGNORE_FILE | TAKEN_AT_FILE
    )
}

/// Write `export` as the item layout at `dir`. The directory form of
/// [`crate::export_write::chain_and_write`], which calls it: the same four
/// things happen at this seam — where lineage anchors, whether the write would
/// drop design the layout already holds, what `wrote` reports, and recording
/// that this seat is in step.
pub(crate) fn write_items(
    export: &mut GraphExport,
    dir_text: &str,
    graph_path: Option<&str>,
    record_to: Option<&str>,
    accept_divergence: bool,
) -> Result<Written, WriteRefusal> {
    let dir = Path::new(dir_text.trim_end_matches(['/', std::path::MAIN_SEPARATOR]));
    let dir = if dir.as_os_str().is_empty() {
        Path::new(dir_text)
    } else {
        dir
    };
    let path_key = dir.to_string_lossy().to_string();

    // WHAT IS THERE NOW. A directory that holds anything but a layout is not
    // reflow2's to write into: an export pointed at the wrong place must not
    // scatter files through somebody's folder.
    let mut existing_stamp: Option<DesignStamp> = None;
    let mut parsed: Vec<ParsedItem> = Vec::new();
    if dir.exists() {
        if !dir.is_dir() {
            return Err(WriteRefusal::Io(format!(
                "{path_key} exists and is not a directory — an item layout is a directory. Name a \
                 .json path for the single-file export, or a directory for the item layout."
            )));
        }
        let foreign: Vec<String> = std::fs::read_dir(dir)
            .map_err(|e| WriteRefusal::Io(format!("cannot list {path_key}: {e}")))?
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().to_str().map(str::to_string))
            .filter(|n| !is_layout_entry(n) && !n.starts_with('.'))
            .collect();
        if !dir.join(DESIGN_FILE).exists() && !foreign.is_empty() {
            return Err(WriteRefusal::Io(format!(
                "{path_key} already holds files that are not a reflow2 item layout ({}), so \
                 nothing was written. Export into an empty directory, or into one that already \
                 holds a layout.",
                foreign.join(", ")
            )));
        }
        let (stamp, items) = read_items(dir).map_err(|e| {
            WriteRefusal::Io(format!(
                "{e}. The layout at {path_key} could not be read, so nothing was written — a \
                 half-resolved merge looks exactly like this; resolve it and export again."
            ))
        })?;
        existing_stamp = stamp;
        parsed = items;
    }

    // THE STALE-SEAT GUARD, as for the single file: would this write drop
    // design the layout already holds? Assembled from what is there.
    let mut sync_note = None;
    let mut wrote = if existing_stamp.is_some() {
        "changed"
    } else {
        "created"
    };
    let on_disk: BTreeMap<String, OnDisk> = parsed
        .iter()
        .map(|p| (p.rel_path.clone(), OnDisk::from(p)))
        .collect();
    if let Some(stamp) = existing_stamp.clone() {
        match item_layout::assemble(stamp, parsed) {
            Ok(predecessor) => {
                let last =
                    graph_path.and_then(|g| reflow2_core::provenance::last_synced(g, &path_key));
                let verdict = reflow2_core::sync::assess_overwrite(
                    Some(&predecessor.export),
                    export,
                    last.as_deref(),
                );
                if verdict.is_loss() && !accept_divergence {
                    return Err(WriteRefusal::Loss(
                        verdict.message(&path_key).unwrap_or_default(),
                    ));
                }
                sync_note = verdict.message(&path_key);
                if predecessor.export.effective_content_hash() == export.effective_content_hash() {
                    wrote = "unchanged";
                }
            }
            Err(e) => {
                if !accept_divergence {
                    return Err(WriteRefusal::Loss(format!(
                        "{path_key} cannot be read as one design ({e}), so writing over it could \
                         drop what one of the copies holds. Resolve it, or pass \
                         accept_divergence: true to replace it with this graph's design."
                    )));
                }
            }
        }
    }

    // WHERE PER-ITEM LINEAGE ANCHORS: the record as committed at the
    // merge-base with the default branch, asked once for every item this
    // write changes. Outside git, the file each item replaces on disk.
    let anchor = crate::git::item_anchor(dir);
    let (chained_from, chain_note) = match &anchor {
        Ok(a) => (a.source.clone(), None),
        Err(reason) => ("disk".to_string(), Some(reason.reason())),
    };
    let plan = match &anchor {
        Ok(a) => {
            let mut lookup = |rels: &[String]| -> BTreeMap<String, Anchored> { a.lookup(rels) };
            item_layout::plan_write(export, &on_disk, Anchor::Committed(&mut lookup))
        }
        Err(_) => item_layout::plan_write(export, &on_disk, Anchor::Disk),
    };

    // THE STAMP: graph_id and schema_version, and where a converted design
    // came from. `migrated_from` is set once — when this writes a layout for
    // the first time beside the single file it replaces (`<dir>.json`) — and
    // carried from then on.
    let schema_version = export.stamp.as_ref().map(|s| s.schema_version).unwrap_or(1);
    let migrated_from = match &existing_stamp {
        Some(s) => s.migrated_from.clone(),
        None => {
            let sibling = PathBuf::from(format!("{path_key}.json"));
            std::fs::read_to_string(&sibling)
                .ok()
                .and_then(|raw| serde_json::from_str::<GraphExport>(&raw).ok())
                .map(|doc| doc.compute_content_hash())
        }
    };
    let stamp = DesignStamp {
        graph_id: export.graph_id.clone(),
        schema_version,
        migrated_from,
    };

    let mut bytes = 0usize;
    let io = |e: std::io::Error, what: &str| {
        WriteRefusal::Io(format!(
            "cannot write the design to {path_key} ({what}): {e}"
        ))
    };
    std::fs::create_dir_all(dir).map_err(|e| io(e, "creating it"))?;
    if existing_stamp.as_ref() != Some(&stamp) {
        let text = item_layout::render_stamp(&stamp);
        bytes += text.len();
        write_atomic(&dir.join(DESIGN_FILE), text.as_bytes()).map_err(|e| io(e, DESIGN_FILE))?;
    }
    let ignore = dir.join(GITIGNORE_FILE);
    if std::fs::read_to_string(&ignore).ok().as_deref() != Some(GITIGNORE_TEXT) {
        std::fs::write(&ignore, GITIGNORE_TEXT).map_err(|e| io(e, GITIGNORE_FILE))?;
    }
    for (rel, text) in &plan.writes {
        bytes += text.len();
        write_atomic(&dir.join(rel), text.as_bytes()).map_err(|e| io(e, rel))?;
    }
    for rel in &plan.deletes {
        remove_item(dir, rel).map_err(|e| io(e, rel))?;
    }

    // WHERE THIS WAS TAKEN — in the git-ignored sidecar now (decision 5), so
    // it is never part of what a PR commits. Kept while nothing moved.
    export.prev_content_hash = None;
    let sidecar = dir.join(TAKEN_AT_FILE);
    let kept = (wrote == "unchanged")
        .then(|| {
            std::fs::read_to_string(&sidecar)
                .ok()
                .and_then(|raw| serde_json::from_str::<reflow2_core::TakenAt>(&raw).ok())
        })
        .flatten();
    export.taken_at = kept.or_else(|| crate::git::taken_at(&dir.join(DESIGN_FILE)));
    if let Some(taken) = &export.taken_at
        && let Ok(text) = serde_json::to_string_pretty(taken)
    {
        let _ = std::fs::write(&sidecar, format!("{text}\n"));
    }

    if let (Some(gp), Some(hash)) = (record_to, &export.content_hash) {
        reflow2_core::provenance::record_sync(gp, &path_key, hash);
    }

    Ok(Written {
        wrote,
        chained_from,
        chain_note,
        sync_note,
        bytes,
        items: Some(crate::export_write::ItemCounts {
            written: plan.writes.len(),
            changed: plan.changed_items,
            deleted: plan.deletes.len(),
            unchanged: plan.unchanged,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_names_its_form_by_its_shape() {
        assert!(!is_item_layout(Path::new("docs/design/reflow2.json")));
        assert!(is_item_layout(Path::new("docs/design/reflow2/")));
        // A path that does not exist and does not end in a separator keeps
        // the old meaning, whatever its extension.
        assert!(!is_item_layout(Path::new("no/such/place/reflow2")));
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(is_item_layout(dir.path()));
    }
}
