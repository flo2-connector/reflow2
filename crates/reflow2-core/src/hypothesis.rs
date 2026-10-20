//! A STATISTICAL HYPOTHESIS TEST, READ FROM ITS KEPT RECORD — phase 5,
//! reflow2's half (`epoch:planned-p5-hypothesis-tests`).
//!
//! Settled in flo2-calc's
//! `dec:idea-how-a-statistical-hypothesis-test-is-designed-computed-and-read`
//! (Anthony, 2026-10-19): flo2-calc COMPUTES a named test and keeps it as a
//! record (options a, e); reflow2 READS it. Three requirements:
//!
//! - **(b) the verdict reads as "supported at α", never as shown**
//!   (`req:a-hypothesis-tests-verdict-reads-as-supported-at-alpha-never-as-shown`).
//!   A check carrying a test (`Verification.hypothesis_test`, the record's
//!   block) is a computed check, so it counts only through its kept record
//!   ([`crate::kept_record`]). Then a test that rejects H0 reads SUPPORTED AT
//!   α — its α, p-value, test and n named — and is carried, like an
//!   assumption, by everything resting on it ([`crate::inference`]'s
//!   `evidential`), so a claim on it reads `supported`: never `holds`,
//!   `shown` or `proven`, however its steps are confirmed. A test that does
//!   not reject H0 reads NOT REJECTED AT α and supports nothing; it never
//!   reads "H0 true". The test's assumptions are Constraints the check
//!   DEPENDS_ON (written when the test is recorded), so an objection can
//!   attack each: a refuted or contested assumption leaves the test
//!   supporting nothing. A test used as an OBJECTION contests a claim and
//!   never refutes it: evidence against, at α, is not a counterexample.
//! - **(c) a change to the criteria after the data is reported**
//!   (`req:a-change-to-a-tests-criteria-after-its-data-arrived-is-reported`).
//!   The record carries no time and the core takes no clock. The one ordering
//!   reflow2 can vouch for is its own write order, so the write that FIRST
//!   attaches a test's data LOCKS its criteria as they stand then
//!   (`Verification.criteria_at_data`): the record's own test, alternative,
//!   α, H0 and H1; the α Constraint the record names; what the check VERIFIES
//!   (the hypothesis); and the Decisions it is GOVERNED_BY (the chosen test).
//!   Any of them that no longer says what it said is reported by name with
//!   both values, and with the ChangeEvents that changed it and their dates
//!   where they carry one. Nothing is refused: a change can be legitimate,
//!   but it is never silent.
//! - **(d) α is spent like a budget across a family**
//!   (`req:alpha-is-spent-like-a-budget-across-a-family-of-tests`). Mostly the
//!   served recipe over the existing rollup ([`crate::budget`]). The code adds
//!   only what the rollup cannot: a test whose record names an α Constraint
//!   that is a budget SPENDS what it was tested at (a CONSTRAINS with that
//!   `contribution`, drawn when the test is recorded, so a test added later
//!   is in the sum), and a test TESTED at more than it spends reads unknown,
//!   because the family's error rate is then not controlled.
//!
//! # What it does not know
//!
//! That the block is what the record holds: reflow2 reads the binding (the
//! kept Artifact, its checksum) and the block it was given, never the file's
//! bytes, exactly as for any computed check. Whether the assumptions hold is
//! the person's judgement, and an objection's. A criterion changed outside
//! the design, or before the lock was taken, is invisible to it.

use std::collections::{BTreeMap, HashMap};

use serde_json::Value as Json;

use crate::foundation::core::{DynoError, Value};
use crate::graph::DesignGraph;
use crate::nodes::{Props, edge, node};

/// The Verification property holding the record's block.
pub const HYPOTHESIS_TEST: &str = "hypothesis_test";
/// The Verification property holding the criteria lock.
pub const CRITERIA_AT_DATA: &str = "criteria_at_data";

/// What every reading of a test says it is.
pub const EVIDENCE_NOT_PROOF: &str = "evidence, not proof: a statistical test supports a claim at \
     its significance level and never shows or proves it";

/// What a test reading means, carried beside it.
pub const WHAT_A_TEST_READING_MEANS: &str = "A hypothesis test is read from the block of its \
     kept flo2-calc record, never re-computed. `supported` means the test rejected H0 at α: \
     evidence for H1 at that level, never proof. `not_rejected` means it did not: the data do \
     not reject H0, which never shows H0 true. Its assumptions are records an objection can \
     attack. `criteria_changed` lists what differs from the lock taken when its data were \
     first attached; reflow2 cannot see a change made outside the design.";

