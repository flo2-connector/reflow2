//! The changelog's BALANCE LINE: what was taken away beside what was added
//! between two releases, and the failures that came back
//! (`cap:changelog-view-carries-a-balance-line`, for
//! `req:a-changelog-shows-what-was-taken-away-beside-what-was-added`).
//!
//! # Where the counts come from (`dec:how-the-changelog-balance-is-counted`, option C)
//!
//! Measured on reflow2's own design before this was built: a status move is
//! not a change. `CHANGED` edges record an action and never a status before
//! and after, so 11 dropped requirements and 38 reversed decisions were on the
//! time axis nowhere, and a count of what the window recorded would have read
//! "0 dropped". So each release KEEPS A STATUS SNAPSHOT at its cut — the status
//! of every Requirement, Capability, Decision and Verification, and whether an
//! accepted decision has withdrawn it (a stored status does not move on
//! withdrawal) — and the balance is the difference between two of them.
//!
//! - **At the cut, with no new tool.** [`DesignGraph::release_includes_all`]
//!   with `apply: true` keeps it, on the existing `Snapshot` type, linked to
//!   the release (`target_id`, so the store draws `HAS_SNAPSHOT`). The payload
//!   is the Snapshot's `state` JSON, [`StatusSnapshotPayload`].
//! - **Stored as a delta** from the previous release's snapshot, where
//!   "previous" is the release with the highest epoch sequence below this one
//!   that keeps one. The first is stored whole. The payload names its base,
//!   and reading one back follows that chain, so a delta is exact whichever
//!   base it was taken from.
//! - **What a release saw does not move.** A snapshot is never rewritten: a
//!   re-run of the cut reports it as already kept, as the manifest's frozen
//!   checksums are. Nor is one written for a release whose cut is past (its
//!   manifest was frozen before the call, or a later release keeps one),
//!   because today's statuses are not what that release saw.
//!
//! # Limits, said rather than hidden
//!
//! - An end with no snapshot is NAMED, and its counts are absent — never zero.
//! - The counts are the NET move between two ends: something added and taken
//!   away inside one window appears in neither column.
//! - It says that something moved, never which change moved it.
//! - `[Unreleased]` reads the last deployed release against the live design.
//! - It never says a design does too much. Scope stays the person's call.
//!
//! Back-filling past releases from the design copy committed at each tag is
//! part of the decision and NOT built here: a named follow-up. Until it is, a
//! release cut before this keeps no snapshot and the balance says so.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::foundation::core::{DynoError, Value};
use crate::foundation::store::StoredNode;
use crate::graph::DesignGraph;
use crate::nodes::{Props, edge, node};

/// Every status snapshot's id starts with this, followed by the release id.
pub const STATUS_SNAPSHOT_PREFIX: &str = "snap:release-status:";
/// The payload's `kind`, so a reader never mistakes one for a node's state.
pub const STATUS_SNAPSHOT_KIND: &str = "release_status_snapshot";
/// The payload format this build writes and reads.
pub const STATUS_SNAPSHOT_FORMAT: u32 = 1;
/// The node types a status snapshot covers, in the order it stores them.
pub const STATUS_SNAPSHOT_COVERS: [&str; 4] = [
    node::CAPABILITY,
    node::DECISION,
    node::REQUIREMENT,
    node::VERIFICATION,
];

/// The id of the status snapshot `release_id` keeps.
pub fn status_snapshot_id(release_id: &str) -> String {
    format!("{STATUS_SNAPSHOT_PREFIX}{release_id}")
}

/// Whether a Snapshot id is a release's status snapshot rather than a capture
/// of one node's state. Readers of a node's own history skip these.
pub fn is_status_snapshot_id(id: &str) -> bool {
    id.starts_with(STATUS_SNAPSHOT_PREFIX)
}

/// One node's status as a release saw it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusEntry {
    /// The stored `status`, absent when the node carried none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// An ACCEPTED Decision had withdrawn it (`OBSOLETES`). Kept beside the
    /// status because the status does not move on withdrawal: a withdrawn
    /// capability goes on saying `realized`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub discontinued: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Statuses by node type, then by id. Keyed by type first so two types
/// sharing an id (writable, though against convention) never collide.
pub type StatusMap = BTreeMap<String, BTreeMap<String, StatusEntry>>;

/// How a status snapshot is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoredAs {
    /// Every covered node's status.
    Whole,
    /// Only what differs from the base snapshot it names.
    Delta,
}

