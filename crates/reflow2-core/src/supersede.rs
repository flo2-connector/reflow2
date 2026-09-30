//! The choice made at the near-match check, kept: a new node that REPLACES an
//! older one says so, and a new node judged DISTINCT from one says that.
//!
//! # The finding this answers
//!
//! `fact:root-cause-a-status-that-falls-behind-reality-is-silent-and-a-superseding-capability-leaves-no-trace-2026-09-30`.
//! The capture guard offered two routes past a same-type near-match — sharpen
//! the existing node, or create anyway with `distinct_from` — and no route for
//! "this new node takes that one's place". `distinct_from` was consumed in the
//! call and written nowhere. Measured on flo2's design the same day: a
//! function rebuilt under a NEW capability left its predecessor `planned`, with
//! no edge between them and a requirement still pointing only at the old node.
//! (The first reading counted three such pairs; the reconciliation found one
//! was a supersession and two were not — which is exactly the judgement only
//! the writer could have recorded at the time.) Whether the writer of the new
//! node had seen the old one could not be established on any design.
//!
//! This is shape (B) of `dec:idea-is-the-evolution-vocabulary-unused-because-nothing-asks-for-it`
//! — ask at capture, where the duplicate guard is already looking — built in
//! the vocabulary the design already sanctions rather than new words.
//!
//! # Which edge, and why this one
//!
//! **`OBSOLETES`, drawn from the successor to what it replaced.** It is the
//! edge the served `retire-from-design` skill names for "a Capability /
//! Component with a successor", the edge `dec:reopen-supersedes` uses when a
//! new Decision takes an old one's place, and — since
//! `dec:idea-discontinued-is-a-first-class-state` — the one retirement edge
//! anything reads.
//!
//! - `SUPERSEDES` is declared for Fragment and Verification only, and which of
//!   the two retirement words survives is `dec:one-retire-edge`'s open
//!   question. Widening it here would settle that question by implementation.
//! - `EVOLVES_INTO` says an earlier form BECAME the later one: it is the
//!   promotion edge (an idea to the requirement it became, capture-intent), and
//!   `heal`'s follow-through reads it as "acted on". A replacement retires the
//!   older node; that is a different claim.
//!
//! # How the replaced node is left
//!
//! Exactly as `retire-from-design`'s Path A leaves a retired node:
//!
//! 1. its ending is RECORDED FIRST — a `deprecation` ChangeEvent that `removed`
//!    it from the live design, whose snapshot holds its final properties AND
//!    edges (BL-63), so the thread about to move survives as history;
//! 2. the thread that says what the node is FOR moves to the successor;
//! 3. the successor `OBSOLETES` it, with the reason in `evidence`.
//!
//! Its stored `status` does NOT move: that field records what was BUILT
//! (`dec:idea-does-a-capability-need-a-cancelled-state`, no `cancelled` state,
//! ever). Withdrawing it from the gap and delivery counts stays an ACCEPTED
//! Decision's act (`dec:idea-discontinued-is-a-first-class-state`), because a
//! capture is not the owner's word. The reply says which of those the node now
//! stands at, per type, so the step that remains is named rather than implied.
//!
//! # What moves, and what deliberately does not
//!
//! Only the edges that say what the node is FOR ([`inherited_edges`]) — a
//! Capability's `SATISFIES`, a Component's incoming `ALLOCATED_TO`. A successor
//! does the old node's job, so it inherits the job. Everything else is a fact
//! about the OLD node — what built it, what checked it, where it was going to
//! live, who wrote it — and moving it would forge evidence for the new one. It
//! stays, and every such edge is NAMED in the reply so nothing is left behind
//! silently. A Requirement moves nothing: whether what was built for its old
//! wording meets the new one is a delivery claim, and nobody made it.
//!
//! # Why `distinct_from` became a property
//!
//! The judgement "read X, judged this different" is not a relation between the
//! two nodes — the schema has no comparison edge that says "not the same", and
//! a `DUPLICATES` with a negated basis would be read as a suspicion by every
//! reader that treats an unknown basis as `suspected`. The design's precedent
//! for a recorded judgement that is not an edge is `no_relation_note`
//! (`relate.rs`): the judgement gets a place of its own on the node, written by
//! the door that asked for it. So `distinct_from` is kept on the node it
//! created, under the argument's own name.

use serde::Serialize;

use crate::foundation::core::{DynoError, Value};
use crate::graph::DesignGraph;
use crate::nodes::{Props, edge, node};
use crate::temporal::{ChangeAction, ChangeRecord, ChangeType, EpochType, PRESERVE_EPOCH_ID};