/// A test's block, read: the fields reflow2 uses. The block itself is kept
/// whole on the Verification.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HypothesisTest {
    /// The named test (`t_test_two_sample` ...).
    pub test: String,
    /// Its title, when the record gives one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternative: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variance: Option<String>,
    /// α as the record writes it.
    pub alpha: String,
    /// The design node the record says α came from (`alpha.source.design_node`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpha_node: Option<String>,
    /// The p-value as the record writes it.
    pub p_value: String,
    /// Each sample's size.
    pub n: Vec<String>,
    /// The record's decision, in its words.
    pub decision: String,
    pub reject_h0: bool,
    /// The premises the result stands on, in the record's words.
    pub assumptions: Vec<String>,
}

fn field<'a>(block: &'a Json, path: &str) -> Option<&'a Json> {
    let mut at = block;
    for part in path.split('.') {
        at = at.get(part)?;
    }
    Some(at)
}

fn text_at(block: &Json, path: &str) -> Option<String> {
    field(block, path)
        .and_then(Json::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn needed(block: &Json, arg: &str, path: &str, what: &str) -> Result<String, String> {
    text_at(block, path).ok_or_else(|| {
        format!(
            "`{arg}.{path}` is missing or not a string: it is {what}, as flo2-calc's record \
             writes it"
        )
    })
}

impl HypothesisTest {
    /// Read a record's `hypothesis_test` block. Refuses, naming the field
    /// path, a block that lacks what a reading needs.
    pub fn from_block(block: &Json) -> Result<Self, String> {
        Self::from_block_named(block, HYPOTHESIS_TEST)
    }

    /// [`Self::from_block`], its refusals naming the block as `arg` — the
    /// argument a caller passed it in.
    pub fn from_block_named(block: &Json, arg: &str) -> Result<Self, String> {
        if !block.is_object() {
            return Err(format!(
                "`{arg}` must be the object flo2-calc's record holds under a tested value's \
                 `hypothesis_test`"
            ));
        }
        let test = needed(
            block,
            arg,
            "test",
            "the named test (`t_test_two_sample` ...)",
        )?;
        let alpha = needed(
            block,
            arg,
            "alpha.value",
            "the significance level fixed in advance",
        )?;
        let p_value = needed(block, arg, "p_value.value", "the test's p-value")?;
        let decision = needed(
            block,
            arg,
            "decision",
            "the decision at α, in the record's words",
        )?;
        let reject_h0 = field(block, "reject_h0")
            .and_then(Json::as_bool)
            .ok_or_else(|| {
                format!(
                    "`{arg}.reject_h0` is missing or not a boolean: it is whether the test \
                     rejected H0 at α"
                )
            })?;
        let n: Vec<String> = match field(block, "n") {
            Some(Json::Array(items)) if !items.is_empty() => items
                .iter()
                .enumerate()
                .map(|(i, v)| match v {
                    Json::String(s) if !s.trim().is_empty() => Ok(s.trim().to_string()),
                    Json::Number(x) => Ok(x.to_string()),
                    _ => Err(format!(
                        "`{arg}.n[{i}]` is not a sample size: each is a count, as the \
                         record writes it"
                    )),
                })
                .collect::<Result<_, _>>()?,
            _ => {
                return Err(format!(
                    "`{arg}.n` is missing or empty: it is each sample's size"
                ));
            }
        };
        let assumptions: Vec<String> = match field(block, "assumptions") {
            Some(Json::Array(items)) => items
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    v.as_str()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .ok_or_else(|| {
                            format!(
                                "`{arg}.assumptions[{i}]` is not a sentence: each is a \
                                 premise the result stands on"
                            )
                        })
                })
                .collect::<Result<_, _>>()?,
            None => Vec::new(),
            Some(_) => {
                return Err(format!("`{arg}.assumptions` must be a list of sentences"));
            }
        };
        let parsed = HypothesisTest {
            test,
            title: text_at(block, "title"),
            h0: text_at(block, "h0"),
            h1: text_at(block, "h1"),
            alternative: text_at(block, "alternative"),
            variance: text_at(block, "variance"),
            alpha,
            alpha_node: text_at(block, "alpha.source.design_node"),
            p_value,
            n,
            decision,
            reject_h0,
            assumptions,
        };
        if parsed.alpha_f64().is_none_or(|a| !(a > 0.0 && a < 1.0)) {
            return Err(format!(
                "`{arg}.alpha.value` is `{}`: a significance level is a number \
                 strictly between 0 and 1",
                parsed.alpha
            ));
        }
        if parsed.p_f64().is_none_or(|p| !(0.0..=1.0).contains(&p)) {
            return Err(format!(
                "`{arg}.p_value.value` is `{}`: a p-value is a number from 0 to 1",
                parsed.p_value
            ));
        }
        Ok(parsed)
    }

    /// The block stored on a Verification, when it carries one: `None` when
    /// it does not, `Some(Err)` when what it carries cannot be read.
    pub fn from_props(props: &HashMap<String, Value>) -> Option<Result<Self, String>> {
        let text = props.get(HYPOTHESIS_TEST).and_then(Value::as_str)?;
        Some(
            serde_json::from_str::<Json>(text)
                .map_err(|e| format!("its `hypothesis_test` is not JSON ({e})"))
                .and_then(|b| Self::from_block(&b)),
        )
    }

    fn number(s: &str) -> Option<f64> {
        s.split_whitespace().next()?.parse::<f64>().ok()
    }

    /// α as a number.
    pub fn alpha_f64(&self) -> Option<f64> {
        Self::number(&self.alpha)
    }

    /// The p-value as a number.
    pub fn p_f64(&self) -> Option<f64> {
        Self::number(&self.p_value)
    }

    /// The p-value as a reader is shown it: three significant digits (the
    /// record keeps every digit).
    pub fn p_shown(&self) -> String {
        match self.p_f64() {
            Some(0.0) => "0".to_string(),
            Some(p) if p < 1e-4 => format!("{p:.2e}"),
            Some(p) => {
                let decimals = (2 - p.log10().floor() as i32).max(0) as usize;
                format!("{p:.decimals$}")
            }
            None => self.p_value.clone(),
        }
    }

    /// The test as a reader is shown it: its name and, when given, its title.
    pub fn named(&self) -> String {
        match &self.title {
            Some(t) => format!("{}: {t}", self.test),
            None => self.test.clone(),
        }
    }

    /// `n = 10, 10`.
    pub fn n_shown(&self) -> String {
        format!("n = {}", self.n.join(", "))
    }

    /// The reading of a test that rejected H0.
    pub fn supported(&self) -> String {
        format!(
            "supported at α = {} (p = {}, {}, {}): {EVIDENCE_NOT_PROOF}",
            self.alpha,
            self.p_shown(),
            self.named(),
            self.n_shown()
        )
    }

    /// The reading of a test that did not reject H0.
    pub fn not_rejected(&self) -> String {
        format!(
            "not rejected at α = {} (p = {}, {}, {}): the data do not reject H0{}, which never \
             shows H0 true, so it supports nothing here",
            self.alpha,
            self.p_shown(),
            self.named(),
            self.n_shown(),
            self.h0
                .as_deref()
                .map(|h| format!(" ({h})"))
                .unwrap_or_default()
        )
    }

    /// The record's own criteria, as the lock keeps them.
    fn own_criteria(&self) -> BTreeMap<&'static str, Option<String>> {
        BTreeMap::from([
            ("test", Some(self.test.clone())),
            ("alternative", self.alternative.clone()),
            ("alpha", Some(self.alpha.clone())),
            ("h0", self.h0.clone()),
            ("h1", self.h1.clone()),
            ("variance", self.variance.clone()),
        ])
    }
}

