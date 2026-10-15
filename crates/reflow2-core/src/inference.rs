//! INFERENCE — a stated step from premises to one conclusion, and the check
//! that the conclusion follows.
//!
//! `req:a-design-states-how-premises-support-a-conclusion-and-reflow2-checks-it-follows`
//! (Anthony, 2026-10-14), shaped by
//! `dec:an-inference-is-a-record-between-premises-and-conclusion`: an
//! `Inference` record sits between its premises (`PREMISE_OF`, premise → step)
//! and its one conclusion (`CONCLUDES`, step → conclusion) and holds the RULE by
//! which they combine — `all`, `any`, or `k_of_n` with its `k`. A named rule
//! (`rule_name`, "modus ponens") is a label; the step is checked by its
//! combination, never by its name.
//!
//! # What "holds" means for a premise — computed, never guessed
//!
//! Three values, from what reflow2 ALREADY computes for each type
//! ([`DesignGraph::premise_truth`]):
//!
//! - a **Requirement** holds when the existing roll-up delivers it or its status
//!   is `met`, and fails when it is `dropped`;
//! - a **Verification** holds when `passing` and fails when `failing`;
//! - a **Decision** holds when `accepted`, and fails when `rejected` or
//!   `superseded`;
//! - a **Constraint** fails when something VIOLATES it or its budget is
//!   exceeded, is unknown while its budget cannot be judged, and holds
//!   otherwise;
//! - a **Capability** holds when it is realized and checked (the leaf of the
//!   delivery roll-up);
//! - anything discontinued by an accepted decision fails;
//! - a node concluded by an Inference is whatever its steps yield (ANY of them:
//!   two steps concluding one node are alternative derivations);
//! - an Inference used as a premise is its own verdict;
//! - every other type is UNKNOWN, and says so.
//!
//! The rule is evaluated over those values the Kleene way: `all` fails when any
//! premise fails and is unknown when none fails and one is unknown; `any` holds
//! when one holds and fails only when all fail; `k_of_n` holds at k holding and
//! fails once even the unknown ones could not reach k.
//!
//! # Where the check is read
//!
//! Nowhere new. The roll-up (`report.rs`, `requirement_delivered_by_within`)
//! delivers a requirement an Inference concludes when one of its steps holds;
//! the change walk ([`DesignGraph::inference_walk`], carried on every blast
//! radius) reports each step downstream of a lost premise as BROKEN or
//! WEAKENED; and `detect_gaps` reports a conclusion whose claimed status is past
//! what its steps support (`conclusion_does_not_follow`). A design that records
//! no Inference reads exactly as it did: every one of those paths asks one
//! adjacency question first and finds nothing.
//!
//! # What it does not check
//!
//! The CONTENT of the statements. Whether Q really is what P → Q gives from P
//! is a person's (or a later checker's) question; a false premise passes. What
//! the step gains is that its warrant becomes something a person can challenge.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::foundation::core::{DynoError, Value};
use crate::graph::DesignGraph;
use crate::nodes::{Props, edge, node};

/// How far a chain of steps is followed before the answer is `unknown`. A
/// chain this deep is far past any design written by hand; the bound exists
/// for a graph imported from somewhere that never refused a cycle.
pub const INFERENCE_DEPTH: usize = 64;

/// What every refusal of a circle adds, so it is never read as "a closed chain
/// can't be proved". A chain of equivalences — A ⇒ B ⇒ C ⇒ A, "the following
/// are equivalent" — is a valid proof; what has no ground is a conclusion
/// supporting its own premise. The chain is recorded by making EACH
/// IMPLICATION A STATEMENT OF ITS OWN and a premise beside its antecedent, so
/// every step is modus ponens and the support graph stays acyclic.
pub const EQUIVALENCES: &str = "A closed chain of equivalences (A ⇒ B ⇒ C ⇒ A) is still a valid \
     proof: record each implication as a statement of its own (A ⇒ B, B ⇒ C, C ⇒ A) and make it a \
     premise beside its antecedent, so each step is modus ponens and the support stays acyclic.";

/// The rules a step may state, as `Inference.rule` spells them.
pub const INFERENCE_RULES: [&str; 3] = ["all", "any", "k_of_n"];

/// A premise's truth, or a step's verdict: three values, never two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Truth {
    /// Computed to hold.
    Holds,
    /// Computed to fail.
    Fails,
    /// Nothing reflow2 computes says either way.
    Unknown,
}

impl Truth {
    /// The value as the replies spell it.
    pub fn as_str(self) -> &'static str {
        match self {
            Truth::Holds => "holds",
            Truth::Fails => "fails",
            Truth::Unknown => "unknown",
        }
    }
}

/// A step's rule, parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceRule {
    /// Every premise must hold (AND).
    All,
    /// One holding premise is enough (OR).
    Any,
    /// At least `k` premises must hold.
    KOfN(i64),
}

/// The rule over premise values, three-valued (Kleene). `Err` names why the
/// rule cannot be applied (a `k` outside `1..=n`).
pub fn combine(rule: InferenceRule, values: &[Truth]) -> Result<Truth, String> {
    let n = values.len() as i64;
    let holds = values.iter().filter(|v| **v == Truth::Holds).count() as i64;
    let fails = values.iter().filter(|v| **v == Truth::Fails).count() as i64;
    let unknown = n - holds - fails;
    if n == 0 {
        return Err("it has no premises, so nothing supports its conclusion".to_string());
    }
    Ok(match rule {
        InferenceRule::All => {
            if fails > 0 {
                Truth::Fails
            } else if unknown > 0 {
                Truth::Unknown
            } else {
                Truth::Holds
            }
        }
        InferenceRule::Any => {
            if holds > 0 {
                Truth::Holds
            } else if unknown > 0 {
                Truth::Unknown
            } else {
                Truth::Fails
            }
        }
        InferenceRule::KOfN(k) => {
            if k < 1 || k > n {
                return Err(format!(
                    "its k is {k} and it has {n} premise(s): k_of_n needs 1 ≤ k ≤ {n}"
                ));
            }
            if holds >= k {
                Truth::Holds
            } else if holds + unknown < k {
                Truth::Fails
            } else {
                Truth::Unknown
            }
        }
    })
}

