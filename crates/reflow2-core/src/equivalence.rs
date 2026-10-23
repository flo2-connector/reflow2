//! PROVEN EQUIVALENCE, and the CIRCULARITY CHECK that reads it.
//!
//! **Equivalence** (`req:two-statements-can-be-recorded-as-proven-equivalent`,
//! Anthony 2026-10-22, the Riemann-map rung). Two statements a cited or proven
//! biconditional shows equivalent are joined by an `EQUIVALENT_TO` edge, which
//! may name the biconditional in `by`. The map of the Riemann Hypothesis had to
//! join RH and Robin's inequality with DUPLICATES, and detect_defects then
//! proposed MERGING them. The edge:
//!
//! 1. never reads as duplicates and is never offered for a merge: HEAL refuses
//!    the merge of an equivalent pair in propose and apply alike, and a
//!    DUPLICATES edge drawn beside one raises no merge defect and no
//!    possible_duplicate gap;
//! 2. leaves each side's own reading unchanged: no truth is read across it, so
//!    an open conjecture stays open and neither side becomes conditional on the
//!    other;
//! 3. is walked by a what-if (propagate_from) both ways, which names each
//!    equivalent a changed statement has (`equivalents_reached`);
//! 4. gives the circularity check what it needs.
//!
//! **Circularity** (`dec:idea-speculate-the-bridge-that-would-close-a-gap-
//! then-test-it`, option c). A step whose premise is equivalent to its
//! conclusion — through any chain of EQUIVALENT_TO edges — or is the same
//! formula (`same_formula_as`) only restates the gap it would close: in
//! mathematics, a lemma equivalent to RH "closes" RH's hole by assuming it. It
//! is FLAGGED (the step's `circular`, reasoning_report's `circular_steps`, a
//! `reasoning_not_as_stated` gap), never refused and never read as failing:
//! the step reads by its rule as before, and whether it begs the question is
//! for the person to judge.

use std::collections::{BTreeSet, VecDeque};

use crate::foundation::core::{DynoError, Value};
use crate::graph::DesignGraph;
use crate::inference::Truth;
use crate::nodes::{edge, node};

/// One statement another is recorded as proven equivalent to.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Equivalent {
    /// The other statement.
    pub id: String,
    /// The biconditional the edge names in `by`, when it names one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// How the equivalence stands: `holds` or `conditional` (the
    /// biconditional's computed truth), `unsupported` or `refuted` when it
    /// does not hold, `asserted` when the edge names no biconditional.
    pub reads: &'static str,
    /// Why, in a sentence.
    pub why: String,
}

/// A step that only restates the gap it would close.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CircularStep {
    /// The step.
    pub step: String,
    /// What it concludes.
    pub conclusion: String,
    /// The premise equivalent to the conclusion, or the same formula.
    pub premise: String,
    /// Why, in a sentence.
    pub why: String,
}

