//! A KEPT COMPUTATION NAMES ITS INPUTS, AND A CHANGE SAYS WHAT MUST BE RE-RUN.
//!
//! The web of rules @ajs accepted on 2026-10-24
//! (`dec:idea-reflow2-becomes-the-web-of-rules-for-a-buildings-ifc-model`,
//! options (b) and (c)):
//!
//! - `req:a-changed-quantity-names-every-kept-computation-that-used-it`;
//! - `req:a-change-to-a-physical-designs-part-returns-its-redo-list-by-trade`.
//!
//! It was never built, and the Shop sample showed what that cost
//! (`fact:root-cause-a-changed-requirement-does-not-flow-because-the-accepted-
//! web-of-rules-was-never-built-2026-10-25`): a household changed from four to
//! five, the food calc's input cited the requirement but held its own copy of
//! "4", and nothing read owed.
//!
//! # The shape
//!
//! - A DESIGN QUANTITY is a Constraint that carries a `limit` (and its
//!   `unit`): "household = 5 people" is a setpoint Constraint. No new type: the
//!   Constraint already holds a named number with its unit, how it was obtained
//!   and where from, and `direction: setpoint` already means "a value the thing
//!   is held at, not a budget".
//! - A KEPT OUTPUT is an Artifact registered with its INPUTS: one `DEPENDS_ON`
//!   edge per input (`dependency_type: data_flow`, data passing from the input
//!   to the record), drawn when the record is registered, carrying what the
//!   record USED: `value_used` and `unit_used` for a quantity, `checksum_used`
//!   for another kept output (a calc record, a model a drawing was drawn
//!   from). `used_basis` marks the edge as an input and says whether the
//!   registrant stated what was used or the design's value at registration was
//!   taken. reflow2 never opens the record: whoever registers it says.
//! - An output is OWED (to be re-run or redrawn) when a quantity it used now
//!   holds another value or unit, when another output it used was re-run (its
//!   checksum moved), or when an output it used is itself owed. Transitive,
//!   with distances.
//! - A decision or limit RESTS ON a stale number when an owed output documents
//!   it, it depends on the output, its `limit_source` is the output, a check
//!   produced the output as its record, or the output is a premise of a step.
//!
//! THE DIVIDING LINE (2026-10-13) holds: computed values stay in the
//! calculator's files, and the design holds which numbers depend on which.
//! Nothing here recomputes.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use crate::foundation::core::{DynoError, Value};
use crate::foundation::store::StoredEdge;
use crate::graph::DesignGraph;
use crate::nodes::{Props, edge, node};

/// The property that marks a `DEPENDS_ON` edge as a kept output's input.
pub const USED_BASIS: &str = "used_basis";

/// One input a registrant names when it registers a kept output.
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct ArtifactInput {
    /// The design quantity (a Constraint with a `limit`) or the kept output
    /// (an Artifact) the record took as this input.
    pub node_id: String,
    /// The record's own name for the input (`persons`).
    pub input: Option<String>,
    /// For a quantity: the value the record used.
    pub value: Option<f64>,
    /// For a quantity: the unit of `value`. It must be the unit the design
    /// holds the quantity in.
    pub unit: Option<String>,
    /// For a kept output: the checksum of the version the record used.
    pub checksum: Option<String>,
}

/// What registering an input recorded.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RecordedInput {
    pub node_id: String,
    pub node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_used: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_used: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum_used: Option<String>,
    /// `stated` (the registrant said) or `current` (nobody said, so the
    /// design's value at registration was taken).
    pub used_basis: &'static str,
}

/// Why an output is owed, one input at a time.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OwedBecause {
    /// The input that moved.
    pub input_id: String,
    /// The record's name for it, when the registrant gave one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    /// `quantity_changed`, `input_re_run` or `input_owed`.
    pub reason: &'static str,
    /// What the record used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub was: Option<String>,
    /// What the design holds now.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub now: Option<String>,
}

/// A kept output owed a re-run (or a redraw).
#[derive(Debug, Clone, serde::Serialize)]
pub struct OwedComputation {
    pub artifact_id: String,
    pub name: String,
    /// `computation`, `drawing` or `model`, from its `artifact_type`.
    pub kind: &'static str,
    /// 1 = an input it used moved; n = it used an output owed at n-1.
    pub distance: usize,
    pub because: Vec<OwedBecause>,
}