/// One premise as a step read it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PremiseReading {
    /// The premise's id.
    pub id: String,
    /// Its type, or `None` when no node holds the id.
    pub node_type: Option<String>,
    /// Its computed truth.
    pub truth: Truth,
    /// Why, in a sentence: the predicate that decided it.
    pub why: String,
}

/// What a step's rule gives over its premises, read now. Served on
/// `get_node` for an Inference as `inference_reading`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InferenceReading {
    /// The step's id.
    pub inference: String,
    /// Its rule, as stored (`all` when it never said).
    pub rule: String,
    /// Its `k`, for `k_of_n`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub k: Option<i64>,
    /// Its rule's name, when it has one — a label, never what is checked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_name: Option<String>,
    /// The one node it concludes, when it concludes one.
    pub conclusion: Option<String>,
    /// The rule's verdict over the premises.
    pub verdict: Truth,
    /// How many premises hold, fail, and are unknown.
    pub holding: usize,
    pub failing: usize,
    pub unknown: usize,
    /// Every premise, with its truth and why.
    pub premises: Vec<PremiseReading>,
    /// Why the verdict is `unknown` when the rule itself could not be applied
    /// (no premises, a `k` outside its range, an unknown rule, a circle).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// What losing a premise does to one step: the change walk's row.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InferenceImpact {
    /// The step reached.
    pub inference: String,
    /// `broken` (the rule no longer holds), `weakened` (it still holds on its
    /// other premises), or `unknown` (what is left cannot say either way).
    pub impact: &'static str,
    /// The step's rule (`all` when it never said).
    pub rule: String,
    /// Its `k`, for `k_of_n`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub k: Option<i64>,
    /// Its rule's name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_name: Option<String>,
    /// The node it concludes.
    pub conclusion: Option<String>,
    /// How many steps from a seed: 1 when a seed is its premise, one more per
    /// broken conclusion the walk carried on through.
    pub distance: usize,
    /// The verdict before the loss, as the design stands.
    pub before: Truth,
    /// The verdict with every lost premise counted as failing.
    pub after: Truth,
    /// Its premises the walk counts as lost: the seeds, and conclusions it
    /// found broken on the way.
    pub lost_premises: Vec<String>,
    /// Whether the CONCLUSION still stands with the losses counted, across
    /// every step that concludes it (alternative derivations are ANY).
    pub conclusion_after: Truth,
    /// Every premise with its truth after the loss.
    pub premises: Vec<PremiseReading>,
}

/// A conclusion whose claimed status is past what its steps support.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UnfollowedConclusion {
    /// The conclusion.
    pub node_id: String,
    pub node_type: String,
    /// Its name, for the finding's words.
    pub name: String,
    /// The status it claims (`met`, `passing`, `accepted`, `verified`).
    pub claimed: String,
    /// What its steps support, ANY across them: `fails` or `unknown`.
    pub supported: Truth,
    /// Each step that concludes it, as it reads now.
    pub steps: Vec<InferenceReading>,
}

/// One evaluation: the premises counted as lost, what has been computed, and
/// the steps on the current path (a step met again on its own path is a
/// circle, read as unknown).
struct Eval<'l> {
    lost: &'l BTreeSet<String>,
    memo: HashMap<String, (Truth, String)>,
    path: BTreeSet<String>,
}

impl<'l> Eval<'l> {
    fn new(lost: &'l BTreeSet<String>) -> Self {
        Eval {
            lost,
            memo: HashMap::new(),
            path: BTreeSet::new(),
        }
    }
}

/// The parts of a step a reading needs, read once.
struct StepRecord {
    rule: String,
    parsed: Result<InferenceRule, String>,
    k: Option<i64>,
    rule_name: Option<String>,
}

fn prop<'a>(props: &'a HashMap<String, Value>, key: &str) -> Option<&'a str> {
    props.get(key).and_then(Value::as_str)
}

/// The status values that CLAIM a node holds, per type — what the finding
/// compares with what its steps support.
fn claimed_holds(node_type: &str, props: &HashMap<String, Value>) -> Option<&'static str> {
    let status = prop(props, "status")?;
    match (node_type, status) {
        (node::REQUIREMENT, "met") => Some("met"),
        (node::VERIFICATION, "passing") => Some("passing"),
        (node::DECISION, "accepted") => Some("accepted"),
        (node::CAPABILITY, "verified") => Some("verified"),
        _ => None,
    }
}

impl DesignGraph {
    // ─── Reading the steps ─────────────────────────────────────────────────

    /// The Inferences that conclude `id`, sorted. Empty for every node of a
    /// design that records none.
    pub fn inferences_concluding(&self, id: &str) -> Result<Vec<String>, DynoError> {
        let mut steps: Vec<String> = self
            .incoming(id, Some(edge::CONCLUDES))?
            .into_iter()
            .map(|e| e.from_id)
            .collect();
        steps.sort();
        steps.dedup();
        Ok(steps)
    }

    /// The premises of one step, sorted.
    pub fn premises_of(&self, inference_id: &str) -> Result<Vec<String>, DynoError> {
        let mut premises: Vec<String> = self
            .incoming(inference_id, Some(edge::PREMISE_OF))?
            .into_iter()
            .map(|e| e.from_id)
            .collect();
        premises.sort();
        premises.dedup();
        Ok(premises)
    }

