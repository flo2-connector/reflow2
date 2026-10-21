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
//! `contested` Decision, and then every claim of the reasoning those do not
//! reach (below). From each node it follows:
//!
//! - the steps that conclude it, and their premises;
//! - the objections against it (opposing CONTRADICTS), whose own objections
//!   are the replies;
//! - read over the whole design, also the claim an objection is AGAINST, so a
//!   claim only an objection points to (a refuted conjecture) is read;
//! - for a `contested` Decision, the nodes it governs, and for a node one
//!   governs, that Decision.
//!
//! A whole-design read covers EVERY claim of the reasoning: every node a step
//! concludes, every statement (`statement_kind`), and both ends of every
//! opposing CONTRADICTS. One the walk from the top conclusions does not reach
//! becomes a root of its own, an objection's target before the objection, so a
//! bare conjecture refuted by a bare counterexample is read from the
//! conjecture.
//! `fact:root-cause-the-reasoning-walk-never-reaches-a-claim-only-an-objection-points-to-2026-10-18`:
//! until 2026-10-18 the walk followed an objection only from its target's
//! side, and a refuted conjecture was never read. A read under one `claim`
//! still reads only what is under it: the claim's support and what stands
//! against it, never what the claim itself objects to.
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

/// How many claims one page returns when the caller does not say.
pub const REASONING_DEFAULT_LIMIT: usize = 60;
/// The most one page returns, whatever the caller says, and the most records
/// the walk reads: past it, what is left is named in `not_walked`.
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
const NOTHING_TO_READ: &str = "This design records no inference step, no statement \
     (`statement_kind`), no objection and no contested decision, so there is no line of reasoning \
     to read back. A claim's support is recorded with \
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
     for), a CASE SPLIT whose cases nothing declares exhaustive (the step does not hold), and an \
     INCONSISTENCY (a falsum derived under no open assumption: the foundations contradict each \
     other). An UNVERIFIED CITATION is a result cited from elsewhere that no pinned source \
     documents: what rests on it reads conditional until its source is checked. And what has LOST \
     ITS REASON NOW: a REQUIREMENT whose reason to exist reads lost or stands on its Decision's \
     acceptance alone, an ACCEPTED DECISION no line of whose chain holds, and a STALE CHECK whose \
     conditions no longer hold. Confirming a \
     step is the owner's word, never an agent's. reasoning_report reads each one in full.";

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
    /// Results cited from elsewhere (taken as given) that no pinned source
    /// documents: what rests on each reads conditional on it.
    pub unverified_citations: usize,
    pub unverified_citation_ids: Vec<String>,
    /// Case splits whose cases nothing declares exhaustive: each does not
    /// hold.
    pub cases_not_declared_exhaustive: usize,
    pub cases_not_declared_exhaustive_ids: Vec<String>,
    /// Falsums derived under no open assumption: the foundations they rest on
    /// contradict each other.
    pub inconsistencies: usize,
    pub inconsistency_ids: Vec<String>,
    /// WHAT HAS LOST ITS REASON NOW, read across the whole design and not
    /// only as a what-if's delta (req:what-has-lost-its-reason-now-is-one-
    /// read; the capstone trial: after U1 the 10–12 GHz requirement had
    /// already lost its reason and no read listed it): requirements whose
    /// reason to exist reads `lost` or `stands_on_acceptance_alone`.
    pub reasons_lost: usize,
    pub reason_lost_ids: Vec<String>,
    /// Accepted Decisions a chain concludes and no line of it holds.
    pub decisions_without_a_holding_chain: usize,
    pub decision_without_a_holding_chain_ids: Vec<String>,
    /// Checks whose conditions no longer hold: stale.
    pub stale_checks: usize,
    pub stale_check_ids: Vec<String>,
    /// How many ids each list may name.
    pub list_cap: usize,
    /// What these counts mean.
    pub note: &'static str,
}

impl ReasoningDebt {
    /// Whether anything is owed.
    pub fn is_owed(&self) -> bool {
        self.confirmation_owed() || self.not_as_stated() || self.reasons_lost_now()
    }

    /// Whether confirming or citing is owed: an unconfirmed step, an uncited
    /// premise, or a claim shown but not proven.
    pub fn confirmation_owed(&self) -> bool {
        self.unconfirmed_steps
            + self.uncited_premises
            + self.shown_not_proven
            + self.unverified_citations
            > 0
    }

    /// Whether the reasoning is NOT AS STATED somewhere: an undischarged
    /// assumption, a generalization that does not hold, or a stated
    /// independence the lines do not bear out.
    pub fn not_as_stated(&self) -> bool {
        self.undischarged_assumptions
            + self.generalizations_not_holding
            + self.independence_not_borne_out
            + self.cases_not_declared_exhaustive
            + self.inconsistencies
            > 0
    }

