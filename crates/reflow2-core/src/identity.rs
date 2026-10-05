//! A design knows its own name, and remembers it across opens.
//!
//! `req:design-identity`, governed by `dec:identity-out-of-band` — *names are
//! assigned with zero coordination, never derived from shared state.*
//!
//! Until now every reflow2 graph answered to the same hardcoded id, so **no
//! design could tell another design from itself**. `mirror_surface` has to
//! refuse a surface whose source is the importing graph (a filtered copy of
//! your own design would overwrite the full one), and with one constant that
//! check could never pass for anybody. Composition between designs was
//! meaningless: they all had the same name.
//!
//! ## Why the id lives beside the store and not in it
//!
//! **The graph id namespaces every stored key.** Reading anything requires
//! already knowing it, so it cannot be a node inside the design — that is a
//! chicken-and-egg, and getting it wrong is silent: a graph reopened under a
//! name it was not created with finds nothing and presents as an *empty
//! design*. So identity sits in a sibling file, exactly where the version stamp
//! already sits, and is read before the design is.
//!
//! Its own file rather than a field in `<graph>.meta.json`, for the same reason
//! the sync marker got one: `check_and_stamp` rewrites that file wholesale on
//! every open, and changing its shape would make every existing graph fail to
//! open with "the version stamp is not readable".
//!
//! ## The migration is the dangerous part, so it is the explicit part
//!
//! Every graph that exists today holds its design under the old default id.
//! Minting a fresh id for those would be a catastrophe of exactly the silent
//! kind above — the design would still be on disk, and reflow2 would open a new
//! empty one beside it and report nothing wrong. So a graph that **already has
//! design data under the default id adopts that id** as its identity, forever.
//! Only a graph with nothing under it mints.
//!
//! One consequence worth stating: `graph_id` is part of the export's content
//! hash, so adoption is also what keeps every existing export, chain link and
//! committed record valid across this change.

use std::path::{Component, Path, PathBuf};

use crate::foundation::core::DynoError;
use serde::{Deserialize, Serialize};

/// How a design came by its name — recorded because the two cases have very
/// different consequences, and a later reader should not have to guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// Minted for a graph that had no design in it yet.
    Minted,
    /// Kept from the era when every graph shared one id, because this graph
    /// already held a design under it.
    Adopted,
}

/// What this design is called, and what it was called by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignIdentity {
    /// The storage-scoping id. Stable for the life of the design, including
    /// across machines and copies — a copy of a design *is* that design.
    pub graph_id: String,
    /// The human-facing name. A label on top of the id, changeable at will,
    /// and never load-bearing: two designs may share a label and still be
    /// distinct.
    pub label: String,
    /// Minted or adopted.
    pub origin: Origin,
    /// Which reflow2 wrote this record.
    pub minted_by: String,
}

// ---------------------------------------------------------------------------
// Where the identity file is: beside the store's REAL directory.
// ---------------------------------------------------------------------------

/// The store directory as the filesystem means it — every symlink followed,
/// `.` and `..` resolved — however the caller spelled the path.
///
/// **Why the identity file is placed by this and not by the spelling.** Until
/// 0.79.0 it was placed beside the path AS TYPED, so one store got a different
/// identity location for each way of reaching it: opened through a symlink,
/// the file went beside the LINK; opened as `--graph-path .` from inside the
/// store, it went INSIDE the store as `..id.json`. Either way the next open by
/// the store's real path found nothing and was refused as "lost its identity
/// file" (fact:root-cause-a-stores-identity-file-is-placed-by-how-its-path-was-spelled-2026-10-05,
/// measured on 0.76.0, 0.78.0 and main).
///
/// A store that does not exist yet — every first open — cannot be
/// canonicalized, so the deepest part of the path that exists is, and the rest
/// is appended with `.` dropped and `..` taken lexically. Following a symlink
/// moves the file only when the LAST component is the link (or the spelling is
/// `.`): a symlink higher up the path leaves the store's parent directory, and
/// so the file, physically where it was.
pub fn store_real_path(graph_path: &str) -> PathBuf {
    let p = Path::new(graph_path);
    if let Ok(real) = std::fs::canonicalize(p) {
        return real;
    }
    let absolute = if p.is_absolute() {
        p.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(p),
            Err(_) => return p.to_path_buf(),
        }
    };
    let mut out = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(name) => {
                out.push(name);
                if let Ok(real) = std::fs::canonicalize(&out) {
                    out = real;
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// `<store>.<suffix>`, a sibling of `store`.
fn beside(store: &Path, suffix: &str) -> PathBuf {
    match store.file_name() {
        Some(name) => {
            let mut sibling = name.to_os_string();
            sibling.push(".");
            sibling.push(suffix);
            store.with_file_name(sibling)
        }
        None => PathBuf::from(format!("{}.{suffix}", store.display())),
    }
}

/// `<real store>.id.json` — where this design's identity file belongs, and
/// where every identity is written: a sibling of the store's REAL directory
/// ([`store_real_path`]), like the version stamp. For a store reached by its
/// plain path this is exactly the file every earlier version used.
///
/// Reading goes through [`locate`], which also looks where an older reflow2
/// may have put the file.
pub fn identity_path(graph_path: &str) -> PathBuf {
    beside(&store_real_path(graph_path), "id.json")
}

/// Where reflow2 0.79.0 and earlier put the identity for THIS spelling of the
/// path: beside the path as typed, unresolved. Kept verbatim, because finding
/// what an older version wrote is the whole use.
fn identity_path_as_typed(graph_path: &str) -> PathBuf {
    let p = Path::new(graph_path);
    match p.file_name().map(|n| n.to_string_lossy().to_string()) {
        Some(n) => p.with_file_name(format!("{n}.id.json")),
        None => PathBuf::from(format!("{graph_path}.id.json")),
    }
}

/// One file, however it is spelled: its directory resolved, its name kept.
fn physical(file: &Path) -> PathBuf {
    let name = file.file_name().map(|n| n.to_os_string());
    let parent = file.parent().filter(|p| !p.as_os_str().is_empty());
    let dir = match parent {
        Some(dir) => store_real_path(&dir.to_string_lossy()),
        None => store_real_path("."),
    };
    match name {
        Some(n) => dir.join(n),
        None => dir,
    }
}

/// Where an identity file was found, or looked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    /// Beside the store's real directory: where every identity is written.
    BesideTheStore,
    /// Beside the path as it was typed, when that is not the store's real
    /// path — a symlink to the store. Where reflow2 0.79.0 and earlier put it.
    BesideTheTypedPath,
    /// Inside the store directory: where reflow2 0.79.0 and earlier put it for
    /// `--graph-path .` (`..id.json`) or `--graph-path ./` (`.id.json`).
    InsideTheStore,
}

impl Placement {
    fn explained(self) -> &'static str {
        match self {
            Placement::BesideTheStore => "beside the store, where reflow2 keeps it",
            Placement::BesideTheTypedPath => {
                "beside the path as typed, where reflow2 0.79.0 and earlier put it when a store \
                 was opened through a symlink"
            }
            Placement::InsideTheStore => {
                "inside the store directory, where reflow2 0.79.0 and earlier put it when a store \
                 was opened as `--graph-path .` (`..id.json`) or `./` (`.id.json`)"
            }
        }
    }
}

