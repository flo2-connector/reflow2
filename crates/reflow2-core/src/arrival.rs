//! ARRIVAL — one answer to "has this node followed through?"
//!
//! Two detectors ask whether something the design pointed at has since been
//! acted on: `unresolved_setup` (heal.rs), of the node an `ANTICIPATES` edge
//! points at, and `decision_overtaken_by_promotion` (detect.rs), of what an open
//! idea `EVOLVES_INTO`. Until 2026-10-13 each kept its own list, and the lists
//! disagreed: the promotion detector counted a ChangeEvent, a DesignRule and a
//! realized Artifact and the follow-through rule did not; the follow-through
//! rule counted an arrived epoch and the promotion detector did not; and
//! neither could see a requirement delivered by its thread or a finding closed
//! the way this design closes findings. Measured on reflow2's own design that
//! day, 20 of the follow-through rule's 47 findings pointed at something
//! already acted on
//! (`fact:root-cause-follow-through-reads-arrival-from-a-status-nothing-sets-and-misses-how-findings-close-2026-10-13`).
//!
//! The class is "arrived" re-derived by hand in each detector, so the fix is
//! one place: [`DesignGraph::arrival`]. Each detector composes it with the one
//! clause its own definition adds, and both clauses are written HERE, beside
//! the predicate, so a reader sees every way the two rules can differ:
//!
//! - [`DesignGraph::follows_through`] (the follow-through rule) also counts a
//!   node that `EVOLVES_INTO` anything — work recorded against it, on the
//!   owner's word of 2026-09-15 (`dec:a-rule-computes-the-condition-its-message-names`).
//! - [`DesignGraph::promotion_has_landed`] (the promotion detector) also counts
//!   a Requirement the owner `accepted`: the question an idea asked is answered
//!   when what it became is agreed, before anything is built. The follow-through
//!   rule does NOT count it — an accepted requirement is a need agreed, not a
//!   need met — and `tests/follow_through_reads_one_arrival.rs` pins both sides.
//!
//! A COMPONENT'S BUILD READING lives here too ([`DesignGraph::component_build`]),
//! since 2026-10-13: how much of a part has arrived, read from the same
//! "built" clause ([`is_built_status`]) as a capability's or a file's own
//! arrival. It is served beside the stored `status` and never writes it, and
//! `understated_status` reads it, so the reading and the finding cannot
//! disagree
//! (`fact:root-cause-built-work-stays-open-because-its-record-is-kept-by-hand-and-the-ship-moves-only-the-capability-2026-10-13`).
//!
//! Everything here is deterministic and reads only the graph.

use std::collections::HashMap;

use crate::foundation::core::{DynoError, Value};
use crate::foundation::store::{StoredEdge, StoredNode};
use crate::graph::DesignGraph;
use crate::nodes::{edge, node};

/// How a node arrived — the reason [`DesignGraph::arrival`] gives.
///
/// No reply carries it, so adding a way to arrive never changes a reply's
/// shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    /// A Decision `accepted`.
    Accepted,
    /// A Capability, Component or Artifact `realized` or `verified`.
    Built,
    /// A Requirement whose thread delivers it, by the derivation
    /// [`DesignGraph::requirement_is_delivered`] owns.
    Delivered,
    /// A Requirement whose stored status says `met`. Still counted: it is the
    /// owner's word, even though the design does not rely on it (BL-104).
    Met,
    /// A DesignEpoch `arrived`.
    EpochArrived,
    /// A ChangeEvent: a change recorded as having happened. Status-free.
    ChangeRecorded,
    /// A DesignRule: a rule in force. Status-free.
    RuleInForce,
    /// A finding closed — by a `valid_to`, or by an INVALIDATES edge from
    /// anything ([`DesignGraph::finding_is_closed`]).
    FindingClosed,
    /// A finding answered — MITIGATES from an `accepted` Decision or from a
    /// ChangeEvent.
    FindingAnswered,
    /// A finding that led somewhere — CAUSES or TRIGGERS into a Decision, a
    /// ChangeEvent or a Requirement, on an edge that asserts the link.
    FindingLedOn,
}