/// A JSON value as one canonical string: object keys sorted at every depth,
/// so a block stored by any build reads byte-identically.
pub fn canonical(v: &Json) -> String {
    fn walk(v: &Json, out: &mut String) {
        match v {
            Json::Object(m) => {
                let mut keys: Vec<&String> = m.keys().collect();
                keys.sort();
                out.push('{');
                for (i, k) in keys.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push_str(&Json::String((*k).clone()).to_string());
                    out.push(':');
                    walk(&m[*k], out);
                }
                out.push('}');
            }
            Json::Array(items) => {
                out.push('[');
                for (i, x) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    walk(x, out);
                }
                out.push(']');
            }
            other => out.push_str(&other.to_string()),
        }
    }
    let mut out = String::new();
    walk(v, &mut out);
    out
}

/// The fields of a criterion node the lock keeps, per type.
fn criterion_fields(node_type: &str) -> &'static [&'static str] {
    match node_type {
        node::CONSTRAINT => &["limit", "statement"],
        node::DECISION => &["decision", "chose"],
        node::REQUIREMENT => &["statement"],
        _ => &["statement", "description"],
    }
}

/// A stored value as the lock and the reading write it.
fn shown(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) => Some(s.clone()),
        Value::Float(f) => Some(Json::from(*f).to_string()),
        Value::Int(i) => Some(i.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        other => serde_json::to_string(other).ok(),
    }
}

/// One assumption written when a test is recorded.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AssumptionWritten {
    pub id: String,
    pub text: String,
}

/// What a test spends of the α Constraint its record names.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AlphaSpent {
    pub constraint_id: String,
    /// The contribution on the CONSTRAINS edge, when it carries one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contribution: Option<f64>,
    /// `drawn` (this call drew it, at the record's α) or `kept` (it was there
    /// already, and is never overwritten).
    pub edge: &'static str,
}

