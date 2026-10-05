//! A hub's ADDRESS BOOK: where each member design's store is, ON THIS MACHINE.
//!
//! # The finding this answers
//!
//! A local hub's list of member designs is its declared dependencies, one
//! `external_dependency` per member, committed and the same on every machine.
//! What no record held is how THIS machine reaches each member: a door agent
//! (`--call`) opens exactly one `--graph-path`, so it could tell a member had
//! moved and could not open it
//! (`fact:root-cause-a-local-hubs-list-is-its-watch-manifest-but-the-hub-skill-names-the-mcp-config-and-no-pin-says-where-a-store-is-2026-10-02`).
//! A work hub kept the answer by hand: a committed list of members, and a
//! generated, never-committed file of absolute store paths beside it.
//!
//! `dec:idea-a-hub-on-one-machine-can-say-where-each-tracked-design-is-reached-from-the-door`,
//! accepted 2026-10-05: *a per-machine record, never committed, mapping each
//! member design to its store path, written by add_member.* This is that record.
//!
//! # Why it lives beside the user's reflow2 settings, never in a repository
//!
//! A store path is a fact about one machine. Committed, it would be wrong on
//! every other machine, and it would put a per-machine fact into the manifest
//! the accepted rule keeps machine-independent: a member is watched by its
//! server's address, never through a file
//! (`dec:idea-one-blueprint-the-store-is-the-design-and-an-export-is-a-perishable-photocopy`).
//! So the book is kept in the per-user settings folder
//! ([`crate::client_setup::config_dir`]), one file per hub, keyed by the hub's
//! graph id, and a write that would land inside a git work tree is REFUSED: a
//! settings folder pointed into a repository is the one way the book could be
//! committed by accident. Nothing in the hub's design names it.
//!
//! # A stale entry is reported, never dropped
//!
//! A store can move or be deleted after it was recorded. Every read checks
//! each entry against what is at its path NOW, by the identity file beside the
//! store (no store is opened for that), and an entry whose store is gone, or
//! holds another design, is reported as `stale` with the reason. Nothing here
//! deletes an entry: replacing one is `add_member` with `replace`, a deliberate
//! act by the person who knows where the store went.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// What the file says it is, so a stray JSON file is never read as a book.
pub const FORMAT: &str = "reflow2-hub-address-book";
/// The file's shape. Bumped only by a change an older reader cannot read.
pub const VERSION: u32 = 1;

/// The sentence every book carries about itself, so a person who finds the
/// file knows what it is and that it must not be committed.
pub const SELF_NOTE: &str = "reflow2's per-machine address book for one hub: where each member \
     design's store is on THIS machine. Written by add_member; read by member_stores and \
     upstream_status. Never commit it: store paths differ per machine, and the hub's committed \
     design names each member by its graph id only.";

/// One hub's address book.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressBook {
    pub format: String,
    pub version: u32,
    /// The hub this book belongs to — the design whose pins it resolves.
    pub hub_graph_id: String,
    #[serde(default)]
    pub hub_name: Option<String>,
    #[serde(default)]
    pub note: String,
    /// Each member, by its graph id.
    #[serde(default)]
    pub members: BTreeMap<String, Entry>,
}

/// Where one member design's store is on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The member design's graph id, as its identity file named it when the
    /// entry was written.
    pub graph_id: String,
    /// Its label at that moment.
    pub name: String,
    /// The hub's pin for it (`dep:…`).
    pub dependency: String,
    /// The store's absolute path — what `--graph-path` takes to open it.
    pub store: String,
    /// The export `add_member` wrote for it, when it was asked to: what
    /// `upstream_status` compares the live store against when the pin names no
    /// export of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export: Option<String>,
    /// The day the entry was written (UTC).
    pub recorded_on: String,
}

impl AddressBook {
    pub fn new(hub_graph_id: &str, hub_name: Option<String>) -> Self {
        Self {
            format: FORMAT.to_string(),
            version: VERSION,
            hub_graph_id: hub_graph_id.to_string(),
            hub_name,
            note: SELF_NOTE.to_string(),
            members: BTreeMap::new(),
        }
    }
}

/// Where the book for the hub `hub_graph_id` is kept on this machine:
/// `<settings folder>/hubs/<hub graph id>.json`.
pub fn path_for(hub_graph_id: &str) -> anyhow::Result<PathBuf> {
    Ok(crate::client_setup::config_dir()?
        .join("hubs")
        .join(format!("{}.json", file_stem(hub_graph_id))))
}