/// A decision, limit or other record resting on an owed output.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RestingOnStale {
    pub node_id: String,
    pub node_type: String,
    pub name: String,
    /// The owed outputs it rests on.
    pub rests_on: Vec<String>,
    /// How it rests on them: `documents`, `depends_on`, `limit_source`,
    /// `check_record` or `premise`.
    pub how: Vec<&'static str>,
    /// One more than the nearest owed output's distance.
    pub distance: usize,
}

/// Every owed output and what rests on one.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct OwedReport {
    pub computations: Vec<OwedComputation>,
    pub resting_on_stale_numbers: Vec<RestingOnStale>,
}

/// One row of a redo list.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RedoRow {
    pub node_id: String,
    pub node_type: String,
    pub name: String,
    /// Hops from the nearest seed along the redo walk.
    pub distance: usize,
    /// The record it was reached through.
    pub via: String,
    /// Why it is on the list, in a sentence.
    pub because: String,
}

/// What a change to a part, a quantity or a kept output asks to be redone,
/// grouped by kind. Each group nearest first.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct RedoList {
    /// Kept computations to re-run.
    pub computations: Vec<RedoRow>,
    /// Decisions, limits and checks resting on those computations.
    pub decisions_and_limits: Vec<RedoRow>,
    /// Parts to redo: governed by those decisions, bounded by those limits,
    /// or depending on a changed part.
    pub parts: Vec<RedoRow>,
    /// Quantities describing a changed part, whose computations follow.
    pub quantities: Vec<RedoRow>,
    /// Model files that realize or document those parts.
    pub models: Vec<RedoRow>,
    /// Drawings and renders of those parts (REALIZES or DOCUMENTS), or made
    /// from those models or outputs.
    pub drawings: Vec<RedoRow>,
    /// What the list can and cannot say.
    pub note: &'static str,
}

impl RedoList {
    pub fn is_empty(&self) -> bool {
        self.computations.is_empty()
            && self.decisions_and_limits.is_empty()
            && self.parts.is_empty()
            && self.quantities.is_empty()
            && self.models.is_empty()
            && self.drawings.is_empty()
    }
}

const REDO_NOTE: &str = "Read from the links the design records, each group by its own: \
     `computations`, a kept output that took a reached record as an input (add_artifact \
     `inputs`); `decisions_and_limits`, what a reached kept output DOCUMENTS or is a PREMISE_OF, \
     the check that PRODUCES it, a limit whose limit_source it is; `parts`, a part GOVERNED_BY a \
     reached decision, CONSTRAINED by a reached limit, or that DEPENDS_ON a reached part or \
     output; `quantities`, a quantity that CONSTRAINS a reached part and that kept outputs took \
     as an input; `models` and `drawings`, an Artifact of that kind that REALIZES or DOCUMENTS a \
     reached part (the link drawn from the file to the part), or that took a reached record as \
     an input. A drawing or model is redrawn, not a reason to redo the other parts it shows. It \
     is only as right as those links: a dependency nobody drew is not on it. reflow2 re-runs and \
     redraws nothing; it says what must be.";

/// Is this edge a kept output's input?
pub fn is_input_edge(e: &StoredEdge) -> bool {
    e.edge_type == edge::DEPENDS_ON && e.properties.contains_key(USED_BASIS)
}

/// `computation`, `drawing` or `model`, from an Artifact's `artifact_type`.
pub fn output_kind(artifact_type: Option<&str>) -> &'static str {
    match artifact_type {
        Some("drawing" | "diagram" | "asset") => "drawing",
        Some("model") => "model",
        _ => "computation",
    }
}

/// A number as a person writes it: `4`, not `4.0`.
pub fn show_number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn with_unit(v: Option<f64>, unit: Option<&str>) -> Option<String> {
    v.map(|v| match unit {
        Some(u) if !u.trim().is_empty() => format!("{} {u}", show_number(v)),
        _ => show_number(v),
    })
}

fn prop_str<'a>(p: &'a HashMap<String, Value>, k: &str) -> Option<&'a str> {
    p.get(k).and_then(Value::as_str)
}

fn prop_f64(p: &HashMap<String, Value>, k: &str) -> Option<f64> {
    p.get(k).and_then(Value::as_f64)
}

fn invalid(property: &str, message: String) -> DynoError {
    DynoError::Validation {
        node_type: node::ARTIFACT.into(),
        property: property.into(),
        message,
    }
}