/// What a status snapshot stores, as the JSON in the Snapshot's `state`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusSnapshotPayload {
    /// Always [`STATUS_SNAPSHOT_KIND`].
    pub kind: String,
    /// Always [`STATUS_SNAPSHOT_FORMAT`] for what this build writes.
    pub format: u32,
    pub release_id: String,
    pub stored_as: StoredAs,
    /// For a delta: the release whose snapshot it is a delta from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_release: Option<String>,
    /// For a delta: that release's snapshot id, which reading back follows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_snapshot: Option<String>,
    /// The node types it covers ([`STATUS_SNAPSHOT_COVERS`]).
    pub covers: Vec<String>,
    /// Whole: every covered node. Delta: the nodes new or changed since the
    /// base; a type with none is left out.
    #[serde(default)]
    pub set: StatusMap,
    /// Delta only: the ids the base held that are no longer in the design.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub removed: BTreeMap<String, Vec<String>>,
}

impl StatusSnapshotPayload {
    /// How many entries this record holds: every status when whole; the
    /// changed, new and removed ones when a delta.
    pub fn entries(&self) -> usize {
        self.set.values().map(BTreeMap::len).sum::<usize>()
            + self.removed.values().map(Vec::len).sum::<usize>()
    }
}

fn status_count(map: &StatusMap) -> usize {
    map.values().map(BTreeMap::len).sum()
}

/// What `release_includes_all` did about the release's status snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusSnapshotOutcome {
    /// Kept by this call.
    Written,
    /// Kept at an earlier call and left exactly as it was.
    AlreadyKept,
    /// Not applied: what applying would keep.
    DryRun,
    /// Not taken, and `note` says why.
    NotTaken,
}

/// The status snapshot's part of a derived manifest's reply.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatusSnapshotReport {
    pub snapshot_id: String,
    pub outcome: StatusSnapshotOutcome,
    pub stored_as: Option<StoredAs>,
    /// The release a delta is taken from.
    pub base_release: Option<String>,
    /// Entries the stored record holds (or would hold).
    pub entries: Option<usize>,
    /// Statuses it gives once read back: requirements, capabilities,
    /// decisions and checks together.
    pub statuses: Option<usize>,
    pub note: String,
}

/// Where one end of a balance took its statuses from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BalanceSource {
    /// A release's status snapshot.
    Snapshot,
    /// The design as it stands now (`[Unreleased]`'s later end).
    LiveDesign,
    /// Nothing: this end keeps no snapshot, and `notes` names it.
    NotKept,
}

/// One end of the balance.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BalanceEnd {
    /// The release or epoch this end is; `None` for the live design.
    pub id: Option<String>,
    pub source: BalanceSource,
    pub snapshot_id: Option<String>,
}

/// One thing counted in a column.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BalanceItem {
    pub id: String,
    pub node_type: String,
    /// Its name in the design now; absent when it is no longer there.
    pub name: Option<String>,
    /// Its status at the earlier end; absent when it was not in the design.
    pub before: Option<String>,
    /// Its status at the later end; absent when it is no longer in the design.
    pub after: Option<String>,
    /// What moved, in words.
    pub how: String,
}

/// One column: a count, how it was counted, and what it counted.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BalanceColumn {
    pub count: usize,
    pub how_counted: String,
    pub items: Vec<BalanceItem>,
}

/// What was added beside what was taken away, for one kind of intent.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BalancePair {
    pub added: BalanceColumn,
    pub taken_away: BalanceColumn,
}

/// The changelog draft's balance line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChangelogBalance {
    pub from: BalanceEnd,
    pub to: BalanceEnd,
    /// Whether both ends gave statuses, so the four pairs below were counted.
    /// When false they are absent — never zero — and `notes` names the end.
    pub counted: bool,
    /// Added, against dropped or deferred.
    pub requirements: Option<BalancePair>,
    /// Added, against removed or discontinued.
    pub capabilities: Option<BalancePair>,
    /// Accepted, against reversed.
    pub decisions: Option<BalancePair>,
    /// Checks that went from passing to failing.
    pub checks_now_failing: Option<BalanceColumn>,
    /// The window's `test_failure_fix` changes: fixes a check caught. Counted
    /// from the window's pinned ChangeEvents, so present even when an end
    /// keeps no snapshot.
    pub failures_that_came_back: BalanceColumn,
    /// What this way of counting cannot see, said every time.
    pub limits: Vec<String>,
    /// What limited this answer: an end with no snapshot, a snapshot that
    /// cannot be read back.
    pub notes: Vec<String>,
}

const LIMIT_NET: &str = "Each count is the NET move between the two ends: something added and \
     taken away inside one window appears in neither column, and something taken away and \
     restored inside it appears in neither too.";
const LIMIT_WHICH: &str = "It says THAT something moved between the two ends, never WHICH change \
     moved it.";
