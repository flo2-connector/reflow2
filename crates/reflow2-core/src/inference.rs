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
//!
//!   ⭐ BUT A PREMISE SAYS WHAT IT RESTS ON
//!   (`dec:a-premise-says-whether-it-rests-on-intent-or-delivery`, Anthony
//!   2026-10-24): a requirement record carries two propositions, "X is
//!   required" and "X holds in the system". Read as its owner's INTENT
//!   ([`ReadsAs::Intent`], `intent_truth`) a Requirement,
//!   Capability or Interface holds on its owner's acceptance; read by its
//!   DELIVERY it reads as this list says. The premise's link says which
//!   (PREMISE_OF `reads_as`); unmarked, a step that concludes a DECISION
//!   reads it as intent and every other step by delivery. These per-type
//!   values were first taken from what reflow2 already computed, which is
//!   why a design choice could never read supported before it was built
//!   (`fact:root-cause-a-design-choice-read-its-requirements-by-delivery-because-the-premise-truth-reused-the-delivery-line-2026-10-24`);
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

mod memo;
use memo::{Kept, Stack, Stacks, Tracker};
pub(crate) use memo::{debt_held, note_truncation};
use std::rc::Rc;

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
pub const INFERENCE_RULES: [&str; 4] = ["all", "any", "k_of_n", "immediate"];

/// The values a premise's `reads_as` may take (PREMISE_OF `reads_as`).
pub const READS_AS: [&str; 2] = ["intent", "delivery"];

/// WHAT A STEP READS ONE OF ITS PREMISES AS
/// (dec:a-premise-says-whether-it-rests-on-intent-or-delivery, Anthony
/// 2026-10-24). A requirement record carries two propositions: "X is
/// required", true once its owner accepts it, and "X holds in the system",
/// true once it is delivered. A design choice rests on the first; an
/// assurance argument or a go/no-go decision on the second. Only a
/// Requirement that is a NEED (no `statement_kind`, not taken as given), a
/// Capability and an Interface carry the two; every other premise reads as
/// itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadsAs {
    /// By its owner's acceptance: accepted holds, proposed is unknown,
    /// dropped or rejected fails.
    Intent,
    /// By whether it is delivered — the reading every premise had before.
    /// `marked` when the premise says so on its link.
    Delivery { marked: bool },
}

impl ReadsAs {
    /// What a read-back shows: `intent`, `delivery` when the premise is
    /// marked so, and nothing for a premise read as it always was.
    pub fn served(self) -> Option<&'static str> {
        match self {
            ReadsAs::Intent => Some("intent"),
            ReadsAs::Delivery { marked: true } => Some("delivery"),
            ReadsAs::Delivery { marked: false } => None,
        }
    }
}

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
    /// NOT AN INFERENCE (req:a-step-can-say-it-is-not-an-inference): the
    /// conclusion is grasped immediately — by intuition, self-evidence or a
    /// self-verifying act — not derived. Any premises are what it is grasped
    /// IN, read as `all`; with none it holds on the grasp alone.
    Immediate,
}

/// The rule over premise values, three-valued (Kleene). `Err` names why the
/// rule cannot be applied (a `k` outside `1..=n`).
pub fn combine(rule: InferenceRule, values: &[Truth]) -> Result<Truth, String> {
    let n = values.len() as i64;
    let holds = values.iter().filter(|v| **v == Truth::Holds).count() as i64;
    let fails = values.iter().filter(|v| **v == Truth::Fails).count() as i64;
    let unknown = n - holds - fails;
    if n == 0 && rule == InferenceRule::Immediate {
        return Ok(Truth::Holds);
    }
    if n == 0 {
        return Err("it has no premises, so nothing supports its conclusion".to_string());
    }
    Ok(match rule {
        InferenceRule::All | InferenceRule::Immediate => {
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

/// What a cited result reads as when no pinned source documents it: the
/// whole `why`, the same sentence wherever such a citation is read. One
/// constant, so a reader that says it ONCE for every id it applies to
/// (`reasoning_report`'s `advisories`) finds every place it was said.
pub const CITED_UNVERIFIED_WHY: &str = "cited, unverified: a result from elsewhere that no pinned \
     source documents (recalled, or named only in its `source` text), so what rests on it holds \
     only if it says what it is recalled to say. Register the source that was read as an Artifact \
     with its checksum, and draw DOCUMENTS from it";

/// What a piece of evidence reads as when no pinned source documents it,
/// after `evidence` and its grade: the advisory every such report carries,
/// said once by `reasoning_report` for every id it applies to.
pub const EVIDENCE_UNVERIFIED_TAIL: &str = ", unverified: no pinned source documents it, so it is \
     named only in its `source` text. What rests on it holds only if the report says what it is \
     recalled to say — conditional on it, as on a recalled citation — until its source is \
     documented; then it is supported, never proven. Register the report that was read as an \
     Artifact with its checksum, and draw DOCUMENTS from it";

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
    /// HOW MANY TIMES THE STEP USES IT, when more than once — A, A ⊢ A∧A
    /// (req:natural-deduction-gaps-from-the-logic-rung-are-closed, item 4).
    /// Read once by the step's rule. Absent for once.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uses: Option<i64>,
    /// WHAT THE STEP READS THIS PREMISE AS
    /// (dec:a-premise-says-whether-it-rests-on-intent-or-delivery, Anthony
    /// 2026-10-24): `intent` — a requirement, capability or interface read by
    /// its owner's acceptance ("X is required"); `delivery` — read by whether
    /// it is delivered ("X holds in the system"), when the premise is marked
    /// so. Absent for a premise read as it always was: an unmarked one the
    /// step reads by delivery, and every type the marker does not apply to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reads_as: Option<&'static str>,
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
    /// A derivation holds, but it rests on a STATISTICAL TEST
    /// ([`crate::hypothesis`]): SUPPORTED AT α, evidence and never proof, so
    /// never `shown` or `proven`, however its steps are confirmed
    /// (req:a-hypothesis-tests-verdict-reads-as-supported-at-alpha-never-as-
    /// shown).
    Supported,
    /// A derivation holds, but an objection stands against the conclusion:
    /// neither established nor answered (requirement A). Or no derivation
    /// holds, but one would were the disputes under it — objections standing
    /// against its steps or premises, a `contested` Decision it rests on —
    /// settled its way.
    Contested,
    /// No derivation holds, no dispute holds one up, and a NAMED PARTY claims
    /// it — on the claim itself (`claimed_by`) or on the argument it rests on
    /// — and nobody has verified it: "claimed by <party>, unverified", never
    /// `unproven` (which also means refuted, and also means nobody tried)
    /// and never `open`, which hides that a claim was made
    /// (req:a-claim-made-by-a-named-party-and-not-yet-verified-reads-claimed).
    Claimed,
    /// It does not hold now, and a STATUS IT HELD OVER A PERIOD has ended (a
    /// dated `status` record, `valid_from`–`valid_to`): accepted for a time
    /// and withdrawn — Kempe's proof, 1879–1890 — read back with the period
    /// and what withdrew it (same requirement).
    Withdrawn,
    /// Another claim takes its place (`replaces`, drawn as OBSOLETES from the
    /// successor), or the design it was mirrored from withdrew it
    /// (`superseded_by_surface`): it feeds no chain and is read as
    /// superseded, never as contested (req:a-superseded-mirrored-judgment-
    /// is-kept-and-what-rested-on-it-reads-reason-lost, part 5).
    Superseded,
    /// No derivation holds (the conclusion does not follow) and no dispute is
    /// what keeps one from holding, or an objection that holds refutes it. A
    /// claim no route holds while a dispute holds one up is `Contested`
    /// (req:a-conclusion-whose-every-route-is-contested-reads-contested).
    Unproven,
}

impl ProofStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ProofStatus::Proven => "proven",
            ProofStatus::Shown => "shown",
            ProofStatus::Conditional => "conditional",
            ProofStatus::Supported => "supported",
            ProofStatus::Contested => "contested",
            ProofStatus::Claimed => "claimed",
            ProofStatus::Withdrawn => "withdrawn",
            ProofStatus::Superseded => "superseded",
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
    /// THE NODE STATING ITS RULE (`rule_node`), read: the step holds only
    /// while it holds, and it is never one of the premises
    /// (req:natural-deduction-gaps-from-the-logic-rung-are-closed, item 2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_node: Option<RuleNodeReading>,
    /// WHAT RELATION IT ESTABLISHES (`establishes`): `entailment` (⊨, by
    /// truth table or model) or `derivability` (⊢, by deduction) — item 5.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub establishes: Option<String>,
    /// THE CONCLUSION IS MADE OF ITS PREMISES AS PARTS (`conclusion_form`):
    /// `conjunction` or `biconditional` — item 6.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conclusion_form: Option<String>,
    /// HOW IT HOLDS, when not outright (item 7): `conditional` (on an
    /// assumption no step on the way discharges, or on an unverified
    /// citation) or `supported` (only through evidence: a statistical test, a
    /// report). Read at the step itself, as its conclusion reads it. Absent
    /// when it holds outright or does not hold.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holds_as: Option<&'static str>,
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
    /// THE BRIDGES this step's verdict rests on, at any depth: statements of
    /// kind `bridge`, each saying a formal statement models a claim of
    /// another field (dec:idea-a-bridge-premise-says-a-formal-statement-
    /// models-a-claim, option b). A step resting on one is conditional on it.
    /// Empty when it rests on none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rests_on_bridges: Vec<String>,
    /// CIRCULAR, AND WHY: a premise of this step is proven EQUIVALENT to its
    /// conclusion (an EQUIVALENT_TO edge, at any distance) or is the same
    /// formula (`same_formula_as`), so the step only restates the gap it
    /// would close (dec:idea-speculate-the-bridge-that-would-close-a-gap-
    /// then-test-it, option c). A flag, never a verdict: the step reads by
    /// its rule as before. Read only on the step itself (get_node,
    /// reasoning_report's support), never inside another step's reading.
    /// Absent when it is not circular.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circular: Option<String>,
    /// THE STATISTICAL TESTS this step's verdict rests on, at any depth:
    /// checks whose support is a hypothesis test that rejected H0
    /// ([`crate::hypothesis`]). A step resting on one holds as EVIDENCE at
    /// that test's α, never as proof. Empty when it rests on none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rests_on_tests: Vec<String>,
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
    /// EVERY STATUS THE STEP HELD OVER A PERIOD (dated `status` records),
    /// read on the step itself. Empty when it records none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub status_over_time: Vec<StatusPeriod>,
    /// ACCEPTED FOR A PERIOD AND WITHDRAWN: the step does not hold now and
    /// every status it held has ended — the period, and what withdrew it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withdrawn: Option<WithdrawnReading>,
    /// The named party whose argument this step is (`claimed_by`), when it
    /// says one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<String>,
    /// EVERY LINE this step holds on, with what each holds under, when there
    /// is more than one (req:a-claim-with-several-lines-reads-each-lines-
    /// assumptions): it holds under any one of them. `assumes`,
    /// `rests_on_unverified` and `rests_on_tests` are the lightest line's.
    /// Empty for a step with one line, or one that does not hold.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holds_under_any_of: Vec<LineConditions>,
    /// True when it holds on more minimal lines than [`LINES_CAP`], and only
    /// the lightest are listed.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub lines_capped: bool,
    /// THE OBJECTIONS AIMED AT THIS STEP ITSELF — at its warrant or rule, not
    /// at a premise or the conclusion: an opposing CONTRADICTS into the step
    /// (an undercutter; req:an-objection-aimed-at-a-step-is-read-against-that-
    /// step, Anthony 2026-10-18), each with how it stands.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub objections: Vec<ObjectionReading>,
    /// How the step stands against them: `defeated` (one holds, unanswered:
    /// the step FAILS and supports nothing — its conclusion is left
    /// unsupported, never refuted), `contested` (one stands, neither
    /// established nor answered: the step's verdict is at most unknown) or
    /// `answered` (every one fails: the step is reinstated and reads by its
    /// rule). Absent when nothing objects to the step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub objected: Option<&'static str>,
    /// WHAT KEEPS THIS STEP FROM HOLDING, WHEN ONLY A DISPUTE DOES: the
    /// objections standing against it or against a premise it rests on, and
    /// the `contested` Decisions it rests on. Its verdict is unknown, and it
    /// would hold were they settled its way: a CONTESTED route, never an
    /// unsupported one (req:a-conclusion-whose-every-route-is-contested-
    /// reads-contested). Empty when it holds, fails, or is unknown for any
    /// other reason.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contested_by: Vec<String>,
    /// THE REFUTED SUPPOSITIONS it was derived under: open suppositions it,
    /// or a premise beneath it, takes up as `assumed` while they read refuted
    /// outside the argument (req:a-claim-derived-under-a-refuted-supposition-
    /// says-so). The step then does not hold. Empty almost always.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rests_on_refuted_suppositions: Vec<String>,
    /// Every minimal line, lightest first, for the readers above it.
    #[serde(skip)]
    lines: Vec<Condition>,
}

/// The node stating a step's rule (`rule_node`), read.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RuleNodeReading {
    /// The rule's node.
    pub id: String,
    /// Its computed truth: the step holds only while it holds.
    pub truth: Truth,
    /// Why, in a sentence.
    pub why: String,
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
    /// CITED FOUNDATIONS both lines rest on that no named lemma accounts for
    /// — a cited theorem, a piece of evidence (req:natural-deduction-gaps-
    /// from-the-logic-rung-are-closed, item 7): two lines through one cited
    /// result are not independent, whatever their steps. Empty when none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub shared_citations: Vec<String>,
    /// PREMISES both lines rest on that no named lemma accounts for: a
    /// statement neither line derives — a proposition taken as given, a claim
    /// nobody has shown, an assumption no step anywhere discharges (PR A2,
    /// three trials: Navier–Stokes, four colour, Gettier). Losing one costs
    /// both lines, so two lines through it are not independent as stated.
    /// Empty when none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub shared_premises: Vec<String>,
    /// WHAT BOTH LINES SHARE THAT LEAVES THEM INDEPENDENT, named so a reader
    /// sees it: the vocabulary (a definition, an axiom or a rule of method,
    /// which fixes what the words mean and is no premise a line can lose) and
    /// the theorem's own stated setting (a premise BOTH final steps take up
    /// directly, as every proof of the theorem must). A definition that is
    /// really data — a chosen list, a measured set — is recorded as a
    /// proposition taken as given, and then counts. Empty when none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub shared_setting: Vec<String>,
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
    /// `discharged` / `not_assumed` (a hypothesis), `claimed` (a named party
    /// claims it and nothing here establishes it), `withdrawn` (accepted for
    /// a period that ended), `superseded` (replaced, or withdrawn by the
    /// design it was mirrored from), `false` / `inconsistent`
    /// (a falsum derived under an assumption, or under none), or the proof
    /// status of a concluded one: `proven`, `shown`, `conditional`,
    /// `contested`, `unproven`.
    pub reads: &'static str,
    /// Why, in a sentence.
    pub why: String,
}

/// The change walk, whole: its rows, the objections it brings back and the
/// checks it leaves stale.
#[derive(Debug, Clone, Default)]
pub struct InferenceWalk {
    pub rows: Vec<InferenceImpact>,
    pub objections_returning: Vec<ObjectionReturning>,
    pub checks_gone_stale: Vec<StaleCheck>,
}

/// An objection a change brings back into force: the reply that answered it
/// is lost, so it counts again (reinstatement in reverse).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ObjectionReturning {
    /// The objection.
    pub objection: String,
    /// The reply that answered it, which the change takes away.
    pub answered_by: String,
    /// What it attacks again.
    pub against: Vec<String>,
    /// `refutes` (it holds again) or `stands` (it contests again).
    pub now: &'static str,
    /// Why, in a sentence.
    pub why: String,
    /// How far from a seed.
    pub distance: usize,
}

/// A check a change leaves stale, with each condition that no longer holds.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StaleCheck {
    /// The check.
    pub check: String,
    /// The conditions it was run against that no longer hold outright.
    pub conditions: Vec<ConditionReading>,
}

/// How a reading reads, in one word: `holds`, `conditional`, `contested`,
/// `unsupported` or `refuted`.
fn reads_as(r: &Read) -> &'static str {
    match r.truth {
        Truth::Holds
            if !r.assumes.is_empty() || !r.unverified.is_empty() || !r.bridges.is_empty() =>
        {
            "conditional"
        }
        Truth::Holds => "holds",
        Truth::Fails => "refuted",
        Truth::Unknown if !r.contested_by.is_empty() => "contested",
        Truth::Unknown => "unsupported",
    }
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
    /// THE BRIDGES among them: lost premises of kind `bridge`. Losing a
    /// bridge leaves the formal statement under it standing and the claim it
    /// carried that statement into unsupported through it
    /// (dec:idea-a-bridge-premise-says-a-formal-statement-models-a-claim).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bridges_lost: Vec<String>,
    /// Whether the CONCLUSION still stands with the losses counted, across
    /// every step that concludes it (alternative derivations are ANY).
    pub conclusion_after: Truth,
    /// WHICH IS TRUE OF THE CLAIM (req:unsupported-is-not-refuted):
    /// `still_supported` (another line holds it: `still_supported_by`),
    /// `contested` (no line holds it, and a dispute still holds one up),
    /// `unsupported` (nothing left holds it — it is not thereby false),
    /// `refuted` (an objection that holds refutes it), or, for an accepted
    /// Decision, `accepted_without_a_holding_chain` (it stands on its owner's
    /// acceptance, and no line of its chain holds any more). Since PR A2 also
    /// `claimed` (a named party still claims it: `claimed_by`) and `open` (it
    /// read claimed, and the change drops the claimant's argument).
    pub conclusion_reads: &'static str,
    /// THE NAMED PARTIES who still claim the conclusion with the losses
    /// counted. Empty when none does.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub claimed_by: Vec<String>,
    /// The steps concluding the claim that still hold with the losses counted.
    /// Empty unless `conclusion_reads` is `still_supported`.
    pub still_supported_by: Vec<String>,
    /// What the claim read BEFORE the change: `holds`, `conditional`,
    /// `contested`, `unsupported` or `refuted` — so a change to a claim that
    /// was already only contested says what it cost
    /// (req:a-what-if-shows-what-a-premise-costs-a-contested-conclusion-and-
    /// objections-that-return).
    pub conclusion_was: &'static str,
    /// When the claim is still supported: every line it still holds on, with
    /// what each holds under (req:a-what-if-on-an-assumed-principle-shows-
    /// what-holds-only-under-it). Empty otherwise.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub still_holds_under: Vec<LineConditions>,
    /// The claim's other routes the change does not reach at all — whatever
    /// each reads — so a reader can tell a route the change leaves alone from
    /// one it costs. Empty when there are none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub untouched_routes: Vec<String>,
    /// The refuted suppositions the step rests on after the change.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rests_on_refuted_suppositions: Vec<String>,
    /// WHICH PART STILL STANDS, for a step whose conclusion is made of its
    /// premises as parts (`conclusion_form`: a conjunction, a biconditional's
    /// two directions): each part and how it reads after the change
    /// (req:natural-deduction-gaps-from-the-logic-rung-are-closed, item 6).
    /// Empty for any other step.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<PartReading>,
    /// Every premise with its truth after the loss.
    pub premises: Vec<PremiseReading>,
}

/// One part of a conclusion made of its premises (a conjunct, one direction
/// of a biconditional), read after a change.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PartReading {
    /// The part.
    pub id: String,
    /// `stands`, `unsupported` or `refuted`.
    pub reads: &'static str,
}

/// One axiom a conclusion ultimately rests on (a terminal premise — not
/// concluded by any Inference). Requirement E.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Foundation {
    /// The axiom's id.
    pub id: String,
    /// Its type, or `None` when no node holds it.
    pub node_type: Option<String>,
    /// Its computed truth, as the step that takes it up reads it: its
    /// `premise_truth`, or its owner's intent when that step reads it so.
    pub truth: Truth,
    /// Why, in a sentence.
    pub why: String,
    /// `intent` or `delivery`, as [`PremiseReading::reads_as`]: absent when
    /// it is read as it always was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reads_as: Option<&'static str>,
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
    /// discharged one is in neither list. With several lines, the lightest
    /// line's; `holds_under_any_of` names every line.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assumes: Vec<String>,
    /// EVERY LINE it holds on, each with what it holds under, when there is
    /// more than one: it holds under any one of them, never only the first.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub holds_under_any_of: Vec<LineConditions>,
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
    /// What this evaluation computed with no circle cut beneath it, handed
    /// back whenever it is asked again, exactly as before readings were
    /// shared. Each with what it depends on, for the readings above it.
    memo: HashMap<String, Rc<Kept>>,
    /// The memo shared across the evaluations of one report, and what the
    /// computation under way has consulted ([`memo`]).
    track: Tracker,
    path: BTreeSet<String>,
    /// Nodes whose truth is being computed. Objections may form circles (A
    /// objects to B, B to A), which the write never refuses.
    nodes: BTreeSet<String>,
    /// How many times a circle was cut. A truth computed while a cut happened
    /// beneath it rests on a provisional `unknown`, so it is not memoized:
    /// asked again from elsewhere, it is computed again.
    cuts: usize,
    /// Every assumption some step discharges (`discharges` or `cases`), read
    /// once per evaluation: an OPEN supposition is one no step discharges.
    discharged: Option<BTreeSet<String>>,
}

impl<'l> Eval<'l> {
    fn new(graph: &DesignGraph, lost: &'l BTreeSet<String>) -> Self {
        Eval {
            lost,
            memo: HashMap::new(),
            track: Tracker::new(graph, lost),
            path: BTreeSet::new(),
            nodes: BTreeSet::new(),
            cuts: 0,
            discharged: None,
        }
    }
}

/// How many lines a reading keeps, each with what it holds under. A claim with
/// more minimal lines than this keeps the lightest [`LINES_CAP`] (the ones
/// resting on the fewest assumptions, citations and tests) and says so
/// (`lines_capped`): the bound exists for a design whose steps multiply lines
/// (five premises of five lines each are 3,125 combinations before the
/// supersets go), never silently.
pub const LINES_CAP: usize = 32;

/// How many combinations of `k` premises a `k_of_n` step's lines are read
/// over before every holding premise is taken together instead (and the
/// reading says it was capped).
pub const K_OF_N_COMBINATIONS_CAP: usize = 256;