/// What arrival means for a node of `node_type`, in the words a finding uses
/// when it says the node has NOT arrived. The 09-15 rule: a message names the
/// condition its rule computed, so this sits beside the code that computes it.
pub fn arrival_condition(node_type: Option<&str>) -> String {
    match node_type {
        Some(node::DECISION) => "a Decision arrives when it is `accepted`".to_string(),
        Some(t @ (node::CAPABILITY | node::COMPONENT | node::ARTIFACT)) => {
            format!("a {t} arrives when it is `realized` or `verified`")
        }
        Some(node::REQUIREMENT) => "a Requirement arrives when it is delivered (a realized \
             capability whose check passes satisfies it) or set `met`"
            .to_string(),
        Some(node::DESIGN_EPOCH) => "an epoch arrives when it is `arrived`".to_string(),
        Some(node::TEMPORAL_FACT) => "a finding arrives when it is closed (a `valid_to`, or \
             INVALIDATES from a change), answered (MITIGATES from an accepted Decision or a \
             change), or has led to a decision, change or requirement (CAUSES or TRIGGERS)"
            .to_string(),
        Some(t) => format!("a {t} has no way to arrive of its own, so only evolving counts"),
        None => "it names no node in this design".to_string(),
    }
}

/// Is this the status of something BUILT — `realized` or `verified`? The one
/// clause a Capability, Component or Artifact arrives by, shared by
/// [`DesignGraph::arrival`], the direct build evidence `understated_status`
/// reads, and [`DesignGraph::component_build`], so "built" is said once.
pub fn is_built_status(status: Option<&str>) -> bool {
    matches!(status, Some("realized" | "verified"))
}

/// What a Component's build evidence READS AS — the value of `built_reading`'s
/// `reads_as`.
///
/// Deliberately NOT the stored status's words (`planned` / `in_progress` /
/// `realized` / `verified`): this is computed, the status is the owner's
/// record, and a reader must never mistake one for the other or copy one into
/// the other. Snake_case on the wire, like `checked`'s `component_verified`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildState {
    /// There is built evidence, and every live capability allocated to it is
    /// built (or it hosts none, and a realized file or a passing check of its
    /// own shows it built).
    Built,
    /// There is built evidence, and a live capability allocated to it is not
    /// built yet.
    PartlyBuilt,
    /// No built evidence at all.
    NotBuilt,
}

impl BuildState {
    pub fn as_str(self) -> &'static str {
        match self {
            BuildState::Built => "built",
            BuildState::PartlyBuilt => "partly_built",
            BuildState::NotBuilt => "not_built",
        }
    }
}

/// The evidence behind a Component's build reading, by id, each list sorted.
///
/// Every list is ONE-HOP and DIRECT, exactly what `understated_status` has
/// read since 2026-09-30: a capability counts by its own status, a file only
/// when its own status says it exists, a check only when it passes. Nothing
/// is read from a contained sub-component.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComponentBuild {
    /// The live Capabilities `ALLOCATED_TO` it — its function. One an accepted
    /// Decision has withdrawn is not counted.
    pub capabilities_allocated: Vec<String>,
    /// Of those, the ones that have arrived as built ([`is_built_status`]).
    pub capabilities_built: Vec<String>,
    /// Live Artifacts that `REALIZES` it whose own status is built.
    pub files_built: Vec<String>,
    /// Live Verifications at `passing` that `VERIFIES` it.
    pub checks_passing: Vec<String>,
}

/// What a reader is served as `built_reading`: the reading and the evidence
/// counted. Ids stay behind `get_node` with `include_edges`, so a listing of
/// every component stays small.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct BuiltReading {
    pub reads_as: BuildState,
    pub capabilities_allocated: usize,
    pub capabilities_built: usize,
    pub files_built: usize,
    pub checks_passing: usize,
}

impl ComponentBuild {
    /// Is anything at all shown built?
    pub fn has_evidence(&self) -> bool {
        !(self.capabilities_built.is_empty()
            && self.files_built.is_empty()
            && self.checks_passing.is_empty())
    }

    /// The reading: built, partly built or not built.
    pub fn state(&self) -> BuildState {
        if !self.has_evidence() {
            BuildState::NotBuilt
        } else if self.capabilities_built.len() < self.capabilities_allocated.len() {
            BuildState::PartlyBuilt
        } else {
            BuildState::Built
        }
    }

    /// The reading with its evidence counted, as served.
    pub fn reading(&self) -> BuiltReading {
        BuiltReading {
            reads_as: self.state(),
            capabilities_allocated: self.capabilities_allocated.len(),
            capabilities_built: self.capabilities_built.len(),
            files_built: self.files_built.len(),
            checks_passing: self.checks_passing.len(),
        }
    }
}