impl DesignGraph {
    /// Record the INPUTS a kept output took, replacing the ones it had: one
    /// `DEPENDS_ON` edge per input, carrying what the record used. Every input
    /// is checked before anything is written, so a refused call writes
    /// nothing. An empty list clears them.
    pub fn set_artifact_inputs(
        &mut self,
        artifact_id: &str,
        inputs: &[ArtifactInput],
    ) -> Result<Vec<RecordedInput>, DynoError> {
        if self.get_node(node::ARTIFACT, artifact_id)?.is_none() {
            return Err(DynoError::NodeNotFound {
                node_type: node::ARTIFACT.to_string(),
                node_id: artifact_id.to_string(),
            });
        }
        let mut seen = BTreeSet::new();
        let mut planned: Vec<(String, Props, RecordedInput)> = Vec::new();
        for i in inputs {
            let id = i.node_id.trim();
            if id == artifact_id {
                return Err(invalid(
                    "inputs",
                    format!("`{artifact_id}` cannot take itself as an input"),
                ));
            }
            if !seen.insert(id.to_string()) {
                return Err(invalid(
                    "inputs",
                    format!(
                        "`{id}` is named twice among the inputs; name each input once (one \
                         edge per input)"
                    ),
                ));
            }
            let (ty, recorded, props) = if let Some(q) = self.get_node(node::CONSTRAINT, id)? {
                let held = prop_f64(&q.properties, "limit");
                let unit = prop_str(&q.properties, "unit").map(str::to_string);
                if held.is_none() {
                    return Err(invalid(
                        "inputs",
                        format!(
                            "`{id}` holds no number: an input names a design quantity, a \
                             Constraint with a `limit` (add_constraint with `quantity`, `unit`, \
                             `limit` and `direction: setpoint`), or another kept output (an \
                             Artifact)"
                        ),
                    ));
                }
                if i.checksum.is_some() {
                    return Err(invalid(
                        "inputs",
                        format!(
                            "`{id}` is a quantity: give the `value` the record used, not a \
                             `checksum`"
                        ),
                    ));
                }
                if let (Some(given), Some(held_unit)) = (i.unit.as_deref(), unit.as_deref())
                    && given.trim() != held_unit.trim()
                {
                    return Err(invalid(
                        "inputs",
                        format!(
                            "`{id}` is held in {held_unit}, and the input says {given}: give \
                             the value the record used in {held_unit}, so a later change can \
                             be compared with it"
                        ),
                    ));
                }
                let (value, basis) = match i.value {
                    Some(v) => (v, "stated"),
                    None => (held.unwrap_or_default(), "current"),
                };
                let unit_used = i.unit.clone().or(unit);
                let props = Props::new()
                    .set("dependency_type", "data_flow")
                    .set(USED_BASIS, basis)
                    .set_opt("input", i.input.clone())
                    .set("value_used", value)
                    .set_opt("unit_used", unit_used.clone());
                (
                    node::CONSTRAINT,
                    RecordedInput {
                        node_id: id.to_string(),
                        node_type: node::CONSTRAINT.into(),
                        input: i.input.clone(),
                        value_used: Some(value),
                        unit_used,
                        checksum_used: None,
                        used_basis: basis,
                    },
                    props,
                )
            } else if let Some(a) = self.get_node(node::ARTIFACT, id)? {
                if i.value.is_some() || i.unit.is_some() {
                    return Err(invalid(
                        "inputs",
                        format!(
                            "`{id}` is a kept output: give the `checksum` of the version the \
                             record used (or nothing, for the version the design holds now), \
                             not a `value`"
                        ),
                    ));
                }
                let (checksum, basis) = match i.checksum.clone() {
                    Some(c) => (Some(c), "stated"),
                    None => (
                        prop_str(&a.properties, "checksum").map(str::to_string),
                        "current",
                    ),
                };
                let props = Props::new()
                    .set("dependency_type", "data_flow")
                    .set(USED_BASIS, basis)
                    .set_opt("input", i.input.clone())
                    .set_opt("checksum_used", checksum.clone());
                (
                    node::ARTIFACT,
                    RecordedInput {
                        node_id: id.to_string(),
                        node_type: node::ARTIFACT.into(),
                        input: i.input.clone(),
                        value_used: None,
                        unit_used: None,
                        checksum_used: checksum,
                        used_basis: basis,
                    },
                    props,
                )
            } else {
                let held_by = self.node_types_holding(id)?;
                return Err(invalid(
                    "inputs",
                    if held_by.is_empty() {
                        format!("`{id}` names nothing in this design")
                    } else {
                        format!(
                            "`{id}` is a {}, which holds no number a record can have used: an \
                             input names a design quantity, a Constraint with a `limit` \
                             (add_constraint with `quantity`, `unit`, `limit` and `direction: \
                             setpoint`), or another kept output (an Artifact). A number written \
                             in prose is first recorded as such a quantity",
                            held_by.join(" / ")
                        )
                    },
                ));
            };
            let _ = ty;
            planned.push((id.to_string(), props, recorded));
        }
        // Replace: an input the record no longer names goes.
        let keep: BTreeSet<&str> = planned.iter().map(|(id, _, _)| id.as_str()).collect();
        for e in self.outgoing(artifact_id, Some(edge::DEPENDS_ON))? {
            if is_input_edge(&e) && !keep.contains(e.to_id.as_str()) {
                self.delete_edge(edge::DEPENDS_ON, artifact_id, &e.to_id)?;
            }
        }
        let mut out = Vec::new();
        for (id, props, recorded) in planned {
            // Written whole, not merged: what a record used is replaced by
            // what its new version used, and a property the new call leaves
            // out must not survive from the old one.
            if self
                .outgoing(artifact_id, Some(edge::DEPENDS_ON))?
                .iter()
                .any(|e| e.to_id == id)
            {
                self.delete_edge(edge::DEPENDS_ON, artifact_id, &id)?;
            }
            self.create_edge(
                edge::DEPENDS_ON,
                node::ARTIFACT,
                artifact_id,
                &recorded.node_type,
                &id,
                props,
            )?;
            out.push(recorded);
        }
        Ok(out)
    }