const LIMIT_LIVE: &str = "The later end is the live design, read now rather than at a cut: it \
     moves with every write, so this [Unreleased] balance is provisional until the next release \
     keeps its own snapshot.";
const LIMIT_NEVER_JUDGES: &str = "This line reports and never judges: it never says a design does \
     too much. Scope stays the person's call.";

/// A status snapshot read back, or why it could not be.
enum Kept {
    Statuses {
        snapshot_id: String,
        statuses: StatusMap,
    },
    Absent,
    Unreadable(String),
}

/// What the cut is about to do about the status snapshot.
enum SnapshotPlan {
    /// Nothing is written; the report is final.
    Settled(StatusSnapshotReport),
    /// Keep this payload when applying.
    Take {
        payload: StatusSnapshotPayload,
        statuses: usize,
        base_note: Option<String>,
    },
}

fn parse_payload(snapshot: &StoredNode) -> Result<StatusSnapshotPayload, String> {
    let Some(state) = snapshot.properties.get("state").and_then(Value::as_str) else {
        return Err(format!(
            "status snapshot '{}' has no `state` to read",
            snapshot.node_id
        ));
    };
    let payload: StatusSnapshotPayload = serde_json::from_str(state)
        .map_err(|e| format!("status snapshot '{}' does not parse: {e}", snapshot.node_id))?;
    if payload.kind != STATUS_SNAPSHOT_KIND {
        return Err(format!(
            "snapshot '{}' is of kind '{}', not a release's status snapshot",
            snapshot.node_id, payload.kind
        ));
    }
    if payload.format != STATUS_SNAPSHOT_FORMAT {
        return Err(format!(
            "status snapshot '{}' is format {}, and this reflow2 reads format {}",
            snapshot.node_id, payload.format, STATUS_SNAPSHOT_FORMAT
        ));
    }
    Ok(payload)
}

/// Apply one record, oldest first, onto the state built so far.
fn apply_payload(state: &mut StatusMap, payload: &StatusSnapshotPayload) {
    if payload.stored_as == StoredAs::Whole {
        state.clone_from(&payload.set);
    } else {
        for (ty, entries) in &payload.set {
            let at = state.entry(ty.clone()).or_default();
            for (id, entry) in entries {
                at.insert(id.clone(), entry.clone());
            }
        }
        for (ty, ids) in &payload.removed {
            if let Some(at) = state.get_mut(ty) {
                for id in ids {
                    at.remove(id);
                }
            }
        }
    }
    for ty in &payload.covers {
        state.entry(ty.clone()).or_default();
    }
}

fn covers() -> Vec<String> {
    STATUS_SNAPSHOT_COVERS
        .iter()
        .map(|t| (*t).to_string())
        .collect()
}

/// A delta from `base` to `now`: what is new or changed, and what is gone.
fn delta(
    release_id: &str,
    base_release: &str,
    base_snapshot: &str,
    base: &StatusMap,
    now: &StatusMap,
) -> StatusSnapshotPayload {
    let mut set = StatusMap::new();
    for (ty, entries) in now {
        for (id, entry) in entries {
            if base.get(ty).and_then(|m| m.get(id)) != Some(entry) {
                set.entry(ty.clone())
                    .or_default()
                    .insert(id.clone(), entry.clone());
            }
        }
    }
    let mut removed: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (ty, entries) in base {
        for id in entries.keys() {
            if !now.get(ty).is_some_and(|m| m.contains_key(id)) {
                removed.entry(ty.clone()).or_default().push(id.clone());
            }
        }
    }
    StatusSnapshotPayload {
        kind: STATUS_SNAPSHOT_KIND.to_string(),
        format: STATUS_SNAPSHOT_FORMAT,
        release_id: release_id.to_string(),
        stored_as: StoredAs::Delta,
        base_release: Some(base_release.to_string()),
        base_snapshot: Some(base_snapshot.to_string()),
        covers: covers(),
        set,
        removed,
    }
}

fn whole(release_id: &str, now: &StatusMap) -> StatusSnapshotPayload {
    StatusSnapshotPayload {
        kind: STATUS_SNAPSHOT_KIND.to_string(),
        format: STATUS_SNAPSHOT_FORMAT,
        release_id: release_id.to_string(),
        stored_as: StoredAs::Whole,
        base_release: None,
        base_snapshot: None,
        covers: covers(),
        set: now.clone(),
        removed: BTreeMap::new(),
    }
}

fn shown(entry: Option<&StatusEntry>) -> String {
    match entry {
        None => "not in the design".to_string(),
        Some(e) => {
            let status = e.status.as_deref().unwrap_or("no status");
            if e.discontinued {
                format!("{status}, withdrawn by an accepted decision")
            } else {
                status.to_string()
            }
        }
    }
}

