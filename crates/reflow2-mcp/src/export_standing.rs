//! Has this design ever been exported anywhere this machine knows of?
//!
//! # The finding this exists to fix
//!
//! The VS Code field log of 2026-10-05 (`art:vscode-call-field-log-2026-10-05`,
//! reflow2 0.76.0): a member design held 195 nodes ONLY in its machine-local
//! store — no export anywhere, no `--export-to` — "one disk away from loss",
//! and `loop_status` answered `clean: true`, `next: []`. Measured again on
//! 0.77.0 (`fact:a-never-exported-local-design-is-still-silent-in-loop-status-through-the-door-on-0-77-0-2026-10-02`)
//! and on 0.78.0 and main.
//!
//! # The cause, and why it was not the door's
//!
//! The only sentence `loop_status` had about exports is
//! [`crate::sync_debt::unexported_work`], and it is computed from the records
//! this seat has exported to (`provenance::last_synced`): "this graph holds N
//! nodes, the fullest record holds M". A store with NO record returned `None`
//! — the same answer as a store whose record holds everything — so "nothing
//! tracked" and "nothing owed" shared one reply
//! (`fact:root-cause-a-hosted-session-hears-nothing-about-exports-because-the-only-export-line-rides-a-tracked-export-2026-09-28`,
//! candidate (b), confirmed). Every door ran that same code. The `--call` door
//! only made the case common: a door-only design acquires a record only if an
//! export is configured or run by hand, so "never exported" is its default
//! state rather than an accident.
//!
//! The fix is not another branch in `unexported_work` (which answers a
//! different question, about unexported work beside a record, and is pinned to
//! stay silent with no record). It is this one computation, from the store's
//! own export record and the export the project configures, which every path
//! that reports on the design — `loop_status` and `graph_report`, through MCP,
//! `--call` and `read` alike — calls the same way.
//!
//! # When it speaks
//!
//! Only for a design on THIS machine's disk, served with its project tree, that
//! holds at least one node and has no readable copy anywhere this machine knows
//! of:
//!
//! - no record of any export or import of it that still holds a design, and
//! - no export target configured for it that exists on disk (`--export-to` on
//!   this server, or the one the project's MCP configuration names — the same
//!   lookup a writing `--call` uses, [`crate::call_export::find`]), and
//! - no write-through waiting to write one.
//!
//! A design served without its tree (a host's registry) is never told: there
//! the store is the design and its backup is the host's
//! (`dec:idea-one-blueprint-the-store-is-the-design-and-an-export-is-a-perishable-photocopy`,
//! rules 3 and 6). What a hosted session should hear instead is open in
//! `dec:idea-loop-status-says-whether-an-export-is-owed-on-a-hosted-design`.
//!
//! # A design kept local-only on purpose
//!
//! reflow2 records no "local only" marker. So the choice is recorded the way
//! every other accepted gap is: `acknowledge_gap` with
//! [`NEVER_EXPORTED_GAP_ID`] and the reason, ideally with the owner as
//! approver. The block then says it was acknowledged and `next` stops carrying
//! it; `withdraw_gap_acknowledgement` brings it back.
//!
//! # Additive, for every client that already reads loop_status
//!
//! Nothing here renames, removes or re-means a field: an `export_standing`
//! block and one `next` line appear only in the state above, and `clean` is
//! the core's verdict on the coherence loop exactly as before. A design that
//! has been exported gets exactly the reply it got before
//! (`req:the-mcp-server-keeps-working-as-is-and-door-improvements-are-additive-or-opt-in`).

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::arguments::Transport;
use crate::sync_debt::SyncDebt;

pub use reflow2_core::NEVER_EXPORTED_GAP_ID;

/// What the project configures as this design's export, as far as can be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Configured {
    /// Nothing names an export for this design.
    Nothing,
    /// One file is named, and by whom.
    Target { path: String, named_by: String },
    /// The project's MCP configurations name different files.
    Disagree(String),
}

/// The owner's recorded choice to keep this design local-only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Acknowledged {
    pub decision_id: String,
    pub reason: String,
}

/// The item: this design has no copy anywhere this machine knows of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportStanding {
    /// `never_exported`: no export of it was ever recorded, and none is
    /// configured that exists. `no_export_survives`: it was exported (or
    /// imported) before, and nothing readable is at any of those paths now.
    pub state: String,
    /// Always `high`: losing the store directory loses the design.
    pub severity: String,
    pub live_nodes: usize,
    /// The store that holds the only copy.
    pub store: String,
    /// The records this store knew of, none readable now (`no_export_survives`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub records_gone: Vec<String>,
    /// The export the project configures, when one is named but not written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configured: Option<String>,
    /// The one command that fixes it, for the door this call came through.
    pub fix: String,
    /// The gap a design kept local-only on purpose acknowledges.
    pub gap_id: String,
    /// Set when that choice is on record; `next` then stays quiet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acknowledged: Option<Acknowledged>,
    /// The item in words — the line `next` carries.
    pub message: String,
}