    /// Every kept output's input edges, by the output's id.
    fn input_edges_by_output(&self) -> Result<BTreeMap<String, Vec<StoredEdge>>, DynoError> {
        let mut out: BTreeMap<String, Vec<StoredEdge>> = BTreeMap::new();
        for a in self.scan_nodes(node::ARTIFACT)? {
            let ins: Vec<StoredEdge> = self
                .outgoing(&a.node_id, Some(edge::DEPENDS_ON))?
                .into_iter()
                .filter(is_input_edge)
                .collect();
            if !ins.is_empty() {
                out.insert(a.node_id, ins);
            }
        }
        Ok(out)
    }

    /// Why an input edge reads moved now, if it does.
    fn input_moved(&self, e: &StoredEdge) -> Result<Option<OwedBecause>, DynoError> {
        let input = prop_str(&e.properties, "input").map(str::to_string);
        if let Some(q) = self.get_node(node::CONSTRAINT, &e.to_id)? {
            let used = prop_f64(&e.properties, "value_used");
            let used_unit = prop_str(&e.properties, "unit_used");
            let now = prop_f64(&q.properties, "limit");
            let now_unit = prop_str(&q.properties, "unit");
            let unit_moved =
                matches!((used_unit, now_unit), (Some(a), Some(b)) if a.trim() != b.trim());
            if used != now || unit_moved {
                return Ok(Some(OwedBecause {
                    input_id: e.to_id.clone(),
                    input,
                    reason: "quantity_changed",
                    was: with_unit(used, used_unit),
                    now: with_unit(now, now_unit),
                }));
            }
            return Ok(None);
        }
        if let Some(a) = self.get_node(node::ARTIFACT, &e.to_id)? {
            let used = prop_str(&e.properties, "checksum_used");
            let now = prop_str(&a.properties, "checksum");
            if used != now {
                return Ok(Some(OwedBecause {
                    input_id: e.to_id.clone(),
                    input,
                    reason: "input_re_run",
                    was: used.map(str::to_string),
                    now: now.map(str::to_string),
                }));
            }
        }
        Ok(None)
    }

