//! A ROUTE THAT WAS TRIED AND FAILS, kept as a record of its own.
//!
//! `req:a-failed-route-is-a-record-of-its-own-read-back-with-the-reasoning`
//! (Anthony, 2026-10-30, option (a) of
//! `dec:idea-the-preserved-search-is-a-deliverable-of-working-a-proof-hole`,
//! promoting idea (D) of 2026-10-22, `dec:idea-a-failed-route-is-a-record-of-its-own`).
//! Working a hole in a proof leaves a map of the approaches that did not go
//! through. Kept as prose, a later reader cannot find them, count them or see
//! what each one rules out, and tries them again.
//!
//! # The encoding, decided once
//!
//! A failed route is a STATEMENT of the line of reasoning: a Requirement with
//! `statement_kind: failed_route`, written by `add_requirement`. It uses the
//! statement vocabulary a proof already uses, plus six fields of its own:
//!
//! - `statement` — WHAT WAS TRIED, the approach in words;
//! - `route_to` — THE CLAIM IT WAS A ROUTE TO (a node reference): the hole it
//!   tried to close. A step named here is read under the claim it concludes;
//! - `fails_at` — THE STEP AT WHICH IT FAILS, in words, and `fails_at_step`
//!   the record of that step or statement when the design holds it;
//! - `fails_because` — WHY IT FAILS THERE;
//! - `route_rests_on` — WHAT IT RESTS ON, by id;
//! - `would_hold_if` — THE NATURAL CONDITION UNDER WHICH IT WOULD GO THROUGH
//!   (`req:a-failed-route-records-the-natural-condition-under-which-it-would-hold`),
//!   optional and written only when the condition is natural: a dead end for
//!   one claim kept as a possible result for another. It moves no reading; it
//!   is neither a hypothesis of the claim nor a result shown.
//!
//! # What it never does
//!
//! A failed route records the space ruled out. It SUPPORTS NOTHING AND REFUTES
//! NOTHING:
//!
//! - its own truth is never `holds` ([`DesignGraph::failed_route_truth`]), and
//!   its statement reads `failed_route`, never `open`, `stated` or `unproven`;
//! - it is never a premise, an assumption or a conclusion of a step, and never
//!   the source of an objection: the store refuses each link where it is
//!   written ([`DesignGraph::refuse_a_failed_route_link`]). An objection
//!   AIMED AT the record — "the route does not fail there after all" — is
//!   allowed, and read on the record;
//! - its references are PROPERTIES, NEVER EDGES. The heat rung of 2026-10-22
//!   drew DEPENDS_ON from a failed case to its failing steps, and a what-if's
//!   blast radius grew to 40 nodes. Nothing walks a property: no truth, no
//!   what-if, no structural detector crosses a failed route;
//! - it is never an unsupported claim, a gap or a debt.
//!
//! What reads it: `reasoning_report`, under the claim it was a route to
//! (`failed_routes` on that claim) and in the report's `failed_routes` tally;
//! `get_node` on the record (`failed_route`); and `search_design`, which finds
//! it by what was tried, where it fails and why, and names its `route_to`.

use std::collections::BTreeSet;

use crate::foundation::core::{DynoError, Value};
use crate::graph::DesignGraph;
use crate::inference::{Truth, list_prop, statement_kind};
use crate::nodes::{edge, node};

/// The `statement_kind` a failed route carries.
pub const FAILED_ROUTE: &str = "failed_route";

/// The six fields only a failed route carries, in the order they are said.
pub const ROUTE_FIELDS: [&str; 6] = [
    "route_to",
    "fails_at",
    "fails_at_step",
    "fails_because",
    "route_rests_on",
    "would_hold_if",
];

/// What the read-back's tally means, carried with it.
pub const FAILED_ROUTES_NOTE: &str = "FAILED ROUTES: routes tried toward a claim that fail, each \
     a record of its own (statement_kind `failed_route`): what was tried, the step at which it \
     fails, why it fails there, and what it rests on. Each is read in full under the claim it was \
     a route to (that claim's `failed_routes`); one whose claim this read did not walk is read in \
     full here, under `read_here`. A failed route records the space ruled out: it supports \
     nothing and refutes nothing, so no claim reads proven, refuted, contested or open because of \
     it, and it is never a gap or a debt. `with_step_and_reason` counts the routes that name both \
     the step at which they fail and why.";