/// Whether a node counts as present, per kind of intent.
fn requirement_is_live(e: &StatusEntry) -> bool {
    !e.discontinued && !matches!(e.status.as_deref(), Some("dropped" | "deferred"))
}
fn capability_is_live(e: &StatusEntry) -> bool {
    !e.discontinued
}
fn decision_is_live(e: &StatusEntry) -> bool {
    !e.discontinued && e.status.as_deref() == Some("accepted")
}

impl DesignGraph {
    /// Every Requirement, Capability, Decision and Verification's status as
    /// the design holds it now, each marked when an accepted Decision has
    /// withdrawn it. What a status snapshot stores, and the live end of an
    /// `[Unreleased]` balance.
    pub fn current_statuses(&self) -> Result<StatusMap, DynoError> {
        let gone = self.discontinued_ids()?;
        let mut out = StatusMap::new();
        for ty in STATUS_SNAPSHOT_COVERS {
            let mut by_id = BTreeMap::new();
            for n in self.scan_nodes(ty)? {
                let status = n
                    .properties
                    .get("status")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let discontinued = gone.contains(&n.node_id);
                by_id.insert(
                    n.node_id,
                    StatusEntry {
                        status,
                        discontinued,
                    },
                );
            }
            out.insert(ty.to_string(), by_id);
        }
        Ok(out)
    }

    /// The statuses `release_id` saw at its cut, read back through its delta
    /// chain. `None` when the release keeps no status snapshot. A snapshot
    /// that cannot be read back (a missing base, a format this build does not
    /// read) is an error naming why, never an empty answer.
    pub fn release_statuses(&self, release_id: &str) -> Result<Option<StatusMap>, DynoError> {
        match self.kept_statuses(release_id)? {
            Kept::Statuses { statuses, .. } => Ok(Some(statuses)),
            Kept::Absent => Ok(None),
            Kept::Unreadable(why) => Err(DynoError::Query(why)),
        }
    }

    fn kept_statuses(&self, release_id: &str) -> Result<Kept, DynoError> {
        let first = status_snapshot_id(release_id);
        let Some(node) = self.get_node(node::SNAPSHOT, &first)? else {
            return Ok(Kept::Absent);
        };
        // Walk back to the whole snapshot the chain starts from.
        let mut chain: Vec<StatusSnapshotPayload> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut at = Some(node);
        while let Some(snapshot) = at.take() {
            if !seen.insert(snapshot.node_id.clone()) {
                return Ok(Kept::Unreadable(format!(
                    "reading back '{first}' reaches '{}' twice, so its chain of deltas is a \
                     cycle",
                    snapshot.node_id
                )));
            }
            let payload = match parse_payload(&snapshot) {
                Ok(p) => p,
                Err(why) => return Ok(Kept::Unreadable(why)),
            };
            if payload.stored_as == StoredAs::Delta {
                let Some(base_id) = payload.base_snapshot.clone() else {
                    return Ok(Kept::Unreadable(format!(
                        "status snapshot '{}' is a delta that names no base",
                        snapshot.node_id
                    )));
                };
                let Some(base) = self.get_node(node::SNAPSHOT, &base_id)? else {
                    return Ok(Kept::Unreadable(format!(
                        "status snapshot '{}' is a delta from '{base_id}', which is no longer in \
                         the design",
                        snapshot.node_id
                    )));
                };
                at = Some(base);
            }
            chain.push(payload);
        }
        let mut statuses = StatusMap::new();
        for payload in chain.iter().rev() {
            apply_payload(&mut statuses, payload);
        }
        Ok(Kept::Statuses {
            snapshot_id: first,
            statuses,
        })
    }

    /// The epoch sequence a Release is pinned to (`AT_EPOCH`), if any.
    fn release_epoch_sequence(&self, release_id: &str) -> Result<Option<i64>, DynoError> {
        for e in self.outgoing(release_id, Some(edge::AT_EPOCH))? {
            if let Some(ep) = self.get_node(node::DESIGN_EPOCH, &e.to_id)? {
                return Ok(ep.properties.get("sequence").and_then(Value::as_i64));
            }
        }
        Ok(None)
    }

    /// Every OTHER release that keeps a status snapshot and is pinned to an
    /// epoch, as `(sequence, release id)`, sorted.
    fn releases_keeping_a_snapshot(&self, except: &str) -> Result<Vec<(i64, String)>, DynoError> {
        let mut out = Vec::new();
        for rel in self.scan_nodes(node::RELEASE)? {
            if rel.node_id == except
                || self
                    .get_node(node::SNAPSHOT, &status_snapshot_id(&rel.node_id))?
                    .is_none()
            {
                continue;
            }
            if let Some(seq) = self.release_epoch_sequence(&rel.node_id)? {
                out.push((seq, rel.node_id));
            }
        }
        out.sort();
        Ok(out)
    }

