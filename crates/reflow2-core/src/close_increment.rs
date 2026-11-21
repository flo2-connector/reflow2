//! CLOSING AN INCREMENT HONESTLY: an increment that will not be finished is
//! closed as `deferred`, `dropped` or `superseded`, on the owner's word, and
//! every unfinished item scheduled for it is answered for.
//!
//! req:an-increment-that-will-not-be-finished-can-be-closed-honestly
//! (Anthony, 2026-11-12), with the two choices he made on 2026-11-17
//! (dec:a-deferred-increment-may-keep-its-items-and-a-dropped-leftover-is-only-unscheduled-2026-11-17):
//!
//! - **deferred**: still meant, later. Each unfinished item STAYS with it,
//!   MOVES to another increment, or is DROPPED from it. Staying is an answer
//!   given per item, never a default. `set_epoch_status planned` reopens it.
//! - **dropped**: no longer meant, with the reason. Each unfinished item moves
//!   or is dropped from it.
//! - **superseded**: by another named increment. Each unfinished item moves
//!   (most often to that one) or is dropped from it.
//!
//! Without this an increment that will not be finished can only sit `planned`
//! for ever, listed as open at every session's start and end, or be marked
//! `arrived`, which is false. A reminder with no honest answer pushes the
//! person to the false one.
//!
//! # What a leftover's answer writes
//!
//! - **moves**: its `SCHEDULED_FOR` to the closing increment is removed and
//!   one to the named increment drawn, with the same modality. That is the
//!   schedule's own way to slip (`schedule_for`: "record the change against the
//!   epoch rather than re-pointing this edge"): the plan is snapshotted first,
//!   so `arrival_delta` on the closed increment reads the item as deferred to
//!   where it went, and the slip is on the record.
//! - **dropped**: its `SCHEDULED_FOR` to the closing increment is removed,
//!   after the same snapshot, and nothing else. `arrival_delta` reads it
//!   discontinued there. The requirement or capability ITSELF keeps its
//!   status: retiring intent stays its own act (set_requirement_status,
//!   retire-from-design), so closing a plan never retires intent as a side
//!   effect (the owner's choice (2) of 2026-11-17).
//! - **stays** (a deferral only): nothing moves.
//!
//! # One record of the close
//!
//! The close writes ONE ChangeEvent (`scope_change`), dated by the owner's
//! word, that says how the increment closed and the fate of every leftover
//! with its reason, and CHANGED edges to the increment, to each item that
//! moved or was dropped, and to each increment that received one. The
//! reasons live there, searchable; the increment keeps only `close_reason`
//! and `superseded_by`, which a reopening clears.
//!
//! # Refuse first, write whole
//!
//! Every check runs before the first write, and every leftover problem is
//! named in one refusal, so a caller learns all of them at once and a refusal
//! leaves nothing half-closed.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::foundation::core::{DynoError, Value};
use crate::foundation::store::StoredNode;
use crate::graph::DesignGraph;
use crate::nodes::{Props, edge, node};
use crate::temporal::{ChangeAction, ChangeEventRecord, ChangeType, ScheduleOutcome};

/// How an increment closes without arriving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAs {
    /// Still meant, later.
    Deferred,
    /// No longer meant, with the reason.
    Dropped,
    /// Another named increment took its place.
    Superseded,
}

impl CloseAs {
    /// The status word it stores.
    pub fn as_str(self) -> &'static str {
        match self {
            CloseAs::Deferred => "deferred",
            CloseAs::Dropped => "dropped",
            CloseAs::Superseded => "superseded",
        }
    }

    /// Parse the status word; `None` for anything else.
    pub fn parse(s: &str) -> Option<CloseAs> {
        match s {
            "deferred" => Some(CloseAs::Deferred),
            "dropped" => Some(CloseAs::Dropped),
            "superseded" => Some(CloseAs::Superseded),
            _ => None,
        }
    }
}

