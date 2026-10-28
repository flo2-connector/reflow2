//! WHICH CAME FIRST: a prediction, or the result that tests it — read from the
//! times THIS SERVER recorded, and said, never enforced.
//!
//! `req:a-prediction-reads-whether-it-was-stated-before-the-result-it-is-tested-by`
//! (accepted 2026-10-28), governed by
//! `dec:prediction-order-is-read-from-server-recorded-times-and-said-not-enforced-2026-10-28`.
//! A prediction stated before the data is what gives a pre-registered test its
//! weight; one written after is a post hoc analysis, which real science records
//! and labels. So the order is SAID, beside the support it leaves untouched:
//! nothing here refuses a late prediction or moves a supported, contested or
//! refuted reading.
//!
//! # Where a result tests a prediction
//!
//! As the design-a-phenomenon skill records it:
//! - **supports**: a step whose premises hold a result (`statement_kind:
//!   evidence`) and the conditional predictions (Requirements) it is weighed
//!   against. Each such prediction is read against each such result.
//! - **contests**: a CONTRADICTS from a result to the claim it goes against.
//!   That claim is read against the result.
//!
//! # The only times read
//!
//! The server's stamps (`foundation::store` stamps): when the result's node
//! was created, and when the prediction's node was created and last changed in
//! content. Never `reported_at`, `valid_from`, `acted_at` or any other date a
//! caller typed. A stamp this store did not record — copied in by an import,
//! or on a node written before stamping began — is never compared: it is said.

use std::collections::BTreeSet;

use serde::Serialize;

use crate::foundation::core::DynoError;
use crate::foundation::store::NodeStamp;
use crate::graph::DesignGraph;
use crate::nodes::{edge, node};

/// How a prediction stands in time against the result that tests it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    /// Its wording, as it stands, was recorded before the result was.
    StatedBeforeTheResult,
    /// It was first recorded after the result: a post hoc prediction.
    StatedAfterTheResult,
    /// It was recorded before the result, and its content changed after it.
    RevisedAfterTheResult,
    /// Its content last changed after the result, and when it was first
    /// stated was not recorded here.
    ChangedAfterTheResult,
    /// A time this comparison needs was never recorded: a node written before
    /// reflow2 stamped its writes.
    OrderNotRecorded,
    /// A time this comparison needs was recorded somewhere else — carried in
    /// by an import or a design copy, or left on content edited outside
    /// reflow2 — and this server cannot vouch for it.
    ImportedNotRecordedHere,
}

/// One prediction read against one result that tests it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OrderReading {
    /// The prediction (or, for a result that contests, the claim it goes
    /// against).
    pub prediction: String,
    /// The result.
    pub result: String,
    /// `supports` (the result is a premise of a step beside the prediction) or
    /// `contests` (a CONTRADICTS from the result).
    pub bears: &'static str,
    /// The step that weighs them, for a result that supports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    /// How the prediction stands against the result.
    pub order: Order,
    /// The same, in a sentence.
    pub reads: String,
    /// When the server recorded the prediction's creation, when it was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prediction_stated_at: Option<String>,
    /// When the server recorded the prediction's last content change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prediction_changed_at: Option<String>,
    /// When the server recorded the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_recorded_at: Option<String>,
    /// For a prediction revised after the result: the Snapshot that keeps the
    /// wording it had when the result was recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub earlier_wording_in: Option<String>,
}

/// Read one prediction's stamp against one result's: the order, and the
/// sentence that says it.
pub fn read_order(prediction: &NodeStamp, result: &NodeStamp) -> (Order, String) {
    let has_any = |s: &NodeStamp| s.created.is_some() || s.changed.is_some();
    if !(prediction.recorded_here && result.recorded_here) {
        let imported = (!prediction.recorded_here && has_any(prediction))
            || (!result.recorded_here && has_any(result));
        return if imported {
            (
                Order::ImportedNotRecordedHere,
                "the order is not read: a time it needs was recorded somewhere else (an import \
                 or a design copy carried it here), and this server cannot vouch for it"
                    .to_string(),
            )
        } else {
            (
                Order::OrderNotRecorded,
                "the order is not recorded: the prediction or the result was written before \
                 reflow2 stamped its writes"
                    .to_string(),
            )
        };
    }
    let (Some(result_at), Some(changed)) =
        (result.created.as_deref(), prediction.changed.as_deref())
    else {
        return (
            Order::OrderNotRecorded,
            "the order is not recorded: the result's recording was not stamped".to_string(),
        );
    };
    if changed < result_at {
        return (
            Order::StatedBeforeTheResult,
            format!(
                "stated before the result: its wording was recorded at {changed}, the result at \
                 {result_at}"
            ),
        );
    }
    match prediction.created.as_deref() {
        Some(created) if created < result_at => (
            Order::RevisedAfterTheResult,
            format!(
                "revised after the result: first stated at {created}, before the result \
                 ({result_at}), and its content changed at {changed}, after it"
            ),
        ),
        Some(created) => (
            Order::StatedAfterTheResult,
            format!(
                "stated after the result (post hoc): recorded at {created}, after the result \
                 ({result_at})"
            ),
        ),
        None => (
            Order::ChangedAfterTheResult,
            format!(
                "changed after the result: its content changed at {changed}, after the result \
                 ({result_at}); when it was first stated is not recorded here"
            ),
        ),
    }
}

