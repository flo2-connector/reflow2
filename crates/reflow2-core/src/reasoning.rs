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
use crate::inference::{
    AssumptionReading, IndependenceReading, InferenceReading, ObjectionReading, ProofStatus,
    StatementReading, Truth,
};
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

/// How many ids each list of [`ReasoningDebt`] names; the counts are whole.
pub const REASONING_DEBT_LIST_CAP: usize = 20;

/// What a [`ReasoningDebt`] means, carried with it.
pub const REASONING_DEBT_NOTE: &str = "What the reasoning still owes. An UNCONFIRMED step is one \
     no person or checker has confirmed (`confirmed_how`); an UNCITED premise is one the \
     reasoning rests on that cites no source and is not marked taken as given; a claim SHOWN BUT \
     NOT PROVEN holds through steps not all of which are confirmed. And where the reasoning is \
     NOT AS STATED: an ASSUMPTION NO STEP DISCHARGES (what rests on it holds only under it), a \
     GENERALIZATION over a variable something beneath it fixes (the step does not hold), and a \
     STATED INDEPENDENCE the lines do not bear out (they share a step no named lemma accounts \
     for). Confirming a step is the owner's word, never an agent's. reasoning_report reads each \
     one in full.";

/// What the reasoning of a design still owes: counted by `loop_status` and
/// raised as one low-severity `unconfirmed_reasoning` gap.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReasoningDebt {
    /// How many Inference steps the design holds.
    pub steps: usize,
    /// Steps no person or checker has confirmed.
    pub unconfirmed_steps: usize,
    /// Premises the reasoning rests on that cite no source and are not taken
    /// as given.
    pub uncited_premises: usize,
    /// Claims that read `shown`: they hold, and a step under them is
    /// unconfirmed.
    pub shown_not_proven: usize,
    /// Premises marked taken as given: stated foundations.
    pub stated_foundations: usize,
    /// The first [`REASONING_DEBT_LIST_CAP`] of each, by id.
    pub unconfirmed_step_ids: Vec<String>,
    pub uncited_premise_ids: Vec<String>,
    pub shown_not_proven_ids: Vec<String>,
    /// Assumptions some step takes up that no step discharges.
    pub undischarged_assumptions: usize,
    pub undischarged_assumption_ids: Vec<String>,
    /// Steps generalizing over a variable something they rest on fixes: each
    /// does not hold.
    pub generalizations_not_holding: usize,
    pub generalization_not_holding_ids: Vec<String>,
    /// Steps whose stated independence the structure does not bear out.
    pub independence_not_borne_out: usize,
    pub independence_not_borne_out_ids: Vec<String>,
    /// How many ids each list may name.
    pub list_cap: usize,
    /// What these counts mean.
    pub note: &'static str,
}

impl ReasoningDebt {
    /// Whether anything is owed.
    pub fn is_owed(&self) -> bool {
        self.unconfirmed_steps + self.uncited_premises + self.shown_not_proven > 0
            || self.not_as_stated()
    }

    /// Whether confirming or citing is owed: an unconfirmed step, an uncited
    /// premise, or a claim shown but not proven.
    pub fn confirmation_owed(&self) -> bool {
        self.unconfirmed_steps + self.uncited_premises + self.shown_not_proven > 0
    }

    /// Whether the reasoning is NOT AS STATED somewhere: an undischarged
    /// assumption, a generalization that does not hold, or a stated
    /// independence the lines do not bear out.
    pub fn not_as_stated(&self) -> bool {
        self.undischarged_assumptions
            + self.generalizations_not_holding
            + self.independence_not_borne_out
            > 0
    }
}

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
    /// Sources cited for it. For a Verification, its kept records count too:
    /// an Artifact its `location` names, or that it PRODUCES.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<CitedSource>,
    /// Marked TAKEN AS GIVEN: a stated foundation, never an uncited premise.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub taken_as_given: bool,
    /// What the statement says it is (`statement_kind`) and how it reads:
    /// a conjecture reads `open` or `refuted`, a theorem `stated` until a
    /// step concludes it. Absent when it says nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement: Option<StatementReading>,
    /// The assumptions it holds under: `assumed` premises beneath it that no
    /// step on the way discharges.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assumes: Vec<String>,
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
    /// value; a factual one needs a source. A premise marked TAKEN AS GIVEN is
    /// not here: it is in `stated_foundations`.
    pub uncited_premises: Vec<String>,
    /// Premises marked TAKEN AS GIVEN: definitions and axioms the reasoning
    /// states as its starting point.
    pub stated_foundations: Vec<String>,
    /// Points still contested.
    pub still_contested: Vec<StillContested>,
    /// Disputes kept open on purpose.
    pub contested_by_design: Vec<HeldOpen>,
    /// Claims with no support: no step concludes them, no source is cited,
    /// and on their own they are unknown.
    pub unsupported: Vec<Unsupported>,
    /// THE ASSUMPTIONS the reasoning makes, each with the steps that take it
    /// up and the steps that discharge it, and how it reads: "assumed,
    /// discharged by …" or "assumed, never discharged". Only those the walk
    /// reached; absent when it assumes nothing.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assumptions: Vec<AssumptionRead>,
    /// Every stated independence among the steps the walk reached, checked.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub independence: Vec<IndependenceReading>,
    /// Steps the walk reached that generalize over a variable something
    /// beneath them fixes: each does not hold (its reading says which).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub generalizations_not_holding: Vec<String>,
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

