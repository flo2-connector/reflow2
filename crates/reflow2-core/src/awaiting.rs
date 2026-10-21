//! AWAITING THE OWNER'S WORD — built work whose record is still open, listed
//! with what delivers each item, so an owner can close it in one look.
//!
//! Measured on reflow2's own design 2026-10-13
//! (`fact:root-cause-built-work-stays-open-because-its-record-is-kept-by-hand-and-the-ship-moves-only-the-capability-2026-10-13`):
//! 33 of 82 proposed requirements were already delivered, and no gap kind
//! asked about a proposed requirement whose work had shipped; 13 open ideas had
//! a promotion that had landed. Each is closed by a status only its owner may
//! move (`dec:certainty-derived`), so nothing here writes. It LISTS, which is
//! the half an owner was missing: what they are asked to close, and why it is
//! ready.
//!
//! Two families, each read by code that already computes it:
//!
//! - **a delivered requirement** — `proposed`, and DELIVERED by the shared
//!   arrival predicate ([`DesignGraph::arrival`] → [`Arrival::Delivered`]),
//!   with what delivers it ([`DesignGraph::requirement_delivered_by`]);
//! - **a promoted idea** — `proposed`, with an `EVOLVES_INTO` whose promotion
//!   has landed: the promotion detector's own population
//!   ([`DesignGraph::ideas_whose_promotion_landed`]), with what it became.
//!
//! It rides `loop_status`'s owner-scoped answer
//! ([`crate::report::LoopStatus::awaiting_word`]), beside the decisions a
//! person was asked to settle and the questions put to them by name: the
//! three families the accepted approval page
//! (`dec:idea-an-owner-reads-what-they-are-asked-to-approve-in-one-addressed-digest`)
//! reads, in one call.

use crate::arrival::Arrival;
use crate::foundation::core::{DynoError, Value};
use crate::graph::DesignGraph;
use crate::nodes::node;

/// Which family an item belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AwaitingKind {
    /// A `proposed` Requirement its thread already delivers.
    DeliveredRequirement,
    /// A `proposed` Decision whose promotion has landed.
    PromotedIdea,
}

/// One item awaiting its owner's word.
///
/// Ids sit under `node_id` and `evidence_ids`, so a hub's view prefixes them
/// with the member design as it does every other id.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AwaitingWord {
    pub kind: AwaitingKind,
    pub node_id: String,
    pub node_type: &'static str,
    pub name: String,
    /// The stored status, as it stands: this read never moves it.
    pub status: String,
    /// What delivers it, by id — the realized, checked capabilities that
    /// satisfy a requirement (or its delivered children), or what an idea
    /// became that has landed.
    pub evidence_ids: Vec<String>,
    /// The same, in a sentence.
    pub evidence: String,
    /// The call that closes it, and whose call it is.
    pub closed_by: &'static str,
}

/// Short on purpose: the list repeats it per item, and what the owner may
/// answer is said once, in the scoped `next`.
const REQUIREMENT_CLOSED_BY: &str = "set_requirement_status, on the owner's word";

const IDEA_CLOSED_BY: &str = "set_decision_status, on the owner's word";

impl DesignGraph {
    /// Everything built whose record still awaits its owner's word:
    /// delivered requirements first, then promoted ideas, each sorted by id.
    /// See the module documentation. Moves nothing.
    pub fn awaiting_word(&self) -> Result<Vec<AwaitingWord>, DynoError> {
        let index = self.node_type_index()?;
        let mut requirements = Vec::new();
        for req in self.scan_live_nodes(node::REQUIREMENT)? {
            let status = req
                .properties
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("proposed");
            if status != "proposed" {
                continue;
            }
            // A STATEMENT OF A LINE OF REASONING IS NOT INTENT
            // (req:evidence-carries-its-source-grade-and-confidence-and-is-not-
            // intent): a report, an analytic judgement, a lemma — anything
            // carrying a `statement_kind` — is what the reasoning says, never
            // a need awaiting its owner's acceptance. Measured 2026-10-19: the
            // capstone's loop_status listed 7 analytic claims here, the logic
            // rung 19 lemmas.
            if req.properties.contains_key("statement_kind") {
                continue;
            }
            if self.arrival(&index, &req.node_id)? != Some(Arrival::Delivered) {
                continue;
            }
            let by = self
                .requirement_delivered_by(&req.node_id)?
                .unwrap_or_default();
            // The derivation hands back capabilities, or — when none satisfies
            // it — the requirement's own delivered children.
            let through_children = by
                .iter()
                .all(|id| index.get(id).map(String::as_str) == Some(node::REQUIREMENT));
            let evidence = if through_children {
                format!(
                    "delivered through its parts, each delivered: {}",
                    by.join(", ")
                )
            } else {
                format!(
                    "delivered: satisfied by {}, realized with a passing check",
                    by.join(", ")
                )
            };
            requirements.push(AwaitingWord {
                kind: AwaitingKind::DeliveredRequirement,
                node_id: req.node_id.clone(),
                node_type: node::REQUIREMENT,
                name: name_of(&req),
                status: status.to_string(),
                evidence_ids: by,
                evidence,
                closed_by: REQUIREMENT_CLOSED_BY,
            });
        }
        requirements.sort_by(|a, b| a.node_id.cmp(&b.node_id));

        let mut ideas: Vec<AwaitingWord> = self
            .ideas_whose_promotion_landed()?
            .open
            .into_iter()
            .map(|(dec, became)| {
                let ids: Vec<String> = became.into_iter().map(|(id, _)| id).collect();
                AwaitingWord {
                    kind: AwaitingKind::PromotedIdea,
                    node_id: dec.node_id.clone(),
                    node_type: node::DECISION,
                    name: name_of(&dec),
                    status: "proposed".to_string(),
                    evidence: format!(
                        "promoted, and it landed: it EVOLVES_INTO {}, accepted, built, \
                         delivered or recorded",
                        ids.join(", ")
                    ),
                    evidence_ids: ids,
                    closed_by: IDEA_CLOSED_BY,
                }
            })
            .collect();
        ideas.sort_by(|a, b| a.node_id.cmp(&b.node_id));

        requirements.extend(ideas);
        Ok(requirements)
    }
}

fn name_of(n: &crate::foundation::store::StoredNode) -> String {
    n.properties
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