impl ExportStanding {
    /// Whether the item belongs in `next`: always, unless the owner recorded
    /// that this design is kept local-only on purpose.
    pub fn owed(&self) -> bool {
        self.acknowledged.is_none()
    }
}

/// What one reading of the store and its surroundings found.
pub struct Seen<'a> {
    pub graph_path: &'a str,
    pub project_root: &'a Path,
    pub live_nodes: usize,
    /// The records this store has synced with, as `sync_debt` read them.
    pub debts: &'a [SyncDebt],
    /// Records this store knows of that `debts` did not open.
    pub unopened_records: usize,
    pub configured: Configured,
    /// A write-through is waiting to write the export.
    pub export_pending: bool,
    pub door: Transport,
    pub acknowledged: Option<Acknowledged>,
}

/// A record state that means a readable copy of the design is at that path.
fn holds_a_copy(d: &SyncDebt) -> bool {
    matches!(d.state.as_str(), "in_step" | "moved_but_current" | "behind")
}

/// The item, or `None` when this design has a copy (or nothing to lose).
pub fn assess(seen: Seen<'_>) -> Option<ExportStanding> {
    if seen.live_nodes == 0 || seen.export_pending {
        return None;
    }
    // A readable record anywhere, or records not opened by this roll (which
    // may be readable): a copy exists, or cannot be ruled out.
    if seen.debts.iter().any(holds_a_copy) || seen.unopened_records > 0 {
        return None;
    }
    let target: Option<PathBuf> = match &seen.configured {
        Configured::Target { path, .. } => {
            let p = PathBuf::from(path);
            let p = if p.is_absolute() {
                p
            } else {
                seen.project_root.join(p)
            };
            // A file at the configured path is the committed export: a copy.
            if p.exists() {
                return None;
            }
            Some(p)
        }
        _ => None,
    };
    // Where to write one when nothing is configured: the path `reflow2 init`
    // configures (`docs/design/<project>.json`) when that folder exists, and
    // otherwise `reflow2.json` at the project root — `export_graph` does not
    // create folders, so the command named must work as given.
    let export_path = target.clone().unwrap_or_else(|| {
        let name = seen
            .project_root
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| !n.is_empty())
            .unwrap_or("design");
        let conventional = seen.project_root.join("docs").join("design");
        if conventional.is_dir() {
            conventional.join(format!("{name}.json"))
        } else {
            seen.project_root.join("reflow2.json")
        }
    });
    let export_path = export_path.display().to_string();
    let store = absolute(seen.graph_path);
    let fix = match seen.door {
        Transport::Session => format!("export_graph with {{\"path\": \"{export_path}\"}}"),
        Transport::CallDoor => format!(
            "reflow2-mcp --graph-path {store} --call export_graph --args '{{\"path\":\"{export_path}\"}}'"
        ),
    };
    let keep = match (&seen.configured, seen.door) {
        (Configured::Target { .. }, _) => {
            "the configured export is then kept current after every write".to_string()
        }
        (_, Transport::Session) => format!(
            "start the server with --export-to {export_path} (what `reflow2 init` configures) \
             to keep it current"
        ),
        (_, Transport::CallDoor) => format!(
            "pass --export-to {export_path} on each later writing --call to keep it current"
        ),
    };
    let local_only = format!(
        "If this design is meant to stay on this machine only, acknowledge_gap with gap_id \
         \"{NEVER_EXPORTED_GAP_ID}\", affected_ids [] and your reason, and this stops."
    );
    let records_gone: Vec<String> = seen.debts.iter().map(|d| d.path.clone()).collect();
    let (state, message) = if records_gone.is_empty() {
        let why = match &seen.configured {
            Configured::Nothing => {
                "neither --export-to nor the project's MCP configuration names an export for it"
                    .to_string()
            }
            Configured::Target { named_by, .. } => {
                format!("the export {named_by} names, {export_path}, has never been written")
            }
            Configured::Disagree(e) => {
                format!("the project's MCP configurations disagree about its export ({e})")
            }
        };
        (
            "never_exported",
            format!(
                "THIS DESIGN HAS NEVER BEEN EXPORTED anywhere this machine has a record of. Its \
                 {n} node(s) exist only in the store at {store}: no export of it is on record, and \
                 {why}. Losing that directory loses the design. Fix: {fix} — then commit the file; \
                 {keep}. (A copy written with `--export > FILE` leaves no record; export_graph \
                 with that path makes one.) {local_only}",
                n = seen.live_nodes,
            ),
        )
    } else {
        (
            "no_export_survives",
            format!(
                "NO EXPORT OF THIS DESIGN SURVIVES. It was exported to (or imported from) {}, and \
                 none of them holds a readable design now, so its {n} node(s) exist only in the \
                 store at {store}. Losing that directory loses the design. Fix: {fix} — then \
                 commit the file; {keep}. {local_only}",
                records_gone.join(", "),
                n = seen.live_nodes,
            ),
        )
    };
    Some(ExportStanding {
        state: state.to_string(),
        severity: "high".to_string(),
        live_nodes: seen.live_nodes,
        store,
        records_gone,
        configured: match &seen.configured {
            Configured::Target { path, named_by } => Some(format!("{path} (named by {named_by})")),
            Configured::Disagree(e) => Some(e.clone()),
            Configured::Nothing => None,
        },
        fix,
        gap_id: NEVER_EXPORTED_GAP_ID.to_string(),
        acknowledged: seen.acknowledged,
        message,
    })
}