/// One assumption, as the read-back says it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AssumptionRead {
    #[serde(flatten)]
    pub assumption: AssumptionReading,
    /// "assumed, discharged by inf:…", or "assumed, never discharged: …".
    pub reads: String,
}

impl AssumptionRead {
    fn of(a: AssumptionReading) -> Self {
        let reads = if a.discharged_by.is_empty() {
            format!(
                "assumed, never discharged: what rests on it holds only under it (assumed in {})",
                a.assumed_in.join(", ")
            )
        } else {
            format!("assumed, discharged by {}", a.discharged_by.join(", "))
        };
        AssumptionRead {
            assumption: a,
            reads,
        }
    }
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

        // THE ASSUMPTIONS the walked steps take up, each read "assumed,
        // discharged by …" — on the list, and on every premise that is one.
        let walked_steps: BTreeSet<String> = claims
            .iter()
            .flat_map(|c| c.support.iter().map(|s| s.reading.inference.clone()))
            .collect();
        let assumptions: Vec<AssumptionRead> = self
            .assumptions()?
            .into_iter()
            .filter(|a| a.assumed_in.iter().any(|s| walked_steps.contains(s)))
            .map(AssumptionRead::of)
            .collect();
        let reads_of: BTreeMap<String, String> = assumptions
            .iter()
            .map(|a| (a.assumption.id.clone(), a.reads.clone()))
            .collect();
        for c in &mut claims {
            for s in &mut c.support {
                for p in &mut s.reading.premises {
                    if p.role == Some("assumed")
                        && let Some(r) = reads_of.get(&p.id)
                    {
                        p.why = format!("{r}; {}", p.why);
                    }
                }
            }
        }
        let mut independence = Vec::new();
        let mut generalizations_not_holding = Vec::new();
        for c in &claims {
            for s in &c.support {
                independence.extend(self.independence_of(&s.reading.inference)?);
                if s.reading.generalization.as_ref().is_some_and(|g| !g.holds) {
                    generalizations_not_holding.push(s.reading.inference.clone());
                }
            }
        }
        generalizations_not_holding.sort();
        generalizations_not_holding.dedup();