/// Does this CAUSES (or TRIGGERS) edge assert a link at all? A person who
/// marked it `spurious`, or a test that `refuted` it, says no causal link
/// holds — so it records neither that a change introduced a defect nor that a
/// finding led to anything.
pub(crate) fn asserts_a_link(e: &StoredEdge) -> bool {
    let p = |k: &str| e.properties.get(k).and_then(Value::as_str);
    p("validation_status") != Some("refuted") && p("basis") != Some("spurious")
}

impl DesignGraph {
    /// Has `id` ARRIVED on its own type's terms — and if so, how?
    ///
    /// `index` maps node id to node type (`node_type_index`); both callers
    /// already hold one. `None` means it has not, including when `id` names no node.
    pub fn arrival(
        &self,
        index: &HashMap<String, String>,
        id: &str,
    ) -> Result<Option<Arrival>, DynoError> {
        let Some(node_type) = index.get(id) else {
            return Ok(None);
        };
        let Some(n) = self.get_node(node_type, id)? else {
            return Ok(None);
        };
        let status = n.properties.get("status").and_then(Value::as_str);
        Ok(match node_type.as_str() {
            node::DECISION => (status == Some("accepted")).then_some(Arrival::Accepted),
            node::CAPABILITY | node::COMPONENT | node::ARTIFACT => {
                is_built_status(status).then_some(Arrival::Built)
            }
            node::REQUIREMENT => match status {
                Some("met") => Some(Arrival::Met),
                // A dropped requirement is a withdrawn need, not delivered
                // work — the same scope `delivery_coverage` draws.
                Some("dropped") => None,
                _ => self
                    .requirement_is_delivered(id)?
                    .then_some(Arrival::Delivered),
            },
            node::DESIGN_EPOCH => (status == Some("arrived")).then_some(Arrival::EpochArrived),
            node::CHANGE_EVENT => Some(Arrival::ChangeRecorded),
            node::DESIGN_RULE => Some(Arrival::RuleInForce),
            node::TEMPORAL_FACT => self.finding_arrival(index, &n)?,
            _ => None,
        })
    }

    /// [`Self::arrival`] as a yes or no.
    pub fn has_arrived(
        &self,
        index: &HashMap<String, String>,
        id: &str,
    ) -> Result<bool, DynoError> {
        Ok(self.arrival(index, id)?.is_some())
    }

    /// Has `id` EVOLVED INTO anything? Work recorded against it, whatever that
    /// work's own state.
    pub fn has_evolved(&self, id: &str) -> Result<bool, DynoError> {
        Ok(!self.outgoing(id, Some(edge::EVOLVES_INTO))?.is_empty())
    }

    /// What the follow-through rule means by "follows through": the node
    /// evolved into something, or it arrived on its own type's terms — the two
    /// clauses of the owner's word of 2026-09-15.
    pub fn follows_through(
        &self,
        index: &HashMap<String, String>,
        id: &str,
    ) -> Result<bool, DynoError> {
        Ok(self.has_evolved(id)? || self.has_arrived(index, id)?)
    }

    /// What the promotion detector means by "taken up": what the idea became
    /// has arrived, or is a Requirement the owner accepted.
    ///
    /// Evolving does NOT count here: an idea that became a second idea, which
    /// became a third, is a chain of ideas and not an answer. Measured on
    /// reflow2's own design 2026-10-13, counting it would have reported five
    /// such chains as answered questions.
    pub fn promotion_has_landed(
        &self,
        index: &HashMap<String, String>,
        id: &str,
    ) -> Result<bool, DynoError> {
        if self.has_arrived(index, id)? {
            return Ok(true);
        }
        if index.get(id).map(String::as_str) != Some(node::REQUIREMENT) {
            return Ok(false);
        }
        Ok(self.get_node(node::REQUIREMENT, id)?.is_some_and(|n| {
            n.properties.get("status").and_then(Value::as_str) == Some("accepted")
        }))
    }