/// The node types whose capture tool may say it REPLACES another node of its
/// own type — the five that run the near-match guard, since that is where the
/// question arises.
pub const REPLACEABLE_TYPES: &[&str] = &[
    node::REQUIREMENT,
    node::CAPABILITY,
    node::COMPONENT,
    node::DECISION,
    node::DESIGN_RULE,
];

/// The property a capture tool's `distinct_from` judgement is kept in, on the
/// node the call created. Same name as the argument, so the record and the
/// call read alike.
pub const DISTINCT_FROM: &str = "distinct_from";

/// Which end of an inherited edge the replaced node sits at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The replaced node is the edge's source (`cap:old SATISFIES req:x`).
    Outgoing,
    /// The replaced node is the edge's target (`cap:x ALLOCATED_TO cmp:old`).
    Incoming,
}

/// The edges a successor INHERITS from the node it replaces: the ones that say
/// what the node is FOR, and nothing else. See the module header for why the
/// rest stay.
#[must_use]
pub fn inherited_edges(node_type: &str) -> &'static [(&'static str, Side)] {
    match node_type {
        node::CAPABILITY => &[(edge::SATISFIES, Side::Outgoing)],
        node::COMPONENT => &[(edge::ALLOCATED_TO, Side::Incoming)],
        _ => &[],
    }
}

/// What one replacement did, in sentences whose subject comes first.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Supersession {
    /// The node replaced.
    pub replaced: String,
    /// `new OBSOLETES old` — the retirement edge, drawn or already present.
    pub retired_by: String,
    /// Each inherited edge as it read on the old node, then on the successor:
    /// `cap:old SATISFIES req:x → cap:new SATISFIES req:x`.
    pub moved: Vec<String>,
    /// The old node's other design edges, which stay where they are — named so
    /// nothing is left behind without a word.
    pub stays_on_replaced: Vec<String>,
    /// Where the old node's ending is recorded, in a sentence.
    pub ending: String,
    /// The ChangeEvent that records the ending, when this call recorded one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_event: Option<String>,
    /// The snapshot holding the old node's final state and edges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
    /// Where the old node now stands for the detectors, and the step (if any)
    /// that withdraws it — per type, read from the graph.
    pub standing: String,
    /// Edges this call drew onto the successor, for the reply's shared echo.
    #[serde(skip)]
    pub drawn: Vec<String>,
    /// Edges this call would have drawn and found already present.
    #[serde(skip)]
    pub already_present: Vec<String>,
}

/// What recording a `distinct_from` judgement did.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JudgedDistinct {
    /// Every id the node now records as read and judged different — this call's
    /// and any earlier call's, since a judgement is history.
    pub recorded: Vec<String>,
    /// Ids this call named that resolve to no node, so were NOT recorded: a
    /// judgement about nothing is not a judgement. Named, never dropped.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub not_recorded: Vec<String>,
    /// The fact, as a sentence.
    pub note: String,
}

/// One inherited edge, read off the old node before anything is written.
struct ToMove {
    edge_type: String,
    side: Side,
    other: String,
    other_type: String,
    props: std::collections::HashMap<String, Value>,
}

fn sentence(from: &str, relation: &str, to: &str) -> String {
    format!("{from} {relation} {to}")
}

fn refuse(node_type: &str, message: String) -> DynoError {
    DynoError::Validation {
        node_type: node_type.to_string(),
        property: "replaces".to_string(),
        message,
    }
}

