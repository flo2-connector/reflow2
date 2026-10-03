//! A session names the design it wants by `graph_id`, and the server maps id to
//! path.
//!
//! `cap:select-graph-by-id`, and the second half of `req:a-session-chooses-its-design`
//! (accepted). The first half — finding out what designs exist WITHOUT opening
//! them — already shipped as `describe_designs`; this is the half that lets a
//! session be pointed at one.
//!
//! # Why an id and not a path
//!
//! `rule:a-design-is-named-by-an-id-not-a-path`, Anthony 2026-08-09: *"the id is
//! primary and the path is a storage detail. A surface that treats a filesystem
//! path as the canonical identity forecloses object storage, where the location
//! is a key and the identity rides as metadata."*
//!
//! That rule is ADVISORY, and it says exactly why: *"this clause has no
//! compliant surface at all… an honest detector would fail against the entire
//! existing tool surface on the day it was written."* It also names its own
//! trigger — *"cap:select-graph-by-id becoming realized"* — which is this
//! module. The rule can flip to enforced once a NEW surface has somewhere
//! compliant to be.
//!
//! # The two clauses this module is accountable for
//!
//! `ver:a-session-cannot-name-another-design`, and the conditions
//! `dec:one-process-many-stores` was accepted on:
//!
//! 1. **an id a session was not attached to is REFUSED rather than served**
//! 2. **a path is not an alternative route in**
//!
//! Clause 2 is why [`Registry::attach`] takes `&str` and treats it as an ID
//! ALWAYS — a path handed to it is an unknown id, never a location to open.
//! There is deliberately no `path_for(id)` and no `attach_path`: a convenience
//! overload is exactly how this property would be lost, so it is absent rather
//! than documented.
//!
//! Clause 1 is why the registry's ROOT is the boundary. A design that genuinely
//! exists elsewhere on the machine is refused identically to one that was made
//! up — knowing a real `graph_id` is not a way in.
//!
//! # A binding is the capability
//!
//! [`Registry::attach`] returns a [`Binding`], and a `Binding` exposes exactly
//! one design. There is no operation on it that takes another id, so "a session
//! cannot name another design" holds BY CONSTRUCTION rather than by a check
//! somebody must remember to keep.
//!
//! # What this deliberately does not settle
//!
//! **Who may see which designs.** [`Registry::graph_ids`] lists what the
//! OPERATOR placed under the root. That is right for one owner's own
//! neighbourhood — Anthony, 2026-08-05: *"can a reflow2 tool be to simply return
//! all graph_ids to the agent and the agent then can choose whichever graph_id
//! the user specifies"* — and it is NOT a multi-tenant policy.
//! `dec:idea-a-session-holds-several-graphs` records the distinction that
//! decides it, *"multi-tenant isolation is not the same as composition"*, and it
//! is unanswered. Nothing here forecloses either answer: a registry per tenant,
//! or a filtered listing, both remain available because the root is a parameter.
//!
//! **The transport.** `cap:select-graph-by-id` prefers an HTTP path prefix
//! (`/g/<graph_id>/`) so selection is visible in logs and routable by ordinary
//! proxies, with an MCP initialize parameter second and an attaching tool call
//! worst — *"it makes every session stateful in a way a reconnect silently
//! loses"*. This module is the resolution half only; no transport is wired to it
//! yet, and the single-`--graph-path` server is untouched.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Why an attach was refused.
///
/// An id this registry does not hold is ONE refusal whatever the caller sent —
/// a path, a stranger's real id and a typo read the same, because telling them
/// apart would leak what exists beyond the root. The one other refusal is
/// about a design that IS under this root and cannot be served, which the
/// listing names anyway (the root is the tenant boundary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachError {
    /// This registry holds no design under that id.
    ///
    /// The requested id is echoed because a refusal that does not say what it
    /// refused is a wall — but nothing about what DOES exist is disclosed here.
    UnknownGraphId { requested: String },
    /// An identity file under this root names that id, and the store it names
    /// is not there. Opening it would create an EMPTY store under the old id
    /// and serve the design as empty, so it is refused by name instead.
    StoreMissing { requested: String, store: String },
}

impl std::fmt::Display for AttachError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttachError::UnknownGraphId { requested } => write!(
                f,
                "no design named `{requested}` is registered here. A session names a design by \
                 its graph_id, and a filesystem path is not an alternative route in — list what \
                 this server holds and name one of those."
            ),
            AttachError::StoreMissing { requested, store } => write!(
                f,
                "design `{requested}` is named by an identity file under this root, but its \
                 store is not here ({store} does not exist). It is NOT opened: opening would \
                 create an empty store under that id and serve the design as empty while its \
                 data is somewhere else. {}",
                Unserved::REMEDY_STORE_MISSING
            ),
        }
    }
}