    /// The one node a step concludes, when it concludes one.
    pub fn conclusion_of(&self, inference_id: &str) -> Result<Option<String>, DynoError> {
        Ok(self
            .outgoing(inference_id, Some(edge::CONCLUDES))?
            .into_iter()
            .map(|e| e.to_id)
            .min())
    }

    /// The steps `id` is a premise of, sorted.
    fn steps_resting_on(&self, id: &str) -> Result<Vec<String>, DynoError> {
        let mut steps: Vec<String> = self
            .outgoing(id, Some(edge::PREMISE_OF))?
            .into_iter()
            .map(|e| e.to_id)
            .collect();
        steps.sort();
        steps.dedup();
        Ok(steps)
    }

    fn step_record(&self, inference_id: &str) -> Result<Option<StepRecord>, DynoError> {
        let Some(n) = self.get_node(node::INFERENCE, inference_id)? else {
            return Ok(None);
        };
        let rule = prop(&n.properties, "rule").unwrap_or("all").to_string();
        let k = n.properties.get("k").and_then(Value::as_i64);
        let parsed = match rule.as_str() {
            "all" => Ok(InferenceRule::All),
            "any" => Ok(InferenceRule::Any),
            "k_of_n" => match k {
                Some(k) => Ok(InferenceRule::KOfN(k)),
                None => Err("its rule is k_of_n and it states no k".to_string()),
            },
            other => Err(format!(
                "its rule `{other}` is not one of {}",
                INFERENCE_RULES.join(", ")
            )),
        };
        Ok(Some(StepRecord {
            rule,
            parsed,
            k,
            rule_name: prop(&n.properties, "rule_name").map(str::to_string),
        }))
    }

    /// The one type holding `id`, when exactly one does.
    fn type_holding(&self, id: &str) -> Result<Option<String>, DynoError> {
        let held = self.node_types_holding(id)?;
        Ok(if held.len() == 1 {
            held.into_iter().next()
        } else {
            None
        })
    }

    // ─── The truth of a premise ────────────────────────────────────────────

    /// What reflow2 computes about whether `id` holds, and why. See the module
    /// documentation for the predicate each type gets.
    pub fn premise_truth(&self, id: &str) -> Result<(Truth, String), DynoError> {
        let lost = BTreeSet::new();
        let mut ev = Eval::new(&lost);
        self.truth_of(id, &mut ev, INFERENCE_DEPTH)
    }

    fn truth_of(
        &self,
        id: &str,
        ev: &mut Eval<'_>,
        depth: usize,
    ) -> Result<(Truth, String), DynoError> {
        if ev.lost.contains(id) {
            return Ok((Truth::Fails, "counted as lost by this change".to_string()));
        }
        if let Some(hit) = ev.memo.get(id) {
            return Ok(hit.clone());
        }
        let answer = self.truth_uncached(id, ev, depth)?;
        ev.memo.insert(id.to_string(), answer.clone());
        Ok(answer)
    }

    fn truth_uncached(
        &self,
        id: &str,
        ev: &mut Eval<'_>,
        depth: usize,
    ) -> Result<(Truth, String), DynoError> {
        if depth == 0 {
            return Ok((
                Truth::Unknown,
                format!("the chain of steps is deeper than {INFERENCE_DEPTH}"),
            ));
        }
        let Some(node_type) = self.type_holding(id)? else {
            return Ok((
                Truth::Unknown,
                "no single node of the design holds this id".to_string(),
            ));
        };
        if node_type == node::INFERENCE {
            let r = self.step_reading(id, ev, depth)?;
            let why = match &r.note {
                Some(note) => format!("a step whose verdict is unknown: {note}"),
                None => format!("a step whose rule `{}` {}", r.rule, r.verdict.as_str()),
            };
            return Ok((r.verdict, why));
        }
        if self.is_discontinued(id)? {
            return Ok((
                Truth::Fails,
                "discontinued by an accepted decision".to_string(),
            ));
        }
        let Some(n) = self.get_node(&node_type, id)? else {
            return Ok((Truth::Unknown, "the node is not there".to_string()));
        };
        let status = prop(&n.properties, "status").map(str::to_string);
        if node_type == node::REQUIREMENT && status.as_deref() == Some("dropped") {
            return Ok((Truth::Fails, "the requirement is dropped".to_string()));
        }

        // A CONCLUSION IS WHATEVER ITS STEPS YIELD. For a requirement a
        // realized, checked satisfier still delivers it directly, as it always
        // has; its own `met` is a claim the steps are checked against, never a
        // reason to believe them.
        let steps = self.inferences_concluding(id)?;
        if !steps.is_empty() {
            if node_type == node::REQUIREMENT {
                let by = self.requirement_directly_delivered_by(id)?;
                if !by.is_empty() {
                    return Ok((
                        Truth::Holds,
                        format!("delivered directly by {}", by.join(", ")),
                    ));
                }
            }
            let mut verdicts = Vec::new();
            for s in &steps {
                verdicts.push((s.clone(), self.step_reading(s, ev, depth)?.verdict));
            }
            let truth = combine(
                InferenceRule::Any,
                &verdicts.iter().map(|(_, v)| *v).collect::<Vec<_>>(),
            )
            .unwrap_or(Truth::Unknown);
            let said: Vec<String> = verdicts
                .iter()
                .map(|(s, v)| format!("{s} {}", v.as_str()))
                .collect();
            return Ok((truth, format!("concluded by {}", said.join("; "))));
        }

        Ok(match node_type.as_str() {
            node::REQUIREMENT => {
                if self.requirement_delivered_by_within(id, depth)?.is_some() {
                    (Truth::Holds, "delivered by the roll-up".to_string())
                } else if status.as_deref() == Some("met") {
                    (Truth::Holds, "its status is met".to_string())
                } else {
                    (
                        Truth::Unknown,
                        format!(
                            "not delivered, and its status ({}) says neither met nor dropped",
                            status.as_deref().unwrap_or("proposed")
                        ),
                    )
                }
            }
            node::VERIFICATION => match status.as_deref() {
                Some("passing") => (Truth::Holds, "the check is passing".to_string()),
                Some("failing") => (Truth::Fails, "the check is failing".to_string()),
                other => (
                    Truth::Unknown,
                    format!(
                        "the check is {}, so it says nothing either way",
                        other.unwrap_or("planned")
                    ),
                ),
            },
            node::DECISION => match status.as_deref() {
                Some("accepted") => (Truth::Holds, "the decision is accepted".to_string()),
                Some(s @ ("rejected" | "superseded")) => {
                    (Truth::Fails, format!("the decision is {s}"))
                }
                other => (
                    Truth::Unknown,
                    format!(
                        "the decision is {}, not settled",
                        other.unwrap_or("proposed")
                    ),
                ),
            },
            node::CONSTRAINT => self.constraint_truth(id, &n.properties)?,
            node::CAPABILITY => {
                let checked = !matches!(
                    self.capability_verification(id)?,
                    crate::verify::CapabilityVerification::Unchecked
                );
                if checked && self.capability_is_realized(id)? {
                    (
                        Truth::Holds,
                        "the capability is realized and checked".to_string(),
                    )
                } else {
                    (
                        Truth::Unknown,
                        "the capability is not both realized and checked".to_string(),
                    )
                }
            }
            other => (
                Truth::Unknown,
                format!("reflow2 computes no truth for a {other}, so it is unknown, never guessed"),
            ),
        })
    }

