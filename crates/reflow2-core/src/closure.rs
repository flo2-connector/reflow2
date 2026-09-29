//! The closure report — does the design CLOSE, against a threshold the owner
//! declared?
//! (`req:a-design-closes-against-a-declared-threshold-and-the-report-names-the-first-hole`,
//! Anthony 2026-09-16: "need to ensure that designs close and are verified").
//!
//! Five legs, every one a computation that already exists, summed into one
//! read:
//!
//! 1. TRACEABILITY — every live requirement traced. BUILD phase: to a
//!    capability that is realized with a passing check
//!    ([`DesignGraph::requirement_is_delivered`], the delivery line's own
//!    predicate). DESIGN phase: to a capability allocated to a part with a
//!    check planned. Parked requirements are counted as parked in both.
//! 2. BUDGETS — every Constraint with a `limit` inside it, with the declared
//!    `margin` ([`DesignGraph::budget_report`]). The BUILD phase also needs
//!    every numbered contribution `measured`; the DESIGN phase closes on an
//!    estimate that carries its basis.
//!
//! EVERY LEG IS READ TWICE, once per [`ClosurePhase`]: the top level of the
//! report is the BUILD reading, unchanged in meaning, and `design` carries
//! the DESIGN reading beside it against the same criterion. Each reading and
//! each leg names its phase, so "design done" is never quotable as "done".
//! 3. SEAMS — every coupling between parts specified on both sides
//!    ([`DesignGraph::seam_coverage`]).
//! 4. DECISIONS — no scheduled work governed by a decision still open
//!    (the same GOVERNED_BY / SCHEDULED_FOR read `what_next` scores).
//! 5. PROVENANCE — no quantity without a source
//!    ([`DesignGraph::quantity_provenance_sweep`], the detector's own sweep).
//!
//! THREE RULES THE SHAPE ENFORCES, each from the requirement:
//!
//! * A LEG SAYS WHAT IT SWEPT. `swept` is the population and `swept_note`
//!   says it in words ("budgets: 1 modelled"); a leg with NOTHING TO RUN ON
//!   carries `share: None` and `closes: None`, and a criterion that names it
//!   does not close — a detector reporting zero because it had nothing to
//!   check reads exactly like one that ran clean, and this report refuses
//!   that reading (the standing rule, `loop_status` applies it to an unknown
//!   contributor).
//! * THE THRESHOLD IS DECLARED, NEVER DEFAULTED. `Project.closure_legs` names
//!   which legs count and `Project.closure_threshold` the share of each that
//!   must close; 100 is a legal declaration and so is "traceability and
//!   budgets only". No declaration reads `no_closure_criterion_stated` — the
//!   legs are still computed and shown, the verdict is withheld.
//! * A REPORT, NEVER A GATE. Nothing here refuses a release or a commit; the
//!   release report may be cut while this says `does_not_close`, and says so.

use serde::Serialize;

use crate::budget::BudgetVerdict;
use crate::foundation::core::{DynoError, Value};
use crate::graph::DesignGraph;
use crate::nodes::{Props, edge, node};

/// The five legs, in the order the report walks them — which is also the
/// order `first_hole` is chosen in: the first leg in the DECLARED order that
/// does not close is the hole.
pub const CLOSURE_LEGS: &[&str] = &[
    "traceability",
    "budgets",
    "seams",
    "decisions",
    "provenance",
];