/// What recording a test wrote, said back to the caller.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HypothesisRecorded {
    pub test: String,
    /// The test's reading now, in a sentence.
    pub reads: String,
    /// Each assumption, as a Constraint the check DEPENDS_ON.
    pub assumptions: Vec<AssumptionWritten>,
    /// What it spends of the α Constraint its record names, when it names one
    /// in this design.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpha_spent: Option<AlphaSpent>,
    /// Why nothing was drawn to an α node, when nothing was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpha_note: Option<String>,
    /// `taken` (this was the first attachment of data) or `kept` (an earlier
    /// attachment's lock stands, and this one is compared with it).
    pub criteria_lock: &'static str,
    /// The criterion nodes the lock holds.
    pub criteria_locked: Vec<String>,
}

/// One criterion that no longer says what it said when the data were attached.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CriterionChange {
    /// The node (or, for the record's own fields, the check itself).
    pub criterion: String,
    /// Which field (`limit`, `statement`, `hypothesis_test.alpha` ...).
    pub field: String,
    /// What it said when the data were attached (`null`: it said nothing).
    pub at_data: Option<String>,
    /// What it says now (`null`: nothing, or the node is gone).
    pub now: Option<String>,
    /// The date the attaching write gave, when it gave one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attached_at: Option<String>,
    /// The ChangeEvents that changed the criterion, with their dates.
    pub changed_when: Vec<ChangeWhen>,
    /// The sentence a reader is given.
    pub says: String,
}

/// A ChangeEvent that changed a criterion.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChangeWhen {
    pub change_event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epoch: Option<String>,
    /// Whether its date is on or after the attachment's: `null` when either
    /// date is missing, which reflow2 cannot order.
    pub after_the_data: Option<bool>,
}

/// One assumption of a test, read.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AssumptionReading {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub truth: crate::inference::Truth,
    pub why: String,
}

/// A test, read: served on `get_node` for a Verification that carries one.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TestReading {
    /// `supported`, `not_rejected` or `unknown`.
    pub reads: &'static str,
    /// Why, in a sentence.
    pub why: String,
    pub test: HypothesisTest,
    pub assumptions: Vec<AssumptionReading>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpha_spent: Option<AlphaSpent>,
    /// Criteria that changed after the data were attached.
    pub criteria_changed: Vec<CriterionChange>,
    /// Criteria linked after the lock was taken: reflow2 holds no value from
    /// the time of the data to compare them with.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub criteria_unlocked: Vec<String>,
    pub what_this_means: &'static str,
}

/// The id of a test's `i`-th assumption (1-based): `con:<check>-assumption-<i>`.
pub fn assumption_id(verification_id: &str, i: usize) -> String {
    let local = verification_id
        .split_once(':')
        .map_or(verification_id, |(_, rest)| rest);
    format!("con:{local}-assumption-{i}")
}

impl DesignGraph {
    /// The test a Verification carries, when it carries one that reads.
    pub fn hypothesis_test_of(&self, id: &str) -> Result<Option<HypothesisTest>, DynoError> {
        Ok(self
            .get_node(node::VERIFICATION, id)?
            .and_then(|n| HypothesisTest::from_props(&n.properties))
            .and_then(Result::ok))
    }

    /// The nodes a test's criteria live in, as the design links them now:
    /// the α node its record names, what it VERIFIES, and the Decisions it is
    /// GOVERNED_BY. Sorted, each once, with its type.
    fn criterion_nodes(
        &self,
        id: &str,
        test: &HypothesisTest,
    ) -> Result<Vec<(String, String)>, DynoError> {
        let mut out: BTreeMap<String, String> = BTreeMap::new();
        if let Some(a) = &test.alpha_node
            && let Some(t) = self.type_holding(a)?
        {
            out.insert(a.clone(), t);
        }
        for e in self.outgoing(id, Some(edge::VERIFIES))? {
            if let Some(t) = self.type_holding(&e.to_id)? {
                out.insert(e.to_id, t);
            }
        }
        for e in self.outgoing(id, Some(edge::GOVERNED_BY))? {
            if let Some(t) = self.type_holding(&e.to_id)?
                && t == node::DECISION
            {
                out.insert(e.to_id, t);
            }
        }
        Ok(out.into_iter().collect())
    }

