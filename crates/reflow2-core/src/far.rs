//! A PREMISE IN ANOTHER DESIGN CARRIES THAT DESIGN'S COMPUTED TRUTH
//! (`req:reasoning-crosses-designs-within-a-hub`, part 1 and 3; Anthony,
//! 2026-10-18, choosing B of the build list).
//!
//! # What failed
//!
//! A relation to a node in another design is recorded through a stand-in, a
//! Resource `xref:<design>:<node>` ([`crate::crosslink`]), because an edge
//! cannot cross a store. Measured 2026-10-18 on 0.87.0 (flo2.io), case 3 of
//! the reasoning proof of concept: flo2-calc's step citing the theorem "a
//! Fermat test failure proves n composite", PROVED (shown) in design
//! ef749c7f6e03a853, read its one premise as "reflow2 computes no truth for a
//! Resource, so it is unknown", and its conclusion as unproven, although the
//! theorem holds where it lives.
//!
//! # The cure: a reading of the far node, handed in per call
//!
//! The core holds one design and opens no other, so it cannot compute the far
//! node's truth itself. The server that CAN reach the far design (a registry
//! serving both, behind a host that hands over what the caller may read) reads
//! it there first, under the far design's own guard alone, with
//! [`DesignGraph::read_for_another_design`], and hands the readings to this
//! design's evaluation for the length of ONE call through [`scoped`]. A
//! reference used as a premise then reads ([`DesignGraph::far_premise`]):
//!
//! - **read**: the far node's own computed truth and why, its proof status
//!   there, and whatever it holds only under there (an open assumption, an
//!   unverified citation), carried as an unverified citation here, so a
//!   conclusion resting on a conditional far result is conditional too;
//! - **stale**: the far design no longer holds the node: unknown, never
//!   holding;
//! - **unreachable**: the far design could not be read on this call (not in
//!   what the caller may read, not served here, busy), or nothing handed a
//!   reading in at all: unknown, never holding, saying which.
//!
//! The readings are per CALL, never stored on the design and never shared
//! between callers: two callers may read different sets of designs, and a
//! truth read for one must not reach the other. They live in a thread-local
//! that [`scoped`] sets around each synchronous step of the call (the server
//! installs it on every poll of the call's future), so the core stays free of
//! any async runtime, as AGENTS.md asks.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use crate::foundation::core::DynoError;
use crate::graph::DesignGraph;
use crate::inference::Truth;

/// How a far end read on this call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarState {
    /// The far design was read and holds the node.
    Read,
    /// The far design was read and no longer holds the node.
    Stale,
    /// The far design could not be read on this call.
    Unreachable,
}

impl FarState {
    pub fn as_str(self) -> &'static str {
        match self {
            FarState::Read => "read",
            FarState::Stale => "stale",
            FarState::Unreachable => "unreachable",
        }
    }
}

/// What one reference's far node reads as, in its own design.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FarReading {
    /// The far design's graph_id.
    pub design: String,
    /// The node's id there.
    pub node_id: String,
    pub state: FarState,
    /// The far node's computed truth there; `unknown` unless `state` is `read`.
    pub truth: Truth,
    /// Why, in a sentence: the far design's own reason, or why it was not read.
    pub why: String,
    /// The far node's proof status there (`proven`, `shown`, `conditional`,
    /// `contested`, `unproven`), when a step concludes it there.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_status: Option<String>,
    /// What the far node holds only under there — its open assumptions and
    /// unverified citations — each as `<design>:<id>`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holds_only_if: Vec<String>,
}

impl FarReading {
    /// A far end that could not be read, and why.
    pub fn unreachable(design: &str, node_id: &str, why: String) -> Self {
        FarReading {
            design: design.to_string(),
            node_id: node_id.to_string(),
            state: FarState::Unreachable,
            truth: Truth::Unknown,
            why,
            proof_status: None,
            holds_only_if: Vec::new(),
        }
    }
}

/// The readings handed in for one call, by the reference's id here.
pub type FarReadings = HashMap<String, FarReading>;

thread_local! {
    static SCOPE: RefCell<Option<Arc<FarReadings>>> = const { RefCell::new(None) };
}

/// Run `f` with `readings` as the far readings of the call it belongs to, and
/// put back whatever was there before, even if `f` panics.
pub fn scoped<R>(readings: Option<Arc<FarReadings>>, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<Arc<FarReadings>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let prev = self.0.take();
            SCOPE.with(|s| *s.borrow_mut() = prev);
        }
    }
    let prev = SCOPE.with(|s| s.replace(readings));
    let _restore = Restore(prev);
    f()
}

/// The readings in force for the current call, if any were handed in.
pub fn in_scope() -> Option<Arc<FarReadings>> {
    SCOPE.with(|s| s.borrow().clone())
}