    /// Every kept output owed a re-run now, nearest first, and every record
    /// resting on one.
    pub fn owed_computations(&self) -> Result<OwedReport, DynoError> {
        let inputs = self.input_edges_by_output()?;
        if inputs.is_empty() {
            return Ok(OwedReport::default());
        }
        // Who used each output: input id -> the outputs that took it.
        let mut consumers: BTreeMap<String, Vec<(String, Option<String>)>> = BTreeMap::new();
        let mut because: BTreeMap<String, Vec<OwedBecause>> = BTreeMap::new();
        let mut distance: BTreeMap<String, usize> = BTreeMap::new();
        let mut queue = VecDeque::new();
        for (out, edges) in &inputs {
            for e in edges {
                consumers.entry(e.to_id.clone()).or_default().push((
                    out.clone(),
                    prop_str(&e.properties, "input").map(str::to_string),
                ));
                if let Some(b) = self.input_moved(e)? {
                    because.entry(out.clone()).or_default().push(b);
                }
            }
            if because.contains_key(out) {
                distance.insert(out.clone(), 1);
                queue.push_back(out.clone());
            }
        }
        // Transitive: an output that used an owed one is owed one step further.
        while let Some(id) = queue.pop_front() {
            let d = distance[&id];
            for (c, input) in consumers.get(&id).cloned().unwrap_or_default() {
                let reasons = because.entry(c.clone()).or_default();
                if !reasons.iter().any(|b| b.input_id == id) {
                    reasons.push(OwedBecause {
                        input_id: id.clone(),
                        input,
                        reason: "input_owed",
                        was: None,
                        now: None,
                    });
                }
                if !distance.contains_key(&c) {
                    distance.insert(c.clone(), d + 1);
                    queue.push_back(c);
                }
            }
        }
        let mut computations = Vec::new();
        for (id, d) in &distance {
            let n = self.get_node(node::ARTIFACT, id)?;
            let props = n.as_ref().map(|n| &n.properties);
            computations.push(OwedComputation {
                artifact_id: id.clone(),
                name: props
                    .and_then(|p| prop_str(p, "name"))
                    .unwrap_or_default()
                    .to_string(),
                kind: output_kind(props.and_then(|p| prop_str(p, "artifact_type"))),
                distance: *d,
                because: because.remove(id).unwrap_or_default(),
            });
        }
        computations.sort_by(|a, b| {
            a.distance
                .cmp(&b.distance)
                .then(a.artifact_id.cmp(&b.artifact_id))
        });
        let resting = self.resting_on(&distance, &inputs)?;
        Ok(OwedReport {
            computations,
            resting_on_stale_numbers: resting,
        })
    }