    /// Decide what the cut does about `release_id`'s status snapshot. Reads
    /// only. `manifest_was_empty` is whether the release's manifest held
    /// nothing before this call.
    fn plan_status_snapshot(
        &self,
        release_id: &str,
        manifest_was_empty: bool,
    ) -> Result<SnapshotPlan, DynoError> {
        let snapshot_id = status_snapshot_id(release_id);
        let settled = |outcome, note: String| StatusSnapshotReport {
            snapshot_id: snapshot_id.clone(),
            outcome,
            stored_as: None,
            base_release: None,
            entries: None,
            statuses: None,
            note,
        };

        if let Some(existing) = self.get_node(node::SNAPSHOT, &snapshot_id)? {
            let parsed = parse_payload(&existing);
            let read_back = self.kept_statuses(release_id)?;
            let mut report = settled(StatusSnapshotOutcome::AlreadyKept, String::new());
            if let Ok(p) = &parsed {
                report.stored_as = Some(p.stored_as);
                report.base_release.clone_from(&p.base_release);
                report.entries = Some(p.entries());
            }
            let mut note = format!(
                "This release already keeps its status snapshot '{snapshot_id}'{}, and it is left \
                 exactly as it was: what a release saw must not move, as its manifest's frozen \
                 checksums do not.",
                report
                    .entries
                    .map(|n| format!(" ({n} entries)"))
                    .unwrap_or_default()
            );
            match read_back {
                Kept::Statuses { statuses, .. } => report.statuses = Some(status_count(&statuses)),
                Kept::Unreadable(why) => note.push_str(&format!(" It cannot be read back: {why}.")),
                Kept::Absent => {}
            }
            report.note = note;
            return Ok(SnapshotPlan::Settled(report));
        }

        if !manifest_was_empty {
            return Ok(SnapshotPlan::Settled(settled(
                StatusSnapshotOutcome::NotTaken,
                "No status snapshot was taken: this release's manifest was frozen before this \
                 call, so its cut is past, and a snapshot taken now would record today's statuses \
                 as the ones it saw. A past release is back-filled from the design copy committed \
                 at its tag, which is a named follow-up of dec:how-the-changelog-balance-is-counted; \
                 until then changelog_view names this release as keeping none."
                    .to_string(),
            )));
        }

        let own_sequence = self.release_epoch_sequence(release_id)?;
        let kept = self.releases_keeping_a_snapshot(release_id)?;
        if let Some(own) = own_sequence
            && let Some((seq, later)) = kept.iter().rev().find(|(s, _)| *s > own)
        {
            return Ok(SnapshotPlan::Settled(settled(
                StatusSnapshotOutcome::NotTaken,
                format!(
                    "No status snapshot was taken: a later release, '{later}' (epoch sequence \
                     {seq}, after this release's {own}), already keeps one, so a snapshot taken \
                     now would record today's statuses as what this earlier release saw."
                ),
            )));
        }

        // The previous release: the highest epoch sequence below this one
        // that keeps a snapshot. A release not yet pinned (the cut pins it at
        // the next step) is taken as the newest.
        let base = kept
            .iter()
            .rev()
            .find(|(s, _)| own_sequence.is_none_or(|own| *s < own));
        let now = self.current_statuses()?;
        let statuses = status_count(&now);
        let mut base_note = None;
        let payload = match base {
            None => whole(release_id, &now),
            Some((seq, base_release)) => match self.kept_statuses(base_release)? {
                Kept::Statuses {
                    snapshot_id: base_snapshot,
                    statuses: base_state,
                } => {
                    if own_sequence.is_none() {
                        base_note = Some(format!(
                            " This release is not pinned to an epoch yet (the cut pins it next), \
                             so it is taken as the newest: its base is '{base_release}', the \
                             release with the highest epoch sequence ({seq}) that keeps one."
                        ));
                    }
                    delta(release_id, base_release, &base_snapshot, &base_state, &now)
                }
                Kept::Unreadable(why) => {
                    base_note = Some(format!(
                        " Stored whole rather than as a delta from '{base_release}', because its \
                         snapshot cannot be read back: {why}."
                    ));
                    whole(release_id, &now)
                }
                Kept::Absent => whole(release_id, &now),
            },
        };
        Ok(SnapshotPlan::Take {
            payload,
            statuses,
            base_note,
        })
    }