fn absolute(path: &str) -> String {
    let p = Path::new(path);
    if p.is_absolute() {
        return path.to_string();
    }
    let p = p.strip_prefix("./").unwrap_or(p);
    std::env::current_dir()
        .map(|cwd| cwd.join(p).display().to_string())
        .unwrap_or_else(|_| path.to_string())
}

/// The one line that answers "does this store hold work its export lacks?",
/// for the TOP of a report — `ahead_of_export` on `loop_status` and
/// `sync_status`.
///
/// # Why a headline, measured
///
/// The owner's work laptop, upgrading 20 stores to 0.79.0 (field log,
/// 2026-10-05): the recipe read `--call loop_status | head -40` to ask
/// "anything unexported?", and the first 40 lines were the artifact block (a
/// member store's unmeasurable remote locations); `sync_status` says by design
/// that being ahead of the record is not reported. No served answer reached
/// the top of a reply, so the check was done by hand: export to /tmp and
/// compare ids with the committed copy. The `--call` door prints the reply's
/// keys in sorted order, and `ahead_of_export` sorts before every key
/// `loop_status` had, so it is the reply's first line.
///
/// # What it can say, and how sure it is
///
/// - A design with no copy anywhere (`standing`, not acknowledged): that.
/// - With the list in hand (`since_export: true`), the exact count of nodes
///   and edges added, changed and removed, and who wrote them.
/// - Otherwise the cheap reading every call already makes: how many more
///   NODES the store holds than its fullest in-step export. A node count, so
///   a change to a property or an edge alone is not counted, and the line
///   says so and names the call that counts everything.
///
/// `None` — nothing added to the reply — when none of those finds the store
/// ahead, so a design whose export is current gets the reply it got before.
pub fn ahead_of_export(
    standing: Option<&ExportStanding>,
    debts: &[SyncDebt],
    live_nodes: usize,
    listed: Option<&reflow2_core::unexported::UnexportedChanges>,
) -> Option<String> {
    if let Some(s) = standing {
        if !s.owed() {
            return None;
        }
        let gist = if s.state == "never_exported" {
            "THIS DESIGN HAS NEVER BEEN EXPORTED"
        } else {
            "NO EXPORT OF THIS DESIGN SURVIVES"
        };
        return Some(format!(
            "ahead of every export: {gist} — all {} node(s) exist only in the store at {}. \
             export_standing.fix is the command that writes a copy.",
            s.live_nodes, s.store
        ));
    }
    if let Some(u) = listed {
        if u.identical {
            return None;
        }
        let mut by: Vec<String> = u
            .groups
            .iter()
            .map(|g| {
                format!(
                    "{} {}",
                    g.written_by.as_deref().unwrap_or("no recorded writer"),
                    g.counts.total()
                )
            })
            .collect();
        if u.groups_not_shown > 0 {
            by.push(format!(
                "{} more group(s) holding {}",
                u.groups_not_shown, u.changes_not_shown
            ));
        }
        return Some(format!(
            "ahead of {}: {} not in the export — by writer: {}. `unexported` lists them.",
            u.base,
            u.totals.phrase(),
            by.join(", ")
        ));
    }
    let best = debts
        .iter()
        .filter(|d| d.state == "in_step" || d.state == "moved_but_current")
        .max_by(|a, b| {
            a.export_nodes
                .cmp(&b.export_nodes)
                .then_with(|| b.path.cmp(&a.path))
        })?;
    if best.export_nodes >= live_nodes {
        return None;
    }
    Some(format!(
        "ahead of {}: {} node(s) not in the export (a node count: a change to a property or an \
         edge alone is not counted). loop_status with since_export: true counts every node and \
         edge and says who wrote them.",
        best.path,
        live_nodes - best.export_nodes
    ))
}