/// What one candidate location held.
#[derive(Debug, Clone)]
pub enum Reading {
    Absent,
    Identity(DesignIdentity),
    /// There, and not an identity reflow2 can read — never treated as absent.
    Unreadable(String),
}

/// One place an identity file may be.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub path: PathBuf,
    pub placement: Placement,
    /// The file a reflow2 0.79.0 or earlier read for this exact spelling.
    pub this_spelling: bool,
    pub reading: Reading,
}

/// Every place a store's identity file may be, for one spelling of its path,
/// in the order they are consulted, each with what it held.
#[derive(Debug, Clone)]
pub struct IdentityLookup {
    /// The path as the caller typed it.
    pub graph_path: String,
    /// The store's real directory.
    pub real_store: PathBuf,
    /// Beside the store first, then the older placements; each file once.
    pub candidates: Vec<Candidate>,
}

/// Something an open did or found about the identity file that a person
/// should be told: said on stderr by the door that opened the store, and in
/// `loop_status` for the session.
#[derive(Debug, Clone, Serialize)]
pub struct IdentityNote {
    /// What happened, in sentences.
    pub summary: String,
    /// The file the identity was read from.
    pub read_from: PathBuf,
    /// Where this open wrote a copy beside the store, when it did.
    pub copied_to: Option<PathBuf>,
    /// Other identity files for this store naming a DIFFERENT design.
    pub disagreeing: Vec<(PathBuf, String)>,
    /// True when a person has something to decide; `loop_status` then puts it
    /// in `next`.
    pub needs_attention: bool,
}

/// The identity an open will use, and where it came from.
#[derive(Debug, Clone)]
pub struct FoundIdentity {
    pub identity: DesignIdentity,
    pub file: PathBuf,
    pub placement: Placement,
    /// Found only where an older reflow2 put it, and nothing disagrees: the
    /// open writes a copy beside the store once it has succeeded.
    pub copy_beside_store: bool,
    pub note: Option<IdentityNote>,
}

/// What [`IdentityLookup::decide`] concluded.
#[derive(Debug)]
pub enum Located {
    Found(FoundIdentity),
    /// No identity file in any place looked.
    Absent,
    /// A file is there and cannot be used. Nothing has been written.
    Refused(DynoError),
}

fn read_candidate(path: &Path) -> Reading {
    match std::fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<DesignIdentity>(&text) {
            Ok(identity) => Reading::Identity(identity),
            Err(e) => Reading::Unreadable(e.to_string()),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Reading::Absent,
        Err(e) => Reading::Unreadable(e.to_string()),
    }
}

/// Look for a store's identity file everywhere it may be, writing nothing.
///
/// In order: beside the store's real directory; beside the path as typed (a
/// symlink); inside the store (`..id.json` for `.`, `.id.json` for `./`). The
/// inside places are looked at whatever the spelling, so a store first opened
/// as `.` opens by its real path too. Beside a symlink can only be looked at
/// when the store is reached THROUGH that symlink — nothing on the real path
/// says a link to it exists — which is why a refusal says where to look.
pub fn locate(graph_path: &str) -> IdentityLookup {
    let real_store = store_real_path(graph_path);
    let typed = identity_path_as_typed(graph_path);
    let typed_physical = physical(&typed);
    let typed_placement = if typed_physical.parent() == Some(real_store.as_path()) {
        Placement::InsideTheStore
    } else {
        Placement::BesideTheTypedPath
    };
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut candidates = Vec::new();
    for (path, placement) in [
        (beside(&real_store, "id.json"), Placement::BesideTheStore),
        (typed, typed_placement),
        (real_store.join("..id.json"), Placement::InsideTheStore),
        (real_store.join(".id.json"), Placement::InsideTheStore),
    ] {
        let key = physical(&path);
        if seen.contains(&key) {
            continue;
        }
        seen.push(key.clone());
        let reading = read_candidate(&path);
        candidates.push(Candidate {
            this_spelling: key == typed_physical,
            path: if path.is_absolute() { path } else { key },
            placement,
            reading,
        });
    }
    IdentityLookup {
        graph_path: graph_path.to_string(),
        real_store,
        candidates,
    }
}

impl IdentityLookup {
    /// The file beside the store's real directory, where an identity is written.
    pub fn beside_the_store(&self) -> &Path {
        &self.candidates[0].path
    }

    /// Is any identity file there at all, readable or not?
    pub fn any_present(&self) -> bool {
        self.candidates
            .iter()
            .any(|c| !matches!(c.reading, Reading::Absent))
    }

    /// Which identity an open of this spelling uses, without writing anything.
    ///
    /// **The rule.** The file beside the store's real directory comes first,
    /// always — one store, one identity, however it is reached. Only when it is
    /// absent are the places an older reflow2 used consulted: this spelling's
    /// own first (the file a 0.79.0 or earlier open of this exact path read),
    /// then the inside-the-store files. An identity found only there is used,
    /// and the open writes a COPY beside the store once it has succeeded, so
    /// the next open by the real path finds it. The old file is never moved or
    /// deleted: an older reflow2 opening by the same spelling still needs it.
    ///
    /// **Every store that opened before still opens.** A spelling that read a
    /// file before reads the same identity now, with one exception, made loud:
    /// when the file beside the store and an older file name DIFFERENT designs,
    /// the one beside the store wins and the other is reported. That takes two
    /// identity files for one store, which only a reflow2 older than the
    /// lost-identity refusal (2026-08-07) or a hand edit could leave.
    ///
    /// A file that is there and cannot be read is refused rather than skipped,
    /// as it always was for the spelling's own file: it may be the only record
    /// of the design's name.
    pub fn decide(&self) -> Located {
        let store = &self.candidates[0];
        let older = &self.candidates[1..];
        match &store.reading {
            Reading::Identity(identity) => {
                let disagreeing = disagreeing_with(identity, older);
                let note = (!disagreeing.is_empty()).then(|| IdentityNote {
                    summary: format!(
                        "{} The file beside the store comes first, so this open uses design `{}` \
                         from {}. Nothing was changed. Ask whoever owns this store which design it \
                         is, compare each id with the design's export (`graph_id` in its \
                         design.json or single-file export), and move the wrong file aside.",
                        two_names_sentence(&self.real_store, &store.path, identity, &disagreeing),
                        identity.graph_id,
                        store.path.display()
                    ),
                    read_from: store.path.clone(),
                    copied_to: None,
                    disagreeing,
                    needs_attention: true,
                });
                Located::Found(FoundIdentity {
                    identity: identity.clone(),
                    file: store.path.clone(),
                    placement: Placement::BesideTheStore,
                    copy_beside_store: false,
                    note,
                })
            }
            Reading::Unreadable(why) => Located::Refused(unreadable(&store.path, why)),
            Reading::Absent => self.decide_among_older(older),
        }
    }

