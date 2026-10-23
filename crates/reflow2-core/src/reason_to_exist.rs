//! REASONING ATTACHED TO THE STRUCTURE IT JUSTIFIES.
//!
//! `req:an-edge-names-the-claim-that-argues-for-it-and-a-governed-requirement-reads-its-reason-to-exist`
//! (Anthony, 2026-10-18, phase 2 of the build list), with the two accepted
//! requirements it builds:
//! `req:a-derived-requirement-inherits-its-reasoning-from-the-decision-that-created-it`
//! and `req:a-choice-that-rests-on-checkable-premises-is-recorded-as-a-chain`.
//!
//! The 2026-10-18 proof of concept
//! (`fact:reasoning-embedded-in-a-traditional-design-works-within-one-design-and-stops-at-premises-with-no-truth-and-at-design-borders-2026-10-18`,
//! cases 2 and 4) measured reasoning that was recorded and then reached by
//! nothing that it justifies:
//!
//! - a requirement GOVERNED_BY a Decision concluded by its chain read only
//!   "delivered by the roll-up", naming neither the Decision nor its chain;
//! - when the Decision's support broke in a change walk, the requirements it
//!   governs were reached only by coincidence, as distance rings, and never
//!   named as losing their reason to exist;
//! - an edge argued for by a claim named the claim in prose (`evidence`), so
//!   breaking the claim touched nothing else, and the claim's premise "the
//!   requirement is delivered" was computed along the very edge it argued for.
//!
//! # What this module reads
//!
//! - [`DesignGraph::reason_to_exist`]: a requirement's REASON TO EXIST — the
//!   Decision(s) that govern it, each with its support as phase 1 reads it (the
//!   owner's acceptance, the chain's verdict beside it). Kept APART from whether
//!   the requirement is delivered: why it exists and whether it is met are two
//!   claims (item 2 of the derived-requirement requirement), so nothing here
//!   moves the requirement's truth, its delivery or its status.
//! - [`DesignGraph::edge_argument`]: the claim an edge names in `argued_by`,
//!   read beside the edge — its truth, its proof status, and whether it is
//!   CIRCULAR (its premise is the target's delivery computed along that same
//!   edge).
//! - [`DesignGraph::reasons_and_arguments_lost`]: what a change walk costs
//!   them — every requirement whose governing Decision the walk leaves without
//!   support, and every edge whose arguing claim it leaves unsupported or
//!   refuted.
//!
//! # A `parks` ruling is not a reason to exist
//!
//! `GOVERNED_BY ruling: parks` declares a node's unattached state deliberate
//! (a dated field report parked under its decision). It says nothing about why
//! the node exists, so it is never read here.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::foundation::core::{DynoError, Value};
use crate::graph::DesignGraph;
use crate::inference::{InferenceImpact, ProofStatus, Truth};
use crate::nodes::{edge, node};
use crate::reasoning::{ChainVerdict, ClaimTruth};

/// What [`ReasonToExist`] means, carried with it.
pub const REASON_TO_EXIST_NOTE: &str = "WHY THIS REQUIREMENT EXISTS, read apart from whether it \
     is delivered: the Decision(s) it is GOVERNED_BY, each with its support (the owner's \
     acceptance, and beside it the verdict of the chain that concludes the Decision, when one \
     does). A requirement is never the conclusion of its Decision's chain; its delivery is read \
     on its own. A `derived` requirement exists because of these Decisions, whether it is a \
     technical necessity one forced or the owner's words chosen through one: when a Decision \
     loses its support or is re-opened, the requirement may lose its reason to exist. An \
     `original` or `decomposed` one is its owner's word or its parent's piece, and these \
     Decisions shape it.";

/// One Decision a requirement is GOVERNED_BY, read as a reason to exist.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GoverningDecision {
    /// The Decision.
    pub decision: String,
    /// Its name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Its status as its OWNER set it (`accepted`, `proposed` ...).
    pub acceptance: String,
    /// What the chain concluding it yields on its own, read beside the
    /// acceptance; absent when no step concludes it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain: Option<ChainVerdict>,
    /// How it supports the requirement's existence: `accepted_and_its_chain_holds`,
    /// `accepted` (no chain recorded), `accepted_without_a_holding_chain`,
    /// `contested`, `not_settled`, `refuted` or `withdrawn`.
    pub support: &'static str,
    /// Why, in a sentence.
    pub why: String,
    /// WHAT THE DECISION'S CHAIN RESTS ON, all the way down: each terminal
    /// premise of the steps concluding it, with its truth and the
    /// `confidence` it records — the reports and assessments a requirement's
    /// reason to exist comes down to (req:a-requirements-read-back-follows-
    /// its-reason-to-the-evidence). Empty when no step concludes it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub chain_rests_on: Vec<EvidenceRead>,
}

