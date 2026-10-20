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
//! - a **Verification** holds when `passing` and fails when `failing` — and a
//!   check made by COMPUTATION holds only through its kept record
//!   ([`crate::kept_record`]): with no record it reads `asserted`, so it is
//!   unknown (`req:a-computed-fact-counts-only-through-its-kept-record`);
//! - a **Decision** holds when `accepted`, and fails when `rejected` or
//!   `superseded`. An ACCEPTED Decision a step concludes holds on its owner's
//!   acceptance, and the chain's own verdict is read beside it
//!   ([`DesignGraph::chain_reading`]), so a first, incomplete chain never reads
//!   as the decision being unsupported, and a lost chain still shows in the
//!   change walk (`accepted_without_a_holding_chain`);
//! - a **TemporalFact** (a dated finding) holds when it is MEASURED and still
//!   STANDING: nothing INVALIDATES it, it states no end (`valid_to`, or a
//!   `VALID_TO` epoch), and no accepted decision discontinues it. A `forecast`
//!   projects what is expected and is evidence of nothing that happened, so it
//!   is unknown; so is a finding that ended or was invalidated (stale, which is
//!   unsupported, never refuted). `basis` unstated is `measured`, the schema's
//!   default for every fact written before the property existed. A fact's
//!   `confidence` is the author's weight and changes nothing here;
//! - a **ChangeEvent** is HISTORY: it records that something happened, so
//!   reflow2 computes no truth for it, and an objection to it never makes it
//!   read false. A counter-argument it once carried is recorded as a statement
//!   of its own that opposes the claim, and answered there
//!   (`req:the-records-a-choice-rests-on-carry-a-truth-and-a-counter-argument-is-its-own-statement`);
//! - a **Constraint** fails when something VIOLATES it or its budget is
//!   exceeded, is unknown while its budget cannot be judged, and holds
//!   otherwise;
//! - a **Capability** holds when it is realized and checked (the leaf of the
//!   delivery roll-up);
//! - anything discontinued by an accepted decision fails;
//! - a node concluded by an Inference HOLDS when one of its steps holds (two
//!   steps concluding one node are alternative derivations), and is otherwise
//!   UNSUPPORTED — unknown, never `fails`. Losing its support does not make a
//!   claim false; only an objection that holds refutes it
//!   (`req:unsupported-is-not-refuted`, Anthony 2026-10-17);
//! - an Inference used as a premise is its own verdict;
//! - a **reference to a node in another design** (`xref:<design>:<node>`)
//!   reads as that node reads THERE — its truth, carried with what it holds
//!   only under, and its proof status limiting a proof here — when the server
//!   read it for this call ([`crate::far`]); a stale or unreachable far end is
//!   unknown, never holding
//!   (`req:reasoning-crosses-designs-within-a-hub`);
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
//! radius) reports each step downstream of a lost premise as UNSUPPORTED or
//! WEAKENED, and each claim it reaches as still supported (by which steps),
//! unsupported, or refuted; and `detect_gaps` reports a conclusion whose
//! claimed status is past what its steps support (`conclusion_does_not_follow`). A design that records
//! no Inference reads exactly as it did: every one of those paths asks one
//! adjacency question first and finds nothing.
//!
//! # Who confirmed the step (requirement B)
//!
//! `req:a-design-can-be-a-proof` (Anthony, 2026-10-14): each step records
//! `confirmed_by` (who), `confirmed_how` (`person`, `checker`, or
//! `unconfirmed`) and `confirmed_at` (when). A conclusion reads as **proven**
//! only when every step in at least one holding derivation chain is confirmed;
//! otherwise it reads as **shown** (holds but unconfirmed) or **unproven** (no
//! derivation holds). Served as `proof_status` on `get_node` for a concluded
//! node.
//!
//! # An objection counts as far as it holds (requirement A)
//!
//! `req:a-design-holds-a-line-of-reasoning` (Anthony, 2026-10-16: "contest
//! until established"). An opposing CONTRADICTS — a counterexample, or an
//! objection in an argument for and against — counts against its target by
//! the objection's OWN computed truth:
//!
//! - one that **holds** refutes it: the target `fails` — it is REFUTED, the
//!   one way a claim reads false — and every step resting on it is reported
//!   broken (a checked counterexample); what those steps conclude is left
//!   unsupported, not refuted;
//! - one that is **unknown** — raised, neither established nor answered —
//!   leaves the target at most `unknown`: contested, never refuted by being
//!   said;
//! - one that **fails** — answered by a reply that holds, or rejected — no
//!   longer counts.
//!
//! So a reply is recorded the way the objection was: a CONTRADICTS from the
//! reply to the objection. Two claims that object to each other, with nothing
//! else to settle it, both read `unknown`: a circle of objections has no
//! verdict. A dispute its owner keeps open on purpose is a Decision set to
//! `contested` that governs both sides; its own truth is `unknown`.
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
    /// Its name, when the reader filled it in: `reasoning_report` does, so a
    /// step reads with the words of what it rests on. Absent elsewhere.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `assumed` when the step takes this premise up as a HYPOTHESIS
    /// (PREMISE_OF `role: assumed`): read as holding inside the argument,
    /// whatever its own truth. Absent for an ordinary premise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<&'static str>,
}

/// Who confirmed a step: a person, a checker, or nobody.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Confirmation {
    /// `unconfirmed`, `person` or `checker`.
    pub confirmed_how: String,
    /// Who: a contributor id (`who:ajs`) or a checker's name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed_by: Option<String>,
    /// When, as an ISO 8601 timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed_at: Option<String>,
}

impl Default for Confirmation {
    fn default() -> Self {
        Confirmation {
            confirmed_how: "unconfirmed".to_string(),
            confirmed_by: None,
            confirmed_at: None,
        }
    }
}

impl Confirmation {
    pub fn is_confirmed(&self) -> bool {
        self.confirmed_how == "person" || self.confirmed_how == "checker"
    }
}

/// Whether a conclusion reads as proven, shown, or unproven.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofStatus {
    /// Every step in at least one holding derivation chain is confirmed.
    Proven,
    /// At least one derivation holds, but one or more steps are unconfirmed.
    Shown,
    /// A derivation holds, but only UNDER AN ASSUMPTION it does not
    /// discharge (an `assumed` premise under it that no step on the way
    /// discharges): a result inside a hypothetical argument, never a proof of
    /// the claim on its own.
    Conditional,
    /// A derivation holds, but an objection stands against the conclusion:
    /// neither established nor answered (requirement A).
    Contested,
    /// No derivation holds (the conclusion does not follow), or an objection
    /// that holds refutes it.
    Unproven,
}

impl ProofStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ProofStatus::Proven => "proven",
            ProofStatus::Shown => "shown",
            ProofStatus::Conditional => "conditional",
            ProofStatus::Contested => "contested",
            ProofStatus::Unproven => "unproven",
        }
    }
}

/// A conclusion's proof status: proven, shown, or unproven, with why.
///
/// THE COUNTS ARE OF ONE SET: the distinct steps (Inferences) under the claim,
/// reached through every one of its HOLDING lines (a concluding step whose
/// rule holds over its premises) and on down through each premise's own
/// concluding steps. A step two lines share is counted ONCE, so a claim never
/// counts more steps than the design holds, and a theorem resting on a claim
/// counts at least that claim's steps.
/// `fact:root-cause-a-claims-step-count-sums-its-lines-so-a-shared-step-counts-twice-2026-10-18`:
/// until 2026-10-18 the counts were the SUM of the lines, and a step two lines
/// shared counted twice. `proven` is still judged line by line: a line is
/// proven only when every one of ITS steps, shared ones included, is confirmed.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProofReading {
    pub status: ProofStatus,
    pub why: String,
    /// The distinct steps under the claim's holding lines, each once.
    pub total_steps: usize,
    /// Of those, the steps a person or a checker confirmed.
    pub confirmed_steps: usize,
    /// Of those, the steps nobody confirmed: `total_steps - confirmed_steps`.
    pub unconfirmed_steps: usize,
    /// What the three counts mean, carried beside them on every reading
    /// ([`PROOF_COUNTS_MEAN`]), so the number is never read without it.
    pub counts: &'static str,
}

/// What a [`ProofReading`]'s counts mean: served on every `proof_status`.
pub const PROOF_COUNTS_MEAN: &str = "total_steps is the distinct steps under this claim's \
     holding lines, each counted once however many lines share it; confirmed_steps and \
     unconfirmed_steps split those. The status is judged line by line: proven when every step \
     of one holding line, shared ones included, is confirmed.";

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
    /// Who confirmed this step, how, and when.
    pub confirmation: Confirmation,
    /// THE ASSUMPTIONS THIS STEP'S VERDICT STILL RESTS ON: every `assumed`
    /// premise under it, at any depth, that neither it nor a step between
    /// discharges. Empty for a step that rests on no assumption, or whose
    /// every assumption is discharged.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assumes: Vec<String>,
    /// THE UNVERIFIED CITATIONS this step's verdict rests on, at any depth:
    /// results cited from elsewhere that no pinned source documents
    /// (req:a-proof-can-state-a-contradiction-cases-suppositions-and-cited-
    /// theorems, item 4). Empty when every citation beneath it is verified.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rests_on_unverified: Vec<String>,
    /// The assumptions this step DISCHARGES, as it records them
    /// (`discharges`): a reductio's supposition, an induction hypothesis.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub discharges: Vec<String>,
    /// The case split this step makes (`cases`, `cases_exhaustive`), read:
    /// which cases, and whether they are DECLARED EXHAUSTIVE and by what.
    /// Absent when it splits on no cases.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cases: Option<CasesReading>,
    /// The variables it generalizes over, and whether anything it rests on
    /// fixes one (`generalizes_over`, checked). Absent when it declares none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generalization: Option<GeneralizationReading>,
    /// Each line it is STATED independent of (`independent_of`), checked.
    /// Read only on the step itself (get_node, add_inference), never inside
    /// another step's reading.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub independence: Vec<IndependenceReading>,
}

/// A step's case split, read (`cases`, `cases_exhaustive`): a proof by cases
/// holds only when its cases are DECLARED exhaustive, and the declaration is a
/// record's word — the owner's (taken as given) or a checker's (a passing
/// computed check) — never reflow2's.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CasesReading {
    /// The cases' hypotheses, as the step names them. Each is discharged by
    /// the step.
    pub cases: Vec<String>,
    /// Whether a record is named as saying the cases are exhaustive.
    pub declared: bool,
    /// That record, when one is named.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exhaustive_by: Option<String>,
    /// Its computed truth, when one is named: the split holds only while it
    /// holds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truth: Option<Truth>,
    /// Why, in a sentence.
    pub why: String,
}

/// A step's generalization over its declared variables, checked
/// (req:a-proof-can-discharge-an-assumption-and-generalize-over-a-variable).
///
/// # What "fixes" means
///
/// A record FIXES a variable when its `fixes` lists it — as `name=value`
/// (`n=40`, `L=all the primes`) or a bare name: the record is about that one
/// value, not an arbitrary one. A step that generalizes over `v` HOLDS ONLY
/// WHEN NOTHING IT RESTS ON, AT ANY DEPTH, FIXES `v` (assumed premises
/// included).
///
/// # The eigenvariable rule (fixed 2026-10-18, the Arrow trial's N19)
///
/// AND ONLY WHEN NO ASSUMPTION STILL OPEN AT THE STEP NAMES `v`. Natural
/// deduction allows ∀-introduction over x only when x is free in no
/// assumption the conclusion still depends on; a step that generalized over
/// F under an open hypothesis about F read `holds` until then
/// (`fact:root-cause-the-generalization-check-read-only-fixes-and-never-the-assumptions-open-at-the-step-2026-10-18`).
/// An assumption is OPEN at the step when it is assumed beneath it and
/// neither the step nor a step on the way discharges it — the step's own
/// discharges close first, so a conditional proof that discharges a
/// hypothesis on F and generalizes over F in one step stays valid. It NAMES
/// `v` when its `variables` or its `fixes` lists `v`; one that declares no
/// `variables` at all leaves the question unanswerable, so the step does not
/// hold until it does (an empty list declares that it names none).
///
/// `variables` is otherwise read only to show which records the variable runs
/// through. Structural, not a logic: the same name is the same variable, and
/// what a statement says is never read.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GeneralizationReading {
    /// The variables the step generalizes over.
    pub over: Vec<String>,
    /// Whether the check passes: nothing in the chain fixes any of them, and
    /// no assumption open at the step names one or leaves it unanswerable.
    pub holds: bool,
    /// Every record in the chain that fixes one, with what it fixes.
    pub fixed_by: Vec<FixedBy>,
    /// Every assumption OPEN at the step that names one of them, with the
    /// ones it names (the eigenvariable rule).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub named_by_open_assumptions: Vec<FixedBy>,
    /// Assumptions open at the step that declare no `variables`, so whether
    /// they name one cannot be told: the step does not hold until they do.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub undeclared_open_assumptions: Vec<String>,
    /// Records in the chain that mention one of them (`variables`).
    pub mentioned_by: Vec<String>,
}

/// One record that fixes, or (open) names, a variable a step generalizes over.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FixedBy {
    /// The record.
    pub id: String,
    /// What it fixes, as it says (`n=40`); for an open assumption, the
    /// variables of the generalization it names.
    pub fixes: Vec<String>,
}