impl DesignGraph {
    /// Check a `replaces` list BEFORE anything is written, so a refusal leaves
    /// the design exactly as it was.
    ///
    /// # Errors
    ///
    /// Refuses, naming what would have worked:
    /// - a type that runs no near-match guard;
    /// - the node naming itself;
    /// - an id also named in `distinct_from` — two contradictory judgements;
    /// - an id that names nothing;
    /// - an id of another type — a record at another layer is not a
    ///   predecessor, and its route is `distinct_from`.
    pub fn check_replaces(
        &self,
        node_type: &str,
        new_id: &str,
        replaces: &[String],
        distinct_from: &[String],
    ) -> Result<(), DynoError> {
        if replaces.is_empty() {
            return Ok(());
        }
        if !REPLACEABLE_TYPES.contains(&node_type) {
            return Err(refuse(
                node_type,
                format!(
                    "a {node_type} cannot say what it replaces here — `replaces` is the third \
                     route past the near-match check, which runs for {}. To retire a node, \
                     follow retire-from-design.",
                    REPLACEABLE_TYPES.join(", ")
                ),
            ));
        }
        for old in replaces {
            if old == new_id {
                return Err(refuse(
                    node_type,
                    format!("`{new_id}` cannot replace itself; nothing was written."),
                ));
            }
            if distinct_from.iter().any(|d| d == old) {
                return Err(refuse(
                    node_type,
                    format!(
                        "`{old}` is named in BOTH `replaces` and `distinct_from` — this node \
                         cannot take its place and be a different thing from it. Nothing was \
                         written. Keep the one you mean."
                    ),
                ));
            }
            let holders = self.node_types_holding(old)?;
            if holders.is_empty() {
                return Err(refuse(
                    node_type,
                    format!(
                        "`{old}` names nothing in this design, so `{new_id}` cannot replace it \
                         and nothing was written. Check the id — the near-match refusal lists \
                         the exact ids, and search_design finds one by its words."
                    ),
                ));
            }
            if !holders.iter().any(|t| t == node_type) {
                return Err(refuse(
                    node_type,
                    format!(
                        "`{old}` is a {} and a {node_type} replaces only a {node_type}, so \
                         nothing was written. A record at another layer is not a predecessor — \
                         it is the same work recorded twice on purpose, and the route past it \
                         is `distinct_from`.",
                        holders.join(" / ")
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Replace `old_id` with `new_id` (both `node_type`, both already present):
    /// record the old node's ending, move the thread that says what it is for,
    /// and draw `OBSOLETES` from the successor. See the module header for the
    /// shape and its precedents.
    ///
    /// Idempotent: replacing again records no second ending, draws no second
    /// edge, and reports what is already there as already present.
    ///
    /// # Errors
    ///
    /// A missing node, or a store failure. Call
    /// [`check_replaces`](Self::check_replaces) first; this does not re-check
    /// the caller's judgement.
    pub fn supersede(
        &mut self,
        node_type: &str,
        new_id: &str,
        old_id: &str,
        via: &str,
    ) -> Result<Supersession, DynoError> {
        let old = self
            .get_node(node_type, old_id)?
            .ok_or_else(|| DynoError::NodeNotFound {
                node_type: node_type.to_string(),
                node_id: old_id.to_string(),
            })?;
        let index = self.node_type_index()?;

        // Read everything BEFORE the first write, so the lists describe the
        // old node as the caller found it.
        let inherited = inherited_edges(node_type);
        let mut to_move: Vec<ToMove> = Vec::new();
        for (et, side) in inherited {
            let edges = match side {
                Side::Outgoing => self.outgoing(old_id, Some(et))?,
                Side::Incoming => self.incoming(old_id, Some(et))?,
            };
            for e in edges {
                let other = match side {
                    Side::Outgoing => e.to_id.clone(),
                    Side::Incoming => e.from_id.clone(),
                };
                if other == new_id {
                    continue;
                }
                let Some(other_type) = index.get(&other).cloned() else {
                    continue; // dangling — nothing to carry across
                };
                to_move.push(ToMove {
                    edge_type: (*et).to_string(),
                    side: *side,
                    other,
                    other_type,
                    props: e.properties,
                });
            }
        }
        let mut stays: Vec<String> = Vec::new();
        for (e, out) in self
            .outgoing(old_id, None)?
            .into_iter()
            .map(|e| (e, true))
            .chain(self.incoming(old_id, None)?.into_iter().map(|e| (e, false)))
        {
            let other = if out { &e.to_id } else { &e.from_id };
            if other == new_id || e.edge_type == edge::AUTHORED_BY {
                continue;
            }
            if inherited
                .iter()
                .any(|(et, side)| *et == e.edge_type && (*side == Side::Outgoing) == out)
            {
                continue;
            }
            let bookkeeping = index
                .get(other)
                .is_some_and(|t| crate::temporal::BOOKKEEPING_TYPES.contains(&t.as_str()))
                && !crate::temporal::COMMITMENT_EDGES.contains(&e.edge_type.as_str());
            if bookkeeping {
                continue;
            }
            stays.push(format!(
                "{} stays on {old_id} — draw it on {new_id} too if it holds there",
                sentence(&e.from_id, &e.edge_type, &e.to_id)
            ));
        }
        stays.sort();

        let already_retired = self
            .outgoing(new_id, Some(edge::OBSOLETES))?
            .iter()
            .any(|e| e.to_id == old_id);

        // 1. THE ENDING, RECORDED BEFORE ANYTHING MOVES — retire-from-design's
        //    step 2. The snapshot captures the old node's edges as they stand,
        //    so the SATISFIES about to move is still on its timeline.
        let (change_event, snapshot) = if already_retired {
            (None, None)
        } else {
            if self
                .get_node(node::DESIGN_EPOCH, PRESERVE_EPOCH_ID)?
                .is_none()
            {
                self.add_epoch(
                    PRESERVE_EPOCH_ID,
                    "Preserved on write",
                    EpochType::Revision,
                    0,
                )?;
            }
            let chg_id = format!("chg:replaced:{old_id}:by:{new_id}");
            let name = format!("{old_id} is replaced by {new_id}");
            let (snap, _) = self.record_change(ChangeRecord {
                epoch_id: PRESERVE_EPOCH_ID,
                change_event_id: &chg_id,
                name: &name,
                change_type: ChangeType::Deprecation,
                // Not inferred: whether the THING changed or only the design's
                // record of it did is the writer's to say, and a replacement
                // can honestly be either.
                subject: None,
                target_type: node_type,
                target_id: old_id,
                action: ChangeAction::Removed,
                repair: None,
            })?;
            self.upsert_node(
                node::CHANGE_EVENT,
                &chg_id,
                Props::new().set(
                    "summary",
                    format!(
                        "{new_id} takes {old_id}'s place, declared through `replaces` on \
                         {via}. {old_id}'s final state and edges are this event's snapshot; \
                         {} edge(s) of its thread moved to {new_id}.",
                        to_move.len()
                    ),
                ),
            )?;
            (Some(chg_id), snap.map(|s| s.node_id))
        };

        // 2. THE THREAD MOVES — drawn on the successor first, then taken off
        //    the old node, so a refused draw never leaves the thread cut.
        let mut moved = Vec::new();
        let mut drawn = Vec::new();
        let mut already_present = Vec::new();
        for ToMove {
            edge_type: et,
            side,
            other,
            other_type,
            props,
        } in to_move
        {
            let (old_s, new_s, from_t, from_i, to_t, to_i) = match side {
                Side::Outgoing => (
                    sentence(old_id, &et, &other),
                    sentence(new_id, &et, &other),
                    node_type.to_string(),
                    new_id.to_string(),
                    other_type,
                    other.clone(),
                ),
                Side::Incoming => (
                    sentence(&other, &et, old_id),
                    sentence(&other, &et, new_id),
                    other_type,
                    other.clone(),
                    node_type.to_string(),
                    new_id.to_string(),
                ),
            };
            let present = self
                .outgoing(&from_i, Some(&et))?
                .iter()
                .any(|e| e.to_id == to_i);
            if present {
                already_present.push(new_s.clone());
            } else {
                self.create_edge(&et, &from_t, &from_i, &to_t, &to_i, props)?;
                drawn.push(new_s.clone());
            }
            match side {
                Side::Outgoing => self.delete_edge(&et, old_id, &other)?,
                Side::Incoming => self.delete_edge(&et, &other, old_id)?,
            };
            moved.push(format!("{old_s} → {new_s}"));
        }

        // 3. THE RETIREMENT EDGE, from the successor, with its reason.
        let retired_by = sentence(new_id, edge::OBSOLETES, old_id);
        if already_retired {
            already_present.push(retired_by.clone());
        } else {
            let evidence = format!(
                "{new_id} replaces {old_id}: declared by the writer through `replaces` on {via}. \
                 {old_id}'s ending is recorded in chg:replaced:{old_id}:by:{new_id}, and the \
                 thread that says what it was for moved to {new_id}."
            );
            self.create_edge(
                edge::OBSOLETES,
                node_type,
                new_id,
                node_type,
                old_id,
                Props::new().set("evidence", evidence),
            )?;
            drawn.push(retired_by.clone());
        }

        let ending = match (&change_event, &snapshot) {
            (Some(c), Some(s)) => format!(
                "{c} records {old_id}'s ending as a deprecation that removed it from the live \
                 design; {s} holds its final state and edges, so what moved is still on its \
                 timeline"
            ),
            (Some(c), None) => format!(
                "{c} records {old_id}'s ending as a deprecation that removed it from the live \
                 design"
            ),
            _ => format!(
                "{old_id} was already replaced by {new_id}; its ending was recorded then, and \
                 no second one was written"
            ),
        };
        let standing = self.standing_after_replacement(node_type, new_id, old_id, &old)?;
        Ok(Supersession {
            replaced: old_id.to_string(),
            retired_by,
            moved,
            stays_on_replaced: stays,
            ending,
            change_event,
            snapshot,
            standing,
            drawn,
            already_present,
        })
    }

    /// Where a replaced node stands for the detectors, read from the graph —
    /// and the step, if one remains, that withdraws it. Never a fixed sentence:
    /// an accepted successor Decision has ALREADY withdrawn its predecessor,
    /// and saying otherwise would be the fixed-hint class this project keeps
    /// finding.
    fn standing_after_replacement(
        &self,
        node_type: &str,
        new_id: &str,
        old_id: &str,
        old: &crate::foundation::store::StoredNode,
    ) -> Result<String, DynoError> {
        let status = old
            .properties
            .get("status")
            .and_then(Value::as_str)
            .map(str::to_string);
        if self.is_discontinued(old_id)? {
            return Ok(format!(
                "{old_id} now reads `discontinued`: an ACCEPTED Decision OBSOLETES it, so the \
                 detectors and delivery stop counting it"
            ));
        }
        Ok(match node_type {
            node::DECISION => format!(
                "{old_id} still counts until {new_id} is accepted — an ACCEPTED Decision that \
                 OBSOLETES a node withdraws it, and {new_id} stands `{}`; only the owner's word \
                 moves it (set_decision_status with their approver)",
                self.get_node(node::DECISION, new_id)?
                    .and_then(|n| n
                        .properties
                        .get("status")
                        .and_then(Value::as_str)
                        .map(str::to_string))
                    .unwrap_or_else(|| "proposed".into())
            ),
            node::REQUIREMENT => format!(
                "{old_id} still stands `{}` and still counts — a requirement leaves the design \
                 on the owner's word: set_requirement_status `dropped` with their approver \
                 (retire-from-design). Its satisfiers were NOT moved: whether what was built for \
                 its wording meets {new_id}'s is a delivery claim, so draw `satisfies` to \
                 {new_id} for each one that does",
                status.as_deref().unwrap_or("proposed")
            ),
            _ => format!(
                "{old_id} keeps its stored status{} — it records what was BUILT, and no status \
                 means 'retired' — and it still counts in the detectors: OBSOLETES from a \
                 successor retires it on the record, and only an ACCEPTED Decision that \
                 OBSOLETES it withdraws it from the gap and delivery counts \
                 (retire-from-design)",
                status
                    .as_deref()
                    .map(|s| format!(" `{s}`"))
                    .unwrap_or_default()
            ),
        })
    }

    /// Keep a `distinct_from` judgement on the node it concerns: the ids its
    /// writer read and judged different, merged with any judged before.
    ///
    /// # Errors
    ///
    /// A store failure. An id naming nothing is reported in `not_recorded`,
    /// never refused: the guard already accepted the call, and refusing now
    /// would undo a create the caller was told was deliberate.
    pub fn record_judged_distinct(
        &mut self,
        node_type: &str,
        id: &str,
        judged: &[String],
    ) -> Result<Option<JudgedDistinct>, DynoError> {
        if judged.is_empty() {
            return Ok(None);
        }
        let Some(current) = self.get_node(node_type, id)? else {
            return Err(DynoError::NodeNotFound {
                node_type: node_type.to_string(),
                node_id: id.to_string(),
            });
        };
        let mut recorded: Vec<String> = match current.properties.get(DISTINCT_FROM) {
            Some(Value::List(items)) => items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
            _ => Vec::new(),
        };
        let before = recorded.clone();
        let mut not_recorded = Vec::new();
        for other in judged {
            if other == id || recorded.contains(other) {
                continue;
            }
            if self.node_types_holding(other)?.is_empty() {
                not_recorded.push(other.clone());
                continue;
            }
            recorded.push(other.clone());
        }
        if recorded != before {
            self.upsert_node(
                node_type,
                id,
                Props::new().set(
                    DISTINCT_FROM,
                    Value::List(recorded.iter().map(|s| Value::from(s.as_str())).collect()),
                ),
            )?;
        }
        let note = if recorded.is_empty() {
            format!(
                "{id} records no judgement: none of the ids passed in `distinct_from` names a \
                 node in this design"
            )
        } else {
            format!(
                "{id} records that its writer read {} and judged it a different thing — kept \
                 on the node as `distinct_from`, so a later reader can tell a judgement from a \
                 node nobody compared",
                recorded.join(", ")
            )
        };
        Ok(Some(JudgedDistinct {
            recorded,
            not_recorded,
            note,
        }))
    }
}