    fn decide_among_older(&self, older: &[Candidate]) -> Located {
        if let Some(own) = older.iter().find(|c| c.this_spelling)
            && let Reading::Unreadable(why) = &own.reading
        {
            return Located::Refused(unreadable(&own.path, why));
        }
        let readable: Vec<(&Candidate, &DesignIdentity)> = older
            .iter()
            .filter_map(|c| match &c.reading {
                Reading::Identity(i) => Some((c, i)),
                _ => None,
            })
            .collect();
        if readable.is_empty() {
            return match older
                .iter()
                .find(|c| matches!(c.reading, Reading::Unreadable(_)))
            {
                Some(c) => match &c.reading {
                    Reading::Unreadable(why) => Located::Refused(unreadable(&c.path, why)),
                    _ => Located::Absent,
                },
                None => Located::Absent,
            };
        }
        let all_agree = readable
            .iter()
            .all(|(_, i)| i.graph_id == readable[0].1.graph_id);
        let chosen = readable
            .iter()
            .find(|(c, _)| c.this_spelling)
            .or_else(|| all_agree.then(|| &readable[0]));
        let Some((chosen, identity)) = chosen else {
            return Located::Refused(DynoError::Storage(format!(
                "the design at {} has no identity file beside the store, and the files an older \
                 reflow2 left name DIFFERENT designs: {}. reflow2 will not guess which design \
                 this store is. Compare each id with the design's export (`graph_id` in its \
                 design.json or single-file export) and put the right file beside the store, at \
                 {}. Nothing was written.",
                self.graph_path,
                readable
                    .iter()
                    .map(|(c, i)| format!("{} names `{}`", c.path.display(), i.graph_id))
                    .collect::<Vec<_>>()
                    .join(", "),
                self.beside_the_store().display()
            )));
        };
        let others: Vec<Candidate> = older
            .iter()
            .filter(|c| c.path != chosen.path)
            .cloned()
            .collect();
        let disagreeing = disagreeing_with(identity, &others);
        let copy = disagreeing.is_empty();
        let summary = if copy {
            format!(
                "The identity file of the design at {} (`{}`) was found only at {} — {}. A copy \
                 was written beside the store, at {}, so the store now opens by its real path \
                 too, however it is reached. The old file was left where it is, because an older \
                 reflow2 opening by the same path still reads it; nothing needs doing.",
                self.graph_path,
                identity.graph_id,
                chosen.path.display(),
                chosen.placement.explained(),
                self.beside_the_store().display()
            )
        } else {
            format!(
                "{} This open uses design `{}` from {}, the file this path has always opened, \
                 and wrote NO copy beside the store, because which design the store is is not \
                 settled. Ask whoever owns this store, compare each id with the design's export, \
                 and put the right file beside the store, at {}.",
                two_names_sentence(&self.real_store, &chosen.path, identity, &disagreeing),
                identity.graph_id,
                chosen.path.display(),
                self.beside_the_store().display()
            )
        };
        Located::Found(FoundIdentity {
            identity: (*identity).clone(),
            file: chosen.path.clone(),
            placement: chosen.placement,
            copy_beside_store: copy,
            note: Some(IdentityNote {
                summary,
                read_from: chosen.path.clone(),
                copied_to: copy.then(|| self.beside_the_store().to_path_buf()),
                disagreeing,
                needs_attention: !copy,
            }),
        })
    }

    /// The identity this spelling would open with, read without writing
    /// anything — for a describe, a registry sweep, a version guard.
    /// `Some(Ok)`, `Some(Err)` for a file that cannot be used, `None` when
    /// there is none.
    pub fn read(&self) -> Option<Result<DesignIdentity, String>> {
        match self.decide() {
            Located::Found(found) => Some(Ok(found.identity)),
            Located::Absent => None,
            Located::Refused(e) => Some(Err(e.to_string())),
        }
    }

    /// The file this spelling reads its identity from, if any.
    pub fn file_in_use(&self) -> Option<PathBuf> {
        match self.decide() {
            Located::Found(found) => Some(found.file),
            _ => None,
        }
    }
}

fn disagreeing_with(identity: &DesignIdentity, others: &[Candidate]) -> Vec<(PathBuf, String)> {
    others
        .iter()
        .filter_map(|c| match &c.reading {
            Reading::Identity(o) if o.graph_id != identity.graph_id => {
                Some((c.path.clone(), o.graph_id.clone()))
            }
            _ => None,
        })
        .collect()
}

