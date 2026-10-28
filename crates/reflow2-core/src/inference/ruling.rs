//! A RESULT REFUTES ONLY ON THE OWNER'S SIGNED RULING
//! (`req:the-owner-can-rule-a-prediction-refuted-by-a-named-result`, governed
//! by `dec:a-result-refutes-a-prediction-only-on-the-owners-ruling-2026-10-28`,
//! Anthony 2026-10-28), and what a ruling rests on.
//!
//! An experiment's result is evidence. On its own it CONTESTS what it goes
//! against and never refutes it (`dec:idea-an-objection-resting-on-evidence-
//! contests-or-refutes`, the owner's 10-20 rule, which this keeps): a failed
//! prediction may lie in the hypothesis, the model, an idealization or the
//! instrument, and judging which is the scientist's, never the tool's.
//!
//! THE OWNER SETTLES IT WITH A RULING, recorded with acts reflow2 already has:
//!
//! - a Decision (`add_decision`), ACCEPTED and SIGNED by the owner (an
//!   AUTHORED_BY with role `approver`: `set_decision_status` with `approver`,
//!   refused for anyone but the caller on an engine served for others);
//! - a concession from that Decision to the result: CONTRADICTS with
//!   `alignment: conceding`, the act by which a reply GRANTS an objection
//!   against what it attacks
//!   (`req:a-reply-can-concede-an-objection-against-one-reading-and-move-to-another`);
//! - optionally, the ruling's own step: `add_inference` concluding the
//!   Decision from the result and the prediction it missed, whose warrant
//!   says why the owner lays the miss on the hypothesis rather than the model.
//!
//! While the ruling is in force (accepted, signed, and nothing it holds under
//! taken away), each claim the result goes against reads REFUTED BY THE RULING,
//! naming the ruling, the result and the CONDITIONS: the model and principles
//! held on purpose (`held_as`) under what the result goes against and under
//! the ruling's step. Each HYPOTHESIS discharged into those predictions (a
//! statement of kind `hypothesis`) reads refuted by the same ruling. Withdraw
//! the ruling (`set_decision_status` to rejected or superseded, or delete the
//! concession) and the result only contests again.
//!
//! A Decision conceding a result is a ruling and nothing else: unsigned, or
//! not accepted, it is not in force and the result contests, as it does
//! alone. Every other concession reads as it did.

use std::collections::{BTreeSet, VecDeque};

use super::{Eval, INFERENCE_DEPTH, Truth, is_evidence, list_prop, statement_kind};
use crate::foundation::core::{DynoError, Value};
use crate::graph::DesignGraph;
use crate::nodes::{edge, node};

/// How many records the walks below read before they stop: what a ruling
/// rests on is read structurally, never through a truth, and a design far
/// larger than any argument still ends.
const WALK_CAP: usize = 4096;

/// The owner's ruling on a result, read: served on the objection it settles
/// (`ruling`), in reasoning_report's `refuted_by_ruling` and in loop_status's
/// `reasoning.rulings_in_force`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RulingReading {
    /// The Decision that rules.
    pub ruling: String,
    /// The result it rules on: a statement of kind `evidence`, or a
    /// statistical test.
    pub result: String,
    /// Who signed it: the approvers of the Decision.
    pub signed_by: Vec<String>,
    /// What the result goes against, which reads refuted by the ruling.
    pub against: Vec<String>,
    /// The hypotheses (statements of kind `hypothesis`) discharged into, or
    /// taken up under, those predictions and the ruling's own step: each reads
    /// refuted by the ruling too.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hypotheses: Vec<String>,
    /// THE CONDITIONS: the model and principles held on purpose (`held_as`)
    /// under what the result goes against and under the ruling's step. The
    /// refutation holds only as far as they do; a change that takes one away
    /// takes the ruling's force with it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub given: Vec<String>,
    /// The ruling in a sentence.
    pub why: String,
}

/// A ruling a change takes the force of: served by `propagate_from` and
/// `propagate_change` as `rulings_lost`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RulingLost {
    /// The Decision that ruled.
    pub ruling: String,
    /// The result it ruled on, which only contests again.
    pub result: String,
    /// What the result goes against: refuted by the ruling before the
    /// change, only contested after it.
    pub against: Vec<String>,
    /// The hypotheses that read refuted by the ruling before the change.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hypotheses: Vec<String>,
    /// What of it the change takes away: the ruling itself, or a condition
    /// it holds under.
    pub taken: Vec<String>,
    /// Why, in a sentence.
    pub why: String,
}