/// Whether a record's properties say it is a FAILED ROUTE.
pub fn is_failed_route(props: &std::collections::HashMap<String, Value>) -> bool {
    statement_kind(props) == Some(FAILED_ROUTE)
}

/// A record a failed route names, by id, with what the design holds for it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RouteRecord {
    pub id: String,
    /// Its type, when the design holds it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_type: Option<String>,
    /// Its name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// True when no node of the design holds the id (it was renamed or
    /// deleted after the route was written).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub not_in_the_design: bool,
}

/// ONE FAILED ROUTE, read: what was tried, the claim it was a route to, the
/// step at which it fails and why, and what it rests on.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FailedRouteReading {
    /// The record's id.
    pub id: String,
    /// Its name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// WHAT WAS TRIED: the record's `statement`, whole.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tried: Option<String>,
    /// THE CLAIM IT WAS A ROUTE TO (`route_to`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_to: Option<RouteRecord>,
    /// The claim it is READ UNDER, when that differs from `route_to`: a route
    /// to a step is read under the claim the step concludes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_under: Option<String>,
    /// THE STEP AT WHICH IT FAILS, in words (`fails_at`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fails_at: Option<String>,
    /// THAT STEP OR STATEMENT, when the design holds it (`fails_at_step`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fails_at_step: Option<RouteRecord>,
    /// WHY IT FAILS THERE (`fails_because`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fails_because: Option<String>,
    /// THE NATURAL CONDITION UNDER WHICH IT WOULD GO THROUGH
    /// (`would_hold_if`), when one was said. Read with the route and never
    /// apart from it: it is not a hypothesis of the claim, not a result
    /// shown, and it moves no reading. Never counted in `missing`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub would_hold_if: Option<String>,
    /// WHAT THE ROUTE RESTS ON (`route_rests_on`), each with its name. Never
    /// premises: nothing rests on a failed route, and these are not what any
    /// claim rests on.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rests_on: Vec<RouteRecord>,
    /// WHICH OF ITS PARTS IT DOES NOT RECORD: `route_to`, `fails_at` (no step,
    /// in words or by id), `fails_because`, `route_rests_on`. Said, never
    /// raised: an incomplete route is a thinner map, not a debt.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub missing: Vec<&'static str>,
    /// Whether it names both the step at which it fails and why — what a
    /// search is scored on (req:working-a-proof-hole-is-scored-on-its-
    /// preserved-search-as-well-as-its-result).
    pub names_step_and_reason: bool,
    /// How the record reads: `failed_route`, the statement reading's word.
    pub reads: &'static str,
    /// Why, in a sentence.
    pub why: String,
}

/// The routes read under one claim, by id.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RoutesToward {
    /// The claim.
    pub claim: String,
    /// Its name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The failed routes read under it, by id.
    pub routes: Vec<String>,
}

/// THE FAILED ROUTES A READ-BACK COVERS, tallied: every route toward a claim
/// the walk read, and on a whole-design read every route.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FailedRoutesRead {
    /// How many.
    pub total: usize,
    /// How many name both the step at which they fail and why.
    pub with_step_and_reason: usize,
    /// The ids of those that do not name both.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub without_step_or_reason: Vec<String>,
    /// Each claim the walk read that failed routes were tried toward, with
    /// their ids: each route is read in full under that claim.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub toward: Vec<RoutesToward>,
    /// The routes whose claim this read did not walk (none named, or not
    /// reached), each in full.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub read_here: Vec<FailedRouteReading>,
    /// What this means, in a paragraph.
    pub note: &'static str,
}

impl DesignGraph {
    /// Whether `id` is a failed route: a Requirement whose `statement_kind`
    /// is `failed_route`.
    pub fn is_a_failed_route(&self, id: &str) -> Result<bool, DynoError> {
        Ok(self
            .get_node(node::REQUIREMENT, id)?
            .is_some_and(|n| is_failed_route(&n.properties)))
    }