/// WHAT A NODE HOLDS UNDER ON ONE LINE OF SUPPORT: the open assumptions, the
/// unverified citations and the statistical tests that line rests on
/// (req:a-claim-with-several-lines-reads-each-lines-assumptions, Anthony
/// 2026-10-19). A claim supported by several lines holds under ANY ONE of
/// them; a step holds under the union of what each premise's chosen line
/// needs, choosing per premise the line that costs least.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Condition {
    assumes: BTreeSet<String>,
    unverified: BTreeSet<String>,
    /// THE BRIDGES the line rests on: statements of kind `bridge`, each saying
    /// a formal statement models a claim of another field
    /// (dec:idea-a-bridge-premise-says-a-formal-statement-models-a-claim,
    /// option b). Carried like an unverified citation — what rests on one
    /// holds only if the modelling is faithful — and never discharged.
    bridges: BTreeSet<String>,
    evidential: BTreeSet<String>,
}

impl Condition {
    fn union(&self, other: &Condition) -> Condition {
        Condition {
            assumes: self.assumes.union(&other.assumes).cloned().collect(),
            unverified: self.unverified.union(&other.unverified).cloned().collect(),
            bridges: self.bridges.union(&other.bridges).cloned().collect(),
            evidential: self.evidential.union(&other.evidential).cloned().collect(),
        }
    }

    /// Whether this line needs nothing `other` does not: then `other` is
    /// never the cheaper line, and only this one is kept.
    fn within(&self, other: &Condition) -> bool {
        self.assumes.is_subset(&other.assumes)
            && self.unverified.is_subset(&other.unverified)
            && self.bridges.is_subset(&other.bridges)
            && self.evidential.is_subset(&other.evidential)
    }

    /// What every line in `lines` holds under: their intersection.
    fn common(lines: &[Condition]) -> Condition {
        let mut it = lines.iter();
        let Some(first) = it.next() else {
            return Condition::default();
        };
        it.fold(first.clone(), |acc, l| Condition {
            assumes: acc.assumes.intersection(&l.assumes).cloned().collect(),
            unverified: acc
                .unverified
                .intersection(&l.unverified)
                .cloned()
                .collect(),
            bridges: acc.bridges.intersection(&l.bridges).cloned().collect(),
            evidential: acc
                .evidential
                .intersection(&l.evidential)
                .cloned()
                .collect(),
        })
    }

    fn weight(&self) -> (usize, usize, usize, usize) {
        (
            self.assumes.len(),
            self.unverified.len(),
            self.bridges.len(),
            self.evidential.len(),
        )
    }
}

/// The MINIMAL lines of `lines`: a line that needs everything another needs
/// and more is never the one a reader rests on, so it goes. Sorted, stably,
/// lightest first — fewest open assumptions, then unverified citations, then
/// tests, the order a holding line was always chosen in — and kept to
/// [`LINES_CAP`]; the flag says whether any were left out.
fn minimal_lines(lines: Vec<Condition>) -> (Vec<Condition>, bool) {
    let mut sorted = lines;
    sorted.sort_by_key(Condition::weight);
    let mut kept: Vec<Condition> = Vec::new();
    for l in sorted {
        if !kept.iter().any(|k| k.within(&l)) {
            kept.push(l);
        }
    }
    let capped = kept.len() > LINES_CAP;
    kept.truncate(LINES_CAP);
    if kept.is_empty() {
        kept.push(Condition::default());
    }
    (kept, capped)
}

/// Every way of taking one line from each of `parts` together: what a step
/// over all of them holds under, line by line, minimal.
fn cross_lines(parts: &[&Vec<Condition>]) -> (Vec<Condition>, bool) {
    let mut acc: Vec<Condition> = vec![Condition::default()];
    let mut capped = false;
    for part in parts {
        let mut next = Vec::with_capacity(acc.len() * part.len().max(1));
        for a in &acc {
            for l in part.iter() {
                next.push(a.union(l));
            }
        }
        let (kept, c) = minimal_lines(next);
        capped |= c;
        acc = kept;
    }
    (acc, capped)
}

/// One line of support as a reading serves it: what that line holds under.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LineConditions {
    /// The open assumptions this line rests on: `assumed` premises beneath it
    /// that no step on the way discharges.
    pub assumes: Vec<String>,
    /// The unverified citations this line rests on.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rests_on_unverified: Vec<String>,
    /// The bridges this line rests on: statements of kind `bridge`, each
    /// saying a formal statement models a claim of another field.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rests_on_bridges: Vec<String>,
    /// The statistical tests this line rests on.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rests_on_tests: Vec<String>,
}

impl LineConditions {
    fn of(c: &Condition) -> Self {
        LineConditions {
            assumes: c.assumes.iter().cloned().collect(),
            rests_on_unverified: c.unverified.iter().cloned().collect(),
            rests_on_bridges: c.bridges.iter().cloned().collect(),
            rests_on_tests: c.evidential.iter().cloned().collect(),
        }
    }

