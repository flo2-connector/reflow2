//! reflow2's DERIVED relations — declared as data, evaluated by a served read.
//!
//! `req:reflow2-declares-its-derived-relations-and-serves-a-read-that-runs-them`,
//! promoted by Anthony on 2026-09-29. Until then reflow2 declared none of what
//! it computes: 23 derived relations (delivered, realized, checked,
//! discontinued, the seams, the blast radius, …) lived only as hand-written code
//! across about fifteen files, and the only list of them was the
//! graph-primitives study's hand-kept file
//! (`fact:root-cause-no-tool-can-evaluate-reflow2s-derived-relations-because-reflow2-declares-none-of-them-2026-09-29`).
//!
//! The declarations live in `schema/derived/relations.yaml`, beside the edge
//! readings, in the same vocabulary. Each names its reading in the sixteen
//! primitives, the rule it computes, whether it is deduced or induced, the edge
//! types it reads and the functions that compute it.
//!
//! ⚠️ A DECLARATION KEPT BESIDE HAND-WRITTEN CODE IS A SECOND COPY. It is honest
//! only while a test holds it to the code, and
//! `tests/derived_relations_are_declared_and_checked.rs` is that test: it reads
//! the named functions' source and fails when a declaration and its code
//! disagree about which edges the relation reads, or when a relation's code
//! calls a helper nobody declared.
//!
//! EVERY EVALUATOR HERE CALLS THE EXISTING CODE PATH. None re-implements a
//! relation — a second implementation would be exactly the drift the
//! declarations exist to prevent, and the designer's hand-written script that
//! prompted this (a re-derivation over the export) is the example.
//!
//! NOTHING IS STORED. Whether a derived result is KEPT is open in
//! `dec:idea-which-derived-results-are-kept-and-by-what-rule` and is the owner's
//! call; this read computes on demand.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::foundation::core::{DynoError, Reading};
use crate::graph::DesignGraph;
use crate::nodes::{edge, node};

/// The declarations, compiled in so a served binary always carries the list
/// that matches its own code.
const DECLARATIONS: &str = include_str!("../../../schema/derived/relations.yaml");

/// Every relation the served read can evaluate, in declaration-file order.
/// The test holds this list and the declaration file to each other in both
/// directions: a declaration nobody evaluates, or an evaluator nobody declared,
/// fails it.
pub const DERIVED_EVALUATORS: [&str; 23] = [
    "delivered",
    "realized",
    "checked",
    "discontinued",
    "contract_pair",
    "coupling",
    "coupling_at_altitude",
    "impact",
    "claimed_region",
    "level_mismatch",
    "rerun_owed",
    "requirement_certainty",
    "gaps_on_owned_ground",
    "assigned_decision",
    "shaping_decisions",
    "simulation_only",
    "unresolved_setup",
    "budget_rollup",
    "allocation_scores",
    "flow_step_order",
    "readiness_gate",
    "arrival_delta",
    "closure",
];

/// Where the code computes a relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeSite {
    /// The source file under `crates/reflow2-core/src/`.
    pub file: String,
    /// The function in it.
    pub function: String,
    /// True when the function computes nothing but this relation, so every edge
    /// it reads must be declared on it.
    #[serde(default)]
    pub dedicated: bool,
}

/// One derived relation, as `schema/derived/relations.yaml` declares it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedRelation {
    #[serde(default)]
    pub id: String,
    pub name: String,
    /// What the relation MEANS, in the shape of an edge reading.
    pub reading: Reading,
    /// What it computes, written over the declared readings where it can be.
    pub rule: String,
    /// `full`, `partial` or `none` — how much of the rule is stated over edge
    /// readings reflow2 declares.
    pub stated_over_readings: String,
    /// What no declared reading carries yet, when not `full`.
    #[serde(default)]
    pub not_yet_stated: Option<String>,
    /// `deduced` or `induced`.
    pub inference: String,
    /// The edge types the rule reads.
    #[serde(default)]
    pub edges: Vec<String>,
    /// Other derived relations this one is computed over.
    #[serde(default)]
    pub over: Vec<String>,
    /// Where the code computes it.
    pub code: Vec<CodeSite>,
    /// What ONE derived fact of it is.
    pub counts: String,
    /// `bounded`, `report`, `detectors` or `closure`.
    pub cost: String,
}

/// The values `stated_over_readings` may take.
pub const STATED_OVER_READINGS: [&str; 3] = ["full", "partial", "none"];
/// The values `inference` may take.
pub const INFERENCE_KINDS: [&str; 2] = ["deduced", "induced"];
/// The values `cost` may take.
pub const DERIVED_COSTS: [&str; 4] = ["bounded", "report", "detectors", "closure"];