    /// The status snapshot half of `release_includes_all`: plan it before
    /// anything is written. [`Self::settle_status_snapshot`] writes it.
    pub(crate) fn prepare_status_snapshot(
        &self,
        release_id: &str,
        manifest_was_empty: bool,
    ) -> Result<PreparedStatusSnapshot, DynoError> {
        Ok(PreparedStatusSnapshot {
            release_id: release_id.to_string(),
            plan: self.plan_status_snapshot(release_id, manifest_was_empty)?,
        })
    }

    /// Write the prepared snapshot when `apply`, and say what was done. Run
    /// inside the same batch as the manifest's edges.
    pub(crate) fn settle_status_snapshot(
        &mut self,
        prepared: PreparedStatusSnapshot,
        apply: bool,
    ) -> Result<StatusSnapshotReport, DynoError> {
        let snapshot_id = status_snapshot_id(&prepared.release_id);
        let (payload, statuses, base_note) = match prepared.plan {
            SnapshotPlan::Settled(report) => return Ok(report),
            SnapshotPlan::Take {
                payload,
                statuses,
                base_note,
            } => (payload, statuses, base_note),
        };
        let entries = payload.entries();
        let how_stored = match (&payload.stored_as, &payload.base_release) {
            (StoredAs::Delta, Some(base)) => format!(
                "stored as a delta from the snapshot of '{base}', holding {entries} entr{}",
                if entries == 1 { "y" } else { "ies" }
            ),
            _ => format!("stored whole as {entries} entries"),
        };
        let mut report = StatusSnapshotReport {
            snapshot_id: snapshot_id.clone(),
            outcome: StatusSnapshotOutcome::DryRun,
            stored_as: Some(payload.stored_as),
            base_release: payload.base_release.clone(),
            entries: Some(entries),
            statuses: Some(statuses),
            note: String::new(),
        };
        if !apply {
            report.note = format!(
                "Dry run, nothing written. With apply: true this release keeps its status \
                 snapshot '{snapshot_id}': {statuses} statuses of requirements, capabilities, \
                 decisions and checks, {how_stored}.{}",
                base_note.unwrap_or_default()
            );
            return Ok(report);
        }
        let state = serde_json::to_string(&payload)
            .map_err(|e| DynoError::Serialization(format!("status snapshot payload: {e}")))?;
        self.create_node(
            node::SNAPSHOT,
            &snapshot_id,
            Props::new()
                .set("target_id", prepared.release_id.as_str())
                .set("target_type", node::RELEASE)
                .set("state", state),
        )?;
        report.outcome = StatusSnapshotOutcome::Written;
        report.note = format!(
            "Kept this release's status snapshot '{snapshot_id}': {statuses} statuses of \
             requirements, capabilities, decisions and checks, each marked when an accepted \
             decision has withdrawn it, {how_stored}. changelog_view reads it as one end of the \
             balance between two releases, and it is never rewritten.{}",
            base_note.unwrap_or_default()
        );
        Ok(report)
    }

    /// The balance between two ends of a changelog window. `from` and `to`
    /// are each `(point id, release id when the point is a Release)`;
    /// `to_is_live` is `[Unreleased]`'s later end, the design as it stands.
    /// `epochs_in_window` is the window `changelog_view` computed.
    pub(crate) fn changelog_balance(
        &self,
        from: Option<(&str, Option<&str>)>,
        to: Option<(&str, Option<&str>)>,
        to_is_live: bool,
        epochs_in_window: &[String],
    ) -> Result<ChangelogBalance, DynoError> {
        let mut notes = Vec::new();
        let (from_end, before) = self.balance_end("earlier", from, false, &mut notes)?;
        let (to_end, after) = self.balance_end("later", to, to_is_live, &mut notes)?;
        let from_label = from_end
            .id
            .clone()
            .unwrap_or_else(|| "the earlier end".to_string());
        let to_label = to_end
            .id
            .clone()
            .unwrap_or_else(|| "the live design".to_string());

        let failures_that_came_back =
            self.failures_in_window(epochs_in_window, &from_label, &to_label)?;

        let mut limits = vec![LIMIT_NET.to_string(), LIMIT_WHICH.to_string()];
        if to_is_live {
            limits.push(LIMIT_LIVE.to_string());
        }
        limits.push(LIMIT_NEVER_JUDGES.to_string());

        let (Some(a), Some(b)) = (before, after) else {
            return Ok(ChangelogBalance {
                from: from_end,
                to: to_end,
                counted: false,
                requirements: None,
                capabilities: None,
                decisions: None,
                checks_now_failing: None,
                failures_that_came_back,
                limits,
                notes,
            });
        };

        let (f, t) = (from_label.as_str(), to_label.as_str());
        let requirements = self.balance_pair(
            node::REQUIREMENT,
            &a,
            &b,
            requirement_is_live,
            (f, t),
            format!(
                "Requirements live at {t} that were not at {f}: new since {f}, or back from \
                 dropped, deferred or withdrawn. Live means any status but dropped or deferred, \
                 and not withdrawn by an accepted decision."
            ),
            format!(
                "Requirements live at {f} that are not at {t}: dropped, deferred, withdrawn by an \
                 accepted decision (a stored status does not move on withdrawal), or no longer in \
                 the design."
            ),
        )?;
        let capabilities = self.balance_pair(
            node::CAPABILITY,
            &a,
            &b,
            capability_is_live,
            (f, t),
            format!(
                "Capabilities in the design and not withdrawn at {t} that were not at {f}: new, \
                 or no longer withdrawn."
            ),
            format!(
                "Capabilities in the design and not withdrawn at {f} that, at {t}, an accepted \
                 decision has withdrawn (a capability's status records what was built and does \
                 not move when it is withdrawn) or that are no longer in the design."
            ),
        )?;
        let decisions = self.balance_pair(
            node::DECISION,
            &a,
            &b,
            decision_is_live,
            (f, t),
            format!(
                "Decisions accepted at {t} that were not at {f}: new, or moved to accepted. \
                 Every Decision counts, gap acknowledgements (decision:ack:…) included."
            ),
            format!(
                "Decisions accepted at {f} that are not at {t}: superseded or rejected, withdrawn \
                 by an accepted decision, moved back to proposed or deferred, or no longer in the \
                 design. Each item's `how` says which."
            ),
        )?;
        let checks_now_failing = self.checks_now_failing(&a, &b, (f, t))?;

        Ok(ChangelogBalance {
            from: from_end,
            to: to_end,
            counted: true,
            requirements: Some(requirements),
            capabilities: Some(capabilities),
            decisions: Some(decisions),
            checks_now_failing: Some(checks_now_failing),
            failures_that_came_back,
            limits,
            notes,
        })
    }