    /// A Constraint holds while nothing records it violated: it FAILS when a
    /// VIOLATES edge reaches it or its budget is exceeded, and is UNKNOWN while
    /// its budget cannot be judged (a contribution unstated).
    fn constraint_truth(
        &self,
        id: &str,
        props: &HashMap<String, Value>,
    ) -> Result<(Truth, String), DynoError> {
        let breakers: Vec<String> = self
            .incoming(id, Some(edge::VIOLATES))?
            .into_iter()
            .map(|e| e.from_id)
            .collect();
        if !breakers.is_empty() {
            return Ok((Truth::Fails, format!("violated by {}", breakers.join(", "))));
        }
        if props.get("limit").is_some_and(|v| !v.is_null()) {
            let report = self.budget_report(id)?;
            return Ok(match report.verdict {
                crate::budget::BudgetVerdict::Exceeded => {
                    (Truth::Fails, "its budget is exceeded".to_string())
                }
                crate::budget::BudgetVerdict::Incomplete => (
                    Truth::Unknown,
                    "its budget cannot be judged: a contribution is unstated".to_string(),
                ),
                crate::budget::BudgetVerdict::Within | crate::budget::BudgetVerdict::Ungated => {
                    (Truth::Holds, "its budget is within the limit".to_string())
                }
            });
        }
        Ok((Truth::Holds, "nothing records it violated".to_string()))
    }

    // ─── The verdict of a step ─────────────────────────────────────────────

    /// One step's verdict over its premises, as the design stands — served on
    /// `get_node` as `inference_reading`. `None` when no Inference has the id.
    pub fn inference_reading(
        &self,
        inference_id: &str,
    ) -> Result<Option<InferenceReading>, DynoError> {
        if self.get_node(node::INFERENCE, inference_id)?.is_none() {
            return Ok(None);
        }
        let lost = BTreeSet::new();
        let mut ev = Eval::new(&lost);
        Ok(Some(self.step_reading(
            inference_id,
            &mut ev,
            INFERENCE_DEPTH,
        )?))
    }

    fn step_reading(
        &self,
        inference_id: &str,
        ev: &mut Eval<'_>,
        depth: usize,
    ) -> Result<InferenceReading, DynoError> {
        let conclusion = self.conclusion_of(inference_id)?;
        let Some(rec) = self.step_record(inference_id)? else {
            return Ok(InferenceReading {
                inference: inference_id.to_string(),
                rule: "all".to_string(),
                k: None,
                rule_name: None,
                conclusion,
                verdict: Truth::Unknown,
                holding: 0,
                failing: 0,
                unknown: 0,
                premises: Vec::new(),
                note: Some("no Inference holds this id".to_string()),
            });
        };
        let mut reading = InferenceReading {
            inference: inference_id.to_string(),
            rule: rec.rule.clone(),
            k: rec.k,
            rule_name: rec.rule_name.clone(),
            conclusion,
            verdict: Truth::Unknown,
            holding: 0,
            failing: 0,
            unknown: 0,
            premises: Vec::new(),
            note: None,
        };
        if ev.lost.contains(inference_id) {
            reading.verdict = Truth::Fails;
            reading.note = Some("the step itself is counted as lost by this change".to_string());
            return Ok(reading);
        }
        if ev.path.contains(inference_id) {
            reading.note = Some(
                "it rests, through its premises, on its own conclusion: circular reasoning has \
                 no verdict"
                    .to_string(),
            );
            return Ok(reading);
        }
        if depth == 0 {
            reading.note = Some(format!(
                "the chain of steps under it is deeper than {INFERENCE_DEPTH}"
            ));
            return Ok(reading);
        }
        if self.is_discontinued(inference_id)? {
            reading.verdict = Truth::Fails;
            reading.note = Some("the step is discontinued by an accepted decision".to_string());
            return Ok(reading);
        }
        ev.path.insert(inference_id.to_string());
        // EVERY PREMISE_OF COUNTS TODAY. Room is left on purpose for a ROLE on
        // the link (req:a-design-can-be-a-proof: a hypothesis assumed and
        // discharged inside a proof by contradiction or an induction step, which
        // the final conclusion does not rest on): an optional edge property,
        // added later, read here, with absent meaning what it means now.
        let mut values = Vec::new();
        for p in self.premises_of(inference_id)? {
            let (truth, why) = self.truth_of(&p, ev, depth - 1)?;
            values.push(truth);
            reading.premises.push(PremiseReading {
                node_type: self.type_holding(&p)?,
                id: p,
                truth,
                why,
            });
        }
        ev.path.remove(inference_id);
        reading.holding = values.iter().filter(|v| **v == Truth::Holds).count();
        reading.failing = values.iter().filter(|v| **v == Truth::Fails).count();
        reading.unknown = values.len() - reading.holding - reading.failing;
        match rec.parsed.and_then(|rule| combine(rule, &values)) {
            Ok(v) => reading.verdict = v,
            Err(why) => reading.note = Some(why),
        }
        Ok(reading)
    }