fn two_names_sentence(
    real_store: &Path,
    used: &Path,
    identity: &DesignIdentity,
    disagreeing: &[(PathBuf, String)],
) -> String {
    format!(
        "The store at {} has more than one identity file, and they name different designs: {} \
         names `{}`, and {}. A store with two names may hold two designs.",
        real_store.display(),
        used.display(),
        identity.graph_id,
        disagreeing
            .iter()
            .map(|(p, id)| format!("{} names `{id}`", p.display()))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn unreadable(path: &Path, why: &str) -> DynoError {
    DynoError::Serialization(format!(
        "the design identity at {} is not readable ({why}). It records which design this store \
         holds, and reflow2 will not guess: fix the file, or move it aside to have a new identity \
         established. Nothing was written.",
        path.display()
    ))
}

/// The design id recorded for a graph, or `None` when there is no usable
/// identity file in any place [`locate`] looks.
///
/// Deliberately quiet: this is used by the version guard to decide WHICH id to
/// count retired-type instances under, and a missing or unparseable sidecar
/// must not be an error there — it simply means the count falls back to the
/// default id, which is the only id a pre-identity graph could be under.
pub fn read_graph_id(graph_path: &str) -> Option<String> {
    match locate(graph_path).decide() {
        Located::Found(found) => Some(found.identity.graph_id),
        _ => None,
    }
}

/// A name assigned with zero coordination — nothing shared is read, so nothing
/// can race, at one seat or a thousand (`dec:identity-out-of-band`).
///
/// Deliberately not a UUID crate: the inputs already make it unique by
/// construction — the nanosecond it was created, the process that created it,
/// and where it lives — and a dependency added for sixteen hex characters is a
/// dependency every consumer pays a rebuild for.
fn mint(graph_path: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let absolute = std::fs::canonicalize(graph_path)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| graph_path.to_string());
    let seed = format!("{nanos}|{}|{absolute}", std::process::id());
    format!("{:016x}", crate::nodes::fnv1a(&seed))
}

/// A friendly default: the project directory's name, not the store's.
///
/// `<project>/.reflow2/graph` should read as "project", which is what a person
/// would call it — the two path segments below it are reflow2's plumbing.
fn default_label(graph_path: &str) -> String {
    let p = std::fs::canonicalize(graph_path).unwrap_or_else(|_| PathBuf::from(graph_path));
    let mut cursor = p.as_path();
    while let Some(name) = cursor.file_name().and_then(|n| n.to_str()) {
        if name != "graph" && name != ".reflow2" {
            return name.to_string();
        }
        match cursor.parent() {
            Some(parent) => cursor = parent,
            None => break,
        }
    }
    "design".to_string()
}

/// A new identity for a store that has none, WITHOUT writing it: adopted
/// when the store already holds the pre-identity design under the shared id
/// (`holds_default_design`), minted otherwise. The caller writes it, with
/// [`write`], once the open that needs it has succeeded.
pub fn establish(graph_path: &str, default_id: &str, holds_default_design: bool) -> DesignIdentity {
    if holds_default_design {
        DesignIdentity {
            graph_id: default_id.to_string(),
            label: default_label(graph_path),
            origin: Origin::Adopted,
            minted_by: env!("CARGO_PKG_VERSION").to_string(),
        }
    } else {
        DesignIdentity {
            graph_id: mint(graph_path),
            label: default_label(graph_path),
            origin: Origin::Minted,
            minted_by: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Read this design's identity, establishing it on first open.
///
/// `holds_default_design` is asked only when there is no identity file in any
/// place [`locate`] looks, and answers the migration question: does this store
/// already contain a design under the old shared id? If it does, that id is
/// adopted rather than replaced — see the module docs for why the alternative
/// is silent data loss. An identity found only where an older reflow2 put it is
/// copied beside the store, as an open does.
pub fn resolve(
    graph_path: &str,
    default_id: &str,
    holds_default_design: impl FnOnce() -> bool,
) -> Result<DesignIdentity, DynoError> {
    match locate(graph_path).decide() {
        Located::Found(found) => {
            if found.copy_beside_store {
                copy_beside_store(graph_path, &found.file)?;
            }
            Ok(found.identity)
        }
        Located::Refused(e) => Err(e),
        Located::Absent => {
            let identity = establish(graph_path, default_id, holds_default_design());
            write(graph_path, &identity)?;
            Ok(identity)
        }
    }
}

/// Has a RocksDB store ever held data at this path?
///
/// Asked BEFORE the store is opened, because opening creates the directory and
/// erases the distinction. Deliberately "has ever held data" rather than "exists":
/// a store that was created and never written carries `CURRENT`, `MANIFEST-*` and
/// `OPTIONS-*` from the open alone, and treating those as content would refuse a
/// path somebody merely touched. Data lands either in an SST or, before it is
/// flushed, in a non-empty write-ahead log.
///
/// A read failure answers `false`. This feeds a REFUSAL, and a directory we
/// cannot list is not evidence that a design is in it — the conservative answer
/// for a guard is the one that does not invent a reason to refuse.
pub fn store_has_content(graph_path: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(graph_path) else {
        return false;
    };
    entries.flatten().any(|e| {
        let name = e.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".sst") {
            return true;
        }
        // The WAL: data written and not yet flushed is here and nowhere else.
        name.ends_with(".log") && e.metadata().map(|m| m.len() > 0).unwrap_or(false)
    })
}

/// What a store with data and no identity file was found to hold, read
/// without opening it for writing: every design id its node keys carry, with
/// how many nodes each.
#[derive(Debug, Clone, Default)]
pub struct StoreReading {
    pub ids: std::collections::BTreeMap<String, usize>,
}

/// The refusal for a store that holds data, has no identity file in any place
/// [`locate`] looks, and is not the pre-identity design.
///
/// **The case.** The identity sidecar sits BESIDE the store, so the two can be
/// parted — a partial restore, a sync tool that skips dotfiles, a container
/// volume mounted at the store instead of its parent, or (measured, and fixed
/// for new files by [`identity_path`]) a store first opened through a symlink,
/// whose file an older reflow2 put beside the LINK. Opening anyway would mint a
/// new name, write it over the missing one, and present the design as empty
/// while it is still on disk (fact:defect-lost-identity-sidecar-opens-empty).
///
/// **What it says, because the person reading it has to act on it:** every
/// place it looked; the design id(s) the store's own keys carry, read without
/// writing (`reading`), which is the id the file has to name; the file to put
/// back and where; and that nothing was written — an open that refuses leaves
/// the store and every file beside it as it found them. When the store could
/// not be read without opening it for writing (`reading` is `Err`), it says
/// that instead, because then the open did touch the store's own files.
pub fn lost_identity(lookup: &IdentityLookup, reading: Result<&StoreReading, String>) -> DynoError {
    let looked = lookup
        .candidates
        .iter()
        .map(|c| format!("  - {} ({})", c.path.display(), c.placement.explained()))
        .collect::<Vec<_>>()
        .join("\n");
    let target = lookup.beside_the_store().display().to_string();
    let label = default_label(&lookup.real_store.to_string_lossy());
    let holds = match &reading {
        Ok(r) if r.ids.len() == 1 => {
            let (id, n) = r.ids.iter().next().expect("one id");
            format!(
                "WHAT THE STORE HOLDS: {n} node(s), all under the design id `{id}`. That is the id \
                 its identity file has to name. If you have no copy of the file, writing this one \
                 at {target} recovers it (`label` is only a name, and `origin` only a note):\n  \
                 {{\"graph_id\": \"{id}\", \"label\": \"{label}\", \"origin\": \"minted\", \
                 \"minted_by\": \"{}\"}}\n\
                 To check before trusting it: open read-only (`reflow2 read loop_status`) and \
                 compare_designs against the design's export, if one exists.",
                env!("CARGO_PKG_VERSION")
            )
        }
        Ok(r) if r.ids.is_empty() => "WHAT THE STORE HOLDS: no node under any design id that can \
                                      be read — its data may be edges or index entries only. \
                                      Restore the identity file from a backup."
            .to_string(),
        Ok(r) => format!(
            "WHAT THE STORE HOLDS: nodes under {} design ids — {}. Only one is the design the \
             missing file named; compare each with the design's export (`graph_id` in its \
             design.json or single-file export) before writing an identity file at {target}.",
            r.ids.len(),
            r.ids
                .iter()
                .map(|(id, n)| format!("`{id}` ({n} node(s))"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Err(why) => format!(
            "WHAT THE STORE HOLDS could not be read without opening it for writing ({why}), so \
             this open DID touch the store's own files, though it wrote nothing beside it."
        ),
    };
    let nothing_written = if reading.is_ok() {
        "\nNothing was written: this refused open left the store and every file beside it as it \
         found them."
    } else {
        ""
    };
    DynoError::Storage(format!(
        "the design at {} has lost its identity file, and reflow2 will not guess.\n\
         It looked in these places and found none:\n{looked}\n\
         This store already holds data, but not under the shared id every pre-identity design \
         used — so it belongs to a design whose name lived only in that file. Opening anyway \
         would mint a NEW name, write it over the missing one, and present the design as empty \
         while it is still on disk and no longer reachable.\n\
         TO RECOVER: put the design's id file beside the store, at {target}. The identity file \
         is a SIBLING of the store, not inside it. If an older reflow2 ever opened this store \
         through a symlink, it put the file beside that LINK, named `<link-name>.id.json` — find \
         it (`find ~ -name '*.id.json'`) and copy it here. If this is a container, the usual \
         cause is a volume mounted at the store directory instead of its parent — mount the \
         parent. Otherwise restore it from a backup.\n\
         {holds}{nothing_written}",
        lookup.graph_path
    ))
}

/// Write `bytes` to `path` so a reader never sees half of it: a temporary file
/// in the same directory, then a rename over the target. A plain write
/// truncates first, and a process killed between the truncate and the write
/// left an EMPTY identity file — refused as unreadable on the next open.
fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), DynoError> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut tmp_name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    tmp_name.push(format!(".tmp{}", std::process::id()));
    let tmp = path.with_file_name(tmp_name);
    let fail = |e: std::io::Error| {
        DynoError::Storage(format!(
            "cannot write the design identity at {}: {e}",
            path.display()
        ))
    };
    std::fs::write(&tmp, bytes).map_err(fail)?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        fail(e)
    })
}

/// Persist an identity: to the file this spelling reads it from, or — for a
/// store that has none yet — beside the store's REAL directory
/// ([`identity_path`]), never beside a symlink or inside the store.
///
/// Writing back to the file in use is what keeps a rename or an adoption from
/// creating a second, disagreeing file.
pub fn write(graph_path: &str, identity: &DesignIdentity) -> Result<(), DynoError> {
    // A file that is there and cannot be read is never written over: it may be
    // the only record of the design's name.
    let path = match locate(graph_path).decide() {
        Located::Found(found) => found.file,
        Located::Absent => identity_path(graph_path),
        Located::Refused(e) => return Err(e),
    };
    let json = serde_json::to_string_pretty(identity).map_err(|e| {
        DynoError::Serialization(format!("cannot serialize the design identity: {e}"))
    })?;
    write_atomically(&path, (json + "\n").as_bytes())
}

/// Copy an identity file found where an older reflow2 put it to beside the
/// store, byte for byte, leaving the original where it is. A file already
/// beside the store is left as it is: that one is read first, so finding it
/// means there is nothing to copy.
pub fn copy_beside_store(graph_path: &str, from: &Path) -> Result<PathBuf, DynoError> {
    let to = identity_path(graph_path);
    if to.exists() {
        return Ok(to);
    }
    let bytes = std::fs::read(from).map_err(|e| {
        DynoError::Storage(format!(
            "cannot read the design identity at {} to copy it beside the store: {e}",
            from.display()
        ))
    })?;
    write_atomically(&to, &bytes)?;
    Ok(to)
}

/// Rename the design. The label is a label: the id never moves, because
/// everything stored is keyed by it and every export ever written names it.
pub fn set_label(graph_path: &str, label: &str) -> Result<DesignIdentity, DynoError> {
    let mut identity = match locate(graph_path).decide() {
        Located::Found(found) => found.identity,
        Located::Refused(e) => return Err(e),
        Located::Absent => {
            return Err(DynoError::Storage(format!(
                "no design identity at {} to rename — open the graph once to establish it.",
                identity_path(graph_path).display()
            )));
        }
    };
    identity.label = label.to_string();
    write(graph_path, &identity)?;
    Ok(identity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sidecar_sits_beside_the_store() {
        assert_eq!(
            identity_path("/p/.reflow2/graph"),
            PathBuf::from("/p/.reflow2/graph.id.json")
        );
    }

    #[test]
    fn two_designs_minted_at_once_do_not_collide() {
        // Unique by construction: same nanosecond is possible, same path is not.
        let a = mint("/tmp/one");
        let b = mint("/tmp/two");
        assert_ne!(a, b);
        assert_eq!(a.len(), 16, "a readable, fixed-width id: {a}");
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn a_path_that_does_not_exist_yet_resolves_lexically_past_what_does() {
        let dir = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(dir.path()).unwrap();
        let spelled = format!("{}/proj/./.reflow2/x/../graph", root.display());
        assert_eq!(
            store_real_path(&spelled),
            root.join("proj/.reflow2/graph"),
            "`.` dropped and `..` taken, for a store a first open has not created"
        );
        assert_eq!(
            identity_path(&spelled),
            root.join("proj/.reflow2/graph.id.json")
        );
    }

    #[test]
    fn a_symlink_to_the_store_resolves_to_the_store_and_its_old_place_is_still_looked_at() {
        let dir = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(dir.path()).unwrap();
        let store = root.join("proj/.reflow2/graph");
        std::fs::create_dir_all(&store).unwrap();
        std::fs::create_dir_all(root.join("stores")).unwrap();
        std::os::unix::fs::symlink(&store, root.join("stores/bq")).unwrap();
        let link = root.join("stores/bq");
        let link = link.to_str().unwrap();

        assert_eq!(store_real_path(link), store);
        assert_eq!(
            identity_path(link),
            root.join("proj/.reflow2/graph.id.json")
        );

        let lookup = locate(link);
        let looked: Vec<(PathBuf, Placement, bool)> = lookup
            .candidates
            .iter()
            .map(|c| (c.path.clone(), c.placement, c.this_spelling))
            .collect();
        assert_eq!(
            looked,
            vec![
                (
                    root.join("proj/.reflow2/graph.id.json"),
                    Placement::BesideTheStore,
                    false
                ),
                (
                    root.join("stores/bq.id.json"),
                    Placement::BesideTheTypedPath,
                    true
                ),
                (store.join("..id.json"), Placement::InsideTheStore, false),
                (store.join(".id.json"), Placement::InsideTheStore, false),
            ],
            "beside the store first, then where 0.79.0 and earlier put it"
        );
        assert!(matches!(lookup.decide(), Located::Absent));
    }

    #[test]
    fn a_plain_path_looks_beside_the_store_and_inside_it_and_nowhere_else() {
        let dir = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(dir.path()).unwrap();
        let store = root.join("graph");
        std::fs::create_dir_all(&store).unwrap();
        let lookup = locate(store.to_str().unwrap());
        assert_eq!(lookup.candidates.len(), 3, "{:?}", lookup.candidates);
        assert!(lookup.candidates[0].this_spelling);
        assert_eq!(lookup.candidates[0].path, root.join("graph.id.json"));
    }

    fn identity_named(graph_id: &str) -> String {
        serde_json::to_string(&DesignIdentity {
            graph_id: graph_id.to_string(),
            label: "proj".to_string(),
            origin: Origin::Minted,
            minted_by: "0.76.0".to_string(),
        })
        .unwrap()
    }

    #[test]
    fn an_identity_found_only_inside_the_store_is_used_and_copied_beside_it() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("graph");
        std::fs::create_dir_all(&store).unwrap();
        std::fs::write(store.join("..id.json"), identity_named("aaaaaaaaaaaaaaaa")).unwrap();
        let path = store.to_str().unwrap();

        let Located::Found(found) = locate(path).decide() else {
            panic!("an identity inside the store is found from the real path");
        };
        assert_eq!(found.identity.graph_id, "aaaaaaaaaaaaaaaa");
        assert_eq!(found.placement, Placement::InsideTheStore);
        assert!(found.copy_beside_store);
        let note = found.note.expect("said");
        assert!(!note.needs_attention, "nothing to decide: {}", note.summary);
        assert!(
            !dir.path().join("graph.id.json").exists(),
            "deciding writes nothing"
        );

        // `resolve` makes the copy an open makes, and leaves the original.
        assert_eq!(
            resolve(path, "unused", || unreachable!()).unwrap().graph_id,
            "aaaaaaaaaaaaaaaa"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("graph.id.json")).unwrap(),
            identity_named("aaaaaaaaaaaaaaaa")
        );
        assert!(store.join("..id.json").exists());
    }

    #[test]
    fn older_files_that_disagree_with_no_file_of_this_spellings_own_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("graph");
        std::fs::create_dir_all(&store).unwrap();
        std::fs::write(store.join("..id.json"), identity_named("aaaaaaaaaaaaaaaa")).unwrap();
        std::fs::write(store.join(".id.json"), identity_named("bbbbbbbbbbbbbbbb")).unwrap();
        match locate(store.to_str().unwrap()).decide() {
            Located::Refused(e) => {
                let m = e.to_string();
                assert!(
                    m.contains("aaaaaaaaaaaaaaaa") && m.contains("bbbbbbbbbbbbbbbb"),
                    "{m}"
                );
                assert!(m.contains("will not guess"), "{m}");
            }
            other => {
                panic!("two older files naming two designs must not be guessed between: {other:?}")
            }
        }
    }

    #[test]
    fn an_unreadable_identity_beside_the_store_is_refused_and_never_written_over() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("graph");
        std::fs::create_dir_all(&store).unwrap();
        std::fs::write(dir.path().join("graph.id.json"), "{ not json").unwrap();
        let path = store.to_str().unwrap();
        assert!(matches!(locate(path).decide(), Located::Refused(_)));
        let identity = establish(path, "unused", false);
        assert!(write(path, &identity).is_err());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("graph.id.json")).unwrap(),
            "{ not json"
        );
    }
}