/// The sentence a reference reads with when nothing handed a reading in.
pub const NOT_REACHED_HERE: &str = "reflow2 reads another design's truth only where the server \
     can reach that design — a registry serving both, behind a host that hands over what the \
     caller may read — and on this call it could not, so it is unknown (unreachable), never read \
     as holding";

impl DesignGraph {
    /// Every reference to a node in another design that takes part in this
    /// design's reasoning — a premise of a step, concluded by one, or either
    /// end of an objection — as `(reference id, design, node id there)`,
    /// sorted. What a server reads in the far designs before a call.
    pub fn reasoning_references(&self) -> Result<Vec<(String, String, String)>, DynoError> {
        let mut out = Vec::new();
        for r in self.design_references()? {
            let objection = !self
                .incoming(&r.id, Some(crate::nodes::edge::CONTRADICTS))?
                .is_empty()
                || !self
                    .outgoing(&r.id, Some(crate::nodes::edge::CONTRADICTS))?
                    .is_empty();
            if (objection || self.takes_part_in_reasoning(&r.id)?)
                && !r.design.is_empty()
                && !r.node_id.is_empty()
            {
                out.push((r.id, r.design, r.node_id));
            }
        }
        Ok(out)
    }

    /// Every reference here to a node of `design` in `node_ids`: where a change
    /// walk that reached those nodes there continues in this design.
    pub fn references_into(
        &self,
        design: &str,
        node_ids: &BTreeSet<String>,
    ) -> Result<Vec<String>, DynoError> {
        Ok(self
            .design_references()?
            .into_iter()
            .filter(|r| r.design == design && node_ids.contains(&r.node_id))
            .map(|r| r.id)
            .collect())
    }

    /// What `node_id` reads as in THIS design, for a reference to it in
    /// another: its computed truth and why, its proof status, and what it
    /// holds only under, each prefixed with this design's graph_id. A node
    /// this design does not hold is `stale`.
    pub fn read_for_another_design(&self, node_id: &str) -> Result<FarReading, DynoError> {
        let here = self.graph_id().to_string();
        if self.type_holding(node_id)?.is_none() {
            return Ok(FarReading {
                design: here.clone(),
                node_id: node_id.to_string(),
                state: FarState::Stale,
                truth: Truth::Unknown,
                why: format!(
                    "stale: design {here} no longer holds {node_id}, so the reference names \
                     nothing there and is never read as holding. Link to what it became, or \
                     remove the link"
                ),
                proof_status: None,
                holds_only_if: Vec::new(),
            });
        }
        let (truth, why) = self.premise_truth(node_id)?;
        let proof_status = self
            .proof_reading(node_id)?
            .map(|p| p.status.as_str().to_string());
        let mut holds_only_if: Vec<String> = self
            .assumptions_under(node_id)?
            .into_iter()
            .chain(self.unverified_under(node_id)?)
            .map(|id| format!("{here}:{id}"))
            .collect();
        holds_only_if.sort();
        holds_only_if.dedup();
        Ok(FarReading {
            design: here,
            node_id: node_id.to_string(),
            state: FarState::Read,
            truth,
            why,
            proof_status,
            holds_only_if,
        })
    }

    /// The reading a reference `id` here takes as a premise on this call:
    /// the far node's, when one was handed in, else unreachable. `None` when
    /// `id` is not a reference to another design.
    pub fn far_premise(&self, id: &str) -> Result<Option<FarReading>, DynoError> {
        let Some((design, node_id)) = self.reference_target(id)? else {
            return Ok(None);
        };
        let reading = match in_scope() {
            None => FarReading::unreachable(&design, &node_id, NOT_REACHED_HERE.to_string()),
            Some(readings) => readings.get(id).cloned().unwrap_or_else(|| {
                FarReading::unreachable(
                    &design,
                    &node_id,
                    format!(
                        "unreachable: design {design} was not read on this call, so {node_id} \
                         there is unknown here, never read as holding"
                    ),
                )
            }),
        };
        Ok(Some(reading))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(truth: Truth) -> FarReading {
        FarReading {
            design: "far".into(),
            node_id: "req:x".into(),
            state: FarState::Read,
            truth,
            why: "w".into(),
            proof_status: None,
            holds_only_if: vec![],
        }
    }

    #[test]
    fn a_scope_is_seen_inside_and_restored_after_even_on_a_panic() {
        assert!(in_scope().is_none());
        let mut map = FarReadings::new();
        map.insert("xref:far:req:x".into(), reading(Truth::Holds));
        let map = Arc::new(map);
        scoped(Some(map.clone()), || {
            assert_eq!(in_scope().unwrap().len(), 1);
            scoped(None, || assert!(in_scope().is_none()));
            assert_eq!(in_scope().unwrap().len(), 1);
        });
        assert!(in_scope().is_none());
        let caught = std::panic::catch_unwind(|| scoped(Some(map), || panic!("inside")));
        assert!(caught.is_err());
        assert!(
            in_scope().is_none(),
            "a panic must not leave a scope behind"
        );
    }
}
