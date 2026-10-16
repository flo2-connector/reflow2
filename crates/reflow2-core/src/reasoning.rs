//! A LINE OF REASONING, READ BACK — `reasoning_report`.
//!
//! `req:a-design-holds-a-line-of-reasoning` (Anthony, 2026-10-14). An argument
//! for and against, an experiment's case, a paper's chain of claims or a legal
//! case is recorded with reflow2's own vocabulary:
//!
//! - the claims, and the steps that support each one (`add_inference`, with
//!   its rule and warrant);
//! - what the reasoning rests on: a premise taken as given is a Constraint,
//!   and holds until something violates or refutes it;
//! - its sources: an Artifact that DOCUMENTS a claim, with where to open it;
//! - objections and replies: CONTRADICTS, counted as far as each holds
//!   (`crate::inference`);
//! - a dispute kept open on purpose: a Decision set to `contested` that
//!   governs both sides.
//!
//! This module reads all of it back in one place: the claims, their support,
//! what the reasoning rests on, the points still contested, the disputes kept
//! open by design, and any claim with no support.
//!
//! # What is in the reasoning
//!
//! A walk from the ROOTS: the claim asked about or, when none is, every top
//! conclusion (a node a step concludes that is a premise of no step) and every
//! `contested` Decision. From each node it follows:
//!
//! - the steps that conclude it, and their premises;
//! - the objections against it (opposing CONTRADICTS), whose own objections
//!   are the replies;
//! - for a `contested` Decision, the nodes it governs, and for a node one
//!   governs, that Decision.
//!
//! Corroboration (CONTRADICTS marked `supporting`) and cited sources are
//! listed on the node and never walked.
//!
//! # Shown here, never raised design-wide
//!
//! `con:no-gap-is-raised-for-a-conclusion-that-has-no-inference`: a claim with
//! no support is listed in this read-back and in no gap or finding.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::foundation::core::{DynoError, Value};
use crate::graph::DesignGraph;
use crate::inference::{InferenceReading, ObjectionReading, ProofStatus, Truth};
use crate::nodes::{edge, node};

/// How many nodes the walk reads when the caller does not say.
pub const REASONING_DEFAULT_LIMIT: usize = 60;
/// The most it reads, whatever the caller says.
pub const REASONING_MAX_LIMIT: usize = 500;

/// WHAT THESE CHECKS MEAN, AND WHAT THEY DO NOT. Carried on every read-back
/// (requirement A, part 4: "structure is not truth").
pub const WHAT_THIS_MEANS: &str = "This read-back shows the STRUCTURE of the reasoning: which \
     claims a recorded step supports and whether the step's rule holds over its premises; what \
     the reasoning rests on without showing it; which objections stand, which are answered and \
     which refute; which disputes the owner keeps open on purpose; and which claims and premises \
     cite a source. STRUCTURE IS NOT TRUTH: a claim with every step, source and reply recorded \
     can still be wrong. reflow2 never checks that a premise is true, that a source says what it \
     is cited for, or that a warrant is good; a reader challenges those. A source counts as cited \
     only when the design names it (an Artifact that DOCUMENTS the claim, with where to open it), \
     never when a model recalls it. 'Contested, by design' is the owner's choice to keep both \
     sides, not unfinished work. Nothing here is legal, medical or other professional advice.";

/// What the report says when the design records no reasoning to read.
const NOTHING_TO_READ: &str = "This design records no inference step and no contested decision, \
     so there is no line of reasoning to read back. A claim's support is recorded with \
     add_inference (its premises, its rule and its warrant); an objection is a CONTRADICTS from \
     the objection to the claim, and a reply a CONTRADICTS from the reply to the objection; a \
     dispute kept open on purpose is a Decision set to `contested` that governs both sides. Pass \
     `claim` to read one node and what stands against it.";

/// One step that supports a claim, with its warrant.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SupportStep {
    /// The step's verdict over its premises, each premise with its truth.
    #[serde(flatten)]
    pub reading: InferenceReading,
    /// WHY its premises support its conclusion, as the step records it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warrant: Option<String>,
}

/// A source cited for a claim: an Artifact that DOCUMENTS it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CitedSource {
    /// The Artifact's id.
    pub id: String,
    /// Its name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Where a person opens it (a path or an address), when recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// Whether a checksum pins the version that was read.
    pub pinned: bool,
}