/// Whether a ruling is in force, and why not.
pub(super) enum RulingState {
    InForce(RulingReading),
    NotInForce { why: String },
}

impl DesignGraph {
    /// Whether `id` is a RESULT: a statement of kind `evidence` (a report, a
    /// measurement) or a statistical test — what on its own only contests.
    pub(crate) fn is_result(&self, id: &str) -> Result<bool, DynoError> {
        let Some(ty) = self.type_holding(id)? else {
            return Ok(false);
        };
        let Some(n) = self.get_node(&ty, id)? else {
            return Ok(false);
        };
        Ok(is_evidence(&n.properties)
            || (ty == node::VERIFICATION
                && n.properties
                    .contains_key(crate::hypothesis::HYPOTHESIS_TEST)))
    }

    /// HOW `id` IS HELD AS AN ASSUMPTION, design-wide, the way
    /// [`DesignGraph::assumptions`] says it: `None` when no step takes it up
    /// as `assumed` or none says how; `supposition` when any step holds it as
    /// one or says nothing; else `principle` when any says so; else `model`.
    pub(crate) fn held_on_purpose_as(&self, id: &str) -> Result<Option<&'static str>, DynoError> {
        let marks: Vec<Option<&'static str>> = self
            .outgoing(id, Some(edge::PREMISE_OF))?
            .into_iter()
            .filter(|e| e.properties.get("role").and_then(Value::as_str) == Some("assumed"))
            .map(|e| {
                e.properties
                    .get("held_as")
                    .and_then(Value::as_str)
                    .and_then(|m| super::HELD_AS.iter().copied().find(|h| *h == m))
            })
            .collect();
        Ok(if marks.iter().all(Option::is_none) {
            None
        } else if marks
            .iter()
            .any(|m| matches!(m, None | Some("supposition")))
        {
            Some("supposition")
        } else if marks.contains(&Some("principle")) {
            Some("principle")
        } else {
            Some("model")
        })
    }

    /// Premises held on purpose in words: "con:a (held as a model), con:b
    /// (held as a principle)".
    pub(crate) fn held_said(&self, ids: &[String]) -> Result<String, DynoError> {
        let mut out = Vec::new();
        for id in ids {
            match self.held_on_purpose_as(id)? {
                Some(h @ ("principle" | "model")) => out.push(format!("{id} (held as a {h})")),
                _ => out.push(id.clone()),
            }
        }
        Ok(out.join(", "))
    }

    /// Whether `id` is HELD ON PURPOSE: a principle or a model.
    pub(crate) fn is_held_on_purpose(&self, id: &str) -> Result<bool, DynoError> {
        Ok(matches!(
            self.held_on_purpose_as(id)?,
            Some("principle" | "model")
        ))
    }

    /// Who signed `decision`: each Contributor an AUTHORED_BY names as
    /// `approver`, sorted.
    fn approvers_of(&self, decision: &str) -> Result<Vec<String>, DynoError> {
        let mut out: Vec<String> = self
            .outgoing(decision, Some(edge::AUTHORED_BY))?
            .into_iter()
            .filter(|e| {
                crate::graph::list_of_strings(e.properties.get("roles"))
                    .iter()
                    .any(|r| r == "approver")
                    || e.properties.get("role").and_then(Value::as_str) == Some("approver")
            })
            .map(|e| e.to_id)
            .collect();
        out.sort();
        out.dedup();
        Ok(out)
    }

    /// The Decisions that concede the result `result` (CONTRADICTS marked
    /// `conceding` from a Decision), sorted: the rulings on it, in force or
    /// not.
    fn rulings_on(&self, result: &str) -> Result<Vec<String>, DynoError> {
        let mut out = Vec::new();
        for e in self.incoming(result, Some(edge::CONTRADICTS))? {
            if e.properties.get("alignment").and_then(Value::as_str) == Some("conceding")
                && self.get_node(node::DECISION, &e.from_id)?.is_some()
            {
                out.push(e.from_id);
            }
        }
        out.sort();
        out.dedup();
        Ok(out)
    }

    /// Whether `reply` conceding `objection` is a RULING: a Decision
    /// conceding a result. Such a concession counts only as a ruling, never
    /// as an ordinary one.
    pub(super) fn concession_is_a_ruling(
        &self,
        reply: &str,
        objection: &str,
    ) -> Result<bool, DynoError> {
        Ok(self.get_node(node::DECISION, reply)?.is_some() && self.is_result(objection)?)
    }

    /// EVERY RULING THE DESIGN RECORDS, in force or not: (the Decision, the
    /// result it concedes), sorted. Read structurally.
    pub fn rulings(&self) -> Result<Vec<(String, String)>, DynoError> {
        let mut out = Vec::new();
        for d in self.scan_nodes(node::DECISION)? {
            for e in self.outgoing(&d.node_id, Some(edge::CONTRADICTS))? {
                if e.properties.get("alignment").and_then(Value::as_str) == Some("conceding")
                    && self.is_result(&e.to_id)?
                {
                    out.push((d.node_id.clone(), e.to_id));
                }
            }
        }
        out.sort();
        out.dedup();
        Ok(out)
    }

    /// What `decision` ruling on `result` rests on, read structurally: what
    /// the result goes against (`against`) and the premises of the ruling's
    /// own step other than the result (its grounds).
    fn ruling_roots(
        &self,
        decision: &str,
        result: &str,
    ) -> Result<(Vec<String>, Vec<String>), DynoError> {
        let against = self.attack_targets_of_objection(result)?;
        let mut grounds: BTreeSet<String> = BTreeSet::new();
        for s in self.inferences_concluding(decision)? {
            for p in self.premises_of(&s)? {
                if p != result {
                    grounds.insert(p);
                }
            }
        }
        Ok((against, grounds.into_iter().collect()))
    }

    /// The support beneath `roots`, read structurally: every step concluding
    /// one of them or a premise beneath, with each premise link (and whether
    /// it is assumed). Iterative, bounded by [`WALK_CAP`].
    fn support_links(&self, roots: &[String]) -> Result<Vec<(String, String, bool)>, DynoError> {
        let mut out = Vec::new();
        let mut seen: BTreeSet<String> = roots.iter().cloned().collect();
        let mut queue: VecDeque<String> = roots.iter().cloned().collect();
        let mut read = 0usize;
        while let Some(x) = queue.pop_front() {
            read += 1;
            if read > WALK_CAP {
                break;
            }
            for s in self.inferences_concluding(&x)? {
                for (p, assumed) in self.premise_links(&s)? {
                    out.push((s.clone(), p.clone(), assumed));
                    if seen.insert(p.clone()) {
                        queue.push_back(p);
                    }
                }
            }
        }
        Ok(out)
    }

    /// THE CONDITIONS beneath `roots`: every premise taken up as `assumed`
    /// and held on purpose (a model or a principle), sorted.
    fn held_beneath(&self, roots: &[String]) -> Result<BTreeSet<String>, DynoError> {
        let mut out = BTreeSet::new();
        for (_, p, assumed) in self.support_links(roots)? {
            if assumed && !out.contains(&p) && self.is_held_on_purpose(&p)? {
                out.insert(p);
            }
        }
        Ok(out)
    }

    /// THE HYPOTHESES of `x`: statements of kind `hypothesis` that a step
    /// concluding `x` discharges (the hypothesis discharged into it), and
    /// those taken up as `assumed` beneath it that no step on the way
    /// discharges (it was derived under them).
    fn hypotheses_of(&self, x: &str) -> Result<BTreeSet<String>, DynoError> {
        let is_hypothesis = |id: &str| -> Result<bool, DynoError> {
            Ok(self
                .node_props(id)?
                .is_some_and(|p| statement_kind(&p) == Some("hypothesis")))
        };
        let mut out = BTreeSet::new();
        for s in self.inferences_concluding(x)? {
            if let Some(n) = self.get_node(node::INFERENCE, &s)? {
                for h in list_prop(&n.properties, "discharges") {
                    if is_hypothesis(&h)? {
                        out.insert(h);
                    }
                }
            }
        }
        let links = self.support_links(&[x.to_string()])?;
        let mut discharged: BTreeSet<String> = BTreeSet::new();
        let mut steps: BTreeSet<&str> = BTreeSet::new();
        for (s, _, _) in &links {
            steps.insert(s.as_str());
        }
        for s in steps {
            if let Some(n) = self.get_node(node::INFERENCE, s)? {
                discharged.extend(list_prop(&n.properties, "discharges"));
            }
        }
        for (_, p, assumed) in links {
            if assumed && !discharged.contains(&p) && is_hypothesis(&p)? {
                out.insert(p);
            }
        }
        Ok(out)
    }

    /// Whether `decision`'s ruling on `result` is IN FORCE in `ev`: the
    /// Decision holds (accepted, and nothing that holds stands against it),
    /// an approver signed it, and nothing it holds under is taken away.
    pub(super) fn ruling_state(
        &self,
        decision: &str,
        result: &str,
        ev: &mut Eval<'_>,
        depth: usize,
    ) -> Result<RulingState, DynoError> {
        // ACCEPTED, by its owner: a Decision a holding chain concludes reads
        // as its chain whatever its status, and a ruling is the owner's word,
        // never its argument's.
        let status = self.get_node(node::DECISION, decision)?.and_then(|n| {
            n.properties
                .get("status")
                .and_then(Value::as_str)
                .map(str::to_string)
        });
        if status.as_deref() != Some("accepted") {
            return Ok(RulingState::NotInForce {
                why: format!(
                    "{decision} would rule on it and is {}, not accepted, so the result only \
                     contests",
                    status.as_deref().unwrap_or("proposed")
                ),
            });
        }
        let r = self.truth_full(decision, ev, depth.saturating_sub(1))?;
        if r.truth != Truth::Holds {
            return Ok(RulingState::NotInForce {
                why: format!(
                    "{decision} would rule on it and is not in force ({}), so the result only \
                     contests",
                    r.why
                ),
            });
        }
        let signed_by = self.approvers_of(decision)?;
        if signed_by.is_empty() {
            return Ok(RulingState::NotInForce {
                why: format!(
                    "{decision} would rule on it, and no approver signed its acceptance: a ruling \
                     is the owner's signed word (set_decision_status with `approver`), so the \
                     result only contests"
                ),
            });
        }
        let (against, grounds) = self.ruling_roots(decision, result)?;
        let roots: Vec<String> = against.iter().chain(grounds.iter()).cloned().collect();
        let given: Vec<String> = self.held_beneath(&roots)?.into_iter().collect();
        // A change that takes away the result, or a premise held on purpose
        // beneath the ruling, takes its force with it.
        let taken: Vec<String> = std::iter::once(result.to_string())
            .chain(given.iter().cloned())
            .filter(|g| ev.lost.contains(g))
            .collect();
        if !taken.is_empty() {
            return Ok(RulingState::NotInForce {
                why: format!(
                    "{decision} ruled on it{}, and the change takes away {}: the ruling is no \
                     longer in force",
                    if given.is_empty() {
                        String::new()
                    } else {
                        format!(" given {}", given.join(", "))
                    },
                    taken.join(", ")
                ),
            });
        }
        let mut hypotheses: BTreeSet<String> = BTreeSet::new();
        for x in &roots {
            hypotheses.extend(self.hypotheses_of(x)?);
        }
        let given_said = if given.is_empty() {
            "on the ruling alone: nothing beneath it is held on purpose as a model or a principle"
                .to_string()
        } else {
            format!(
                "conditional on what is held on purpose beneath it, {}: if one of them fails, so \
                 does the refutation",
                given.join(", ")
            )
        };
        let why = format!(
            "refuted by the owner's ruling {decision} (signed by {}) on the result {result}, {given_said}. \
             A result alone only contests; this is the owner's judgement that the miss lies here",
            signed_by.join(", ")
        );
        Ok(RulingState::InForce(RulingReading {
            ruling: decision.to_string(),
            result: result.to_string(),
            signed_by,
            against,
            hypotheses: hypotheses.into_iter().collect(),
            given,
            why,
        }))
    }

    /// The ruling on `result` in force in `ev`, the first in id order; else
    /// why the first one is not, when a Decision concedes it at all.
    pub(super) fn ruling_of_result(
        &self,
        result: &str,
        ev: &mut Eval<'_>,
        depth: usize,
    ) -> Result<Option<RulingState>, DynoError> {
        let mut first_not: Option<RulingState> = None;
        for d in self.rulings_on(result)? {
            match self.ruling_state(&d, result, ev, depth)? {
                s @ RulingState::InForce(_) => return Ok(Some(s)),
                s => {
                    if first_not.is_none() {
                        first_not = Some(s);
                    }
                }
            }
        }
        Ok(first_not)
    }

    /// THE RULING THAT REFUTES THE HYPOTHESIS `id`, when one in force does:
    /// a ruling whose result goes against a claim `id` was discharged into
    /// or derived under, or whose own step rests on one. Walked forward from
    /// `id` through the steps that take it up, to the step that discharges
    /// it.
    pub(super) fn ruling_on_hypothesis(
        &self,
        id: &str,
        ev: &mut Eval<'_>,
        depth: usize,
    ) -> Result<Option<RulingReading>, DynoError> {
        // The claims `id` was derived under or discharged into.
        let mut candidates: BTreeSet<String> = BTreeSet::new();
        let mut queue: VecDeque<String> = self.steps_assuming(id)?.into_iter().collect();
        let mut seen: BTreeSet<String> = queue.iter().cloned().collect();
        let mut read = 0usize;
        while let Some(s) = queue.pop_front() {
            read += 1;
            if read > WALK_CAP {
                break;
            }
            let Some(c) = self.conclusion_of(&s)? else {
                continue;
            };
            if !candidates.insert(c.clone()) {
                continue;
            }
            let discharges = self.get_node(node::INFERENCE, &s)?.is_some_and(|n| {
                list_prop(&n.properties, "discharges")
                    .iter()
                    .any(|h| h == id)
            });
            if discharges {
                continue;
            }
            for next in self.steps_resting_on(&c)? {
                if seen.insert(next.clone()) {
                    queue.push_back(next);
                }
            }
        }
        if candidates.is_empty() {
            return Ok(None);
        }
        // Each ruling one of them bears on: as what its result goes against,
        // or as a ground of its own step.
        let mut rulings: BTreeSet<(String, String)> = BTreeSet::new();
        for c in &candidates {
            for e in self.incoming(c, Some(edge::CONTRADICTS))? {
                if matches!(
                    e.properties.get("alignment").and_then(Value::as_str),
                    Some("supporting" | "conceding")
                ) || !self.is_result(&e.from_id)?
                {
                    continue;
                }
                for d in self.rulings_on(&e.from_id)? {
                    rulings.insert((d, e.from_id.clone()));
                }
            }
            for s in self.steps_resting_on(c)? {
                let Some(d) = self.conclusion_of(&s)? else {
                    continue;
                };
                if self.get_node(node::DECISION, &d)?.is_none() {
                    continue;
                }
                for e in self.outgoing(&d, Some(edge::CONTRADICTS))? {
                    if e.properties.get("alignment").and_then(Value::as_str) == Some("conceding")
                        && self.is_result(&e.to_id)?
                    {
                        rulings.insert((d.clone(), e.to_id));
                    }
                }
            }
        }
        for (d, result) in rulings {
            if let RulingState::InForce(r) = self.ruling_state(&d, &result, ev, depth)?
                && r.hypotheses.iter().any(|h| h == id)
            {
                return Ok(Some(r));
            }
        }
        Ok(None)
    }

    /// EVERY RULING IN FORCE, as the design stands, sorted by ruling and
    /// result.
    pub fn rulings_in_force(&self) -> Result<Vec<RulingReading>, DynoError> {
        let lost = BTreeSet::new();
        let mut ev = Eval::new(self, &lost);
        let mut out = Vec::new();
        for (d, result) in self.rulings()? {
            if let RulingState::InForce(r) =
                self.ruling_state(&d, &result, &mut ev, INFERENCE_DEPTH)?
            {
                out.push(r);
            }
        }
        Ok(out)
    }

    /// THE RULINGS A CHANGE TAKES THE FORCE OF: each in force before and not
    /// with `lost` counted, with what of it the change takes away.
    pub(super) fn rulings_lost(
        &self,
        lost: &BTreeSet<String>,
        before: &mut Eval<'_>,
        after: &mut Eval<'_>,
    ) -> Result<Vec<RulingLost>, DynoError> {
        let mut out = Vec::new();
        for (d, result) in self.rulings()? {
            let RulingState::InForce(was) =
                self.ruling_state(&d, &result, before, INFERENCE_DEPTH)?
            else {
                continue;
            };
            if let RulingState::NotInForce { why } =
                self.ruling_state(&d, &result, after, INFERENCE_DEPTH)?
            {
                let mut taken: Vec<String> = std::iter::once(result.clone())
                    .chain(was.given.iter().cloned())
                    .filter(|g| lost.contains(g))
                    .collect();
                if lost.contains(&d) {
                    taken.insert(0, d.clone());
                }
                out.push(RulingLost {
                    ruling: d,
                    result,
                    against: was.against,
                    hypotheses: was.hypotheses,
                    taken,
                    why,
                });
            }
        }
        Ok(out)
    }
}