    /// The steps concluding `id` whose rule holds, sorted — what delivers a
    /// requirement an Inference concludes. `depth` bounds the chain, shared
    /// with the roll-up that asked.
    pub(crate) fn holding_inferences_within(
        &self,
        id: &str,
        depth: usize,
    ) -> Result<Vec<String>, DynoError> {
        let lost = BTreeSet::new();
        let mut ev = Eval::new(&lost);
        let mut holding = Vec::new();
        for s in self.inferences_concluding(id)? {
            if self.step_reading(&s, &mut ev, depth)?.verdict == Truth::Holds {
                holding.push(s);
            }
        }
        Ok(holding)
    }

    // ─── The change walk ───────────────────────────────────────────────────

    /// What losing `seeds` does to the steps that rest on them. Each step a
    /// lost premise reaches reads BROKEN (its rule no longer holds), WEAKENED
    /// (it still holds on its other premises) or UNKNOWN. The walk carries on
    /// past a conclusion that no longer stands — counting it lost in turn — and
    /// stops at one that still stands, so a weakened conclusion's dependents
    /// are never made broken. A seed is counted lost whatever it is: the walk
    /// answers "what if this premise no longer held?", the question to ask
    /// BEFORE the edit (propagate_change, then edit).
    ///
    /// Empty when no seed is a premise of anything — the whole cost for a
    /// design that records no Inference is one adjacency read per seed.
    pub fn inference_walk(&self, seeds: &[&str]) -> Result<Vec<InferenceImpact>, DynoError> {
        let mut lost: BTreeSet<String> = seeds.iter().map(|s| s.to_string()).collect();
        // Every step reached, with the distance it was first reached at.
        let mut reached: BTreeMap<String, usize> = BTreeMap::new();
        let mut frontier: Vec<(String, usize)> = seeds.iter().map(|s| (s.to_string(), 1)).collect();
        loop {
            while let Some((x, distance)) = frontier.pop() {
                let mut rests: Vec<String> = self.steps_resting_on(&x)?;
                // A lost STEP is reached itself: its conclusion is what it costs.
                if self.get_node(node::INFERENCE, &x)?.is_some() {
                    rests.push(x.clone());
                }
                for s in rests {
                    reached.entry(s).or_insert(distance);
                }
            }
            // Carry on past every conclusion that no longer stands. Losses only
            // grow, so a conclusion that stood a round ago may fall now:
            // re-read until nothing new is lost.
            let mut ev = Eval::new(&lost);
            let mut newly: Vec<(String, usize)> = Vec::new();
            for (s, distance) in &reached {
                let Some(c) = self.conclusion_of(s)? else {
                    continue;
                };
                if lost.contains(&c) || newly.iter().any(|(n, _)| *n == c) {
                    continue;
                }
                if self.truth_of(&c, &mut ev, INFERENCE_DEPTH)?.0 == Truth::Fails {
                    newly.push((c, distance + 1));
                }
            }
            if newly.is_empty() {
                break;
            }
            for (c, distance) in newly {
                lost.insert(c.clone());
                frontier.push((c, distance));
            }
        }

        let before_set = BTreeSet::new();
        let mut before_ev = Eval::new(&before_set);
        let mut after_ev = Eval::new(&lost);
        let mut rows = Vec::new();
        for (s, distance) in &reached {
            let before = self.step_reading(s, &mut before_ev, INFERENCE_DEPTH)?;
            // The step's own verdict with the losses counted — not "the step
            // is lost" when it is itself a seed: read its rule over what is left.
            let after = if lost.contains(s) {
                let without_itself: BTreeSet<String> =
                    lost.iter().filter(|x| *x != s).cloned().collect();
                let mut ev = Eval::new(&without_itself);
                let mut r = self.step_reading(s, &mut ev, INFERENCE_DEPTH)?;
                r.verdict = Truth::Fails;
                r.note = Some("the step itself is counted as lost by this change".to_string());
                r
            } else {
                self.step_reading(s, &mut after_ev, INFERENCE_DEPTH)?
            };
            let conclusion_after = match &after.conclusion {
                Some(c) => self.truth_of(c, &mut after_ev, INFERENCE_DEPTH)?.0,
                None => Truth::Unknown,
            };
            let lost_premises: Vec<String> = after
                .premises
                .iter()
                .filter(|p| lost.contains(&p.id))
                .map(|p| p.id.clone())
                .collect();
            rows.push(InferenceImpact {
                inference: s.clone(),
                impact: match after.verdict {
                    Truth::Fails => "broken",
                    Truth::Holds => "weakened",
                    Truth::Unknown => "unknown",
                },
                rule: after.rule,
                k: after.k,
                rule_name: after.rule_name,
                conclusion: after.conclusion,
                distance: *distance,
                before: before.verdict,
                after: after.verdict,
                lost_premises,
                conclusion_after,
                premises: after.premises,
            });
        }
        rows.sort_by(|a, b| {
            a.distance
                .cmp(&b.distance)
                .then_with(|| a.inference.cmp(&b.inference))
        });
        Ok(rows)
    }