/// One node of the reasoning, read.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ClaimReading {
    /// The node's id.
    pub id: String,
    /// Its type.
    pub node_type: String,
    /// Its name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Computed — holds, fails or unknown — never the node's own status.
    pub truth: Truth,
    /// Why, in a sentence.
    pub why: String,
    /// For a node a step concludes: proven, shown, contested or unproven.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_status: Option<ProofStatus>,
    /// Where the record came from, as it says (`authored`, `inferred` ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
    /// The steps that conclude it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub support: Vec<SupportStep>,
    /// What stands against it, and how each stands: `refutes`, `stands` or
    /// `answered`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub objections: Vec<ObjectionReading>,
    /// What it stands against, when it is an objection or a reply.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub objects_to: Vec<String>,
    /// What corroborates it (CONTRADICTS marked `supporting`): listed, never
    /// counted as support.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub corroborated_by: Vec<String>,
    /// Sources cited for it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<CitedSource>,
    /// The `contested` Decision that keeps its dispute open on purpose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contested_by_design: Option<String>,
}

/// A point still contested: an objection stands against the claim, neither
/// established nor answered, and no `contested` Decision keeps it open on
/// purpose.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StillContested {
    /// The claim.
    pub claim: String,
    /// Its name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// For it: the steps that conclude it.
    pub for_it: Vec<String>,
    /// Against it: the objections that stand.
    pub against: Vec<String>,
}

/// A dispute kept open on purpose: a `contested` Decision and the sides it
/// governs.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HeldOpen {
    /// The Decision.
    pub decision: String,
    /// Its name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The nodes it governs: the sides kept.
    pub sides: Vec<String>,
}

/// A claim with no support.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Unsupported {
    /// The node.
    pub id: String,
    /// Its type.
    pub node_type: String,
    /// Its name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Why it reads as unsupported.
    pub why: String,
}

/// A line of reasoning, read back.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReasoningReport {
    /// The claim asked about, when one was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim: Option<String>,
    /// Where the walk started.
    pub roots: Vec<String>,
    /// Every node of the reasoning, in the order the walk reached it.
    pub claims: Vec<ClaimReading>,
    /// What the reasoning rests on without showing it: every premise of a
    /// step in it that no step concludes. Each is read in `claims`.
    pub rests_on: Vec<String>,
    /// Premises that cite no source: they rest on someone's word. Fine for a
    /// value or a premise stated as given; a factual one needs a source.
    pub uncited_premises: Vec<String>,
    /// Points still contested.
    pub still_contested: Vec<StillContested>,
    /// Disputes kept open on purpose.
    pub contested_by_design: Vec<HeldOpen>,
    /// Claims with no support: no step concludes them, no source is cited,
    /// and on their own they are unknown.
    pub unsupported: Vec<Unsupported>,
    /// How many nodes the walk read.
    pub walked: usize,
    /// Whether `limit` stopped the walk before it reached everything.
    pub capped: bool,
    /// Why there is nothing to read, and how reasoning is recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// What these checks mean, and what they do not.
    pub what_this_means: &'static str,
}

fn prop(props: &std::collections::HashMap<String, Value>, key: &str) -> Option<String> {
    props.get(key).and_then(Value::as_str).map(str::to_string)
}

fn is_supporting(props: &std::collections::HashMap<String, Value>) -> bool {
    props.get("alignment").and_then(Value::as_str) == Some("supporting")
}