    /// Whether something has LOST ITS REASON NOW: a requirement whose reason
    /// to exist is lost or stands on acceptance alone, an accepted Decision
    /// with no holding chain, or a stale check.
    pub fn reasons_lost_now(&self) -> bool {
        self.reasons_lost + self.decisions_without_a_holding_chain + self.stale_checks > 0
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
    /// When a `contested` Decision governs this step — a dispute between
    /// rival readings kept open on purpose — that Decision, and whether it
    /// holds this step as ITS reading (`held`: a GOVERNED_BY with `ruling:
    /// held`) or as a `rival` (req:the-read-back-under-a-claim-shows-a-
    /// dispute-kept-open-and-the-owners-chosen-reading). Absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dispute: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as_reading: Option<&'static str>,
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

/// A claim's truth as the read-back serves it: the three values, and
/// `conditional` for a claim that holds only under an assumption no step on
/// the way discharges
/// (req:a-correct-proof-reads-correct-assumptions-objections-and-conditional-claims,
/// part 3). Inside the argument such a claim holds, and the steps resting on
/// it read it so; a READER is never told it holds on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimTruth {
    /// Computed to hold, resting on no open assumption.
    Holds,
    /// Holds only under the assumption(s) in `assumes`, which no step on the
    /// way to it discharges, or only if the unverified citation(s) in
    /// `rests_on_unverified` say what they are recalled to say.
    Conditional,
    /// Holds only through a STATISTICAL TEST (`rests_on_tests`): supported at
    /// that test's α, evidence and never proof — a reader is never told it
    /// holds (req:a-hypothesis-tests-verdict-reads-as-supported-at-alpha-
    /// never-as-shown).
    Supported,
    /// Computed to fail: refuted.
    Fails,
    /// Nothing reflow2 computes says either way.
    Unknown,
}

impl ClaimTruth {
    pub(crate) fn of(
        truth: Truth,
        assumes: &[String],
        unverified: &[String],
        tests: &[String],
    ) -> Self {
        match truth {
            Truth::Holds if !assumes.is_empty() || !unverified.is_empty() => {
                ClaimTruth::Conditional
            }
            Truth::Holds if !tests.is_empty() => ClaimTruth::Supported,
            Truth::Holds => ClaimTruth::Holds,
            Truth::Fails => ClaimTruth::Fails,
            Truth::Unknown => ClaimTruth::Unknown,
        }
    }
}

/// A chain's own verdict, read beside an accepted Decision's acceptance.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChainVerdict {
    /// What the steps concluding it yield, ANY across them.
    pub verdict: Truth,
    /// Why, in a sentence.
    pub why: String,
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
    /// WHAT IT SAYS, in its own words: a statement's `statement`, a
    /// Decision's `chose` once it records one (what was decided, not the
    /// deliberation's "options, none chosen"), else its `decision`, otherwise
    /// its `description` or a change's `summary`. Never cut: a claim read back without its words is not read
    /// back (req:the-reasoning-read-back-keeps-the-words-of-the-claims-it-returns).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub says: Option<String>,
    /// Computed — holds, conditional, supported, fails or unknown — never the
    /// node's own status. `conditional` when it holds only under an
    /// assumption no step on the way discharges (named in `assumes` and in
    /// `why`); `supported` when it holds only through a statistical test
    /// (`rests_on_tests`).
    pub truth: ClaimTruth,
    /// Why, in a sentence.
    pub why: String,
    /// For a node a step concludes: proven, shown, contested or unproven.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_status: Option<ProofStatus>,
    /// For a Decision a step concludes: its status as its OWNER set it
    /// (`accepted`, `proposed` ...), read beside `chain`. An accepted one
    /// holds on that word, whatever its chain says.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<String>,
    /// For a Decision a step concludes: what its CHAIN yields on its own,
    /// beside the owner's `acceptance`, so a first, incomplete chain reads as
    /// an incomplete chain and never as the decision being unsupported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain: Option<ChainVerdict>,
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
    /// ITS GRADE, as fields: a piece of evidence's source reliability,
    /// information credibility and date, a judgement's confidence
    /// (req:evidence-carries-its-source-grade-and-confidence-and-is-not-intent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grade: Option<crate::inference::Grade>,
    /// THE CASES IT GIVES NO VERDICT ON (`silent_on`): a principle silent on
    /// a case is neither for nor against it (item 8 of req:small-read-back-
    /// faults-from-the-cogito-trial-are-fixed). Empty when it says none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub silent_on: Vec<String>,
    /// THE FORMULA IT IS AN OCCURRENCE OF (`same_formula_as`), when it names
    /// one: grouped under `formulas` in the read-back.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_formula_as: Option<String>,
    /// What the statement says it is (`statement_kind`) and how it reads:
    /// a conjecture reads `open` or `refuted`, a theorem `stated` until a
    /// step concludes it. Absent when it says nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement: Option<StatementReading>,
    /// The assumptions it holds under: `assumed` premises beneath it that no
    /// step on the way discharges. With several lines, the lightest line's.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assumes: Vec<String>,
    /// EVERY LINE it holds on, each with what it holds under, when there is
    /// more than one (req:a-claim-with-several-lines-reads-each-lines-
    /// assumptions): it holds under any one of them, never only the first.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holds_under_any_of: Vec<crate::inference::LineConditions>,
    /// True when it holds on more minimal lines than `LINES_CAP` keeps, and
    /// only the lightest are listed.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub lines_capped: bool,
    /// THE UNVERIFIED CITATIONS it rests on (itself included, for one): results
    /// cited from elsewhere that no pinned source documents. While any is
    /// here, `truth` reads `conditional`, never `holds`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rests_on_unverified: Vec<String>,
    /// THE STATISTICAL TESTS it rests on, at any depth: what makes it read
    /// `supported` rather than `holds`. Empty when it rests on none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rests_on_tests: Vec<String>,
    /// The `contested` Decision that keeps its dispute open on purpose: one
    /// governing the claim itself, or one governing the steps that conclude
    /// it (rival readings of one argument).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contested_by_design: Option<String>,
    /// WHAT KEEPS IT FROM HOLDING, WHEN ONLY A DISPUTE DOES: the objections
    /// standing against it, against a step under it or against a premise it
    /// rests on, and the `contested` Decisions it rests on
    /// (req:a-conclusion-whose-every-route-is-contested-reads-contested).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contested_by: Vec<String>,
    /// For a check a reasoned claim constrains: its conditions, read, and
    /// whether it is STALE (req:a-check-resting-on-a-claim-that-loses-
    /// support-reads-stale).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_conditions: Option<crate::inference::CheckConditions>,
    /// For a Requirement a Decision governs: WHY IT EXISTS — each governing
    /// Decision and its support — read apart from `truth`, which says whether
    /// it is delivered (crate::reason_to_exist).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_to_exist: Option<crate::reason_to_exist::ReasonToExist>,
    /// The edges that name this claim in `argued_by`: what it argues for, each
    /// with how the argument reads (`argued`, `circular` ...).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub argues_for: Vec<crate::reason_to_exist::ArguedEdge>,
}

/// One formula and the occurrences of it the walk read.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FormulaOccurrences {
    /// The formula: the record its occurrences name.
    pub formula: String,
    /// Every occurrence, the formula's own record first when the walk read it.
    pub occurrences: Vec<String>,
}

/// A piece of evidence or an analytic judgement, read: what it is, how it
/// reads, and its grade.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EvidenceItem {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `evidence` or `judgement`, or another kind that carries a grade.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// How it reads: `cited_as_evidence`, `evidence_unverified`, or a
    /// judgement's proof status (`supported` ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reads: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grade: Option<crate::inference::Grade>,
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