// ---------------------------------------------------------------------------
// Seat identity — who is *working*, as opposed to what is being worked on.
// ---------------------------------------------------------------------------

/// This session's name, minted once per process.
///
/// `req:claims-have-owners`. A claim that does not say who made it cannot be
/// told from a claim nobody is working any more, and a ghost claim makes the
/// overlap report lie — which is worse than no report, because people act on it.
///
/// Same doctrine as the design's own name (`dec:identity-out-of-band`): nothing
/// shared is read, so nothing can race at one seat or fifteen. The shape is
/// `<machine>:<pid>:<mint>`, and it is chosen to make **liveness computable**
/// rather than asserted — a later reader can ask the operating system whether
/// that process still exists instead of trusting a flag somebody set.
pub fn seat_id() -> String {
    static SEAT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SEAT.get_or_init(mint_seat).clone()
}

/// A fresh seat, not the process-wide one.
///
/// `req:seat-per-client`. One server can hold many client sessions
/// (`req:sessions-share-a-graph`), and the process-wide seat is exactly wrong
/// there: every client would report the same owner, so every claim would name
/// the same seat and the overlap report would tell six sessions they are each
/// other. A session mints its own on connect.
///
/// **Honest limit, because it is easy to misread.** The seat carries a pid, so
/// liveness answers "is the process that made this claim still running". Under
/// one server that is the right answer about *the server*, and only a proxy for
/// the session: a client that disconnects while the server lives still reads
/// `live`. Per-session liveness needs the server's own session registry, which
/// the core cannot see — recorded rather than papered over.
pub fn mint_seat() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    // The address of a stack local disambiguates two mints in the same
    // nanosecond within one process — which is exactly the case a shared server
    // creates when two sessions connect at once.
    let here = &nanos as *const u128 as usize;
    let mint = format!(
        "{:08x}",
        crate::nodes::fnv1a(&format!("{nanos}|{here:x}")) & 0xffff_ffff
    );
    format!("{}:{}:{mint}", machine(), std::process::id())
}