/// A stated independence between two lines of reasoning, checked: the lines
/// share no step other than those under the lemmas the statement names.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndependenceReading {
    /// The step stating it.
    pub step: String,
    /// The other line, as the statement names it.
    pub of: String,
    /// Whether the structure bears the statement out.
    pub holds: bool,
    /// Steps both lines rest on that no named lemma accounts for: what makes
    /// the statement false. Empty when it holds.
    pub shared_steps: Vec<String>,
    /// The named lemmas both lines do rest on.
    pub shares_named: Vec<String>,
    /// The lemmas the statement names (`independent_except`).
    pub except: Vec<String>,
    /// Why, in a sentence.
    pub why: String,
}

/// One assumption of the design: where it is assumed and what discharges it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AssumptionReading {
    /// The assumed node.
    pub id: String,
    /// Its name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The steps that take it up as an `assumed` premise.
    pub assumed_in: Vec<String>,
    /// The steps that discharge it, resting on one of those. Empty means it is
    /// NEVER DISCHARGED: whatever rests on it holds only under it.
    pub discharged_by: Vec<String>,
}

/// What a statement says it is (`statement_kind`) and how it reads now.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StatementReading {
    /// `theorem`, `lemma`, `corollary`, `proposition`, `definition`, `axiom`,
    /// `conjecture`, `hypothesis` or `falsum`.
    pub kind: String,
    /// How it reads: `taken_as_given` (a definition or axiom), `cited` (a
    /// theorem, lemma or corollary taken as given: a result from
    /// elsewhere, checked against a pinned source), `cited_unverified` (one no
    /// pinned source documents), `stated` (a result no step concludes yet, or
    /// a falsum nothing derives), `open` (a conjecture nothing shows or
    /// refutes), `refuted` (an objection that holds refutes it), `assumed` /
    /// `discharged` / `not_assumed` (a hypothesis), `false` / `inconsistent`
    /// (a falsum derived under an assumption, or under none), or the proof
    /// status of a concluded one: `proven`, `shown`, `conditional`,
    /// `contested`, `unproven`.
    pub reads: &'static str,
    /// Why, in a sentence.
    pub why: String,
}

/// What losing a premise does to one step: the change walk's row.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InferenceImpact {
    /// The step reached.
    pub inference: String,
    /// `weakened` (it still holds on its other premises), `unsupported` (it
    /// held, and with the loss its rule no longer holds — a lost premise reads
    /// unsupported, never false), `broken` (its rule FAILS: a premise it rests
    /// on really fails or is refuted, which a loss alone never makes), or
    /// `unknown` (it did not hold before the loss either). A step's word,
    /// never a claim's: what the CLAIM is left with is `conclusion_reads`.
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
    /// conclusion the walk carried on through (one left unsupported).
    pub distance: usize,
    /// The verdict before the loss, as the design stands.
    pub before: Truth,
    /// The verdict with every lost premise counted as no longer holding:
    /// unknown (unsupported), or failing where it already fails.
    pub after: Truth,
    /// Its premises the walk counts as lost: the seeds, and conclusions it
    /// found left unsupported on the way.
    pub lost_premises: Vec<String>,
    /// Whether the CONCLUSION still stands with the losses counted, across
    /// every step that concludes it (alternative derivations are ANY).
    pub conclusion_after: Truth,
    /// WHICH IS TRUE OF THE CLAIM (req:unsupported-is-not-refuted):
    /// `still_supported` (another line holds it: `still_supported_by`),
    /// `unsupported` (nothing left holds it — it is not thereby false),
    /// `refuted` (an objection that holds refutes it), or, for an accepted
    /// Decision, `accepted_without_a_holding_chain` (it stands on its owner's
    /// acceptance, and no line of its chain holds any more).
    pub conclusion_reads: &'static str,
    /// The steps concluding the claim that still hold with the losses counted.
    /// Empty unless `conclusion_reads` is `still_supported`.
    pub still_supported_by: Vec<String>,
    /// Every premise with its truth after the loss.
    pub premises: Vec<PremiseReading>,
}

/// One axiom a conclusion ultimately rests on (a terminal premise — not
/// concluded by any Inference). Requirement E.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Foundation {
    /// The axiom's id.
    pub id: String,
    /// Its type, or `None` when no node holds it.
    pub node_type: Option<String>,
    /// Its computed truth (same as `premise_truth`).
    pub truth: Truth,
    /// Why, in a sentence.
    pub why: String,
}

/// A node's foundations: the axioms it rests on, the honest limit, and whether
/// the list was capped.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FoundationsReading {
    /// The axioms this conclusion rests on.
    pub foundations: Vec<Foundation>,
    /// How many axioms in total (before capping).
    pub total: usize,
    /// Whether the list was capped.
    pub capped: bool,
    /// The honest limit: what the read says it does and does not mean.
    pub honest_limit: String,
    /// THE ASSUMPTIONS IT HOLDS UNDER: `assumed` premises beneath it that no
    /// step on the way discharges. An assumption is never a foundation — it is
    /// taken up for the sake of argument — so it is listed here, apart, and a
    /// discharged one is in neither list.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assumes: Vec<String>,
}

/// How many axioms the foundations list shows before saying "capped".
pub const FOUNDATIONS_CAP: usize = 100;

/// THE HONEST LIMIT. The foundations tell what a conclusion follows from. They
/// never say the axioms are right. In an abstract design they are values, and
/// may be contested by design.
pub const FOUNDATIONS_HONEST_LIMIT: &str = "These foundations show what this conclusion follows from — the terminal \
     premises (axioms) its inference chain ultimately rests on. They never say \
     the axioms are right: in an abstract design they are chosen principles or \
     values, and they may be contested by design.";

/// A node every holding line of a claim rests on: see
/// [`DesignGraph::reasoning_single_points`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReasoningSinglePoint {
    /// The claim with two or more lines.
    pub claim: String,
    /// The node all of them rest on.
    pub node: String,
    /// The steps concluding the claim that hold: its lines.
    pub lines: Vec<String>,
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
/// the steps and nodes on the current path (one met again on its own path is a
/// circle, read as unknown).
struct Eval<'l> {
    lost: &'l BTreeSet<String>,
    memo: HashMap<String, Read>,
    path: BTreeSet<String>,
    /// Nodes whose truth is being computed. Objections may form circles (A
    /// objects to B, B to A), which the write never refuses.
    nodes: BTreeSet<String>,
    /// How many times a circle was cut. A truth computed while a cut happened
    /// beneath it rests on a provisional `unknown`, so it is not memoized:
    /// asked again from elsewhere, it is computed again.
    cuts: usize,
}

impl<'l> Eval<'l> {
    fn new(lost: &'l BTreeSet<String>) -> Self {
        Eval {
            lost,
            memo: HashMap::new(),
            path: BTreeSet::new(),
            nodes: BTreeSet::new(),
            cuts: 0,
        }
    }
}

/// A node's truth as one evaluation reads it: the value, why, and the
/// ASSUMPTIONS it holds under — every `assumed` premise beneath it that no
/// step on the way discharges (empty for almost every node).
#[derive(Debug, Clone)]
struct Read {
    truth: Truth,
    why: String,
    assumes: BTreeSet<String>,
    /// THE UNVERIFIED CITATIONS it rests on: results cited from elsewhere
    /// (taken as given) that no pinned source documents. Carried like an
    /// assumption — what rests on one holds only if it says what it is
    /// recalled to say — but never discharged by a step: checking its source
    /// is what clears it.
    unverified: BTreeSet<String>,
}

/// A reference's reading as a premise here: the far node's truth when it was
/// read, with what it holds only under carried as unverified citations, so a
/// conclusion resting on a conditional far result reads conditional; unknown
/// for a stale or unreachable far end, whatever it once read.
fn far_read(far: &crate::far::FarReading) -> Read {
    use crate::far::FarState;
    match far.state {
        FarState::Read => Read {
            truth: far.truth,
            why: format!(
                "in design {}, where {} lives, it {}: {}{}",
                far.design,
                far.node_id,
                match far.truth {
                    Truth::Holds => "holds",
                    Truth::Fails => "fails",
                    Truth::Unknown => "is unknown",
                },
                far.why,
                far.proof_status
                    .as_deref()
                    .map(|p| format!(" (its proof there: {p})"))
                    .unwrap_or_default()
            ),
            assumes: BTreeSet::new(),
            unverified: far.holds_only_if.iter().cloned().collect(),
        },
        FarState::Stale | FarState::Unreachable => Read::plain(Truth::Unknown, far.why.clone()),
    }
}

impl Read {
    fn plain(truth: Truth, why: String) -> Self {
        Read {
            truth,
            why,
            assumes: BTreeSet::new(),
            unverified: BTreeSet::new(),
        }
    }

    fn pair(self) -> (Truth, String) {
        (self.truth, self.why)
    }
}

/// One objection against a node, as the node's truth reads it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ObjectionReading {
    /// The objecting node: the source of an opposing CONTRADICTS.
    pub id: String,
    /// Its type, or `None` when no node holds the id.
    pub node_type: Option<String>,
    /// Its computed truth.
    pub truth: Truth,
    /// Why, in a sentence.
    pub why: String,
    /// `refutes` (it holds, inside the claim's scope), `stands` (neither
    /// established nor answered, or holding only under a supposition the
    /// claim is outside: `holds_only_under`) or `answered` (it fails, so it
    /// no longer counts).
    pub standing: &'static str,
    /// The edge's own `evidence`, when it carries one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    /// For an objection that holds only under an assumption no step
    /// discharges (or an unverified citation) that the claim it opposes does
    /// not hold under: those, by id. It then `stands` — it contests — and
    /// never refutes (req:a-claim-that-holds-only-under-an-open-supposition-
    /// cannot-refute-a-claim-outside-it). Empty otherwise.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holds_only_under: Vec<String>,
}

fn standing_of(truth: Truth) -> &'static str {
    match truth {
        Truth::Holds => "refutes",
        Truth::Unknown => "stands",
        Truth::Fails => "answered",
    }
}

/// The parts of a step a reading needs, read once.
struct StepRecord {
    rule: String,
    parsed: Result<InferenceRule, String>,
    k: Option<i64>,
    rule_name: Option<String>,
    confirmation: Confirmation,
    discharges: Vec<String>,
    generalizes_over: Vec<String>,
    cases: Vec<String>,
    cases_exhaustive: Option<String>,
}

/// A `list:string` property as a sorted, deduplicated list; empty when absent.
pub(crate) fn list_prop(props: &HashMap<String, Value>, key: &str) -> Vec<String> {
    let mut out: Vec<String> = match props.get(key) {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .collect(),
        Some(Value::String(s)) if !s.trim().is_empty() => vec![s.trim().to_string()],
        _ => Vec::new(),
    };
    out.sort();
    out.dedup();
    out
}

/// The variable a `fixes` entry names: the part before `=`, trimmed.
pub fn fixed_variable(entry: &str) -> &str {
    entry.split_once('=').map_or(entry, |(name, _)| name).trim()
}

/// The statement kinds `statement_kind` may say.
pub const STATEMENT_KINDS: [&str; 9] = [
    "theorem",
    "lemma",
    "corollary",
    "proposition",
    "definition",
    "axiom",
    "conjecture",
    "hypothesis",
    "falsum",
];

/// The kinds that, TAKEN AS GIVEN, are a result CITED FROM ELSEWHERE
/// (req:a-proof-can-state-a-contradiction-cases-suppositions-and-cited-
/// theorems, item 4): a theorem, a lemma, a corollary — results somebody
/// proved. NOT `proposition`: in a line of reasoning a proposition taken as
/// given is a stated position the argument starts from (a premise of an
/// argument for and against, "X satisfies Y" on an argued edge), so it stays
/// taken as given, as a definition or an axiom does.
pub const CITED_KINDS: [&str; 3] = ["theorem", "lemma", "corollary"];

/// A node's `statement_kind`, when it says one.
pub fn statement_kind(props: &HashMap<String, Value>) -> Option<&str> {
    prop(props, "statement_kind")
}

/// Whether a node states a RESULT CITED FROM ELSEWHERE: a theorem, lemma or
/// corollary marked taken as given ([`CITED_KINDS`]). It is never a foundation
/// the way a definition or axiom is: it was proved somewhere else, and it
/// reads `cited` (or `cited_unverified`).
pub fn is_cited_result(props: &HashMap<String, Value>) -> bool {
    statement_kind(props).is_some_and(|k| CITED_KINDS.contains(&k)) && is_taken_as_given(props)
}

/// Whether a node is a FALSUM: a contradiction derived inside a reductio.
pub fn is_falsum(props: &HashMap<String, Value>) -> bool {
    statement_kind(props) == Some("falsum")
}

/// The variables a record NAMES: its `variables` and the variables its
/// `fixes` bind. `None` when it declares no `variables` at all, which leaves
/// open whether it names a given one (ABSENT MEANS NOBODY SAID); an empty
/// list is a declaration that it names none.
pub fn named_variables(props: &HashMap<String, Value>) -> Option<BTreeSet<String>> {
    let declared = matches!(
        props.get("variables"),
        Some(Value::List(_)) | Some(Value::String(_))
    );
    if !declared {
        return None;
    }
    let mut out: BTreeSet<String> = list_prop(props, "variables").into_iter().collect();
    out.extend(
        list_prop(props, "fixes")
            .iter()
            .map(|f| fixed_variable(f).to_string()),
    );
    Some(out)
}

fn prop<'a>(props: &'a HashMap<String, Value>, key: &str) -> Option<&'a str> {
    props.get(key).and_then(Value::as_str)
}