    /// The line in words: "con:a", "con:a and con:b, if cit:x is right".
    fn said(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if !self.assumes.is_empty() {
            parts.push(format!("under {}", self.assumes.join(" and ")));
        }
        if !self.rests_on_unverified.is_empty() {
            parts.push(format!(
                "if the unverified citation(s) {} say what they are recalled to say",
                self.rests_on_unverified.join(", ")
            ));
        }
        if !self.rests_on_bridges.is_empty() {
            parts.push(format!(
                "if the bridge(s) {} model what they are taken to model",
                self.rests_on_bridges.join(", ")
            ));
        }
        if !self.rests_on_tests.is_empty() {
            parts.push(format!(
                "through the evidence {} (a statistical test or a report)",
                self.rests_on_tests.join(", ")
            ));
        }
        if parts.is_empty() {
            "outright".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// One reading of a claim, every part of it: see
/// [`DesignGraph::claim_parts`].
pub(crate) struct ClaimParts {
    pub truth: Truth,
    pub why: String,
    pub assumes: Vec<String>,
    pub unverified: Vec<String>,
    pub bridges: Vec<String>,
    pub tests: Vec<String>,
    pub lines: Vec<LineConditions>,
    pub lines_capped: bool,
    pub contested_by: Vec<String>,
}

/// Several lines in words: "under con:a; or under con:b; or under con:c".
pub fn lines_said(lines: &[LineConditions]) -> String {
    lines
        .iter()
        .map(LineConditions::said)
        .collect::<Vec<_>>()
        .join("; or ")
}

/// A node's truth as one evaluation reads it: the value, why, and the
/// ASSUMPTIONS it holds under — every `assumed` premise beneath it that no
/// step on the way discharges (empty for almost every node).
///
/// `assumes`, `unverified` and `evidential` are the LIGHTEST line's: the one a
/// reader rests on when they rest on the least. `lines` holds every minimal
/// line, lightest first (`lines[0]` is those three), so a claim with several
/// lines is never read as holding under only the first
/// (req:a-claim-with-several-lines-reads-each-lines-assumptions).
#[derive(Debug, Clone)]
struct Read {
    truth: Truth,
    why: String,
    lines: Vec<Condition>,
    /// Whether more minimal lines exist than [`LINES_CAP`] keeps.
    lines_capped: bool,
    assumes: BTreeSet<String>,
    /// THE UNVERIFIED CITATIONS it rests on: results cited from elsewhere
    /// (taken as given) that no pinned source documents. Carried like an
    /// assumption — what rests on one holds only if it says what it is
    /// recalled to say — but never discharged by a step: checking its source
    /// is what clears it.
    unverified: BTreeSet<String>,
    /// THE BRIDGES it rests on (statement_kind `bridge`): carried like an
    /// unverified citation, never discharged — what rests on one holds only
    /// if the formal statement models the claim faithfully
    /// (dec:idea-a-bridge-premise-says-a-formal-statement-models-a-claim).
    bridges: BTreeSet<String>,
    /// THE STATISTICAL TESTS it rests on ([`crate::hypothesis`]): checks whose
    /// support is a hypothesis test that rejected H0. Carried like an
    /// unverified citation and never discharged: what rests on one is
    /// SUPPORTED AT α, evidence and never proof, however its steps are
    /// confirmed (req:a-hypothesis-tests-verdict-reads-as-supported-at-alpha-
    /// never-as-shown).
    evidential: BTreeSet<String>,
    /// WHAT KEEPS IT FROM HOLDING, WHEN A DISPUTE DOES: the objections that
    /// stand against it, against a step under it or against a premise it
    /// rests on, and the `contested` Decisions it rests on — so a claim whose
    /// every route is held up only by standing objections reads `contested`,
    /// never `unproven`, a word that also covers a claim nothing supports
    /// (req:a-conclusion-whose-every-route-is-contested-reads-contested).
    /// Empty for a node that holds, or is unknown for any other reason.
    contested_by: BTreeSet<String>,
    /// THE REFUTED SUPPOSITIONS IT WAS DERIVED UNDER: open suppositions (never
    /// discharged) a step beneath it takes up as `assumed` while they read
    /// refuted outside (req:a-claim-derived-under-a-refuted-supposition-says-
    /// so). What rests on one is no live conditional support and no standing
    /// objection. Empty almost always.
    refuted_under: BTreeSet<String>,
}

/// A reference's reading as a premise here: the far node's truth when it was
/// read, with what it holds only under carried as unverified citations, so a
/// conclusion resting on a conditional far result reads conditional; unknown
/// for a stale or unreachable far end, whatever it once read.
fn far_read(far: &crate::far::FarReading) -> Read {
    use crate::far::FarState;
    match far.state {
        FarState::Read => Read::one(
            far.truth,
            format!(
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
            Condition {
                assumes: BTreeSet::new(),
                unverified: far.holds_only_if.iter().cloned().collect(),
                bridges: BTreeSet::new(),
                // A far claim that is only SUPPORTED there (it rests on a
                // statistical test) is evidence here too.
                evidential: if far.proof_status.as_deref() == Some("supported") {
                    BTreeSet::from([format!("xref:{}:{}", far.design, far.node_id)])
                } else {
                    BTreeSet::new()
                },
            },
        ),
        FarState::Stale | FarState::Unreachable => Read::plain(Truth::Unknown, far.why.clone()),
    }
}

impl Read {
    fn plain(truth: Truth, why: String) -> Self {
        Read::one(truth, why, Condition::default())
    }

    /// A reading with ONE line, holding under `cond`.
    fn one(truth: Truth, why: String, cond: Condition) -> Self {
        Read::with_lines(truth, why, vec![cond], false)
    }

    /// A reading with every minimal line it holds under, lightest first.
    fn with_lines(truth: Truth, why: String, lines: Vec<Condition>, lines_capped: bool) -> Self {
        let lines = if lines.is_empty() {
            vec![Condition::default()]
        } else {
            lines
        };
        let best = lines[0].clone();
        Read {
            truth,
            why,
            lines,
            lines_capped,
            assumes: best.assumes,
            unverified: best.unverified,
            bridges: best.bridges,
            evidential: best.evidential,
            contested_by: BTreeSet::new(),
            refuted_under: BTreeSet::new(),
        }
    }

    /// This reading, carried THROUGH the bridge `id`: when it holds, every
    /// line it holds on rests on the bridge too.
    fn through_bridge(mut self, id: &str) -> Self {
        if self.truth == Truth::Holds {
            for l in &mut self.lines {
                l.bridges.insert(id.to_string());
            }
            self.bridges.insert(id.to_string());
        }
        self
    }

    /// This reading, unknown because it was derived under the refuted
    /// suppositions `under`.
    fn under_refuted(mut self, under: BTreeSet<String>) -> Self {
        self.refuted_under = under;
        self
    }

    /// This reading, unknown only because a dispute holds it up: `by` stand
    /// against it or against what it rests on.
    fn contested(mut self, by: BTreeSet<String>) -> Self {
        self.contested_by = by;
        self
    }

    /// What EVERY line holds under: the scope an objection is judged against.
    /// An objection holding only under S refutes a claim only when the claim
    /// holds under S on every line; a claim that also holds outside S (on
    /// another line) is contested by it, never refuted.
    fn scope(&self) -> Condition {
        Condition::common(&self.lines)
    }

    /// The lines as a reading serves them, when there is more than one:
    /// empty for a node with one line, which `assumes` already says. Only a
    /// node whose own support holds carries several — a contested one keeps
    /// the lines it holds on before the objection is counted, so a reader
    /// sees what each side rests on while the dispute stands.
    fn served_lines(&self) -> Vec<LineConditions> {
        if self.lines.len() > 1 {
            self.lines.iter().map(LineConditions::of).collect()
        } else {
            Vec::new()
        }
    }

    fn pair(self) -> (Truth, String) {
        (self.truth, self.why)
    }
}

/// How one record reads refuted: what it reads (`refuted`, `defeated`,
/// `conceded`), what holds against it, and why.
type RefutedAs = (&'static str, Vec<String>, String);

/// A check's conditions, read: see [`DesignGraph::check_conditions`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct CheckConditions {
    /// Whether one of them no longer holds outright.
    pub stale: bool,
    /// Why, in a sentence.
    pub why: String,
    /// Each condition, read.
    pub conditions: Vec<ConditionReading>,
}

/// One condition a check was run against, read now.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ConditionReading {
    /// The claim that constrains the check.
    pub id: String,
    /// `holds`, `conditional`, `contested`, `unsupported` or `refuted`.
    pub reads: &'static str,
    /// Why, in a sentence.
    pub why: String,
}

/// How the watched records read before a write: see
/// [`DesignGraph::watch_refutations`].
#[derive(Debug, Clone, Default)]
pub struct RefutationWatch {
    before: BTreeMap<String, Option<&'static str>>,
}

/// A record a write turned refuted, defeated or conceded — served in the
/// write's reply as `now_refuted`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NowRefuted {
    /// The record.
    pub id: String,
    /// `refuted` (a claim an objection that holds refutes), `defeated` (a
    /// step an objection that holds defeats) or `conceded` (granted by a
    /// reply).
    pub reads: &'static str,
    /// The objections that hold against it.
    pub by: Vec<String>,
    /// Why, in a sentence.
    pub why: String,
    /// For a claim: how to keep it and its objector as an open dispute
    /// instead, if that is what was meant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_as_dispute: Option<String>,
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
    /// THE REPLY THAT GRANTS IT (req:a-reply-can-concede-an-objection-
    /// against-one-reading-and-move-to-another): a CONTRADICTS marked
    /// `alignment: conceding` from a reply that holds. The objection then
    /// counts against what it attacks as if it held — `standing: conceded` —
    /// by the arguer's own admission. Absent when nothing concedes it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conceded_by: Option<String>,
    /// Where that reply moves the argument instead: the conceding edge's
    /// `moves_to`, the reading the argument is meant as. Absent when the
    /// concession names none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moves_to: Option<String>,
}

/// Whether an objection's standing counts against its target as holding:
/// one that `refutes`, or one a reply concedes.
pub fn counts_as_holding(standing: &str) -> bool {
    standing == "refutes" || standing == "conceded"
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
    rule_node: Option<String>,
    establishes: Option<String>,
    conclusion_form: Option<String>,
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

/// The statement kinds `statement_kind` may say. `objection` to
/// `method_rule` are the roles a line of reasoning gives its statements
/// (req:statement-kinds-cover-an-objection-a-reply-an-unstated-premise-and-a-method-rule);
/// `evidence` and `judgement` are reasoning from uncertain evidence
/// (req:evidence-carries-its-source-grade-and-confidence-and-is-not-intent);
/// `bridge` is a translation premise carrying a formal statement into another
/// field's claim (dec:idea-a-bridge-premise-says-a-formal-statement-models-a-
/// claim, option b).
pub const STATEMENT_KINDS: [&str; 16] = [
    "theorem",
    "lemma",
    "corollary",
    "proposition",
    "definition",
    "axiom",
    "conjecture",
    "hypothesis",
    "falsum",
    "objection",
    "reply",
    "unstated_premise",
    "method_rule",
    "evidence",
    "judgement",
    "bridge",
];

/// Whether a node is EVIDENCE: a report, study, survey or measurement
/// (`statement_kind: evidence`) — "the report says X". It holds AS EVIDENCE:
/// what rests on it reads supported, never shown or proven
/// (req:evidence-carries-its-source-grade-and-confidence-and-is-not-intent).
pub fn is_evidence(props: &HashMap<String, Value>) -> bool {
    statement_kind(props) == Some("evidence")
}

/// THE GRADE A RECORD CARRIES, as fields (req:evidence-carries-its-source-
/// grade-and-confidence-and-is-not-intent): a piece of evidence's source
/// reliability, information credibility and date, an analytic judgement's
/// confidence. The owner's grades, read back; none changes a truth.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct Grade {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_reliability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub information_credibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reported_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
}

/// A record's [`Grade`], when it carries any of the four fields.
pub fn grade_of(props: &HashMap<String, Value>) -> Option<Grade> {
    let g = Grade {
        source_reliability: prop(props, "source_reliability").map(str::to_string),
        information_credibility: prop(props, "information_credibility").map(str::to_string),
        reported_at: prop(props, "reported_at").map(str::to_string),
        confidence: prop(props, "confidence").map(str::to_string),
    };
    (g != Grade::default()).then_some(g)
}

/// A piece of evidence's grade and an analytic judgement's confidence, in
/// words — "source reliability B, information credibility 2, reported 2025",
/// "moderate confidence" — when it carries any. Read back; it changes no
/// truth.
pub fn grade_said(props: &HashMap<String, Value>) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(r) = prop(props, "source_reliability") {
        parts.push(format!("source reliability {r}"));
    }
    if let Some(c) = prop(props, "information_credibility") {
        parts.push(format!("information credibility {c}"));
    }
    if let Some(d) = prop(props, "reported_at") {
        parts.push(format!("reported {d}"));
    }
    if let Some(c) = prop(props, "confidence") {
        parts.push(format!("{c} confidence"));
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}

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
/// A statement whose `statement_kind` is `definition`, `axiom` or
/// `method_rule` (a rule of method the reasoning follows) is taken as given
/// too, unless it says `taken_as_given: false` in so many words.
pub fn is_taken_as_given(props: &HashMap<String, Value>) -> bool {
    match props.get("taken_as_given").and_then(Value::as_bool) {
        Some(said) => said,
        None => matches!(
            prop(props, "statement_kind"),
            Some("definition" | "axiom" | "method_rule")
        ),
    }
}

/// Whether a source Artifact is marked NAMED, NOT READ (`not_read: true`):
/// named in a packet or a text, and nobody read it. It never counts as citing
/// (dec:idea-a-source-says-what-kind-of-support-it-gives, settled 2026-10-23).
pub fn is_not_read(props: &HashMap<String, Value>) -> bool {
    props.get("not_read").and_then(Value::as_bool) == Some(true)
}

/// Why a source's GRADE is too weak to check a citation against, when it is:
/// evidence's source reliability D–F (not usually reliable, unreliable,
/// cannot be judged) or information credibility 4–6 (doubtful, improbable,
/// cannot be judged), reused for a source (option (c) of
/// dec:idea-a-source-says-what-kind-of-support-it-gives). A citation it
/// documents then reads no stronger than that: unverified. `None` for an
/// ungraded source or one graded A–C and 1–3.
pub fn source_grade_too_weak(props: &HashMap<String, Value>) -> Option<String> {
    let reliability = prop(props, "source_reliability").map(str::trim);
    let credibility = prop(props, "information_credibility").map(str::trim);
    let weak_r = reliability.is_some_and(|r| {
        matches!(
            r.chars().next().map(|c| c.to_ascii_uppercase()),
            Some('D' | 'E' | 'F')
        )
    });
    let weak_c = credibility.is_some_and(|c| matches!(c.chars().next(), Some('4' | '5' | '6')));
    if !(weak_r || weak_c) {
        return None;
    }
    let mut said: Vec<String> = Vec::new();
    if let Some(r) = reliability {
        said.push(format!("source reliability {r}"));
    }
    if let Some(c) = credibility {
        said.push(format!("information credibility {c}"));
    }
    Some(format!(
        "graded {}: too weak to check a citation against, so what it documents reads no \
         stronger than unverified",
        said.join(", ")
    ))
}

/// The NAMED PARTY a record says claims it (`claimed_by`), when it says one:
/// "OpenAI", "Kempe". A claim nobody here has verified
/// (req:a-claim-made-by-a-named-party-and-not-yet-verified-reads-claimed).
pub fn claimed_by(props: &HashMap<String, Value>) -> Option<&str> {
    prop(props, "claimed_by")
        .map(str::trim)
        .filter(|v| !v.is_empty())
}

/// The surface that WITHDREW a mirrored node, when one did
/// (`superseded_by_surface`, written by a refreshed `mirror_surface`).
pub fn superseded_by_surface(props: &HashMap<String, Value>) -> Option<&str> {
    prop(props, "superseded_by_surface")
}

/// One status a record held OVER A PERIOD: a dated `TemporalFact` with
/// `fact_type: status` about it, read back
/// (req:a-claim-made-by-a-named-party-and-not-yet-verified-reads-claimed:
/// "a status that held over a time is read back, not left in dated notes
/// nobody reads").
#[derive(Debug, Clone, serde::Serialize)]
pub struct StatusPeriod {
    /// The dated record.
    pub fact: String,
    /// What it says, in its own words (its name, else its statement).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub says: Option<String>,
    /// When the status began (`valid_from`), as recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// When it ended (`valid_to`); absent while it still holds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
}

impl StatusPeriod {
    /// The period in words: "1879–1890", "since 1976".
    pub fn period(&self) -> String {
        match (&self.from, &self.until) {
            (Some(f), Some(u)) => format!("{f}–{u}"),
            (Some(f), None) => format!("since {f}"),
            (None, Some(u)) => format!("until {u}"),
            (None, None) => "undated".to_string(),
        }
    }
}

/// A claim ACCEPTED FOR A PERIOD AND WITHDRAWN: it does not hold now, and the
/// last status it held has ended. What withdrew it is what stands against it
/// or a step under it now.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WithdrawnReading {
    /// The period it held: "1879–1890".
    pub held: String,
    /// The status record that says so.
    pub fact: String,
    /// What that record says.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub says: Option<String>,
    /// What withdrew it: each objection standing against it, or against a
    /// step it rests on, that is not answered.
    pub withdrawn_by: Vec<String>,
    /// In a sentence.
    pub why: String,
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

    /// Each premise of one step whose link says what it is read as
    /// (PREMISE_OF `reads_as`), by id.
    fn premise_marks(&self, inference_id: &str) -> Result<BTreeMap<String, String>, DynoError> {
        Ok(self
            .incoming(inference_id, Some(edge::PREMISE_OF))?
            .into_iter()
            .filter_map(|e| {
                let mark = e.properties.get("reads_as").and_then(Value::as_str)?;
                READS_AS
                    .contains(&mark)
                    .then(|| (e.from_id.clone(), mark.to_string()))
            })
            .collect())
    }

    /// Whether `id` carries the two propositions a premise can rest on — its
    /// owner's intent and its delivery: a Requirement that is a NEED (no
    /// `statement_kind`, not taken as given), a Capability or an Interface.
    /// A statement of a proof or an argument reads as itself, always.
    pub fn intent_applies_to(&self, id: &str) -> Result<bool, DynoError> {
        Ok(match self.type_holding(id)?.as_deref() {
            Some(node::REQUIREMENT) => self.node_props(id)?.is_some_and(|props| {
                statement_kind(&props).is_none_or(|k| k.trim().is_empty())
                    && !is_taken_as_given(&props)
            }),
            Some(node::CAPABILITY | node::INTERFACE) => true,
            _ => false,
        })
    }

    /// What the step `inference_id` reads its ordinary premise `premise` as:
    /// the premise's own `reads_as` when it says, and otherwise INTENT when
    /// the step concludes a Decision and DELIVERY for every other step.
    /// `None` for a premise the marker does not apply to: it reads as itself.
    pub fn premise_reads_as(
        &self,
        inference_id: &str,
        premise: &str,
    ) -> Result<Option<ReadsAs>, DynoError> {
        let marks = self.premise_marks(inference_id)?;
        let concludes_a_decision = self.concludes_a_decision(inference_id)?;
        self.reads_as_with(
            premise,
            marks.get(premise).map(String::as_str),
            concludes_a_decision,
        )
    }

    fn concludes_a_decision(&self, inference_id: &str) -> Result<bool, DynoError> {
        Ok(match self.conclusion_of(inference_id)? {
            Some(c) => self.type_holding(&c)?.as_deref() == Some(node::DECISION),
            None => false,
        })
    }

    fn reads_as_with(
        &self,
        premise: &str,
        mark: Option<&str>,
        concludes_a_decision: bool,
    ) -> Result<Option<ReadsAs>, DynoError> {
        if !self.intent_applies_to(premise)? {
            return Ok(None);
        }
        Ok(Some(match mark {
            Some("intent") => ReadsAs::Intent,
            Some("delivery") => ReadsAs::Delivery { marked: true },
            _ if concludes_a_decision => ReadsAs::Intent,
            _ => ReadsAs::Delivery { marked: false },
        }))
    }

    /// A premise's truth AS THE STEP THAT TAKES IT UP READS IT: its owner's
    /// intent, or [`Self::premise_truth`]; with what it was read as.
    pub fn premise_truth_in_step(
        &self,
        inference_id: &str,
        premise: &str,
    ) -> Result<(Truth, String, Option<ReadsAs>), DynoError> {
        let reads_as = self.premise_reads_as(inference_id, premise)?;
        if reads_as == Some(ReadsAs::Intent) {
            let lost = BTreeSet::new();
            let mut ev = Eval::new(self, &lost);
            let conclusion = self.conclusion_of(inference_id)?;
            let (t, why) = self
                .intent_full(premise, conclusion.as_deref(), &mut ev, INFERENCE_DEPTH)?
                .pair();
            return Ok((t, why, reads_as));
        }
        let (t, why) = self.premise_truth(premise)?;
        Ok((t, why, reads_as))
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
            "immediate" => Ok(InferenceRule::Immediate),
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
            rule_node: prop(&n.properties, "rule_node")
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            establishes: prop(&n.properties, "establishes").map(str::to_string),
            conclusion_form: prop(&n.properties, "conclusion_form").map(str::to_string),
        }))
    }

    /// The one type holding `id`, when exactly one does.
    pub(crate) fn type_holding(&self, id: &str) -> Result<Option<String>, DynoError> {
        memo::type_held(self, id, || {
            let held = self.node_types_holding(id)?;
            Ok(if held.len() == 1 {
                held.into_iter().next()
            } else {
                None
            })
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
        let mut ev = Eval::new(self, &lost);
        self.truth_full(id, &mut ev, INFERENCE_DEPTH)
    }

    /// The ASSUMPTIONS `id` holds under, as the design stands: every `assumed`
    /// premise beneath it that no step on the way discharges, sorted. Empty
    /// for a node that rests on no assumption, or whose every assumption is
    /// discharged on the way to it.
    pub fn assumptions_under(&self, id: &str) -> Result<Vec<String>, DynoError> {
        Ok(self.premise_reading(id)?.assumes.into_iter().collect())
    }

    /// Everything one reading of `id` says, read ONCE: its truth and why,
    /// the lightest line's assumptions, unverified citations and tests, and
    /// every line when there are several. What the read-back of one claim
    /// needs, without evaluating the claim five times.
    pub(crate) fn claim_parts(&self, id: &str) -> Result<ClaimParts, DynoError> {
        let r = self.premise_reading(id)?;
        Ok(ClaimParts {
            lines: r.served_lines(),
            lines_capped: r.lines_capped,
            truth: r.truth,
            assumes: r.assumes.into_iter().collect(),
            unverified: r.unverified.into_iter().collect(),
            bridges: r.bridges.into_iter().collect(),
            tests: r.evidential.into_iter().collect(),
            contested_by: r.contested_by.into_iter().collect(),
            why: r.why,
        })
    }

    /// EVERY LINE `id` holds on, each with what it holds under, lightest
    /// first, when it holds on more than one minimal line; and whether more
    /// lines exist than [`LINES_CAP`] keeps. Empty when it holds on one line
    /// (its `assumes` says that one) or does not hold
    /// (req:a-claim-with-several-lines-reads-each-lines-assumptions).
    pub fn lines_under(&self, id: &str) -> Result<(Vec<LineConditions>, bool), DynoError> {
        let r = self.premise_reading(id)?;
        Ok((r.served_lines(), r.lines_capped))
    }

    /// The UNVERIFIED CITATIONS `id` rests on, as the design stands: results
    /// cited from elsewhere, beneath it or itself, that no pinned source
    /// documents, sorted. Empty when it rests on none.
    pub fn unverified_under(&self, id: &str) -> Result<Vec<String>, DynoError> {
        Ok(self.premise_reading(id)?.unverified.into_iter().collect())
    }

    /// THE BRIDGES `id` rests on, as the design stands: statements of kind
    /// `bridge`, beneath it or itself, sorted. Empty when it rests on none.
    pub fn bridges_under(&self, id: &str) -> Result<Vec<String>, DynoError> {
        Ok(self.premise_reading(id)?.bridges.into_iter().collect())
    }

    // ─── A write that refutes says so ──────────────────────────────────────

    /// The records a CONTRADICTS from `id` reaches as an attack: each opposing
    /// target, and for a concession the targets of the objection it grants —
    /// what a write touching `id` could turn refuted.
    pub fn attack_targets_of(&self, id: &str) -> Result<Vec<String>, DynoError> {
        let mut out: BTreeSet<String> = BTreeSet::new();
        for e in self.outgoing(id, Some(edge::CONTRADICTS))? {
            match e.properties.get("alignment").and_then(Value::as_str) {
                Some("supporting") => {}
                Some("conceding") => {
                    out.extend(self.attack_targets_of_objection(&e.to_id)?);
                }
                _ => {
                    out.insert(e.to_id);
                }
            }
        }
        Ok(out.into_iter().collect())
    }

    /// The opposing targets of an objection, one hop.
    pub fn attack_targets_of_objection(&self, objection: &str) -> Result<Vec<String>, DynoError> {
        let mut out: Vec<String> = self
            .outgoing(objection, Some(edge::CONTRADICTS))?
            .into_iter()
            .filter(|e| {
                !matches!(
                    e.properties.get("alignment").and_then(Value::as_str),
                    Some("supporting" | "conceding")
                )
            })
            .map(|e| e.to_id)
            .collect();
        out.sort();
        out.dedup();
        Ok(out)
    }

    /// Note how each of `ids` reads now, before a write, so
    /// [`Self::refuted_since`] can say which the write turned refuted.
    pub fn watch_refutations(
        &self,
        ids: impl IntoIterator<Item = String>,
    ) -> Result<RefutationWatch, DynoError> {
        let mut before = BTreeMap::new();
        for id in ids {
            if before.contains_key(&id) {
                continue;
            }
            let state = self.refutation_state(&id)?.map(|(reads, _, _)| reads);
            before.insert(id, state);
        }
        Ok(RefutationWatch { before })
    }

    /// Every watched record a write turned REFUTED (a claim), or DEFEATED or
    /// CONCEDED (a step), that did not read so before it
    /// (req:a-write-that-makes-a-claim-read-refuted-says-so-in-its-reply).
    /// Measured in the cogito trial's AI branch: one CONTRADICTS between rival
    /// verdicts made one read refuted, and nothing said so.
    pub fn refuted_since(&self, watch: &RefutationWatch) -> Result<Vec<NowRefuted>, DynoError> {
        let mut out = Vec::new();
        for (id, was) in &watch.before {
            let Some((reads, by, why)) = self.refutation_state(id)? else {
                continue;
            };
            if *was == Some(reads) {
                continue;
            }
            let step = self.get_node(node::INFERENCE, id)?.is_some();
            let keep_as_dispute = (!step).then(|| {
                format!(
                    "If {id} and {} are rival positions in an open dispute rather than a \
                     refutation, draw a CONTRADICTS back from {id} to {} as well: both then read \
                     contested and neither refuted. Or keep both under a Decision the owner sets \
                     `contested`.",
                    by.join(", "),
                    by.join(" and to ")
                )
            });
            out.push(NowRefuted {
                id: id.clone(),
                reads,
                by,
                why,
                keep_as_dispute,
            });
        }
        Ok(out)
    }

    /// `Some((reads, by, why))` when `id` reads refuted, defeated or conceded
    /// by objections that hold against it: for a step its `objected`, for any
    /// other record its truth failing through an objection.
    fn refutation_state(&self, id: &str) -> Result<Option<RefutedAs>, DynoError> {
        if let Some(r) = self.inference_reading(id)? {
            let Some(reads @ ("defeated" | "conceded")) = r.objected else {
                return Ok(None);
            };
            let by = r
                .objections
                .iter()
                .filter(|o| counts_as_holding(o.standing))
                .map(|o| o.id.clone())
                .collect();
            return Ok(Some((reads, by, r.note.unwrap_or_default())));
        }
        let (truth, why) = self.premise_truth(id)?;
        if truth != Truth::Fails {
            return Ok(None);
        }
        let holding: Vec<ObjectionReading> = self
            .objections_to(id)?
            .into_iter()
            .filter(|o| counts_as_holding(o.standing))
            .collect();
        if holding.is_empty() {
            return Ok(None);
        }
        let reads = if holding.iter().all(|o| o.standing == "conceded") {
            "conceded"
        } else {
            "refuted"
        };
        Ok(Some((
            reads,
            holding.into_iter().map(|o| o.id).collect(),
            why,
        )))
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
        if let Some(hit) = ev.memo.get(id).cloned() {
            ev.track.took(id, &hit);
            return Ok(hit.read.clone());
        }
        if ev.nodes.contains(id) {
            ev.cuts += 1;
            ev.track.found_on(id, Stack::Node);
            return Ok(Read::plain(
                Truth::Unknown,
                "it comes back to itself, through its support or a circle of objections, and a \
                 circle has no verdict"
                    .to_string(),
            ));
        }
        // ASKED BEFORE, ON A STACK THAT AGREES: computing it again would take
        // the same path and read the same (memo.rs). A cut beneath it is
        // counted as computing it again would count it, so what rests on it
        // is kept in this evaluation's own memo exactly as before.
        let stacks = Stacks {
            nodes: &ev.nodes,
            path: &ev.path,
        };
        if let Some(hit) = ev.track.recall(id, &stacks) {
            ev.track.took(id, &hit);
            if hit.had_cuts {
                ev.cuts += 1;
            } else {
                ev.memo.insert(id.to_string(), hit.clone());
            }
            return Ok(hit.read.clone());
        }
        ev.nodes.insert(id.to_string());
        ev.track.frames.push(memo::Frame::default());
        let cuts_before = ev.cuts;
        let truncations_before = memo::truncations();
        let answer = self.truth_uncached(id, ev, depth);
        ev.nodes.remove(id);
        let frame = ev.track.frames.pop().unwrap_or_default();
        let answer = answer?;
        let stacks = Stacks {
            nodes: &ev.nodes,
            path: &ev.path,
        };
        let kept = ev.track.keep(
            id,
            answer.clone(),
            frame,
            ev.cuts != cuts_before,
            memo::truncations() != truncations_before,
            &stacks,
        );
        if ev.cuts == cuts_before {
            ev.memo.insert(id.to_string(), kept);
        }
        Ok(answer)
    }

    fn truth_uncached(&self, id: &str, ev: &mut Eval<'_>, depth: usize) -> Result<Read, DynoError> {
        if depth == 0 {
            note_truncation();
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
                Some(note) if matches!(r.objected, Some("defeated" | "conceded")) => {
                    format!("a step defeated by an objection to the step itself: {note}")
                }
                Some(note) if r.verdict == Truth::Unknown => {
                    format!("a step whose verdict is unknown: {note}")
                }
                _ if r.rule == "immediate" => format!(
                    "not an inference: an immediate grasp (rule `immediate`), which {}",
                    r.verdict.as_str()
                ),
                _ => format!("a step whose rule `{}` {}", r.rule, r.verdict.as_str()),
            };
            let contested: BTreeSet<String> = r.contested_by.iter().cloned().collect();
            let refuted: BTreeSet<String> =
                r.rests_on_refuted_suppositions.iter().cloned().collect();
            return Ok(Read::with_lines(r.verdict, why, r.lines, r.lines_capped)
                .contested(contested)
                .under_refuted(refuted));
        }
        self.node_reading(id, &node_type, ev, depth, None)
    }

    /// A premise read as its OWNER'S INTENT
    /// (dec:a-premise-says-whether-it-rests-on-intent-or-delivery): a
    /// requirement, capability or interface by its acceptance, never by
    /// whether it is delivered. Everything else about the reading is the
    /// ordinary one — a node a what-if takes away is lost, a discontinued,
    /// withdrawn or replaced one is read so, and an objection counts against
    /// it as far as it holds. `conclusion` is what the step taking it up
    /// concludes: a Decision never lends acceptance to a premise of its own
    /// step. Read per step, so never memoized.
    fn intent_full(
        &self,
        id: &str,
        conclusion: Option<&str>,
        ev: &mut Eval<'_>,
        depth: usize,
    ) -> Result<Read, DynoError> {
        if ev.lost.contains(id) {
            let (t, why) = self.lost_truth(id)?;
            return Ok(Read::plain(t, why));
        }
        if depth == 0 {
            note_truncation();
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
        self.node_reading(id, &node_type, ev, depth, Some(conclusion))
    }

    /// One node's reading, objections counted: by its own predicate
    /// (`intent: None`), or, read as its owner's intent, by its acceptance
    /// (`intent: Some(conclusion of the step taking it up)`).
    fn node_reading(
        &self,
        id: &str,
        node_type: &str,
        ev: &mut Eval<'_>,
        depth: usize,
        intent: Option<Option<&str>>,
    ) -> Result<Read, DynoError> {
        if self.is_discontinued(id)? {
            return Ok(Read::plain(
                Truth::Fails,
                "discontinued by an accepted decision".to_string(),
            ));
        }
        // A MIRRORED NODE ITS SOURCE WITHDREW (req:a-superseded-mirrored-
        // judgment-is-kept-and-what-rested-on-it-reads-reason-lost) feeds no
        // chain: what rested on it has lost that reason. Unsupported, never
        // refuted — the far side withdrew it, nobody showed it false.
        if self.is_withdrawn_mirror(id)?
            && let Some(why) = self.superseded_reason(id)?
        {
            return Ok(Read::plain(Truth::Unknown, why));
        }
        // A CLAIM REPLACED BY ANOTHER (`replaces`, recorded as OBSOLETES from
        // its successor) feeds no chain any more
        // (req:a-requirements-read-back-follows-its-reason-to-the-evidence,
        // the capstone trial: after A2 was replaced by A2b, A2 still kept a
        // decision's chain holding). It is unsupported, never refuted: the
        // successor is what the reasoning rests on now.
        if let Some(by) = self.replaced_by(id)? {
            return Ok(Read::plain(
                Truth::Unknown,
                format!(
                    "replaced by {by}: a claim another takes the place of feeds no chain; what \
                     rested on it rests on {by} only if a step says so"
                ),
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
        let own = match intent {
            None => self.own_truth(id, node_type, ev, depth)?,
            Some(conclusion) => {
                let (truth, why) = self.intent_truth(id, node_type, conclusion)?;
                Read::plain(truth, why)
            }
        };
        let objections = self.objections_within(id, ev, depth, &own)?;
        let refuting: Vec<String> = objections
            .iter()
            .filter(|o| counts_as_holding(o.standing))
            .map(|o| match &o.conceded_by {
                Some(reply) => format!("{} (conceded by {reply})", o.id),
                None => o.id.clone(),
            })
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
            // CONTESTED only when its own support holds, or is itself held up
            // only by a dispute: a claim nothing supports stays unsupported
            // whatever objects to it.
            refuted_under: own.refuted_under.clone(),
            contested_by: if own.truth == Truth::Holds || !own.contested_by.is_empty() {
                ids.iter()
                    .map(|s| s.to_string())
                    .chain(own.contested_by.iter().cloned())
                    .collect()
            } else {
                BTreeSet::new()
            },
            lines: own.lines,
            lines_capped: own.lines_capped,
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
            bridges: own.bridges,
            evidential: own.evidential,
        })
    }

    /// Every opposing CONTRADICTS against `id`, each with the objection's own
    /// truth and how it stands: `refutes`, `stands` or `answered`. Sorted by
    /// the objection's id; empty when nothing objects.
    pub fn objections_to(&self, id: &str) -> Result<Vec<ObjectionReading>, DynoError> {
        let lost = BTreeSet::new();
        let mut ev = Eval::new(self, &lost);
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
        // THE SCOPE IS WHAT EVERY LINE OF THE TARGET HOLDS UNDER
        // (req:a-claim-with-several-lines-reads-each-lines-assumptions): a
        // claim that holds under S on one line and outside S on another is
        // not inside S, so an objection holding only under S contests it.
        let scope = scope.scope();
        for e in self.incoming(id, Some(edge::CONTRADICTS))? {
            let alignment = e
                .properties
                .get("alignment")
                .and_then(Value::as_str)
                .unwrap_or("opposing");
            // Corroboration is never an objection, and a CONCESSION is the
            // granting of one: neither attacks its target.
            if alignment == "supporting" || alignment == "conceding" {
                continue;
            }
            let r = self.truth_full(&e.from_id, ev, depth.saturating_sub(1))?;
            let truth = r.truth;
            // The objection's own line that needs least beyond the scope: it
            // refutes when ONE of its lines lies wholly inside the scope.
            let (outside, tests): (Vec<String>, Vec<String>) = r
                .lines
                .iter()
                .map(|line| {
                    let outside: Vec<String> = line
                        .assumes
                        .difference(&scope.assumes)
                        .chain(line.unverified.difference(&scope.unverified))
                        .chain(line.bridges.difference(&scope.bridges))
                        .cloned()
                        .collect();
                    // EVIDENCE AGAINST IS NOT A COUNTEREXAMPLE
                    // (req:a-hypothesis-tests-verdict-reads-as-supported-at-
                    // alpha-never-as-shown): an objection that holds only
                    // through a statistical test is evidence against the
                    // claim at α, so it contests the claim and never refutes
                    // it.
                    let tests: Vec<String> = line
                        .evidential
                        .difference(&scope.evidential)
                        .cloned()
                        .collect();
                    (outside, tests)
                })
                .min_by_key(|(o, t)| (o.len(), t.len()))
                .unwrap_or_default();
            let (standing, why) = if truth != Truth::Holds && !r.refuted_under.is_empty() {
                // NO STANDING OBJECTION: it was derived under a supposition
                // that now reads refuted (req:a-claim-derived-under-a-refuted-
                // supposition-says-so).
                (
                    "answered",
                    format!(
                        "it rests on the refuted supposition(s) {}, so it no longer counts: {}",
                        r.refuted_under
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", "),
                        r.why
                    ),
                )
            } else if truth == Truth::Holds && self.is_unsourced_given(&e.from_id)? {
                // AN OBJECTION NOBODY HAS SOURCED CONTESTS AND NEVER DEFEATS
                // (req:an-objection-taken-as-given-with-no-pinned-source-
                // contests-and-never-defeats): its force is the design's own
                // word (`taken_as_given`), and no Artifact documents it by
                // checksum. The four colour rung's Heawood objection, recalled
                // and unread, defeated Kempe's step outright. Once a pinned
                // source documents it, it holds as before.
                (
                    "stands",
                    format!(
                        "contested by an objection nobody has sourced: {} is taken as given and \
                         no pinned source documents it, so it contests what it attacks and never \
                         defeats or refutes it — register the source that was read (an Artifact \
                         with its checksum, DOCUMENTS to it) and it holds as stated; {}",
                        e.from_id, r.why
                    ),
                )
            } else if truth == Truth::Holds && !tests.is_empty() {
                (
                    "stands",
                    format!(
                        "{}; it rests on the evidence {} (a statistical test or a report), so it is \
                         evidence against the claim, not a refutation: it contests the claim and \
                         never refutes it",
                        r.why,
                        tests.join(", ")
                    ),
                )
            } else if truth == Truth::Holds && !outside.is_empty() {
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
            // GRANTED BY A REPLY THAT HOLDS: the objection counts against its
            // target as if it held, whatever its own truth — the arguer has
            // conceded it, and moved the argument to another reading.
            let (standing, why, conceded_by, moves_to) = match self
                .concession_of(&e.from_id, ev, depth)?
            {
                Some((reply, moves_to)) => (
                    "conceded",
                    format!(
                        "conceded: the reply {reply} grants it against this{}; on its own, {why}",
                        moves_to
                            .as_deref()
                            .map(|m| format!(", and moves the argument to {m}"))
                            .unwrap_or_default()
                    ),
                    Some(reply),
                    moves_to,
                ),
                None => (standing, why, None, None),
            };
            out.push(ObjectionReading {
                node_type: self.type_holding(&e.from_id)?,
                conceded_by,
                moves_to,
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

    /// A check's CONDITIONS: the claims that CONSTRAIN it and take part in a
    /// line of reasoning (a test setting concluded from an assessment), each
    /// read now — `holds`, `conditional`, `contested`, `unsupported` or
    /// `refuted`; and whether the check is STALE: one of them no longer holds
    /// outright (req:a-check-resting-on-a-claim-that-loses-support-reads-
    /// stale). `None` for a check no reasoned claim constrains, or not a
    /// Verification.
    pub fn check_conditions(&self, id: &str) -> Result<Option<CheckConditions>, DynoError> {
        if self.get_node(node::VERIFICATION, id)?.is_none() {
            return Ok(None);
        }
        let lost = BTreeSet::new();
        let mut ev = Eval::new(self, &lost);
        let conditions = self.condition_readings(id, &mut ev, INFERENCE_DEPTH)?;
        if conditions.is_empty() {
            return Ok(None);
        }
        let failing: Vec<String> = conditions
            .iter()
            .filter(|c| c.reads != "holds")
            .map(|c| c.id.clone())
            .collect();
        Ok(Some(CheckConditions {
            stale: !failing.is_empty(),
            why: if failing.is_empty() {
                "every condition it was run against holds".to_string()
            } else {
                format!(
                    "STALE: the condition(s) {} it was run against no longer hold outright, so \
                     whatever status it records says nothing about them until they are settled \
                     or it is run again",
                    failing.join(", ")
                )
            },
            conditions,
        }))
    }

    /// The conditions of `id` that no longer hold outright, read in `ev`
    /// (so a what-if's losses count); `None` when every one holds or none is
    /// reasoned.
    fn stale_conditions(
        &self,
        id: &str,
        ev: &mut Eval<'_>,
        depth: usize,
    ) -> Result<Option<Vec<ConditionReading>>, DynoError> {
        let failing: Vec<ConditionReading> = self
            .condition_readings(id, ev, depth)?
            .into_iter()
            .filter(|c| c.reads != "holds")
            .collect();
        Ok((!failing.is_empty()).then_some(failing))
    }

    fn condition_readings(
        &self,
        id: &str,
        ev: &mut Eval<'_>,
        depth: usize,
    ) -> Result<Vec<ConditionReading>, DynoError> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        for e in self.incoming(id, Some(edge::CONSTRAINS))? {
            let c = e.from_id;
            if !seen.insert(c.clone()) || !self.is_in_a_line_of_reasoning(&c)? {
                continue;
            }
            let r = self.truth_full(&c, ev, depth.saturating_sub(1))?;
            let reads = match r.truth {
                Truth::Holds if !r.assumes.is_empty() || !r.unverified.is_empty() => "conditional",
                Truth::Holds => "holds",
                Truth::Fails => "refuted",
                Truth::Unknown if !r.contested_by.is_empty() => "contested",
                Truth::Unknown => "unsupported",
            };
            out.push(ConditionReading {
                id: c,
                reads,
                why: r.why,
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// The live statement that REPLACES `id` — an OBSOLETES into it from a
    /// Requirement or Constraint that is not itself dropped or discontinued —
    /// when one does. A Decision's OBSOLETES is a withdrawal, read as
    /// discontinued elsewhere.
    pub fn replaced_by(&self, id: &str) -> Result<Option<String>, DynoError> {
        let mut by: Vec<String> = Vec::new();
        for e in self.incoming(id, Some(edge::OBSOLETES))? {
            for ty in [node::REQUIREMENT, node::CONSTRAINT] {
                if let Some(n) = self.get_node(ty, &e.from_id)?
                    && prop(&n.properties, "status") != Some("dropped")
                    && !self.is_discontinued(&e.from_id)?
                {
                    by.push(e.from_id.clone());
                }
            }
        }
        by.sort();
        Ok(by.into_iter().next())
    }

    /// Whether `id` is an OBJECTION whose force is the design's own word:
    /// marked `taken_as_given`, with no pinned source documenting it
    /// (req:an-objection-taken-as-given-with-no-pinned-source-contests-and-
    /// never-defeats). Only an objection: a counterexample is a statement
    /// shown by its own step, and a reply answers by the rule it always had.
    pub fn is_unsourced_given(&self, id: &str) -> Result<bool, DynoError> {
        let Some(props) = self.node_props(id)? else {
            return Ok(false);
        };
        if statement_kind(&props) != Some("objection")
            || props.get("taken_as_given").and_then(Value::as_bool) != Some(true)
        {
            return Ok(false);
        }
        Ok(self.pinned_sources(id)?.is_empty())
    }

    /// WHY `id` IS SUPERSEDED, when it is: another claim takes its place
    /// (`replaces`, an OBSOLETES from a live successor), or the design it was
    /// mirrored from withdrew it (`superseded_by_surface`, written by a
    /// refreshed mirror). `None` otherwise
    /// (req:a-superseded-mirrored-judgment-is-kept-and-what-rested-on-it-
    /// reads-reason-lost, parts 1 and 5).
    pub fn superseded_reason(&self, id: &str) -> Result<Option<String>, DynoError> {
        if let Some(by) = self.replaced_by(id)? {
            return Ok(Some(format!(
                "superseded: {by} takes its place, so it feeds no chain and is not published"
            )));
        }
        if let Some(props) = self.node_props(id)?
            && let Some(at) = superseded_by_surface(&props)
        {
            let from = prop(&props, "mirrored_from").unwrap_or("the design it was mirrored from");
            return Ok(Some(format!(
                "superseded: withdrawn by {from} at surface {at}, and kept here so that what \
                 rested on it reads reason-lost until it is re-grounded"
            )));
        }
        Ok(None)
    }

    /// Whether `id` is a node a refreshed mirror withdrew and kept.
    pub fn is_withdrawn_mirror(&self, id: &str) -> Result<bool, DynoError> {
        Ok(self
            .node_props(id)?
            .is_some_and(|p| superseded_by_surface(&p).is_some()))
    }

    /// THE WITHDRAWN MIRRORED PREMISES `id` rests on, at any depth: nodes a
    /// refreshed mirror kept and marked superseded because their source
    /// withdrew them. Non-empty means `id` has lost a reason
    /// (req:a-superseded-mirrored-judgment-is-kept-and-what-rested-on-it-
    /// reads-reason-lost, part 3). Sorted.
    pub fn rests_on_withdrawn(&self, id: &str) -> Result<Vec<String>, DynoError> {
        let mut out: Vec<String> = Vec::new();
        for n in self.rests_on_closure_of_node(id)? {
            if self.is_withdrawn_mirror(&n)? {
                out.push(n);
            }
        }
        Ok(out)
    }

    /// THE NAMED PARTIES WHO CLAIM `id`, with the nodes in `lost` counted as
    /// dropped: the party on `id` itself (`claimed_by`), and every party on a
    /// statement or step `id` rests on — the claimant's argument
    /// (req:a-claim-made-by-a-named-party-and-not-yet-verified-reads-
    /// claimed). A party carried only by what `lost` drops no longer claims
    /// it, and nor does `id`'s own party when the what-if drops that party's
    /// argument and nothing of it is left: "a what-if that drops the
    /// claimant's argument returns it to open". An assumed premise is
    /// supposed, never relied on, so a claim on one carries nothing.
    pub fn claimants_of(
        &self,
        id: &str,
        lost: &BTreeSet<String>,
    ) -> Result<BTreeSet<String>, DynoError> {
        let mut live: BTreeSet<String> = BTreeSet::new();
        let mut dropped: BTreeSet<String> = BTreeSet::new();
        let mut seen: BTreeSet<(String, bool)> = BTreeSet::new();
        let mut queue: Vec<(String, bool)> = self
            .inferences_concluding(id)?
            .into_iter()
            .map(|s| (s, false))
            .collect();
        while let Some((n, cut)) = queue.pop() {
            let cut = cut || lost.contains(&n);
            if !seen.insert((n.clone(), cut)) {
                continue;
            }
            if let Some(props) = self.node_props(&n)?
                && let Some(party) = claimed_by(&props)
            {
                if cut {
                    dropped.insert(party.to_string());
                } else {
                    live.insert(party.to_string());
                }
            }
            if self.get_node(node::INFERENCE, &n)?.is_some() {
                for (p, assumed) in self.premise_links(&n)? {
                    if !assumed {
                        queue.push((p, cut));
                    }
                }
            } else {
                for st in self.inferences_concluding(&n)? {
                    queue.push((st, cut));
                }
            }
        }
        if let Some(props) = self.node_props(id)?
            && let Some(party) = claimed_by(&props)
            && !lost.contains(id)
            && (!dropped.contains(party) || live.contains(party))
        {
            live.insert(party.to_string());
        }
        Ok(live)
    }

    /// "claimed by OpenAI, unverified" — the claim in words, for a set of
    /// parties.
    fn claimed_said(parties: &BTreeSet<String>) -> String {
        format!(
            "claimed by {}, unverified: no line of this design establishes it, and nothing \
             refutes it",
            parties.iter().cloned().collect::<Vec<_>>().join(" and ")
        )
    }

    /// EVERY STATUS `id` HELD OVER A PERIOD: each dated `TemporalFact` with
    /// `fact_type: status` about it, earliest first. Read back by
    /// `reasoning_report`, `get_node` and the step and proof readings, so
    /// "accepted 1879–1890, open 1890–1976" is not left in notes nobody
    /// reads (the four colour rung's finding (a)).
    pub fn status_over_time(&self, id: &str) -> Result<Vec<StatusPeriod>, DynoError> {
        let mut out: Vec<StatusPeriod> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for e in self.outgoing(id, Some(edge::HAS_TEMPORAL_FACT))? {
            if !seen.insert(e.to_id.clone()) {
                continue;
            }
            let Some(f) = self.get_node(node::TEMPORAL_FACT, &e.to_id)? else {
                continue;
            };
            if prop(&f.properties, "fact_type") != Some("status") {
                continue;
            }
            out.push(StatusPeriod {
                fact: f.node_id.clone(),
                says: prop(&f.properties, "name")
                    .or_else(|| prop(&f.properties, "statement"))
                    .map(str::to_string),
                from: prop(&f.properties, "valid_from").map(str::to_string),
                until: prop(&f.properties, "valid_to").map(str::to_string),
            });
        }
        out.sort_by(|a, b| (&a.from, &a.fact).cmp(&(&b.from, &b.fact)));
        Ok(out)
    }

    /// A CLAIM ACCEPTED FOR A PERIOD AND WITHDRAWN, read: `id` (a statement or
    /// a step) does not hold now, every status it held over a period has
    /// ended, and the last one names the period. What withdrew it is every
    /// objection not answered that stands against it or a step it rests on.
    /// `None` when it holds, or still holds a status, or held none.
    pub fn withdrawn_reading(&self, id: &str) -> Result<Option<WithdrawnReading>, DynoError> {
        let periods = self.status_over_time(id)?;
        if periods.is_empty() || periods.iter().any(|p| p.until.is_none()) {
            return Ok(None);
        }
        let (truth, why_now) = self.premise_truth(id)?;
        if truth == Truth::Holds {
            return Ok(None);
        }
        let Some(last) = periods
            .iter()
            .max_by(|a, b| (&a.until, &a.fact).cmp(&(&b.until, &b.fact)))
        else {
            return Ok(None);
        };
        let mut steps: BTreeSet<String> = BTreeSet::new();
        if self.get_node(node::INFERENCE, id)?.is_some() {
            steps.extend(self.steps_under(id)?);
        } else {
            for st in self.inferences_concluding(id)? {
                steps.extend(self.steps_under(&st)?);
            }
        }
        let mut withdrawn_by: BTreeSet<String> = BTreeSet::new();
        for target in std::iter::once(id.to_string()).chain(steps) {
            for e in self.incoming(&target, Some(edge::CONTRADICTS))? {
                if e.properties.get("alignment").and_then(Value::as_str) == Some("supporting") {
                    continue;
                }
                if self.premise_truth(&e.from_id)?.0 != Truth::Fails {
                    withdrawn_by.insert(e.from_id);
                }
            }
        }
        let withdrawn_by: Vec<String> = withdrawn_by.into_iter().collect();
        let why = format!(
            "withdrawn: it held {} ({}), and that ended{}; now {why_now}",
            last.period(),
            last.says.as_deref().unwrap_or(&last.fact),
            if withdrawn_by.is_empty() {
                String::new()
            } else {
                format!(", withdrawn by {}", withdrawn_by.join(", "))
            }
        );
        Ok(Some(WithdrawnReading {
            held: last.period(),
            fact: last.fact.clone(),
            says: last.says.clone(),
            withdrawn_by,
            why,
        }))
    }

    /// Whether some step discharges `id` (`discharges` or `cases`): a
    /// supposition closed somewhere, not an open one. Read once per
    /// evaluation.
    fn discharged_somewhere(&self, id: &str, ev: &mut Eval<'_>) -> Result<bool, DynoError> {
        if ev.discharged.is_none() {
            let mut all = BTreeSet::new();
            for st in self.scan_nodes(node::INFERENCE)? {
                all.extend(list_prop(&st.properties, "discharges"));
                all.extend(list_prop(&st.properties, "cases"));
            }
            ev.discharged = Some(all);
        }
        Ok(ev.discharged.as_ref().is_some_and(|d| d.contains(id)))
    }

    /// The reply that CONCEDES `objection` — a CONTRADICTS marked `alignment:
    /// conceding` from a reply that holds — with the reading it moves the
    /// argument to (`moves_to`), when one does. The first in id order.
    fn concession_of(
        &self,
        objection: &str,
        ev: &mut Eval<'_>,
        depth: usize,
    ) -> Result<Option<(String, Option<String>)>, DynoError> {
        let mut conceding: Vec<(String, Option<String>)> = self
            .incoming(objection, Some(edge::CONTRADICTS))?
            .into_iter()
            .filter(|e| e.properties.get("alignment").and_then(Value::as_str) == Some("conceding"))
            .map(|e| {
                let to = e
                    .properties
                    .get("moves_to")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                (e.from_id, to)
            })
            .collect();
        conceding.sort();
        for (reply, moves_to) in conceding {
            if self.truth_full(&reply, ev, depth.saturating_sub(1))?.truth == Truth::Holds {
                return Ok(Some((reply, moves_to)));
            }
        }
        Ok(None)
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
        let mut holding_lines: Vec<Condition> = Vec::new();
        let mut lines_capped = false;
        let mut contested_routes: Vec<(String, Vec<String>)> = Vec::new();
        let mut refuted_routes: BTreeSet<String> = BTreeSet::new();
        for s in &steps {
            let r = self.step_reading(s, ev, depth)?;
            if r.verdict == Truth::Holds {
                holding_lines.extend(r.lines.iter().cloned());
                lines_capped |= r.lines_capped;
            }
            if !r.contested_by.is_empty() {
                contested_routes.push((s.clone(), r.contested_by.clone()));
            }
            if r.verdict != Truth::Holds {
                refuted_routes.extend(r.rests_on_refuted_suppositions.iter().cloned());
            }
            verdicts.push((s.clone(), r.verdict));
        }
        let said: Vec<String> = verdicts
            .iter()
            .map(|(s, v)| format!("{s} {}", v.as_str()))
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
        // Then the fewest STATISTICAL TESTS: a deductive line beats one that
        // rests on evidence, which supports the claim only at its α.
        //
        // AND EVERY LINE IS KEPT (req:a-claim-with-several-lines-reads-each-
        // lines-assumptions, the trolley trial): the claim holds under ANY
        // ONE of its lines, so it carries all of the minimal ones, lightest
        // first. The lightest decides what it is read as holding under; the
        // rest are named beside it, never dropped.
        if !holding_lines.is_empty() {
            let (lines, capped) = minimal_lines(holding_lines);
            let lines_capped = lines_capped || capped;
            let assumes = lines[0].assumes.clone();
            let unverified = lines[0].unverified.clone();
            let bridges = lines[0].bridges.clone();
            let evidential = lines[0].evidential.clone();
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
            if !bridges.is_empty() {
                why.push_str(&format!(
                    ", and only as far as the bridge(s) {} model what they are taken to model",
                    bridges.iter().cloned().collect::<Vec<_>>().join(", ")
                ));
            }
            if !evidential.is_empty() {
                why.push_str(&format!(
                    ", resting on the evidence {}",
                    self.tests_said(&evidential)?
                ));
            }
            if lines.len() > 1 {
                let served: Vec<LineConditions> = lines.iter().map(LineConditions::of).collect();
                why.push_str(&format!(
                    "; it holds on {} lines, under any one of them: {}{}",
                    lines.len(),
                    lines_said(&served),
                    if lines_capped {
                        format!(" (and more: only the lightest {LINES_CAP} are kept)")
                    } else {
                        String::new()
                    }
                ));
            }
            return Ok(Some(Read::with_lines(
                Truth::Holds,
                why,
                lines,
                lines_capped,
            )));
        }
        // CONTESTED, NOT UNSUPPORTED (req:a-conclusion-whose-every-route-is-
        // contested-reads-contested, the cogito trial): no route holds, and at
        // least one would hold were the disputes under it settled its way. It
        // reads contested, naming what stands against each such route, never
        // `unproven` — the word that also covers a claim nothing supports.
        if !contested_routes.is_empty() {
            let against: Vec<String> = contested_routes
                .iter()
                .map(|(s, by)| format!("{s} by {}", by.join(", ")))
                .collect();
            let by: BTreeSet<String> = contested_routes
                .iter()
                .flat_map(|(_, by)| by.iter().cloned())
                .collect();
            return Ok(Some(
                Read::plain(
                    Truth::Unknown,
                    format!(
                        "contested: no route holds while a dispute holds up {} of its {} route(s) \
                         — {} — neither established nor answered ({})",
                        contested_routes.len(),
                        verdicts.len(),
                        against.join("; "),
                        said.join("; ")
                    ),
                )
                .contested(by),
            ));
        }
        // DERIVED UNDER A REFUTED SUPPOSITION (req:a-claim-derived-under-a-
        // refuted-supposition-says-so): it says so, and is no open
        // conditional that still holds.
        if !refuted_routes.is_empty() {
            return Ok(Some(
                Read::plain(
                    Truth::Unknown,
                    format!(
                        "unsupported: it rests on the refuted supposition(s) {} — derived under a \
                         supposition that now reads refuted, so it no longer holds even \
                         conditionally ({})",
                        refuted_routes
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", "),
                        said.join("; ")
                    ),
                )
                .under_refuted(refuted_routes),
            ));
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
        let mut ev = Eval::new(self, &lost);
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

    /// What a requirement, capability or interface says AS ITS OWNER'S
    /// INTENT, before any objection is counted
    /// (dec:a-premise-says-whether-it-rests-on-intent-or-delivery):
    ///
    /// - a **Requirement** by its status: accepted or met holds, proposed or
    ///   deferred is unknown, dropped fails;
    /// - a **Capability**, whose status says only how far it is BUILT, by the
    ///   needs it SATISFIES: it holds when one is accepted (or met), fails
    ///   when every one is dropped, and is unknown otherwise;
    /// - an **Interface**, which has no status, by the Decisions that govern
    ///   it: it holds when one is accepted, fails when every one was rejected
    ///   or superseded, and is unknown otherwise.
    ///
    /// The Decision the step concludes (`conclusion`) lends nothing: a
    /// requirement or an interface that exists because of it is no reason
    /// for it, and reads unknown, saying the support would be circular.
    fn intent_truth(
        &self,
        id: &str,
        node_type: &str,
        conclusion: Option<&str>,
    ) -> Result<(Truth, String), DynoError> {
        const AS_INTENT: &str = "read as its owner's intent";
        let governed_by_the_conclusion = |g: &Self| -> Result<bool, DynoError> {
            Ok(match conclusion {
                Some(c) => g
                    .outgoing(id, Some(edge::GOVERNED_BY))?
                    .iter()
                    .any(|e| e.to_id == c),
                None => false,
            })
        };
        let status_of = |g: &Self, ty: &str, of: &str| -> Result<String, DynoError> {
            Ok(g.get_node(ty, of)?
                .and_then(|n| prop(&n.properties, "status").map(str::to_string))
                .unwrap_or_else(|| "proposed".to_string()))
        };
        match node_type {
            node::REQUIREMENT => {
                if governed_by_the_conclusion(self)? {
                    return Ok((
                        Truth::Unknown,
                        format!(
                            "{AS_INTENT}, but it exists because of {}, the decision this step                              concludes, so it cannot be a reason for that decision: the                              support would be circular",
                            conclusion.unwrap_or_default()
                        ),
                    ));
                }
                let status = status_of(self, node::REQUIREMENT, id)?;
                Ok(match status.as_str() {
                    "accepted" => (
                        Truth::Holds,
                        format!(
                            "{AS_INTENT}: accepted by its owner, so it is required (whether it is                              delivered is read apart)"
                        ),
                    ),
                    "met" => (
                        Truth::Holds,
                        format!("{AS_INTENT}: accepted by its owner, and met"),
                    ),
                    "dropped" => (
                        Truth::Fails,
                        format!("{AS_INTENT}: dropped by its owner, so it is no longer required"),
                    ),
                    "deferred" => (
                        Truth::Unknown,
                        format!(
                            "{AS_INTENT}: deferred by its owner, so it is set aside, not                              required now"
                        ),
                    ),
                    other => (
                        Truth::Unknown,
                        format!(
                            "{AS_INTENT}: its status is {other}, so its owner has not accepted it"
                        ),
                    ),
                })
            }
            node::CAPABILITY => {
                let mut needs: Vec<(String, String)> = Vec::new();
                for e in self.outgoing(id, Some(edge::SATISFIES))? {
                    if self.type_holding(&e.to_id)?.as_deref() != Some(node::REQUIREMENT) {
                        continue;
                    }
                    let status = if self.is_discontinued(&e.to_id)? {
                        "dropped".to_string()
                    } else {
                        status_of(self, node::REQUIREMENT, &e.to_id)?
                    };
                    needs.push((e.to_id, status));
                }
                needs.sort();
                needs.dedup();
                if let Some((need, status)) = needs
                    .iter()
                    .find(|(_, s)| matches!(s.as_str(), "accepted" | "met"))
                {
                    return Ok((
                        Truth::Holds,
                        format!(
                            "{AS_INTENT}: it serves {need}, a need its owner accepted ({status});                              whether it is built is read apart"
                        ),
                    ));
                }
                if needs.is_empty() {
                    return Ok((
                        Truth::Unknown,
                        format!(
                            "{AS_INTENT}: it satisfies no requirement, so no owner has accepted a                              need for it"
                        ),
                    ));
                }
                let said = needs
                    .iter()
                    .map(|(n, s)| format!("{n} ({s})"))
                    .collect::<Vec<_>>()
                    .join(", ");
                if needs.iter().all(|(_, s)| s == "dropped") {
                    return Ok((
                        Truth::Fails,
                        format!("{AS_INTENT}: every need it serves was dropped: {said}"),
                    ));
                }
                Ok((
                    Truth::Unknown,
                    format!("{AS_INTENT}: no need it serves is accepted by its owner: {said}"),
                ))
            }
            node::INTERFACE => {
                let mut governors: Vec<(String, String)> = Vec::new();
                let mut by_the_conclusion = false;
                for e in self.outgoing(id, Some(edge::GOVERNED_BY))? {
                    if self.type_holding(&e.to_id)?.as_deref() != Some(node::DECISION) {
                        continue;
                    }
                    if Some(e.to_id.as_str()) == conclusion {
                        by_the_conclusion = true;
                        continue;
                    }
                    let status = status_of(self, node::DECISION, &e.to_id)?;
                    governors.push((e.to_id, status));
                }
                governors.sort();
                governors.dedup();
                if let Some((d, _)) = governors.iter().find(|(_, s)| s == "accepted") {
                    return Ok((
                        Truth::Holds,
                        format!("{AS_INTENT}: governed by {d}, a decision its owner accepted"),
                    ));
                }
                if governors.is_empty() {
                    return Ok((
                        Truth::Unknown,
                        if by_the_conclusion {
                            format!(
                                "{AS_INTENT}, but it is governed only by {}, the decision this                                  step concludes, so it cannot be a reason for that decision: the                                  support would be circular",
                                conclusion.unwrap_or_default()
                            )
                        } else {
                            format!(
                                "{AS_INTENT}: no decision governs it, so its owner has not                                  accepted it"
                            )
                        },
                    ));
                }
                let said = governors
                    .iter()
                    .map(|(d, s)| format!("{d} ({s})"))
                    .collect::<Vec<_>>()
                    .join(", ");
                if governors
                    .iter()
                    .all(|(_, s)| matches!(s.as_str(), "rejected" | "superseded"))
                {
                    return Ok((
                        Truth::Fails,
                        format!("{AS_INTENT}: every decision governing it was let go: {said}"),
                    ));
                }
                Ok((
                    Truth::Unknown,
                    format!("{AS_INTENT}: no decision governing it is accepted: {said}"),
                ))
            }
            other => Ok((Truth::Unknown, format!("a {other} is not read as intent"))),
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
            // A BRIDGE ARGUED FOR IS STILL A BRIDGE: what rests on it carries
            // it, whatever its own argument shows.
            if statement_kind(&n.properties) == Some("bridge") {
                return Ok(chain.through_bridge(id));
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
        // EVIDENCE HOLDS AS EVIDENCE (req:evidence-carries-its-source-grade-
        // and-confidence-and-is-not-intent): "the report says X", cited as
        // evidence through the source that documents it, and what rests on it
        // is supported, never shown or proven.
        if node_type == node::REQUIREMENT && is_evidence(&n.properties) {
            return self.evidence_read(id, &n.properties);
        }
        if node_type == node::CONSTRAINT && is_evidence(&n.properties) {
            let (truth, why) = self.constraint_truth(id, &n.properties)?;
            if truth != Truth::Holds {
                return Ok(Read::plain(truth, why));
            }
            return self.evidence_read(id, &n.properties);
        }

        // A BRIDGE HOLDS AS A BRIDGE (dec:idea-a-bridge-premise-says-a-formal-
        // statement-models-a-claim, option b, chosen 2026-10-22 after the
        // Lucas–Penrose trial): "the formal statement F models the claim P".
        // It holds — it is the step a cross-domain argument takes — but only
        // as itself: what rests on it carries it, like an unverified citation,
        // so it reads CONDITIONAL on the bridge; and because it is an ordinary
        // premise, never an assumed one, an objection to it reaches it and
        // contests every claim resting on it. An `assumed` premise was immune
        // to objection, a `taken_as_given` one filed the translation beside
        // the theorem as a foundation, and a plain one read "not delivered".
        if matches!(node_type, node::REQUIREMENT | node::CONSTRAINT)
            && statement_kind(&n.properties) == Some("bridge")
        {
            return Ok(Read::one(
                Truth::Holds,
                "a bridge: it says a formal statement models a claim of another field, and \
                 what rests on it holds only as far as that modelling is faithful"
                    .to_string(),
                Condition {
                    bridges: BTreeSet::from([id.to_string()]),
                    ..Condition::default()
                },
            ));
        }

        // A HYPOTHESIS IS ASSUMED (fact:root-cause-proof-statements-fall-
        // through-to-software-predicates-and-assumed-premises-stopped-counting-
        // 2026-10-22, cause 2). It is a supposition taken up for the sake of
        // argument, so it holds only UNDER ITSELF: what rests on it as an
        // ordinary premise holds only under it, until a step discharges it.
        // Until 2026-10-22 it fell through to the requirement predicate below
        // and read "not delivered" while `statement_reading` read it
        // `assumed` or `discharged`: the two readers disagreed.
        if matches!(node_type, node::REQUIREMENT | node::CONSTRAINT)
            && statement_kind(&n.properties) == Some("hypothesis")
        {
            let assumed_in = self.steps_assuming(id)?;
            let why = if assumed_in.is_empty() {
                "a hypothesis: a supposition no step takes up yet, so it holds only under itself \
                 and is never a stated foundation"
                    .to_string()
            } else {
                format!(
                    "a hypothesis: a supposition taken up in {} for the sake of argument, so it \
                     holds only under itself, and what rests on it holds only under it until a \
                     step discharges it",
                    assumed_in.join(", ")
                )
            };
            return Ok(Read::one(
                Truth::Holds,
                why,
                Condition {
                    assumes: BTreeSet::from([id.to_string()]),
                    ..Condition::default()
                },
            ));
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
            // A CHECK WHOSE CONDITIONS LOST THEIR SUPPORT IS STALE
            // (req:a-check-resting-on-a-claim-that-loses-support-reads-stale):
            // a passing check run against settings concluded from claims that
            // no longer hold says nothing about them.
            node::VERIFICATION
                if status.as_deref() == Some("passing")
                    && self.stale_conditions(id, ev, depth)?.is_some() =>
            {
                let failing = self.stale_conditions(id, ev, depth)?.unwrap_or_default();
                (
                    Truth::Unknown,
                    format!(
                        "stale: it was run against conditions that no longer hold — {}",
                        failing
                            .iter()
                            .map(|c| format!("{} ({})", c.id, c.reads))
                            .collect::<Vec<_>>()
                            .join("; ")
                    ),
                )
            }
            node::VERIFICATION => match status.as_deref() {
                Some("passing") => match self.computed_check_truth(id, &n.properties)? {
                    // A HYPOTHESIS TEST, KEPT: read from its record's block
                    // (crate::hypothesis), never as plain `holds`.
                    Some((Truth::Holds, _))
                        if n.properties
                            .contains_key(crate::hypothesis::HYPOTHESIS_TEST) =>
                    {
                        return self.test_read(id, &n.properties, ev, depth);
                    }
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
                // A DISPUTE KEPT OPEN ON PURPOSE: what rests on it is
                // contested by design, never merely unsupported.
                Some("contested") => {
                    return Ok(Read::plain(
                        Truth::Unknown,
                        "the decision is contested, by design: both sides are kept and no \
                         verdict is owed"
                            .to_string(),
                    )
                    .contested(BTreeSet::from([id.to_string()])));
                }
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

    /// A HYPOTHESIS TEST'S READING as a premise, once its kept record counts
    /// (`crate::hypothesis`): a test that did not reject H0 reads NOT
    /// REJECTED and supports nothing; one that did is SUPPORTED AT α, carried
    /// as evidence by everything that rests on it — but only while every
    /// assumption it DEPENDS_ON holds, and only when it was tested at no more
    /// than it spends of its α budget.
    fn test_read(
        &self,
        id: &str,
        props: &HashMap<String, Value>,
        ev: &mut Eval<'_>,
        depth: usize,
    ) -> Result<Read, DynoError> {
        let test = match crate::hypothesis::HypothesisTest::from_props(props) {
            Some(Ok(t)) => t,
            Some(Err(e)) => {
                return Ok(Read::plain(
                    Truth::Unknown,
                    format!("its hypothesis test cannot be read: {e}"),
                ));
            }
            None => {
                return Ok(Read::plain(
                    Truth::Unknown,
                    "it carries no hypothesis test to read".to_string(),
                ));
            }
        };
        if !test.reject_h0 {
            return Ok(Read::plain(Truth::Unknown, test.not_rejected()));
        }
        let mut assumes = BTreeSet::new();
        let mut unverified = BTreeSet::new();
        let mut bridges = BTreeSet::new();
        let mut evidential = BTreeSet::from([id.to_string()]);
        let mut against = Vec::new();
        for e in self.outgoing(id, Some(edge::DEPENDS_ON))? {
            let r = self.truth_full(&e.to_id, ev, depth.saturating_sub(1))?;
            match r.truth {
                Truth::Holds => {
                    assumes.extend(r.assumes);
                    unverified.extend(r.unverified);
                    bridges.extend(r.bridges);
                    evidential.extend(r.evidential);
                }
                Truth::Fails => against.push(format!("{} is refuted: {}", e.to_id, r.why)),
                Truth::Unknown => {
                    against.push(format!("{} is not established: {}", e.to_id, r.why))
                }
            }
        }
        if !against.is_empty() {
            return Ok(Read::plain(
                Truth::Unknown,
                format!(
                    "its test rejected H0 at α = {} (p = {}), but an assumption it rests on does                      not hold, so it supports nothing (unsupported, never refuted): {}",
                    test.alpha,
                    test.p_shown(),
                    against.join("; ")
                ),
            ));
        }
        if let Some(over) = self.alpha_overspent(id, &test)? {
            return Ok(Read::plain(
                Truth::Unknown,
                format!(
                    "its test rejected H0, but it was {over} (p = {}, {}, {})",
                    test.p_shown(),
                    test.named(),
                    test.n_shown()
                ),
            ));
        }
        // A CRITERION CHANGED AFTER THE DATA stops the test counting until
        // its owner acknowledges the change with a reason
        // (req:a-test-whose-criteria-moved-after-its-data-reads-unknown-until-
        // acknowledged; dec:idea-should-a-criterion-changed-after-the-data-
        // move-the-tests-reading, option b). Until 2026-10-20 it was only
        // reported. Every write still lands; nothing is refused.
        let (moved, _) = self.criteria_changes(id)?;
        if !moved.is_empty()
            && self
                .gap_acknowledgement(&crate::detect::criteria_gap_id(id, &moved))?
                .is_none()
        {
            return Ok(Read::plain(
                Truth::Unknown,
                format!(
                    "its criteria moved after its data ({}): it supports nothing until the owner \
                     acknowledges the change with a reason (acknowledge the \
                     criteria_changed_after_data gap); then it reads as before",
                    moved
                        .iter()
                        .map(|c| format!("{} `{}`", c.criterion, c.field))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
        Ok(Read::one(
            Truth::Holds,
            test.supported(),
            Condition {
                assumes,
                unverified,
                bridges,
                evidential,
            },
        ))
    }

    /// Each piece of evidence in `ids` — a statistical test, or a report cited
    /// as evidence — as its reading names it.
    fn tests_said(&self, ids: &BTreeSet<String>) -> Result<String, DynoError> {
        let mut out = Vec::new();
        for t in ids {
            if let Some(props) = self.node_props(t)?
                && is_evidence(&props)
            {
                let grade = grade_said(&props)
                    .map(|g| format!(" ({g})"))
                    .unwrap_or_default();
                let cited = if self.pinned_sources(t)?.is_empty() {
                    "unverified"
                } else {
                    "cited as evidence"
                };
                out.push(format!("{t}, evidence{grade}, {cited}"));
                continue;
            }
            match self.hypothesis_test_of(t)? {
                Some(test) => out.push(format!("{t}, {}", test.supported())),
                None => out.push(format!("{t}, {}", crate::hypothesis::EVIDENCE_NOT_PROOF)),
            }
        }
        Ok(out.join("; "))
    }

    /// THE STATISTICAL TESTS `id` rests on, as the design stands: checks whose
    /// support is a hypothesis test that rejected H0, beneath it or itself,
    /// sorted. Empty when it rests on none.
    pub fn tests_under(&self, id: &str) -> Result<Vec<String>, DynoError> {
        Ok(self.premise_reading(id)?.evidential.into_iter().collect())
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
        Ok(Read::with_lines(Truth::Fails, why, d.lines, d.lines_capped))
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
            ev.track.found_on(id, Stack::Node);
            return Ok(Read::plain(
                Truth::Unknown,
                "it comes back to itself through its support, and a circle has no verdict"
                    .to_string(),
            ));
        }
        ev.track.found_off(id, Stack::Node);
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
            Some(d) if d.truth == Truth::Holds => Read::with_lines(
                Truth::Holds,
                format!("a falsum (⊥), derived: {}", d.why),
                d.lines,
                d.lines_capped,
            ),
            // A DERIVATION HELD UP ONLY BY A DISPUTE IS CARRIED, not dropped
            // (req:a-conclusion-whose-every-route-is-contested-reads-
            // contested; the Lucas–Penrose trial, 2026-10-22): the reductio
            // resting on a contested ⊥ is a contested route, and its
            // conclusion reads `contested`. Until 2026-10-23 this read plain
            // unknown, and the main conclusion read "unproven, no derivation
            // holds" while every objection against it stood on a translation
            // step under the ⊥.
            Some(d) => Read::plain(
                Truth::Unknown,
                format!("a falsum (⊥) not derived: {}", d.why),
            )
            .contested(d.contested_by)
            .under_refuted(d.refuted_under),
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
        Ok(Read::one(
            Truth::Holds,
            CITED_UNVERIFIED_WHY.to_string(),
            Condition {
                assumes: BTreeSet::new(),
                unverified: BTreeSet::from([id.to_string()]),
                bridges: BTreeSet::new(),
                evidential: BTreeSet::new(),
            },
        ))
    }

    /// A piece of EVIDENCE as a premise: it holds AS EVIDENCE, and everything
    /// resting on it carries it as evidence — supported, never shown or
    /// proven. Cited as evidence when a pinned source documents it; with
    /// none it still holds as evidence, and reads `evidence_unverified`.
    fn evidence_read(&self, id: &str, props: &HashMap<String, Value>) -> Result<Read, DynoError> {
        let pinned = self.pinned_sources(id)?;
        let grade = grade_said(props)
            .map(|g| format!(" ({g})"))
            .unwrap_or_default();
        let why = if pinned.is_empty() {
            format!("evidence{grade}{EVIDENCE_UNVERIFIED_TAIL}")
        } else {
            format!(
                "cited as evidence{grade}: the report its pinned source(s) {} document; what rests \
                 on it is supported by evidence, never proven",
                pinned.join(", ")
            )
        };
        // AN UNDOCUMENTED REPORT IS AN UNVERIFIED CITATION
        // (dec:idea-what-rests-on-unsourced-evidence-reads-supported-or-
        // conditional, option b, accepted 2026-10-20): what rests on it reads
        // CONDITIONAL on it until its source is documented.
        let unverified = if pinned.is_empty() {
            BTreeSet::from([id.to_string()])
        } else {
            BTreeSet::new()
        };
        Ok(Read::one(
            Truth::Holds,
            why,
            Condition {
                assumes: BTreeSet::new(),
                unverified,
                bridges: BTreeSet::new(),
                evidential: BTreeSet::from([id.to_string()]),
            },
        ))
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
        if prop(props, "direction") == Some("setpoint") {
            return Ok((
                Truth::Holds,
                "a setpoint: a value the thing is held at, never a budget to exceed".to_string(),
            ));
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
        let mut ev = Eval::new(self, &lost);
        let mut reading = self.step_reading(inference_id, &mut ev, INFERENCE_DEPTH)?;
        reading.independence = self.independence_of(inference_id)?;
        // CIRCULAR (dec:idea-speculate-the-bridge-that-would-close-a-gap-then-
        // test-it, option c): a flag beside the verdict, never in its place.
        // Read on the step itself, as `independence` is, never inside another
        // step's reading.
        reading.circular = self.circularity_of(inference_id)?.map(|c| c.why);
        // A STATUS OVER A PERIOD, read back on the step itself (PR A2): Kempe's
        // 1879 proof "accepted 1879–1890" reads withdrawn, with the period
        // and the objection that withdrew it.
        reading.status_over_time = self.status_over_time(inference_id)?;
        reading.withdrawn = self.withdrawn_reading(inference_id)?;
        reading.claimed_by = self
            .node_props(inference_id)?
            .and_then(|p| claimed_by(&p).map(str::to_string));
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
    /// that), no cited foundation, and NO PREMISE (PR A2): a statement neither
    /// line derives — taken as given, a claim nobody has shown, an assumption
    /// nothing discharges — whose loss would cost both. A premise beneath a
    /// named lemma is that lemma's, and is shared with it. The vocabulary
    /// (definitions, axioms, rules of method) and the theorem's own setting
    /// (premises both final steps take up) are named in `shared_setting` and
    /// leave the lines independent.
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
        // What a named lemma rests on is the lemma's: sharing the lemma
        // shares it, and naming the lemma accepts that.
        let mut allowed_premises: BTreeSet<String> = except.iter().cloned().collect();
        for lemma in &except {
            for s in self.inferences_concluding(lemma)? {
                allowed.extend(self.steps_under(&s)?);
            }
            allowed_premises.extend(self.rests_on_closure_of_node(lemma)?);
        }
        let my_nodes = self.rests_on_closure(step)?;
        let no_loss: BTreeSet<String> = BTreeSet::new();
        let mut out = Vec::new();
        for other in others {
            if other == step || self.get_node(node::INFERENCE, &other)?.is_none() {
                out.push(IndependenceReading {
                    step: step.to_string(),
                    of: other.clone(),
                    holds: false,
                    shared_steps: Vec::new(),
                    shared_citations: Vec::new(),
                    shared_premises: Vec::new(),
                    shared_setting: Vec::new(),
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
            // The premises both FINAL steps take up directly: the theorem's
            // own stated setting, which every proof of it shares.
            let my_direct: BTreeSet<String> = self.premises_of(step)?.into_iter().collect();
            let setting: BTreeSet<String> = self
                .premises_of(&other)?
                .into_iter()
                .filter(|p| my_direct.contains(p))
                .collect();
            let mut shared_citations: Vec<String> = Vec::new();
            let mut shared_premises: Vec<String> = Vec::new();
            let mut shared_setting: Vec<String> = Vec::new();
            for f in my_nodes.intersection(&their_nodes) {
                if except.contains(f) {
                    continue;
                }
                let props = self.node_props(f)?.unwrap_or_default();
                if is_cited_result(&props) || is_evidence(&props) {
                    shared_citations.push(f.clone());
                    continue;
                }
                // A statement a step derives is shared through that step
                // (`shared_steps`); a premise that is itself a step likewise.
                // A supposition some step discharges (a reductio's ¬P, a
                // theorem's hypothesis closed above both lines) costs nothing
                // when lost, as the what-if reads it: never shared.
                let mut ev = Eval::new(self, &no_loss);
                if allowed_premises.contains(f)
                    || !self.inferences_concluding(f)?.is_empty()
                    || self.get_node(node::INFERENCE, f)?.is_some()
                    || self.discharged_somewhere(f, &mut ev)?
                {
                    continue;
                }
                if setting.contains(f)
                    || matches!(
                        statement_kind(&props),
                        Some("definition" | "axiom" | "method_rule")
                    )
                {
                    shared_setting.push(f.clone());
                    continue;
                }
                shared_premises.push(f.clone());
            }
            let holds = shared_steps.is_empty()
                && shared_citations.is_empty()
                && shared_premises.is_empty();
            let why = if holds {
                let setting_said = if shared_setting.is_empty() {
                    String::new()
                } else {
                    format!(
                        "; both start from {}, the vocabulary or the theorem's own setting, which \
                         leaves them independent",
                        shared_setting.join(", ")
                    )
                };
                if shares_named.is_empty() {
                    format!(
                        "the two lines share no step and no premise: {step} and {other} are \
                         independent{setting_said}"
                    )
                } else {
                    format!(
                        "the two lines share no step and no premise beyond the named lemma(s) \
                         {}{setting_said}",
                        shares_named.join(", ")
                    )
                }
            } else {
                let mut parts: Vec<String> = Vec::new();
                if !shared_steps.is_empty() {
                    parts.push(format!("the step(s) {}", shared_steps.join(", ")));
                }
                if !shared_citations.is_empty() {
                    parts.push(format!(
                        "the cited foundation(s) {} — a result or a piece of evidence neither \
                         line shows",
                        shared_citations.join(", ")
                    ));
                }
                if !shared_premises.is_empty() {
                    parts.push(format!(
                        "the premise(s) {} — neither line derives them, and losing one costs both",
                        shared_premises.join(", ")
                    ));
                }
                format!(
                    "the two lines both rest on {}, under no lemma the statement names, so they \
                     stand or fall together and are not independent as stated: name what they \
                     share in `independent_except` (a lemma, or a premise you accept both rest \
                     on), or the lines are one argument",
                    parts.join("; and ")
                )
            };
            out.push(IndependenceReading {
                step: step.to_string(),
                of: other,
                holds,
                shared_steps,
                shared_citations,
                shared_premises,
                shared_setting,
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
        // One evaluation for everything this reads (inference/memo.rs).
        let _shared = self.reasoning_scope();
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
            let mut ev = Eval::new(self, &lost);
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
        // AN OBJECTION OR A REPLY READS BY HOW IT STANDS AGAINST WHAT IT
        // ATTACKS (req:statement-kinds-cover-an-objection-a-reply-an-unstated-
        // premise-and-a-method-rule): a reply is an objection to an objection.
        if matches!(kind.as_str(), "objection" | "reply") {
            if truth == Truth::Fails {
                // ANSWERED: a reply that holds fails it, so it no longer
                // counts against anything.
                return Ok(Some(StatementReading {
                    kind,
                    reads: "answered",
                    why: format!("answered, so it no longer counts: {why}"),
                }));
            }
            return Ok(Some(self.attack_reading(id, &kind, why)?));
        }
        let contested = truth == Truth::Unknown
            && proof.is_none()
            && !self.premise_reading(id)?.contested_by.is_empty();
        // PR A2: a claim another takes the place of, or its source withdrew,
        // reads superseded; one accepted for a period that ended reads
        // withdrawn; one a named party claims and nothing here establishes
        // reads claimed — each before "refuted", "stated" or "open".
        let superseded = self.superseded_reason(id)?;
        let withdrawn = if superseded.is_none() {
            self.withdrawn_reading(id)?
        } else {
            None
        };
        let claimants = if truth != Truth::Holds && proof.is_none() {
            self.claimants_of(id, &BTreeSet::new())?
        } else {
            BTreeSet::new()
        };
        let (reads, why): (&'static str, String) = if let Some(s) = superseded {
            ("superseded", s)
        } else if let Some(w) = withdrawn {
            ("withdrawn", w.why)
        } else if truth == Truth::Fails {
            ("refuted", why)
        } else if contested {
            // A FOUNDATION UNDER A STANDING OBJECTION reads contested, never
            // "stated" (req:a-conclusion-whose-every-route-is-contested-reads-
            // contested, the stage-5 re-read).
            ("contested", why)
        } else if !claimants.is_empty() {
            ("claimed", Self::claimed_said(&claimants))
        } else if kind == "bridge" {
            // A BRIDGE READS AS A BRIDGE (dec:idea-a-bridge-premise-says-a-
            // formal-statement-models-a-claim, option b): neither a cited
            // result nor a foundation, and never "supported at α" — the four
            // colour trial's bridge, recorded as a proposition with evidence,
            // read as a statistical test.
            (
                "bridge",
                format!(
                    "a bridge: it says a formal statement models a claim of another field, so \
                     what rests on it holds only as far as the modelling is faithful, and an \
                     objection to it contests everything that rests on it; {why}"
                ),
            )
        } else if kind == "unstated_premise" {
            if is_taken_as_given(&self.node_props(id)?.unwrap_or_default()) {
                (
                    "taken_as_given",
                    format!(
                        "an unstated premise the argument relies on, made explicit and taken as \
                         given: {why}"
                    ),
                )
            } else if let Some(p) = proof {
                (
                    p.status.as_str(),
                    format!(
                        "an unstated premise the argument relies on, made explicit: {}",
                        p.why
                    ),
                )
            } else {
                (
                    "stated",
                    format!(
                        "an unstated premise the argument relies on, made explicit; nothing shows \
                         it and it is not taken as given: {why}"
                    ),
                )
            }
        } else if kind == "method_rule" && truth == Truth::Holds && proof.is_none() {
            (
                "taken_as_given",
                "a rule of method: how the reasoning proceeds, taken as given by choice"
                    .to_string(),
            )
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
        } else if kind == "evidence" && proof.is_none() {
            // CITED AS EVIDENCE, never as a theorem (req:evidence-carries-its-
            // source-grade-and-confidence-and-is-not-intent, extended
            // 2026-10-20): an empirical source names the report that
            // documents it.
            if self.pinned_sources(id)?.is_empty() {
                ("evidence_unverified", why)
            } else {
                ("cited_as_evidence", why)
            }
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
        let blank = |rule: String,
                     k: Option<i64>,
                     rule_name: Option<String>,
                     conclusion: Option<String>,
                     confirmation: Confirmation,
                     discharges: Vec<String>| InferenceReading {
            inference: inference_id.to_string(),
            rule,
            k,
            rule_name,
            rule_node: None,
            establishes: None,
            conclusion_form: None,
            holds_as: None,
            conclusion,
            verdict: Truth::Unknown,
            holding: 0,
            failing: 0,
            unknown: 0,
            premises: Vec::new(),
            note: None,
            confirmation,
            assumes: Vec::new(),
            rests_on_unverified: Vec::new(),
            rests_on_bridges: Vec::new(),
            circular: None,
            rests_on_tests: Vec::new(),
            discharges,
            cases: None,
            generalization: None,
            independence: Vec::new(),
            status_over_time: Vec::new(),
            withdrawn: None,
            claimed_by: None,
            holds_under_any_of: Vec::new(),
            lines_capped: false,
            objections: Vec::new(),
            objected: None,
            contested_by: Vec::new(),
            rests_on_refuted_suppositions: Vec::new(),
            lines: vec![Condition::default()],
        };
        let Some(rec) = self.step_record(inference_id)? else {
            let mut r = blank(
                "all".to_string(),
                None,
                None,
                conclusion,
                Confirmation::default(),
                Vec::new(),
            );
            r.note = Some("no Inference holds this id".to_string());
            return Ok(r);
        };
        let mut reading = blank(
            rec.rule.clone(),
            rec.k,
            rec.rule_name.clone(),
            conclusion,
            rec.confirmation.clone(),
            rec.discharges.clone(),
        );
        reading.establishes = rec.establishes.clone();
        reading.conclusion_form = rec.conclusion_form.clone();
        if ev.lost.contains(inference_id) {
            let (truth, why) = self.lost_truth(inference_id)?;
            reading.verdict = truth;
            reading.note = Some(why);
            return Ok(reading);
        }
        if ev.path.contains(inference_id) {
            ev.cuts += 1;
            ev.track.found_on(inference_id, Stack::Step);
            reading.note = Some(
                "it rests, through its premises, on its own conclusion: circular reasoning has \
                 no verdict"
                    .to_string(),
            );
            return Ok(reading);
        }
        ev.track.found_off(inference_id, Stack::Step);
        if depth == 0 {
            note_truncation();
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
        //
        // EACH PREMISE KEEPS EVERY LINE IT HOLDS ON (req:a-claim-with-several-
        // lines-reads-each-lines-assumptions): `parts` holds, per premise, its
        // truth and its minimal lines, and the step's own lines are taken from
        // them by its rule below.
        let mut values = Vec::new();
        let mut parts: Vec<(Truth, Vec<Condition>)> = Vec::new();
        // The refuted suppositions this step, or a premise it rests on, was
        // derived under.
        let mut refuted_sup: BTreeSet<String> = BTreeSet::new();
        // Per premise, what holds it up when only a dispute does.
        let mut disputed: Vec<BTreeSet<String>> = Vec::new();
        // A PREMISE SAYS WHAT IT RESTS ON (dec:a-premise-says-whether-it-
        // rests-on-intent-or-delivery, Anthony 2026-10-24): its owner's
        // INTENT or its DELIVERY, on its link; unmarked, intent when this
        // step concludes a Decision and delivery for every other step.
        let marks = self.premise_marks(inference_id)?;
        let concludes_a_decision = match reading.conclusion.as_deref() {
            Some(c) => self.type_holding(c)?.as_deref() == Some(node::DECISION),
            None => false,
        };
        for (p, assumed) in self.premise_links(inference_id)? {
            if assumed {
                // AN OPEN SUPPOSITION — one no step anywhere discharges, a
                // principle an argument applies, a hypothesis left open — is
                // read for what it is outside the argument, twice:
                let open = !self.discharged_somewhere(&p, ev)?;
                // (A what-if that DROPS it is read at the claims, not here:
                // inside the argument it still holds, and a step that later
                // discharges it is untouched — see `without_dropped`.)
                // REFUTED OUTSIDE (req:a-claim-derived-under-a-refuted-
                // supposition-says-so, the capstone trial): inside the argument
                // it holds whatever its own truth, which is correct logic; but
                // once it reads refuted, what is derived under it is no live
                // conditional support and no standing objection.
                if open && self.truth_full(&p, ev, depth - 1)?.truth == Truth::Fails {
                    values.push(Truth::Unknown);
                    disputed.push(BTreeSet::new());
                    parts.push((Truth::Unknown, vec![Condition::default()]));
                    refuted_sup.insert(p.clone());
                    reading.premises.push(PremiseReading {
                        node_type: self.type_holding(&p)?,
                        id: p,
                        truth: Truth::Unknown,
                        why: "assumed, but the supposition reads refuted outside the argument: \
                              what is derived under it rests on a refuted supposition"
                            .to_string(),
                        role: Some("assumed"),
                        name: None,
                        uses: None,
                        reads_as: None,
                    });
                    continue;
                }
                values.push(Truth::Holds);
                disputed.push(BTreeSet::new());
                parts.push((
                    Truth::Holds,
                    vec![Condition {
                        assumes: BTreeSet::from([p.clone()]),
                        ..Condition::default()
                    }],
                ));
                reading.premises.push(PremiseReading {
                    node_type: self.type_holding(&p)?,
                    id: p,
                    truth: Truth::Holds,
                    why: "assumed: taken up by this step for the sake of argument, and held \
                          inside it whatever its own truth"
                        .to_string(),
                    role: Some("assumed"),
                    name: None,
                    uses: None,
                    reads_as: None,
                });
                continue;
            }
            let reads_as =
                self.reads_as_with(&p, marks.get(&p).map(String::as_str), concludes_a_decision)?;
            // A FALSUM is read as a premise by whether it is DERIVED: a
            // reductio rests on "⊥ follows", never on "⊥ is true".
            let r = if self.node_props(&p)?.is_some_and(|props| is_falsum(&props)) {
                self.falsum_derivation(&p, ev, depth - 1)?
            } else if reads_as == Some(ReadsAs::Intent) {
                let conclusion = reading.conclusion.clone();
                self.intent_full(&p, conclusion.as_deref(), ev, depth - 1)?
            } else {
                self.truth_full(&p, ev, depth - 1)?
            };
            values.push(r.truth);
            reading.lines_capped |= r.lines_capped;
            if r.truth != Truth::Holds {
                refuted_sup.extend(r.refuted_under.iter().cloned());
            }
            disputed.push(if r.truth == Truth::Unknown {
                r.contested_by.clone()
            } else {
                BTreeSet::new()
            });
            parts.push((r.truth, r.lines));
            reading.premises.push(PremiseReading {
                node_type: self.type_holding(&p)?,
                id: p,
                truth: r.truth,
                why: r.why,
                role: None,
                name: None,
                uses: None,
                reads_as: reads_as.and_then(ReadsAs::served),
            });
        }
        for (p, n) in self.premise_uses(inference_id)? {
            if let Some(pr) = reading.premises.iter_mut().find(|pr| pr.id == p) {
                pr.uses = Some(n);
            }
        }
        reading.holding = values.iter().filter(|v| **v == Truth::Holds).count();
        reading.failing = values.iter().filter(|v| **v == Truth::Fails).count();
        reading.unknown = values.len() - reading.holding - reading.failing;
        let parsed = rec.parsed.clone();
        match parsed.clone().and_then(|rule| combine(rule, &values)) {
            Ok(v) => reading.verdict = v,
            Err(why) => reading.note = Some(why),
        }
        // WOULD IT HOLD WERE EVERY DISPUTE UNDER IT SETTLED ITS WAY? Each
        // premise held up only by a dispute counted as holding: if the rule
        // then holds, the step is a contested route, not an unsupported one.
        let settled: Vec<Truth> = values
            .iter()
            .zip(&disputed)
            .map(|(v, d)| if d.is_empty() { *v } else { Truth::Holds })
            .collect();
        let would_hold = parsed
            .clone()
            .and_then(|rule| combine(rule, &settled))
            .is_ok_and(|v| v == Truth::Holds);
        // A gate the step's own structure fails (a case split nothing
        // declares exhaustive, a generalization over a fixed variable) is no
        // dispute: such a step is not contested, whatever stands elsewhere.
        let mut gated = false;
        // A CASE SPLIT HOLDS ONLY WHEN ITS CASES ARE DECLARED EXHAUSTIVE
        // (item 2). Which record declares it is the step's to say; whether it
        // holds is that record's own computed truth — the owner's word (taken
        // as given) or a checker's (a passing computed check) — never
        // reflow2's judgement of the cases.
        let mut declared_by: Option<(Truth, Vec<Condition>)> = None;
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
                // A DISJUNCTION ASSUMED IN THE SAME SUBPROOF (item 1, the
                // logic rung): a supposition is read as ASSUMED here — it
                // holds inside the argument, carried as an assumption until a
                // step discharges it — never by its standing status.
                Some(by) if self.is_supposition(by)? => {
                    let line = Condition {
                        assumes: BTreeSet::from([by.clone()]),
                        ..Condition::default()
                    };
                    declared_by = Some((Truth::Holds, vec![line]));
                    CasesReading {
                        cases: rec.cases.clone(),
                        declared: true,
                        exhaustive_by: Some(by.clone()),
                        truth: Some(Truth::Holds),
                        why: format!(
                            "the cases {} are declared exhaustive by {by}, a disjunction assumed \
                             in the argument: read as assumed here, so the split holds under it \
                             until a step discharges it",
                            rec.cases.join(", ")
                        ),
                    }
                }
                Some(by) => {
                    let r = self.truth_full(by, ev, depth - 1)?;
                    reading.lines_capped |= r.lines_capped;
                    declared_by = Some((r.truth, r.lines.clone()));
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
                gated = true;
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
        // THE RULE IS A REFERENCE OF ITS OWN (item 2, the logic rung): the
        // step holds only while the node stating its rule holds, and what
        // that node holds under is carried like a premise's — but it is
        // never a premise, never uncited, and never what a claim rests on.
        let mut ruled_by: Option<Vec<Condition>> = None;
        if let Some(rn) = &rec.rule_node {
            let r = self.truth_full(rn, ev, depth - 1)?;
            reading.lines_capped |= r.lines_capped;
            if r.truth == Truth::Holds {
                ruled_by = Some(r.lines.clone());
            } else {
                gated = true;
                if reading.verdict == Truth::Holds {
                    reading.verdict = Truth::Unknown;
                }
                let why = format!(
                    "its rule {rn} does not hold ({}): {}; a step holds only while its rule does",
                    r.truth.as_str(),
                    r.why
                );
                reading.note = Some(match reading.note.take() {
                    Some(n) => format!("{n}; {why}"),
                    None => why,
                });
            }
            reading.rule_node = Some(RuleNodeReading {
                id: rn.clone(),
                truth: r.truth,
                why: r.why,
            });
        }
        // WHAT THE STEP HOLDS UNDER, LINE BY LINE. A holding step takes, per
        // premise its rule needs, the line of that premise that costs least
        // together with the others' — every such combination, minimal — so a
        // step citing a claim held on several lines inherits only what its
        // chosen line needs, never another line's assumption (the trolley
        // trial: a Thomson-2008 step inherited Foot's). `any` holds on any one
        // holding premise's lines; `k_of_n` on any k of them. A step that does
        // not hold reads, as it always has, the union of what every premise
        // rests on.
        let mut union = Condition::default();
        for (_, lines) in &parts {
            union = union.union(&lines[0]);
        }
        if let Some((_, lines)) = &declared_by {
            union = union.union(&lines[0]);
        }
        if let Some(lines) = &ruled_by {
            union = union.union(&lines[0]);
        }
        for d in rec.discharges.iter().chain(rec.cases.iter()) {
            union.assumes.remove(d);
        }
        let mut lines: Vec<Condition> = if reading.verdict == Truth::Holds {
            let holding: Vec<&Vec<Condition>> = parts
                .iter()
                .filter(|(t, _)| *t == Truth::Holds)
                .map(|(_, l)| l)
                .collect();
            let (mut lines, capped) = match parsed {
                Ok(InferenceRule::Any) => {
                    minimal_lines(holding.iter().flat_map(|l| l.iter().cloned()).collect())
                }
                Ok(InferenceRule::KOfN(k)) => self.k_of_n_lines(&holding, k),
                _ => cross_lines(&parts.iter().map(|(_, l)| l).collect::<Vec<_>>()),
            };
            reading.lines_capped |= capped;
            if let Some((_, by)) = &declared_by {
                let (crossed, capped) = cross_lines(&[&lines, by]);
                reading.lines_capped |= capped;
                lines = crossed;
            }
            if let Some(by) = &ruled_by {
                let (crossed, capped) = cross_lines(&[&lines, by]);
                reading.lines_capped |= capped;
                lines = crossed;
            }
            lines
        } else {
            vec![union.clone()]
        };
        // The cases of a case split are discharged by it, as `discharges`
        // would (∨-elimination closes each case's hypothesis).
        for line in &mut lines {
            for d in rec.discharges.iter().chain(rec.cases.iter()) {
                line.assumes.remove(d);
            }
        }
        let (lines, capped) = minimal_lines(lines);
        reading.lines_capped |= capped;
        let assumes = lines[0].assumes.clone();
        reading.rests_on_unverified = lines[0].unverified.iter().cloned().collect();
        reading.rests_on_bridges = lines[0].bridges.iter().cloned().collect();
        reading.rests_on_tests = lines[0].evidential.iter().cloned().collect();
        // A GENERALIZATION IS CHECKED (Anthony's choice of 2026-10-17:
        // declared and checked). A step generalizing over a variable that
        // something it rests on FIXES does not hold: its verdict reads
        // unknown with the reason, so its conclusion is unsupported, never
        // refuted.
        // THE EIGENVARIABLE RULE (2026-10-18, the Arrow trial's N19): and
        // over a variable no assumption still OPEN at this step names. The
        // open ones are `assumes`, read after this step's own discharges —
        // the lightest line's, so a step whose cheapest line is clean holds.
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
                gated = true;
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
        reading.lines = lines;
        // AN OBJECTION AIMED AT THE STEP ITSELF (req:an-objection-aimed-at-a-
        // step-is-read-against-that-step, Anthony 2026-10-18; the cogito and
        // trolley trials, idea (b) of the argumentation frameworks): an
        // opposing CONTRADICTS into the step attacks its warrant or rule, not
        // a premise or the conclusion. It counts as far as it holds, judged
        // against what the step holds under, exactly as an objection to a
        // claim is: one that HOLDS defeats the step, which then fails and
        // supports nothing (its conclusion is left unsupported, never
        // refuted); one that STANDS contests it, leaving it at most unknown;
        // one that is ANSWERED no longer counts, and the step is reinstated.
        // Until 2026-10-20 such an edge was stored and never read: the step
        // read `holds` and the read-back did not list it.
        let scope = Read::with_lines(
            reading.verdict,
            String::new(),
            reading.lines.clone(),
            reading.lines_capped,
        );
        let before_objections = reading.verdict;
        let objections = self.objections_within(inference_id, ev, depth, &scope)?;
        ev.path.remove(inference_id);
        if !objections.is_empty() {
            let refuting: Vec<&str> = objections
                .iter()
                .filter(|o| o.standing == "refutes")
                .map(|o| o.id.as_str())
                .collect();
            let conceded: Vec<String> = objections
                .iter()
                .filter(|o| o.standing == "conceded")
                .map(|o| {
                    format!(
                        "{} grants {} against it{}",
                        o.conceded_by.as_deref().unwrap_or(""),
                        o.id,
                        o.moves_to
                            .as_deref()
                            .map(|m| format!(" and moves the argument to {m}"))
                            .unwrap_or_default()
                    )
                })
                .collect();
            let standing: Vec<&str> = objections
                .iter()
                .filter(|o| o.standing == "stands")
                .map(|o| o.id.as_str())
                .collect();
            let (objected, why) = if !conceded.is_empty() {
                // CONCEDED (req:a-reply-can-concede-an-objection-against-one-
                // reading-and-move-to-another): the arguer grants the
                // objection against this reading and moves to another, so
                // this route supports nothing and the move is named.
                reading.verdict = Truth::Fails;
                (
                    "conceded",
                    format!(
                        "conceded: {}; this reading supports nothing, and the argument is meant \
                         as the one it moves to",
                        conceded.join("; ")
                    ),
                )
            } else if !refuting.is_empty() {
                reading.verdict = Truth::Fails;
                (
                    "defeated",
                    format!(
                        "defeated by {}, which holds: an objection to this step itself (its \
                         warrant or rule), not to a premise or the conclusion, and nothing answers \
                         it, so the step supports nothing; its conclusion is left unsupported by \
                         it, never refuted",
                        refuting.join(", ")
                    ),
                )
            } else if !standing.is_empty() {
                if reading.verdict == Truth::Holds {
                    reading.verdict = Truth::Unknown;
                }
                (
                    "contested",
                    format!(
                        "contested: {} stand(s) against this step itself (its warrant or rule), \
                         neither established nor answered, so it supports its conclusion only once \
                         that is settled",
                        standing.join(", ")
                    ),
                )
            } else {
                (
                    "answered",
                    "every objection to this step itself is answered, so it reads by its rule \
                     again"
                        .to_string(),
                )
            };
            reading.objected = Some(objected);
            if objected != "answered" {
                reading.note = Some(match reading.note.take() {
                    Some(n) => format!("{n}; {why}"),
                    None => why,
                });
            }
            reading.objections = objections;
        }
        if reading.verdict != Truth::Holds && !refuted_sup.is_empty() {
            let names: Vec<String> = refuted_sup.iter().cloned().collect();
            let why = format!(
                "it rests on the refuted supposition(s) {}: held inside the argument, but refuted \
                 outside it, so what it concludes is no live conditional support",
                names.join(", ")
            );
            reading.note = Some(match reading.note.take() {
                Some(n) => format!("{n}; {why}"),
                None => why,
            });
            reading.rests_on_refuted_suppositions = names;
        }
        // A CONTESTED ROUTE: unknown, and held up only by disputes — the
        // objections standing against the step itself, and those under the
        // premises it rests on.
        if reading.verdict == Truth::Unknown && !gated {
            let mut by: BTreeSet<String> = BTreeSet::new();
            let step_contested = reading.objected == Some("contested");
            if step_contested {
                by.extend(
                    reading
                        .objections
                        .iter()
                        .filter(|o| o.standing == "stands")
                        .map(|o| o.id.clone()),
                );
            }
            let premises_hold_up = before_objections == Truth::Holds
                || (before_objections == Truth::Unknown && would_hold);
            if premises_hold_up && (step_contested || before_objections == Truth::Unknown) {
                if before_objections == Truth::Unknown {
                    for d in &disputed {
                        by.extend(d.iter().cloned());
                    }
                }
                reading.contested_by = by.into_iter().collect();
            }
        }
        // A step that does not hold reads, as it always has, the union of
        // what every premise rests on.
        if reading.verdict != Truth::Holds {
            reading.assumes = union.assumes.iter().cloned().collect();
            reading.rests_on_unverified = union.unverified.iter().cloned().collect();
            reading.rests_on_bridges = union.bridges.iter().cloned().collect();
            reading.rests_on_tests = union.evidential.iter().cloned().collect();
            reading.lines = vec![union];
            reading.lines_capped = false;
        } else if reading.lines.len() > 1 {
            reading.holds_under_any_of = reading.lines.iter().map(LineConditions::of).collect();
        }
        // HOW IT HOLDS, AT THE STEP (item 7, the logic rung): a step resting
        // on an unverified citation or an open assumption is conditional
        // here too, not only at its claim; one resting on evidence is
        // supported. Until 2026-10-20 the step read plain `holds`.
        if reading.verdict == Truth::Holds {
            reading.holds_as = if !reading.assumes.is_empty()
                || !reading.rests_on_unverified.is_empty()
                || !reading.rests_on_bridges.is_empty()
            {
                Some("conditional")
            } else if !reading.rests_on_tests.is_empty() {
                Some("supported")
            } else {
                None
            };
        }
        Ok(reading)
    }

    /// The lines a `k_of_n` step holds on: any `k` of its holding premises,
    /// each combination's lines crossed. The number of combinations is bounded
    /// ([`K_OF_N_COMBINATIONS_CAP`]); past it every holding premise is taken
    /// together — never a lighter line than the step really has — and the
    /// reading says it was capped.
    fn k_of_n_lines(&self, holding: &[&Vec<Condition>], k: i64) -> (Vec<Condition>, bool) {
        let n = holding.len();
        let k = usize::try_from(k).unwrap_or(0).clamp(1, n.max(1));
        let mut count: u128 = 1;
        for i in 0..k {
            count = count * (n - i) as u128 / (i + 1) as u128;
            if count > K_OF_N_COMBINATIONS_CAP as u128 {
                let (lines, _) = cross_lines(holding);
                return (lines, true);
            }
        }
        let mut all: Vec<Condition> = Vec::new();
        let mut capped = false;
        let mut pick: Vec<usize> = (0..k).collect();
        loop {
            let chosen: Vec<&Vec<Condition>> = pick.iter().map(|i| holding[*i]).collect();
            let (lines, c) = cross_lines(&chosen);
            capped |= c;
            all.extend(lines);
            // The next combination, in order.
            let mut i = k;
            while i > 0 && pick[i - 1] == n - k + (i - 1) {
                i -= 1;
            }
            if i == 0 {
                break;
            }
            pick[i - 1] += 1;
            for j in i..k {
                pick[j] = pick[j - 1] + 1;
            }
        }
        let (lines, c) = minimal_lines(all);
        (lines, capped || c)
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

    /// How an `objection` or a `reply` reads: by how it stands against each
    /// record it attacks — `refutes` (it holds; for a reply, `answers`),
    /// `stands`, `answered`, or `conceded` / `concedes` when a reply grants
    /// it. One that attacks nothing yet reads `stated`.
    fn attack_reading(
        &self,
        id: &str,
        kind: &str,
        why: String,
    ) -> Result<StatementReading, DynoError> {
        let mut standings: Vec<(String, &'static str)> = Vec::new();
        let mut concedes: Vec<String> = Vec::new();
        for e in self.outgoing(id, Some(edge::CONTRADICTS))? {
            match e.properties.get("alignment").and_then(Value::as_str) {
                Some("supporting") => continue,
                Some("conceding") => {
                    concedes.push(e.to_id);
                    continue;
                }
                _ => {}
            }
            let st = self
                .objections_to(&e.to_id)?
                .into_iter()
                .find(|o| o.id == id)
                .map(|o| o.standing)
                .unwrap_or("stands");
            standings.push((e.to_id, st));
        }
        if !concedes.is_empty() && standings.is_empty() {
            return Ok(StatementReading {
                kind: kind.to_string(),
                reads: "concedes",
                why: format!(
                    "a reply that grants {} against what it attacks, and moves the argument on",
                    concedes.join(", ")
                ),
            });
        }
        let pick = |want: &str| -> Vec<String> {
            standings
                .iter()
                .filter(|(_, s)| *s == want)
                .map(|(t, _)| t.clone())
                .collect()
        };
        let (reads, against): (&'static str, Vec<String>) = if !pick("conceded").is_empty() {
            ("conceded", pick("conceded"))
        } else if !pick("refutes").is_empty() {
            (
                if kind == "reply" {
                    "answers"
                } else {
                    "refutes"
                },
                pick("refutes"),
            )
        } else if !pick("stands").is_empty() {
            ("stands", pick("stands"))
        } else if !pick("answered").is_empty() {
            ("answered", pick("answered"))
        } else {
            return Ok(StatementReading {
                kind: kind.to_string(),
                reads: "stated",
                why: format!(
                    "a{} {kind} aimed at nothing yet: {why}",
                    if kind == "objection" { "n" } else { "" }
                ),
            });
        };
        Ok(StatementReading {
            kind: kind.to_string(),
            reads,
            why: format!("{reads} against {}: {why}", against.join(", ")),
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
        let mut ev = Eval::new(self, &lost);
        let mut holding = Vec::new();
        for s in self.inferences_concluding(id)? {
            let r = self.step_reading(&s, &mut ev, depth)?;
            if r.verdict == Truth::Holds
                && r.assumes.is_empty()
                && r.rests_on_unverified.is_empty()
                && r.rests_on_bridges.is_empty()
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
        Ok(self.inference_walk_full(seeds)?.rows)
    }

    /// How the change walk reads a conclusion: by its truth, except a FALSUM,
    /// which it reads by its DERIVATION ([`Self::falsum_derivation`]) — what a
    /// reductio rests on is "⊥ follows", and a ⊥'s own truth is false before
    /// and after any change.
    fn walk_read(&self, id: &str, ev: &mut Eval<'_>) -> Result<Read, DynoError> {
        if self.node_props(id)?.is_some_and(|p| is_falsum(&p)) {
            self.falsum_derivation(id, ev, INFERENCE_DEPTH)
        } else {
            self.truth_full(id, ev, INFERENCE_DEPTH)
        }
    }

    /// [`Self::inference_walk`], with what the change brings BACK and what it
    /// leaves STALE beside the rows: every objection whose reply the change
    /// takes away (reinstatement in reverse), and every check whose
    /// conditions it leaves without support.
    pub fn inference_walk_full(&self, seeds: &[&str]) -> Result<InferenceWalk, DynoError> {
        // One evaluation for everything this reads (inference/memo.rs).
        let _shared = self.reasoning_scope();
        let seed_set: BTreeSet<&str> = seeds.iter().copied().collect();
        let mut lost: BTreeSet<String> = seeds.iter().map(|s| s.to_string()).collect();
        let mut lost_at: BTreeMap<String, usize> =
            seeds.iter().map(|s| (s.to_string(), 0)).collect();
        // Every step reached, with the distance it was first reached at.
        let mut reached: BTreeMap<String, usize> = BTreeMap::new();
        let mut frontier: Vec<(String, usize)> = seeds.iter().map(|s| (s.to_string(), 1)).collect();
        // The open suppositions (no step discharges them): dropping one costs
        // every step that takes it up (req:a-what-if-on-an-assumed-principle-
        // shows-what-holds-only-under-it).
        let before_set = BTreeSet::new();
        let mut before_ev = Eval::new(self, &before_set);
        let mut returning: Vec<ObjectionReturning> = Vec::new();
        let mut returned: BTreeSet<String> = BTreeSet::new();
        // Claims that hold only under a dropped assumption: reached, not lost.
        let mut under_dropped: BTreeSet<String> = BTreeSet::new();
        // Which steps each rule node is the rule of, read once and only when
        // something lost might be one.
        let mut rule_index: Option<BTreeMap<String, Vec<String>>> = None;
        fn ruled<'a>(
            g: &DesignGraph,
            index: &'a mut Option<BTreeMap<String, Vec<String>>>,
        ) -> Result<&'a BTreeMap<String, Vec<String>>, DynoError> {
            if index.is_none() {
                let mut m: BTreeMap<String, Vec<String>> = BTreeMap::new();
                for n in g.scan_nodes(node::INFERENCE)? {
                    if let Some(r) = prop(&n.properties, "rule_node").map(str::trim)
                        && !r.is_empty()
                    {
                        m.entry(r.to_string()).or_default().push(n.node_id);
                    }
                }
                *index = Some(m);
            }
            Ok(index.as_ref().expect("filled above"))
        }
        loop {
            let mut frontier_later: Vec<(String, usize)> = Vec::new();
            while let Some((x, distance)) = frontier.pop() {
                let mut rests: Vec<String> = self.steps_resting_on(&x)?;
                if lost.contains(&x) {
                    rests.extend(self.steps_assuming(&x)?);
                    // A lost RULE costs every step it is the rule of (item 2).
                    if let Some(steps) = ruled(self, &mut rule_index)?.get(&x) {
                        rests.extend(steps.iter().cloned());
                    }
                }
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
            // turn — and past one that was only CONTESTED and is now left with
            // nothing (req:a-what-if-shows-what-a-premise-costs-a-contested-
            // conclusion-and-objections-that-return: "route B loses its last
            // unobjected premise"). Losses only grow, so a conclusion that
            // stood a round ago may fall now: re-read until nothing new is lost.
            let mut ev = Eval::new(self, &lost);
            let mut newly: Vec<(String, usize)> = Vec::new();
            for (s, distance) in &reached {
                let Some(c) = self.conclusion_of(s)? else {
                    continue;
                };
                if lost.contains(&c) || newly.iter().any(|(n, _)| *n == c) {
                    continue;
                }
                // A FALSUM IS READ BY ITS DERIVATION HERE TOO (the Lucas–
                // Penrose trial, 2026-10-22: "the what-if walk stops at the
                // falsum and never reaches the reductio or the conclusion").
                // Its own truth is false before and after any change, so read
                // that way nothing about it ever "fell", and the walk ended at
                // the ⊥. What a reductio rests on is "⊥ follows", so whether
                // the change costs the ⊥ is whether it costs its derivation.
                let raw = self.walk_read(&c, &mut ev)?;
                let after = self.without_dropped(raw.clone(), &lost);
                let before = self.walk_read(&c, &mut before_ev)?;
                // HELD ONLY UNDER A DROPPED ASSUMPTION: it still holds inside
                // the argument, so it is not counted lost — a step that
                // discharges the assumption above it is untouched (a reductio
                // whose supposition is dropped still holds) — but what rests
                // on it is reached and read without the dropped lines.
                if before.truth == Truth::Holds
                    && raw.truth == Truth::Holds
                    && after.truth != Truth::Holds
                {
                    if under_dropped.insert(c.clone()) {
                        frontier_later.push((c, distance + 1));
                    }
                    continue;
                }
                let fell = before.truth == Truth::Holds && after.truth != Truth::Holds;
                let lost_its_dispute = before.truth == Truth::Unknown
                    && !before.contested_by.is_empty()
                    && after.truth == Truth::Unknown
                    && after.contested_by.is_empty();
                if fell || lost_its_dispute {
                    newly.push((c, distance + 1));
                }
            }
            // OBJECTIONS THAT COME BACK: a reply the change takes away no
            // longer answers its objection, which counts again against what
            // it attacks — and what rests on that is reached in turn.
            let mut came_back: Vec<(String, usize)> = Vec::new();
            for (x, at) in lost_at.clone() {
                for e in self.outgoing(&x, Some(edge::CONTRADICTS))? {
                    if matches!(
                        e.properties.get("alignment").and_then(Value::as_str),
                        Some("supporting" | "conceding")
                    ) {
                        continue;
                    }
                    let o = e.to_id;
                    if returned.contains(&o) {
                        continue;
                    }
                    let was = self.truth_full(&o, &mut before_ev, INFERENCE_DEPTH)?.truth;
                    let now = self.truth_full(&o, &mut ev, INFERENCE_DEPTH)?.truth;
                    if was != Truth::Fails || now == Truth::Fails {
                        continue;
                    }
                    returned.insert(o.clone());
                    let against = self.attack_targets_of_objection(&o)?;
                    returning.push(ObjectionReturning {
                        objection: o.clone(),
                        answered_by: x.clone(),
                        now: if now == Truth::Holds {
                            "refutes"
                        } else {
                            "stands"
                        },
                        why: format!(
                            "{x} answered it, and the change takes {x} away: it {} again against \
                             {}",
                            if now == Truth::Holds {
                                "holds"
                            } else {
                                "stands"
                            },
                            against.join(", ")
                        ),
                        against: against.clone(),
                        distance: at + 1,
                    });
                    for t in against {
                        came_back.push((t, at + 1));
                    }
                }
            }
            if newly.is_empty() && came_back.is_empty() && frontier_later.is_empty() {
                break;
            }
            frontier.extend(frontier_later);
            for (c, distance) in newly {
                lost.insert(c.clone());
                lost_at.insert(c.clone(), distance);
                frontier.push((c, distance));
            }
            frontier.extend(came_back);
        }

        let mut after_ev = Eval::new(self, &lost);
        let mut rows = Vec::new();
        for (s, distance) in &reached {
            let before = self.step_reading(s, &mut before_ev, INFERENCE_DEPTH)?;
            // The step's own verdict with the losses counted — not "the step
            // is lost" when it is itself a seed: read its rule over what is left.
            let after = if lost.contains(s) {
                let without_itself: BTreeSet<String> =
                    lost.iter().filter(|x| *x != s).cloned().collect();
                let mut ev = Eval::new(self, &without_itself);
                let mut r = self.step_reading(s, &mut ev, INFERENCE_DEPTH)?;
                let (truth, why) = self.lost_truth(s)?;
                r.verdict = truth;
                r.note = Some(why);
                r
            } else {
                let r = self.step_reading(s, &mut after_ev, INFERENCE_DEPTH)?;
                self.step_without_dropped(r, &lost)
            };
            let (conclusion_was, conclusion_after, conclusion_read) = match &after.conclusion {
                Some(c) => {
                    let was = self.walk_read(c, &mut before_ev)?;
                    // A CLAIM THE WALK COUNTED LOST IS READ ON WHAT IS LEFT,
                    // never as "lost" (req:a-conclusion-whose-every-route-is-
                    // contested-reads-contested; the Gettier rung, 2026-10-23:
                    // "JTB is not sufficient" read `unsupported` while its
                    // untouched barn route was contested). Losing it is what
                    // carries the walk on; what it is left with is read over
                    // every other loss, as a lost step is.
                    let now = if lost.contains(c) && !seed_set.contains(c.as_str()) {
                        let without_it: BTreeSet<String> =
                            lost.iter().filter(|x| *x != c).cloned().collect();
                        let mut ev = Eval::new(self, &without_it);
                        self.walk_read(c, &mut ev)?
                    } else {
                        self.walk_read(c, &mut after_ev)?
                    };
                    let now = self.without_dropped(now, &lost);
                    // A CLAIM A NAMED PARTY MAKES reads `claimed` before the
                    // change (PR A2), never "unsupported".
                    let was_reads = match reads_as(&was) {
                        "unsupported" if !self.claimants_of(c, &BTreeSet::new())?.is_empty() => {
                            "claimed"
                        }
                        other => other,
                    };
                    (was_reads, now.truth, Some(now))
                }
                None => ("unsupported", Truth::Unknown, None),
            };
            // Who still claims it with the losses counted: dropping the
            // claimant's argument drops the claim, and the statement returns
            // to open (req:a-claim-made-by-a-named-party-and-not-yet-
            // verified-reads-claimed).
            let claimed_by_after: Vec<String> = match &after.conclusion {
                Some(c) if conclusion_after == Truth::Unknown => {
                    self.claimants_of(c, &lost)?.into_iter().collect()
                }
                _ => Vec::new(),
            };
            // The other lines: every step concluding the claim that still
            // holds with the losses counted, and every one the change does not
            // reach at all, whatever it reads.
            let mut still_supported_by = Vec::new();
            let mut untouched_routes = Vec::new();
            if let Some(c) = &after.conclusion {
                for other in self.inferences_concluding(c)? {
                    if lost.contains(&other) {
                        continue;
                    }
                    if conclusion_after == Truth::Holds && {
                        let r = self.step_reading(&other, &mut after_ev, INFERENCE_DEPTH)?;
                        self.step_without_dropped(r, &lost).verdict == Truth::Holds
                    } {
                        still_supported_by.push(other.clone());
                    }
                    if !reached.contains_key(&other)
                        && self.rests_on_closure(&other)?.is_disjoint(&lost)
                    {
                        untouched_routes.push(other);
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
            let conclusion_reads = match (&conclusion_read, conclusion_after) {
                (_, Truth::Holds) if held_by_acceptance_alone => "accepted_without_a_holding_chain",
                (_, Truth::Holds) => "still_supported",
                (_, Truth::Fails) => "refuted",
                (Some(r), Truth::Unknown) if !r.contested_by.is_empty() => "contested",
                (_, Truth::Unknown) if !claimed_by_after.is_empty() => "claimed",
                (_, Truth::Unknown) if conclusion_was == "claimed" => "open",
                (_, Truth::Unknown) => "unsupported",
            };
            let still_holds_under = match &conclusion_read {
                Some(r) if conclusion_reads == "still_supported" => {
                    r.lines.iter().map(LineConditions::of).collect()
                }
                _ => Vec::new(),
            };
            let lost_premises: Vec<String> = after
                .premises
                .iter()
                .filter(|p| lost.contains(&p.id))
                .map(|p| p.id.clone())
                .collect();
            let mut bridges_lost = Vec::new();
            for p in &lost_premises {
                if self
                    .node_props(p)?
                    .is_some_and(|props| statement_kind(&props) == Some("bridge"))
                {
                    bridges_lost.push(p.clone());
                }
            }
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
                bridges_lost,
                conclusion_after,
                conclusion_was,
                conclusion_reads,
                claimed_by: claimed_by_after,
                still_supported_by,
                still_holds_under,
                untouched_routes,
                rests_on_refuted_suppositions: after.rests_on_refuted_suppositions,
                parts: if after.conclusion_form.is_some() {
                    after
                        .premises
                        .iter()
                        .map(|p| PartReading {
                            id: p.id.clone(),
                            reads: match p.truth {
                                Truth::Holds => "stands",
                                Truth::Fails => "refuted",
                                Truth::Unknown => "unsupported",
                            },
                        })
                        .collect()
                } else {
                    Vec::new()
                },
                premises: after.premises,
            });
        }
        rows.sort_by(|a, b| {
            a.distance
                .cmp(&b.distance)
                .then_with(|| a.inference.cmp(&b.inference))
        });
        // CHECKS GONE STALE (req:a-check-resting-on-a-claim-that-loses-
        // support-reads-stale): a check constrained by a claim the change
        // leaves without support, named with each condition that no longer
        // holds.
        let mut touched: BTreeSet<String> = lost.clone();
        touched.extend(rows.iter().filter_map(|r| r.conclusion.clone()));
        let mut checks: BTreeSet<String> = BTreeSet::new();
        for t in &touched {
            for e in self.outgoing(t, Some(edge::CONSTRAINS))? {
                if self.get_node(node::VERIFICATION, &e.to_id)?.is_some() {
                    checks.insert(e.to_id);
                }
            }
        }
        let mut checks_gone_stale = Vec::new();
        for check in checks {
            if self
                .stale_conditions(&check, &mut before_ev, INFERENCE_DEPTH)?
                .is_some()
            {
                continue;
            }
            if let Some(failing) = self.stale_conditions(&check, &mut after_ev, INFERENCE_DEPTH)? {
                checks_gone_stale.push(StaleCheck {
                    check,
                    conditions: failing,
                });
            }
        }
        Ok(InferenceWalk {
            rows,
            objections_returning: returning,
            checks_gone_stale,
        })
    }

    /// A reading in a what-if that DROPS assumptions (req:a-what-if-on-an-
    /// assumed-principle-shows-what-holds-only-under-it): every line that
    /// still rests on a dropped assumption — one taken up and not
    /// discharged on the way here — goes. With none left, what held reads
    /// UNSUPPORTED in the what-if; with some left, it holds on those.
    /// Inside a step that discharges the assumption nothing changes, which
    /// is why this is read at a claim and never at a step's premise.
    fn without_dropped(&self, r: Read, lost: &BTreeSet<String>) -> Read {
        if r.truth != Truth::Holds || lost.is_empty() {
            return r;
        }
        let keep: Vec<Condition> = r
            .lines
            .iter()
            .filter(|l| l.assumes.is_disjoint(lost))
            .cloned()
            .collect();
        if keep.len() == r.lines.len() {
            return r;
        }
        if keep.is_empty() {
            let dropped: BTreeSet<String> = r
                .lines
                .iter()
                .flat_map(|l| l.assumes.intersection(lost).cloned())
                .collect();
            return Read::plain(
                Truth::Unknown,
                format!(
                    "unsupported in this what-if: every line it held on rests on the dropped \
                     assumption(s) {}",
                    dropped.into_iter().collect::<Vec<_>>().join(", ")
                ),
            );
        }
        let why = format!(
            "{}; without the dropped assumption(s) it holds on its remaining line(s)",
            r.why
        );
        Read::with_lines(Truth::Holds, why, keep, r.lines_capped)
    }

    /// A step's reading in a what-if that drops assumptions: it no longer
    /// holds when every line it holds on rests on one.
    fn step_without_dropped(
        &self,
        mut r: InferenceReading,
        lost: &BTreeSet<String>,
    ) -> InferenceReading {
        if r.verdict != Truth::Holds || lost.is_empty() {
            return r;
        }
        let keep: Vec<Condition> = r
            .lines
            .iter()
            .filter(|l| l.assumes.is_disjoint(lost))
            .cloned()
            .collect();
        if keep.is_empty() {
            r.verdict = Truth::Unknown;
            r.note =
                Some("every line it held on rests on an assumption this what-if drops".to_string());
            r.holds_under_any_of = Vec::new();
        } else if keep.len() < r.lines.len() {
            r.assumes = keep[0].assumes.iter().cloned().collect();
            r.holds_under_any_of = if keep.len() > 1 {
                keep.iter().map(LineConditions::of).collect()
            } else {
                Vec::new()
            };
            r.lines = keep;
        }
        r
    }

    /// The steps that take `id` up as an `assumed` premise, sorted.
    /// The premises a step uses MORE THAN ONCE, with how many times
    /// (PREMISE_OF `uses`).
    fn premise_uses(&self, inference_id: &str) -> Result<Vec<(String, i64)>, DynoError> {
        let mut out: Vec<(String, i64)> = self
            .incoming(inference_id, Some(edge::PREMISE_OF))?
            .into_iter()
            .filter_map(|e| {
                let n = e.properties.get("uses").and_then(Value::as_i64)?;
                (n > 1).then_some((e.from_id, n))
            })
            .collect();
        out.sort();
        out.dedup_by(|a, b| a.0 == b.0);
        Ok(out)
    }

    /// Whether `id` is a SUPPOSITION: a statement of kind `hypothesis`, or
    /// one some step takes up as an `assumed` premise.
    fn is_supposition(&self, id: &str) -> Result<bool, DynoError> {
        if self
            .node_props(id)?
            .is_some_and(|p| statement_kind(&p) == Some("hypothesis"))
        {
            return Ok(true);
        }
        Ok(!self.steps_assuming(id)?.is_empty())
    }

    /// The steps whose rule `id` states (`rule_node`), sorted.
    pub fn steps_ruled_by(&self, id: &str) -> Result<Vec<String>, DynoError> {
        let mut out: Vec<String> = self
            .scan_nodes(node::INFERENCE)?
            .into_iter()
            .filter(|n| prop(&n.properties, "rule_node").map(str::trim) == Some(id))
            .map(|n| n.node_id)
            .collect();
        out.sort();
        Ok(out)
    }

    fn steps_assuming(&self, id: &str) -> Result<Vec<String>, DynoError> {
        let mut steps: Vec<String> = self
            .outgoing(id, Some(edge::PREMISE_OF))?
            .into_iter()
            .filter(|e| e.properties.get("role").and_then(Value::as_str) == Some("assumed"))
            .map(|e| e.to_id)
            .collect();
        steps.sort();
        steps.dedup();
        Ok(steps)
    }

    // ─── The finding ───────────────────────────────────────────────────────

    /// Every conclusion whose own status CLAIMS it holds (`met`, `passing`,
    /// `accepted`, `verified`) while the steps concluding it do not support
    /// that: their verdict, ANY across them, is `fails` or `unknown`. Reported,
    /// never repaired (dec:report-dont-judge) — the status is left as claimed.
    pub fn conclusions_that_do_not_follow(&self) -> Result<Vec<UnfollowedConclusion>, DynoError> {
        // One evaluation for everything this reads (inference/memo.rs).
        let _shared = self.reasoning_scope();
        let mut conclusions: BTreeSet<String> = BTreeSet::new();
        for i in self.scan_nodes(node::INFERENCE)? {
            if let Some(c) = self.conclusion_of(&i.node_id)? {
                conclusions.insert(c);
            }
        }
        let lost = BTreeSet::new();
        let mut ev = Eval::new(self, &lost);
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
        // One evaluation for everything this reads (inference/memo.rs).
        let _shared = self.reasoning_scope();
        let mut conclusions: BTreeSet<String> = BTreeSet::new();
        for i in self.scan_nodes(node::INFERENCE)? {
            if let Some(c) = self.conclusion_of(&i.node_id)? {
                conclusions.insert(c);
            }
        }
        let lost = BTreeSet::new();
        let mut ev = Eval::new(self, &lost);
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
    /// it is a premise of one — an ORDINARY premise or an ASSUMED one.
    ///
    /// AN ASSUMED PREMISE TAKES PART (fact:root-cause-proof-statements-fall-
    /// through-to-software-predicates-and-assumed-premises-stopped-counting-
    /// 2026-10-22). Until 4879171c5 (2026-10-17) this read every PREMISE_OF;
    /// that commit narrowed [`Self::steps_resting_on`] to what a CHANGE
    /// reaches, for the what-if, and so narrowed this too: a hypothesis only
    /// ever assumed read as outside the reasoning, `unlinked_intent` asked
    /// about it, and a proof design stopped reading as one. Membership reads
    /// the edges themselves; `steps_resting_on` stays as it is for the walk.
    pub fn takes_part_in_reasoning(&self, id: &str) -> Result<bool, DynoError> {
        Ok(!self.inferences_concluding(id)?.is_empty()
            || !self.outgoing(id, Some(edge::PREMISE_OF))?.is_empty())
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
            .any(|o| o.id == objection && counts_as_holding(o.standing))
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
        // One evaluation for everything this reads (inference/memo.rs).
        let _shared = self.reasoning_scope();
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
            let mut ev = Eval::new(self, &none);
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
                let mut ev = Eval::new(self, &lost);
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
                    // READ AS THE STEP TAKING IT UP READS IT: its owner's
                    // intent there, or its own truth
                    // (dec:a-premise-says-whether-it-rests-on-intent-or-delivery).
                    if !axioms.contains_key(&premise_id) {
                        let (truth, why, reads_as) =
                            self.premise_truth_in_step(step_id, &premise_id)?;
                        let node_type = self.type_holding(&premise_id)?;
                        axioms.insert(
                            premise_id.clone(),
                            Foundation {
                                id: premise_id,
                                node_type,
                                truth,
                                why,
                                reads_as: reads_as.and_then(ReadsAs::served),
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
        // One evaluation for everything this reads (inference/memo.rs).
        let _shared = self.reasoning_scope();
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
            holds_under_any_of: self.lines_under(id)?.0,
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
        // One evaluation for everything this reads (inference/memo.rs).
        let _shared = self.reasoning_scope();
        let top_steps = self.inferences_concluding(id)?;
        if top_steps.is_empty() {
            return Ok(None);
        }
        // A FALSUM IS NO CLAIM TO PROVE (the four colour trial, 2026-10-23:
        // ⊥ read proof_status `unproven` beside `statement_reading` `false`).
        // What a step concluding it shows is that the assumptions under it
        // cannot all hold, and `statement_reading` says that — `false`, or
        // `inconsistent` when nothing is assumed. A proof status for it would
        // read the contradiction as an unproven result.
        if self.node_props(id)?.is_some_and(|p| is_falsum(&p)) {
            return Ok(None);
        }
        let reading = self.proof_reading_counted(id, &top_steps)?;
        // WHAT A CLAIM THAT DOES NOT HOLD IS, SAID BEFORE "UNPROVEN": replaced
        // or withdrawn by its source (superseded), accepted for a period that
        // ended (withdrawn), or claimed by a named party nobody has verified
        // (claimed) — PR A2. A refuted claim with no such history stays
        // `unproven`, and a contested one `contested`.
        let Some(mut reading) = reading else {
            return Ok(None);
        };
        if let Some(why) = self.superseded_reason(id)? {
            reading.status = ProofStatus::Superseded;
            reading.why = why;
            return Ok(Some(reading));
        }
        if matches!(
            reading.status,
            ProofStatus::Unproven | ProofStatus::Contested
        ) && let Some(w) = self.withdrawn_reading(id)?
        {
            reading.status = ProofStatus::Withdrawn;
            reading.why = w.why;
            return Ok(Some(reading));
        }
        if reading.status == ProofStatus::Unproven && self.premise_truth(id)?.0 != Truth::Fails {
            let parties = self.claimants_of(id, &BTreeSet::new())?;
            if !parties.is_empty() {
                reading.status = ProofStatus::Claimed;
                reading.why = Self::claimed_said(&parties);
            }
        }
        Ok(Some(reading))
    }

    /// The proof reading by its steps alone: proven, shown, conditional,
    /// supported, contested or unproven, with the step counts.
    fn proof_reading_counted(
        &self,
        id: &str,
        top_steps: &[String],
    ) -> Result<Option<ProofReading>, DynoError> {
        // TWO QUESTIONS, ANSWERED APART. Whether a line is wholly confirmed is
        // judged per line, so each line walks from its own empty visited set
        // and sees every one of its steps, shared ones included. How many
        // steps are under the claim is the size of the UNION of those lines:
        // a step two lines share is one step.
        let lost = BTreeSet::new();
        let mut ev = Eval::new(self, &lost);
        let mut any_holds = false;
        let mut any_proven_chain = false;
        let mut under: BTreeSet<String> = BTreeSet::new();
        let mut confirmed_under: BTreeSet<String> = BTreeSet::new();
        for s in top_steps {
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
                // EVERY LINE IS NAMED (req:a-claim-with-several-lines-reads-
                // each-lines-assumptions): Driver, held by five principles,
                // read "only under act consequentialism" until 2026-10-20.
                let (lines, _) = self.lines_under(id)?;
                let why = if lines.len() > 1 {
                    format!(
                        "it holds on {} lines, each only under an assumption no step on the way to \
                         it discharges — {} — so it is a result inside an argument on every line, \
                         not a proof of it",
                        lines.len(),
                        lines_said(&lines)
                    )
                } else {
                    format!(
                        "it holds only under the assumption(s) {}, which no step on the way to it \
                         discharges: a result inside an argument, not a proof of it",
                        assumes.join(", ")
                    )
                };
                return Ok(Some(ProofReading {
                    status: ProofStatus::Conditional,
                    why,
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
                        "it holds only if the unverified citation(s) or undocumented report(s) \
                         {} say what they are recalled to say: no pinned source documents them",
                        unverified.join(", ")
                    ),
                    total_steps: total,
                    confirmed_steps: confirmed,
                    unconfirmed_steps: unconfirmed,
                    counts: PROOF_COUNTS_MEAN,
                }));
            }
        }
        // A derivation holds only AS FAR AS A BRIDGE MODELS WHAT IT IS TAKEN TO
        // MODEL (dec:idea-a-bridge-premise-says-a-formal-statement-models-a-
        // claim, option b): conditional on it, never shown outright.
        if any_holds && conclusion_truth == Truth::Holds {
            let bridges = self.bridges_under(id)?;
            if !bridges.is_empty() {
                return Ok(Some(ProofReading {
                    status: ProofStatus::Conditional,
                    why: format!(
                        "it holds only as far as the bridge(s) {} model what they are taken to \
                         model: a formal result carried into another field's claim through them",
                        bridges.join(", ")
                    ),
                    total_steps: total,
                    confirmed_steps: confirmed,
                    unconfirmed_steps: unconfirmed,
                    counts: PROOF_COUNTS_MEAN,
                }));
            }
        }
        // A derivation holds only THROUGH A STATISTICAL TEST: supported at
        // its α, evidence and never proof, however its steps are confirmed.
        if any_holds && conclusion_truth == Truth::Holds {
            let tests = self.tests_under(id)?;
            if !tests.is_empty() {
                let set: BTreeSet<String> = tests.into_iter().collect();
                return Ok(Some(ProofReading {
                    status: ProofStatus::Supported,
                    why: format!(
                        "it holds only through the evidence {}, so it is supported, \
                         never shown or proven",
                        self.tests_said(&set)?
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
        // NO ROUTE HOLDS, BUT A DISPUTE IS WHAT HOLDS ONE UP: contested, never
        // unproven (req:a-conclusion-whose-every-route-is-contested-reads-
        // contested). The cogito's "I exist" read `unproven` while all three
        // of its readings were only contested.
        if !any_holds && conclusion_truth == Truth::Unknown {
            let r = self.premise_reading(id)?;
            if !r.contested_by.is_empty() {
                return Ok(Some(ProofReading {
                    status: ProofStatus::Contested,
                    why: r.why,
                    total_steps: total,
                    confirmed_steps: confirmed,
                    unconfirmed_steps: unconfirmed,
                    counts: PROOF_COUNTS_MEAN,
                }));
            }
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
        // One evaluation for everything this reads (inference/memo.rs).
        let _shared = self.reasoning_scope();
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
                     `any` (one is enough), `k_of_n` (at least k of them, with `k`) and \
                     `immediate` (not an inference: the conclusion is grasped directly). A \
                     named rule such as modus ponens is `all` with `rule_name`",
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
        // A PREMISE NAMED MORE THAN ONCE IS USED MORE THAN ONCE (item 4 of
        // req:natural-deduction-gaps-from-the-logic-rung-are-closed: A, A ⊢
        // A∧A): one edge, carrying how many times. Until 2026-10-20 the
        // second mention was dropped without a word.
        let mut uses: BTreeMap<String, i64> = BTreeMap::new();
        for p in &spec.premises {
            *uses.entry(p.clone()).or_default() += 1;
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
        // An IMMEDIATE grasp may stand on no premise: it is not an inference.
        if given.is_empty() && stored.is_empty() && spec.rule != "immediate" {
            return Err(invalid(
                "premises",
                "a step needs at least one premise: what supports the conclusion? (A conclusion \
                 grasped directly, not inferred, is a step with rule `immediate`, which may name \
                 none.)"
                    .to_string(),
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
        // WHAT A PREMISE IS READ AS (dec:a-premise-says-whether-it-rests-on-
        // intent-or-delivery): each marked id must be an ordinary premise of
        // this step, of a type that carries the two propositions, and marked
        // one way only.
        let stored_assumed: BTreeSet<String> = if existing.is_some() {
            self.premise_links(&spec.id)?
                .into_iter()
                .filter(|(_, a)| *a)
                .map(|(p, _)| p)
                .collect()
        } else {
            BTreeSet::new()
        };
        let mut marked: BTreeMap<String, &'static str> = BTreeMap::new();
        for (field, list, mark) in [
            ("as_intent", &spec.as_intent, "intent"),
            ("as_delivery", &spec.as_delivery, "delivery"),
        ] {
            for p in list {
                if !all_premises.contains(p) {
                    return Err(invalid(
                        field,
                        format!(
                            "'{p}' is not a premise of this step: name it in `premises` too, or                              link it first, then say what it is read as"
                        ),
                    ));
                }
                if assumed_given.contains(p) || stored_assumed.contains(p) {
                    return Err(invalid(
                        field,
                        format!(
                            "'{p}' is assumed by this step: a hypothesis holds inside the                              argument whatever its own truth, so it is read as neither intent                              nor delivery"
                        ),
                    ));
                }
                if !self.intent_applies_to(p)? {
                    let ty = self.type_holding(p)?.unwrap_or_else(|| "node".to_string());
                    return Err(invalid(
                        field,
                        format!(
                            "'{p}' is a {ty} that reads as itself: only a requirement that is a                              need (no `statement_kind`, not taken as given), a capability or an                              interface carries both its owner's intent and its delivery"
                        ),
                    ));
                }
                if let Some(other) = marked.insert(p.clone(), mark)
                    && other != mark
                {
                    return Err(invalid(
                        field,
                        format!(
                            "'{p}' is named in both `as_intent` and `as_delivery`: a premise is                              read one way in a step"
                        ),
                    ));
                }
            }
        }
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
        for (field, value, legal) in [
            (
                "establishes",
                &spec.establishes,
                &["entailment", "derivability"][..],
            ),
            (
                "conclusion_form",
                &spec.conclusion_form,
                &["conjunction", "biconditional"][..],
            ),
        ] {
            if let Some(v) = value
                && !legal.contains(&v.as_str())
            {
                return Err(invalid(
                    field,
                    format!(
                        "'{v}' is not a value of `{field}`: the legal values are {}",
                        legal.join(", ")
                    ),
                ));
            }
        }
        if let Some(rn) = spec.rule_node.as_deref().map(str::trim)
            && !rn.is_empty()
        {
            if rn == spec.id || rn == spec.conclusion {
                return Err(invalid(
                    "rule_node",
                    format!("'{rn}' cannot state this step's own rule: name the node that does"),
                ));
            }
            if given.iter().any(|p| p == rn) {
                return Err(invalid(
                    "rule_node",
                    format!(
                        "'{rn}' is named as a premise too: a rule is the step's rule, never one of \
                         its premises — leave it out of `premises`"
                    ),
                ));
            }
            self.one_type_for(rn, "rule_node")?;
        }
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
        match spec.rule_node.as_deref().map(str::trim) {
            Some("") => {
                props.remove("rule_node");
            }
            Some(rn) => {
                props.insert("rule_node".into(), Value::String(rn.to_string()));
            }
            None => {}
        }
        for (key, value) in [
            ("establishes", &spec.establishes),
            ("conclusion_form", &spec.conclusion_form),
        ] {
            if let Some(v) = value {
                props.insert(key.into(), Value::String(v.clone()));
            }
        }
        // WHAT THIS WRITE CLEARS: a list given empty (`independent_of: []`),
        // `cases_exhaustive: ""`, a rule that drops `k`. `upsert_node` MERGES,
        // so a key removed from `props` alone survived in the store: the
        // Navier–Stokes map's `independent_of: []` left the stale statement
        // standing, though the field's description says it overwrites (PR A2).
        let cleared: Vec<String> = existing
            .as_ref()
            .map(|n| {
                let mut keys: Vec<String> = n
                    .properties
                    .keys()
                    .filter(|k| !props.contains_key(*k))
                    .cloned()
                    .collect();
                keys.sort();
                keys
            })
            .unwrap_or_default();
        let mut builder = Props::new();
        for (k, v) in props {
            builder = builder.set(&k, v);
        }
        let mut stored_node = self.upsert_node(node::INFERENCE, &spec.id, builder)?;
        if !cleared.is_empty() {
            stored_node = self.remove_properties(node::INFERENCE, &spec.id, &cleared)?;
        }
        let mut drawn = Vec::new();
        for (p, ty) in &typed {
            let assumed = assumed_given.contains(p);
            let stored_as = stored_links.iter().find(|(s, _)| s == p).map(|(_, a)| *a);
            // Already drawn in the role asked for: nothing to write. A
            // premise newly named as assumed is re-drawn with its role.
            if stored_as == Some(assumed) || (stored_as.is_some() && !assumed) {
                continue;
            }
            let mut props = if assumed {
                Props::new().set("role", "assumed")
            } else {
                Props::new()
            };
            if let Some(n) = uses.get(p).copied().filter(|n| *n > 1) {
                props = props.set("uses", Value::Int(n));
            }
            if let Some(mark) = marked.get(p) {
                props = props.set("reads_as", *mark);
            }
            self.create_edge(edge::PREMISE_OF, ty, p, node::INFERENCE, &spec.id, props)?;
            drawn.push(format!("{p} PREMISE_OF {}", spec.id));
        }
        // A PREMISE ALREADY LINKED, NEWLY MARKED: its link is re-drawn with
        // what it is read as, keeping everything else the link carried.
        for (p, mark) in &marked {
            let Some(link) = self
                .incoming(&spec.id, Some(edge::PREMISE_OF))?
                .into_iter()
                .find(|e| &e.from_id == p)
            else {
                continue;
            };
            if link.properties.get("reads_as").and_then(Value::as_str) == Some(*mark) {
                continue;
            }
            let mut props = Props::new();
            for (k, v) in link.properties.iter() {
                props = props.set(k, v.clone());
            }
            props = props.set("reads_as", *mark);
            let ty = self.one_type_for(p, "premise")?;
            self.create_edge(edge::PREMISE_OF, &ty, p, node::INFERENCE, &spec.id, props)?;
            drawn.push(format!("{p} PREMISE_OF {} (reads_as: {mark})", spec.id));
        }
        // The exhaustiveness record is a premise of the split: losing it
        // reaches the step in a change walk.
        if let Some((by, ty)) = &exhaustive_by
            && !stored_links.iter().any(|(s, _)| s == by)
            && !typed.iter().any(|(p, _)| p == by)
        {
            // A SUPPOSITION is drawn as ASSUMED: a disjunction assumed in the
            // same subproof holds inside the argument, carried until a step
            // discharges it, never read by its standing status (item 1).
            let props = if self.is_supposition(by)? {
                Props::new().set("role", "assumed")
            } else {
                Props::new()
            };
            self.create_edge(edge::PREMISE_OF, ty, by, node::INFERENCE, &spec.id, props)?;
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
        // A revise that adds premises leaves a generated name saying the old
        // count; say the new one (item 2).
        let stored_node = if spec.name.is_none() {
            self.refresh_generated_step_name(&spec.id)?;
            self.get_node(node::INFERENCE, &spec.id)?
                .unwrap_or(stored_node)
        } else {
            stored_node
        };
        Ok(InferenceWritten {
            node: stored_node,
            edges_drawn: drawn,
            revised: existing.is_some(),
        })
    }

    /// Rewrite a step's name when it is still the one reflow2 GENERATED
    /// ("all of 2 premise(s) support X") and no longer says its premises
    /// (items 2 and 9 of req:small-read-back-faults-from-the-cogito-trial-are-
    /// fixed: the cogito trial read "all of 1 premise(s)" after a second was
    /// added, the trolley trial after one was deleted). A name somebody wrote
    /// is never touched.
    pub(crate) fn refresh_generated_step_name(&mut self, step: &str) -> Result<(), DynoError> {
        let Some(n) = self.get_node(node::INFERENCE, step)? else {
            return Ok(());
        };
        let Some(name) = prop(&n.properties, "name").map(str::to_string) else {
            return Ok(());
        };
        let Some(conclusion) = self.conclusion_of(step)? else {
            return Ok(());
        };
        let rule = prop(&n.properties, "rule").unwrap_or("all").to_string();
        let k = n.properties.get("k").and_then(Value::as_i64);
        let head = match k {
            Some(k) => format!("{k} of "),
            None => format!("{rule} of "),
        };
        let tail = format!(" premise(s) support {conclusion}");
        let generated = name
            .strip_prefix(&head)
            .and_then(|r| r.strip_suffix(&tail))
            .is_some_and(|count| count.parse::<usize>().is_ok());
        if !generated {
            return Ok(());
        }
        let now = format!("{head}{}{tail}", self.premises_of(step)?.len());
        if now != name {
            self.upsert_node(node::INFERENCE, step, Props::new().set("name", now))?;
        }
        Ok(())
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
    /// step — as an `assumed` one when it is a supposition (a disjunction
    /// assumed in the same subproof). Overwrites; an empty string removes it.
    pub cases_exhaustive: Option<String>,
    /// The node that states the step's rule — never a premise. Overwrites;
    /// an empty string removes it.
    pub rule_node: Option<String>,
    /// `entailment` (⊨) or `derivability` (⊢): the relation this line
    /// establishes. Overwrites.
    pub establishes: Option<String>,
    /// `conjunction` or `biconditional`: the conclusion is made of the
    /// premises as parts. Overwrites.
    pub conclusion_form: Option<String>,
    /// Premises this step reads as their OWNER'S INTENT (PREMISE_OF
    /// `reads_as: intent`): a requirement, capability or interface read by
    /// its acceptance (dec:a-premise-says-whether-it-rests-on-intent-or-
    /// delivery). Each must be an ordinary premise of the step, given here
    /// or already linked. Unmarked, a premise of a step concluding a
    /// Decision reads as intent anyway; marking says so on the record.
    pub as_intent: Vec<String>,
    /// Premises this step reads by their DELIVERY (`reads_as: delivery`):
    /// today's reading, said on a step that concludes a Decision resting on
    /// delivery — a release, a go/no-go.
    pub as_delivery: Vec<String>,
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