    // ─── The finding ───────────────────────────────────────────────────────

    /// Every conclusion whose own status CLAIMS it holds (`met`, `passing`,
    /// `accepted`, `verified`) while the steps concluding it do not support
    /// that: their verdict, ANY across them, is `fails` or `unknown`. Reported,
    /// never repaired (dec:report-dont-judge) — the status is left as claimed.
    pub fn conclusions_that_do_not_follow(&self) -> Result<Vec<UnfollowedConclusion>, DynoError> {
        let mut conclusions: BTreeSet<String> = BTreeSet::new();
        for i in self.scan_nodes(node::INFERENCE)? {
            if let Some(c) = self.conclusion_of(&i.node_id)? {
                conclusions.insert(c);
            }
        }
        let lost = BTreeSet::new();
        let mut ev = Eval::new(&lost);
        let mut out = Vec::new();
        for c in conclusions {
            let Some(node_type) = self.type_holding(&c)? else {
                continue;
            };
            let Some(n) = self.get_node(&node_type, &c)? else {
                continue;
            };
            let Some(claimed) = claimed_holds(&node_type, &n.properties) else {
                continue;
            };
            if self.is_discontinued(&c)? {
                continue;
            }
            let supported = self.truth_of(&c, &mut ev, INFERENCE_DEPTH)?.0;
            if supported == Truth::Holds {
                continue;
            }
            let mut steps = Vec::new();
            for s in self.inferences_concluding(&c)? {
                steps.push(self.step_reading(&s, &mut ev, INFERENCE_DEPTH)?);
            }
            out.push(UnfollowedConclusion {
                name: prop(&n.properties, "name").unwrap_or(&c).to_string(),
                node_id: c,
                node_type,
                claimed: claimed.to_string(),
                supported,
                steps,
            });
        }
        Ok(out)
    }

    /// Every conclusion that follows — at least one of its steps holds — for
    /// the served read of derived relations.
    pub fn conclusions_that_follow(&self) -> Result<Vec<String>, DynoError> {
        let mut conclusions: BTreeSet<String> = BTreeSet::new();
        for i in self.scan_nodes(node::INFERENCE)? {
            if let Some(c) = self.conclusion_of(&i.node_id)? {
                conclusions.insert(c);
            }
        }
        let lost = BTreeSet::new();
        let mut ev = Eval::new(&lost);
        let mut out = Vec::new();
        for c in conclusions {
            for s in self.inferences_concluding(&c)? {
                if self.step_reading(&s, &mut ev, INFERENCE_DEPTH)?.verdict == Truth::Holds {
                    out.push(c);
                    break;
                }
            }
        }
        Ok(out)
    }

    /// Whether a step concludes `id` — the one question the older status
    /// checks ask before speaking about a conclusion this module owns.
    pub fn is_concluded(&self, id: &str) -> Result<bool, DynoError> {
        Ok(!self.inferences_concluding(id)?.is_empty())
    }

    // ─── Writing a step ────────────────────────────────────────────────────

    /// CIRCULAR REASONING IS REFUSED WHERE IT IS WRITTEN — the store's one edge
    /// write calls this for every edge, and it answers only for the two links of
    /// a step. A premise the step's own conclusion already supports, a step
    /// resting on itself, or a conclusion that supports its own step would make
    /// a circle with no ground: its verdict could never be computed, so it is
    /// refused before anything is stored, the choice DECOMPOSES already made for
    /// a decomposition that contains itself. And ONE STEP CONCLUDES EXACTLY ONE
    /// NODE: a second CONCLUDES from the same step is refused. Re-writing an edge
    /// that is already stored is never refused (an import that upserts).
    pub(crate) fn refuse_an_unsound_step_link(
        &self,
        edge_type: &str,
        from_id: &str,
        to_id: &str,
    ) -> Result<(), DynoError> {
        let premise_link = edge_type == edge::PREMISE_OF;
        let conclusion_link = edge_type == edge::CONCLUDES;
        if !premise_link && !conclusion_link {
            return Ok(());
        }
        if self
            .outgoing(from_id, Some(edge_type))?
            .iter()
            .any(|e| e.to_id == to_id)
        {
            return Ok(());
        }
        let refuse = |message: String| DynoError::EdgeValidation {
            edge_type: edge_type.to_string(),
            property: "endpoints".to_string(),
            message,
        };
        if from_id == to_id {
            return Err(refuse(format!(
                "'{from_id}' cannot support itself — that is circular reasoning, and a circle \
                 has no verdict. {EQUIVALENCES}"
            )));
        }
        if conclusion_link {
            if let Some(other) = self.conclusion_of(from_id)? {
                return Err(refuse(format!(
                    "'{from_id}' already concludes '{other}'. One Inference concludes exactly \
                     one node; to support '{to_id}' too, record a second step (add_inference)"
                )));
            }
        }
        // The new link runs from → to; a path back from `to` to `from` along
        // the links already stored closes a circle.
        if let Some(path) = self.support_path(to_id, from_id)? {
            return Err(refuse(format!(
                "this would be circular reasoning: '{to_id}' already supports '{from_id}' \
                 ({}), so '{from_id}' supporting '{to_id}' closes a circle with no ground, and \
                 its verdict could never be computed. {EQUIVALENCES}",
                path.join(" → ")
            )));
        }
        Ok(())
    }