    /// Every failed route the design holds, by id.
    pub fn failed_route_ids(&self) -> Result<Vec<String>, DynoError> {
        let mut out: Vec<String> = self
            .scan_nodes(node::REQUIREMENT)?
            .into_iter()
            .filter(|n| is_failed_route(&n.properties))
            .map(|n| n.node_id)
            .collect();
        out.sort();
        Ok(out)
    }

    /// The claim a failed route is READ UNDER: its `route_to`, or, for a
    /// step, the claim that step concludes. `None` when it names none, or
    /// names a node the design no longer holds.
    pub fn failed_route_claim(&self, id: &str) -> Result<Option<String>, DynoError> {
        let Some(n) = self.get_node(node::REQUIREMENT, id)? else {
            return Ok(None);
        };
        let Some(to) = n
            .properties
            .get("route_to")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            return Ok(None);
        };
        if self.node_types_holding(to)?.is_empty() {
            return Ok(None);
        }
        if self.get_node(node::INFERENCE, to)?.is_some() {
            return self.conclusion_of(to);
        }
        Ok(Some(to.to_string()))
    }

    /// What a failed route's own truth is, before any objection to the record
    /// is counted: NEVER `holds`. It records a route that fails, and supports
    /// nothing; read as a premise it would leave its step unknown, which is
    /// why the store refuses that link.
    pub(crate) fn failed_route_truth(&self, id: &str) -> Result<(Truth, String), DynoError> {
        let to = self.failed_route_claim(id)?;
        let toward = match &to {
            Some(c) => format!(" toward {c}"),
            None => String::new(),
        };
        Ok((
            Truth::Unknown,
            format!(
                "a failed route{toward}: a record of a route tried that fails, so it supports \
                 nothing and refutes nothing; what it rules out is read under the claim it was a \
                 route to"
            ),
        ))
    }

    fn route_record(&self, id: &str) -> Result<RouteRecord, DynoError> {
        let held = self.node_types_holding(id)?;
        let node_type = (held.len() == 1).then(|| held[0].clone());
        let name = match &node_type {
            Some(t) => self.get_node(t, id)?.and_then(|n| {
                n.properties
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            }),
            None => None,
        };
        Ok(RouteRecord {
            id: id.to_string(),
            not_in_the_design: held.is_empty(),
            node_type,
            name,
        })
    }

    /// ONE FAILED ROUTE, read whole; `None` when `id` is not one.
    pub fn failed_route_reading(&self, id: &str) -> Result<Option<FailedRouteReading>, DynoError> {
        let Some(n) = self.get_node(node::REQUIREMENT, id)? else {
            return Ok(None);
        };
        if !is_failed_route(&n.properties) {
            return Ok(None);
        }
        let props = &n.properties;
        let text = |k: &str| {
            props
                .get(k)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        let route_to = match text("route_to") {
            Some(to) => Some(self.route_record(&to)?),
            None => None,
        };
        let under = self.failed_route_claim(id)?;
        let read_under = under
            .clone()
            .filter(|u| route_to.as_ref().is_none_or(|r| r.id != *u));
        let fails_at_step = match text("fails_at_step") {
            Some(s) => Some(self.route_record(&s)?),
            None => None,
        };
        let mut rests_on = Vec::new();
        for r in list_prop(props, "route_rests_on") {
            rests_on.push(self.route_record(&r)?);
        }
        let fails_at = text("fails_at");
        let fails_because = text("fails_because");
        let would_hold_if = text("would_hold_if");
        let names_step = fails_at.is_some() || fails_at_step.is_some();
        let names_step_and_reason = names_step && fails_because.is_some();
        let mut missing = Vec::new();
        if route_to.is_none() {
            missing.push("route_to");
        }
        if !names_step {
            missing.push("fails_at");
        }
        if fails_because.is_none() {
            missing.push("fails_because");
        }
        if rests_on.is_empty() {
            missing.push("route_rests_on");
        }
        let statement = self.statement_reading(id)?;
        let (reads, why) = match statement {
            Some(s) => (s.reads, s.why),
            None => ("failed_route", String::new()),
        };
        Ok(Some(FailedRouteReading {
            id: id.to_string(),
            name: text("name"),
            tried: text("statement"),
            route_to,
            read_under,
            fails_at,
            fails_at_step,
            fails_because,
            would_hold_if,
            rests_on,
            missing,
            names_step_and_reason,
            reads,
            why,
        }))
    }

    /// How a failed route's STATEMENT reads, given its truth with any
    /// objection to the record counted: `failed_route`, with what it ruled
    /// out said in words. An objection aimed at the record is said, and moves
    /// nothing else.
    pub(crate) fn failed_route_statement(
        &self,
        id: &str,
        truth: Truth,
        why: &str,
        contested: bool,
    ) -> Result<String, DynoError> {
        let props = self
            .get_node(node::REQUIREMENT, id)?
            .map(|n| n.properties)
            .unwrap_or_default();
        let text = |k: &str| {
            props
                .get(k)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        let mut said = match self.failed_route_claim(id)? {
            Some(c) => format!("a failed route toward {c}"),
            None => "a failed route that names no claim it was a route to (`route_to`)".to_string(),
        };
        match (text("fails_at"), text("fails_at_step")) {
            (Some(at), _) => said.push_str(&format!(", failing at {at}")),
            (None, Some(step)) => said.push_str(&format!(", failing at {step}")),
            (None, None) => said.push_str(", with no step named at which it fails"),
        }
        match text("fails_because") {
            Some(b) => said.push_str(&format!(", because {b}")),
            None => said.push_str(", and no reason named"),
        }
        if let Some(c) = text("would_hold_if") {
            said.push_str(&format!(
                "; it would go through {c}, a condition said with the route and never a \
                 hypothesis of its claim nor a result shown"
            ));
        }
        said.push_str(
            ". It records the space ruled out: it supports nothing and refutes nothing, so \
             nothing reads proven, refuted, contested or open because of it",
        );
        if truth == Truth::Fails {
            said.push_str(&format!(
                ". THE RECORD ITSELF DOES NOT HOLD (an objection to it holds, or an accepted \
                 decision withdrew it), so the route may not fail as recorded: {why}"
            ));
        } else if contested {
            said.push_str(&format!(
                ". An objection to this record stands, so whether the route fails as recorded is \
                 disputed: {why}"
            ));
        }
        Ok(said)
    }

    /// REFUSED WHERE IT IS WRITTEN, on every door: a link that would let a
    /// failed route support, conclude or object to anything — a PREMISE_OF
    /// from one, a CONCLUDES to one, an opposing or conceding CONTRADICTS
    /// from one. Re-writing an edge already stored is never refused (an
    /// import that upserts). Answers nothing for any other edge.
    pub(crate) fn refuse_a_failed_route_link(
        &self,
        edge_type: &str,
        from_id: &str,
        to_id: &str,
        props: &std::collections::HashMap<String, Value>,
    ) -> Result<(), DynoError> {
        let (route, as_what) = match edge_type {
            edge::PREMISE_OF => (from_id, "a premise or an assumption of a step"),
            edge::CONCLUDES => (to_id, "the conclusion of a step"),
            edge::CONTRADICTS => {
                let alignment = props
                    .get("alignment")
                    .and_then(Value::as_str)
                    .unwrap_or("opposing");
                if alignment == "supporting" {
                    return Ok(());
                }
                (from_id, "an objection")
            }
            _ => return Ok(()),
        };
        if !self.is_a_failed_route(route)? {
            return Ok(());
        }
        if self
            .outgoing(from_id, Some(edge_type))?
            .iter()
            .any(|e| e.to_id == to_id)
        {
            return Ok(());
        }
        Err(DynoError::EdgeValidation {
            edge_type: edge_type.to_string(),
            property: "endpoints".to_string(),
            message: failed_route_refusal(route, as_what),
        })
    }

    /// Why a record about to BECOME a failed route cannot: it already takes
    /// part in a step (a premise of one, or concluded by one) or objects to
    /// something. `None` when nothing stands in the way.
    pub fn failed_route_conflict(&self, id: &str) -> Result<Option<String>, DynoError> {
        let mut parts: BTreeSet<String> = BTreeSet::new();
        for e in self.outgoing(id, Some(edge::PREMISE_OF))? {
            parts.insert(format!("a premise of {}", e.to_id));
        }
        for e in self.incoming(id, Some(edge::CONCLUDES))? {
            parts.insert(format!("concluded by {}", e.from_id));
        }
        for e in self.outgoing(id, Some(edge::CONTRADICTS))? {
            let alignment = e
                .properties
                .get("alignment")
                .and_then(Value::as_str)
                .unwrap_or("opposing");
            if alignment != "supporting" {
                parts.insert(format!("an objection to {}", e.to_id));
            }
        }
        if parts.is_empty() {
            return Ok(None);
        }
        Ok(Some(format!(
            "'{id}' cannot become a failed route: it is {}. A failed route supports nothing and \
             refutes nothing, so it takes part in no step and objects to nothing. Record the \
             failed route as a statement of its own (a new id) with `route_to` naming the claim \
             it was a route to, and leave '{id}' as it is",
            parts.into_iter().collect::<Vec<_>>().join(", ")
        )))
    }
}

impl DesignGraph {
    /// THE FAILED ROUTES A READ-BACK COVERS: each route toward a claim the
    /// walk read is read whole under that claim (its `failed_routes`); on a
    /// whole-design read, and for a route that is itself the claim asked
    /// about, one whose claim was not walked is read whole in `read_here`.
    /// `None` when the read covers none, so a design that records none reads
    /// exactly as before.
    pub(crate) fn failed_routes_over(
        &self,
        claims: &mut [crate::reasoning::ClaimReading],
        order: &[String],
        whole: bool,
    ) -> Result<Option<FailedRoutesRead>, DynoError> {
        let ids = self.failed_route_ids()?;
        if ids.is_empty() {
            return Ok(None);
        }
        let index: std::collections::BTreeMap<String, usize> = claims
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id.clone(), i))
            .collect();
        let walked: BTreeSet<&str> = order.iter().map(String::as_str).collect();
        let mut toward: std::collections::BTreeMap<String, Vec<String>> = Default::default();
        let mut read_here = Vec::new();
        let mut without_step_or_reason = Vec::new();
        let mut total = 0usize;
        let mut with_step_and_reason = 0usize;
        for id in ids {
            let placed = self
                .failed_route_claim(&id)?
                .filter(|c| index.contains_key(c));
            if !whole && placed.is_none() && !walked.contains(id.as_str()) {
                continue;
            }
            let Some(reading) = self.failed_route_reading(&id)? else {
                continue;
            };
            total += 1;
            if reading.names_step_and_reason {
                with_step_and_reason += 1;
            } else {
                without_step_or_reason.push(id.clone());
            }
            match placed {
                Some(c) => {
                    toward.entry(c.clone()).or_default().push(id);
                    claims[index[&c]].failed_routes.push(reading);
                }
                None => read_here.push(reading),
            }
        }
        if total == 0 {
            return Ok(None);
        }
        Ok(Some(FailedRoutesRead {
            total,
            with_step_and_reason,
            without_step_or_reason,
            toward: toward
                .into_iter()
                .map(|(claim, routes)| RoutesToward {
                    name: claims[index[&claim]].name.clone(),
                    claim,
                    routes,
                })
                .collect(),
            read_here,
            note: FAILED_ROUTES_NOTE,
        }))
    }
}

/// The refusal a failed route gets in a step or an objection, worded once.
pub(crate) fn failed_route_refusal(route: &str, as_what: &str) -> String {
    format!(
        "'{route}' is a failed route (statement_kind `failed_route`): a record of a route that \
         was tried and fails. It supports nothing and refutes nothing, so it is never {as_what}. \
         What the route rested on goes in its `route_rests_on`; the claim it tried to reach in \
         its `route_to`, where reasoning_report reads it. If what blocks the route also tells \
         against a claim, record that as an objection of its own"
    )
}