    /// The records resting on any of `outputs` (id -> distance), other than
    /// the kept outputs that took them as inputs (those are owed themselves).
    fn resting_on(
        &self,
        outputs: &BTreeMap<String, usize>,
        inputs: &BTreeMap<String, Vec<StoredEdge>>,
    ) -> Result<Vec<RestingOnStale>, DynoError> {
        let index = self.node_type_index()?;
        let mut rows: BTreeMap<String, RestingOnStale> = BTreeMap::new();
        let mut note = |node_id: &str, on: &str, how: &'static str, d: usize| {
            let node_type = index.get(node_id).cloned().unwrap_or_default();
            let r = rows
                .entry(node_id.to_string())
                .or_insert_with(|| RestingOnStale {
                    node_id: node_id.to_string(),
                    node_type,
                    name: String::new(),
                    rests_on: Vec::new(),
                    how: Vec::new(),
                    distance: d + 1,
                });
            if !r.rests_on.iter().any(|x| x == on) {
                r.rests_on.push(on.to_string());
            }
            if !r.how.contains(&how) {
                r.how.push(how);
            }
            r.distance = r.distance.min(d + 1);
        };
        let limit_sources: Vec<(String, String)> = self
            .scan_nodes(node::CONSTRAINT)?
            .into_iter()
            .filter_map(|c| {
                prop_str(&c.properties, "limit_source").map(|s| (c.node_id.clone(), s.to_string()))
            })
            .collect();
        for (out, d) in outputs {
            for e in self.outgoing(out, Some(edge::DOCUMENTS))? {
                if !outputs.contains_key(&e.to_id) {
                    note(&e.to_id, out, "documents", *d);
                }
            }
            for e in self.outgoing(out, Some(edge::PREMISE_OF))? {
                note(&e.to_id, out, "premise", *d);
            }
            for e in self.incoming(out, Some(edge::DEPENDS_ON))? {
                if !is_input_edge(&e) && !inputs.contains_key(&e.from_id) {
                    note(&e.from_id, out, "depends_on", *d);
                }
            }
            for e in self.incoming(out, Some(edge::PRODUCES))? {
                note(&e.from_id, out, "check_record", *d);
            }
            for (c, src) in &limit_sources {
                if src == out {
                    note(c, out, "limit_source", *d);
                }
            }
        }
        let mut out: Vec<RestingOnStale> = rows.into_values().collect();
        for r in out.iter_mut() {
            if let Some(n) = self.get_node(&r.node_type, &r.node_id)? {
                r.name = prop_str(&n.properties, "name")
                    .unwrap_or_default()
                    .to_string();
            }
        }
        out.sort_by(|a, b| a.distance.cmp(&b.distance).then(a.node_id.cmp(&b.node_id)));
        Ok(out)
    }

    /// The REDO LIST of a change to `seeds` (parts, quantities, kept outputs,
    /// decisions): what it asks to be re-run, re-decided, rebuilt and redrawn,
    /// grouped by kind. `None` when it reaches nothing of the kind.
    pub fn redo_list(&self, seeds: &[&str]) -> Result<Option<RedoList>, DynoError> {
        let index = self.node_type_index()?;
        let inputs = self.input_edges_by_output()?;
        let mut consumers: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (out, edges) in &inputs {
            for e in edges {
                consumers
                    .entry(e.to_id.as_str())
                    .or_default()
                    .push(out.as_str());
            }
        }
        let limit_sources: Vec<(String, String)> = self
            .scan_nodes(node::CONSTRAINT)?
            .into_iter()
            .filter_map(|c| {
                prop_str(&c.properties, "limit_source").map(|s| (c.node_id.clone(), s.to_string()))
            })
            .collect();

        let mut w = RedoWalk {
            g: self,
            index: &index,
            list: RedoList {
                note: REDO_NOTE,
                ..RedoList::default()
            },
            seen: seeds.iter().map(|s| s.to_string()).collect(),
            queue: seeds
                .iter()
                .filter(|s| index.contains_key(**s))
                .map(|s| (s.to_string(), 0))
                .collect(),
        };

        while let Some((id, d)) = w.queue.pop_front() {
            let ty = index.get(&id).cloned().unwrap_or_default();
            let next = d + 1;
            // Anything a kept output took as an input: the outputs that used it.
            for c in consumers.get(id.as_str()).cloned().unwrap_or_default() {
                w.reach(c, &id, next, format!("it took {id} as an input"))?;
            }
            if ty == node::ARTIFACT {
                // What rests on a kept output. A drawing or a model DOCUMENTS
                // what it shows, and redrawing it asks nothing of the other
                // parts on the sheet: that link is walked from the part to the
                // file (below), never back out to the rest of what it shows
                // (fact:the-redo-list-reaches-a-drawing-or-model-only-by-
                // realizes-so-documents-edges-leave-it-empty-2026-10-26).
                let kind = self
                    .get_node(node::ARTIFACT, &id)?
                    .map(|n| output_kind(prop_str(&n.properties, "artifact_type")));
                if !matches!(kind, Some("model" | "drawing")) {
                    for e in self.outgoing(&id, Some(edge::DOCUMENTS))? {
                        w.reach(&e.to_id, &id, next, format!("{id} documents it"))?;
                    }
                }
                for e in self.outgoing(&id, Some(edge::PREMISE_OF))? {
                    w.reach(&e.to_id, &id, next, format!("{id} is a premise of it"))?;
                }
                for e in self.incoming(&id, Some(edge::DEPENDS_ON))? {
                    if !is_input_edge(&e) {
                        w.reach(&e.from_id, &id, next, format!("it depends on {id}"))?;
                    }
                }
                for e in self.incoming(&id, Some(edge::PRODUCES))? {
                    w.reach(&e.from_id, &id, next, format!("{id} is its kept record"))?;
                }
                for (c, src) in &limit_sources {
                    if src == &id {
                        w.reach(c, &id, next, format!("its limit came from {id}"))?;
                    }
                }
            } else if ty == node::DECISION {
                // The parts the decision shapes.
                for e in self.incoming(&id, Some(edge::GOVERNED_BY))? {
                    if index.get(&e.from_id).map(String::as_str) == Some(node::COMPONENT) {
                        w.reach(&e.from_id, &id, next, format!("it is governed by {id}"))?;
                    }
                }
            } else if ty == node::CONSTRAINT {
                // The parts a limit bounds, when the limit itself was reached
                // (a seed quantity is walked to its computations above).
                if d > 0 {
                    for e in self.outgoing(&id, Some(edge::CONSTRAINS))? {
                        if index.get(&e.to_id).map(String::as_str) == Some(node::COMPONENT) {
                            w.reach(&e.to_id, &id, next, format!("it is bounded by {id}"))?;
                        }
                    }
                }
            } else if ty == node::COMPONENT {
                // A part: the parts that depend on it, the quantities that
                // describe it, and the model files and drawings of it.
                for e in self.incoming(&id, Some(edge::DEPENDS_ON))? {
                    if index.get(&e.from_id).map(String::as_str) == Some(node::COMPONENT) {
                        w.reach(&e.from_id, &id, next, format!("it depends on {id}"))?;
                    }
                }
                for e in self.incoming(&id, Some(edge::CONSTRAINS))? {
                    let q = e.from_id.as_str();
                    let is_quantity = self
                        .get_node(node::CONSTRAINT, q)?
                        .is_some_and(|c| c.properties.contains_key("limit"));
                    if is_quantity && consumers.contains_key(q) && w.seen.insert(q.to_string()) {
                        let row = w.row(
                            q,
                            &id,
                            next,
                            format!("it describes {id}, and kept outputs took it as an input"),
                        )?;
                        w.list.quantities.push(row);
                        // Walked as a seed is: to its computations only.
                        for c in consumers.get(q).cloned().unwrap_or_default() {
                            w.reach(c, q, next + 1, format!("it took {q} as an input"))?;
                        }
                    }
                }
                // The model files and drawings of the part: one that REALIZES
                // it, and one that DOCUMENTS it — a sheet or a model drawn to
                // the part it shows, which is how a designer links a drawing.
                for (edge_type, verb) in
                    [(edge::REALIZES, "realizes"), (edge::DOCUMENTS, "documents")]
                {
                    for e in self.incoming(&id, Some(edge_type))? {
                        let kind = self
                            .get_node(node::ARTIFACT, &e.from_id)?
                            .map(|n| output_kind(prop_str(&n.properties, "artifact_type")));
                        if matches!(kind, Some("model" | "drawing")) {
                            w.reach(&e.from_id, &id, next, format!("it {verb} {id}"))?;
                        }
                    }
                }
            }
        }
        let mut list = w.list;
        for g in [
            &mut list.computations,
            &mut list.decisions_and_limits,
            &mut list.parts,
            &mut list.quantities,
            &mut list.models,
            &mut list.drawings,
        ] {
            g.sort_by(|a, b| a.distance.cmp(&b.distance).then(a.node_id.cmp(&b.node_id)));
        }
        Ok((!list.is_empty()).then_some(list))
    }
}