/// A store discovery found under the root and will not serve, and why.
///
/// ⭐ DISCOVERY NEVER SILENTLY DISCARDS WHAT IT CLASSIFIED. Until 2026-09-28
/// `discover` kept only what `describe_at` read as a design and dropped the
/// rest, so a store whose identity file was missing — a volume mounted at the
/// store instead of its parent — made the root read as holding "no designs"
/// while the single-design server refused the same store loudly (GitHub issue
/// #616; `fact:root-cause-the-registry-drops-a-store-without-identity-and-says-no-designs-2026-09-28`).
/// Each of these is now kept, named at startup, in the listing and in the
/// refusal for an unknown id, and NEVER opened: opening would mint or recreate
/// an identity and so answer its own question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unserved {
    /// The directory under the root that holds it, as found.
    pub dir: String,
    /// What is wrong.
    pub kind: UnservedKind,
    /// `describe_at`'s own reading, verbatim — the one sentence every surface
    /// quotes, so no surface keeps its own table of what each case means.
    pub reading: String,
}

/// The three things discovery can find that must not be served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnservedKind {
    /// A store is there with no identity file beside it.
    IdentityMissing,
    /// An identity file is there and cannot be read.
    IdentityUnreadable,
    /// An identity file names this design, and the store it names is gone.
    StoreMissing { graph_id: String },
}

impl Unserved {
    const REMEDY_IDENTITY_MISSING: &'static str = "The identity file (graph.id.json) is a SIBLING of the store, not inside it. If this is \
         a container, the usual cause is a volume mounted at the store directory (…/.reflow2/graph) \
         instead of its parent (…/.reflow2) — mount the parent. Otherwise restore graph.id.json \
         from a backup, beside the store it belongs to. The root is re-read on every request, so \
         the design is served as soon as its identity is back, with no restart.";
    const REMEDY_IDENTITY_UNREADABLE: &'static str = "Restore graph.id.json from a backup, beside the store it belongs to. reflow2 will not \
         guess which design this is.";
    const REMEDY_STORE_MISSING: &'static str = "The store (…/.reflow2/graph) is a SIBLING of the identity file. If this is a container, \
         check that the volume holding the store is mounted; otherwise restore the store from a \
         backup, beside its identity file.";

    /// What would make it servable.
    pub fn remedy(&self) -> &'static str {
        match self.kind {
            UnservedKind::IdentityMissing => Self::REMEDY_IDENTITY_MISSING,
            UnservedKind::IdentityUnreadable => Self::REMEDY_IDENTITY_UNREADABLE,
            UnservedKind::StoreMissing { .. } => Self::REMEDY_STORE_MISSING,
        }
    }

    /// One line: where it is, what is wrong, and what would fix it.
    pub fn sentence(&self) -> String {
        let what = match &self.kind {
            UnservedKind::IdentityMissing => {
                "a store with no identity file (graph.id.json) beside it".to_string()
            }
            UnservedKind::IdentityUnreadable => {
                "a store whose identity file (graph.id.json) cannot be read".to_string()
            }
            UnservedKind::StoreMissing { graph_id } => {
                format!("an identity file naming design {graph_id}, whose store is not here")
            }
        };
        format!(
            "{dir}: {what} — {reading} {remedy}",
            dir = self.dir,
            reading = self.reading,
            remedy = self.remedy()
        )
    }
}

impl std::error::Error for AttachError {}

/// One session's attachment to one design.
///
/// Holds the resolved store path so the server can open it, and offers NO way to
/// name a second design. That absence is the isolation property, not an
/// oversight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    graph_id: String,
    graph_path: String,
}

impl Binding {
    /// The design this session is attached to.
    pub fn graph_id(&self) -> &str {
        &self.graph_id
    }

    /// Where that design's store lives — for the server that opens it, never
    /// for a client. The path is the storage detail the id exists to hide.
    pub fn graph_path(&self) -> &str {
        &self.graph_path
    }
}

/// The designs one server offers, by id — and the stores it found and will not
/// serve, each with why.
#[derive(Debug, Clone)]
pub struct Registry {
    root: String,
    by_id: BTreeMap<String, String>,
    unserved: Vec<Unserved>,
}