/// A graph id as a file name: anything but letters, digits, `-`, `_` and `.`
/// becomes `_`, and a name of dots alone is never a path step.
fn file_stem(graph_id: &str) -> String {
    let s: String = graph_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() || s.chars().all(|c| c == '.') {
        format!("_{s}")
    } else {
        s
    }
}

/// The git work tree `path` is inside, if any: the nearest ancestor (the path
/// itself included) holding a `.git` directory or file. A path that does not
/// exist yet is judged by its nearest existing ancestor.
///
/// Read from the file system, not asked of git, so it answers the same with or
/// without git installed. A `.git` FILE counts: a worktree and a submodule
/// carry one, and both are repositories a file can be committed to.
pub fn enclosing_repository(path: &Path) -> Option<PathBuf> {
    let absolute = absolute(path);
    absolute
        .ancestors()
        .find(|a| a.join(".git").exists())
        .map(Path::to_path_buf)
}

/// `path` made absolute against the current directory, without resolving
/// links or requiring that it exist.
pub fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|d| d.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

/// Read the book at `path`. `Ok(None)` when there is none — the ordinary state
/// of a hub nobody has run `add_member` for on this machine.
///
/// An unreadable or foreign file is an ERROR naming the file, never an empty
/// book: reading it as empty would report every member as unknown and invite
/// a rewrite that destroys what the file held.
pub fn load(path: &Path) -> Result<Option<AddressBook>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("the address book {} cannot be read: {e}", path.display())),
    };
    let book: AddressBook = serde_json::from_str(&text).map_err(|e| {
        format!(
            "{} is not a reflow2 hub address book ({e}). Nothing was read from it and nothing \
             will be written over it: move it aside or fix it, then run add_member again.",
            path.display()
        )
    })?;
    if book.format != FORMAT {
        return Err(format!(
            "{} says it is `{}`, not a reflow2 hub address book (`{FORMAT}`). Nothing was read \
             from it and nothing will be written over it.",
            path.display(),
            book.format
        ));
    }
    if book.version > VERSION {
        return Err(format!(
            "{} was written by a newer reflow2 (address book version {}, this one reads up to \
             {VERSION}). Nothing was read from it and nothing will be written over it: use that \
             reflow2, or upgrade this one.",
            path.display(),
            book.version
        ));
    }
    Ok(Some(book))
}

/// A book written to a temporary file beside its place and not yet moved in.
/// [`PendingWrite::commit`] renames it into place; dropping it uncommitted
/// removes the temporary file, so a call that fails after preparing leaves the
/// book exactly as it was.
pub struct PendingWrite {
    tmp: PathBuf,
    target: PathBuf,
    committed: bool,
}

impl PendingWrite {
    /// Move the prepared book into place. A rename within one folder: the book
    /// is either the old one or the new one, never half of each.
    pub fn commit(mut self) -> Result<PathBuf, String> {
        std::fs::rename(&self.tmp, &self.target).map_err(|e| {
            format!(
                "the address book could not be moved into place at {}: {e}",
                self.target.display()
            )
        })?;
        self.committed = true;
        Ok(self.target.clone())
    }
}

impl Drop for PendingWrite {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_file(&self.tmp);
        }
    }
}