    /// Resolve one end to the statuses it gives, naming it in `notes` when it
    /// gives none.
    fn balance_end(
        &self,
        which: &str,
        point: Option<(&str, Option<&str>)>,
        live: bool,
        notes: &mut Vec<String>,
    ) -> Result<(BalanceEnd, Option<StatusMap>), DynoError> {
        let not_kept = |id: Option<&str>| BalanceEnd {
            id: id.map(str::to_string),
            source: BalanceSource::NotKept,
            snapshot_id: None,
        };
        const ABSENT: &str = "so the balance counts nothing: its counts are absent, not zero.";
        if live {
            return Ok((
                BalanceEnd {
                    id: None,
                    source: BalanceSource::LiveDesign,
                    snapshot_id: None,
                },
                Some(self.current_statuses()?),
            ));
        }
        match point {
            None => {
                notes.push(format!(
                    "The balance has no {which} end (it resolved to no release), {ABSENT}"
                ));
                Ok((not_kept(None), None))
            }
            Some((id, None)) => {
                notes.push(format!(
                    "'{id}' is a DesignEpoch, and only a Release keeps a status snapshot, {ABSENT}"
                ));
                Ok((not_kept(Some(id)), None))
            }
            Some((id, Some(release))) => match self.kept_statuses(release)? {
                Kept::Statuses {
                    snapshot_id,
                    statuses,
                } => Ok((
                    BalanceEnd {
                        id: Some(id.to_string()),
                        source: BalanceSource::Snapshot,
                        snapshot_id: Some(snapshot_id),
                    },
                    Some(statuses),
                )),
                Kept::Absent => {
                    notes.push(format!(
                        "'{id}' keeps no status snapshot (it was cut before releases kept one, or \
                         its manifest was never derived with release_includes_all apply: true), \
                         {ABSENT} Back-filling a past release from the design copy committed at \
                         its tag is a named follow-up."
                    ));
                    Ok((not_kept(Some(id)), None))
                }
                Kept::Unreadable(why) => {
                    notes.push(format!(
                        "The status snapshot of '{id}' cannot be read back ({why}), {ABSENT}"
                    ));
                    Ok((not_kept(Some(id)), None))
                }
            },
        }
    }