/// This machine's name, for telling "their session died" from "their session is
/// on a different computer, and I cannot see it from here".
///
/// Public so tests can build a seat this machine will recognise, rather than
/// reimplementing the lookup and disagreeing with it somewhere subtle.
///
/// Best effort by design, and honest when it fails: an unknown machine makes
/// every foreign claim report as `Unknown` rather than as alive or dead, which
/// is the only truthful answer available.
pub fn machine() -> String {
    // Each source is trimmed and emptiness-checked BEFORE falling through, so
    // an empty HOSTNAME (set but blank, which happens in stripped environments)
    // still reaches /etc/hostname instead of short-circuiting to unknown.
    let non_empty = |s: String| Some(s.trim().to_string()).filter(|h| !h.is_empty());
    std::env::var("HOSTNAME")
        .ok()
        .and_then(non_empty)
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .and_then(non_empty)
        })
        .unwrap_or_else(|| "unknown-machine".to_string())
}

/// Is the session that made a claim still running?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Liveness {
    /// The process is still there. Somebody is probably working this.
    Live,
    /// The session that made this claim has exited. The claim is a ghost —
    /// still worth reading for what it says, never worth treating as held.
    Gone,
    /// Made on another machine, by a seat with no name, or on a machine that
    /// could not identify itself. Reported as unknown rather than guessed:
    /// calling a foreign claim dead would invite somebody to take work that is
    /// actively being done.
    Unknown,
}