/// Which lifecycle phase a closure reading is FOR
/// (`req:closure-reads-design-done-separately-from-build-done`, Anthony
/// 2026-09-29). Every reading carries one, on the report and on each leg, so
/// "design done" can never be quoted as "done".
///
/// * `Build` — the top-level reading, unchanged in meaning and never
///   replaced: traceability is the delivery line (satisfied by a REALIZED
///   capability whose check PASSES), and a budget closes only on MEASURED
///   contributions.
/// * `Design` — carried beside it as `design`: traceability means satisfied
///   by a capability that is ALLOCATED to a part and has a check PLANNED (of
///   any status), and a budget closes on an estimate that carries its basis.
///   It says nothing about whether anything is built.
///
/// WHY TWO READINGS AND NOT A PHASE KNOB — option (c) of
/// `dec:idea-closure-has-a-design-phase-reading-and-a-build-phase-reading`,
/// chosen when the requirement was built. (a), a declared
/// `closure_phase` on the criterion, shows ONE answer, so a design-phase
/// "closes" would stand alone where it can be quoted as done; (b), a basis
/// per leg, adds five knobs and makes unasked combinations sayable; (d),
/// letting a passing analysis check close traceability, weakens what
/// "passing" means; (e), a separate report, splits the answer the owner asked
/// closure to give. Both readings share the one declared criterion, and the
/// criterion is not reopened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClosurePhase {
    Design,
    Build,
}

impl ClosurePhase {
    fn label(self) -> &'static str {
        match self {
            ClosurePhase::Design => "DESIGN phase",
            ClosurePhase::Build => "BUILD phase",
        }
    }
}

/// What the owner declared closure to mean.
#[derive(Debug, Clone, Serialize)]
pub struct ClosureCriterion {
    /// Which legs count, in the owner's order.
    pub legs: Vec<String>,
    /// The share of each counted leg that must close, 0–100.
    pub threshold: f64,
}

/// The first thing that keeps the design from closing.
#[derive(Debug, Clone, Serialize)]
pub struct ClosureHole {
    pub leg: String,
    /// The offending node or pair, where there is one; absent when the hole
    /// is the leg having nothing to run on.
    pub id: Option<String>,
    pub why: String,
}