    fn balance_item(
        &self,
        node_type: &str,
        id: &str,
        before: Option<&StatusEntry>,
        after: Option<&StatusEntry>,
        how: String,
    ) -> Result<BalanceItem, DynoError> {
        let name = self.get_node(node_type, id)?.and_then(|n| {
            n.properties
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string)
        });
        Ok(BalanceItem {
            id: id.to_string(),
            node_type: node_type.to_string(),
            name,
            before: before.and_then(|e| e.status.clone()),
            after: after.and_then(|e| e.status.clone()),
            how,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn balance_pair(
        &self,
        node_type: &str,
        a: &StatusMap,
        b: &StatusMap,
        is_live: fn(&StatusEntry) -> bool,
        (f, t): (&str, &str),
        added_how: String,
        taken_how: String,
    ) -> Result<BalancePair, DynoError> {
        let empty = BTreeMap::new();
        let before_all = a.get(node_type).unwrap_or(&empty);
        let after_all = b.get(node_type).unwrap_or(&empty);
        let ids: BTreeSet<&String> = before_all.keys().chain(after_all.keys()).collect();
        let mut added = Vec::new();
        let mut taken_away = Vec::new();
        for id in ids {
            let before = before_all.get(id);
            let after = after_all.get(id);
            let was = before.is_some_and(is_live);
            let is = after.is_some_and(is_live);
            if !was && is {
                let how = match before {
                    None => format!("new since {f}: {} at {t}", shown(after)),
                    Some(_) => format!("{} at {f}, {} at {t}", shown(before), shown(after)),
                };
                added.push(self.balance_item(node_type, id, before, after, how)?);
            } else if was && !is {
                let how = match after {
                    None => format!(
                        "no longer in the design at {t}; it was {} at {f}",
                        shown(before)
                    ),
                    Some(e) if e.discontinued => format!(
                        "withdrawn by an accepted decision by {t}; its stored status still \
                         reads {}",
                        e.status.as_deref().unwrap_or("no status")
                    ),
                    Some(_) => format!("{} at {f}, {} at {t}", shown(before), shown(after)),
                };
                taken_away.push(self.balance_item(node_type, id, before, after, how)?);
            }
        }
        Ok(BalancePair {
            added: BalanceColumn {
                count: added.len(),
                how_counted: added_how,
                items: added,
            },
            taken_away: BalanceColumn {
                count: taken_away.len(),
                how_counted: taken_how,
                items: taken_away,
            },
        })
    }

    fn checks_now_failing(
        &self,
        a: &StatusMap,
        b: &StatusMap,
        (f, t): (&str, &str),
    ) -> Result<BalanceColumn, DynoError> {
        let mut items = Vec::new();
        if let (Some(before_all), Some(after_all)) =
            (a.get(node::VERIFICATION), b.get(node::VERIFICATION))
        {
            for (id, before) in before_all {
                let Some(after) = after_all.get(id) else {
                    continue;
                };
                if before.status.as_deref() == Some("passing")
                    && after.status.as_deref() == Some("failing")
                {
                    let how = format!("passing at {f}, failing at {t}");
                    items.push(self.balance_item(
                        node::VERIFICATION,
                        id,
                        Some(before),
                        Some(after),
                        how,
                    )?);
                }
            }
        }
        Ok(BalanceColumn {
            count: items.len(),
            how_counted: format!(
                "Verifications passing at {f} and failing at {t}. A check that failed and was \
                 fixed inside the window reads as neither, and one new since {f} that fails is \
                 not counted here."
            ),
            items,
        })
    }

    /// The window's `test_failure_fix` ChangeEvents: fixes a check caught.
    fn failures_in_window(
        &self,
        epochs_in_window: &[String],
        f: &str,
        t: &str,
    ) -> Result<BalanceColumn, DynoError> {
        let mut by_id: BTreeMap<String, BalanceItem> = BTreeMap::new();
        for epoch_id in epochs_in_window {
            for pin in self.incoming(epoch_id, Some(edge::AT_EPOCH))? {
                let Some(ev) = self.get_node(node::CHANGE_EVENT, &pin.from_id)? else {
                    continue;
                };
                if ev.properties.get("change_type").and_then(Value::as_str)
                    != Some("test_failure_fix")
                {
                    continue;
                }
                let name = ev
                    .properties
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                by_id.entry(ev.node_id.clone()).or_insert(BalanceItem {
                    id: ev.node_id.clone(),
                    node_type: node::CHANGE_EVENT.to_string(),
                    name,
                    before: None,
                    after: None,
                    how: format!("a test_failure_fix change pinned to '{epoch_id}'"),
                });
            }
        }
        let items: Vec<BalanceItem> = by_id.into_values().collect();
        Ok(BalanceColumn {
            count: items.len(),
            how_counted: format!(
                "ChangeEvents whose change_type is test_failure_fix, pinned (AT_EPOCH) to an \
                 epoch after {f} and up to {t}: fixes a check caught. An event pinned to no epoch \
                 is in no window and is not seen. Counted from the window, so given even when an \
                 end keeps no snapshot."
            ),
            items,
        })
    }
}

/// A status snapshot planned by `release_includes_all` before it writes.
pub(crate) struct PreparedStatusSnapshot {
    release_id: String,
    plan: SnapshotPlan,
}