/// Is this a status an increment can only reach by being CLOSED without
/// arriving? The one list, read by `set_epoch_status` (which refuses to write
/// one) and by every reader that must not take a closed increment for one
/// that happened.
pub fn is_closed_status(status: &str) -> bool {
    CloseAs::parse(status).is_some()
}

/// What becomes of one unfinished item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeftoverFate {
    /// It stays with the deferred increment. Only on a deferral.
    Stays,
    /// It moves to this other increment, a recorded slip.
    Moves {
        /// The DesignEpoch it is now scheduled for.
        to: String,
    },
    /// It is dropped from this increment, with the reason. The item itself
    /// keeps its status.
    Dropped {
        /// Why, in the owner's words.
        reason: String,
    },
}

/// One unfinished item and the owner's answer for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leftover {
    /// The scheduled item's id.
    pub item_id: String,
    /// What becomes of it.
    pub fate: LeftoverFate,
}

/// Everything one close says.
#[derive(Debug, Clone)]
pub struct IncrementClose<'a> {
    /// The planned increment being closed.
    pub epoch_id: &'a str,
    /// How it closes.
    pub how: CloseAs,
    /// Why. Required for `dropped`.
    pub reason: Option<&'a str>,
    /// The increment that took its place. Required for `superseded`, refused
    /// otherwise.
    pub superseded_by: Option<&'a str>,
    /// One answer per unfinished item scheduled for it.
    pub leftovers: &'a [Leftover],
    /// An ARRIVED epoch to file the plan's snapshot in, when an item moves or
    /// is dropped and the plan is not on the record yet. A snapshot records
    /// the present, so it is filed in a point that has happened.
    pub recorded_in: Option<&'a str>,
    /// The Contributor whose word closes it.
    pub approver: &'a str,
    /// When they gave it, as a plain date.
    pub acted_at: Option<&'a str>,
}

/// What a close wrote.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ClosedIncrement {
    /// The increment as it now stands.
    pub epoch_id: String,
    /// Its new status.
    pub status: &'static str,
    /// The ChangeEvent recording the close.
    pub change_event_id: String,
    /// The snapshot the close took of the plan, when it needed one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
    /// How many items scheduled for it were already delivered (nothing to
    /// answer for).
    pub delivered: usize,
    /// How many were withdrawn questions (nothing to answer for).
    pub withdrawn: usize,
    /// What became of each unfinished item, in the order given.
    pub leftovers: Vec<LeftoverOutcome>,
}

/// One leftover's recorded fate.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LeftoverOutcome {
    pub item_id: String,
    pub item_type: String,
    /// `stays`, `moves` or `dropped`.
    pub fate: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The unfinished items scheduled for an increment, as `(item_type, item_id,
/// modality)`, with how many were delivered and how many withdrawn.
pub type Unfinished = (Vec<(String, String, String)>, usize, usize);

fn refuse(property: &str, message: String) -> DynoError {
    DynoError::Validation {
        node_type: node::DESIGN_EPOCH.into(),
        property: property.into(),
        message,
    }
}

fn status_of(n: &StoredNode) -> &str {
    n.properties
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("arrived")
}

fn nonblank(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|s| !s.is_empty())
}

impl DesignGraph {
    /// The unfinished items scheduled for `epoch_id` now: each one
    /// `arrival_delta` would read outstanding, as `(item_type, item_id,
    /// modality)`, with the counts of what needs no answer (delivered,
    /// withdrawn). The same reading `open_increments` and `arrival_delta`
    /// make, never a second derivation of delivery.
    pub fn unfinished_items(&self, epoch_id: &str) -> Result<Unfinished, DynoError> {
        let index = self.node_type_index()?;
        let mut out = Vec::new();
        let (mut delivered, mut withdrawn) = (0, 0);
        for item in self.scheduled_now(&index, epoch_id)? {
            match item.outcome {
                ScheduleOutcome::Delivered => delivered += 1,
                ScheduleOutcome::Discontinued => withdrawn += 1,
                _ => out.push((item.item_type, item.item_id, item.modality)),
            }
        }
        Ok((out, delivered, withdrawn))
    }