/// One leg's reading.
#[derive(Debug, Clone, Serialize)]
pub struct ClosureLeg {
    pub leg: String,
    /// The phase this leg was read for — see [`ClosurePhase`].
    pub phase: ClosurePhase,
    /// Whether the declared criterion counts this leg. Every leg is computed
    /// and shown; only counted legs move the verdict.
    pub counted: bool,
    /// The population the leg swept.
    pub swept: usize,
    /// Items a `parks` ruling on an ACCEPTED decision declares deliberately
    /// open. They leave `swept` and are counted here, never as a hole — the
    /// same predicate every parking reader uses (`is_parked`).
    pub parked: usize,
    /// How many of them close.
    pub closed: usize,
    /// `closed / swept` as a percentage; `None` when nothing was swept.
    pub share: Option<f64>,
    /// What was swept, in words — so "0 open" beside "0 modelled" cannot
    /// read as clean.
    pub swept_note: String,
    /// The first offender in deterministic (sorted) order.
    pub worst: Option<ClosureHole>,
    /// Whether this leg meets the threshold; `None` when nothing was swept or
    /// no criterion is declared.
    pub closes: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClosureVerdict {
    Closes,
    DoesNotClose,
    NoClosureCriterionStated,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClosureReport {
    pub project_id: Option<String>,
    /// Always `build`: the top-level reading is the build phase, unchanged in
    /// meaning. The design phase is `design`, beside it.
    pub phase: ClosurePhase,
    pub criterion: Option<ClosureCriterion>,
    pub legs: Vec<ClosureLeg>,
    pub verdict: ClosureVerdict,
    /// The first counted leg, in declared order, that does not close — with
    /// its worst offender, or the fact that it had nothing to run on.
    pub first_hole: Option<ClosureHole>,
    pub note: String,
    /// The DESIGN-phase reading against the same criterion — see
    /// [`ClosurePhase`]. Never a substitute for the build verdict above.
    pub design: ClosureReading,
}

/// One phase's reading: its legs, verdict, first hole and note, each naming
/// the phase.
#[derive(Debug, Clone, Serialize)]
pub struct ClosureReading {
    pub phase: ClosurePhase,
    pub legs: Vec<ClosureLeg>,
    pub verdict: ClosureVerdict,
    pub first_hole: Option<ClosureHole>,
    pub note: String,
}

/// The verdict alone, for reports that carry closure beside their own answer
/// (the release report). The top-level fields are the BUILD reading; the
/// design reading's verdict rides beside it, named.
#[derive(Debug, Clone, Serialize)]
pub struct ClosureSummary {
    pub phase: ClosurePhase,
    pub verdict: ClosureVerdict,
    pub first_hole: Option<ClosureHole>,
    pub note: String,
    pub design_verdict: ClosureVerdict,
    pub design_first_hole: Option<ClosureHole>,
}

/// The provenance sweep, shared with the detector so both read one
/// definition.
#[derive(Debug, Clone, Default)]
pub struct QuantityProvenanceSweep {
    /// Constraints carrying a numeric `limit`.
    pub limits: usize,
    /// Every stated number: limits plus numbered contributions.
    pub quantities: usize,
    /// (affected id, what) for each number with no source, in sweep order.
    pub unsourced: Vec<(String, String)>,
    /// (check, constraint) for each Verification on a limit that no Artifact
    /// IMPLEMENTS, once per check.
    pub checks_without_form: Vec<(String, String)>,
}

fn is_review(id: &str) -> bool {
    id.starts_with("decision:ack:")
}

fn clean(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

impl DesignGraph {
    /// Declare what closure means for this project: which legs count and the
    /// share of each that must close. Validated here rather than by the
    /// schema because `closure_legs` is a list and the schema has no list
    /// enum; an unknown leg name fails loud with the five that exist.
    pub fn set_closure_criterion(
        &mut self,
        project_id: &str,
        legs: &[&str],
        threshold: f64,
    ) -> Result<crate::foundation::store::StoredNode, DynoError> {
        let Some(existing) = self.get_node(node::PROJECT, project_id)? else {
            return Err(DynoError::NodeNotFound {
                node_type: node::PROJECT.to_string(),
                node_id: project_id.to_string(),
            });
        };
        if legs.is_empty() {
            return Err(DynoError::Validation {
                node_type: node::PROJECT.into(),
                property: "closure_legs".into(),
                message: format!(
                    "a closure criterion names at least one leg (of {}); an empty list would \
                     make every design close",
                    CLOSURE_LEGS.join(", ")
                ),
            });
        }
        let mut seen = std::collections::BTreeSet::new();
        for leg in legs {
            if !CLOSURE_LEGS.contains(leg) {
                return Err(DynoError::Validation {
                    node_type: node::PROJECT.into(),
                    property: "closure_legs".into(),
                    message: format!(
                        "'{leg}' is not a closure leg (one of {})",
                        CLOSURE_LEGS.join(", ")
                    ),
                });
            }
            if !seen.insert(*leg) {
                return Err(DynoError::Validation {
                    node_type: node::PROJECT.into(),
                    property: "closure_legs".into(),
                    message: format!("'{leg}' is named twice"),
                });
            }
        }
        if !(0.0..=100.0).contains(&threshold) || threshold.is_nan() {
            return Err(DynoError::Validation {
                node_type: node::PROJECT.into(),
                property: "closure_threshold".into(),
                message: format!(
                    "{threshold} is not a share: the threshold is the percentage of each counted \
                     leg that must close, 0 to 100"
                ),
            });
        }
        let list: Vec<Value> = legs.iter().map(|l| Value::from(*l)).collect();
        let mut props = Props::new()
            .set("closure_legs", Value::List(list))
            .set("closure_threshold", threshold);
        for (k, v) in &existing.properties {
            if k != "closure_legs" && k != "closure_threshold" {
                props = props.set(k, v.clone());
            }
        }
        self.upsert_node(node::PROJECT, project_id, props)
    }

    /// The closure verdict alone, for a report that carries it beside its own.
    pub fn closure_summary(&self) -> Result<ClosureSummary, DynoError> {
        let r = self.closure_report()?;
        Ok(ClosureSummary {
            phase: r.phase,
            verdict: r.verdict,
            first_hole: r.first_hole,
            note: r.note,
            design_verdict: r.design.verdict,
            design_first_hole: r.design.first_hole,
        })
    }

    /// The one sweep behind `quantity_without_source`,
    /// `quantity_check_without_executable_form` and the provenance leg.
    pub fn quantity_provenance_sweep(&self) -> Result<QuantityProvenanceSweep, DynoError> {
        let mut s = QuantityProvenanceSweep::default();
        let mut checks_seen = std::collections::BTreeSet::new();
        for c in self.scan_live_nodes(node::CONSTRAINT)? {
            if c.properties.get("limit").and_then(Value::as_f64).is_none() {
                continue;
            }
            s.limits += 1;
            s.quantities += 1;
            if clean(c.properties.get("limit_source")).is_none() {
                s.unsourced
                    .push((c.node_id.clone(), format!("the limit of '{}'", c.node_id)));
            }
            for e in self.outgoing(&c.node_id, Some(edge::CONSTRAINS))? {
                let has_number = e
                    .properties
                    .get("contribution")
                    .and_then(Value::as_f64)
                    .is_some();
                if !has_number {
                    continue;
                }
                s.quantities += 1;
                if clean(e.properties.get("source")).is_none() {
                    s.unsourced.push((
                        e.to_id.clone(),
                        format!("the contribution of '{}' to '{}'", e.to_id, c.node_id),
                    ));
                }
            }
            for v in self.incoming(&c.node_id, Some(edge::VERIFIES))? {
                if !checks_seen.insert(v.from_id.clone()) {
                    continue;
                }
                if self
                    .incoming(&v.from_id, Some(edge::IMPLEMENTS))?
                    .is_empty()
                {
                    s.checks_without_form
                        .push((v.from_id.clone(), c.node_id.clone()));
                }
            }
        }
        Ok(s)
    }

    /// Does the design close? See the module docs for the five legs and the
    /// three rules.
    pub fn closure_report(&self) -> Result<ClosureReport, DynoError> {
        // The criterion lives on the Project. One project per graph is the
        // norm; with several, the first (sorted) that declares one is read
        // and named, so the choice is visible rather than silent.
        let mut projects = self.scan_live_nodes(node::PROJECT)?;
        projects.sort_by(|a, b| a.node_id.cmp(&b.node_id));
        let mut project_id = projects.first().map(|p| p.node_id.clone());
        let mut criterion: Option<ClosureCriterion> = None;
        for p in &projects {
            let legs: Vec<String> = match p.properties.get("closure_legs") {
                Some(Value::List(items)) => items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect(),
                _ => Vec::new(),
            };
            let threshold = p
                .properties
                .get("closure_threshold")
                .and_then(Value::as_f64);
            if let (false, Some(t)) = (legs.is_empty(), threshold) {
                project_id = Some(p.node_id.clone());
                criterion = Some(ClosureCriterion { legs, threshold: t });
                break;
            }
        }

        let build = self.closure_reading(ClosurePhase::Build, criterion.as_ref())?;
        let design = self.closure_reading(ClosurePhase::Design, criterion.as_ref())?;

        Ok(ClosureReport {
            project_id,
            phase: ClosurePhase::Build,
            criterion,
            legs: build.legs,
            verdict: build.verdict,
            first_hole: build.first_hole,
            note: build.note,
            design,
        })
    }

    /// One phase's reading against the declared criterion (or none). The
    /// legs, the verdict walk and the notes are the same code for both
    /// phases; only the traceability predicate and the budget's basis rule
    /// differ, and every sentence names the phase it is about.
    fn closure_reading(
        &self,
        phase: ClosurePhase,
        criterion: Option<&ClosureCriterion>,
    ) -> Result<ClosureReading, DynoError> {
        let counted = |leg: &str| -> bool {
            criterion
                .map(|c| c.legs.iter().any(|l| l == leg))
                .unwrap_or(false)
        };
        let threshold = criterion.map(|c| c.threshold);

        let mut legs: Vec<ClosureLeg> = vec![
            self.leg_traceability(phase)?,
            self.leg_budgets(phase)?,
            self.leg_seams(phase)?,
            self.leg_decisions(phase)?,
            self.leg_provenance(phase)?,
        ];

        for leg in &mut legs {
            leg.counted = counted(&leg.leg);
            leg.closes = match (threshold, leg.share) {
                (Some(t), Some(share)) if leg.counted => Some(share + 1e-9 >= t),
                _ => None,
            };
        }

        let label = phase.label();
        let about = match phase {
            ClosurePhase::Build => {
                "requirements delivered by built capabilities whose checks pass, and budgets \
                 closing on measured numbers"
            }
            ClosurePhase::Design => {
                "requirements traced to capabilities allocated to a part with a check planned, \
                 and budgets closing on estimates that carry their basis"
            }
        };
        let disclaimer = match phase {
            ClosurePhase::Build => {
                "This is the design's own chain — intent, budgets, seams, decisions and sources \
                 as recorded — and says nothing about whether the built thing does what its \
                 users need."
            }
            ClosurePhase::Design => {
                "This reading says nothing about whether anything is built or works: that is the \
                 top-level BUILD reading, which this never replaces."
            }
        };

        let (verdict, first_hole, note) = match criterion {
            None => (
                ClosureVerdict::NoClosureCriterionStated,
                None,
                format!(
                    "{label} — No closure criterion stated: the project has not declared which \
                     legs count or what share of each must close (set_closure_criterion). The \
                     legs are computed and shown; no verdict is offered in place of the owner's \
                     word, and no default stands in for it."
                ),
            ),
            Some(c) => {
                // Walk the DECLARED order, so the first hole is the first leg
                // the owner named that fails, not the first the report
                // happened to compute.
                let mut hole: Option<ClosureHole> = None;
                for name in &c.legs {
                    let Some(leg) = legs.iter().find(|l| &l.leg == name) else {
                        continue;
                    };
                    match leg.closes {
                        Some(true) => {}
                        Some(false) => {
                            hole = Some(leg.worst.clone().unwrap_or(ClosureHole {
                                leg: leg.leg.clone(),
                                id: None,
                                why: format!(
                                    "{:.0}% of {} closes; the criterion asks {:.0}%",
                                    leg.share.unwrap_or(0.0),
                                    leg.swept_note,
                                    c.threshold
                                ),
                            }));
                            break;
                        }
                        None => {
                            hole = Some(ClosureHole {
                                leg: leg.leg.clone(),
                                id: None,
                                why: format!(
                                    "nothing to run on — {}; a leg the criterion counts cannot \
                                     read as closed with nothing swept",
                                    leg.swept_note
                                ),
                            });
                            break;
                        }
                    }
                }
                match hole {
                    None => (
                        ClosureVerdict::Closes,
                        None,
                        format!(
                            "{label} — Closes: every counted leg ({}) meets {:.0}%, read as \
                             {about}. {disclaimer}",
                            c.legs.join(", "),
                            c.threshold
                        ),
                    ),
                    Some(h) => (
                        ClosureVerdict::DoesNotClose,
                        Some(h.clone()),
                        format!(
                            "{label} — Does not close: the first hole is on {} — {}. A report, \
                             not a gate: a release may still be cut, and the release report \
                             will say the design did not close. {disclaimer}",
                            h.leg, h.why
                        ),
                    ),
                }
            }
        };

        Ok(ClosureReading {
            phase,
            legs,
            verdict,
            first_hole,
            note,
        })
    }

    /// TRACEABILITY. Both phases sweep the live requirements, less the
    /// dropped and less the PARKED — a `parks` ruling on an accepted decision
    /// says the requirement's unsatisfied state is deliberate, so it is
    /// counted in `parked` and can never be the first hole (the sibling of
    /// `fact:root-cause-parking-is-still-not-named-where-an-unsatisfied-requirement-is-read-2026-09-29`
    /// that closure itself carried). A requirement inferred from what
    /// implements it proves nothing in either phase.
    ///
    /// * Build: the delivery line ([`DesignGraph::requirement_is_delivered`]).
    /// * Design: [`design_trace_gap`](Self::design_trace_gap) — satisfied by a
    ///   capability allocated to a part with a check planned.
    fn leg_traceability(&self, phase: ClosurePhase) -> Result<ClosureLeg, DynoError> {
        let mut reqs = self.scan_live_nodes(node::REQUIREMENT)?;
        reqs.sort_by(|a, b| a.node_id.cmp(&b.node_id));
        let mut swept = 0usize;
        let mut closed = 0usize;
        let mut parked = 0usize;
        let mut worst: Option<ClosureHole> = None;
        for r in &reqs {
            let status = r
                .properties
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("proposed");
            if status == "dropped" {
                continue;
            }
            if self.is_parked(&r.node_id)? {
                parked += 1;
                continue;
            }
            swept += 1;
            let inferred =
                r.properties.get("provenance").and_then(Value::as_str) == Some("inferred");
            let gap: Option<String> = if inferred {
                Some(format!(
                    "'{}' was inferred from what implements it, so its thread proves nothing",
                    r.node_id
                ))
            } else {
                match phase {
                    ClosurePhase::Build => {
                        if self.requirement_is_delivered(&r.node_id)? {
                            None
                        } else {
                            Some(format!(
                                "'{}' has no capability that is built and currently checked",
                                r.node_id
                            ))
                        }
                    }
                    ClosurePhase::Design => self
                        .design_trace_gap(&r.node_id)?
                        .map(|why| format!("'{}' is not traced for the design: {why}", r.node_id)),
                }
            };
            match gap {
                None => closed += 1,
                Some(why) => {
                    if worst.is_none() {
                        worst = Some(ClosureHole {
                            leg: "traceability".into(),
                            id: Some(r.node_id.clone()),
                            why,
                        });
                    }
                }
            }
        }
        let note = if parked > 0 {
            format!(
                "requirements: {swept} live, {parked} parked (a `parks` ruling says so; counted \
                 as parked, never as a hole)"
            )
        } else {
            format!("requirements: {swept} live")
        };
        let mut l = leg(phase, "traceability", swept, closed, note, worst);
        l.parked = parked;
        Ok(l)
    }

    /// Why a requirement is NOT traced for the design phase, or `None` when
    /// it is: some capability satisfies it that is not discontinued, is
    /// ALLOCATED to a part, and has a check planned — a Verification of any
    /// status on the capability or on the requirement itself. A decomposed
    /// parent is traced when every child is, rolled up the way the delivery
    /// line rolls up (and bounded the same way).
    fn design_trace_gap(&self, requirement_id: &str) -> Result<Option<String>, DynoError> {
        self.design_trace_gap_within(requirement_id, 64)
    }

    fn design_trace_gap_within(
        &self,
        requirement_id: &str,
        depth: usize,
    ) -> Result<Option<String>, DynoError> {
        if depth == 0 {
            return Ok(Some("its decomposition is too deep to follow".into()));
        }
        let requirement_checked = !self
            .incoming(requirement_id, Some(edge::VERIFIES))?
            .is_empty();
        let mut first_miss: Option<String> = None;
        let mut sats = self.incoming(requirement_id, Some(edge::SATISFIES))?;
        sats.sort_by(|a, b| a.from_id.cmp(&b.from_id));
        for e in &sats {
            let cap = &e.from_id;
            if self.is_discontinued(cap)? {
                continue;
            }
            let allocated = !self.outgoing(cap, Some(edge::ALLOCATED_TO))?.is_empty();
            let checked =
                requirement_checked || !self.incoming(cap, Some(edge::VERIFIES))?.is_empty();
            let miss = match (allocated, checked) {
                (true, true) => return Ok(None),
                (false, false) => format!(
                    "'{cap}' satisfies it but is not allocated to any part and has no check planned"
                ),
                (false, true) => format!(
                    "'{cap}' satisfies it and has a check planned, but is not allocated to any part"
                ),
                (true, false) => {
                    format!("'{cap}' satisfies it and is allocated, but has no check planned")
                }
            };
            if first_miss.is_none() {
                first_miss = Some(miss);
            }
        }
        let children = self.decomposed_children(requirement_id)?;
        if !children.is_empty() {
            for child in &children {
                if let Some(why) = self.design_trace_gap_within(child, depth - 1)? {
                    return Ok(Some(
                        first_miss.unwrap_or(format!("its part '{child}' is not traced: {why}")),
                    ));
                }
            }
            return Ok(None);
        }
        Ok(Some(first_miss.unwrap_or_else(|| {
            "no capability satisfies it, so nothing is allocated and no check is planned".into()
        })))
    }

    /// BUDGETS. Both phases: inside the limit, and inside the declared margin
    /// when one is declared. The BUILD phase also needs every numbered
    /// contribution to be `measured` — the owner's rule from the case that
    /// raised this ("design done may close a budget on an estimate marked as
    /// asserted; build done requires every budget to close on a
    /// measurement"). A contribution with no stated basis reads as
    /// `estimated`, the reading `budget_report` gives it.
    fn leg_budgets(&self, phase: ClosurePhase) -> Result<ClosureLeg, DynoError> {
        let mut cons = self.scan_live_nodes(node::CONSTRAINT)?;
        cons.sort_by(|a, b| a.node_id.cmp(&b.node_id));
        let mut swept = 0usize;
        let mut closed = 0usize;
        let mut worst: Option<ClosureHole> = None;
        let mut with_margin = 0usize;
        for c in &cons {
            if c.properties.get("limit").and_then(Value::as_f64).is_none() {
                continue;
            }
            swept += 1;
            let r = self.budget_report(&c.node_id)?;
            let margin = c.properties.get("margin").and_then(Value::as_f64);
            if margin.is_some() {
                with_margin += 1;
            }
            let (ok, why) = match r.verdict {
                BudgetVerdict::Within => {
                    // Within the limit — and within the declared margin, when
                    // one is declared. A margin is headroom the owner wants
                    // kept, in the limit's unit.
                    match (margin, r.limit) {
                        (Some(m), Some(limit)) => {
                            let inside = if r.direction == "minimum" {
                                r.total >= limit + m
                            } else {
                                r.total <= limit - m
                            };
                            if inside {
                                (true, String::new())
                            } else {
                                (
                                    false,
                                    format!(
                                        "'{}' is within its limit ({}) but not its declared margin ({}): total {}",
                                        c.node_id, limit, m, r.total
                                    ),
                                )
                            }
                        }
                        _ => (true, String::new()),
                    }
                }
                BudgetVerdict::Exceeded => (
                    false,
                    format!(
                        "'{}' is exceeded: total {} against limit {}",
                        c.node_id,
                        r.total,
                        r.limit.unwrap_or(f64::NAN)
                    ),
                ),
                BudgetVerdict::Incomplete => (
                    false,
                    format!(
                        "'{}' cannot be summed: {} contribution(s) unstated, {} in another unit",
                        c.node_id,
                        r.unstated.len(),
                        r.unit_mismatched.len()
                    ),
                ),
                BudgetVerdict::Ungated => (true, String::new()),
            };
            let (ok, why) = if ok && phase == ClosurePhase::Build {
                let numbered: Vec<&crate::budget::BudgetContributor> = r
                    .contributors
                    .iter()
                    .filter(|c| c.contribution.is_some())
                    .collect();
                let unmeasured: Vec<&str> = numbered
                    .iter()
                    .filter(|c| c.basis.as_deref() != Some("measured"))
                    .map(|c| c.node_id.as_str())
                    .collect();
                if unmeasured.is_empty() {
                    (true, why)
                } else {
                    (
                        false,
                        format!(
                            "'{}' closes only on estimates at build: {} of {} numbered \
                             contribution(s) are not measured ({}); the build phase needs \
                             measured numbers",
                            c.node_id,
                            unmeasured.len(),
                            numbered.len(),
                            unmeasured.join(", ")
                        ),
                    )
                }
            } else {
                (ok, why)
            };
            if ok {
                closed += 1;
            } else if worst.is_none() {
                worst = Some(ClosureHole {
                    leg: "budgets".into(),
                    id: Some(c.node_id.clone()),
                    why,
                });
            }
        }
        let note = format!("budgets: {swept} modelled, {with_margin} with a declared margin");
        Ok(leg(phase, "budgets", swept, closed, note, worst))
    }

    fn leg_seams(&self, phase: ClosurePhase) -> Result<ClosureLeg, DynoError> {
        let s = self.seam_coverage(None)?;
        let mut uncovered = s.uncovered.clone();
        uncovered.sort();
        let worst = uncovered.first().map(|(a, b)| ClosureHole {
            leg: "seams".into(),
            id: Some(format!("{a} <-> {b}")),
            why: format!("'{a}' and '{b}' are coupled and no contract between them is declared"),
        });
        let note = format!(
            "seams: {} coupling(s) between parts, {} contract pair(s) declared",
            s.couplings, s.declared
        );
        Ok(leg(phase, "seams", s.couplings, s.covered, note, worst))
    }

    fn leg_decisions(&self, phase: ClosurePhase) -> Result<ClosureLeg, DynoError> {
        // The scheduled increment: every node SCHEDULED_FOR an epoch still
        // `planned`. Each is closed when no decision governing it is still
        // open. Swept = the scheduled items, so "no increment scheduled"
        // reads as nothing to run on rather than as clean.
        let mut epochs = self.scan_live_nodes(node::DESIGN_EPOCH)?;
        epochs.sort_by(|a, b| a.node_id.cmp(&b.node_id));
        let mut items: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut planned = 0usize;
        for ep in &epochs {
            let status = ep
                .properties
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("arrived");
            if status != "planned" {
                continue;
            }
            planned += 1;
            for e in self.incoming(&ep.node_id, Some(edge::SCHEDULED_FOR))? {
                items.insert(e.from_id.clone());
            }
        }
        let mut closed = 0usize;
        let mut worst: Option<ClosureHole> = None;
        for item in &items {
            let mut open_governor: Option<String> = None;
            let mut govs: Vec<String> = self
                .outgoing(item, Some(edge::GOVERNED_BY))?
                .into_iter()
                .map(|e| e.to_id)
                .filter(|id| !is_review(id))
                .collect();
            govs.sort();
            for d in govs {
                if let Some(dec) = self.get_node(node::DECISION, &d)?
                    && dec.properties.get("status").and_then(Value::as_str) == Some("proposed")
                {
                    open_governor = Some(d);
                    break;
                }
            }
            match open_governor {
                None => closed += 1,
                Some(d) => {
                    if worst.is_none() {
                        worst = Some(ClosureHole {
                            leg: "decisions".into(),
                            id: Some(d.clone()),
                            why: format!(
                                "'{item}' is scheduled and governed by '{d}', which is still proposed"
                            ),
                        });
                    }
                }
            }
        }
        let note = format!(
            "decisions: {} item(s) scheduled into {} planned increment(s)",
            items.len(),
            planned
        );
        Ok(leg(phase, "decisions", items.len(), closed, note, worst))
    }

    fn leg_provenance(&self, phase: ClosurePhase) -> Result<ClosureLeg, DynoError> {
        let s = self.quantity_provenance_sweep()?;
        let worst = s.unsourced.first().map(|(id, what)| ClosureHole {
            leg: "provenance".into(),
            id: Some(id.clone()),
            why: format!("{what} has no source"),
        });
        let closed = s.quantities.saturating_sub(s.unsourced.len());
        let note = format!(
            "quantities: {} stated ({} limit(s) and their numbered contributions)",
            s.quantities, s.limits
        );
        Ok(leg(phase, "provenance", s.quantities, closed, note, worst))
    }
}

fn leg(
    phase: ClosurePhase,
    name: &str,
    swept: usize,
    closed: usize,
    swept_note: String,
    worst: Option<ClosureHole>,
) -> ClosureLeg {
    let share = if swept == 0 {
        None
    } else {
        Some(closed as f64 * 100.0 / swept as f64)
    };
    ClosureLeg {
        leg: name.to_string(),
        phase,
        counted: false,
        swept,
        parked: 0,
        closed,
        share,
        swept_note,
        worst,
        closes: None,
    }
}