#[derive(Deserialize)]
struct DeclarationFile {
    relations: BTreeMap<String, DerivedRelation>,
}

/// Every declared derived relation, sorted by id.
pub fn declared_derived_relations() -> Result<Vec<DerivedRelation>, DynoError> {
    let file: DeclarationFile = serde_yaml_ng::from_str(DECLARATIONS).map_err(|e| {
        DynoError::Schema(format!("schema/derived/relations.yaml does not parse: {e}"))
    })?;
    Ok(file
        .relations
        .into_iter()
        .map(|(id, mut r)| {
            r.id = id;
            r
        })
        .collect())
}

/// One relation's tally.
#[derive(Debug, Clone, Serialize)]
pub struct DerivedTally {
    pub id: String,
    pub name: String,
    /// The relation's reading in one line.
    pub reads_as: String,
    pub rule: String,
    pub inference: String,
    pub stated_over_readings: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_yet_stated: Option<String>,
    pub cost: String,
    /// What one fact of it is.
    pub counts: String,
    /// How many derived facts it yields over this design; absent when not counted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    /// Why it was not counted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_counted: Option<String>,
    /// A few of the facts it yields, by id.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sample: Vec<String>,
}

/// What `derived_report` answers.
#[derive(Debug, Clone, Serialize)]
pub struct DerivedReport {
    /// Relations declared.
    pub declared: usize,
    /// Relations counted in this read.
    pub counted: usize,
    /// Declared relations stated fully over edge readings.
    pub stated_fully_over_readings: usize,
    /// Derived facts across the counted relations.
    pub derived_facts: usize,
    /// Asserted facts the design stores: one per node, per non-null node
    /// property, per edge and per non-null edge property.
    pub asserted_facts: usize,
    /// derived_facts / asserted_facts, when anything is asserted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_per_asserted: Option<f64>,
    pub relations: Vec<DerivedTally>,
    /// What this read does and does not do.
    pub note: &'static str,
}

const REPORT_NOTE: &str = "Each relation is computed on read by the code path its declaration \
names; nothing is stored. A transitive closure (cost `closure`) is not counted: it is kept as a \
rule and answered per seed by propagate_from. Whether any derived result should be KEPT is an \
open decision (dec:idea-which-derived-results-are-kept-and-by-what-rule), not settled here.";

/// How many sample ids a relation carries by default.
pub const DEFAULT_DERIVED_SAMPLE: usize = 3;

struct Evaluated {
    count: Option<usize>,
    not_counted: Option<String>,
    sample: Vec<String>,
}

impl Evaluated {
    fn of(count: usize, sample: Vec<String>) -> Self {
        Evaluated {
            count: Some(count),
            not_counted: None,
            sample,
        }
    }
}

fn take(ids: impl IntoIterator<Item = String>, n: usize) -> Vec<String> {
    ids.into_iter().take(n).collect()
}

impl DesignGraph {
    /// Evaluate every declared derived relation over this design, by the code
    /// path each declaration names. `only`, when non-empty, narrows the read to
    /// those ids; an id nobody declared is refused rather than answered empty.
    pub fn derived_report(
        &self,
        only: &[String],
        sample: usize,
    ) -> Result<DerivedReport, DynoError> {
        let declared = declared_derived_relations()?;
        for id in only {
            if !declared.iter().any(|d| &d.id == id) {
                let known: Vec<&str> = declared.iter().map(|d| d.id.as_str()).collect();
                return Err(DynoError::Query(format!(
                    "no derived relation is declared as {id:?}; declared: {}",
                    known.join(", ")
                )));
            }
        }
        let mut relations = Vec::new();
        let mut derived_facts = 0usize;
        let mut counted = 0usize;
        for d in &declared {
            if !only.is_empty() && !only.contains(&d.id) {
                continue;
            }
            let e = self.evaluate_derived(&d.id, sample)?;
            if let Some(n) = e.count {
                derived_facts += n;
                counted += 1;
            }
            relations.push(DerivedTally {
                id: d.id.clone(),
                name: d.name.clone(),
                reads_as: d.reading.summary(),
                rule: d.rule.clone(),
                inference: d.inference.clone(),
                stated_over_readings: d.stated_over_readings.clone(),
                not_yet_stated: d.not_yet_stated.clone(),
                cost: d.cost.clone(),
                counts: d.counts.clone(),
                count: e.count,
                not_counted: e.not_counted,
                sample: e.sample,
            });
        }
        let asserted_facts = self.asserted_fact_count()?;
        Ok(DerivedReport {
            declared: declared.len(),
            counted,
            stated_fully_over_readings: declared
                .iter()
                .filter(|d| d.stated_over_readings == "full")
                .count(),
            derived_facts,
            asserted_facts,
            derived_per_asserted: (asserted_facts > 0)
                .then(|| derived_facts as f64 / asserted_facts as f64),
            relations,
            note: REPORT_NOTE,
        })
    }