/// Whether a node is marked TAKEN AS GIVEN (`taken_as_given: true`): a
/// definition or axiom the reasoning starts from, read as a stated foundation
/// and never as an uncited premise
/// (req:the-loop-sees-unconfirmed-steps-and-uncited-premises).
///
/// A statement whose `statement_kind` is `definition` or `axiom` is taken as
/// given too, unless it says `taken_as_given: false` in so many words.
pub fn is_taken_as_given(props: &HashMap<String, Value>) -> bool {
    match props.get("taken_as_given").and_then(Value::as_bool) {
        Some(said) => said,
        None => matches!(prop(props, "statement_kind"), Some("definition" | "axiom")),
    }
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

    /// The premises of one step with whether each is ASSUMED (PREMISE_OF
    /// `role: assumed`), sorted by id.
    pub fn premise_links(&self, inference_id: &str) -> Result<Vec<(String, bool)>, DynoError> {
        let mut links: Vec<(String, bool)> = self
            .incoming(inference_id, Some(edge::PREMISE_OF))?
            .into_iter()
            .map(|e| {
                let assumed = e.properties.get("role").and_then(Value::as_str) == Some("assumed");
                (e.from_id, assumed)
            })
            .collect();
        links.sort();
        links.dedup_by(|a, b| a.0 == b.0);
        Ok(links)
    }

    /// The steps `id` is an ORDINARY premise of, sorted — what a change to
    /// `id` reaches. A step that takes `id` up as an assumption is not here:
    /// it holds `id` as a hypothesis whatever `id`'s own truth, so losing it
    /// costs that step nothing.
    fn steps_resting_on(&self, id: &str) -> Result<Vec<String>, DynoError> {
        let mut steps: Vec<String> = self
            .outgoing(id, Some(edge::PREMISE_OF))?
            .into_iter()
            .filter(|e| e.properties.get("role").and_then(Value::as_str) != Some("assumed"))
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
        let confirmation = Confirmation {
            confirmed_how: prop(&n.properties, "confirmed_how")
                .unwrap_or("unconfirmed")
                .to_string(),
            confirmed_by: prop(&n.properties, "confirmed_by").map(str::to_string),
            confirmed_at: prop(&n.properties, "confirmed_at").map(str::to_string),
        };
        Ok(Some(StepRecord {
            rule,
            parsed,
            k,
            rule_name: prop(&n.properties, "rule_name").map(str::to_string),
            confirmation,
            discharges: list_prop(&n.properties, "discharges"),
            generalizes_over: list_prop(&n.properties, "generalizes_over"),
            cases: list_prop(&n.properties, "cases"),
            cases_exhaustive: prop(&n.properties, "cases_exhaustive")
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
        }))
    }

    /// The one type holding `id`, when exactly one does.
    pub(crate) fn type_holding(&self, id: &str) -> Result<Option<String>, DynoError> {
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
        Ok(self.premise_reading(id)?.pair())
    }

    /// [`Self::premise_truth`] with the assumptions it holds under.
    fn premise_reading(&self, id: &str) -> Result<Read, DynoError> {
        let lost = BTreeSet::new();
        let mut ev = Eval::new(&lost);
        self.truth_full(id, &mut ev, INFERENCE_DEPTH)
    }

    /// The ASSUMPTIONS `id` holds under, as the design stands: every `assumed`
    /// premise beneath it that no step on the way discharges, sorted. Empty
    /// for a node that rests on no assumption, or whose every assumption is
    /// discharged on the way to it.
    pub fn assumptions_under(&self, id: &str) -> Result<Vec<String>, DynoError> {
        Ok(self.premise_reading(id)?.assumes.into_iter().collect())
    }

    /// The UNVERIFIED CITATIONS `id` rests on, as the design stands: results
    /// cited from elsewhere, beneath it or itself, that no pinned source
    /// documents, sorted. Empty when it rests on none.
    pub fn unverified_under(&self, id: &str) -> Result<Vec<String>, DynoError> {
        Ok(self.premise_reading(id)?.unverified.into_iter().collect())
    }

    /// What a node counted LOST by a change reads as: UNSUPPORTED (unknown) —
    /// "what if this no longer held?" — never false. A node that already
    /// fails on its own keeps failing, so a what-if never reads better than
    /// the design does (req:unsupported-is-not-refuted).
    fn lost_truth(&self, id: &str) -> Result<(Truth, String), DynoError> {
        let (now, why) = self.premise_truth(id)?;
        if now == Truth::Fails {
            return Ok((now, why));
        }
        Ok((
            Truth::Unknown,
            "unsupported: counted as no longer holding by this change".to_string(),
        ))
    }

    fn truth_of(
        &self,
        id: &str,
        ev: &mut Eval<'_>,
        depth: usize,
    ) -> Result<(Truth, String), DynoError> {
        Ok(self.truth_full(id, ev, depth)?.pair())
    }

    fn truth_full(&self, id: &str, ev: &mut Eval<'_>, depth: usize) -> Result<Read, DynoError> {
        if ev.lost.contains(id) {
            let (t, why) = self.lost_truth(id)?;
            return Ok(Read::plain(t, why));
        }
        if let Some(hit) = ev.memo.get(id) {
            return Ok(hit.clone());
        }
        if ev.nodes.contains(id) {
            ev.cuts += 1;
            return Ok(Read::plain(
                Truth::Unknown,
                "it comes back to itself, through its support or a circle of objections, and a \
                 circle has no verdict"
                    .to_string(),
            ));
        }
        ev.nodes.insert(id.to_string());
        let cuts_before = ev.cuts;
        let answer = self.truth_uncached(id, ev, depth);
        ev.nodes.remove(id);
        let answer = answer?;
        if ev.cuts == cuts_before {
            ev.memo.insert(id.to_string(), answer.clone());
        }
        Ok(answer)
    }

    fn truth_uncached(&self, id: &str, ev: &mut Eval<'_>, depth: usize) -> Result<Read, DynoError> {
        if depth == 0 {
            return Ok(Read::plain(
                Truth::Unknown,
                format!("the chain of steps is deeper than {INFERENCE_DEPTH}"),
            ));
        }
        let Some(node_type) = self.type_holding(id)? else {
            return Ok(Read::plain(
                Truth::Unknown,
                "no single node of the design holds this id".to_string(),
            ));
        };
        if node_type == node::INFERENCE {
            let r = self.step_reading(id, ev, depth)?;
            let why = match &r.note {
                Some(note) if r.verdict == Truth::Unknown => {
                    format!("a step whose verdict is unknown: {note}")
                }
                _ => format!("a step whose rule `{}` {}", r.rule, r.verdict.as_str()),
            };
            return Ok(Read {
                truth: r.verdict,
                why,
                assumes: r.assumes.into_iter().collect(),
                unverified: r.rests_on_unverified.into_iter().collect(),
            });
        }
        if self.is_discontinued(id)? {
            return Ok(Read::plain(
                Truth::Fails,
                "discontinued by an accepted decision".to_string(),
            ));
        }
        // HISTORY IS NEVER REFUTED (req:the-records-a-choice-rests-on-carry-a-
        // truth-and-a-counter-argument-is-its-own-statement, Anthony
        // 2026-10-18). A ChangeEvent records that something happened; a reply
        // answers an ARGUMENT, never the event. Measured on reflow2's own
        // design: a reply to the counter-argument a ChangeEvent carried read
        // that ChangeEvent "refuted", i.e. false history.
        if node_type == node::CHANGE_EVENT {
            return Ok(Read::plain(
                Truth::Unknown,
                "a ChangeEvent is history: it records that something happened, so reflow2 \
                 computes no truth for it and an objection never makes it false. An argument it \
                 carried is recorded as a statement of its own (a Requirement with \
                 `statement_kind`) that CONTRADICTS the claim, and answered there"
                    .to_string(),
            ));
        }
        // AN OBJECTION COUNTS AS FAR AS IT HOLDS (requirement A; the module
        // documentation has the rule). One that holds refutes the node, and
        // every step resting on it is reported broken; one that is unknown
        // contests it; one that fails is answered. `supporting` (corroboration)
        // is never an objection.
        //
        // AND ONLY INSIDE ITS SCOPE (req:a-claim-that-holds-only-under-an-open-
        // supposition-cannot-refute-a-claim-outside-it, 2026-10-18, the cogito
        // trial). An objection that holds only under an assumption no step
        // discharges — or only if an unverified citation says what it is
        // recalled to say — is a result inside an argument. Against a claim
        // outside that argument it establishes only "if S, then not this",
        // which refutes nothing: it CONTESTS the claim and names S. Against a
        // claim that holds under the same assumptions (inside the scope) it
        // refutes, as it does inside any hypothetical argument. The node's own
        // reading is computed first, because it is the scope.
        let own = self.own_truth(id, &node_type, ev, depth)?;
        let objections = self.objections_within(id, ev, depth, &own)?;
        let refuting: Vec<&str> = objections
            .iter()
            .filter(|o| o.standing == "refutes")
            .map(|o| o.id.as_str())
            .collect();
        if !refuting.is_empty() {
            return Ok(Read::plain(
                Truth::Fails,
                format!("refuted by {}, which holds", refuting.join(", ")),
            ));
        }
        let standing: Vec<&ObjectionReading> = objections
            .iter()
            .filter(|o| o.standing == "stands")
            .collect();
        if standing.is_empty() || own.truth == Truth::Fails {
            return Ok(own);
        }
        let conditional: Vec<String> = standing
            .iter()
            .filter(|o| !o.holds_only_under.is_empty())
            .map(|o| {
                format!(
                    "{} holds only under {}, outside which it contests and never refutes",
                    o.id,
                    o.holds_only_under.join(", ")
                )
            })
            .collect();
        let ids: Vec<&str> = standing.iter().map(|o| o.id.as_str()).collect();
        Ok(Read {
            truth: Truth::Unknown,
            why: if conditional.is_empty() {
                format!(
                    "contested: {} stand(s) against it, neither established nor answered; on its \
                     own, {}",
                    ids.join(", "),
                    own.why
                )
            } else {
                format!(
                    "contested: {} stand(s) against it, neither established nor answered ({}); on \
                     its own, {}",
                    ids.join(", "),
                    conditional.join("; "),
                    own.why
                )
            },
            assumes: own.assumes,
            unverified: own.unverified,
        })
    }

    /// Every opposing CONTRADICTS against `id`, each with the objection's own
    /// truth and how it stands: `refutes`, `stands` or `answered`. Sorted by
    /// the objection's id; empty when nothing objects.
    pub fn objections_to(&self, id: &str) -> Result<Vec<ObjectionReading>, DynoError> {
        let lost = BTreeSet::new();
        let mut ev = Eval::new(&lost);
        // `id` is on the path, so an objection that comes back to it reads
        // the way its truth reads it: unknown.
        ev.nodes.insert(id.to_string());
        // Its own reading is the scope an objection is judged against.
        let own = match self.type_holding(id)? {
            Some(ty) => self.own_truth(id, &ty, &mut ev, INFERENCE_DEPTH)?,
            None => Read::plain(Truth::Unknown, String::new()),
        };
        self.objections_within(id, &mut ev, INFERENCE_DEPTH, &own)
    }

    /// The objections against `id`, each judged against `scope` — the
    /// target's own reading: an objection holding only under an assumption or
    /// an unverified citation the target does not hold under too STANDS (it
    /// contests), and never refutes.
    fn objections_within(
        &self,
        id: &str,
        ev: &mut Eval<'_>,
        depth: usize,
        scope: &Read,
    ) -> Result<Vec<ObjectionReading>, DynoError> {
        let mut out = Vec::new();
        for e in self.incoming(id, Some(edge::CONTRADICTS))? {
            let alignment = e
                .properties
                .get("alignment")
                .and_then(Value::as_str)
                .unwrap_or("opposing");
            if alignment == "supporting" {
                continue;
            }
            let r = self.truth_full(&e.from_id, ev, depth.saturating_sub(1))?;
            let truth = r.truth;
            let outside: Vec<String> = r
                .assumes
                .difference(&scope.assumes)
                .chain(r.unverified.difference(&scope.unverified))
                .cloned()
                .collect();
            let (standing, why) = if truth == Truth::Holds && !outside.is_empty() {
                (
                    "stands",
                    format!(
                        "{}; it holds only under {}, which the claim it opposes does not hold \
                         under, so it contests that claim and does not refute it (only \"if {}, \
                         then not this\" is established)",
                        r.why,
                        outside.join(", "),
                        outside.join(" and ")
                    ),
                )
            } else {
                (standing_of(truth), r.why)
            };
            out.push(ObjectionReading {
                node_type: self.type_holding(&e.from_id)?,
                standing,
                holds_only_under: if truth == Truth::Holds {
                    outside
                } else {
                    Vec::new()
                },
                evidence: e
                    .properties
                    .get("evidence")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                id: e.from_id,
                truth,
                why,
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// What the steps concluding `id` yield, ANY across them: `holds` when one
    /// holds (under the fewest open assumptions), otherwise UNSUPPORTED
    /// (unknown, never `fails`). For a requirement a realized, checked
    /// satisfier delivers it directly first. `None` when no step concludes it.
    fn concluded_by_steps(
        &self,
        id: &str,
        node_type: &str,
        ev: &mut Eval<'_>,
        depth: usize,
    ) -> Result<Option<Read>, DynoError> {
        let steps = self.inferences_concluding(id)?;
        if steps.is_empty() {
            return Ok(None);
        }
        if node_type == node::REQUIREMENT {
            let by = self.requirement_directly_delivered_by(id)?;
            if !by.is_empty() {
                return Ok(Some(Read::plain(
                    Truth::Holds,
                    format!("delivered directly by {}", by.join(", ")),
                )));
            }
        }
        let mut verdicts = Vec::new();
        for s in &steps {
            let r = self.step_reading(s, ev, depth)?;
            verdicts.push((s.clone(), r.verdict, r.assumes, r.rests_on_unverified));
        }
        let said: Vec<String> = verdicts
            .iter()
            .map(|(s, v, _, _)| format!("{s} {}", v.as_str()))
            .collect();
        // UNSUPPORTED IS NOT REFUTED (req:unsupported-is-not-refuted). A
        // step that fails or cannot be read leaves its conclusion without
        // that line of support; it never makes the conclusion false. So a
        // concluded node holds when one step holds and is otherwise
        // unknown: only an objection that holds refutes it.
        //
        // Of the holding steps, the one resting on the FEWEST open
        // assumptions decides what the node holds under: a line that needs
        // no assumption beats one that does (alternatives are ANY).
        // Then the fewest UNVERIFIED CITATIONS: a line resting on a citation
        // nobody checked holds only if the citation says what it is recalled
        // to say, so a line that needs none beats one that does.
        let best = verdicts
            .iter()
            .filter(|(_, v, _, _)| *v == Truth::Holds)
            .min_by_key(|(_, _, a, u)| (a.len(), u.len()));
        if let Some((_, _, assumes, unverified)) = best {
            let assumes: BTreeSet<String> = assumes.iter().cloned().collect();
            let unverified: BTreeSet<String> = unverified.iter().cloned().collect();
            let mut why = if assumes.is_empty() {
                format!("concluded by {}", said.join("; "))
            } else {
                format!(
                    "concluded by {}, under the assumption(s) {}, which no step on the way \
                     discharges",
                    said.join("; "),
                    assumes.iter().cloned().collect::<Vec<_>>().join(", ")
                )
            };
            if !unverified.is_empty() {
                why.push_str(&format!(
                    ", resting on the unverified citation(s) {}",
                    unverified.iter().cloned().collect::<Vec<_>>().join(", ")
                ));
            }
            return Ok(Some(Read {
                truth: Truth::Holds,
                why,
                assumes,
                unverified,
            }));
        }
        Ok(Some(Read::plain(
            Truth::Unknown,
            format!(
                "unsupported: no step concluding it holds ({}); losing its support does not \
                 make it false",
                said.join("; ")
            ),
        )))
    }

    /// THE CHAIN'S OWN VERDICT for a node steps conclude: what its steps yield,
    /// ANY across them, read apart from its status. For an accepted Decision
    /// this is the verdict read BESIDE its owner's acceptance. `None` when no
    /// step concludes it.
    pub fn chain_reading(&self, id: &str) -> Result<Option<(Truth, String)>, DynoError> {
        let Some(node_type) = self.type_holding(id)? else {
            return Ok(None);
        };
        let lost = BTreeSet::new();
        let mut ev = Eval::new(&lost);
        ev.nodes.insert(id.to_string());
        Ok(self
            .concluded_by_steps(id, &node_type, &mut ev, INFERENCE_DEPTH)?
            .map(Read::pair))
    }

    /// Whether `id` is an ACCEPTED Decision: one that holds on its owner's
    /// word whatever its chain says.
    pub fn is_accepted_decision(&self, id: &str) -> Result<bool, DynoError> {
        Ok(self
            .get_node(node::DECISION, id)?
            .is_some_and(|n| prop(&n.properties, "status") == Some("accepted")))
    }

    /// A TemporalFact's truth: a MEASURED finding that still STANDS holds; a
    /// forecast, an ended or an invalidated one is unknown (the module
    /// documentation has the rule).
    fn finding_truth(
        &self,
        id: &str,
        props: &HashMap<String, Value>,
    ) -> Result<(Truth, String), DynoError> {
        let invalidated: Vec<String> = self
            .incoming(id, Some(edge::INVALIDATES))?
            .into_iter()
            .map(|e| e.from_id)
            .collect();
        if !invalidated.is_empty() {
            return Ok((
                Truth::Unknown,
                format!(
                    "invalidated by {}: a later record says this finding is stale, so it is not \
                     read as standing until it is measured again (stale is unsupported, never \
                     refuted)",
                    invalidated.join(", ")
                ),
            ));
        }
        let ended_at: Option<String> = match prop(props, "valid_to").filter(|v| !v.is_empty()) {
            Some(date) => Some(format!("valid_to {date}")),
            None => self
                .outgoing(id, Some(edge::VALID_TO))?
                .into_iter()
                .map(|e| format!("VALID_TO {}", e.to_id))
                .next(),
        };
        if let Some(at) = ended_at {
            return Ok((
                Truth::Unknown,
                format!(
                    "superseded: it records when it stopped being true ({at}), so it no longer \
                     stands as a premise"
                ),
            ));
        }
        match prop(props, "basis") {
            Some("forecast") => Ok((
                Truth::Unknown,
                "a forecast: it projects what is expected and is evidence of nothing that \
                 happened, so it is unknown"
                    .to_string(),
            )),
            Some("measured") => Ok((
                Truth::Holds,
                "a measured finding that still stands: nothing invalidates it and it states no \
                 end"
                .to_string(),
            )),
            None => Ok((
                Truth::Holds,
                "a measured finding (its basis is unstated, which the schema reads as measured) \
                 that still stands: nothing invalidates it and it states no end"
                    .to_string(),
            )),
            Some(other) => Ok((
                Truth::Unknown,
                format!("its basis `{other}` is neither measured nor forecast"),
            )),
        }
    }

    /// What a node's own type and status say about it, before any objection
    /// is counted: the predicate per type in the module documentation.
    fn own_truth(
        &self,
        id: &str,
        node_type: &str,
        ev: &mut Eval<'_>,
        depth: usize,
    ) -> Result<Read, DynoError> {
        let Some(n) = self.get_node(node_type, id)? else {
            return Ok(Read::plain(
                Truth::Unknown,
                "the node is not there".to_string(),
            ));
        };
        // A NODE IN ANOTHER DESIGN, STOOD IN FOR BY A REFERENCE, READS AS IT
        // READS THERE (req:reasoning-crosses-designs-within-a-hub, crate::far):
        // its computed truth, carried with what it holds only under; a stale or
        // unreachable far end is unknown, never holding.
        if node_type == node::RESOURCE
            && let Some(far) = self.far_premise(id)?
        {
            return Ok(far_read(&far));
        }
        let status = prop(&n.properties, "status").map(str::to_string);
        if node_type == node::REQUIREMENT && status.as_deref() == Some("dropped") {
            return Ok(Read::plain(
                Truth::Fails,
                "the requirement is dropped".to_string(),
            ));
        }

        // A FALSUM IS FALSE (req:a-proof-can-state-a-contradiction-cases-
        // suppositions-and-cited-theorems, item 1). A contradiction derived
        // inside a reductio is never a truth: what its derivation shows is that
        // the assumptions under it cannot all hold. A step resting on it reads
        // it by whether it is DERIVED ([`Self::falsum_derivation`]).
        if is_falsum(&n.properties) {
            return self.falsum_truth(id, node_type, ev, depth);
        }

        // A CONCLUSION IS WHATEVER ITS STEPS YIELD. For a requirement a
        // realized, checked satisfier still delivers it directly, as it always
        // has; its own `met` is a claim the steps are checked against, never a
        // reason to believe them.
        if let Some(chain) = self.concluded_by_steps(id, node_type, ev, depth)? {
            // AN ACCEPTED DECISION READS ITS OWNER'S ACCEPTANCE AND ITS CHAIN
            // SIDE BY SIDE (req:the-records-a-choice-rests-on-carry-a-truth-
            // and-a-counter-argument-is-its-own-statement, part 3). The owner
            // accepted it; the chain is the stated argument for it, read beside
            // the acceptance and never in its place, so a first, incomplete
            // chain does not read as the decision being unsupported. A lost
            // chain still shows: in `chain_reading`, in the change walk
            // (`accepted_without_a_holding_chain`) and in the
            // `conclusion_does_not_follow` finding.
            if node_type == node::DECISION && status.as_deref() == Some("accepted") {
                return Ok(Read::plain(
                    Truth::Holds,
                    format!(
                        "the decision is accepted (its owner's word); beside it, its chain {}: {}",
                        chain.truth.as_str(),
                        chain.why
                    ),
                ));
            }
            return Ok(chain);
        }

        // A RESULT CITED FROM ELSEWHERE (item 4): a theorem, lemma or corollary
        // taken as given holds AS CITED, and one no pinned
        // source documents is carried as an unverified citation.
        if node_type == node::REQUIREMENT && is_cited_result(&n.properties) {
            return self.citation_read(id);
        }
        if node_type == node::CONSTRAINT && is_cited_result(&n.properties) {
            let (truth, why) = self.constraint_truth(id, &n.properties)?;
            if truth != Truth::Holds {
                return Ok(Read::plain(truth, why));
            }
            return self.citation_read(id);
        }

        let (truth, why) = match node_type {
            node::REQUIREMENT => {
                if is_taken_as_given(&n.properties) {
                    (
                        Truth::Holds,
                        "taken as given: a stated foundation the reasoning starts from".to_string(),
                    )
                } else if self.requirement_delivered_by_within(id, depth)?.is_some() {
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
                Some("passing") => match self.computed_check_truth(id, &n.properties)? {
                    Some(computed) => computed,
                    None => (Truth::Holds, "the check is passing".to_string()),
                },
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
                Some("contested") => (
                    Truth::Unknown,
                    "the decision is contested, by design: both sides are kept and no verdict \
                     is owed"
                        .to_string(),
                ),
                other => (
                    Truth::Unknown,
                    format!(
                        "the decision is {}, not settled",
                        other.unwrap_or("proposed")
                    ),
                ),
            },
            node::CONSTRAINT => {
                let (truth, why) = self.constraint_truth(id, &n.properties)?;
                if is_taken_as_given(&n.properties) {
                    (truth, format!("taken as given, a stated foundation; {why}"))
                } else {
                    (truth, why)
                }
            }
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
            node::TEMPORAL_FACT => self.finding_truth(id, &n.properties)?,
            other => (
                Truth::Unknown,
                format!("reflow2 computes no truth for a {other}, so it is unknown, never guessed"),
            ),
        };
        Ok(Read::plain(truth, why))
    }

    /// A falsum's own truth: FALSE, always, with what its derivation shows.
    fn falsum_truth(
        &self,
        id: &str,
        node_type: &str,
        ev: &mut Eval<'_>,
        depth: usize,
    ) -> Result<Read, DynoError> {
        let Some(d) = self.concluded_by_steps(id, node_type, ev, depth)? else {
            return Ok(Read::plain(
                Truth::Fails,
                "a falsum (⊥): false. Nothing derives it yet, so it shows nothing".to_string(),
            ));
        };
        if d.truth != Truth::Holds {
            return Ok(Read::plain(
                Truth::Fails,
                format!(
                    "a falsum (⊥): false, and not derived: {}; a reductio resting on it does not \
                     hold until it is",
                    d.why
                ),
            ));
        }
        let why = if d.assumes.is_empty() {
            format!(
                "a falsum (⊥): false, and derived under NO open assumption ({}): the foundations \
                 it rests on are INCONSISTENT",
                d.why
            )
        } else {
            format!(
                "a falsum (⊥): false. It is derived under the assumption(s) {} ({}), which shows \
                 they cannot all hold with what else it rests on: a reductio that discharges one \
                 of them concludes its negation",
                d.assumes.iter().cloned().collect::<Vec<_>>().join(", "),
                d.why
            )
        };
        Ok(Read {
            truth: Truth::Fails,
            why,
            assumes: d.assumes,
            unverified: d.unverified,
        })
    }

    /// A falsum read AS A PREMISE: by whether it is derived. A reductio rests
    /// on "⊥ follows from the supposition", never on "⊥ is true", so the
    /// premise holds when a step deriving it holds, carrying the assumptions
    /// that derivation is under (which the reductio then discharges); and it
    /// is unknown, never false, while nothing derives it.
    fn falsum_derivation(
        &self,
        id: &str,
        ev: &mut Eval<'_>,
        depth: usize,
    ) -> Result<Read, DynoError> {
        if ev.nodes.contains(id) {
            ev.cuts += 1;
            return Ok(Read::plain(
                Truth::Unknown,
                "it comes back to itself through its support, and a circle has no verdict"
                    .to_string(),
            ));
        }
        let Some(node_type) = self.type_holding(id)? else {
            return Ok(Read::plain(
                Truth::Unknown,
                "no single node of the design holds this id".to_string(),
            ));
        };
        ev.nodes.insert(id.to_string());
        let derivation = self.concluded_by_steps(id, &node_type, ev, depth);
        ev.nodes.remove(id);
        Ok(match derivation? {
            Some(d) if d.truth == Truth::Holds => Read {
                truth: Truth::Holds,
                why: format!("a falsum (⊥), derived: {}", d.why),
                assumes: d.assumes,
                unverified: d.unverified,
            },
            Some(d) => Read::plain(
                Truth::Unknown,
                format!("a falsum (⊥) not derived: {}", d.why),
            ),
            None => Read::plain(
                Truth::Unknown,
                "a falsum (⊥) nothing derives: there is no contradiction to rest on yet"
                    .to_string(),
            ),
        })
    }

    /// The pinned sources that document `id`: Artifacts that DOCUMENTS it
    /// and carry a checksum — the version that was read.
    fn pinned_sources(&self, id: &str) -> Result<Vec<String>, DynoError> {
        Ok(self
            .cited_sources(id)?
            .into_iter()
            .filter(|s| s.pinned)
            .map(|s| s.id)
            .collect())
    }

    /// A cited result as a premise: it holds AS CITED, and unless a pinned
    /// source documents it, it is an UNVERIFIED citation that everything
    /// resting on it carries.
    fn citation_read(&self, id: &str) -> Result<Read, DynoError> {
        let pinned = self.pinned_sources(id)?;
        if !pinned.is_empty() {
            return Ok(Read::plain(
                Truth::Holds,
                format!(
                    "cited, not proved here: a result from elsewhere, checked against its pinned \
                     source(s) {}",
                    pinned.join(", ")
                ),
            ));
        }
        Ok(Read {
            truth: Truth::Holds,
            why: "cited, unverified: a result from elsewhere that no pinned source documents \
                  (recalled, or named only in its `source` text), so what rests on it holds only \
                  if it says what it is recalled to say. Register the source that was read as an \
                  Artifact with its checksum, and draw DOCUMENTS from it"
                .to_string(),
            assumes: BTreeSet::new(),
            unverified: BTreeSet::from([id.to_string()]),
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
        let mut reading = self.step_reading(inference_id, &mut ev, INFERENCE_DEPTH)?;
        reading.independence = self.independence_of(inference_id)?;
        Ok(Some(reading))
    }

    // ─── Assumptions, independence and statement kinds ────────────────────

    /// Every ASSUMPTION the design makes: each node some step takes up as an
    /// `assumed` premise, with those steps and the steps that discharge it
    /// (a step naming it in `discharges` that rests on one of them). Sorted
    /// by id; empty for a design that assumes nothing.
    pub fn assumptions(&self) -> Result<Vec<AssumptionReading>, DynoError> {
        let mut assumed_in: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let steps = self.scan_nodes(node::INFERENCE)?;
        for st in &steps {
            for (p, assumed) in self.premise_links(&st.node_id)? {
                if assumed {
                    assumed_in.entry(p).or_default().insert(st.node_id.clone());
                }
            }
        }
        if assumed_in.is_empty() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for (a, in_steps) in assumed_in {
            let mut discharged_by = Vec::new();
            for d in &steps {
                if !list_prop(&d.properties, "discharges").contains(&a)
                    && !list_prop(&d.properties, "cases").contains(&a)
                {
                    continue;
                }
                let under = self.steps_under(&d.node_id)?;
                if in_steps.iter().any(|s| under.contains(s)) {
                    discharged_by.push(d.node_id.clone());
                }
            }
            discharged_by.sort();
            let name = match self.type_holding(&a)? {
                Some(ty) => self
                    .get_node(&ty, &a)?
                    .and_then(|n| prop(&n.properties, "name").map(str::to_string)),
                None => None,
            };
            out.push(AssumptionReading {
                id: a,
                name,
                assumed_in: in_steps.into_iter().collect(),
                discharged_by,
            });
        }
        Ok(out)
    }

    /// Every step a step rests on, itself included: its premises' concluding
    /// steps, theirs, and so on (assumed premises too).
    pub fn steps_under(&self, step: &str) -> Result<BTreeSet<String>, DynoError> {
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut queue: Vec<String> = vec![step.to_string()];
        while let Some(s) = queue.pop() {
            if !seen.insert(s.clone()) {
                continue;
            }
            for p in self.premises_of(&s)? {
                queue.extend(self.inferences_concluding(&p)?);
                // A premise that is itself a step is a step under it.
                if self.get_node(node::INFERENCE, &p)?.is_some() {
                    queue.push(p);
                }
            }
        }
        Ok(seen)
    }

    /// The assumptions a step being written would rest on through `premises`:
    /// those it takes up itself (`assumed`) and every `assumed` premise under
    /// an ordinary one. What `discharges` may name.
    fn assumptions_beneath(
        &self,
        premises: &[String],
        assumed: &[String],
    ) -> Result<BTreeSet<String>, DynoError> {
        let mut out: BTreeSet<String> = assumed.iter().cloned().collect();
        let mut steps: Vec<String> = Vec::new();
        for p in premises {
            steps.extend(self.inferences_concluding(p)?);
            if self.get_node(node::INFERENCE, p)?.is_some() {
                steps.push(p.clone());
            }
        }
        let mut seen: BTreeSet<String> = BTreeSet::new();
        while let Some(s) = steps.pop() {
            if !seen.insert(s.clone()) {
                continue;
            }
            for (p, is_assumed) in self.premise_links(&s)? {
                if is_assumed {
                    out.insert(p.clone());
                }
                steps.extend(self.inferences_concluding(&p)?);
            }
        }
        Ok(out)
    }

    /// Each stated independence of one step (`independent_of`), checked: the
    /// two lines may share no step other than those under the lemmas named
    /// in `independent_except` (a step concluding one, and every step under
    /// that). Definitions and axioms share no step, so they never count.
    pub fn independence_of(&self, step: &str) -> Result<Vec<IndependenceReading>, DynoError> {
        let Some(n) = self.get_node(node::INFERENCE, step)? else {
            return Ok(Vec::new());
        };
        let others = list_prop(&n.properties, "independent_of");
        if others.is_empty() {
            return Ok(Vec::new());
        }
        let except = list_prop(&n.properties, "independent_except");
        let mine = self.steps_under(step)?;
        let mut allowed: BTreeSet<String> = BTreeSet::new();
        for lemma in &except {
            for s in self.inferences_concluding(lemma)? {
                allowed.extend(self.steps_under(&s)?);
            }
        }
        let my_nodes = self.rests_on_closure(step)?;
        let mut out = Vec::new();
        for other in others {
            if other == step || self.get_node(node::INFERENCE, &other)?.is_none() {
                out.push(IndependenceReading {
                    step: step.to_string(),
                    of: other.clone(),
                    holds: false,
                    shared_steps: Vec::new(),
                    shares_named: Vec::new(),
                    except: except.clone(),
                    why: format!(
                        "'{other}' is not another step of the design, so there is no second line \
                         to be independent of"
                    ),
                });
                continue;
            }
            let theirs = self.steps_under(&other)?;
            let their_nodes = self.rests_on_closure(&other)?;
            let shared_steps: Vec<String> = mine
                .intersection(&theirs)
                .filter(|s| !allowed.contains(*s))
                .cloned()
                .collect();
            let shares_named: Vec<String> = except
                .iter()
                .filter(|l| my_nodes.contains(*l) && their_nodes.contains(*l))
                .cloned()
                .collect();
            let holds = shared_steps.is_empty();
            let why = if holds {
                if shares_named.is_empty() {
                    format!("the two lines share no step: {step} and {other} are independent")
                } else {
                    format!(
                        "the two lines share no step beyond the named lemma(s) {}",
                        shares_named.join(", ")
                    )
                }
            } else {
                format!(
                    "the two lines both rest on {} — step(s) under no lemma the statement names, \
                     so they are not independent as stated: name the shared lemma in \
                     `independent_except`, or the lines are one argument",
                    shared_steps.join(", ")
                )
            };
            out.push(IndependenceReading {
                step: step.to_string(),
                of: other,
                holds,
                shared_steps,
                shares_named,
                except: except.clone(),
                why,
            });
        }
        Ok(out)
    }

    /// Every stated independence in the design, checked, by step.
    pub fn stated_independence(&self) -> Result<Vec<IndependenceReading>, DynoError> {
        let mut out = Vec::new();
        for st in self.scan_nodes(node::INFERENCE)? {
            if list_prop(&st.properties, "independent_of").is_empty() {
                continue;
            }
            out.extend(self.independence_of(&st.node_id)?);
        }
        Ok(out)
    }

    /// Every step that generalizes over a variable something it rests on
    /// fixes — each one does not hold. Sorted by step id.
    pub fn generalizations_that_do_not_hold(
        &self,
    ) -> Result<Vec<(String, GeneralizationReading)>, DynoError> {
        let mut out = Vec::new();
        for st in self.scan_nodes(node::INFERENCE)? {
            if list_prop(&st.properties, "generalizes_over").is_empty() {
                continue;
            }
            // Read through the step, so the assumptions open at it count.
            if let Some(g) = self
                .inference_reading(&st.node_id)?
                .and_then(|r| r.generalization)
                && !g.holds
            {
                out.push((st.node_id.clone(), g));
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    /// What a statement says it is (`statement_kind`) and how it reads now;
    /// `None` when it says nothing.
    pub fn statement_reading(&self, id: &str) -> Result<Option<StatementReading>, DynoError> {
        let Some(ty) = self.type_holding(id)? else {
            return Ok(None);
        };
        let Some(n) = self.get_node(&ty, id)? else {
            return Ok(None);
        };
        let Some(kind) = prop(&n.properties, "statement_kind").map(str::to_string) else {
            return Ok(None);
        };
        let (truth, why) = self.premise_truth(id)?;
        if kind == "falsum" {
            // A FALSUM READS FALSE: `false` when derived under an assumption
            // (the reductio's contradiction), `inconsistent` when derived under
            // none, `stated` while nothing derives it — never holds, and never
            // "refuted", which is what an objection does to a claim.
            let lost = BTreeSet::new();
            let mut ev = Eval::new(&lost);
            let d = self.falsum_derivation(id, &mut ev, INFERENCE_DEPTH)?;
            let reads = if d.truth != Truth::Holds {
                "stated"
            } else if d.assumes.is_empty() {
                "inconsistent"
            } else {
                "false"
            };
            return Ok(Some(StatementReading { kind, reads, why }));
        }
        let proof = self.proof_reading(id)?;
        let (reads, why): (&'static str, String) = if truth == Truth::Fails {
            ("refuted", why)
        } else if kind == "hypothesis" {
            // A HYPOTHESIS IS ASSUMED, never "stated, not shown" (item 3).
            match self.assumptions()?.into_iter().find(|a| a.id == id) {
                Some(a) if !a.discharged_by.is_empty() => (
                    "discharged",
                    format!(
                        "a hypothesis: assumed in {} and discharged by {}, so nothing concluded \
                         past that step rests on it",
                        a.assumed_in.join(", "),
                        a.discharged_by.join(", ")
                    ),
                ),
                Some(a) => (
                    "assumed",
                    format!(
                        "a hypothesis: assumed in {}, and no step discharges it, so what rests on \
                         it holds only under it",
                        a.assumed_in.join(", ")
                    ),
                ),
                None => (
                    "not_assumed",
                    "a hypothesis no step takes up: a step takes it up as an `assumed` premise \
                     (add_inference's `assumed`), and it is never read as a premise that holds"
                        .to_string(),
                ),
            }
        } else if matches!(kind.as_str(), "definition" | "axiom") {
            (
                "taken_as_given",
                format!("a {kind}: what the reasoning starts from, a stated foundation"),
            )
        } else if let Some(p) = proof {
            (p.status.as_str(), p.why)
        } else if CITED_KINDS.contains(&kind.as_str())
            && self
                .node_props(id)?
                .is_some_and(|props| is_cited_result(&props))
        {
            // A RESULT CITED FROM ELSEWHERE (item 4), never an axiom.
            if self.pinned_sources(id)?.is_empty() {
                ("cited_unverified", why)
            } else {
                ("cited", why)
            }
        } else if kind == "conjecture" {
            (
                "open",
                "a conjecture no step shows and nothing that holds refutes".to_string(),
            )
        } else {
            (
                "stated",
                format!("a {kind} no step concludes yet: stated, not shown"),
            )
        };
        Ok(Some(StatementReading { kind, reads, why }))
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
                confirmation: Confirmation::default(),
                assumes: Vec::new(),
                rests_on_unverified: Vec::new(),
                discharges: Vec::new(),
                cases: None,
                generalization: None,
                independence: Vec::new(),
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
            confirmation: rec.confirmation,
            assumes: Vec::new(),
            rests_on_unverified: Vec::new(),
            discharges: rec.discharges.clone(),
            cases: None,
            generalization: None,
            independence: Vec::new(),
        };
        if ev.lost.contains(inference_id) {
            let (truth, why) = self.lost_truth(inference_id)?;
            reading.verdict = truth;
            reading.note = Some(why);
            return Ok(reading);
        }
        if ev.path.contains(inference_id) {
            ev.cuts += 1;
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
        // A PREMISE HAS A ROLE (req:a-proof-can-discharge-an-assumption-and-
        // generalize-over-a-variable, Anthony's choice of 2026-10-17: a
        // premise role). An ordinary premise is read by its computed truth. An
        // ASSUMED one is a hypothesis taken up for the sake of argument — the
        // reductio's "suppose the primes are finite", an induction hypothesis
        // — and holds inside the argument whatever its own truth, so its
        // falsity never fails the step; it is carried, as an assumption, by
        // everything that rests on the step until a step DISCHARGES it.
        let mut values = Vec::new();
        let mut assumes: BTreeSet<String> = BTreeSet::new();
        let mut unverified: BTreeSet<String> = BTreeSet::new();
        for (p, assumed) in self.premise_links(inference_id)? {
            if assumed {
                values.push(Truth::Holds);
                assumes.insert(p.clone());
                reading.premises.push(PremiseReading {
                    node_type: self.type_holding(&p)?,
                    id: p,
                    truth: Truth::Holds,
                    why: "assumed: taken up by this step for the sake of argument, and held \
                          inside it whatever its own truth"
                        .to_string(),
                    role: Some("assumed"),
                    name: None,
                });
                continue;
            }
            // A FALSUM is read as a premise by whether it is DERIVED: a
            // reductio rests on "⊥ follows", never on "⊥ is true".
            let r = if self.node_props(&p)?.is_some_and(|props| is_falsum(&props)) {
                self.falsum_derivation(&p, ev, depth - 1)?
            } else {
                self.truth_full(&p, ev, depth - 1)?
            };
            values.push(r.truth);
            assumes.extend(r.assumes);
            unverified.extend(r.unverified);
            reading.premises.push(PremiseReading {
                node_type: self.type_holding(&p)?,
                id: p,
                truth: r.truth,
                why: r.why,
                role: None,
                name: None,
            });
        }
        // The cases of a case split are discharged by it, as `discharges`
        // would (∨-elimination closes each case's hypothesis).
        for d in rec.discharges.iter().chain(rec.cases.iter()) {
            assumes.remove(d);
        }
        reading.holding = values.iter().filter(|v| **v == Truth::Holds).count();
        reading.failing = values.iter().filter(|v| **v == Truth::Fails).count();
        reading.unknown = values.len() - reading.holding - reading.failing;
        match rec.parsed.and_then(|rule| combine(rule, &values)) {
            Ok(v) => reading.verdict = v,
            Err(why) => reading.note = Some(why),
        }
        // A CASE SPLIT HOLDS ONLY WHEN ITS CASES ARE DECLARED EXHAUSTIVE
        // (item 2). Which record declares it is the step's to say; whether it
        // holds is that record's own computed truth — the owner's word (taken
        // as given) or a checker's (a passing computed check) — never
        // reflow2's judgement of the cases.
        if !rec.cases.is_empty() {
            let cases = match &rec.cases_exhaustive {
                None => CasesReading {
                    cases: rec.cases.clone(),
                    declared: false,
                    exhaustive_by: None,
                    truth: None,
                    why: format!(
                        "it splits on the cases {} and nothing declares them exhaustive: name \
                         the record saying they cover every possibility in `cases_exhaustive` \
                         (the owner's word, or a checker's)",
                        rec.cases.join(", ")
                    ),
                },
                Some(by) => {
                    let r = self.truth_full(by, ev, depth - 1)?;
                    unverified.extend(r.unverified.iter().cloned());
                    assumes.extend(r.assumes.iter().cloned());
                    CasesReading {
                        cases: rec.cases.clone(),
                        declared: true,
                        exhaustive_by: Some(by.clone()),
                        truth: Some(r.truth),
                        why: format!(
                            "the cases {} are declared exhaustive by {by}, which {}: {}",
                            rec.cases.join(", "),
                            r.truth.as_str(),
                            r.why
                        ),
                    }
                }
            };
            let gate = cases.truth.unwrap_or(Truth::Unknown);
            if gate != Truth::Holds {
                if reading.verdict == Truth::Holds || gate == Truth::Fails {
                    reading.verdict = if gate == Truth::Fails {
                        Truth::Fails
                    } else {
                        Truth::Unknown
                    };
                }
                let why = if cases.declared {
                    format!(
                        "a case split holds only while its cases are exhaustive; {}",
                        cases.why
                    )
                } else {
                    format!(
                        "a case split holds only when its cases are declared exhaustive; {}",
                        cases.why
                    )
                };
                reading.note = Some(match reading.note.take() {
                    Some(n) => format!("{n}; {why}"),
                    None => why,
                });
            }
            reading.cases = Some(cases);
        }
        ev.path.remove(inference_id);
        reading.rests_on_unverified = unverified.into_iter().collect();
        // A GENERALIZATION IS CHECKED (Anthony's choice of 2026-10-17:
        // declared and checked). A step generalizing over a variable that
        // something it rests on FIXES does not hold: its verdict reads
        // unknown with the reason, so its conclusion is unsupported, never
        // refuted.
        // THE EIGENVARIABLE RULE (2026-10-18, the Arrow trial's N19): and
        // over a variable no assumption still OPEN at this step names. The
        // open ones are `assumes`, read after this step's own discharges.
        if !rec.generalizes_over.is_empty() {
            let g = self.generalization_reading(inference_id, &rec.generalizes_over, &assumes)?;
            if !g.holds {
                let mut reasons: Vec<String> = g
                    .fixed_by
                    .iter()
                    .map(|f| format!("{} fixes {}", f.id, f.fixes.join(", ")))
                    .collect();
                reasons.extend(g.named_by_open_assumptions.iter().map(|f| {
                    format!(
                        "{} is an assumption still open at this step and names {}",
                        f.id,
                        f.fixes.join(", ")
                    )
                }));
                reasons.extend(g.undeclared_open_assumptions.iter().map(|a| {
                    format!(
                        "{a} is an assumption still open at this step and declares no \
                         `variables`, so whether it names one cannot be told (an empty list says \
                         it names none)"
                    )
                }));
                let why = format!(
                    "it generalizes over {}, and {}: a step generalizes only over a variable \
                     nothing it rests on fixes and no assumption still open at it names \
                     (discharge the assumption first, on this step or one beneath it)",
                    g.over.join(", "),
                    reasons.join("; ")
                );
                if reading.verdict == Truth::Holds {
                    reading.verdict = Truth::Unknown;
                }
                reading.note = Some(match reading.note.take() {
                    Some(n) => format!("{n}; {why}"),
                    None => why,
                });
            }
            reading.generalization = Some(g);
        }
        reading.assumes = assumes.into_iter().collect();
        Ok(reading)
    }

    /// The generalization check for one step over `over`: every record it
    /// rests on, at any depth, that fixes one of the variables (the step does
    /// not hold), and every one that mentions one (shown only).
    fn generalization_reading(
        &self,
        step: &str,
        over: &[String],
        open: &BTreeSet<String>,
    ) -> Result<GeneralizationReading, DynoError> {
        let wanted: BTreeSet<&str> = over.iter().map(String::as_str).collect();
        let mut fixed_by = Vec::new();
        let mut mentioned_by = Vec::new();
        for id in self.rests_on_closure(step)? {
            let Some(ty) = self.type_holding(&id)? else {
                continue;
            };
            let Some(n) = self.get_node(&ty, &id)? else {
                continue;
            };
            let fixes: Vec<String> = list_prop(&n.properties, "fixes")
                .into_iter()
                .filter(|f| wanted.contains(fixed_variable(f)))
                .collect();
            if !fixes.is_empty() {
                fixed_by.push(FixedBy {
                    id: id.clone(),
                    fixes,
                });
            }
            if list_prop(&n.properties, "variables")
                .iter()
                .any(|v| wanted.contains(v.as_str()))
            {
                mentioned_by.push(id);
            }
        }
        let mut named_by_open_assumptions = Vec::new();
        let mut undeclared_open_assumptions = Vec::new();
        for a in open {
            let Some(props) = self.node_props(a)? else {
                undeclared_open_assumptions.push(a.clone());
                continue;
            };
            match named_variables(&props) {
                None => undeclared_open_assumptions.push(a.clone()),
                Some(names) => {
                    let hit: Vec<String> = over
                        .iter()
                        .filter(|v| names.contains(v.as_str()))
                        .cloned()
                        .collect();
                    if !hit.is_empty() {
                        named_by_open_assumptions.push(FixedBy {
                            id: a.clone(),
                            fixes: hit,
                        });
                    }
                }
            }
        }
        Ok(GeneralizationReading {
            over: over.to_vec(),
            holds: fixed_by.is_empty()
                && named_by_open_assumptions.is_empty()
                && undeclared_open_assumptions.is_empty(),
            fixed_by,
            named_by_open_assumptions,
            undeclared_open_assumptions,
            mentioned_by,
        })
    }

    /// A node's properties, when exactly one node holds `id`.
    pub(crate) fn node_props(&self, id: &str) -> Result<Option<HashMap<String, Value>>, DynoError> {
        let Some(ty) = self.type_holding(id)? else {
            return Ok(None);
        };
        Ok(self.get_node(&ty, id)?.map(|n| n.properties))
    }

    /// The steps concluding `id` whose rule holds OUTRIGHT, sorted — what
    /// delivers a requirement an Inference concludes. `depth` bounds the
    /// chain, shared with the roll-up that asked.
    ///
    /// A step holding only under an assumption no step discharges, or only
    /// if an unverified citation is right, is a result inside an argument and
    /// delivers nothing (req:a-claim-that-holds-only-under-an-open-supposition-
    /// cannot-refute-a-claim-outside-it, the same class: until 2026-10-18 it
    /// counted as delivering).
    pub(crate) fn holding_inferences_within(
        &self,
        id: &str,
        depth: usize,
    ) -> Result<Vec<String>, DynoError> {
        let lost = BTreeSet::new();
        let mut ev = Eval::new(&lost);
        let mut holding = Vec::new();
        for s in self.inferences_concluding(id)? {
            let r = self.step_reading(&s, &mut ev, depth)?;
            if r.verdict == Truth::Holds && r.assumes.is_empty() && r.rests_on_unverified.is_empty()
            {
                holding.push(s);
            }
        }
        Ok(holding)
    }

    // ─── The change walk ───────────────────────────────────────────────────

    /// What losing `seeds` does to the steps that rest on them. A lost node
    /// reads UNSUPPORTED (unknown), never false (req:unsupported-is-not-refuted).
    /// Each step a lost premise reaches reads UNSUPPORTED (it held and its rule
    /// no longer holds), WEAKENED (it still holds on its other premises),
    /// BROKEN (its rule fails, because a premise really fails) or UNKNOWN; and
    /// each row says what its CLAIM is left with — `still_supported` (by
    /// `still_supported_by`), `unsupported` or `refuted`. The walk carries on
    /// past a conclusion that held and no longer does — counting it lost in
    /// turn — and stops at one that still stands, so a weakened conclusion's
    /// dependents never lose their support. A seed is counted lost whatever it
    /// is: the walk answers "what if this premise no longer held?", the
    /// question to ask BEFORE the edit (propagate_change, then edit).
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
            // Carry on past every conclusion that held and no longer does — it
            // is left UNSUPPORTED, and what rests on it loses its support in
            // turn. Losses only grow, so a conclusion that stood a round ago
            // may fall now: re-read until nothing new is lost.
            let mut ev = Eval::new(&lost);
            let mut newly: Vec<(String, usize)> = Vec::new();
            for (s, distance) in &reached {
                let Some(c) = self.conclusion_of(s)? else {
                    continue;
                };
                if lost.contains(&c) || newly.iter().any(|(n, _)| *n == c) {
                    continue;
                }
                if self.truth_of(&c, &mut ev, INFERENCE_DEPTH)?.0 != Truth::Holds
                    && self.premise_truth(&c)?.0 == Truth::Holds
                {
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
                let (truth, why) = self.lost_truth(s)?;
                r.verdict = truth;
                r.note = Some(why);
                r
            } else {
                self.step_reading(s, &mut after_ev, INFERENCE_DEPTH)?
            };
            let conclusion_after = match &after.conclusion {
                Some(c) => self.truth_of(c, &mut after_ev, INFERENCE_DEPTH)?.0,
                None => Truth::Unknown,
            };
            // The other lines: every step concluding the claim that still
            // holds with the losses counted.
            let mut still_supported_by = Vec::new();
            if conclusion_after == Truth::Holds
                && let Some(c) = &after.conclusion
            {
                for other in self.inferences_concluding(c)? {
                    if lost.contains(&other) {
                        continue;
                    }
                    if self
                        .step_reading(&other, &mut after_ev, INFERENCE_DEPTH)?
                        .verdict
                        == Truth::Holds
                    {
                        still_supported_by.push(other);
                    }
                }
            }
            // An accepted Decision stands on its owner's acceptance whatever
            // its chain does; when no line of its chain holds any more, the
            // row says THAT, never that another line still supports it.
            let held_by_acceptance_alone = conclusion_after == Truth::Holds
                && still_supported_by.is_empty()
                && match &after.conclusion {
                    Some(c) => self.is_accepted_decision(c)?,
                    None => false,
                };
            let conclusion_reads = match conclusion_after {
                Truth::Holds if held_by_acceptance_alone => "accepted_without_a_holding_chain",
                Truth::Holds => "still_supported",
                Truth::Fails => "refuted",
                Truth::Unknown => "unsupported",
            };
            let lost_premises: Vec<String> = after
                .premises
                .iter()
                .filter(|p| lost.contains(&p.id))
                .map(|p| p.id.clone())
                .collect();
            rows.push(InferenceImpact {
                inference: s.clone(),
                impact: match (before.verdict, after.verdict) {
                    (_, Truth::Holds) => "weakened",
                    (_, Truth::Fails) => "broken",
                    (Truth::Holds, Truth::Unknown) => "unsupported",
                    (_, Truth::Unknown) => "unknown",
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
                conclusion_reads,
                still_supported_by,
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
            let mut supported = self.truth_of(&c, &mut ev, INFERENCE_DEPTH)?.0;
            // An accepted Decision holds on its owner's word; what this
            // finding asks is whether its CHAIN supports it, read beside the
            // acceptance (req:the-records-a-choice-rests-on-carry-a-truth-and-
            // a-counter-argument-is-its-own-statement, part 3).
            if supported == Truth::Holds && node_type == node::DECISION && claimed == "accepted" {
                supported = self
                    .concluded_by_steps(&c, &node_type, &mut ev, INFERENCE_DEPTH)?
                    .map_or(Truth::Holds, |r| {
                        if r.assumes.is_empty() && r.unverified.is_empty() {
                            r.truth
                        } else {
                            Truth::Unknown
                        }
                    });
            }
            // A CONDITIONAL RESULT SUPPORTS NO CLAIMED STATUS: holding only
            // under an open assumption (or an unverified citation), it is a
            // result inside an argument, and a `met` or `accepted` resting on
            // it claims more than its steps support (the same class as
            // req:a-claim-that-holds-only-under-an-open-supposition-cannot-
            // refute-a-claim-outside-it).
            if supported == Truth::Holds {
                let r = self.truth_full(&c, &mut ev, INFERENCE_DEPTH)?;
                if !r.assumes.is_empty() || !r.unverified.is_empty() {
                    supported = Truth::Unknown;
                }
            }
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

    /// Whether `id` takes part in a line of reasoning: a step concludes it, or
    /// it is a premise of one.
    pub fn takes_part_in_reasoning(&self, id: &str) -> Result<bool, DynoError> {
        Ok(!self.inferences_concluding(id)?.is_empty() || !self.steps_resting_on(id)?.is_empty())
    }

    /// Whether `id` belongs to a line of reasoning: a step concludes it, it is
    /// a premise of one, or it is a statement (`statement_kind`, or marked
    /// `taken_as_given`).
    pub fn is_in_a_line_of_reasoning(&self, id: &str) -> Result<bool, DynoError> {
        if self.takes_part_in_reasoning(id)? {
            return Ok(true);
        }
        let Some(ty) = self.type_holding(id)? else {
            return Ok(false);
        };
        Ok(self.get_node(&ty, id)?.is_some_and(|n| {
            n.properties.contains_key("statement_kind")
                || n.properties.contains_key("taken_as_given")
        }))
    }

    /// How an opposing CONTRADICTS from `objection` to `target` stands, as
    /// an attack in a line of reasoning: `refutes` (the objection holds),
    /// `answered` (it fails) or `stands` (neither: a lone objection, or one
    /// side of a mutual attack or a cycle, undecided). `None` when neither
    /// side is in a line of reasoning, where it is no attack but a plain
    /// contradiction.
    pub fn attack_standing(
        &self,
        objection: &str,
        target: &str,
    ) -> Result<Option<&'static str>, DynoError> {
        if !self.is_in_a_line_of_reasoning(objection)? && !self.is_in_a_line_of_reasoning(target)? {
            return Ok(None);
        }
        // Read through the target's scoped objections, so an objection holding
        // only under a supposition the target is outside reads `stands`.
        Ok(Some(
            match self
                .objections_to(target)?
                .into_iter()
                .find(|o| o.id == objection)
            {
                Some(o) => o.standing,
                None => standing_of(self.premise_truth(objection)?.0),
            },
        ))
    }

    /// An opposing CONTRADICTS from `objection` to `target` that REFUTES it, in
    /// a line of reasoning: one side is in a line of reasoning
    /// ([`Self::is_in_a_line_of_reasoning`]), the objection holds, and so the
    /// target reads `fails` (refuted).
    pub fn is_a_standing_refutation(
        &self,
        objection: &str,
        target: &str,
    ) -> Result<bool, DynoError> {
        if !self.is_in_a_line_of_reasoning(objection)? && !self.is_in_a_line_of_reasoning(target)? {
            return Ok(false);
        }
        if !self
            .objections_to(target)?
            .iter()
            .any(|o| o.id == objection && o.standing == "refutes")
        {
            return Ok(false);
        }
        Ok(self.premise_truth(target)?.0 == Truth::Fails)
    }

    /// Whether a step concludes `id` — the one question the older status
    /// checks ask before speaking about a conclusion this module owns.
    pub fn is_concluded(&self, id: &str) -> Result<bool, DynoError> {
        Ok(!self.inferences_concluding(id)?.is_empty())
    }

    // ─── Single points of failure in a line of reasoning ──────────────────

    /// Every node that EVERY holding line of a claim rests on, so losing it
    /// leaves a claim with two or more lines unsupported — a single point of
    /// failure in the reasoning (req:a-proof-is-designed-with-a-skill-and-the-
    /// tools-fit-mathematics; the primes trial's F25: Lemma 5 carried both
    /// proofs of the theorem and nothing said so).
    ///
    /// Only claims with at least two holding steps are asked (one line is not
    /// redundancy, the same selectivity the topology rule keeps), and only a
    /// node a step concludes — a lemma — is a candidate: a definition or
    /// axiom every line starts from is the foundation, not a weak point. Of a
    /// chain of such lemmas, the one nearest the claim is named: a lemma under
    /// it falls with it. Checked, not inferred: each candidate is counted lost
    /// and the claim read again.
    pub fn reasoning_single_points(&self) -> Result<Vec<ReasoningSinglePoint>, DynoError> {
        let mut conclusions: BTreeSet<String> = BTreeSet::new();
        for i in self.scan_nodes(node::INFERENCE)? {
            if let Some(c) = self.conclusion_of(&i.node_id)? {
                conclusions.insert(c);
            }
        }
        let mut out = Vec::new();
        for c in conclusions {
            let steps = self.inferences_concluding(&c)?;
            if steps.len() < 2 || self.premise_truth(&c)?.0 != Truth::Holds {
                continue;
            }
            let none = BTreeSet::new();
            let mut ev = Eval::new(&none);
            let mut lines: Vec<String> = Vec::new();
            for st in &steps {
                if self.step_reading(st, &mut ev, INFERENCE_DEPTH)?.verdict == Truth::Holds {
                    lines.push(st.clone());
                }
            }
            if lines.len() < 2 {
                continue;
            }
            // What each line rests on, all the way down.
            let mut shared: Option<BTreeSet<String>> = None;
            for st in &lines {
                let rests = self.rests_on_closure(st)?;
                shared = Some(match shared {
                    None => rests,
                    Some(acc) => acc.intersection(&rests).cloned().collect(),
                });
            }
            let mut points: Vec<String> = Vec::new();
            for x in shared.unwrap_or_default() {
                if x == c || self.inferences_concluding(&x)?.is_empty() {
                    continue;
                }
                let lost = BTreeSet::from([x.clone()]);
                let mut ev = Eval::new(&lost);
                if self.truth_of(&c, &mut ev, INFERENCE_DEPTH)?.0 != Truth::Holds {
                    points.push(x);
                }
            }
            // The nearest of a chain: drop a point another point rests on.
            let mut nearest: Vec<String> = Vec::new();
            for x in &points {
                let mut under_another = false;
                for y in &points {
                    if y != x && self.rests_on_closure_of_node(y)?.contains(x) {
                        under_another = true;
                        break;
                    }
                }
                if !under_another {
                    nearest.push(x.clone());
                }
            }
            for x in nearest {
                out.push(ReasoningSinglePoint {
                    claim: c.clone(),
                    node: x,
                    lines: lines.clone(),
                });
            }
        }
        Ok(out)
    }

    /// Every node a step rests on, transitively: its premises, and for a
    /// premise a step concludes, what each of those steps rests on.
    fn rests_on_closure(&self, step: &str) -> Result<BTreeSet<String>, DynoError> {
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut steps_seen: BTreeSet<String> = BTreeSet::new();
        let mut queue: Vec<String> = vec![step.to_string()];
        while let Some(s) = queue.pop() {
            if !steps_seen.insert(s.clone()) {
                continue;
            }
            for p in self.premises_of(&s)? {
                if seen.insert(p.clone()) {
                    queue.extend(self.inferences_concluding(&p)?);
                }
            }
        }
        Ok(seen)
    }

    /// What a NODE rests on through every step concluding it.
    pub(crate) fn rests_on_closure_of_node(&self, id: &str) -> Result<BTreeSet<String>, DynoError> {
        let mut all = BTreeSet::new();
        for st in self.inferences_concluding(id)? {
            all.extend(self.rests_on_closure(&st)?);
        }
        Ok(all)
    }

    // ─── Foundations (requirement E) ──────────────────────────────────────

    /// THE FOUNDATIONS WALK. Given a node id, walk backward through the
    /// inference chain (the node's concluding Inferences → their premises →
    /// if a premise is itself concluded, recurse) to find TERMINAL PREMISES —
    /// nodes not concluded by any Inference. These are the axioms: the chosen
    /// principles the conclusion ultimately rests on.
    ///
    /// A node not concluded by any Inference has no foundations (it IS a
    /// terminal node); returns empty, raises nothing
    /// (con:no-gap-is-raised-for-a-conclusion-that-has-no-inference).
    ///
    /// Handles multiple inference chains: a node concluded by more than one
    /// Inference has alternative derivations; reports the foundations of all
    /// (the union). Guards against cycles (C refuses them at write time, but
    /// an import might introduce one).
    pub fn foundations_of(&self, id: &str) -> Result<Vec<Foundation>, DynoError> {
        let steps = self.inferences_concluding(id)?;
        if steps.is_empty() {
            return Ok(Vec::new());
        }
        let mut axioms: BTreeMap<String, Foundation> = BTreeMap::new();
        let mut visited: BTreeSet<String> = BTreeSet::new();
        visited.insert(id.to_string());
        self.foundations_walk(&steps, &mut axioms, &mut visited)?;
        Ok(axioms.into_values().collect())
    }

    /// Recursive walk: for each step, read its premises; terminal premises
    /// are axioms, concluded premises are walked further.
    fn foundations_walk(
        &self,
        steps: &[String],
        axioms: &mut BTreeMap<String, Foundation>,
        visited: &mut BTreeSet<String>,
    ) -> Result<(), DynoError> {
        for step_id in steps {
            if !visited.insert(step_id.clone()) {
                continue; // cycle guard
            }
            for (premise_id, assumed) in self.premise_links(step_id)? {
                // AN ASSUMPTION IS NEVER A FOUNDATION: it is taken up for the
                // sake of argument, and either discharged on the way (the
                // conclusion does not rest on it) or listed apart as what the
                // conclusion holds under (`FoundationsReading.assumes`).
                if assumed {
                    continue;
                }
                if !visited.insert(premise_id.clone()) {
                    continue; // cycle guard on premises too
                }
                let concluding = self.inferences_concluding(&premise_id)?;
                if concluding.is_empty() {
                    // TERMINAL PREMISE — an axiom.
                    if !axioms.contains_key(&premise_id) {
                        let (truth, why) = self.premise_truth(&premise_id)?;
                        let node_type = self.type_holding(&premise_id)?;
                        axioms.insert(
                            premise_id.clone(),
                            Foundation {
                                id: premise_id,
                                node_type,
                                truth,
                                why,
                            },
                        );
                    }
                } else {
                    // Intermediate — recurse through its concluding steps.
                    self.foundations_walk(&concluding, axioms, visited)?;
                }
            }
        }
        Ok(())
    }

    /// The full foundations reading for a node: its axioms, the honest limit,
    /// and whether the list was capped. `None` when the node is not concluded
    /// by any Inference (no foundations to show — it IS terminal).
    pub fn foundations_reading(&self, id: &str) -> Result<Option<FoundationsReading>, DynoError> {
        let steps = self.inferences_concluding(id)?;
        if steps.is_empty() {
            return Ok(None);
        }
        let foundations = self.foundations_of(id)?;
        let total = foundations.len();
        let capped = total > FOUNDATIONS_CAP;
        let shown = if capped {
            foundations.into_iter().take(FOUNDATIONS_CAP).collect()
        } else {
            foundations
        };
        Ok(Some(FoundationsReading {
            foundations: shown,
            total,
            capped,
            honest_limit: FOUNDATIONS_HONEST_LIMIT.to_string(),
            assumes: self.assumptions_under(id)?,
        }))
    }

    /// The proof status of a concluded node: proven, shown, or unproven.
    /// `None` when the node is not concluded by any Inference.
    ///
    /// - **Proven**: at least one derivation chain holds AND every step in it
    ///   is confirmed (`confirmed_how` is `person` or `checker`).
    /// - **Shown**: at least one derivation holds, but one or more steps in
    ///   every holding chain are unconfirmed.
    /// - **Contested**: a derivation holds, but an objection stands against
    ///   the conclusion, neither established nor answered (requirement A).
    /// - **Unproven**: no derivation holds, or an objection that holds
    ///   refutes the conclusion.
    pub fn proof_reading(&self, id: &str) -> Result<Option<ProofReading>, DynoError> {
        let top_steps = self.inferences_concluding(id)?;
        if top_steps.is_empty() {
            return Ok(None);
        }
        // TWO QUESTIONS, ANSWERED APART. Whether a line is wholly confirmed is
        // judged per line, so each line walks from its own empty visited set
        // and sees every one of its steps, shared ones included. How many
        // steps are under the claim is the size of the UNION of those lines:
        // a step two lines share is one step.
        let lost = BTreeSet::new();
        let mut ev = Eval::new(&lost);
        let mut any_holds = false;
        let mut any_proven_chain = false;
        let mut under: BTreeSet<String> = BTreeSet::new();
        let mut confirmed_under: BTreeSet<String> = BTreeSet::new();
        for s in &top_steps {
            let reading = self.step_reading(s, &mut ev, INFERENCE_DEPTH)?;
            if reading.verdict == Truth::Holds {
                any_holds = true;
                let mut chain_confirmed = true;
                let mut visited: BTreeSet<String> = BTreeSet::from([id.to_string()]);
                self.proof_chain_walk(
                    s,
                    &mut chain_confirmed,
                    &mut under,
                    &mut confirmed_under,
                    &mut visited,
                )?;
                if chain_confirmed {
                    any_proven_chain = true;
                }
            }
        }
        let total = under.len();
        let confirmed = confirmed_under.len();
        // A conclusion refuted by an objection that holds is unproven
        // regardless of its steps — a checked counterexample overrides
        // derivation. Its steps are still counted: the count says what is
        // under the claim, not whether it stands.
        let (conclusion_truth, why) = self.premise_truth(id)?;
        if conclusion_truth == Truth::Fails {
            return Ok(Some(ProofReading {
                status: ProofStatus::Unproven,
                why,
                total_steps: total,
                confirmed_steps: confirmed,
                unconfirmed_steps: total.saturating_sub(confirmed),
                counts: PROOF_COUNTS_MEAN,
            }));
        }
        let unconfirmed = total.saturating_sub(confirmed);
        // A derivation holds only UNDER AN ASSUMPTION no step on the way
        // discharges: a result inside a hypothetical argument, never a proof.
        if any_holds && conclusion_truth == Truth::Holds {
            let assumes = self.assumptions_under(id)?;
            if !assumes.is_empty() {
                return Ok(Some(ProofReading {
                    status: ProofStatus::Conditional,
                    why: format!(
                        "it holds only under the assumption(s) {}, which no step on the way to it \
                         discharges: a result inside an argument, not a proof of it",
                        assumes.join(", ")
                    ),
                    total_steps: total,
                    confirmed_steps: confirmed,
                    unconfirmed_steps: unconfirmed,
                    counts: PROOF_COUNTS_MEAN,
                }));
            }
        }
        // A derivation holds only IF AN UNVERIFIED CITATION says what it is
        // recalled to say (item 4): conditional on it, never shown outright.
        if any_holds && conclusion_truth == Truth::Holds {
            let unverified = self.unverified_under(id)?;
            if !unverified.is_empty() {
                return Ok(Some(ProofReading {
                    status: ProofStatus::Conditional,
                    why: format!(
                        "it holds only if the unverified citation(s) {} say what they are \
                         recalled to say: no pinned source documents them",
                        unverified.join(", ")
                    ),
                    total_steps: total,
                    confirmed_steps: confirmed,
                    unconfirmed_steps: unconfirmed,
                    counts: PROOF_COUNTS_MEAN,
                }));
            }
        }
        // A derivation holds, so only a standing objection leaves the
        // conclusion unknown: it is contested, neither proven nor shown.
        if any_holds && conclusion_truth == Truth::Unknown {
            return Ok(Some(ProofReading {
                status: ProofStatus::Contested,
                why,
                total_steps: total,
                confirmed_steps: confirmed,
                unconfirmed_steps: unconfirmed,
                counts: PROOF_COUNTS_MEAN,
            }));
        }
        if any_proven_chain {
            Ok(Some(ProofReading {
                status: ProofStatus::Proven,
                why: "every step in at least one holding derivation is confirmed".to_string(),
                total_steps: total,
                confirmed_steps: confirmed,
                unconfirmed_steps: unconfirmed,
                counts: PROOF_COUNTS_MEAN,
            }))
        } else if any_holds {
            Ok(Some(ProofReading {
                status: ProofStatus::Shown,
                why: format!(
                    "at least one derivation holds, but {unconfirmed} step(s) are unconfirmed"
                ),
                total_steps: total,
                confirmed_steps: confirmed,
                unconfirmed_steps: unconfirmed,
                counts: PROOF_COUNTS_MEAN,
            }))
        } else {
            Ok(Some(ProofReading {
                status: ProofStatus::Unproven,
                why: "no derivation holds".to_string(),
                total_steps: total,
                confirmed_steps: confirmed,
                unconfirmed_steps: unconfirmed,
                counts: PROOF_COUNTS_MEAN,
            }))
        }
    }

    /// One line, walked from `step_id` down: every step on it goes into
    /// `under` (and, when confirmed, `confirmed_under`), the sets shared by
    /// all of a claim's lines, and `chain_confirmed` turns false at the
    /// line's first unconfirmed step. `visited` is THIS line's, so a step an
    /// earlier line already counted is still judged on this one.
    fn proof_chain_walk(
        &self,
        step_id: &str,
        chain_confirmed: &mut bool,
        under: &mut BTreeSet<String>,
        confirmed_under: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> Result<(), DynoError> {
        if !visited.insert(step_id.to_string()) {
            return Ok(());
        }
        if let Some(rec) = self.step_record(step_id)? {
            under.insert(step_id.to_string());
            if rec.confirmation.is_confirmed() {
                confirmed_under.insert(step_id.to_string());
            } else {
                *chain_confirmed = false;
            }
        }
        for premise_id in self.premises_of(step_id)? {
            // A premise in another design is proven only as far as it is
            // proven THERE: a far result that is shown, conditional or worse
            // leaves this line unconfirmed (req:reasoning-crosses-designs-
            // within-a-hub, part 1).
            if let Some(far) = self.far_premise(&premise_id)?
                && far
                    .proof_status
                    .as_deref()
                    .is_some_and(|status| status != "proven")
            {
                *chain_confirmed = false;
            }
            let concluding = self.inferences_concluding(&premise_id)?;
            for sub_step in &concluding {
                self.proof_chain_walk(sub_step, chain_confirmed, under, confirmed_under, visited)?;
            }
        }
        Ok(())
    }

    /// Every node in this design that is a foundation (a terminal premise) of
    /// at least one conclusion. The design-wide view: "which nodes are axioms
    /// that something rests on?"
    pub fn design_foundations(&self) -> Result<Vec<String>, DynoError> {
        let mut conclusions: BTreeSet<String> = BTreeSet::new();
        for i in self.scan_nodes(node::INFERENCE)? {
            if let Some(c) = self.conclusion_of(&i.node_id)? {
                conclusions.insert(c);
            }
        }
        let mut all_axioms: BTreeSet<String> = BTreeSet::new();
        for c in conclusions {
            for f in self.foundations_of(&c)? {
                all_axioms.insert(f.id);
            }
        }
        Ok(all_axioms.into_iter().collect())
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
        if conclusion_link && let Some(other) = self.conclusion_of(from_id)? {
            return Err(refuse(format!(
                "'{from_id}' already concludes '{other}'. One Inference concludes exactly one \
                 node; to support '{to_id}' too, record a second step (add_inference)"
            )));
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
        // The premises, ordinary and assumed.
        let mut given: Vec<String> = spec.premises.clone();
        given.extend(spec.assumed.iter().cloned());
        given.sort();
        given.dedup();
        let assumed_given: BTreeSet<&String> = spec.assumed.iter().collect();
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

        // What it discharges: an assumption it, or a step it rests on, takes
        // up. A discharge of anything else would say a supposition was closed
        // that was never opened.
        let stored_links: Vec<(String, bool)> = if existing.is_some() {
            self.premise_links(&spec.id)?
        } else {
            Vec::new()
        };
        // A case split discharges its cases, so each must be assumed beneath
        // the step exactly as a discharge must.
        let mut closing: Vec<(&str, &Vec<String>)> = Vec::new();
        if let Some(d) = &spec.discharges {
            closing.push(("discharges", d));
        }
        if let Some(c) = &spec.cases {
            closing.push(("cases", c));
        }
        for (field, discharges) in closing {
            let mut assumed_here: Vec<String> = spec.assumed.clone();
            assumed_here.extend(
                stored_links
                    .iter()
                    .filter(|(_, a)| *a)
                    .map(|(p, _)| p.clone()),
            );
            let ordinary: Vec<String> = all_premises
                .iter()
                .filter(|p| !assumed_here.contains(p))
                .map(|p| p.to_string())
                .collect();
            let open = self.assumptions_beneath(&ordinary, &assumed_here)?;
            for d in discharges {
                if !open.contains(d) {
                    return Err(invalid(
                        field,
                        format!(
                            "'{d}' is not assumed anywhere this step rests on, so there is \
                             nothing to discharge: mark it `assumed` on the step that supposes \
                             it (add_inference's `assumed`), then name it in `{field}`. Assumed \
                             beneath this step: {}",
                            if open.is_empty() {
                                "nothing".to_string()
                            } else {
                                open.iter().cloned().collect::<Vec<_>>().join(", ")
                            }
                        ),
                    ));
                }
            }
        }
        for (field, list) in [
            ("generalizes_over", &spec.generalizes_over),
            ("discharges", &spec.discharges),
            ("independent_of", &spec.independent_of),
            ("independent_except", &spec.independent_except),
            ("cases", &spec.cases),
        ] {
            if let Some(items) = list
                && items.iter().any(|v| v.trim().is_empty())
            {
                return Err(invalid(
                    field,
                    format!("`{field}` holds an empty name; give each entry, or leave it out"),
                ));
            }
        }
        if let Some(others) = &spec.independent_of {
            for o in others {
                if *o == spec.id || self.get_node(node::INFERENCE, o)?.is_none() {
                    return Err(invalid(
                        "independent_of",
                        format!(
                            "'{o}' is not another step of this design: a line is stated \
                             independent of another STEP (an Inference id), typically a second \
                             step concluding the same claim"
                        ),
                    ));
                }
            }
        }
        let exhaustive_by: Option<(String, String)> =
            match spec.cases_exhaustive.as_deref().map(str::trim) {
                None | Some("") => None,
                Some(by) => {
                    if by == spec.id || by == spec.conclusion {
                        return Err(invalid(
                            "cases_exhaustive",
                            format!(
                                "'{by}' cannot say this step's own cases are exhaustive: name the \
                             record (\"every integer is even or odd\") that does"
                            ),
                        ));
                    }
                    Some((by.to_string(), self.one_type_for(by, "cases_exhaustive")?))
                }
            };
        if let Some(lemmas) = &spec.independent_except {
            for l in lemmas {
                if self.node_types_holding(l)?.is_empty() {
                    return Err(invalid(
                        "independent_except",
                        format!("no node '{l}' exists to be a lemma the lines may share"),
                    ));
                }
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
        if let Some(by) = &spec.confirmed_by {
            props.insert("confirmed_by".into(), Value::String(by.clone()));
        }
        if let Some(how) = &spec.confirmed_how {
            let valid = ["unconfirmed", "person", "checker"];
            if !valid.contains(&how.as_str()) {
                return Err(invalid(
                    "confirmed_how",
                    format!(
                        "'{}' is not a valid confirmation: the legal values are `person` \
                         (a named human), `checker` (a proof-checking program) and \
                         `unconfirmed` (nobody has)",
                        how
                    ),
                ));
            }
            props.insert("confirmed_how".into(), Value::String(how.clone()));
        }
        if let Some(at) = &spec.confirmed_at {
            props.insert("confirmed_at".into(), Value::String(at.clone()));
        }
        for (key, list) in [
            ("discharges", &spec.discharges),
            ("generalizes_over", &spec.generalizes_over),
            ("independent_of", &spec.independent_of),
            ("independent_except", &spec.independent_except),
            ("cases", &spec.cases),
        ] {
            if let Some(items) = list {
                let mut items: Vec<String> = items.iter().map(|v| v.trim().to_string()).collect();
                items.sort();
                items.dedup();
                if items.is_empty() {
                    props.remove(key);
                } else {
                    props.insert(
                        key.into(),
                        Value::List(items.into_iter().map(Value::String).collect()),
                    );
                }
            }
        }
        match spec.cases_exhaustive.as_deref().map(str::trim) {
            Some("") => {
                props.remove("cases_exhaustive");
            }
            Some(by) => {
                props.insert("cases_exhaustive".into(), Value::String(by.to_string()));
            }
            None => {}
        }
        let mut builder = Props::new();
        for (k, v) in props {
            builder = builder.set(&k, v);
        }
        let stored_node = self.upsert_node(node::INFERENCE, &spec.id, builder)?;
        let mut drawn = Vec::new();
        for (p, ty) in &typed {
            let assumed = assumed_given.contains(p);
            let stored_as = stored_links.iter().find(|(s, _)| s == p).map(|(_, a)| *a);
            // Already drawn in the role asked for: nothing to write. A
            // premise newly named as assumed is re-drawn with its role.
            if stored_as == Some(assumed) || (stored_as.is_some() && !assumed) {
                continue;
            }
            let props = if assumed {
                Props::new().set("role", "assumed")
            } else {
                Props::new()
            };
            self.create_edge(edge::PREMISE_OF, ty, p, node::INFERENCE, &spec.id, props)?;
            drawn.push(format!("{p} PREMISE_OF {}", spec.id));
        }
        // The exhaustiveness record is a premise of the split: losing it
        // reaches the step in a change walk.
        if let Some((by, ty)) = &exhaustive_by
            && !stored_links.iter().any(|(s, _)| s == by)
            && !typed.iter().any(|(p, _)| p == by)
        {
            self.create_edge(
                edge::PREMISE_OF,
                ty,
                by,
                node::INFERENCE,
                &spec.id,
                Props::new(),
            )?;
            drawn.push(format!("{by} PREMISE_OF {}", spec.id));
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
    /// Who confirmed this step: a contributor id or a checker's name.
    pub confirmed_by: Option<String>,
    /// How: `person`, `checker`, or absent (unconfirmed).
    pub confirmed_how: Option<String>,
    /// When, as an ISO 8601 timestamp.
    pub confirmed_at: Option<String>,
    /// Premises the step takes up as ASSUMPTIONS (PREMISE_OF `role:
    /// assumed`): hypotheses held inside the argument whatever their own
    /// truth. Drawn beside `premises`; a premise named in both is assumed.
    pub assumed: Vec<String>,
    /// The assumptions this step discharges. Each must be assumed by this
    /// step or by a step it rests on. Overwrites what is stored.
    pub discharges: Option<Vec<String>>,
    /// The variables this step generalizes over. Overwrites what is stored.
    pub generalizes_over: Option<Vec<String>>,
    /// Other steps this line is stated independent of. Overwrites.
    pub independent_of: Option<Vec<String>>,
    /// The lemmas the independent lines may share. Overwrites.
    pub independent_except: Option<Vec<String>>,
    /// The cases this step splits on: each case's hypothesis, assumed by the
    /// branch that proves it and discharged here. Overwrites.
    pub cases: Option<Vec<String>>,
    /// The record saying the cases are exhaustive, drawn as a premise of the
    /// step. Overwrites; an empty string removes it.
    pub cases_exhaustive: Option<String>,
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