    /// Close a planned increment without it arriving — see the module docs.
    /// Refuses, writing nothing, unless every unfinished item is answered for
    /// exactly once and every name it gives resolves.
    pub fn close_increment(
        &mut self,
        c: &IncrementClose<'_>,
    ) -> Result<ClosedIncrement, DynoError> {
        // ---- 1. The increment itself ------------------------------------
        let Some(epoch) = self.get_node(node::DESIGN_EPOCH, c.epoch_id)? else {
            return Err(DynoError::NodeNotFound {
                node_type: node::DESIGN_EPOCH.into(),
                node_id: c.epoch_id.into(),
            });
        };
        let now = status_of(&epoch).to_string();
        if now != "planned" {
            let hint = if is_closed_status(&now) {
                "To close it another way, reopen it first with set_epoch_status `planned`."
            } else {
                "It has happened, so there is nothing left to close: arrival_delta says what \
                 it delivered, and an item still owed there is moved with schedule_for."
            };
            return Err(refuse(
                "status",
                format!(
                    "'{}' reads `{now}`, and only an open increment (`planned`) can be closed. \
                     {hint}",
                    c.epoch_id
                ),
            ));
        }
        let reason = nonblank(c.reason);
        if c.how == CloseAs::Dropped && reason.is_none() {
            return Err(refuse(
                "close_reason",
                format!(
                    "dropping '{}' needs the reason it is no longer meant, in the owner's words \
                     (`reason`). A drop with no reason is the silent retirement the close exists \
                     to replace.",
                    c.epoch_id
                ),
            ));
        }
        let superseded_by = nonblank(c.superseded_by);
        match (c.how, superseded_by) {
            (CloseAs::Superseded, None) => {
                return Err(refuse(
                    "superseded_by",
                    format!(
                        "superseding '{}' needs the increment that took its place \
                         (`superseded_by`, a DesignEpoch id).",
                        c.epoch_id
                    ),
                ));
            }
            (CloseAs::Superseded, Some(by)) => {
                if by == c.epoch_id {
                    return Err(refuse(
                        "superseded_by",
                        format!("'{by}' cannot supersede itself."),
                    ));
                }
                let Some(successor) = self.get_node(node::DESIGN_EPOCH, by)? else {
                    return Err(refuse(
                        "superseded_by",
                        format!(
                            "no DesignEpoch '{by}' in this design, so it cannot be what \
                             superseded '{}'. Plan it with plan_epoch first, or check the id.",
                            c.epoch_id
                        ),
                    ));
                };
                let s = status_of(&successor);
                if is_closed_status(s) {
                    return Err(refuse(
                        "superseded_by",
                        format!(
                            "'{by}' is itself closed (`{s}`), so it took nobody's place. Name \
                             the increment that carries the work on."
                        ),
                    ));
                }
            }
            (_, Some(_)) => {
                return Err(refuse(
                    "superseded_by",
                    format!(
                        "`superseded_by` names a successor, which only a `superseded` close \
                         has; '{}' is being closed as `{}`.",
                        c.epoch_id,
                        c.how.as_str()
                    ),
                ));
            }
            (_, None) => {}
        }

        // ---- 2. Every leftover answered for, exactly once ----------------
        let (unfinished, delivered, withdrawn) = self.unfinished_items(c.epoch_id)?;
        let owed: BTreeMap<&str, (&str, &str)> = unfinished
            .iter()
            .map(|(t, id, m)| (id.as_str(), (t.as_str(), m.as_str())))
            .collect();
        let scheduled_here: HashSet<String> = self
            .incoming(c.epoch_id, Some(edge::SCHEDULED_FOR))?
            .into_iter()
            .map(|e| e.from_id)
            .collect();
        let mut problems: Vec<String> = Vec::new();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut destinations: HashMap<&str, ()> = HashMap::new();
        for l in c.leftovers {
            let id = l.item_id.as_str();
            if !seen.insert(id) {
                problems.push(format!("'{id}' is answered for twice; give it one answer"));
                continue;
            }
            if !owed.contains_key(id) {
                problems.push(if scheduled_here.contains(id) {
                    format!(
                        "'{id}' is scheduled for '{}' but is not unfinished (it is delivered, \
                         or a withdrawn question), so there is nothing to answer for it; leave \
                         it out",
                        c.epoch_id
                    )
                } else {
                    format!("'{id}' is not scheduled for '{}'", c.epoch_id)
                });
                continue;
            }
            match &l.fate {
                LeftoverFate::Stays => {
                    if c.how != CloseAs::Deferred {
                        problems.push(format!(
                            "'{id}' cannot stay: only a deferred increment keeps its items, and \
                             '{}' is being closed as `{}`. Move it or drop it from the increment",
                            c.epoch_id,
                            c.how.as_str()
                        ));
                    }
                }
                LeftoverFate::Moves { to } => {
                    let to = to.trim();
                    if to.is_empty() {
                        problems.push(format!("'{id}' moves, but to no increment (`to`)"));
                    } else if to == c.epoch_id {
                        problems.push(format!(
                            "'{id}' cannot move to '{to}', the increment being closed"
                        ));
                    } else {
                        match self.get_node(node::DESIGN_EPOCH, to)? {
                            None => problems.push(format!(
                                "'{id}' moves to '{to}', which is no DesignEpoch in this design"
                            )),
                            Some(dest) if status_of(&dest) != "planned" => problems.push(format!(
                                "'{id}' moves to '{to}', which reads `{}`: an item moves to an \
                                 open (`planned`) increment, where it can still be delivered",
                                status_of(&dest)
                            )),
                            Some(_) => {
                                destinations.insert(to, ());
                            }
                        }
                    }
                }
                LeftoverFate::Dropped { reason } => {
                    if reason.trim().is_empty() {
                        problems.push(format!(
                            "'{id}' is dropped from the increment with no reason; say why in \
                             the owner's words"
                        ));
                    }
                }
            }
        }
        let unanswered: Vec<&str> = owed
            .keys()
            .copied()
            .filter(|id| !seen.contains(id))
            .collect();
        if !unanswered.is_empty() {
            let ways = if c.how == CloseAs::Deferred {
                "stays, moves (to an open increment) or dropped (with the reason)"
            } else {
                "moves (to an open increment) or dropped (with the reason)"
            };
            problems.push(format!(
                "{} unfinished item(s) have no answer: {}. None is defaulted: each one {ways}",
                unanswered.len(),
                unanswered.join(", ")
            ));
        }
        if !problems.is_empty() {
            return Err(refuse(
                "leftovers",
                format!(
                    "'{}' was not closed, and nothing was written: {}.",
                    c.epoch_id,
                    problems.join("; ")
                ),
            ));
        }

        // ---- 3. The plan on the record before any of it moves ------------
        let anything_leaves = c
            .leftovers
            .iter()
            .any(|l| !matches!(l.fate, LeftoverFate::Stays));
        let mut snapshot_id = None;
        let needs_snapshot = anything_leaves && !self.schedule_is_recorded(c.epoch_id)?;
        let recorded_in = nonblank(c.recorded_in);
        if let Some(at) = recorded_in {
            match self.get_node(node::DESIGN_EPOCH, at)? {
                Some(e) if status_of(&e) == "arrived" => {}
                Some(e) => {
                    return Err(refuse(
                        "recorded_in",
                        format!(
                            "'{at}' reads `{}`, so it has not happened, and a snapshot of the \
                             present cannot be filed in it, nor the close recorded there. Name \
                             an arrived epoch.",
                            status_of(&e)
                        ),
                    ));
                }
                None => {
                    return Err(refuse(
                        "recorded_in",
                        format!("no DesignEpoch '{at}' in this design to record the close in."),
                    ));
                }
            }
        } else if needs_snapshot {
            return Err(refuse(
                "recorded_in",
                format!(
                    "items leave '{}', and its plan is not on the record yet, so moving them \
                     would rewrite it silently. Name an ARRIVED epoch to file the plan's \
                     snapshot in (`recorded_in`); nothing else is asked.",
                    c.epoch_id
                ),
            ));
        }
        // The signature is checked before the first write, so a refused
        // signer leaves nothing half-closed (`crate::intent::Signer`).
        self.may_sign(c.approver, "approver")?;
        if self.get_node(node::CONTRIBUTOR, c.approver)?.is_none() {
            return Err(refuse(
                "approver",
                format!(
                    "no Contributor '{}' in this design, so its word cannot close '{}'.",
                    c.approver, c.epoch_id
                ),
            ));
        }

        // ---- 4. Write: snapshot, leftovers, the close, its record --------
        if let (true, Some(at)) = (needs_snapshot, recorded_in) {
            let snap = self.snapshot_node(at, node::DESIGN_EPOCH, c.epoch_id)?;
            snapshot_id = Some(snap.node_id);
        }
        let mut outcomes = Vec::with_capacity(c.leftovers.len());
        for l in c.leftovers {
            let (item_type, modality) = owed[l.item_id.as_str()];
            match &l.fate {
                LeftoverFate::Stays => outcomes.push(LeftoverOutcome {
                    item_id: l.item_id.clone(),
                    item_type: item_type.to_string(),
                    fate: "stays",
                    to: None,
                    reason: None,
                }),
                LeftoverFate::Moves { to } => {
                    let to = to.trim();
                    let already = self
                        .outgoing(&l.item_id, Some(edge::SCHEDULED_FOR))?
                        .iter()
                        .any(|e| e.to_id == to);
                    if !already {
                        self.schedule_for(
                            item_type,
                            &l.item_id,
                            node::DESIGN_EPOCH,
                            to,
                            modality,
                            c.acted_at,
                        )?;
                    }
                    self.delete_schedule_edge_on_the_record(&l.item_id, c.epoch_id)?;
                    outcomes.push(LeftoverOutcome {
                        item_id: l.item_id.clone(),
                        item_type: item_type.to_string(),
                        fate: "moves",
                        to: Some(to.to_string()),
                        reason: None,
                    });
                }
                LeftoverFate::Dropped { reason } => {
                    self.delete_schedule_edge_on_the_record(&l.item_id, c.epoch_id)?;
                    outcomes.push(LeftoverOutcome {
                        item_id: l.item_id.clone(),
                        item_type: item_type.to_string(),
                        fate: "dropped",
                        to: None,
                        reason: Some(reason.trim().to_string()),
                    });
                }
            }
        }

        let mut props = Props::new().set("status", c.how.as_str());
        for (k, v) in &epoch.properties {
            if !matches!(k.as_str(), "status" | "close_reason" | "superseded_by") {
                props = props.set(k, v.clone());
            }
        }
        if let Some(r) = reason {
            props = props.set("close_reason", r);
        }
        if let Some(by) = superseded_by {
            props = props.set("superseded_by", by);
        }
        self.create_node(node::DESIGN_EPOCH, c.epoch_id, props)?;
        self.authored_by(
            node::DESIGN_EPOCH,
            c.epoch_id,
            c.approver,
            Some("approver"),
            c.acted_at,
        )?;

        let change_event_id =
            self.write_close_record(c, &epoch, &outcomes, delivered, withdrawn)?;

        Ok(ClosedIncrement {
            epoch_id: c.epoch_id.to_string(),
            status: c.how.as_str(),
            change_event_id,
            snapshot_id,
            delivered,
            withdrawn,
            leftovers: outcomes,
        })
    }