impl Registry {
    /// Read every design directly under `root`, WITHOUT opening any store.
    ///
    /// Identity comes from the sidecar files beside each store, which exist to
    /// be read before opening — so discovery takes no lock, writes nothing, and
    /// a design another session holds right now enumerates fine.
    ///
    /// A directory whose store carries no readable identity is NOT registered.
    /// Naming it would mean opening it, which MINTS an identity and thereby
    /// answers its own question — the failure `describe_designs` exists to
    /// avoid, and the one that once minted a third graph beside two populated
    /// ones. Nor is an identity whose store is gone: opening that creates an
    /// empty store under the old id and serves the design as empty.
    ///
    /// ⭐ BUT NEITHER IS DROPPED. Each is kept in [`Registry::unserved`] with
    /// `describe_at`'s reading, so every surface can say what is here and why
    /// it is not served. A directory with nothing in it (`OptedIn`, `Absent`)
    /// holds nothing to lose and is passed over, as before.
    pub fn discover(root: &str) -> Self {
        let mut by_id = BTreeMap::new();
        let mut unserved = Vec::new();
        if let Ok(entries) = std::fs::read_dir(root) {
            let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            dirs.sort();
            for dir in dirs {
                let store = dir.join(".reflow2").join("graph");
                let Some(store) = store.to_str() else {
                    continue;
                };
                let found = reflow2_core::describe_at(store);
                let kind = match (found.state, found.graph_id) {
                    // `describe_at` sets `graph_id: Some` on exactly one path,
                    // the one that reports `Design` (measured by mutation on
                    // 2026-08-12); the state is matched too, as the statement
                    // of intent and a defence if that ever stops being true.
                    (reflow2_core::DesignPathState::Design, Some(id)) => {
                        // The rule every mode asks before it opens
                        // (`crate::opening`): bind only a store that exists, so
                        // serving this id never creates one.
                        if crate::opening::store_exists(store) {
                            by_id.insert(id, store.to_string());
                            continue;
                        }
                        UnservedKind::StoreMissing { graph_id: id }
                    }
                    (reflow2_core::DesignPathState::Unnamed, _) => {
                        if reflow2_core::identity::identity_path(store).exists() {
                            UnservedKind::IdentityUnreadable
                        } else {
                            UnservedKind::IdentityMissing
                        }
                    }
                    _ => continue,
                };
                unserved.push(Unserved {
                    dir: dir.display().to_string(),
                    kind,
                    reading: found.reading,
                });
            }
        }
        Registry {
            root: root.to_string(),
            by_id,
            unserved,
        }
    }

    /// The stores found under the root that are not served, each with why and
    /// what would fix it. Empty when every store found is a design.
    pub fn unserved(&self) -> &[Unserved] {
        &self.unserved
    }

    /// The sentence an unknown id's refusal adds when stores were found and not
    /// served, so "not here" is never said of a design that is here and broken.
    /// `None` when there are none.
    pub fn unserved_note(&self) -> Option<String> {
        let n = self.unserved.len();
        (n > 0).then(|| {
            format!(
                "This server also found {n} store{s} under its root that it cannot serve, \
                 listed with why on a GET of / and in its startup log. If the design you expect \
                 is one of them, that is why it is not reachable.",
                s = if n == 1 { "" } else { "s" }
            )
        })
    }

    /// The root this registry was built over — echoed so an empty answer reads
    /// as "nothing under here" rather than as "nobody looked".
    pub fn root(&self) -> &str {
        &self.root
    }

    /// Every design a session may name, by id.
    ///
    /// This is the operator's offer, not a claim about the machine: designs
    /// outside the root are invisible here and unreachable through
    /// [`Registry::attach`].
    pub fn graph_ids(&self) -> Vec<String> {
        self.by_id.keys().cloned().collect()
    }

    /// Attach a session to the design named by `graph_id`.
    ///
    /// **The argument is always an ID.** A filesystem path handed here is an
    /// unknown id and is refused as one; there is no path route in, by absence
    /// rather than by a check.
    pub fn attach(&self, graph_id: &str) -> Result<Binding, AttachError> {
        if let Some(path) = self.by_id.get(graph_id) {
            return Ok(Binding {
                graph_id: graph_id.to_string(),
                graph_path: path.clone(),
            });
        }
        // Only an id an identity file UNDER THIS ROOT names reaches this arm,
        // so it discloses nothing the listing does not already show.
        let missing = self.unserved.iter().find_map(|u| match &u.kind {
            UnservedKind::StoreMissing { graph_id: id } if id == graph_id => Some(
                Path::new(&u.dir)
                    .join(".reflow2")
                    .join("graph")
                    .display()
                    .to_string(),
            ),
            _ => None,
        });
        Err(match missing {
            Some(store) => AttachError::StoreMissing {
                requested: graph_id.to_string(),
                store,
            },
            None => AttachError::UnknownGraphId {
                requested: graph_id.to_string(),
            },
        })
    }

    /// How many designs are registered. Stated so a caller can tell an empty
    /// registry from one it never built.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Whether the registry offers nothing.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

/// The conventional store path under a project directory.
pub fn store_path_under(project_dir: &Path) -> PathBuf {
    project_dir.join(".reflow2").join("graph")
}