fn same_formula_as(props: &std::collections::HashMap<String, Value>) -> Option<String> {
    props
        .get("same_formula_as")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

impl DesignGraph {
    /// The statements `id` is DIRECTLY recorded equivalent to, either way
    /// round, each with how the equivalence stands. Sorted by id; empty when
    /// none.
    pub fn equivalents_of(&self, id: &str) -> Result<Vec<Equivalent>, DynoError> {
        let mut out: Vec<Equivalent> = Vec::new();
        let edges = self
            .outgoing(id, Some(edge::EQUIVALENT_TO))?
            .into_iter()
            .map(|e| (e.to_id, e.properties))
            .chain(
                self.incoming(id, Some(edge::EQUIVALENT_TO))?
                    .into_iter()
                    .map(|e| (e.from_id, e.properties)),
            );
        for (other, props) in edges {
            if other == id || out.iter().any(|q| q.id == other) {
                continue;
            }
            let by = props
                .get("by")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let (reads, why) = match &by {
                None => (
                    "asserted",
                    "recorded equivalent, and the edge names no biconditional that shows it"
                        .to_string(),
                ),
                Some(b) => {
                    let (truth, why) = self.premise_truth(b)?;
                    let conditional = !self.assumptions_under(b)?.is_empty()
                        || !self.unverified_under(b)?.is_empty()
                        || !self.bridges_under(b)?.is_empty();
                    let reads = match truth {
                        Truth::Holds if conditional => "conditional",
                        Truth::Holds => "holds",
                        Truth::Fails => "refuted",
                        Truth::Unknown => "unsupported",
                    };
                    (reads, format!("shown by {b}: {why}"))
                }
            };
            out.push(Equivalent {
                id: other,
                by,
                reads,
                why,
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// Every statement `id` is equivalent to through any chain of
    /// EQUIVALENT_TO edges, `id` itself excluded. Sorted.
    pub fn equivalence_class(&self, id: &str) -> Result<BTreeSet<String>, DynoError> {
        let mut seen: BTreeSet<String> = BTreeSet::from([id.to_string()]);
        let mut queue: VecDeque<String> = VecDeque::from([id.to_string()]);
        while let Some(x) = queue.pop_front() {
            let next = self
                .outgoing(&x, Some(edge::EQUIVALENT_TO))?
                .into_iter()
                .map(|e| e.to_id)
                .chain(
                    self.incoming(&x, Some(edge::EQUIVALENT_TO))?
                        .into_iter()
                        .map(|e| e.from_id),
                );
            for n in next {
                if seen.insert(n.clone()) {
                    queue.push_back(n);
                }
            }
        }
        seen.remove(id);
        Ok(seen)
    }

    /// Every pair of statements joined directly by an EQUIVALENT_TO edge,
    /// each ordered, so a lookup does not depend on which way it was drawn.
    pub(crate) fn equivalent_pairs(&self) -> Result<BTreeSet<(String, String)>, DynoError> {
        let index = self.node_type_index()?;
        let mut out = BTreeSet::new();
        for e in self.all_edges_of_type(edge::EQUIVALENT_TO, &index)? {
            out.insert(crate::detect::ordered_pair(&e.from_id, &e.to_id));
        }
        Ok(out)
    }

    /// Whether the step `inference_id` is CIRCULAR: a premise of it is
    /// equivalent to its conclusion (through any chain of EQUIVALENT_TO
    /// edges) or is the same formula (`same_formula_as`, either way, or both
    /// naming one). `None` when it is not, or concludes nothing.
    pub fn circularity_of(&self, inference_id: &str) -> Result<Option<CircularStep>, DynoError> {
        let Some(conclusion) = self.conclusion_of(inference_id)? else {
            return Ok(None);
        };
        let premises = self.premises_of(inference_id)?;
        if premises.is_empty() {
            return Ok(None);
        }
        let class = self.equivalence_class(&conclusion)?;
        if let Some(p) = premises.iter().find(|p| class.contains(*p)) {
            let direct = self
                .equivalents_of(&conclusion)?
                .into_iter()
                .find(|q| &q.id == p);
            let how = match direct.and_then(|q| q.by) {
                Some(b) => format!("is proven equivalent to it by {b}"),
                None => "is recorded equivalent to it".to_string(),
            };
            return Ok(Some(CircularStep {
                step: inference_id.to_string(),
                conclusion: conclusion.clone(),
                premise: p.clone(),
                why: format!(
                    "circular: its premise {p} {how}, so the step only restates the gap it would \
                     close — it shows {conclusion} no more than {p} is shown"
                ),
            }));
        }
        let Some(cprops) = self.node_props(&conclusion)? else {
            return Ok(None);
        };
        let c_formula = same_formula_as(&cprops);
        for p in &premises {
            let Some(pprops) = self.node_props(p)? else {
                continue;
            };
            let p_formula = same_formula_as(&pprops);
            let same = c_formula.as_deref() == Some(p.as_str())
                || p_formula.as_deref() == Some(conclusion.as_str())
                || (c_formula.is_some() && c_formula == p_formula);
            if same {
                return Ok(Some(CircularStep {
                    step: inference_id.to_string(),
                    conclusion: conclusion.clone(),
                    premise: p.clone(),
                    why: format!(
                        "circular: its premise {p} is the same formula as its conclusion \
                         {conclusion} (same_formula_as), so the step only restates it"
                    ),
                }));
            }
        }
        Ok(None)
    }

    /// Every circular step in the design, sorted by step id.
    pub fn circular_steps(&self) -> Result<Vec<CircularStep>, DynoError> {
        let mut out = Vec::new();
        let mut steps: Vec<String> = self
            .scan_nodes(node::INFERENCE)?
            .into_iter()
            .map(|n| n.node_id)
            .collect();
        steps.sort();
        for s in steps {
            if let Some(c) = self.circularity_of(&s)? {
                out.push(c);
            }
        }
        Ok(out)
    }
}