/// Write `book` to a temporary file beside `path`, after refusing a place
/// inside a git work tree. Nothing is in place until the returned write is
/// committed.
pub fn prepare(path: &Path, book: &AddressBook) -> Result<PendingWrite, String> {
    let target = absolute(path);
    if let Some(repo) = enclosing_repository(&target) {
        return Err(format!(
            "the address book would be written to {}, inside the git repository at {}. It is a \
             per-machine record of store paths and must never be committed, so it is not written \
             inside a repository. reflow2 keeps it in its settings folder: set \
             REFLOW2_CONFIG_DIR to a folder outside any repository (or unset it, for \
             ~/.config/reflow2). Nothing was written.",
            target.display(),
            repo.display()
        ));
    }
    let dir = target
        .parent()
        .ok_or_else(|| format!("{} has no folder to be written in", target.display()))?;
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("could not create the folder {}: {e}", dir.display()))?;
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        target
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        std::process::id()
    ));
    let text = serde_json::to_string_pretty(book)
        .map_err(|e| format!("the address book could not be serialized: {e}"))?
        + "\n";
    std::fs::write(&tmp, text)
        .map_err(|e| format!("could not write the address book at {}: {e}", tmp.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    Ok(PendingWrite {
        tmp,
        target,
        committed: false,
    })
}

/// What is at an entry's store path NOW, read from the identity file beside
/// the store. No store is opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntryState {
    /// `present` (the store is there and holds this member) or `stale`.
    pub state: &'static str,
    /// For a stale entry: `missing` (nothing is at the path: the store moved or
    /// was deleted), `holds_another_design` (a store is there and it is a
    /// different design), or `unnamed` (a store is there with no readable
    /// identity, so which design it holds cannot be said without opening it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_because: Option<&'static str>,
    /// What a person should do about it, in a sentence. Present only when
    /// stale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Check one entry against what is at its store path now.
pub fn check(entry: &Entry) -> EntryState {
    let found = reflow2_core::describe_at(&entry.store);
    let stale = |because: &'static str, detail: String| EntryState {
        state: "stale",
        stale_because: Some(because),
        detail: Some(detail),
    };
    let redo = format!(
        "If the store moved, record where it is now: add_member with its new store_path and \
         `replace: true`. The entry is kept as it is until then; nothing here removes it."
    );
    match found.state {
        reflow2_core::DesignPathState::Design
            if found.graph_id.as_deref() == Some(entry.graph_id.as_str())
                && Path::new(&entry.store).exists() =>
        {
            EntryState {
                state: "present",
                stale_because: None,
                detail: None,
            }
        }
        reflow2_core::DesignPathState::Design if !Path::new(&entry.store).exists() => stale(
            "missing",
            format!(
                "'{}' was recorded at {}, and only its identity file is left there: the store \
                 itself is gone. {redo}",
                entry.name, entry.store
            ),
        ),
        reflow2_core::DesignPathState::Design => stale(
            "holds_another_design",
            format!(
                "'{}' ({}) was recorded at {}, and the store there now holds a different design \
                 ({}). {redo}",
                entry.name,
                entry.graph_id,
                entry.store,
                found.graph_id.as_deref().unwrap_or("?")
            ),
        ),
        reflow2_core::DesignPathState::Unnamed => stale(
            "unnamed",
            format!(
                "'{}' was recorded at {}, and the store there has no readable identity file, so \
                 which design it holds cannot be said without opening it: {} {redo}",
                entry.name, entry.store, found.reading
            ),
        ),
        _ => stale(
            "missing",
            format!(
                "'{}' was recorded at {}, and nothing is there now: the store moved or was \
                 deleted. {redo}",
                entry.name, entry.store
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_graph_id_becomes_a_file_name_that_cannot_climb_out_of_its_folder() {
        assert_eq!(file_stem("a1b2c3"), "a1b2c3");
        assert_eq!(file_stem("../etc/passwd"), ".._etc_passwd");
        assert_eq!(file_stem(".."), "_..");
        assert_eq!(file_stem(""), "_");
        assert_eq!(file_stem("hub/with:odd chars"), "hub_with_odd_chars");
    }

    #[test]
    fn a_folder_inside_a_work_tree_is_found_by_its_git_entry_file_or_directory() {
        let root = std::env::temp_dir().join(format!("reflow2-ab-repo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("a/b")).unwrap();
        assert_eq!(enclosing_repository(&root.join("a/b/not-yet")), None);
        std::fs::write(root.join("a/.git"), "gitdir: elsewhere\n").unwrap();
        assert_eq!(
            enclosing_repository(&root.join("a/b/not-yet")),
            Some(root.join("a"))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_foreign_or_newer_file_is_refused_by_name_and_never_read_as_empty() {
        let dir = std::env::temp_dir().join(format!("reflow2-ab-load-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("h.json");
        assert_eq!(load(&p).unwrap(), None);
        std::fs::write(&p, "{\"not\":\"a book\"}").unwrap();
        assert!(load(&p).unwrap_err().contains("not a reflow2 hub address book"));
        let mut book = AddressBook::new("h", None);
        book.version = VERSION + 1;
        std::fs::write(&p, serde_json::to_string(&book).unwrap()).unwrap();
        assert!(load(&p).unwrap_err().contains("newer reflow2"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_uncommitted_write_leaves_the_book_as_it_was() {
        let dir = std::env::temp_dir().join(format!("reflow2-ab-pend-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let p = dir.join("hubs/h.json");
        let pending = prepare(&p, &AddressBook::new("h", None)).unwrap();
        drop(pending);
        assert!(!p.exists());
        assert_eq!(std::fs::read_dir(p.parent().unwrap()).unwrap().count(), 0);
        let pending = prepare(&p, &AddressBook::new("h", None)).unwrap();
        pending.commit().unwrap();
        assert_eq!(load(&p).unwrap().unwrap().hub_graph_id, "h");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
