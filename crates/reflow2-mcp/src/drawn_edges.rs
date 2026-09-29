//! Every edge a constructor draws is named in its reply, as a sentence with its
//! subject first.
//!
//! # The finding this exists to fix
//!
//! `fact:root-cause-add-decision-discards-the-edge-echo-its-related-to-computed-2026-09-28`.
//! `add_decision` drew exploratory `related_to` through the core
//! `review_relations` and then THREW AWAY its outcome — the very subject-first
//! sentences the brainstorm skill promises — and for a `choice` or no-kind
//! decision it drew nothing at all, silently. Reported by dev_storyflow
//! (2026-09-27), reproduced on 0.72.0 and 0.74.0, and hit again on 2026-09-29
//! by a designer agent whose seven `choice` decisions landed with no edges
//! (`art:dev-reflow2-two-agent-exercise-feedback-2026-09-29`, item I16).
//!
//! # The class, which is why this is a module and not a patch to one handler
//!
//! The sentence is rendered once (core `ReviewOutcome`) and shared by both
//! callers, but SURFACING it was left to each handler. Each constructor built
//! its own reply, there was no shared "edges this call drew" block, and no
//! stated invariant that every edge a call draws is named in its reply.
//! Measured on 0.72.0: of the five constructors that draw edges inline, three
//! echoed ids (`verifies`, `changed`, `subject`/`caused_by`), two echoed
//! nothing (`add_decision`, `add_capability` — which built a `drawn` list and
//! discarded it), and none rendered the sentence. The one constructor with a
//! caller-chosen direction (`incoming`) was one of the two silent ones.
//!
//! So the invariant now lives HERE, and every constructor that takes relation
//! targets reports through it:
//!
//! - every edge the call DREW is named, as `from RELATION to`;
//! - an edge it was asked for that ALREADY EXISTED is named apart, so a revise
//!   that re-sends a target does not read as having found something new;
//! - nothing else is named.
//!
//! The sentence shape is the one `review_relations` already uses
//! (`req:deleting-an-artifact BLOCKS cmp:artifact-store`): it reads wrong
//! immediately when it IS wrong, which is the whole reason for it — a reader
//! who has just written the call supplies what they intended, not what they
//! sent, unless the reply states the subject.
//!
//! The existing id-shaped echoes (`verifies`, `changed`, `subject`,
//! `caused_by`) are left in place: consumers read them, and this block is
//! additive.

use reflow2_core::DesignGraph;
use reflow2_core::DynoError;
use serde_json::Value as JsonValue;

/// One edge, read as a sentence with its subject first.
#[must_use]
pub fn sentence(from_id: &str, relation: &str, to_id: &str) -> String {
    format!("{from_id} {relation} {to_id}")
}

/// Whether `from_id -[relation]-> to_id` already exists.
///
/// # Errors
///
/// A store read failure.
pub fn present(
    g: &DesignGraph,
    from_id: &str,
    relation: &str,
    to_id: &str,
) -> Result<bool, DynoError> {
    Ok(g.outgoing(from_id, Some(relation))?
        .into_iter()
        .any(|e| e.to_id == to_id))
}

/// What one constructor call did to the edge set, in sentences.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DrawnEdges {
    /// Edges this call created.
    pub drawn: Vec<String>,
    /// Edges this call was asked for that already existed.
    pub already_present: Vec<String>,
}

impl DrawnEdges {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adopt a core review's outcome. The core already renders each edge as a
    /// subject-first sentence, so nothing is re-rendered here — two renderings
    /// of one edge are two chances to disagree.
    #[must_use]
    pub fn from_review(outcome: &reflow2_core::relate::ReviewOutcome) -> Self {
        Self {
            drawn: outcome.drawn.clone(),
            already_present: outcome.already_present.clone(),
        }
    }

    /// Record an edge this call created.
    pub fn drew(&mut self, from_id: &str, relation: &str, to_id: &str) {
        self.drawn.push(sentence(from_id, relation, to_id));
    }

    /// Record an edge this call was asked for and found already present.
    pub fn found(&mut self, from_id: &str, relation: &str, to_id: &str) {
        self.already_present
            .push(sentence(from_id, relation, to_id));
    }

    /// Record `from -[relation]-> to` as drawn or already present, by asking the
    /// store BEFORE the caller writes it. Returns whether it was already there,
    /// so a caller can skip a redundant write.
    ///
    /// # Errors
    ///
    /// A store read failure.
    pub fn classify(
        &mut self,
        g: &DesignGraph,
        from_id: &str,
        relation: &str,
        to_id: &str,
    ) -> Result<bool, DynoError> {
        let was = present(g, from_id, relation, to_id)?;
        if was {
            self.found(from_id, relation, to_id);
        } else {
            self.drew(from_id, relation, to_id);
        }
        Ok(was)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.drawn.is_empty() && self.already_present.is_empty()
    }

    /// Put the block on a reply object.
    ///
    /// SILENT when the call touched no edge — a block present and empty on
    /// every capture is the noise `with_capture_notes`' siblings are built not
    /// to become. When it did, `edges_drawn` is ALWAYS present (possibly empty,
    /// which says "everything you asked for was already there"), and
    /// `edges_already_present` only when non-empty.
    pub fn attach(&self, reply: &mut JsonValue) {
        if self.is_empty() {
            return;
        }
        let Some(obj) = reply.as_object_mut() else {
            return;
        };
        obj.insert(
            "edges_drawn".into(),
            JsonValue::Array(
                self.drawn
                    .iter()
                    .map(|s| JsonValue::String(s.clone()))
                    .collect(),
            ),
        );
        if !self.already_present.is_empty() {
            obj.insert(
                "edges_already_present".into(),
                JsonValue::Array(
                    self.already_present
                        .iter()
                        .map(|s| JsonValue::String(s.clone()))
                        .collect(),
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn silent_when_nothing_was_touched() {
        let mut v = json!({"node_id": "dec:x"});
        DrawnEdges::new().attach(&mut v);
        assert_eq!(v, json!({"node_id": "dec:x"}));
    }

    #[test]
    fn names_drawn_and_present_apart_subject_first() {
        let mut e = DrawnEdges::new();
        e.drew("dec:a", "DEPENDS_ON", "dec:b");
        e.found("dec:c", "EVOLVES_INTO", "dec:a");
        let mut v = json!({});
        e.attach(&mut v);
        assert_eq!(v["edges_drawn"], json!(["dec:a DEPENDS_ON dec:b"]));
        assert_eq!(
            v["edges_already_present"],
            json!(["dec:c EVOLVES_INTO dec:a"])
        );
    }

    #[test]
    fn an_all_present_call_still_says_it_drew_none() {
        let mut e = DrawnEdges::new();
        e.found("cap:a", "SATISFIES", "req:b");
        let mut v = json!({});
        e.attach(&mut v);
        assert_eq!(v["edges_drawn"], json!([]));
    }
}