/// One record a Decision's chain rests on, read for a requirement's reason
/// to exist.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EvidenceRead {
    /// The record.
    pub id: String,
    /// Its name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Its computed truth.
    pub truth: Truth,
    /// The confidence it records, as it records it; absent when it records
    /// none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<serde_json::Value>,
}

/// A requirement's reason to exist: see [`REASON_TO_EXIST_NOTE`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReasonToExist {
    /// Its lineage, as stored (`original` when unstated).
    pub lineage: String,
    /// The weakest support across its governing Decisions: `stands`,
    /// `stands_on_acceptance_alone`, `not_settled` or `lost`.
    pub reads: &'static str,
    /// Each Decision it is GOVERNED_BY (a `parks` ruling aside), by id.
    pub governed_by: Vec<GoverningDecision>,
    /// What this means.
    pub note: &'static str,
}

/// A requirement a Decision governs.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GovernedRequirement {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Its lineage, as stored (`original` when unstated).
    pub lineage: String,
}

/// What a change walk costs a governed requirement: its Decision's support.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReasonToExistLost {
    /// The Decision whose support the walk breaks.
    pub decision: String,
    /// How the walk leaves it: `counted_lost` (a seed: what if it no longer
    /// held, or were re-opened), `accepted_without_a_holding_chain` (it stands
    /// on its owner's acceptance and no line of its chain holds any more),
    /// `unsupported` or `refuted`.
    pub decision_reads: &'static str,
    /// Every requirement GOVERNED_BY it, each LOSING ITS REASON TO EXIST.
    pub requirements: Vec<GovernedRequirement>,
    /// Why, in a sentence.
    pub why: String,
}

/// The claim an edge names in `argued_by`, read beside the edge.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct EdgeArgument {
    /// The claim's id, as the edge names it.
    pub claim: String,
    /// Its type, or `None` when no single node holds the id.
    pub claim_type: Option<String>,
    /// Its name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Its computed truth: holds, conditional, fails or unknown.
    pub truth: ClaimTruth,
    /// For a claim a step concludes: proven, shown, conditional, contested or
    /// unproven.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_status: Option<ProofStatus>,
    /// What the argument does for the edge: `argued` (the claim holds),
    /// `argued_conditionally` (it holds under an open assumption),
    /// `unsupported`, `refuted`, `circular` (it holds only through the edge it
    /// argues for) or `missing` (the id names no node).
    pub reads: &'static str,
    /// Why, in a sentence.
    pub why: String,
    /// When `reads` is `circular`: the premise and how it rests on this edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circular: Option<String>,
}

/// One edge a claim argues for, as the claim's read-back lists it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ArguedEdge {
    pub edge_type: String,
    pub from_id: String,
    pub to_id: String,
    /// The argument's reading, as [`EdgeArgument::reads`] says it.
    pub reads: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circular: Option<String>,
}

/// What a change walk costs an argued edge: the claim arguing for it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ArgumentLost {
    pub edge_type: String,
    pub from_id: String,
    pub to_id: String,
    /// The claim the edge names in `argued_by`.
    pub claim: String,
    /// How the walk leaves the claim: `counted_lost` (a seed), `unsupported`
    /// or `refuted`.
    pub claim_reads: &'static str,
}

fn prop(props: &HashMap<String, Value>, key: &str) -> Option<String> {
    props.get(key).and_then(Value::as_str).map(str::to_string)
}

fn lineage_of(props: &HashMap<String, Value>) -> String {
    prop(props, "lineage").unwrap_or_else(|| "original".to_string())
}

/// How weak a support word is, for the weakest-across reading.
fn weakness(support: &str) -> u8 {
    match support {
        "accepted_and_its_chain_holds" | "accepted" => 0,
        "accepted_without_a_holding_chain" => 1,
        "contested" | "not_settled" => 2,
        _ => 3,
    }
}

impl DesignGraph {
    /// The Decisions `id` is GOVERNED_BY as a reason to exist, sorted: every
    /// outgoing GOVERNED_BY to a Decision except a `parks` ruling.
    fn governing_decisions(&self, id: &str) -> Result<Vec<String>, DynoError> {
        let mut out: Vec<String> = Vec::new();
        for e in self.outgoing(id, Some(edge::GOVERNED_BY))? {
            if e.properties.get("ruling").and_then(Value::as_str) == Some("parks") {
                continue;
            }
            if self.get_node(node::DECISION, &e.to_id)?.is_some() {
                out.push(e.to_id);
            }
        }
        out.sort();
        out.dedup();
        Ok(out)
    }