    /// RECORD A HYPOTHESIS TEST on the Verification `id` from its kept
    /// record's block: the block, `computed`, each assumption as a Constraint
    /// the check DEPENDS_ON, what it spends of the α Constraint its record
    /// names, and — on the first attachment of data — the criteria lock.
    /// `attached_at` is the date the caller gave the run, when it gave one.
    /// The Verification must exist.
    pub fn record_hypothesis_test(
        &mut self,
        id: &str,
        block: &Json,
        attached_at: Option<&str>,
    ) -> Result<HypothesisRecorded, DynoError> {
        let test = HypothesisTest::from_block(block).map_err(|message| DynoError::Validation {
            node_type: node::VERIFICATION.into(),
            property: HYPOTHESIS_TEST.into(),
            message,
        })?;
        let Some(existing) = self.get_node(node::VERIFICATION, id)? else {
            return Err(DynoError::NodeNotFound {
                node_type: node::VERIFICATION.into(),
                node_id: id.into(),
            });
        };
        let had_lock = existing
            .properties
            .get(CRITERIA_AT_DATA)
            .and_then(Value::as_str)
            .is_some();
        self.upsert_node(
            node::VERIFICATION,
            id,
            Props::new()
                .set(HYPOTHESIS_TEST, canonical(block))
                .set("computed", true),
        )?;

        // THE ASSUMPTIONS, as records an objection can attack.
        let mut assumptions = Vec::new();
        for (i, text) in test.assumptions.iter().enumerate() {
            let aid = assumption_id(id, i + 1);
            self.upsert_node(
                node::CONSTRAINT,
                &aid,
                Props::new()
                    .set("name", text.as_str())
                    .set(
                        "statement",
                        format!("An assumption of the test {}: {text}", test.named()),
                    )
                    .set("statement_kind", "proposition")
                    .set("taken_as_given", true),
            )?;
            self.create_edge(
                edge::DEPENDS_ON,
                node::VERIFICATION,
                id,
                node::CONSTRAINT,
                &aid,
                Props::new(),
            )?;
            assumptions.push(AssumptionWritten {
                id: aid,
                text: text.clone(),
            });
        }
        // A re-attached record with fewer assumptions: the extra ones are no
        // longer this test's, so the check stops resting on them. The
        // Constraints themselves stay, with whatever stands against them.
        let mut extra = test.assumptions.len() + 1;
        loop {
            let aid = assumption_id(id, extra);
            if self.get_node(node::CONSTRAINT, &aid)?.is_none() {
                break;
            }
            self.delete_edge(edge::DEPENDS_ON, id, &aid)?;
            extra += 1;
        }

        // WHAT IT SPENDS of the α Constraint its record names.
        let mut alpha_spent = None;
        let mut alpha_note = None;
        match test.alpha_node.as_deref() {
            None => {
                alpha_note = Some(
                    "the record names no design node for α (`alpha.source.design_node`), so \
                     nothing ties the test to the design's significance level: give α that \
                     source in flo2-calc"
                        .to_string(),
                )
            }
            Some(a) => match self.get_node(node::CONSTRAINT, a)? {
                None => {
                    alpha_note = Some(format!(
                        "the record names {a} for α, and no Constraint of this design holds that \
                         id, so nothing was drawn"
                    ))
                }
                Some(con) => {
                    let held = self
                        .outgoing(a, Some(edge::CONSTRAINS))?
                        .into_iter()
                        .find(|e| e.to_id == id);
                    if let Some(e) = held {
                        alpha_spent = Some(AlphaSpent {
                            constraint_id: a.to_string(),
                            contribution: e.properties.get("contribution").and_then(Value::as_f64),
                            edge: "kept",
                        });
                    } else {
                        let is_budget = con
                            .properties
                            .get("limit")
                            .and_then(Value::as_f64)
                            .is_some();
                        let mut props =
                            Props::new().set("basis", "evidence").set("source", id).set(
                                "note",
                                format!(
                                    "the α this test was run at, from its kept record ({})",
                                    test.named()
                                ),
                            );
                        let contribution = if is_budget { test.alpha_f64() } else { None };
                        if let Some(c) = contribution {
                            props = props.set("contribution", c);
                            if let Some(u) = con.properties.get("unit").and_then(Value::as_str) {
                                props = props.set("unit", u);
                            }
                        }
                        self.create_edge(
                            edge::CONSTRAINS,
                            node::CONSTRAINT,
                            a,
                            node::VERIFICATION,
                            id,
                            props,
                        )?;
                        alpha_spent = Some(AlphaSpent {
                            constraint_id: a.to_string(),
                            contribution,
                            edge: "drawn",
                        });
                    }
                }
            },
        }

        // THE LOCK, on the first attachment of data only.
        let criteria = self.criterion_nodes(id, &test)?;
        if !had_lock {
            let mut nodes = Vec::new();
            for (cid, ctype) in &criteria {
                let props = self
                    .get_node(ctype, cid)?
                    .map(|n| n.properties)
                    .unwrap_or_default();
                for f in criterion_fields(ctype) {
                    nodes.push(serde_json::json!({
                        "id": cid, "type": ctype, "field": f, "value": shown(props.get(*f)),
                    }));
                }
            }
            let lock = serde_json::json!({
                "attached_at": attached_at,
                "record": test.own_criteria(),
                "nodes": nodes,
            });
            self.upsert_node(
                node::VERIFICATION,
                id,
                Props::new().set(CRITERIA_AT_DATA, canonical(&lock)),
            )?;
        }
        let (_, reads) = self.premise_truth(id)?;
        Ok(HypothesisRecorded {
            test: test.test.clone(),
            reads,
            assumptions,
            alpha_spent,
            alpha_note,
            criteria_lock: if had_lock { "kept" } else { "taken" },
            criteria_locked: self
                .locked_nodes(id)?
                .into_iter()
                .map(|(cid, _)| cid)
                .collect(),
        })
    }