    /// How much of Component `component_id` the design shows BUILT — the
    /// evidence behind its `built_reading`, computed on read and never stored.
    ///
    /// The truth a Component's `status` was meant to record, and on reflow2's
    /// own design measured 2026-10-13 did not: 107 of 109 components read
    /// `planned`, and 88 of them had a built capability allocated to them or a
    /// realized file or passing check attached. A status kept by hand at a
    /// not-done default falls behind; a reading computed from the evidence
    /// cannot. It is served BESIDE the status, which stays the owner's record:
    /// nothing here writes it.
    ///
    /// Three one-hop reads, each the one `understated_status` already made, so
    /// that detector now reads this and the two cannot disagree: the live
    /// Capabilities allocated to it, each built by [`is_built_status`] (the
    /// clause [`Self::arrival`] reads); and its direct evidence, a realized file
    /// or a passing check (`direct_build_evidence`). A withdrawn capability,
    /// file or check is not counted. An unknown id reads as not built.
    pub fn component_build(&self, component_id: &str) -> Result<ComponentBuild, DynoError> {
        let mut build = ComponentBuild::default();
        for e in self.incoming(component_id, Some(edge::ALLOCATED_TO))? {
            if self.is_discontinued(&e.from_id)? {
                continue;
            }
            let Some(cap) = self.get_node(node::CAPABILITY, &e.from_id)? else {
                continue;
            };
            if is_built_status(cap.properties.get("status").and_then(Value::as_str)) {
                build.capabilities_built.push(e.from_id.clone());
            }
            build.capabilities_allocated.push(e.from_id);
        }
        build.capabilities_allocated.sort();
        build.capabilities_allocated.dedup();
        build.capabilities_built.sort();
        build.capabilities_built.dedup();
        let (files_built, checks_passing) = self.direct_build_evidence(component_id)?;
        build.files_built = files_built;
        build.checks_passing = checks_passing;
        Ok(build)
    }

    /// Is this finding CLOSED — a `valid_to`, or an INVALIDATES edge from
    /// anything? The rule the design's readers of open findings apply
    /// (`open_follow_ups`, `defect_overtaken_by_change`, the doubt catalogue):
    /// only INVALIDATES or a `valid_to` closes a finding, and a CAUSES or
    /// MITIGATES edge records that something answered it, not that it is gone.
    pub fn finding_is_closed(&self, fact: &StoredNode) -> Result<bool, DynoError> {
        if fact
            .properties
            .get("valid_to")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty())
        {
            return Ok(true);
        }
        Ok(!self
            .incoming(&fact.node_id, Some(edge::INVALIDATES))?
            .is_empty())
    }

    /// A finding arrives when it is closed, answered, or has led somewhere.
    ///
    /// These are the ways reflow2's own design acts on a finding, read from
    /// the 13 follow-through findings of 2026-10-13 that pointed at one
    /// already acted on: INVALIDATES from a ChangeEvent (9, two of them also
    /// carrying a `valid_to`), MITIGATES from an accepted Decision (1), and
    /// CAUSES or TRIGGERS out of the finding into a ChangeEvent (1), a
    /// Decision (1) or a Requirement (1). EVOLVES_INTO, the only route the rule
    /// read before, closed none of them.
    ///
    /// NOT counted, and pinned: MITIGATES from a `proposed` Decision (an idea
    /// answers nothing), a CAUSES edge INTO the finding (what caused a defect
    /// has not acted on it), a CAUSES edge that is refuted or spurious, and
    /// MITIGATES from a DesignRule — a rule in force that reduces a risk may
    /// well answer a finding, but whether it does is one of the readings held
    /// for the owner's ruling on finding-to-finding links, so it is left as it
    /// stood.
    fn finding_arrival(
        &self,
        index: &HashMap<String, String>,
        fact: &StoredNode,
    ) -> Result<Option<Arrival>, DynoError> {
        if self.finding_is_closed(fact)? {
            return Ok(Some(Arrival::FindingClosed));
        }
        let ty = |id: &str| index.get(id).map(String::as_str);
        for e in self.incoming(&fact.node_id, Some(edge::MITIGATES))? {
            let answered = match ty(&e.from_id) {
                Some(node::CHANGE_EVENT) => true,
                Some(node::DECISION) => {
                    self.get_node(node::DECISION, &e.from_id)?.is_some_and(|d| {
                        d.properties.get("status").and_then(Value::as_str) == Some("accepted")
                    })
                }
                _ => false,
            };
            if answered {
                return Ok(Some(Arrival::FindingAnswered));
            }
        }
        for edge_type in [edge::CAUSES, edge::TRIGGERS] {
            for e in self.outgoing(&fact.node_id, Some(edge_type))? {
                if matches!(
                    ty(&e.to_id),
                    Some(node::DECISION | node::CHANGE_EVENT | node::REQUIREMENT)
                ) && asserts_a_link(&e)
                {
                    return Ok(Some(Arrival::FindingLedOn));
                }
            }
        }
        Ok(None)
    }
}