        // WHAT IT RESTS ON: premises of steps in the reasoning that no step
        // concludes. An ASSUMED premise is not something the reasoning rests
        // on: it is in `assumptions`.
        let mut rests_on: BTreeSet<String> = BTreeSet::new();
        for c in &claims {
            for s in &c.support {
                for p in &s.reading.premises {
                    if p.role != Some("assumed") && self.inferences_concluding(&p.id)?.is_empty() {
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
                Some(c) => {
                    c.node_type != node::ARTIFACT && c.sources.is_empty() && !c.taken_as_given
                }
                // A premise past the cap is not read here; nothing is said of it.
                None => false,
            })
            .cloned()
            .collect();
        let stated_foundations: Vec<String> = rests_on
            .iter()
            .filter(|p| by_id.get(p.as_str()).is_some_and(|c| c.taken_as_given))
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
            stated_foundations,
            still_contested,
            contested_by_design,
            unsupported,
            assumptions,
            independence,
            generalizations_not_holding,
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
            taken_as_given: crate::inference::is_taken_as_given(&props),
            statement: self.statement_reading(id)?,
            assumes: self.assumptions_under(id)?,
            contested_by_design: self.contested_governor(id)?,
            node_type,
            truth,
            why,
            support,
            objects_to,
            corroborated_by,
        })
    }

    /// The Artifacts that DOCUMENT `id`, sorted — and for a Verification,
    /// every record it names: one its `location` names, or that it PRODUCES
    /// (req:the-primes-trials-small-tool-snags-are-fixed, F21: a kept record
    /// counted as cited only with an explicit `documents` link).
    fn cited_sources(&self, id: &str) -> Result<Vec<CitedSource>, DynoError> {
        if self.get_node(node::VERIFICATION, id)?.is_some() {
            return Ok(self
                .records_of_check(id)?
                .into_iter()
                .map(|r| CitedSource {
                    pinned: r.pinned,
                    id: r.id,
                    name: r.name,
                    location: r.location,
                })
                .collect());
        }
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

    /// WHAT THE REASONING STILL OWES, design-wide — what `loop_status` and
    /// `detect_gaps` count (req:the-loop-sees-unconfirmed-steps-and-uncited-
    /// premises, Anthony 2026-10-17). `None` for a design holding no
    /// Inference, so nothing is said where there is no reasoning
    /// (con:no-gap-is-raised-for-a-conclusion-that-has-no-inference).
    pub fn reasoning_debt(&self) -> Result<Option<ReasoningDebt>, DynoError> {
        let steps = self.scan_nodes(node::INFERENCE)?;
        if steps.is_empty() {
            return Ok(None);
        }
        let mut unconfirmed: Vec<String> = Vec::new();
        let mut conclusions: BTreeSet<String> = BTreeSet::new();
        let mut terminal: BTreeSet<String> = BTreeSet::new();
        for s in &steps {
            let how = s
                .properties
                .get("confirmed_how")
                .and_then(Value::as_str)
                .unwrap_or("unconfirmed");
            if how != "person" && how != "checker" {
                unconfirmed.push(s.node_id.clone());
            }
            if let Some(c) = self.conclusion_of(&s.node_id)? {
                conclusions.insert(c);
            }
            for (p, assumed) in self.premise_links(&s.node_id)? {
                if !assumed && self.inferences_concluding(&p)?.is_empty() {
                    terminal.insert(p);
                }
            }
        }
        unconfirmed.sort();
        let mut uncited: Vec<String> = Vec::new();
        let mut given: Vec<String> = Vec::new();
        for p in &terminal {
            let Some(ty) = self.type_holding(p)? else {
                continue;
            };
            if ty == node::ARTIFACT {
                continue;
            }
            let props = self
                .get_node(&ty, p)?
                .map(|n| n.properties)
                .unwrap_or_default();
            if crate::inference::is_taken_as_given(&props) {
                given.push(p.clone());
            } else if self.cited_sources(p)?.is_empty() {
                uncited.push(p.clone());
            }
        }
        let mut shown: Vec<String> = Vec::new();
        for c in &conclusions {
            if self
                .proof_reading(c)?
                .is_some_and(|r| r.status == ProofStatus::Shown)
            {
                shown.push(c.clone());
            }
        }
        let cap = |mut v: Vec<String>| -> (usize, Vec<String>) {
            let n = v.len();
            v.truncate(REASONING_DEBT_LIST_CAP);
            (n, v)
        };
        let (unconfirmed_steps, unconfirmed_ids) = cap(unconfirmed);
        let (uncited_premises, uncited_ids) = cap(uncited);
        let (shown_not_proven, shown_ids) = cap(shown);
        let undischarged: Vec<String> = self
            .assumptions()?
            .into_iter()
            .filter(|a| a.discharged_by.is_empty())
            .map(|a| a.id)
            .collect();
        let (undischarged_assumptions, undischarged_ids) = cap(undischarged);
        let general: Vec<String> = self
            .generalizations_that_do_not_hold()?
            .into_iter()
            .map(|(s, _)| s)
            .collect();
        let (generalizations_not_holding, general_ids) = cap(general);
        let mut indep: Vec<String> = self
            .stated_independence()?
            .into_iter()
            .filter(|i| !i.holds)
            .map(|i| i.step)
            .collect();
        indep.sort();
        indep.dedup();
        let (independence_not_borne_out, indep_ids) = cap(indep);
        Ok(Some(ReasoningDebt {
            steps: steps.len(),
            unconfirmed_steps,
            uncited_premises,
            shown_not_proven,
            stated_foundations: given.len(),
            unconfirmed_step_ids: unconfirmed_ids,
            uncited_premise_ids: uncited_ids,
            shown_not_proven_ids: shown_ids,
            undischarged_assumptions,
            undischarged_assumption_ids: undischarged_ids,
            generalizations_not_holding,
            generalization_not_holding_ids: general_ids,
            independence_not_borne_out,
            independence_not_borne_out_ids: indep_ids,
            list_cap: REASONING_DEBT_LIST_CAP,
            note: REASONING_DEBT_NOTE,
        }))
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