    /// A path from `start` to `goal` along stored PREMISE_OF and CONCLUDES
    /// links, in their direction, when there is one.
    fn support_path(&self, start: &str, goal: &str) -> Result<Option<Vec<String>>, DynoError> {
        let mut parent: BTreeMap<String, String> = BTreeMap::new();
        let mut seen: BTreeSet<String> = BTreeSet::from([start.to_string()]);
        let mut queue = std::collections::VecDeque::from([start.to_string()]);
        while let Some(x) = queue.pop_front() {
            if x == goal {
                let mut path = vec![x.clone()];
                let mut at = x;
                while let Some(p) = parent.get(&at) {
                    path.push(p.clone());
                    at = p.clone();
                }
                path.reverse();
                return Ok(Some(path));
            }
            let mut next: Vec<String> = self
                .outgoing(&x, Some(edge::PREMISE_OF))?
                .into_iter()
                .map(|e| e.to_id)
                .collect();
            next.extend(
                self.outgoing(&x, Some(edge::CONCLUDES))?
                    .into_iter()
                    .map(|e| e.to_id),
            );
            for n in next {
                if seen.insert(n.clone()) {
                    parent.insert(n.clone(), x.clone());
                    queue.push_back(n);
                }
            }
        }
        Ok(None)
    }

    /// Record a step in one call: the Inference with its rule and optional
    /// fields, a PREMISE_OF from each premise, and the one CONCLUDES. Every
    /// refusal comes BEFORE anything is written, and names what would have
    /// worked: a rule outside `all` / `any` / `k_of_n`; a `k_of_n` with no `k`,
    /// or a `k` outside 1..=the number of premises; a `k` on another rule; a
    /// premise or conclusion no single node holds; a premise that is the
    /// conclusion or the step; an id another type holds; a step that already
    /// concludes a different node; and a circle.
    ///
    /// Called again with the same id it REVISES: the rule and fields given are
    /// written over the stored ones, the premises given are ADDED to those
    /// stored (removing one is `delete_edge`), and the conclusion must be the
    /// one it already has.
    pub fn add_inference(&mut self, spec: &InferenceSpec) -> Result<InferenceWritten, DynoError> {
        let invalid = |property: &str, message: String| DynoError::Validation {
            node_type: node::INFERENCE.to_string(),
            property: property.to_string(),
            message,
        };
        if !INFERENCE_RULES.contains(&spec.rule.as_str()) {
            return Err(invalid(
                "rule",
                format!(
                    "'{}' is not a rule. The legal values: `all` (every premise must hold), \
                     `any` (one is enough) and `k_of_n` (at least k of them, with `k`). A named \
                     rule such as modus ponens is `all` with `rule_name`",
                    spec.rule
                ),
            ));
        }
        if spec.id.trim().is_empty() {
            return Err(invalid("id", "the step needs an id".to_string()));
        }
        // The id: free, or already this kind of step.
        let holders = self.node_types_holding(&spec.id)?;
        let existing = if holders.iter().any(|t| t == node::INFERENCE) {
            self.get_node(node::INFERENCE, &spec.id)?
        } else if let Some(t) = holders.first() {
            return Err(invalid(
                "id",
                format!(
                    "'{}' is already a {t}; give the step an id of its own",
                    spec.id
                ),
            ));
        } else {
            None
        };
        // The conclusion.
        let conclusion_type = self.one_type_for(&spec.conclusion, "conclusion")?;
        if let Some(had) = self.conclusion_of(&spec.id)?
            && had != spec.conclusion
        {
            return Err(invalid(
                "conclusion",
                format!(
                    "'{}' already concludes '{had}'. One Inference concludes exactly one node: \
                     record another step for '{}'",
                    spec.id, spec.conclusion
                ),
            ));
        }
        // The premises.
        let mut given: Vec<String> = spec.premises.clone();
        given.sort();
        given.dedup();
        let stored = if existing.is_some() {
            self.premises_of(&spec.id)?
        } else {
            Vec::new()
        };
        if given.is_empty() && stored.is_empty() {
            return Err(invalid(
                "premises",
                "a step needs at least one premise: what supports the conclusion?".to_string(),
            ));
        }
        let mut typed: Vec<(String, String)> = Vec::new();
        for p in &given {
            if *p == spec.conclusion || *p == spec.id {
                return Err(invalid(
                    "premises",
                    format!(
                        "'{p}' cannot be a premise of the step that concludes it — that is \
                         circular reasoning. {EQUIVALENCES}"
                    ),
                ));
            }
            typed.push((p.clone(), self.one_type_for(p, "premise")?));
        }
        let all_premises: BTreeSet<&String> = stored.iter().chain(given.iter()).collect();
        let n = all_premises.len() as i64;
        match (spec.rule.as_str(), spec.k) {
            ("k_of_n", None) => {
                return Err(invalid(
                    "k",
                    format!(
                        "rule `k_of_n` needs `k`: how many of the {n} premise(s) must hold, \
                         1 ≤ k ≤ {n}"
                    ),
                ));
            }
            ("k_of_n", Some(k)) if k < 1 || k > n => {
                return Err(invalid(
                    "k",
                    format!("k = {k} with {n} premise(s): k_of_n needs 1 ≤ k ≤ {n}"),
                ));
            }
            (rule, Some(k)) if rule != "k_of_n" => {
                return Err(invalid(
                    "k",
                    format!(
                        "k = {k} is only read by rule `k_of_n`; with `{rule}` leave it out, or \
                         use `k_of_n`"
                    ),
                ));
            }
            _ => {}
        }
        // A circle: the conclusion already supports the step or one of its
        // premises.
        for target in std::iter::once(&spec.id).chain(given.iter()) {
            if let Some(path) = self.support_path(&spec.conclusion, target)? {
                return Err(invalid(
                    "premises",
                    format!(
                        "this would be circular reasoning: '{}' already supports '{target}' ({}), \
                         so the step would rest on its own conclusion and its verdict could never \
                         be computed. {EQUIVALENCES}",
                        spec.conclusion,
                        path.join(" → ")
                    ),
                ));
            }
        }

        // ── Write. ────────────────────────────────────────────────────────
        let mut props: HashMap<String, Value> = existing
            .as_ref()
            .map(|n| n.properties.clone())
            .unwrap_or_default();
        let name = spec.name.clone().unwrap_or_else(|| {
            existing
                .as_ref()
                .and_then(|n| prop(&n.properties, "name").map(str::to_string))
                .unwrap_or_else(|| match spec.k {
                    Some(k) => format!("{k} of {n} premise(s) support {}", spec.conclusion),
                    None => format!(
                        "{} of {n} premise(s) support {}",
                        spec.rule, spec.conclusion
                    ),
                })
        });
        props.insert("name".into(), Value::String(name));
        props.insert("rule".into(), Value::String(spec.rule.clone()));
        match spec.k {
            Some(k) => {
                props.insert("k".into(), Value::Int(k));
            }
            None => {
                props.remove("k");
            }
        }
        if let Some(r) = &spec.rule_name {
            props.insert("rule_name".into(), Value::String(r.clone()));
        }
        if let Some(w) = &spec.warrant {
            props.insert("warrant".into(), Value::String(w.clone()));
        }
        let mut builder = Props::new();
        for (k, v) in props {
            builder = builder.set(&k, v);
        }
        let stored_node = self.upsert_node(node::INFERENCE, &spec.id, builder)?;
        let mut drawn = Vec::new();
        for (p, ty) in &typed {
            if stored.contains(p) {
                continue;
            }
            self.create_edge(
                edge::PREMISE_OF,
                ty,
                p,
                node::INFERENCE,
                &spec.id,
                Props::new(),
            )?;
            drawn.push(format!("{p} PREMISE_OF {}", spec.id));
        }
        if self.conclusion_of(&spec.id)?.is_none() {
            self.create_edge(
                edge::CONCLUDES,
                node::INFERENCE,
                &spec.id,
                &conclusion_type,
                &spec.conclusion,
                Props::new(),
            )?;
            drawn.push(format!("{} CONCLUDES {}", spec.id, spec.conclusion));
        }
        Ok(InferenceWritten {
            node: stored_node,
            edges_drawn: drawn,
            revised: existing.is_some(),
        })
    }