    /// Every Requirement GOVERNED_BY `decision` (a `parks` ruling aside),
    /// sorted by id.
    pub fn governed_requirements(
        &self,
        decision: &str,
    ) -> Result<Vec<GovernedRequirement>, DynoError> {
        let mut out: Vec<GovernedRequirement> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for e in self.incoming(decision, Some(edge::GOVERNED_BY))? {
            if e.properties.get("ruling").and_then(Value::as_str) == Some("parks") {
                continue;
            }
            if !seen.insert(e.from_id.clone()) {
                continue;
            }
            if let Some(n) = self.get_node(node::REQUIREMENT, &e.from_id)? {
                out.push(GovernedRequirement {
                    name: prop(&n.properties, "name"),
                    lineage: lineage_of(&n.properties),
                    id: e.from_id,
                });
            }
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// One Decision's support, as a reason to exist.
    fn governing_decision(&self, decision: &str) -> Result<GoverningDecision, DynoError> {
        let props = self
            .get_node(node::DECISION, decision)?
            .map(|n| n.properties)
            .unwrap_or_default();
        let acceptance = prop(&props, "status").unwrap_or_else(|| "proposed".to_string());
        let chain = self
            .chain_reading(decision)?
            .map(|(verdict, why)| ChainVerdict { verdict, why });
        let (truth, truth_why) = self.premise_truth(decision)?;
        let (support, why): (&'static str, String) = if self.is_discontinued(decision)? {
            (
                "withdrawn",
                "discontinued by an accepted decision".to_string(),
            )
        } else {
            match acceptance.as_str() {
                "rejected" | "superseded" => ("withdrawn", format!("the decision is {acceptance}")),
                "accepted" if truth == Truth::Fails => ("refuted", truth_why),
                "accepted" if truth == Truth::Unknown => ("contested", truth_why),
                "accepted" => match &chain {
                    None => (
                        "accepted",
                        "accepted by its owner; no chain of steps concludes it".to_string(),
                    ),
                    Some(c) if c.verdict == Truth::Holds => (
                        "accepted_and_its_chain_holds",
                        format!("accepted by its owner, and its chain holds: {}", c.why),
                    ),
                    Some(c) => (
                        "accepted_without_a_holding_chain",
                        format!(
                            "accepted by its owner, and no line of its chain holds ({}): {}",
                            c.verdict.as_str(),
                            c.why
                        ),
                    ),
                },
                "contested" => ("contested", truth_why),
                other => (
                    "not_settled",
                    format!("the decision is {other}, not settled"),
                ),
            }
        };
        let mut chain_rests_on = Vec::new();
        for f in self.foundations_of(decision)? {
            let fprops = match &f.node_type {
                Some(t) => self
                    .get_node(t, &f.id)?
                    .map(|n| n.properties)
                    .unwrap_or_default(),
                None => Default::default(),
            };
            chain_rests_on.push(EvidenceRead {
                name: prop(&fprops, "name"),
                confidence: fprops
                    .get("confidence")
                    .and_then(|v| serde_json::to_value(v).ok()),
                id: f.id,
                truth: f.truth,
            });
        }
        Ok(GoverningDecision {
            decision: decision.to_string(),
            name: prop(&props, "name"),
            acceptance,
            chain,
            chain_rests_on,
            support,
            why,
        })
    }

    /// A requirement's REASON TO EXIST (see the module documentation): the
    /// Decision(s) it is GOVERNED_BY and each one's support. `None` for a node
    /// that is not a Requirement or that no Decision governs.
    pub fn reason_to_exist(&self, id: &str) -> Result<Option<ReasonToExist>, DynoError> {
        let Some(n) = self.get_node(node::REQUIREMENT, id)? else {
            return Ok(None);
        };
        let decisions = self.governing_decisions(id)?;
        if decisions.is_empty() {
            return Ok(None);
        }
        let mut governed_by = Vec::with_capacity(decisions.len());
        for d in &decisions {
            governed_by.push(self.governing_decision(d)?);
        }
        let worst = governed_by
            .iter()
            .map(|g| weakness(g.support))
            .max()
            .unwrap_or(0);
        let reads = match worst {
            0 => "stands",
            1 => "stands_on_acceptance_alone",
            2 => "not_settled",
            _ => "lost",
        };
        Ok(Some(ReasonToExist {
            lineage: lineage_of(&n.properties),
            reads,
            governed_by,
            note: REASON_TO_EXIST_NOTE,
        }))
    }

    // ─── An edge's argument ────────────────────────────────────────────────

    /// The edge types whose schema declares `argued_by`, sorted.
    pub fn argued_edge_types(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .schema()
            .edge_types
            .iter()
            .filter(|(_, d)| d.properties.contains_key("argued_by"))
            .map(|(t, _)| t.clone())
            .collect();
        out.sort();
        out
    }

    /// The claim an edge names in `argued_by`, read beside the edge. `None`
    /// when the edge names none.
    pub fn edge_argument(
        &self,
        edge_type: &str,
        from_id: &str,
        to_id: &str,
        props: &BTreeMap<String, Value>,
    ) -> Result<Option<EdgeArgument>, DynoError> {
        let Some(claim) = props
            .get("argued_by")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|c| !c.is_empty())
        else {
            return Ok(None);
        };
        let Some(claim_type) = self.type_holding(claim)? else {
            return Ok(Some(EdgeArgument {
                claim: claim.to_string(),
                claim_type: None,
                name: None,
                truth: ClaimTruth::Unknown,
                proof_status: None,
                reads: "missing",
                why: "no single node of this design holds the id the edge names, so the edge \
                      has no argument to read"
                    .to_string(),
                circular: None,
            }));
        };
        let name = self
            .get_node(&claim_type, claim)?
            .and_then(|n| prop(&n.properties, "name"));
        let (truth, why) = self.premise_truth(claim)?;
        let assumes = self.assumptions_under(claim)?;
        let unverified = self.unverified_under(claim)?;
        let tests = self.tests_under(claim)?;
        let bridges = self.bridges_under(claim)?;
        let truth = ClaimTruth::of(truth, &assumes, &unverified, &bridges, &tests);
        let circular = if matches!(
            truth,
            ClaimTruth::Holds | ClaimTruth::Conditional | ClaimTruth::Supported
        ) {
            self.circular_along(edge_type, from_id, to_id, claim)?
        } else {
            None
        };
        let reads = match (truth, &circular) {
            (_, Some(_)) => "circular",
            (ClaimTruth::Holds, None) => "argued",
            (ClaimTruth::Conditional, None) => "argued_conditionally",
            (ClaimTruth::Supported, None) => "argued_from_evidence",
            (ClaimTruth::Fails, None) => "refuted",
            (ClaimTruth::Unknown, None) => "unsupported",
        };
        Ok(Some(EdgeArgument {
            claim: claim.to_string(),
            claim_type: Some(claim_type),
            name,
            truth,
            proof_status: self.proof_reading(claim)?.map(|p| p.status),
            reads,
            why,
            circular,
        }))
    }

    /// CIRCULARITY (item 4): a claim "X satisfies Y" one of whose premises,
    /// at any depth, is Y, where Y holds only because X delivers it ALONG
    /// THIS VERY EDGE. Without the edge Y would not be delivered, so the
    /// claim rests on what it argues for. Only a SATISFIES carries delivery;
    /// any other edge answers `None`.
    ///
    /// Y holds on another ground — another delivering satisfier, its children
    /// all delivered, a holding step of its own, its status `met`, or taken as
    /// given — and the argument is not circular.
    fn circular_along(
        &self,
        edge_type: &str,
        from_id: &str,
        to_id: &str,
        claim: &str,
    ) -> Result<Option<String>, DynoError> {
        if edge_type != edge::SATISFIES {
            return Ok(None);
        }
        if !self.rests_on_closure_of_node(claim)?.contains(to_id) {
            return Ok(None);
        }
        let Some(target) = self.get_node(node::REQUIREMENT, to_id)? else {
            return Ok(None);
        };
        if self.premise_truth(to_id)?.0 != Truth::Holds {
            return Ok(None);
        }
        let direct = self.requirement_directly_delivered_by(to_id)?;
        if direct.iter().all(|d| d != from_id) || direct.len() > 1 {
            return Ok(None);
        }
        if crate::inference::is_taken_as_given(&target.properties)
            || prop(&target.properties, "status").as_deref() == Some("met")
            || !self
                .holding_inferences_within(to_id, crate::inference::INFERENCE_DEPTH)?
                .is_empty()
        {
            return Ok(None);
        }
        let children = self.decomposed_children(to_id)?;
        if !children.is_empty() {
            let mut all = true;
            for c in &children {
                if self
                    .requirement_delivered_by_within(c, crate::inference::INFERENCE_DEPTH)?
                    .is_none()
                {
                    all = false;
                    break;
                }
            }
            if all {
                return Ok(None);
            }
        }
        Ok(Some(format!(
            "{claim} rests on {to_id}, and {to_id} holds only because {from_id} delivers it along \
             this {edge_type}: the argument for the edge assumes the edge. Argue from what \
             delivery itself rests on (the capability realized and its check passing), not from \
             the requirement's delivery"
        )))
    }

    /// Every edge naming a claim in `argued_by`, keyed by the claim: one scan
    /// of each edge type that declares it.
    pub fn edges_argued_by(&self) -> Result<BTreeMap<String, Vec<ArguedEdge>>, DynoError> {
        let mut out: BTreeMap<String, Vec<ArguedEdge>> = BTreeMap::new();
        let types = self.argued_edge_types();
        if types.is_empty() {
            return Ok(out);
        }
        let index = self.node_type_index()?;
        for t in &types {
            for e in self.all_edges_of_type(t, &index)? {
                let props: BTreeMap<String, Value> = e.properties.clone().into_iter().collect();
                let Some(a) = self.edge_argument(t, &e.from_id, &e.to_id, &props)? else {
                    continue;
                };
                out.entry(a.claim.clone()).or_default().push(ArguedEdge {
                    edge_type: t.clone(),
                    from_id: e.from_id,
                    to_id: e.to_id,
                    reads: a.reads,
                    circular: a.circular,
                });
            }
        }
        for v in out.values_mut() {
            v.sort_by(|a, b| {
                (&a.edge_type, &a.from_id, &a.to_id).cmp(&(&b.edge_type, &b.from_id, &b.to_id))
            });
        }
        Ok(out)
    }

    // ─── What a change walk costs them ─────────────────────────────────────

    /// What losing `seeds` costs the reasons to exist and the arguments the
    /// design records, read from the change walk's rows (`inference_walk`):
    ///
    /// - every Decision the walk leaves without support — a seed, or a
    ///   conclusion whose row does not read `still_supported` — with every
    ///   Requirement GOVERNED_BY it, each losing its reason to exist;
    /// - every edge whose `argued_by` claim the walk leaves without support.
    ///
    /// Both empty for a walk that reaches neither, which is every walk of a
    /// design that governs no requirement by a Decision and argues no edge.
    pub fn reasons_and_arguments_lost(
        &self,
        seeds: &[&str],
        rows: &[InferenceImpact],
    ) -> Result<(Vec<ReasonToExistLost>, Vec<ArgumentLost>), DynoError> {
        // Every node the walk leaves without support, with how it reads.
        let mut lost: BTreeMap<String, &'static str> = BTreeMap::new();
        for s in seeds {
            if self.type_holding(s)?.is_some() {
                lost.insert(s.to_string(), "counted_lost");
            }
        }
        for r in rows {
            let Some(c) = &r.conclusion else { continue };
            if r.conclusion_reads == "still_supported" || lost.contains_key(c) {
                continue;
            }
            lost.insert(c.clone(), r.conclusion_reads);
        }
        let mut reasons = Vec::new();
        for (id, reads) in &lost {
            if self.get_node(node::DECISION, id)?.is_none() {
                continue;
            }
            let requirements = self.governed_requirements(id)?;
            if requirements.is_empty() {
                continue;
            }
            let why = match *reads {
                "counted_lost" => format!(
                    "{id} is a seed of this walk: if it no longer held, or were re-opened, every \
                     requirement it governs would lose its reason to exist"
                ),
                "accepted_without_a_holding_chain" => format!(
                    "{id} stands on its owner's acceptance alone once the change is counted: no \
                     line of the chain that argued for it holds, so what it governs loses the \
                     reasoning behind its existence"
                ),
                other => format!(
                    "{id} reads {other} once the change is counted, so what it governs loses its \
                     reason to exist"
                ),
            };
            reasons.push(ReasonToExistLost {
                decision: id.clone(),
                decision_reads: reads,
                requirements,
                why,
            });
        }
        let mut arguments = Vec::new();
        if !self.argued_edge_types().is_empty() {
            let argued = self.edges_argued_by()?;
            for (claim, reads) in &lost {
                let Some(edges) = argued.get(claim) else {
                    continue;
                };
                for e in edges {
                    arguments.push(ArgumentLost {
                        edge_type: e.edge_type.clone(),
                        from_id: e.from_id.clone(),
                        to_id: e.to_id.clone(),
                        claim: claim.clone(),
                        claim_reads: reads,
                    });
                }
            }
        }
        Ok((reasons, arguments))
    }
}