fn is_evidence_node(g: &DesignGraph, node_type: &str, id: &str) -> Result<bool, DynoError> {
    Ok(g.get_node(node_type, id)?
        .is_some_and(|n| crate::inference::is_evidence(&n.properties)))
}

impl DesignGraph {
    /// The order of every prediction and the result that tests it, among the
    /// steps and nodes a read-back walked: `steps` for results that support,
    /// `nodes` for results that contest. Empty when no result tests anything
    /// there, so a design with no results reads exactly as before.
    pub fn prediction_order_over(
        &self,
        steps: &BTreeSet<String>,
        nodes: &[String],
    ) -> Result<Vec<OrderReading>, DynoError> {
        let mut out = Vec::new();
        for step in steps {
            let mut results = Vec::new();
            let mut predictions = Vec::new();
            for e in self.incoming(step, Some(edge::PREMISE_OF))? {
                let Some(ty) = self.type_holding(&e.from_id)? else {
                    continue;
                };
                if is_evidence_node(self, &ty, &e.from_id)? {
                    results.push((ty, e.from_id));
                } else if ty == node::REQUIREMENT {
                    predictions.push((ty, e.from_id));
                }
            }
            if results.is_empty() {
                continue;
            }
            for (rt, r) in &results {
                for (pt, p) in &predictions {
                    if let Some(reading) =
                        self.order_reading(pt, p, rt, r, "supports", Some(step))?
                    {
                        out.push(reading);
                    }
                }
            }
        }
        for id in nodes {
            let Some(pt) = self.type_holding(id)? else {
                continue;
            };
            for e in self.incoming(id, Some(edge::CONTRADICTS))? {
                let Some(rt) = self.type_holding(&e.from_id)? else {
                    continue;
                };
                if !is_evidence_node(self, &rt, &e.from_id)? {
                    continue;
                }
                if let Some(reading) =
                    self.order_reading(&pt, id, &rt, &e.from_id, "contests", None)?
                {
                    out.push(reading);
                }
            }
        }
        out.sort_by(|a, b| {
            (&a.prediction, &a.result, a.bears, &a.step).cmp(&(
                &b.prediction,
                &b.result,
                b.bears,
                &b.step,
            ))
        });
        out.dedup();
        Ok(out)
    }

    fn order_reading(
        &self,
        prediction_type: &str,
        prediction: &str,
        result_type: &str,
        result: &str,
        bears: &'static str,
        step: Option<&String>,
    ) -> Result<Option<OrderReading>, DynoError> {
        let (Some(p), Some(r)) = (
            self.node_stamp(prediction_type, prediction)?,
            self.node_stamp(result_type, result)?,
        ) else {
            return Ok(None);
        };
        let (order, mut reads) = read_order(&p, &r);
        let mut earlier_wording_in = None;
        if matches!(
            order,
            Order::RevisedAfterTheResult | Order::ChangedAfterTheResult
        ) && let Some(result_at) = r.created.as_deref()
        {
            earlier_wording_in = self.wording_kept_since(prediction, result_at)?;
            match &earlier_wording_in {
                Some(snap) => reads.push_str(&format!(
                    "; the wording it had when the result was recorded is kept in {snap}"
                )),
                None => reads.push_str(
                    "; no snapshot this server recorded keeps the wording it had when the \
                     result was recorded",
                ),
            }
        }
        let recorded = |s: &NodeStamp, v: &Option<String>| {
            if s.recorded_here { v.clone() } else { None }
        };
        Ok(Some(OrderReading {
            prediction: prediction.to_string(),
            result: result.to_string(),
            bears,
            step: step.cloned(),
            order,
            reads,
            prediction_stated_at: recorded(&p, &p.created),
            prediction_changed_at: recorded(&p, &p.changed),
            result_recorded_at: recorded(&r, &r.created),
            earlier_wording_in,
        }))
    }

    /// The Snapshot that keeps the wording `node_id` had at `at`: the earliest
    /// one this server recorded at or after `at`, since a snapshot preserves
    /// the wording a change replaced.
    fn wording_kept_since(&self, node_id: &str, at: &str) -> Result<Option<String>, DynoError> {
        let mut best: Option<(String, String)> = None;
        for snap in self.snapshots_of(node_id)? {
            let Some(stamp) = self.node_stamp(node::SNAPSHOT, &snap.node_id)? else {
                continue;
            };
            let Some(created) = stamp.created.filter(|_| stamp.recorded_here) else {
                continue;
            };
            if created.as_str() >= at && best.as_ref().is_none_or(|(b, _)| created < *b) {
                best = Some((created, snap.node_id.clone()));
            }
        }
        Ok(best.map(|(_, id)| id))
    }
}