    /// The one type holding `id`, or a refusal naming the role it was given in.
    fn one_type_for(&self, id: &str, role: &str) -> Result<String, DynoError> {
        let held = self.node_types_holding(id)?;
        match held.len() {
            1 => Ok(held.into_iter().next().unwrap_or_default()),
            0 => Err(DynoError::Validation {
                node_type: node::INFERENCE.to_string(),
                property: role.to_string(),
                message: format!(
                    "no node '{id}' exists to be the {role}; record it first, then the step"
                ),
            }),
            _ => Err(DynoError::Validation {
                node_type: node::INFERENCE.to_string(),
                property: role.to_string(),
                message: format!(
                    "'{id}' is held by more than one type ({}), so the {role} is ambiguous",
                    held.join(", ")
                ),
            }),
        }
    }
}

/// What [`DesignGraph::add_inference`] is asked to write.
#[derive(Debug, Clone, Default)]
pub struct InferenceSpec {
    /// The step's id.
    pub id: String,
    /// `all`, `any` or `k_of_n`.
    pub rule: String,
    /// For `k_of_n`: how many premises must hold.
    pub k: Option<i64>,
    /// The premises, by id — any node the design holds.
    pub premises: Vec<String>,
    /// The one node the step concludes.
    pub conclusion: String,
    /// The rule's name, e.g. `modus ponens` — a label.
    pub rule_name: Option<String>,
    /// Why these premises support this conclusion.
    pub warrant: Option<String>,
    /// A short handle; composed from the rule and conclusion when absent.
    pub name: Option<String>,
}

/// What [`DesignGraph::add_inference`] wrote.
#[derive(Debug, Clone)]
pub struct InferenceWritten {
    /// The step as stored.
    pub node: crate::foundation::store::StoredNode,
    /// Each edge it drew, as `A REL B`.
    pub edges_drawn: Vec<String>,
    /// True when the id was already a step and this call revised it.
    pub revised: bool,
}

#[cfg(test)]
mod kleene {
    use super::*;
    use Truth::{Fails as F, Holds as H, Unknown as U};

    #[test]
    fn all_any_and_k_of_n_are_three_valued() {
        assert_eq!(combine(InferenceRule::All, &[H, H]), Ok(H));
        assert_eq!(combine(InferenceRule::All, &[H, U]), Ok(U));
        assert_eq!(combine(InferenceRule::All, &[F, U]), Ok(F));
        assert_eq!(combine(InferenceRule::Any, &[F, U]), Ok(U));
        assert_eq!(combine(InferenceRule::Any, &[H, U]), Ok(H));
        assert_eq!(combine(InferenceRule::Any, &[F, F]), Ok(F));
        assert_eq!(combine(InferenceRule::KOfN(2), &[H, H, F]), Ok(H));
        assert_eq!(combine(InferenceRule::KOfN(2), &[H, F, F]), Ok(F));
        assert_eq!(combine(InferenceRule::KOfN(2), &[H, U, F]), Ok(U));
        assert!(combine(InferenceRule::KOfN(4), &[H, H, H]).is_err());
        assert!(combine(InferenceRule::KOfN(0), &[H]).is_err());
        assert!(combine(InferenceRule::All, &[]).is_err());
    }
}