/// The seats this process is currently serving, when it is serving more than
/// itself.
///
/// # Why a registry exists at all, when the whole design says "compute, never remember"
///
/// A seat carries a pid, and asking the OS whether that pid is alive is a real
/// computation that cannot go stale. That was the right and complete answer
/// under stdio, where one process WAS one session. Under `--shared` it silently
/// stopped being an answer at all: every seat is minted BY THE DAEMON, so every
/// seat carries the daemon's pid, and the probe asks "is the server running?" —
/// to which the answer is always yes, because nothing else could have replied.
///
/// Measured in dev_storyflow 2026-08-07 (w-613d836b): `mint_seat` returned a
/// seat whose pid was the `--serve-shared` process, while their session's own
/// shell was a different pid entirely. `Gone` had become unreachable, so a
/// worker that died hours ago still read as holding its region and a peer would
/// defer to nobody. `cap:claim-liveness` was status `verified` throughout.
///
/// So the pid answers a question about the SERVER, and only the server knows
/// which SESSIONS it is holding. That knowledge cannot be computed from the
/// graph or from `/proc` — it exists solely in the process, which is why this is
/// the one thing here that is remembered rather than derived. It is kept honest
/// three ways: it lives only in the serving process (nothing is persisted, so
/// nothing survives to go stale), a session's entry is removed when its handler
/// is dropped, and **a process that never registers keeps the old behaviour
/// exactly** — `seat_liveness` consults this only when it has been populated, so
/// stdio is byte-for-byte unchanged.
/// # Why TWO sets and not one
///
/// The obvious shape — one set of currently-attached seats, consulted whenever
/// the seat's pid is ours — is wrong, and the tests caught it: it makes the
/// registry authoritative over seats it has never heard of. One `attach()`
/// anywhere in a process would flip every seat minted WITHOUT a lease from
/// `Live` to `Gone`, because absent-from-the-set was being read as departed.
/// That is the half-populated registry this type's own warning predicted, and it
/// is far too easy to reach: any caller of bare `mint_seat` in a process that
/// also serves sessions.
///
/// So the registry only answers about seats it actually issued. `ever_leased`
/// records every seat this process has handed out; `attached` records the ones
/// still held. A seat in the first and not the second is DEFINITELY gone. A seat
/// in neither is none of the registry's business, and falls through to the pid
/// probe exactly as before. Absence of evidence stays absence of evidence.
///
/// `ever_leased` grows with sessions served rather than with time, and a shared
/// daemon expires on idle, so it is bounded in practice by one daemon's
/// lifetime — noted rather than capped, because evicting an entry would
/// resurrect the very ambiguity the second set exists to remove.
#[derive(Default)]
struct SeatRegistry {
    attached: std::collections::HashSet<String>,
    ever_leased: std::collections::HashSet<String>,
}

static ATTACHED_SEATS: std::sync::OnceLock<std::sync::Mutex<SeatRegistry>> =
    std::sync::OnceLock::new();

fn registry() -> &'static std::sync::Mutex<SeatRegistry> {
    ATTACHED_SEATS.get_or_init(|| std::sync::Mutex::new(SeatRegistry::default()))
}

/// Declare that this process is serving `seat`, so liveness can answer about the
/// SESSION rather than about the server holding it.
///
/// Idempotent. Only affects seats registered through here: a seat this process
/// never issued is left to the pid probe, so registering one session cannot make
/// another process's — or an unleased — seat read as a ghost.
pub fn register_seat(seat: &str) {
    if let Ok(mut reg) = registry().lock() {
        reg.attached.insert(seat.to_string());
        reg.ever_leased.insert(seat.to_string());
    }
}

/// This session is over: its seat is no longer served, so its claims are ghosts.
///
/// Called from the handler's `Drop`, so a client that disconnects, crashes or is
/// killed releases its seat without having to say anything. Nothing is written
/// to the graph — the CLAIM stays exactly where it was, and only its liveness
/// changes, which is the property that makes a ghost claim still readable for
/// what it says while no longer counting as held.
pub fn release_seat(seat: &str) {
    let Some(lock) = ATTACHED_SEATS.get() else {
        return;
    };
    if let Ok(mut reg) = lock.lock() {
        // Removed from `attached`, KEPT in `ever_leased`: that pair is what
        // makes the next read say `gone` rather than shrugging.
        reg.attached.remove(seat);
    }
}

/// What the registry knows about `seat`, or `None` if it is not its business.
///
/// Split out so the three conditions that must ALL hold before the registry may
/// override the pid probe — it is our own pid, we are tracking, and we issued
/// this seat — read as three refusals rather than as nested ifs.
fn registry_verdict(seat: &str, pid: u32) -> Option<Liveness> {
    if pid != std::process::id() {
        return None;
    }
    let reg = ATTACHED_SEATS.get()?.lock().ok()?;
    if !reg.ever_leased.contains(seat) {
        return None;
    }
    Some(if reg.attached.contains(seat) {
        Liveness::Live
    } else {
        Liveness::Gone
    })
}

/// How many sessions this process is serving right now, or `None` if it is not
/// tracking them (a plain stdio process, which is always serving exactly itself).
///
/// `req:a-session-can-tell-it-is-not-alone`: a seat that can see `attached: 7`
/// cannot honestly report a graph-wide rollup as its own result. dev_storyflow
/// had two bosses attribute a fleet-wide movement to their own change — in the
/// flattering direction, which is the one nobody catches unaided.
pub fn attached_seat_count() -> Option<usize> {
    ATTACHED_SEATS
        .get()
        .and_then(|l| l.lock().ok())
        .map(|reg| reg.attached.len())
}