/// The redo walk's state: what it has listed and what it walks next.
struct RedoWalk<'a> {
    g: &'a DesignGraph,
    index: &'a HashMap<String, String>,
    list: RedoList,
    seen: BTreeSet<String>,
    queue: VecDeque<(String, usize)>,
}

impl RedoWalk<'_> {
    fn row(&self, id: &str, from: &str, d: usize, because: String) -> Result<RedoRow, DynoError> {
        let node_type = self.index.get(id).cloned().unwrap_or_default();
        let name = self
            .g
            .get_node(&node_type, id)?
            .and_then(|n| prop_str(&n.properties, "name").map(str::to_string))
            .unwrap_or_default();
        Ok(RedoRow {
            node_id: id.to_string(),
            node_type,
            name,
            distance: d,
            via: from.to_string(),
            because,
        })
    }

    /// Reach `id` from `from`: list it in its group and walk on from it.
    fn reach(&mut self, id: &str, from: &str, d: usize, because: String) -> Result<(), DynoError> {
        if !self.seen.insert(id.to_string()) {
            return Ok(());
        }
        let row = self.row(id, from, d, because)?;
        let group = if row.node_type == node::ARTIFACT {
            let t = self
                .g
                .get_node(node::ARTIFACT, id)?
                .and_then(|n| prop_str(&n.properties, "artifact_type").map(str::to_string));
            match output_kind(t.as_deref()) {
                "drawing" => &mut self.list.drawings,
                "model" => &mut self.list.models,
                _ => &mut self.list.computations,
            }
        } else if row.node_type == node::COMPONENT {
            &mut self.list.parts
        } else {
            &mut self.list.decisions_and_limits
        };
        group.push(row);
        self.queue.push_back((id.to_string(), d));
        Ok(())
    }
}