impl DesignGraph {
    /// Read a line of reasoning back: from `claim`, or from every top
    /// conclusion and every `contested` Decision when `claim` is `None`.
    /// `limit` bounds the walk (default [`REASONING_DEFAULT_LIMIT`], at most
    /// [`REASONING_MAX_LIMIT`]). Refused when `claim` names no node.
    pub fn reasoning_report(
        &self,
        claim: Option<&str>,
        limit: Option<usize>,
    ) -> Result<ReasoningReport, DynoError> {
        let limit = limit
            .unwrap_or(REASONING_DEFAULT_LIMIT)
            .clamp(1, REASONING_MAX_LIMIT);
        let roots = match claim {
            Some(id) => {
                if self.type_holding(id)?.is_none() {
                    return Err(DynoError::Query(format!(
                        "no single node of the design holds '{id}', so there is no claim to read \
                         back; search_design finds a node by what it says"
                    )));
                }
                vec![id.to_string()]
            }
            None => self.reasoning_roots()?,
        };

        // THE WALK, breadth first from the roots, each level in id order.
        let mut order: Vec<String> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut queue: VecDeque<String> = roots.iter().cloned().collect();
        seen.extend(roots.iter().cloned());
        let mut capped = false;
        while let Some(id) = queue.pop_front() {
            if order.len() == limit {
                capped = true;
                break;
            }
            order.push(id.clone());
            for next in self.reasoning_neighbours(&id)? {
                if seen.insert(next.clone()) {
                    queue.push_back(next);
                }
            }
        }

        let mut claims = Vec::with_capacity(order.len());
        for id in &order {
            claims.push(self.claim_reading(id)?);
        }

        // WHAT IT RESTS ON: premises of steps in the reasoning that no step
        // concludes.
        let mut rests_on: BTreeSet<String> = BTreeSet::new();
        for c in &claims {
            for s in &c.support {
                for p in &s.reading.premises {
                    if self.inferences_concluding(&p.id)?.is_empty() {
                        rests_on.insert(p.id.clone());
                    }
                }
            }
        }
        let by_id: BTreeMap<&str, &ClaimReading> =
            claims.iter().map(|c| (c.id.as_str(), c)).collect();
        let uncited_premises: Vec<String> = rests_on
            .iter()
            .filter(|p| match by_id.get(p.as_str()) {
                Some(c) => c.node_type != node::ARTIFACT && c.sources.is_empty(),
                // A premise past the cap is not read here; nothing is said of it.
                None => false,
            })
            .cloned()
            .collect();

        let mut still_contested = Vec::new();
        let mut contested_by_design = Vec::new();
        let mut unsupported = Vec::new();
        for c in &claims {
            let standing: Vec<String> = c
                .objections
                .iter()
                .filter(|o| o.standing == "stands")
                .map(|o| o.id.clone())
                .collect();
            if !standing.is_empty() && c.contested_by_design.is_none() {
                still_contested.push(StillContested {
                    claim: c.id.clone(),
                    name: c.name.clone(),
                    for_it: c
                        .support
                        .iter()
                        .map(|s| s.reading.inference.clone())
                        .collect(),
                    against: standing,
                });
            }
            if self.is_contested_decision(&c.id)? {
                contested_by_design.push(HeldOpen {
                    decision: c.id.clone(),
                    name: c.name.clone(),
                    sides: self.governed_by_decision(&c.id)?,
                });
                continue;
            }
            if c.truth == Truth::Unknown
                && c.support.is_empty()
                && c.sources.is_empty()
                && c.node_type != node::ARTIFACT
            {
                unsupported.push(Unsupported {
                    id: c.id.clone(),
                    node_type: c.node_type.clone(),
                    name: c.name.clone(),
                    why: format!(
                        "no step concludes it, no source is cited for it, and on its own it is \
                         unknown: {}",
                        c.why
                    ),
                });
            }
        }

        let note = roots.is_empty().then(|| NOTHING_TO_READ.to_string());
        Ok(ReasoningReport {
            claim: claim.map(str::to_string),
            roots,
            walked: claims.len(),
            claims,
            rests_on: rests_on.into_iter().collect(),
            uncited_premises,
            still_contested,
            contested_by_design,
            unsupported,
            capped,
            note,
            what_this_means: WHAT_THIS_MEANS,
        })
    }

    /// Every top conclusion — concluded by a step, a premise of none — and
    /// every `contested` Decision, sorted.
    fn reasoning_roots(&self) -> Result<Vec<String>, DynoError> {
        let mut roots: BTreeSet<String> = BTreeSet::new();
        for i in self.scan_nodes(node::INFERENCE)? {
            if let Some(c) = self.conclusion_of(&i.node_id)?
                && self.outgoing(&c, Some(edge::PREMISE_OF))?.is_empty()
            {
                roots.insert(c);
            }
        }
        for d in self.scan_nodes(node::DECISION)? {
            if d.properties.get("status").and_then(Value::as_str) == Some("contested") {
                roots.insert(d.node_id);
            }
        }
        Ok(roots.into_iter().collect())
    }