    /// One node, one non-null node property, one edge, one non-null edge
    /// property: the stored facts a derived tally is compared with.
    fn asserted_fact_count(&self) -> Result<usize, DynoError> {
        let mut facts = 0usize;
        let mut types: Vec<&String> = self.schema().node_types.keys().collect();
        types.sort();
        for t in types {
            for n in self.scan_nodes(t)? {
                facts += 1 + n.properties.values().filter(|v| !v.is_null()).count();
                for e in self.outgoing(&n.node_id, None)? {
                    facts += 1 + e.properties.values().filter(|v| !v.is_null()).count();
                }
            }
        }
        Ok(facts)
    }

    /// One relation, by the code path its declaration names.
    fn evaluate_derived(&self, id: &str, n: usize) -> Result<Evaluated, DynoError> {
        let live = |t: &str| -> Result<Vec<String>, DynoError> {
            Ok(self
                .scan_live_nodes(t)?
                .into_iter()
                .map(|x| x.node_id)
                .collect())
        };
        let pair = |(a, b): &(String, String)| format!("{a} ↔ {b}");
        Ok(match id {
            "delivered" => {
                let cov = self.delivery_coverage()?;
                let mut hits = Vec::new();
                for r in live(node::REQUIREMENT)? {
                    if hits.len() >= n {
                        break;
                    }
                    if self.requirement_is_delivered(&r)? {
                        hits.push(r);
                    }
                }
                Evaluated::of(cov.delivered, hits)
            }
            "realized" => {
                let mut hits = Vec::new();
                for c in live(node::CAPABILITY)? {
                    if self.capability_is_realized(&c)? {
                        hits.push(c);
                    }
                }
                Evaluated::of(hits.len(), take(hits, n))
            }
            "checked" => {
                let mut hits = Vec::new();
                for c in live(node::CAPABILITY)? {
                    if !matches!(
                        self.capability_verification(&c)?,
                        crate::verify::CapabilityVerification::Unchecked
                    ) {
                        hits.push(c);
                    }
                }
                Evaluated::of(hits.len(), take(hits, n))
            }
            "discontinued" => {
                let ids: BTreeSet<String> = self.discontinued_ids()?.into_iter().collect();
                Evaluated::of(ids.len(), take(ids, n))
            }
            "contract_pair" => {
                let s = self.seam_sets_at(None)?;
                Evaluated::of(s.declared.len(), take(s.declared.iter().map(pair), n))
            }
            "coupling" => {
                let s = self.seam_sets_at(None)?;
                Evaluated::of(s.couplings.len(), take(s.couplings.iter().map(pair), n))
            }
            "coupling_at_altitude" => {
                let mut levels = BTreeSet::new();
                for c in self.scan_live_nodes(node::COMPONENT)? {
                    if let Some(l) = c.properties.get("level").and_then(|v| v.as_str()) {
                        levels.insert(l.to_string());
                    }
                }
                let mut count = 0usize;
                let mut sample = Vec::new();
                for l in &levels {
                    let sc = self.seam_coverage(Some(l))?;
                    count += sc.couplings + sc.covered;
                    sample.push(format!(
                        "{l}: {} couplings, {} covered",
                        sc.couplings, sc.covered
                    ));
                }
                Evaluated::of(count, take(sample, n))
            }
            "impact" => Evaluated {
                count: None,
                not_counted: Some(
                    "a transitive closure (cost `closure`): kept as a rule, answered per seed by \
                     propagate_from. Counting it for every node would cost one walk per node, \
                     and on reflow2's own design it measured about as many pairs one way as \
                     every asserted fact, and up to 60× both ways."
                        .to_string(),
                ),
                sample: Vec::new(),
            },
            "claimed_region" => {
                let rep = self.claim_report()?;
                let mut count = rep.overlaps.len();
                let mut sample = Vec::new();
                for c in &rep.claims {
                    let region = self.claimed_region(c)?;
                    count += region.len();
                    sample.extend(region);
                }
                Evaluated::of(count, take(sample, n))
            }
            "level_mismatch" => {
                let issues = self.hierarchy_issues()?;
                let sample = issues.iter().flat_map(|i| i.components.clone());
                Evaluated::of(issues.len(), take(sample, n))
            }
            "rerun_owed" => {
                let found = self.invalidated_findings()?;
                let sample = found.iter().map(|f| f.finding_id.clone());
                Evaluated::of(found.len(), take(sample, n))
            }
            "requirement_certainty" => {
                let b = self.requirement_certainty_breakdown()?;
                Evaluated::of(
                    b.user_confirmed + b.asserted + b.recovered,
                    vec![format!(
                        "user-confirmed {}, asserted {}, recovered {}",
                        b.user_confirmed, b.asserted, b.recovered
                    )],
                )
            }
            "gaps_on_owned_ground" => {
                let mut count = 0usize;
                let mut sample = Vec::new();
                for c in self.scan_nodes(node::CONTRIBUTOR)? {
                    if self.owned_by_contributor(&c.node_id)?.is_empty() {
                        continue;
                    }
                    let ls = self.loop_status_for(Some(&c.node_id))?;
                    count += ls.gaps_on_owned_ground.len();
                    if ls.gaps_on_owned_ground.is_empty() {
                        continue;
                    }
                    sample.push(format!("{}: {}", c.node_id, ls.gaps_on_owned_ground.len()));
                }
                Evaluated::of(count, take(sample, n))
            }
            "assigned_decision" => {
                let ls = self.loop_status_for(None)?;
                let sample = ls
                    .assigned_decisions
                    .iter()
                    .map(|a| serde_json::to_value(a).ok())
                    .filter_map(|v| {
                        v.and_then(|v| {
                            v.get("decision_id")
                                .and_then(|d| d.as_str().map(str::to_string))
                        })
                    });
                Evaluated::of(ls.assigned_decisions.len(), take(sample, n))
            }
            "shaping_decisions" => {
                let w = self.what_next(usize::MAX)?;
                let sample = w.shaping.iter().map(|s| s.decision_id.clone());
                Evaluated::of(w.shaping.len(), take(sample, n))
            }
            "simulation_only" => {
                let r = self.evidence_report()?;
                Evaluated::of(r.simulation_only, Vec::new())
            }
            "unresolved_setup" => {
                let sweep = self.detect_defects()?;
                let hits: Vec<&crate::heal::HealIssue> = sweep
                    .defects
                    .iter()
                    .filter(|d| matches!(d.category, crate::heal::HealCategory::UnresolvedSetup))
                    .collect();
                let sample = hits.iter().map(|d| d.id.clone());
                Evaluated::of(hits.len(), take(sample, n))
            }
            "budget_rollup" => {
                let s = self.budget_reports()?;
                let sample = s
                    .budgets
                    .iter()
                    .map(|b| serde_json::to_value(b).ok())
                    .filter_map(|v| {
                        v.and_then(|v| {
                            v.get("constraint_id")
                                .and_then(|d| d.as_str().map(str::to_string))
                        })
                    });
                Evaluated::of(s.budgets.len(), take(sample, n))
            }
            "allocation_scores" => {
                let r = self.evaluate_allocation()?;
                Evaluated::of(r.components.len(), Vec::new())
            }
            "flow_step_order" => {
                let mut count = 0usize;
                let mut sample = Vec::new();
                for f in live(node::FLOW)? {
                    let steps = self.flow_report(&f)?.steps.len();
                    count += steps * steps.saturating_sub(1) / 2;
                    sample.push(format!("{f}: {steps} steps"));
                }
                Evaluated::of(count, take(sample, n))
            }
            "readiness_gate" => {
                let mut count = 0usize;
                let mut sample = Vec::new();
                for (subject, _) in self.node_type_index()? {
                    if self.outgoing(&subject, Some(edge::GATED_ON))?.is_empty() {
                        continue;
                    }
                    let gates = self.readiness_report(&subject)?.gates.len();
                    count += gates;
                    sample.push(subject);
                }
                sample.sort();
                Evaluated::of(count, take(sample, n))
            }
            "arrival_delta" => {
                let mut count = 0usize;
                let mut sample = Vec::new();
                for t in [node::DESIGN_EPOCH, node::RELEASE] {
                    for target in live(t)? {
                        let items = self.arrival_delta(&target)?.items.len();
                        if items == 0 {
                            continue;
                        }
                        count += items;
                        sample.push(format!("{target}: {items}"));
                    }
                }
                Evaluated::of(count, take(sample, n))
            }
            "closure" => {
                let r = self.closure_report()?;
                let sample = r
                    .legs
                    .iter()
                    .map(|l| serde_json::to_value(l).ok())
                    .filter_map(|v| {
                        v.and_then(|v| v.get("leg").and_then(|d| d.as_str().map(str::to_string)))
                    });
                Evaluated::of(r.legs.len() + r.design.legs.len(), take(sample, n))
            }
            other => {
                return Err(DynoError::Schema(format!(
                    "derived relation {other:?} is declared but has no evaluator"
                )));
            }
        })
    }
}