/// Whether this process serves sessions OTHER than its own.
///
/// Set once at startup by `--serve-shared`; false everywhere else, which is the
/// honest default because a plain stdio process really is exactly one session.
///
/// # Why the registry alone was not enough
///
/// The registry answers about seats it LEASED. A seat this process merely
/// HANDED OUT — what the `mint_seat` tool returns — is in neither set, so it
/// falls through to the pid probe, and the pid probe asks whether WE are alive.
/// Under `--shared` that is trivially yes for every seat the daemon ever minted,
/// which is the original defect wearing a different hat: the 2026-08-08 registry
/// fixed the leased seat and left the handed-out one reading `live` forever.
///
/// The distinction this flag draws is the one that actually matters, and it
/// cannot be derived from the registry: in a plain stdio server the process IS
/// the session, so our-pid-is-alive is a TRUE and useful answer about a
/// handed-out seat; in a shared daemon it is no answer at all. Counting attached
/// seats cannot tell the two apart — a shared daemon serving one client looks
/// exactly like stdio — so the mode is declared rather than guessed.
static SERVES_MANY_SESSIONS: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Declare that this process serves sessions other than its own, so liveness
/// stops answering about a client it cannot observe. Called by `--serve-shared`.
pub fn declare_serving_many_sessions() {
    SERVES_MANY_SESSIONS.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Whether this process serves sessions other than its own.
#[must_use]
pub fn serving_many_sessions() -> bool {
    SERVES_MANY_SESSIONS.load(std::sync::atomic::Ordering::Relaxed)
}

/// A seat held for exactly as long as the session holding it.
///
/// The registry is only as truthful as its removals, and "remember to release
/// on every exit path" is the kind of discipline that holds until the day a
/// client crashes instead of disconnecting. So the release is not a step anybody
/// has to remember: it is `Drop`. A session that panics, is killed, or simply
/// loses its socket releases its seat on the way out, because the handler owning
/// the lease is dropped either way.
///
/// Hold it behind an `Arc` and clone THAT within a session — cloning the lease
/// itself would mint a second identity, and a service is legitimately cloned in
/// a dozen places inside one session.
#[derive(Debug)]
pub struct SeatLease {
    seat: String,
}

impl SeatLease {
    /// Mint a seat and declare this process is serving it.
    pub fn attach() -> Self {
        let seat = mint_seat();
        register_seat(&seat);
        Self { seat }
    }

    /// The seat handle, for passing to anything that records who is working.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.seat
    }
}

impl Drop for SeatLease {
    fn drop(&mut self) {
        release_seat(&self.seat);
    }
}

/// Ask whether the session that made a claim is still working.
///
/// Computed, not remembered, wherever it can be: a pid probe cannot go stale.
/// Cross-machine is deliberately `Unknown` — a pid means nothing on a computer
/// that is not the one that minted it.
///
/// **The one case a pid cannot answer** is a seat minted by THIS process while
/// this process serves many sessions: the pid is trivially alive (it is us), so
/// it says nothing about the client. There the registry decides, and its absence
/// from the registry is a real `Gone` rather than a guess — the session was
/// registered when it attached and removed when it dropped.
pub fn seat_liveness(seat: &str) -> Liveness {
    let parts: Vec<&str> = seat.split(':').collect();
    let [host, pid, ..] = parts.as_slice() else {
        return Liveness::Unknown;
    };
    if *host != machine() || *host == "unknown-machine" {
        return Liveness::Unknown;
    }
    let Ok(pid) = pid.parse::<u32>() else {
        return Liveness::Unknown;
    };
    // A seat this process minted while serving many: the pid is us, so it proves
    // nothing about the client. Ask what we are actually holding.
    //
    // Note the deliberate asymmetry with the branch below: a seat carrying a
    // DIFFERENT pid is still answered by the probe, because a seat left behind
    // by a previous daemon (a `--stop-shared` bounce, a crash) has a pid that
    // genuinely no longer exists, and `Gone` is the true answer for it.
    // Only seats this process actually issued. One it never leased is none of
    // the registry's business and falls through to the probe below.
    if let Some(verdict) = registry_verdict(seat, pid) {
        return verdict;
    }
    // Our own pid, never leased, and we serve more than ourselves: this is a
    // seat we HANDED OUT (the `mint_seat` tool) without holding anything that
    // tracks its owner. The probe below would ask whether WE are alive and
    // answer `live` for every such seat forever, so the honest answer is that we
    // cannot see. `unknown` is never read as free, so this fails toward
    // deferring to a claim rather than toward taking somebody's work.
    if pid == std::process::id() && serving_many_sessions() {
        return Liveness::Unknown;
    }
    if std::path::Path::new(&format!("/proc/{pid}")).exists() {
        return Liveness::Live;
    }
    // No /proc (macOS): ask ps. One spawn per distinct seat, on a report path
    // that runs when a person asks, never in a loop.
    if std::path::Path::new("/proc").exists() {
        // /proc exists but this pid is not in it: the process is genuinely gone.
        return Liveness::Gone;
    }
    match std::process::Command::new("ps")
        .args(["-p", &pid.to_string()])
        .output()
    {
        Ok(out) if out.status.success() => Liveness::Live,
        Ok(_) => Liveness::Gone,
        Err(_) => Liveness::Unknown,
    }
}

/// Take the identity of a design being imported into an empty store.
///
/// The case: `--import` (or `import_graph`) into a fresh graph is a *restore* —
/// same design, new store — and reflow2 says elsewhere that a copy of a design
/// **is** that design. If the empty graph kept the id it minted at open, the
/// round trip would not come back byte-identical, because `graph_id` is part of
/// the export's content hash. The project's own smoke test caught exactly that
/// the hour identity landed.
///
/// A graph that already holds a design keeps its own name, always. That is the
/// other half of the same rule, and it is what makes the stale-seat remedy safe:
/// absorbing the shared record into a working graph must never rename it.
///
/// Returns the adopted identity when it took, `None` when the graph kept its
/// own. **Call before importing** — the import writes under the current id.
pub fn adopt_on_import(
    graph_path: &str,
    document_graph_id: &str,
    holds_a_design: bool,
) -> Result<Option<DesignIdentity>, DynoError> {
    if holds_a_design || document_graph_id.is_empty() {
        return Ok(None);
    }
    let mut identity = resolve(graph_path, document_graph_id, || false)?;
    if identity.graph_id == document_graph_id {
        return Ok(None);
    }
    identity.graph_id = document_graph_id.to_string();
    identity.origin = Origin::Adopted;
    write(graph_path, &identity)?;
    Ok(Some(identity))
}