    /// What the walk follows from one node, in id order.
    fn reasoning_neighbours(&self, id: &str) -> Result<Vec<String>, DynoError> {
        let mut next: BTreeSet<String> = BTreeSet::new();
        for s in self.inferences_concluding(id)? {
            next.extend(self.premises_of(&s)?);
        }
        for e in self.incoming(id, Some(edge::CONTRADICTS))? {
            if !is_supporting(&e.properties) {
                next.insert(e.from_id);
            }
        }
        if self.is_contested_decision(id)? {
            next.extend(self.governed_by_decision(id)?);
        }
        if let Some(d) = self.contested_governor(id)? {
            next.insert(d);
        }
        next.remove(id);
        Ok(next.into_iter().collect())
    }

    fn claim_reading(&self, id: &str) -> Result<ClaimReading, DynoError> {
        let node_type = self.type_holding(id)?.unwrap_or_default();
        let props = match node_type.is_empty() {
            true => Default::default(),
            false => self
                .get_node(&node_type, id)?
                .map(|n| n.properties)
                .unwrap_or_default(),
        };
        let (truth, why) = self.premise_truth(id)?;
        let mut support = Vec::new();
        for s in self.inferences_concluding(id)? {
            if let Some(reading) = self.inference_reading(&s)? {
                let warrant = self
                    .get_node(node::INFERENCE, &s)?
                    .and_then(|n| prop(&n.properties, "warrant"));
                support.push(SupportStep { reading, warrant });
            }
        }
        let mut objects_to: Vec<String> = Vec::new();
        for e in self.outgoing(id, Some(edge::CONTRADICTS))? {
            if !is_supporting(&e.properties) {
                objects_to.push(e.to_id);
            }
        }
        objects_to.sort();
        let mut corroborated_by: Vec<String> = Vec::new();
        for e in self.incoming(id, Some(edge::CONTRADICTS))? {
            if is_supporting(&e.properties) {
                corroborated_by.push(e.from_id);
            }
        }
        corroborated_by.sort();
        Ok(ClaimReading {
            id: id.to_string(),
            name: prop(&props, "name"),
            proof_status: self.proof_reading(id)?.map(|p| p.status),
            provenance: prop(&props, "provenance"),
            objections: self.objections_to(id)?,
            sources: self.cited_sources(id)?,
            contested_by_design: self.contested_governor(id)?,
            node_type,
            truth,
            why,
            support,
            objects_to,
            corroborated_by,
        })
    }

    /// The Artifacts that DOCUMENT `id`, sorted.
    fn cited_sources(&self, id: &str) -> Result<Vec<CitedSource>, DynoError> {
        let mut out = Vec::new();
        for e in self.incoming(id, Some(edge::DOCUMENTS))? {
            let Some(a) = self.get_node(node::ARTIFACT, &e.from_id)? else {
                continue;
            };
            out.push(CitedSource {
                name: prop(&a.properties, "name"),
                location: prop(&a.properties, "location"),
                pinned: a
                    .properties
                    .get("checksum")
                    .and_then(Value::as_str)
                    .is_some_and(|c| !c.is_empty()),
                id: e.from_id,
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    fn is_contested_decision(&self, id: &str) -> Result<bool, DynoError> {
        Ok(self
            .get_node(node::DECISION, id)?
            .and_then(|n| prop(&n.properties, "status"))
            .is_some_and(|s| s == "contested"))
    }

    /// The `contested` Decision governing `id`, first in id order.
    fn contested_governor(&self, id: &str) -> Result<Option<String>, DynoError> {
        let mut found: Vec<String> = Vec::new();
        for e in self.outgoing(id, Some(edge::GOVERNED_BY))? {
            if self.is_contested_decision(&e.to_id)? {
                found.push(e.to_id);
            }
        }
        found.sort();
        Ok(found.into_iter().next())
    }

    /// The nodes a Decision governs, sorted.
    fn governed_by_decision(&self, decision: &str) -> Result<Vec<String>, DynoError> {
        let mut sides: Vec<String> = self
            .incoming(decision, Some(edge::GOVERNED_BY))?
            .into_iter()
            .map(|e| e.from_id)
            .collect();
        sides.sort();
        sides.dedup();
        Ok(sides)
    }
}