    /// The ChangeEvent that records a close: how, why, and every leftover's
    /// fate, linked to what moved.
    fn write_close_record(
        &mut self,
        c: &IncrementClose<'_>,
        epoch: &StoredNode,
        outcomes: &[LeftoverOutcome],
        delivered: usize,
        withdrawn: usize,
    ) -> Result<String, DynoError> {
        let slug = c
            .epoch_id
            .strip_prefix("epoch:")
            .unwrap_or(c.epoch_id)
            .to_string();
        let base = format!("chg:{slug}-closed-as-{}", c.how.as_str());
        // A reopened increment closed again gets its own record: the first
        // close is history, never overwritten.
        let mut id = base.clone();
        let mut n = 2;
        while self.get_node(node::CHANGE_EVENT, &id)?.is_some() {
            id = format!("{base}-{n}");
            n += 1;
        }
        let name_of = |e: &StoredNode| {
            e.properties
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(&e.node_id)
                .to_string()
        };
        let mut summary = format!(
            "'{}' ({}) closed as {} without arriving",
            c.epoch_id,
            name_of(epoch),
            c.how.as_str()
        );
        if let Some(by) = nonblank(c.superseded_by) {
            summary.push_str(&format!(", superseded by '{by}'"));
        }
        if let Some(r) = nonblank(c.reason) {
            summary.push_str(&format!(": {r}"));
        }
        summary.push_str(&format!(
            ". Of what was scheduled for it, {delivered} delivered{}",
            if withdrawn > 0 {
                format!(", {withdrawn} withdrawn")
            } else {
                String::new()
            }
        ));
        if outcomes.is_empty() {
            summary.push_str("; nothing unfinished.");
        } else {
            let lines: Vec<String> = outcomes
                .iter()
                .map(|o| match o.fate {
                    "moves" => format!(
                        "'{}' moved to '{}'",
                        o.item_id,
                        o.to.as_deref().unwrap_or_default()
                    ),
                    "dropped" => format!(
                        "'{}' dropped from it ({})",
                        o.item_id,
                        o.reason.as_deref().unwrap_or_default()
                    ),
                    _ => format!("'{}' stays with it", o.item_id),
                })
                .collect();
            summary.push_str(&format!(
                "; {} unfinished: {}.",
                outcomes.len(),
                lines.join("; ")
            ));
        }
        let rationale = format!(
            "Closed on the word of '{}'{}, as req:an-increment-that-will-not-be-finished-can-be-\
             closed-honestly provides: every unfinished item was answered for, none defaulted. \
             An item dropped from the increment keeps its own status.",
            c.approver,
            c.acted_at.map(|d| format!(" on {d}")).unwrap_or_default()
        );
        let name = format!("'{}' closed as {}", c.epoch_id, c.how.as_str());
        self.add_change_event_record(&ChangeEventRecord {
            id: &id,
            name: &name,
            change_type: ChangeType::ScopeChange,
            subject: None,
            summary: Some(&summary),
            rationale: Some(&rationale),
            detected_at: c.acted_at,
            repair: None,
            rationale_basis: None,
            commits: None,
        })?;
        self.changed(&id, node::DESIGN_EPOCH, c.epoch_id, ChangeAction::Modified)?;
        let mut touched: HashSet<String> = HashSet::new();
        for o in outcomes {
            if o.fate == "stays" {
                continue;
            }
            if touched.insert(o.item_id.clone()) {
                self.changed(&id, &o.item_type, &o.item_id, ChangeAction::Modified)?;
            }
            if let Some(to) = &o.to
                && touched.insert(to.clone())
            {
                self.changed(&id, node::DESIGN_EPOCH, to, ChangeAction::Modified)?;
            }
        }
        if let Some(at) = nonblank(c.recorded_in) {
            self.pin_at_epoch(node::CHANGE_EVENT, &id, at)?;
        }
        Ok(id)
    }
}