    /// The lock a test's data were attached under, parsed.
    fn criteria_lock(&self, id: &str) -> Result<Option<Json>, DynoError> {
        Ok(self
            .get_node(node::VERIFICATION, id)?
            .and_then(|n| {
                n.properties
                    .get(CRITERIA_AT_DATA)
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .and_then(|s| serde_json::from_str(&s).ok()))
    }

    /// The criterion nodes the lock holds, each once, with its type.
    fn locked_nodes(&self, id: &str) -> Result<Vec<(String, String)>, DynoError> {
        let mut out: BTreeMap<String, String> = BTreeMap::new();
        if let Some(lock) = self.criteria_lock(id)?
            && let Some(nodes) = lock.get("nodes").and_then(Json::as_array)
        {
            for n in nodes {
                if let (Some(i), Some(t)) = (
                    n.get("id").and_then(Json::as_str),
                    n.get("type").and_then(Json::as_str),
                ) {
                    out.insert(i.to_string(), t.to_string());
                }
            }
        }
        Ok(out.into_iter().collect())
    }

    /// The ChangeEvents that CHANGED `node_id`, each with its date and epoch,
    /// and whether its date falls on or after `attached_at`.
    fn changes_to(
        &self,
        node_id: &str,
        attached_at: Option<&str>,
    ) -> Result<Vec<ChangeWhen>, DynoError> {
        let mut out = Vec::new();
        for e in self.incoming(node_id, Some(edge::CHANGED))? {
            let Some(ce) = self.get_node(node::CHANGE_EVENT, &e.from_id)? else {
                continue;
            };
            let detected_at = ce
                .properties
                .get("detected_at")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let epoch = self
                .outgoing(&e.from_id, Some(edge::AT_EPOCH))?
                .into_iter()
                .map(|x| x.to_id)
                .next();
            let after_the_data = match (detected_at.as_deref(), attached_at) {
                (Some(d), Some(a)) => {
                    let day = |s: &str| s.chars().take(10).collect::<String>();
                    Some(day(d) >= day(a))
                }
                _ => None,
            };
            out.push(ChangeWhen {
                change_event: e.from_id,
                detected_at,
                epoch,
                after_the_data,
            });
        }
        out.sort_by(|a, b| a.change_event.cmp(&b.change_event));
        Ok(out)
    }

    /// Every criterion of the test on `id` that no longer says what it said
    /// when its data were first attached, and the criteria linked since the
    /// lock was taken. Empty when it carries no lock.
    pub fn criteria_changes(
        &self,
        id: &str,
    ) -> Result<(Vec<CriterionChange>, Vec<String>), DynoError> {
        let Some(lock) = self.criteria_lock(id)? else {
            return Ok((Vec::new(), Vec::new()));
        };
        let attached_at = lock
            .get("attached_at")
            .and_then(Json::as_str)
            .map(str::to_string);
        let when = match &attached_at {
            Some(a) => format!("when its data were attached ({a})"),
            None => "when its data were attached (the attaching write gave no date)".to_string(),
        };
        let as_text = |v: Option<&Json>| -> Option<String> {
            match v? {
                Json::Null => None,
                Json::String(s) => Some(s.clone()),
                other => Some(other.to_string()),
            }
        };
        let quoted = |v: &Option<String>| match v {
            Some(s) => format!("“{s}”"),
            None => "nothing".to_string(),
        };
        let mut changes = Vec::new();
        // The record's own criteria, against the block attached now.
        if let Some(current) = self.hypothesis_test_of(id)? {
            let now = current.own_criteria();
            if let Some(locked) = lock.get("record").and_then(Json::as_object) {
                for (k, v_now) in &now {
                    let v_then = as_text(locked.get(*k));
                    if &v_then != v_now {
                        changes.push(CriterionChange {
                            criterion: id.to_string(),
                            field: format!("hypothesis_test.{k}"),
                            says: format!(
                                "the test's {k} in its kept record was {} {when}, and the \
                                 record attached now says {}: the test was run again under a \
                                 changed criterion",
                                quoted(&v_then),
                                quoted(v_now)
                            ),
                            at_data: v_then,
                            now: v_now.clone(),
                            attached_at: attached_at.clone(),
                            changed_when: Vec::new(),
                        });
                    }
                }
            }
        }
        // Each criterion node's deciding fields, against the node now.
        if let Some(nodes) = lock.get("nodes").and_then(Json::as_array) {
            for n in nodes {
                let (Some(cid), Some(ctype), Some(f)) = (
                    n.get("id").and_then(Json::as_str),
                    n.get("type").and_then(Json::as_str),
                    n.get("field").and_then(Json::as_str),
                ) else {
                    continue;
                };
                let then = as_text(n.get("value"));
                let current = self.get_node(ctype, cid)?;
                let now = match &current {
                    Some(c) => shown(c.properties.get(f)),
                    None => None,
                };
                if then == now && current.is_some() {
                    continue;
                }
                let changed_when = self.changes_to(cid, attached_at.as_deref())?;
                let dated = if changed_when.is_empty() {
                    " No ChangeEvent records when it changed: the design shows only that it \
                     differs from the lock."
                        .to_string()
                } else {
                    format!(
                        " Changed by {}.",
                        changed_when
                            .iter()
                            .map(|c| match &c.detected_at {
                                Some(d) => format!("{} ({d})", c.change_event),
                                None => format!("{} (undated)", c.change_event),
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                let says = if current.is_none() {
                    format!(
                        "{cid}, a criterion of this test, said {} in `{f}` {when} and is gone \
                         now.{dated}",
                        quoted(&then)
                    )
                } else {
                    format!(
                        "{cid}'s `{f}` was {} {when} and is {} now: changed after the data.{dated}",
                        quoted(&then),
                        quoted(&now)
                    )
                };
                changes.push(CriterionChange {
                    criterion: cid.to_string(),
                    field: f.to_string(),
                    at_data: then,
                    now,
                    attached_at: attached_at.clone(),
                    changed_when,
                    says,
                });
            }
        }
        // Criteria linked since the lock: nothing from the time of the data.
        let locked: Vec<String> = self.locked_nodes(id)?.into_iter().map(|(c, _)| c).collect();
        let unlocked = match self.hypothesis_test_of(id)? {
            Some(t) => self
                .criterion_nodes(id, &t)?
                .into_iter()
                .map(|(c, _)| c)
                .filter(|c| !locked.contains(c))
                .collect(),
            None => Vec::new(),
        };
        Ok((changes, unlocked))
    }

    /// What a test is TESTED at against what it SPENDS: the first budget
    /// whose CONSTRAINS contribution to this check is below the α its record
    /// was run at, as a sentence. `None` when every spend covers it.
    pub(crate) fn alpha_overspent(
        &self,
        id: &str,
        test: &HypothesisTest,
    ) -> Result<Option<String>, DynoError> {
        let Some(alpha) = test.alpha_f64() else {
            return Ok(None);
        };
        for e in self.incoming(id, Some(edge::CONSTRAINS))? {
            let Some(spent) = e.properties.get("contribution").and_then(Value::as_f64) else {
                continue;
            };
            let Some(con) = self.get_node(node::CONSTRAINT, &e.from_id)? else {
                continue;
            };
            if con
                .properties
                .get("limit")
                .and_then(Value::as_f64)
                .is_none()
            {
                continue;
            }
            if alpha > spent * (1.0 + 1e-12) {
                return Ok(Some(format!(
                    "tested at α = {}, more than the {} it spends of {}: the family's error rate \
                     is not controlled at that budget, so this test supports nothing until it \
                     is run at its share or its share is raised",
                    test.alpha,
                    Json::from(spent),
                    e.from_id
                )));
            }
        }
        Ok(None)
    }

    /// The assumptions a test DEPENDS_ON, each with its truth.
    pub fn test_assumptions(&self, id: &str) -> Result<Vec<AssumptionReading>, DynoError> {
        let mut out = Vec::new();
        for e in self.outgoing(id, Some(edge::DEPENDS_ON))? {
            let text = self
                .type_holding(&e.to_id)?
                .and_then(|t| self.get_node(&t, &e.to_id).ok().flatten())
                .and_then(|n| {
                    n.properties
                        .get("name")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                });
            let (truth, why) = self.premise_truth(&e.to_id)?;
            out.push(AssumptionReading {
                id: e.to_id,
                text,
                truth,
                why,
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// A test, read: `None` when the Verification carries none.
    pub fn test_reading(&self, id: &str) -> Result<Option<TestReading>, DynoError> {
        let Some(n) = self.get_node(node::VERIFICATION, id)? else {
            return Ok(None);
        };
        let Some(parsed) = HypothesisTest::from_props(&n.properties) else {
            return Ok(None);
        };
        let test = match parsed {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        let (truth, why) = self.premise_truth(id)?;
        let reads = match truth {
            crate::inference::Truth::Holds => "supported",
            _ if !test.reject_h0 && why.starts_with("not rejected") => "not_rejected",
            _ => "unknown",
        };
        let alpha_spent = match test.alpha_node.as_deref() {
            Some(a) => self
                .outgoing(a, Some(edge::CONSTRAINS))?
                .into_iter()
                .find(|e| e.to_id == id)
                .map(|e| AlphaSpent {
                    constraint_id: a.to_string(),
                    contribution: e.properties.get("contribution").and_then(Value::as_f64),
                    edge: "kept",
                }),
            None => None,
        };
        let (criteria_changed, criteria_unlocked) = self.criteria_changes(id)?;
        Ok(Some(TestReading {
            reads,
            why,
            assumptions: self.test_assumptions(id)?,
            alpha_spent,
            criteria_changed,
            criteria_unlocked,
            test,
            what_this_means: WHAT_A_TEST_READING_MEANS,
        }))
    }

    /// Every Verification whose criteria changed after its data, with the
    /// changes: what `criteria_changed_after_data` raises. Sorted by id.
    pub fn tests_with_moved_criteria(
        &self,
    ) -> Result<Vec<(crate::foundation::store::StoredNode, Vec<CriterionChange>)>, DynoError> {
        let mut out = Vec::new();
        for v in self.scan_live_nodes(node::VERIFICATION)? {
            if !v.properties.contains_key(CRITERIA_AT_DATA) {
                continue;
            }
            let (changes, _) = self.criteria_changes(&v.node_id)?;
            if !changes.is_empty() {
                out.push((v, changes));
            }
        }
        out.sort_by(|a, b| a.0.node_id.cmp(&b.0.node_id));
        Ok(out)
    }
}

#[cfg(test)]
mod a_block_is_read_and_shown {
    use super::*;

    fn welch() -> Json {
        serde_json::json!({
            "test": "t_test_two_sample", "title": "two-sample t-test, Welch's",
            "h0": "μ1 = μ2", "h1": "μ1 < μ2", "alternative": "less",
            "alpha": {"node": "alpha", "value": "0.05",
                      "source": {"design_node": "con:significance-level"}},
            "n": ["10", "10"], "p_value": {"value": "0.0396970700936790691686242761159"},
            "decision": "reject H0 at α = 0.05", "reject_h0": true,
            "assumptions": ["independent", "normal"],
        })
    }

    #[test]
    fn the_reading_names_alpha_p_the_test_and_n() {
        let t = HypothesisTest::from_block(&welch()).unwrap();
        let s = t.supported();
        assert!(
            s.starts_with("supported at α = 0.05 (p = 0.0397, t_test_two_sample"),
            "{s}"
        );
        assert!(s.contains("n = 10, 10"), "{s}");
        assert_eq!(t.alpha_node.as_deref(), Some("con:significance-level"));
    }

    #[test]
    fn a_block_missing_what_a_reading_needs_is_refused_by_its_path() {
        let mut b = welch();
        b["alpha"]["value"] = Json::from("1.5");
        assert!(
            HypothesisTest::from_block(&b)
                .unwrap_err()
                .contains("alpha.value")
        );
        let mut b = welch();
        b.as_object_mut().unwrap().remove("reject_h0");
        assert!(
            HypothesisTest::from_block(&b)
                .unwrap_err()
                .contains("reject_h0")
        );
    }

    #[test]
    fn p_is_shown_to_three_significant_digits() {
        for (p, want) in [
            ("0.0396970700936790691686242761159", "0.0397"),
            ("0.0793941401873581383372485522319", "0.0794"),
            ("0.00141644509869213637016751343293", "0.00142"),
            ("0.5", "0.500"),
            ("0.0000123", "1.23e-5"),
        ] {
            let mut b = welch();
            b["p_value"]["value"] = Json::from(p);
            assert_eq!(HypothesisTest::from_block(&b).unwrap().p_shown(), want);
        }
    }

    #[test]
    fn a_canonical_block_sorts_its_keys_at_every_depth() {
        let a = serde_json::json!({"b": 1, "a": {"d": 2, "c": [ {"z": 1, "y": 2} ]}});
        assert_eq!(canonical(&a), r#"{"a":{"c":[{"y":2,"z":1}],"d":2},"b":1}"#);
    }
}