/// A step an objection is aimed at, defeated or contested by it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StepObjected {
    /// The step.
    pub step: String,
    /// The claim it concludes.
    pub conclusion: String,
    /// `defeated` (an objection to it holds: it supports nothing) or
    /// `contested` (one stands: it supports its conclusion only once that is
    /// settled).
    pub reads: &'static str,
    /// The objections that refute or stand against it.
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
    /// The side the Decision holds as its reading (GOVERNED_BY `ruling:
    /// held`), the others being rivals. Absent when it names none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub held_reading: Option<String>,
}

/// One position of a question kept open: a node a Decision governs, read
/// with what it rests on — the issue/positions read-back (idea (f) of
/// dec:idea-what-reflow2-takes-from-the-argumentation-frameworks).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Position {
    /// The position.
    pub id: String,
    /// Its name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Its computed truth, as its claim reading says it.
    pub truth: ClaimTruth,
    /// Proven, shown, conditional, supported, contested or unproven, when a
    /// step concludes it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_status: Option<ProofStatus>,
    /// `held` when the Decision holds it as its reading, `rival` when it
    /// names another, absent when it names none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as_reading: Option<&'static str>,
    /// The terminal premises its support rests on.
    pub rests_on: Vec<String>,
    /// The assumptions it holds under, lightest line.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assumes: Vec<String>,
    /// Every line it holds on, when there are several.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holds_under_any_of: Vec<crate::inference::LineConditions>,
    /// The objections standing against it or against its routes.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contested_by: Vec<String>,
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
    /// ONE PAGE of the reasoning's nodes, each WHOLE, in proof order: the
    /// order the walk reached them, breadth first from the roots. `offset`
    /// says where the page starts, `next_offset` where to read on.
    pub claims: Vec<ClaimReading>,
    /// Where in the walk this page starts (0 is the roots).
    pub offset: usize,
    /// How many nodes the whole walk holds: every page together.
    pub total_claims: usize,
    /// How many come after this page.
    pub remaining: usize,
    /// Pass it as `offset` to read the next page; absent on the last.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
    /// What the reasoning rests on without showing it: every premise of a
    /// step in the WALK (not only this page) that no step concludes. The
    /// summary lists below cover the whole walk the same way, so a short page
    /// never reads as a reasoning that rests on nothing.
    pub rests_on: Vec<String>,
    /// UNDER ONE CLAIM: what the objections against it, or against what it
    /// rests on, rest on — what stands against the claim, never what supports
    /// it (req:what-a-claim-rests-on-excludes-the-premises-of-objections-
    /// against-it). Absent on a whole-design read, and when nothing objects.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub against_rests_on: Vec<String>,
    /// Premises that cite no source: they rest on someone's word. Fine for a
    /// value; a factual one needs a source. A premise marked TAKEN AS GIVEN is
    /// not here: it is in `stated_foundations`.
    pub uncited_premises: Vec<String>,
    /// Premises marked TAKEN AS GIVEN: definitions and axioms the reasoning
    /// states as its starting point.
    pub stated_foundations: Vec<String>,
    /// Points still contested.
    pub still_contested: Vec<StillContested>,
    /// STEPS AN OBJECTION IS AIMED AT that it defeats or contests
    /// (req:an-objection-aimed-at-a-step-is-read-against-that-step): each
    /// step, the claim it concludes, how it reads, and what stands against
    /// it. Each is also read in full under its claim's `support`. Absent when
    /// no objection aims at a step, or every one is answered.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub steps_objected: Vec<StepObjected>,
    /// Disputes kept open on purpose.
    pub contested_by_design: Vec<HeldOpen>,
    /// READ ON A DECISION: the positions it governs, each with its truth,
    /// what it rests on and whether the Decision holds it as its reading —
    /// the read-back from a question to its positions
    /// (req:the-read-back-under-a-claim-shows-a-dispute-kept-open-and-the-
    /// owners-chosen-reading). Absent for any other claim.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub positions: Vec<Position>,
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
    /// THE EVIDENCE AND JUDGEMENTS the walk read, each with how it reads and
    /// its grade: reports cited as evidence (or unverified), and analytic
    /// judgements with their confidence (req:evidence-carries-its-source-
    /// grade-and-confidence-and-is-not-intent). Empty when there are none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceItem>,
    /// ONE FORMULA, SEVERAL OCCURRENCES: each formula some walked statement
    /// names in `same_formula_as`, with every occurrence the walk read
    /// (dec:idea-one-formula-under-several-suppositions-is-one-node-or-one-
    /// per-occurrence, option b). Empty when none names one.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub formulas: Vec<FormulaOccurrences>,
    /// How many nodes the walk read.
    pub walked: usize,
    /// Whether the walk stopped at its cap ([`REASONING_MAX_LIMIT`]) before it
    /// reached everything. A page's `limit` never caps the walk: it only says
    /// how many of the walked nodes this reply returns.
    pub capped: bool,
    /// NOT WALKED: the nodes the walk reached and did not read, because it
    /// stopped at its cap. What rests under them is in none of the summary
    /// lists, and the lists say nothing about it either way. Empty unless
    /// `capped`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub not_walked: Vec<String>,
    /// What `not_walked` means, in a sentence; present only with it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_walked_note: Option<String>,
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
    /// `limit` bounds the page (default [`REASONING_DEFAULT_LIMIT`], at most
    /// [`REASONING_MAX_LIMIT`]), from the start of the walk. Refused when
    /// `claim` names no node. [`Self::reasoning_report_from`] reads on.
    pub fn reasoning_report(
        &self,
        claim: Option<&str>,
        limit: Option<usize>,
    ) -> Result<ReasoningReport, DynoError> {
        self.reasoning_report_from(claim, limit, None)
    }

    /// [`Self::reasoning_report`], from `offset` in the walk: the page a
    /// reply's `next_offset` names.
    pub fn reasoning_report_from(
        &self,
        claim: Option<&str>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<ReasoningReport, DynoError> {
        self.reasoning_report_walking(claim, limit, offset, REASONING_MAX_LIMIT)
    }

    /// The read-back with the walk's cap stated, so a test can reach it
    /// without a design of 500 records. Callers use
    /// [`Self::reasoning_report_from`].
    #[doc(hidden)]
    pub fn reasoning_report_walking(
        &self,
        claim: Option<&str>,
        limit: Option<usize>,
        offset: Option<usize>,
        walk_cap: usize,
    ) -> Result<ReasoningReport, DynoError> {
        let limit = limit
            .unwrap_or(REASONING_DEFAULT_LIMIT)
            .clamp(1, REASONING_MAX_LIMIT);
        let walk_cap = walk_cap.max(1);
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
        // Over the whole design an objection also leads to its target; under
        // one claim the read stays below that claim.
        let whole = claim.is_none();

        // THE WALK, breadth first from the roots, each level in id order. It
        // reads the whole reasoning up to its cap, whatever one page returns,
        // so every summary list below is the walk's and never the page's.
        let mut order: Vec<String> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut queue: VecDeque<String> = roots.iter().cloned().collect();
        seen.extend(roots.iter().cloned());
        let mut capped = false;
        while let Some(id) = queue.pop_front() {
            if order.len() == walk_cap {
                capped = true;
                queue.push_front(id);
                break;
            }
            order.push(id.clone());
            for next in self.reasoning_neighbours(&id, whole)? {
                if seen.insert(next.clone()) {
                    queue.push_back(next);
                }
            }
        }
        let not_walked: Vec<String> = if capped {
            queue.into_iter().collect()
        } else {
            Vec::new()
        };

        let mut claims = Vec::with_capacity(order.len());
        for id in &order {
            claims.push(self.claim_reading(id)?);
        }
        // What each claim argues for: every edge naming it in `argued_by`,
        // read once for the whole walk.
        let mut argued = self.edges_argued_by()?;
        for c in &mut claims {
            if let Some(edges) = argued.remove(&c.id) {
                c.argues_for = edges;
            }
        }
        // Each premise by its name, so a step reads with the words of what it
        // rests on, not ids alone.
        let mut names: BTreeMap<String, Option<String>> = BTreeMap::new();
        for c in &mut claims {
            for st in &mut c.support {
                for p in &mut st.reading.premises {
                    if !names.contains_key(&p.id) {
                        let name = match &p.node_type {
                            Some(ty) => self
                                .get_node(ty, &p.id)?
                                .and_then(|n| prop(&n.properties, "name")),
                            None => None,
                        };
                        names.insert(p.id.clone(), name);
                    }
                    p.name = names[&p.id].clone();
                }
            }
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
        // on: it is in `assumptions`. A STEP used as a premise is not either:
        // it is walked into, and what it rests on is here.
        //
        // UNDER ONE CLAIM, ONLY ITS OWN SUPPORT (req:what-a-claim-rests-on-
        // excludes-the-premises-of-objections-against-it, the trolley trial):
        // the premises of objections against the claim or its foundations
        // are what stands against it, listed apart in `against_rests_on`.
        // Measured: "Switch: turning permissible" was said to rest on the
        // Footbridge judgment, a premise of an objection to one of its
        // principles.
        let support_closure: Option<BTreeSet<String>> = match claim {
            Some(c) => Some(self.support_closure(c)?),
            None => None,
        };
        let mut rests_on: BTreeSet<String> = BTreeSet::new();
        let mut against: BTreeSet<String> = BTreeSet::new();
        for c in &claims {
            let own = support_closure
                .as_ref()
                .is_none_or(|closure| closure.contains(&c.id));
            for s in &c.support {
                for p in &s.reading.premises {
                    if p.role == Some("assumed")
                        || p.node_type.as_deref() == Some(node::INFERENCE)
                        || !self.inferences_concluding(&p.id)?.is_empty()
                    {
                        continue;
                    }
                    if own {
                        rests_on.insert(p.id.clone());
                    } else {
                        against.insert(p.id.clone());
                    }
                }
            }
        }
        let against_rests_on: Vec<String> = against.difference(&rests_on).cloned().collect();
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

        let mut discharged_only: BTreeSet<String> = BTreeSet::new();
        for a in &assumptions {
            if a.assumption.discharged_by.is_empty() {
                continue;
            }
            let relied_on = self
                .outgoing(&a.assumption.id, Some(edge::PREMISE_OF))?
                .iter()
                .any(|e| e.properties.get("role").and_then(Value::as_str) != Some("assumed"));
            if !relied_on {
                discharged_only.insert(a.assumption.id.clone());
            }
        }
        let mut still_contested = Vec::new();
        let mut contested_by_design = Vec::new();
        let mut unsupported = Vec::new();
        let mut steps_objected = Vec::new();
        for c in &claims {
            for s in &c.support {
                if let Some(reads @ ("defeated" | "contested" | "conceded")) = s.reading.objected {
                    steps_objected.push(StepObjected {
                        step: s.reading.inference.clone(),
                        conclusion: c.id.clone(),
                        reads,
                        against: s
                            .reading
                            .objections
                            .iter()
                            .filter(|o| o.standing != "answered")
                            .map(|o| o.id.clone())
                            .collect(),
                    });
                }
            }
        }
        for c in &claims {
            let standing: Vec<String> = c
                .objections
                .iter()
                .filter(|o| o.standing == "stands")
                .map(|o| o.id.clone())
                .collect();
            // CONTESTED THROUGH ITS ROUTES (req:a-conclusion-whose-every-
            // route-is-contested-reads-contested): no objection against the
            // claim itself, but a dispute holds up every route that would
            // otherwise hold it — so the tally counts it too.
            // A dispute kept open ON PURPOSE (a `contested` Decision) is read
            // under `contested_by_design`, never as still contested.
            let standing: Vec<String> = if standing.is_empty() && c.truth == ClaimTruth::Unknown {
                let mut by = Vec::new();
                for id in &c.contested_by {
                    if !self.is_contested_decision(id)? {
                        by.push(id.clone());
                    }
                }
                by
            } else {
                standing
            };
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
                    held_reading: self.held_reading(&c.id)?,
                });
                continue;
            }
            // A DISCHARGED ASSUMPTION IS NOT UNSUPPORTED (req:a-correct-proof-
            // reads-correct-assumptions-objections-and-conditional-claims,
            // part 1): it is a supposition taken up for the sake of argument
            // and closed, read under `assumptions` as "assumed, discharged by
            // …". Measured: π+e's supposition and all 8 of Arrow's read here.
            // One that is also an ordinary premise somewhere is relied on, and
            // stays.
            if discharged_only.contains(&c.id) {
                continue;
            }
            // A requirement whose reason to exist stands is not an
            // unsupported claim: its read-back names that reason and the
            // evidence beneath it.
            let has_a_reason = c
                .reason_to_exist
                .as_ref()
                .is_some_and(|r| r.reads == "stands");
            // A STATED FOUNDATION IS NOT UNSUPPORTED (item 1 of req:small-read-
            // back-faults-from-the-cogito-trial-are-fixed): taken as given, it
            // is listed under `stated_foundations`; if it reads unknown, an
            // objection is what holds it there. The cogito trial listed such a
            // claim in both, as "no step concludes it".
            if c.truth == ClaimTruth::Unknown
                && c.support.is_empty()
                && c.sources.is_empty()
                && c.node_type != node::ARTIFACT
                && !has_a_reason
                && !c.taken_as_given
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

        // A QUESTION READ BACK TO ITS POSITIONS: asked on a Decision that
        // governs nodes, each position with what it rests on.
        let mut positions = Vec::new();
        if let Some(id) = claim
            && self.get_node(node::DECISION, id)?.is_some()
        {
            let held = self.held_reading(id)?;
            for side in self.governed_by_decision(id)? {
                let c = match claims.iter().find(|c| c.id == side) {
                    Some(c) => c.clone(),
                    None => self.claim_reading(&side)?,
                };
                positions.push(Position {
                    as_reading: held
                        .as_ref()
                        .map(|h| if *h == side { "held" } else { "rival" }),
                    rests_on: self
                        .foundations_of(&side)?
                        .into_iter()
                        .map(|f| f.id)
                        .collect(),
                    id: side,
                    name: c.name,
                    truth: c.truth,
                    proof_status: c.proof_status,
                    assumes: c.assumes,
                    holds_under_any_of: c.holds_under_any_of,
                    contested_by: c.contested_by,
                });
            }
        }
        let note = roots.is_empty().then(|| NOTHING_TO_READ.to_string());
        let not_walked_note = (!not_walked.is_empty()).then(|| {
            format!(
                "NOT WALKED: the walk stopped at {walk_cap} records, so these {} were reached \
                 and not read. What rests under them is in none of `rests_on`, \
                 `stated_foundations` and `uncited_premises`, and those lists say nothing about \
                 it either way: not walked is not resting on nothing. Read one with \
                 reasoning_report on its id as `claim`.",
                not_walked.len()
            )
        });

        let evidence: Vec<EvidenceItem> = claims
            .iter()
            .filter(|c| {
                c.grade.is_some()
                    || c.statement
                        .as_ref()
                        .is_some_and(|s| matches!(s.kind.as_str(), "evidence" | "judgement"))
            })
            .map(|c| EvidenceItem {
                id: c.id.clone(),
                name: c.name.clone(),
                kind: c.statement.as_ref().map(|s| s.kind.clone()),
                reads: c.statement.as_ref().map(|s| s.reads),
                grade: c.grade.clone(),
            })
            .collect();

        let mut grouped: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for c in &claims {
            if let Some(f) = &c.same_formula_as {
                grouped.entry(f.clone()).or_default().insert(c.id.clone());
            }
        }
        let formulas: Vec<FormulaOccurrences> = grouped
            .into_iter()
            .map(|(formula, rest)| {
                let mut occurrences = Vec::new();
                if claims.iter().any(|c| c.id == formula) {
                    occurrences.push(formula.clone());
                }
                occurrences.extend(rest.into_iter().filter(|o| *o != formula));
                FormulaOccurrences {
                    formula,
                    occurrences,
                }
            })
            .collect();

        // THE PAGE: whole claims from `offset`, at most `limit` of them.
        let total_claims = claims.len();
        let start = offset.unwrap_or(0).min(total_claims);
        let end = start.saturating_add(limit).min(total_claims);
        let walked = claims.len();
        let claims: Vec<ClaimReading> = claims.into_iter().skip(start).take(end - start).collect();
        Ok(ReasoningReport {
            claim: claim.map(str::to_string),
            roots,
            walked,
            claims,
            offset: start,
            total_claims,
            remaining: total_claims - end,
            next_offset: (end < total_claims).then_some(end),
            not_walked,
            not_walked_note,
            rests_on: rests_on.into_iter().collect(),
            against_rests_on,
            uncited_premises,
            stated_foundations,
            still_contested,
            steps_objected,
            contested_by_design,
            positions,
            unsupported,
            assumptions,
            independence,
            generalizations_not_holding,
            evidence,
            formulas,
            capped,
            note,
            what_this_means: WHAT_THIS_MEANS,
        })
    }

    /// The roots of the whole-design walk, sorted: every top conclusion
    /// (concluded by a step, a premise of none) and every `contested`
    /// Decision; then every claim of the reasoning the walk from those does
    /// not reach, each a root of its own (see [`Self::reasoning_claims`]).
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
        let mut reached: BTreeSet<String> = BTreeSet::new();
        for r in &roots {
            self.reasoning_reach(r, &mut reached)?;
        }
        for c in self.reasoning_claims()? {
            if !reached.contains(&c) {
                self.reasoning_reach(&c, &mut reached)?;
                roots.insert(c);
            }
        }
        Ok(roots.into_iter().collect())
    }

    /// EVERY CLAIM OF THE REASONING, the set a whole-design read must cover:
    /// every node a step concludes, every statement (`statement_kind`, on
    /// whichever types the schema gives it), and both ends of every opposing
    /// CONTRADICTS. In the order they are taken as roots: an objection's
    /// target that objects to nothing first, then any other target, then the
    /// rest, each group by id, so a refuted claim is read from the claim and
    /// not from what refutes it.
    fn reasoning_claims(&self) -> Result<Vec<String>, DynoError> {
        let mut targets: BTreeSet<String> = BTreeSet::new();
        let mut objectors: BTreeSet<String> = BTreeSet::new();
        let index = self.node_type_index()?;
        for e in self.all_edges_of_type(edge::CONTRADICTS, &index)? {
            if !is_supporting(&e.properties) {
                objectors.insert(e.from_id);
                // A step is not a claim: an objection aimed at one leads to
                // the claim the step concludes.
                targets.extend(self.claim_an_objection_reaches(&e.to_id)?);
            }
        }
        let mut rest: BTreeSet<String> = objectors.clone();
        for i in self.scan_nodes(node::INFERENCE)? {
            if let Some(c) = self.conclusion_of(&i.node_id)? {
                rest.insert(c);
            }
        }
        let mut stated_on: Vec<&String> = self
            .schema()
            .node_types
            .iter()
            .filter(|(_, def)| def.properties.contains_key("statement_kind"))
            .map(|(t, _)| t)
            .collect();
        stated_on.sort();
        for t in stated_on {
            for n in self.scan_nodes(t)? {
                if n.properties.contains_key("statement_kind") {
                    rest.insert(n.node_id);
                }
            }
        }
        let (first, then): (Vec<String>, Vec<String>) = targets
            .iter()
            .cloned()
            .partition(|t| !objectors.contains(t));
        let mut order = first;
        order.extend(then);
        order.extend(rest.into_iter().filter(|c| !targets.contains(c)));
        Ok(order)
    }

    /// Everything the whole-design walk reaches from `from`, added to
    /// `reached`. It reads no record, so it costs only the links.
    fn reasoning_reach(&self, from: &str, reached: &mut BTreeSet<String>) -> Result<(), DynoError> {
        if !reached.insert(from.to_string()) {
            return Ok(());
        }
        let mut queue = VecDeque::from([from.to_string()]);
        while let Some(id) = queue.pop_front() {
            for next in self.reasoning_neighbours(&id, true)? {
                if reached.insert(next.clone()) {
                    queue.push_back(next);
                }
            }
        }
        Ok(())
    }

    /// What the walk follows from one node, in id order. `both_ways` — the
    /// whole-design read — also follows an objection to the claim it is
    /// against.
    fn reasoning_neighbours(&self, id: &str, both_ways: bool) -> Result<Vec<String>, DynoError> {
        let mut next: BTreeSet<String> = BTreeSet::new();
        // A STEP USED AS A PREMISE is walked into: its own premises and what
        // objects to it (req:what-a-claim-rests-on-excludes-the-premises-of-
        // objections-against-it).
        if self.get_node(node::INFERENCE, id)?.is_some() {
            next.extend(self.premises_of(id)?);
            for e in self.incoming(id, Some(edge::CONTRADICTS))? {
                if !is_supporting(&e.properties) {
                    next.insert(e.from_id);
                }
            }
        }
        // A REQUIREMENT'S REASON TO EXIST is read with it: each Decision it
        // is GOVERNED_BY that a chain of steps concludes, and on down that
        // chain to the evidence (req:a-requirements-read-back-follows-its-
        // reason-to-the-evidence; the capstone trial: each requirement walked
        // 1 claim, rests_on []).
        if self.get_node(node::REQUIREMENT, id)?.is_some() {
            for e in self.outgoing(id, Some(edge::GOVERNED_BY))? {
                if e.properties.get("ruling").and_then(Value::as_str) != Some("parks")
                    && self.get_node(node::DECISION, &e.to_id)?.is_some()
                    && !self.inferences_concluding(&e.to_id)?.is_empty()
                {
                    next.insert(e.to_id);
                }
            }
        }
        for s in self.inferences_concluding(id)? {
            next.extend(self.premises_of(&s)?);
            // A DISPUTE BETWEEN RIVAL READINGS of this claim's steps, kept
            // open on purpose, is read with the claim.
            if let Some(d) = self.contested_governor(&s)? {
                next.insert(d);
            }
            // AN OBJECTION AIMED AT A STEP under this claim is read with it
            // (req:an-objection-aimed-at-a-step-is-read-against-that-step):
            // the step is not a claim of its own, so its objectors are walked
            // from the claim it concludes.
            for e in self.incoming(&s, Some(edge::CONTRADICTS))? {
                if !is_supporting(&e.properties) {
                    next.insert(e.from_id);
                }
            }
        }
        for e in self.incoming(id, Some(edge::CONTRADICTS))? {
            if !is_supporting(&e.properties) {
                next.insert(e.from_id);
            }
        }
        if both_ways {
            for e in self.outgoing(id, Some(edge::CONTRADICTS))? {
                if !is_supporting(&e.properties) {
                    next.extend(self.claim_an_objection_reaches(&e.to_id)?);
                }
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

    /// The claim an objection aimed at `target` is read under: the target
    /// itself, or for a step (an Inference) the claim it concludes — a step
    /// is read inside its claim's support, never as a claim of its own.
    fn claim_an_objection_reaches(&self, target: &str) -> Result<Option<String>, DynoError> {
        if self.get_node(node::INFERENCE, target)?.is_some() {
            return self.conclusion_of(target);
        }
        Ok(Some(target.to_string()))
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
        // One evaluation of the claim, read for every part the reading needs.
        let parts = self.claim_parts(id)?;
        let (truth, why) = (parts.truth, parts.why);
        let assumes = parts.assumes;
        let rests_on_unverified = parts.unverified;
        let rests_on_tests = parts.tests;
        let (holds_under_any_of, lines_capped) = (parts.lines, parts.lines_capped);
        let contested_by = parts.contested_by;
        let claim_truth = ClaimTruth::of(truth, &assumes, &rests_on_unverified, &rests_on_tests);
        let why = if claim_truth == ClaimTruth::Conditional && holds_under_any_of.len() > 1 {
            // EVERY LINE IS NAMED (req:a-claim-with-several-lines-reads-each-
            // lines-assumptions): a claim held on several lines holds under any
            // one of them, and is never read as holding under the first alone.
            format!(
                "conditional: it holds on {} lines, under any one of them — {} — and never on its \
                 own; {why}",
                holds_under_any_of.len(),
                crate::inference::lines_said(&holds_under_any_of)
            )
        } else if claim_truth == ClaimTruth::Conditional && !assumes.is_empty() {
            format!(
                "conditional: it holds only under the assumption(s) {}, which no step on the way \
                 to it discharges, never on its own; {why}",
                assumes.join(", ")
            )
        } else if claim_truth == ClaimTruth::Conditional {
            format!(
                "conditional: it holds only if the unverified citation(s) {} say what they are \
                 recalled to say, never plainly; {why}",
                rests_on_unverified.join(", ")
            )
        } else if claim_truth == ClaimTruth::Supported {
            format!(
                "supported at α, never proven: it holds only through the statistical test(s) {}, \
                 which are evidence at their significance level; {why}",
                rests_on_tests.join(", ")
            )
        } else {
            why
        };
        let concluded_decision =
            node_type == node::DECISION && !self.inferences_concluding(id)?.is_empty();
        let (acceptance, chain) = if concluded_decision {
            (
                prop(&props, "status").or_else(|| Some("proposed".to_string())),
                self.chain_reading(id)?
                    .map(|(verdict, why)| ChainVerdict { verdict, why }),
            )
        } else {
            (None, None)
        };
        let mut support = Vec::new();
        let mut steps_dispute: Option<String> = None;
        for s in self.inferences_concluding(id)? {
            if let Some(reading) = self.inference_reading(&s)? {
                let warrant = self
                    .get_node(node::INFERENCE, &s)?
                    .and_then(|n| prop(&n.properties, "warrant"));
                let dispute = self.contested_governor(&s)?;
                let as_reading = match &dispute {
                    Some(d) => self.reading_role(&s, d)?,
                    None => None,
                };
                if steps_dispute.is_none() {
                    steps_dispute = dispute.clone();
                }
                support.push(SupportStep {
                    reading,
                    warrant,
                    dispute,
                    as_reading,
                });
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
        let says = ["statement", "chose", "decision", "description", "summary"]
            .iter()
            .find_map(|k| prop(&props, k).filter(|v| !v.trim().is_empty()));
        Ok(ClaimReading {
            id: id.to_string(),
            name: prop(&props, "name"),
            says,
            proof_status: self.proof_reading(id)?.map(|p| p.status),
            provenance: prop(&props, "provenance"),
            objections: self.objections_to(id)?,
            sources: self.cited_sources(id)?,
            taken_as_given: crate::inference::is_taken_as_given(&props),
            grade: crate::inference::grade_of(&props),
            silent_on: crate::inference::list_prop(&props, "silent_on"),
            same_formula_as: prop(&props, "same_formula_as"),
            statement: self.statement_reading(id)?,
            assumes,
            holds_under_any_of,
            lines_capped,
            rests_on_unverified,
            rests_on_tests,
            contested_by_design: self.contested_governor(id)?.or(steps_dispute),
            contested_by,
            check_conditions: self.check_conditions(id)?,
            reason_to_exist: self.reason_to_exist(id)?,
            argues_for: Vec::new(),
            acceptance,
            chain,
            node_type,
            truth: claim_truth,
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
    pub(crate) fn cited_sources(&self, id: &str) -> Result<Vec<CitedSource>, DynoError> {
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
        let mut unverified: Vec<String> = Vec::new();
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
            if crate::inference::is_cited_result(&props) || crate::inference::is_evidence(&props) {
                given.push(p.clone());
                if self.cited_sources(p)?.iter().all(|source| !source.pinned) {
                    unverified.push(p.clone());
                }
            } else if crate::inference::is_taken_as_given(&props) {
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
        let (unverified_citations, unverified_ids) = cap(unverified);
        let mut unexhausted: Vec<String> = Vec::new();
        let mut inconsistent: Vec<String> = Vec::new();
        for s in &steps {
            let cases = crate::inference::list_prop(&s.properties, "cases");
            let declared = s
                .properties
                .get("cases_exhaustive")
                .and_then(Value::as_str)
                .is_some_and(|v| !v.trim().is_empty());
            if !cases.is_empty() && !declared {
                unexhausted.push(s.node_id.clone());
            }
        }
        for c in &conclusions {
            let is_falsum = self
                .node_props(c)?
                .is_some_and(|p| crate::inference::is_falsum(&p));
            if is_falsum
                && self
                    .statement_reading(c)?
                    .is_some_and(|r| r.reads == "inconsistent")
            {
                inconsistent.push(c.clone());
            }
        }
        let (cases_not_declared_exhaustive, unexhausted_ids) = cap(unexhausted);
        let (inconsistencies, inconsistency_ids) = cap(inconsistent);
        // WHAT HAS LOST ITS REASON NOW: the state, not a delta.
        let mut reasons: Vec<String> = Vec::new();
        for r in self.scan_live_nodes(node::REQUIREMENT)? {
            if self
                .outgoing(&r.node_id, Some(edge::GOVERNED_BY))?
                .is_empty()
            {
                continue;
            }
            if self
                .reason_to_exist(&r.node_id)?
                .is_some_and(|x| matches!(x.reads, "lost" | "stands_on_acceptance_alone"))
            {
                reasons.push(r.node_id);
            }
        }
        reasons.sort();
        let mut chainless: Vec<String> = Vec::new();
        for d in self.scan_live_nodes(node::DECISION)? {
            if d.properties.get("status").and_then(Value::as_str) != Some("accepted") {
                continue;
            }
            if self
                .chain_reading(&d.node_id)?
                .is_some_and(|(verdict, _)| verdict != Truth::Holds)
            {
                chainless.push(d.node_id);
            }
        }
        chainless.sort();
        let mut stale: Vec<String> = Vec::new();
        for v in self.scan_live_nodes(node::VERIFICATION)? {
            if self.check_conditions(&v.node_id)?.is_some_and(|c| c.stale) {
                stale.push(v.node_id);
            }
        }
        stale.sort();
        let (reasons_lost, reason_lost_ids) = cap(reasons);
        let (decisions_without_a_holding_chain, chainless_ids) = cap(chainless);
        let (stale_checks, stale_ids) = cap(stale);
        Ok(Some(ReasoningDebt {
            reasons_lost,
            reason_lost_ids,
            decisions_without_a_holding_chain,
            decision_without_a_holding_chain_ids: chainless_ids,
            stale_checks,
            stale_check_ids: stale_ids,
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
            unverified_citations,
            unverified_citation_ids: unverified_ids,
            cases_not_declared_exhaustive,
            cases_not_declared_exhaustive_ids: unexhausted_ids,
            inconsistencies,
            inconsistency_ids,
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

    /// Everything `claim`'s own support reaches: the claim, the premises of
    /// every step concluding it, and on down through each premise's steps
    /// (a step used as a premise included) — never through an objection.
    fn support_closure(&self, claim: &str) -> Result<BTreeSet<String>, DynoError> {
        let mut seen: BTreeSet<String> = BTreeSet::from([claim.to_string()]);
        let mut queue: VecDeque<String> = VecDeque::from([claim.to_string()]);
        while let Some(x) = queue.pop_front() {
            let mut steps = self.inferences_concluding(&x)?;
            if self.get_node(node::INFERENCE, &x)?.is_some() {
                steps.push(x.clone());
            }
            // A requirement's support includes its reason to exist: the
            // chain of each Decision it is GOVERNED_BY.
            if self.get_node(node::REQUIREMENT, &x)?.is_some() {
                for e in self.outgoing(&x, Some(edge::GOVERNED_BY))? {
                    if e.properties.get("ruling").and_then(Value::as_str) != Some("parks")
                        && self.get_node(node::DECISION, &e.to_id)?.is_some()
                        && !self.inferences_concluding(&e.to_id)?.is_empty()
                        && seen.insert(e.to_id.clone())
                    {
                        queue.push_back(e.to_id);
                    }
                }
            }
            for st in steps {
                for p in self.premises_of(&st)? {
                    if seen.insert(p.clone()) {
                        queue.push_back(p);
                    }
                }
            }
        }
        Ok(seen)
    }

    /// How `decision` holds `side`: `held` when the GOVERNED_BY from the side
    /// says `ruling: held`, `rival` when the Decision holds another side,
    /// `None` when it holds none.
    fn reading_role(&self, side: &str, decision: &str) -> Result<Option<&'static str>, DynoError> {
        let Some(held) = self.held_reading(decision)? else {
            return Ok(None);
        };
        Ok(Some(if held == side { "held" } else { "rival" }))
    }

    /// The side a Decision holds as its reading: the first, in id order, whose
    /// GOVERNED_BY to it says `ruling: held`.
    fn held_reading(&self, decision: &str) -> Result<Option<String>, DynoError> {
        let mut held: Vec<String> = self
            .incoming(decision, Some(edge::GOVERNED_BY))?
            .into_iter()
            .filter(|e| e.properties.get("ruling").and_then(Value::as_str) == Some("held"))
            .map(|e| e.from_id)
            .collect();
        held.sort();
        Ok(held.into_iter().next())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::InferenceSpec;

    /// One claim concluded by one step from `n` premises.
    fn fan(n: usize) -> DesignGraph {
        let mut g = DesignGraph::open_in_memory().expect("in-memory graph");
        g.add_requirement("req:top", "The claim", "What the step concludes.")
            .unwrap();
        let mut premises = Vec::new();
        for i in 0..n {
            let id = format!("req:p{i:02}");
            g.add_requirement(&id, &format!("Premise {i}"), "A premise, uncited.")
                .unwrap();
            premises.push(id);
        }
        g.add_inference(&InferenceSpec {
            id: "inf:top".into(),
            rule: "all".into(),
            premises,
            conclusion: "req:top".into(),
            ..Default::default()
        })
        .unwrap();
        g
    }

    /// A page's `limit` never caps the walk: the summary lists are the walk's,
    /// so a one-claim page still says what the claim rests on.
    #[test]
    fn a_small_page_keeps_the_whole_walks_summary() {
        let g = fan(5);
        let r = g
            .reasoning_report_from(Some("req:top"), Some(1), None)
            .unwrap();
        assert_eq!(r.claims.len(), 1);
        assert_eq!(r.total_claims, 6);
        assert_eq!(r.next_offset, Some(1));
        assert_eq!(r.remaining, 5);
        assert_eq!(r.rests_on.len(), 5, "{:?}", r.rests_on);
        assert_eq!(r.uncited_premises.len(), 5, "{:?}", r.uncited_premises);
        assert!(!r.capped && r.not_walked.is_empty());
        // Read on: the last page says so.
        let last = g
            .reasoning_report_from(Some("req:top"), Some(10), Some(1))
            .unwrap();
        assert_eq!(last.claims.len(), 5);
        assert_eq!(last.next_offset, None);
        assert_eq!(last.remaining, 0);
    }

    /// When the WALK stops at its cap, what it reached and did not read is
    /// named as not walked, and never read as resting on nothing.
    #[test]
    fn a_capped_walk_names_what_it_did_not_walk() {
        let g = fan(5);
        let r = g
            .reasoning_report_walking(Some("req:top"), None, None, 3)
            .unwrap();
        assert!(r.capped);
        assert_eq!(r.walked, 3);
        assert_eq!(r.not_walked, vec!["req:p02", "req:p03", "req:p04"]);
        assert!(
            r.not_walked_note
                .as_deref()
                .is_some_and(|n| n.contains("not walked is not resting on nothing")),
            "{:?}",
            r.not_walked_note
        );
        // The two premises it read are uncited; the three it did not read are
        // not claimed either way.
        assert_eq!(r.uncited_premises, vec!["req:p00", "req:p01"]);
    }

    /// Each premise of a returned step carries its name, and each claim its
    /// own words.
    #[test]
    fn a_claim_carries_its_words_and_its_premises_names() {
        let g = fan(2);
        let r = g.reasoning_report(Some("req:top"), None).unwrap();
        let top = &r.claims[0];
        assert_eq!(top.says.as_deref(), Some("What the step concludes."));
        let names: Vec<_> = top.support[0]
            .reading
            .premises
            .iter()
            .map(|p| p.name.clone())
            .collect();
        assert_eq!(
            names,
            vec![Some("Premise 0".to_string()), Some("Premise 1".to_string())]
        );
    }
}
