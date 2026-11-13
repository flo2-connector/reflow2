//! Functional flows — the write and read side of `Flow` (BL-37).
//!
//! `Flow` had been fully specified in `functional.yaml` since the schema was
//! written — `flow_type: process/control_flow/decision_flow`, with
//! `PART_OF_FLOW (Capability → Flow)` carrying `step_order` — and nothing
//! could create one: no constructor in core, no MCP tool. The
//! eleventh instance of the recurring lesson, found by modelling reflow2's own
//! coherence loop in reflow2 (`tools/model_the_loop.py`): **the one type meant
//! for "an ordered process linking Capabilities end to end" was unreachable**,
//! for the exact model that is its use case.
//!
//! The read side embodies a decision (2026-07-19): **a process's cycles are
//! reported, never judged.** In a product a cycle is a defect —
//! `circular_dependency` walks `DEPENDS_ON` and contracts, and stays that way.
//! In a process the loops are the design: *test fails → fix → test* is a cycle
//! on purpose, and the feedback edges are the most important thing the model
//! says. So [`FlowReport`] computes the cycles among a flow's members over
//! `TRIGGERS` and states them as facts, the same way the confirmation ledger
//! reports claim histories without ruling on them (`dec:report-dont-judge`).
//! Nothing here feeds HEAL.
//!
//! # How a thing is made and put together
//!
//! `req:a-step-of-a-process-names-the-parts-it-puts-together`, settled by the
//! owner on 2026-11-12. A Flow could say a ring is printed, cast, polished,
//! plated and set, and nothing could say WHICH PARTS each step puts together
//! or WHICH JOINS it makes. Now a step names both — `PUTS_TOGETHER` to each
//! part (a Component), `CLOSES` to each join (an Interface one part PROVIDES
//! and another CONSUMES) — drawn with the step itself
//! ([`DesignGraph::draw_flow_step`]), all or nothing.
//!
//! A Flow of `flow_type: assembly` reads back as INSTRUCTIONS: each step's
//! parts and joins (with what each join must hold), who performs it (an Actor
//! INTERACTS_WITH it, `interaction: performs`), its checks (VERIFIES) and its
//! limits (CONSTRAINS) — and confesses what instructions would need and the
//! model never said: a step that names no part and no join, one nobody
//! performs, one nothing checks, and a join between two of the flow's own
//! parts that no step closes.
//!
//! ⚠️ ONLY AN ASSEMBLY IS READ SO, and the restriction is what keeps every
//! other flow's report byte for byte what it was. A process step already
//! carries checks, limits and actors, and `interaction` was free text, so a
//! report reading them for every flow would have changed every existing
//! flow's answer (`tests/an_existing_flow_reads_back_as_it_did.rs`). The two
//! new edges are listed on a step of any flow — they exist only where someone
//! drew them for this — and a flow of another type whose steps carry them says
//! it is not being read as instructions.

use std::collections::{BTreeMap, BTreeSet};

use crate::foundation::core::{DynoError, Value};
use crate::foundation::store::{StoredEdge, StoredNode};
use crate::graphalg::{GraphBuilder, find_cycle, strongly_connected_components};

use crate::graph::DesignGraph;
use crate::nodes::{Props, edge, node};

impl DesignGraph {
    /// P1 · Function — an ordered process linking Capabilities end to end.
    /// `name` is required; `flow_type` defaults to `process`.
    ///
    /// Where the flow begins and ends is NOT stated here. It is computed from
    /// the steps' `step_order` ([`FlowReport::entry_points`]): a stored entry
    /// and exit were a second copy of the order, and the one this design held
    /// named step 8 of 9 because adding a step never updated it
    /// (`dec:a-flows-order-is-its-step-order-and-entry-and-exit-are-computed`).
    pub fn add_flow(
        &mut self,
        id: &str,
        name: &str,
        description: Option<&str>,
        flow_type: Option<&str>,
    ) -> Result<StoredNode, DynoError> {
        self.upsert_node(
            node::FLOW,
            id,
            Props::new()
                .set("name", name)
                .set_opt("description", description)
                .set_opt("flow_type", flow_type),
        )
    }

    /// `Capability PART_OF_FLOW Flow` — a capability is a step of the process.
    /// `step_order` is its position; steps without one sort after those with,
    /// and the report says so rather than inventing an order.
    pub fn part_of_flow(
        &mut self,
        capability_id: &str,
        flow_id: &str,
        step_order: Option<i64>,
    ) -> Result<StoredEdge, DynoError> {
        self.draw_edge_keeping(
            edge::PART_OF_FLOW,
            node::CAPABILITY,
            capability_id,
            node::FLOW,
            flow_id,
            Props::new().set_opt("step_order", step_order),
        )
    }

    /// `Capability PUTS_TOGETHER Component` — this step puts the part together
    /// with the others it names: it places, joins or assembles it.
    ///
    /// Refuses, naming what the id IS, a step that is not a Capability and a
    /// part that is not a Component — a join handed in as a part is told it
    /// is [`closes`](Self::closes)'s. Re-drawing keeps the edge as it is.
    pub fn puts_together(&mut self, step_id: &str, part_id: &str) -> Result<StoredEdge, DynoError> {
        self.refuse_a_wrong_end(edge::PUTS_TOGETHER, "from", step_id, node::CAPABILITY)?;
        self.refuse_a_wrong_end(edge::PUTS_TOGETHER, "to", part_id, node::COMPONENT)?;
        self.draw_edge_keeping(
            edge::PUTS_TOGETHER,
            node::CAPABILITY,
            step_id,
            node::COMPONENT,
            part_id,
            Props::new(),
        )
    }

    /// `Capability CLOSES Interface` — this step makes the join: after it,
    /// what the Interface says must hold there holds, to be checked by the
    /// step's verifications.
    ///
    /// Refuses, naming what the id IS, a step that is not a Capability and a
    /// join that is not an Interface — a part handed in as a join is told it is
    /// [`puts_together`](Self::puts_together)'s.
    pub fn closes(&mut self, step_id: &str, join_id: &str) -> Result<StoredEdge, DynoError> {
        self.refuse_a_wrong_end(edge::CLOSES, "from", step_id, node::CAPABILITY)?;
        self.refuse_a_wrong_end(edge::CLOSES, "to", join_id, node::INTERFACE)?;
        self.draw_edge_keeping(
            edge::CLOSES,
            node::CAPABILITY,
            step_id,
            node::INTERFACE,
            join_id,
            Props::new(),
        )
    }

    /// A step of a flow, with the parts it puts together and the joins it
    /// closes, in ONE write: `PART_OF_FLOW` (keeping what an earlier draw of
    /// it carries, as [`part_of_flow`](Self::part_of_flow) does), then one
    /// `PUTS_TOGETHER` per part and one `CLOSES` per join.
    ///
    /// ALL OR NOTHING. Every id is checked before anything is written: the
    /// capability must be a Capability, the flow a Flow, each part a Component
    /// and each join an Interface. Any that is not is named, with what it is
    /// (or that it names nothing), in ONE refusal, and nothing is written. The
    /// writes run in one batch, so a failure part-way leaves nothing either.
    ///
    /// ADDITIVE, like every re-draw of a typed edge: a part or join an earlier
    /// call named and this one leaves out is kept. Taking one away is
    /// `delete_edge`. With both lists empty this is exactly
    /// [`part_of_flow`](Self::part_of_flow).
    pub fn draw_flow_step(
        &mut self,
        capability_id: &str,
        flow_id: &str,
        step_order: Option<i64>,
        puts_together: &[String],
        closes: &[String],
    ) -> Result<FlowStepDrawn, DynoError> {
        let parts = distinct(puts_together);
        let joins = distinct(closes);
        if parts.is_empty() && joins.is_empty() {
            return Ok(FlowStepDrawn {
                step: self.part_of_flow(capability_id, flow_id, step_order)?,
                puts_together: Vec::new(),
                closes: Vec::new(),
            });
        }

        let mut wrong: Vec<(&str, String)> = Vec::new();
        for (arg, id, want) in [
            ("capability_id", capability_id, node::CAPABILITY),
            ("flow_id", flow_id, node::FLOW),
        ] {
            if let Some(what) = self.what_is_wrong_with(id, want)? {
                wrong.push((arg, what));
            }
        }
        for id in &parts {
            if let Some(what) = self.what_is_wrong_with(id, node::COMPONENT)? {
                wrong.push(("puts_together", what));
            }
        }
        for id in &joins {
            if let Some(what) = self.what_is_wrong_with(id, node::INTERFACE)? {
                wrong.push(("closes", what));
            }
        }
        if !wrong.is_empty() {
            let mut args: Vec<&str> = Vec::new();
            for (arg, _) in &wrong {
                if !args.contains(arg) {
                    args.push(arg);
                }
            }
            return Err(DynoError::EdgeValidation {
                edge_type: edge::PART_OF_FLOW.to_string(),
                property: args.join(", "),
                message: format!(
                    "NOTHING WAS WRITTEN — {}. A step names the parts it puts together in \
                     `puts_together` (Component ids) and the joins it makes in `closes` \
                     (Interface ids); fix each one named and send the whole call again.",
                    wrong
                        .iter()
                        .map(|(arg, what)| format!("{arg}: {what}"))
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
            });
        }

        self.begin_batch();
        match self.draw_flow_step_unbatched(capability_id, flow_id, step_order, &parts, &joins) {
            Ok(drawn) => {
                self.commit_batch()?;
                Ok(drawn)
            }
            Err(e) => {
                self.discard_batch();
                Err(e)
            }
        }
    }

    fn draw_flow_step_unbatched(
        &mut self,
        capability_id: &str,
        flow_id: &str,
        step_order: Option<i64>,
        parts: &[String],
        joins: &[String],
    ) -> Result<FlowStepDrawn, DynoError> {
        let step = self.part_of_flow(capability_id, flow_id, step_order)?;
        let mut puts_together = Vec::with_capacity(parts.len());
        for part in parts {
            puts_together.push(self.puts_together(capability_id, part)?);
        }
        let mut closes = Vec::with_capacity(joins.len());
        for join in joins {
            closes.push(self.closes(capability_id, join)?);
        }
        Ok(FlowStepDrawn {
            step,
            puts_together,
            closes,
        })
    }

    /// What is wrong with `id` as a `want`: `None` when a `want` holds it;
    /// otherwise what it IS (`'x' is an Interface, not a Component`) or that
    /// it names nothing.
    fn what_is_wrong_with(&self, id: &str, want: &str) -> Result<Option<String>, DynoError> {
        if self.get_node(want, id)?.is_some() {
            return Ok(None);
        }
        let holders = self.node_types_holding(id)?;
        Ok(Some(if holders.is_empty() {
            format!("'{id}' names nothing in this design (it must be a {want})")
        } else {
            format!(
                "'{id}' is {} {}, not {} {want}",
                article(&holders[0]),
                holders.join(" and "),
                article(want)
            )
        }))
    }

    /// The single-edge refusal: an end of a step's edge that is not the type
    /// it must be, named, with the edge that takes what it is when there is one.
    fn refuse_a_wrong_end(
        &self,
        edge_type: &str,
        end: &str,
        id: &str,
        want: &str,
    ) -> Result<(), DynoError> {
        let Some(what) = self.what_is_wrong_with(id, want)? else {
            return Ok(());
        };
        let is = |t: &str| self.get_node(t, id).map(|n| n.is_some());
        let instead = match (edge_type, end) {
            (edge::PUTS_TOGETHER, "to") if is(node::INTERFACE)? => {
                " A join between two parts is CLOSES, from the step to its Interface."
            }
            (edge::CLOSES, "to") if is(node::COMPONENT)? => {
                " A part a step places, joins or assembles is PUTS_TOGETHER, from the step to \
                 its Component."
            }
            _ => "",
        };
        let reads = if edge_type == edge::PUTS_TOGETHER {
            "PUTS_TOGETHER runs from a step (a Capability) to a part it puts together (a \
             Component)."
        } else {
            "CLOSES runs from a step (a Capability) to a join it makes (an Interface)."
        };
        Err(DynoError::EdgeValidation {
            edge_type: edge_type.to_string(),
            property: end.to_string(),
            message: format!("{what}: {reads}{instead}"),
        })
    }
}

/// `a` or `an`, for a type name.
fn article(word: &str) -> &'static str {
    match word.chars().next().map(|c| c.to_ascii_lowercase()) {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

/// The ids in the order given, each once.
fn distinct(ids: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    ids.iter()
        .filter(|id| seen.insert(id.as_str()))
        .cloned()
        .collect()
}

/// What [`DesignGraph::draw_flow_step`] drew: the step's own edge, then one
/// edge per part and per join, in the order named.
#[derive(Debug, Clone)]
pub struct FlowStepDrawn {
    /// The `PART_OF_FLOW` edge.
    pub step: StoredEdge,
    /// One `PUTS_TOGETHER` per part.
    pub puts_together: Vec<StoredEdge>,
    /// One `CLOSES` per join.
    pub closes: Vec<StoredEdge>,
}

/// A node a step names, by id and name: a part it puts together, an Actor
/// who performs it, a Constraint or DesignRule that limits it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct StepNamed {
    pub id: String,
    /// The node's `name`; its id when it has none or is not there (a store
    /// restored with a dangling edge), which the report confesses.
    pub name: String,
}

/// A join a step closes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct StepJoin {
    pub id: String,
    pub name: String,
    /// What must hold at the join once the step has made it, in the
    /// Interface's own words: its `description`, or its `spec` when it has no
    /// description. `None` when the Interface states neither.
    pub what_must_hold: Option<String>,
}

/// A check of a step: a Verification that VERIFIES it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct StepCheck {
    pub id: String,
    pub name: String,
    /// `Verification.method` — how the check is made; `None` when nobody said.
    pub method: Option<String>,
    /// `Verification.status` — the last result the design records.
    pub status: Option<String>,
}

/// One step of a flow, in reported order.
///
/// The five lists after `step_order` are a step's INSTRUCTIONS
/// (req:a-step-of-a-process-names-the-parts-it-puts-together), each written
/// only when it has something in it, so a flow whose steps carry none reads
/// exactly as it did. `puts_together` and `closes` are read on a step of any
/// flow; `performed_by`, `checks` and `limits` only on an `assembly` flow's.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FlowStep {
    pub capability_id: String,
    pub name: String,
    /// `PART_OF_FLOW.step_order` — `None` means nobody stated a position.
    pub step_order: Option<i64>,
    /// The parts this step puts together (PUTS_TOGETHER), by id.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub puts_together: Vec<StepNamed>,
    /// The joins this step makes (CLOSES), with what each must hold.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub closes: Vec<StepJoin>,
    /// Who performs this step: each Actor INTERACTS_WITH it with
    /// `interaction: performs`. Assembly flows only.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub performed_by: Vec<StepNamed>,
    /// The Verifications that VERIFY this step. Assembly flows only.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<StepCheck>,
    /// The Constraints and DesignRules that CONSTRAIN this step. Assembly
    /// flows only.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub limits: Vec<StepNamed>,
}

/// A join between two of an assembly flow's own parts — an Interface one part
/// the flow puts together PROVIDES and another CONSUMES — and the steps of the
/// flow that close it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AssemblyJoin {
    pub id: String,
    pub name: String,
    /// The flow's parts that provide it, sorted.
    pub provided_by: Vec<String>,
    /// The flow's parts that consume it, sorted.
    pub consumed_by: Vec<String>,
    /// The flow's steps that close it, sorted. Empty: no step makes it.
    pub closed_by: Vec<String>,
}

/// A `TRIGGERS` edge between two members of the flow.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FlowTransition {
    pub from_id: String,
    pub to_id: String,
    /// `TRIGGERS.role` — what this transition *means* ("feeds", "forces
    /// resync", …). `None` means the model never said, which the report's
    /// confessions call out rather than leaving to be misread as "forward".
    pub role: Option<String>,
}

/// One loop in a process: every step caught in it, and one walk through it.
///
/// The two are reported separately because they are different claims, and
/// conflating them misleads. A strongly-connected cluster says *these steps
/// can all reach each other*; a representative path is one closed walk, and a
/// cluster of N steps often has a shorter cycle inside it.
///
/// The storyflow trial made the cost concrete (F7): the six-process model's
/// cluster held four steps, and the returned path omitted `p-prompt` — the
/// hand-off to the human, and the entire reason the process is a loop rather
/// than a line. The behaviour was correct and the report was still wrong,
/// which is its own lesson: a true statement presented as the whole truth is
/// a way of lying quietly.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FlowCycle {
    /// Every step in the cluster — all mutually reachable, sorted.
    pub members: Vec<String>,
    /// One closed walk through the cluster, rotated to its smallest id so the
    /// output is stable. **May omit members**; `members` is the full answer.
    pub path: Vec<String>,
}

/// What a flow says, read back as facts — steps, transitions, cycles.
///
/// Cycles are the point, not a problem: a process model's loops are its
/// design, so they are listed here and deliberately **not** raised by
/// `detect_defects` (whose `circular_dependency` stays scoped to `DEPENDS_ON`
/// and contracts, where a cycle really is a defect).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FlowReport {
    pub flow_id: String,
    pub flow_name: String,
    pub flow_type: Option<String>,
    /// Where the flow begins: the step(s) at the LOWEST `step_order`, computed
    /// and never stored. One for a linear flow; several when steps share the
    /// first position. Empty when no step states an order, and `confessions`
    /// then says so.
    pub entry_points: Vec<String>,
    /// Where the flow ends: the step(s) at the HIGHEST `step_order`, the steps
    /// nothing follows. Computed the same way as `entry_points`.
    pub exit_points: Vec<String>,
    /// Members ordered by `step_order` (unstated positions sort last), then id.
    pub steps: Vec<FlowStep>,
    /// `TRIGGERS` edges where **both** endpoints are members, with their role.
    /// Edges leaving the flow are out of its scope, not dropped: this report
    /// is about the process, and its boundary is the membership.
    pub transitions: Vec<FlowTransition>,
    /// The process's loops, one entry per strongly-connected cluster of
    /// members. Reported as fact; never a defect.
    pub cycles: Vec<FlowCycle>,
    /// What the projection could not honestly render — steps with no stated
    /// order, transitions with no stated role. Per the projection doctrine, a confession is a gap in the
    /// model, not a fill-in by the renderer.
    pub confessions: Vec<String>,
}

impl DesignGraph {
    /// Read one flow back: its steps in stated order, the transitions among
    /// them with their roles, and the cycles — reported, never judged.
    pub fn flow_report(&self, flow_id: &str) -> Result<FlowReport, DynoError> {
        let Some(flow) = self.get_node(node::FLOW, flow_id)? else {
            return Err(DynoError::NodeNotFound {
                node_type: node::FLOW.to_string(),
                node_id: flow_id.to_string(),
            });
        };
        let prop = |key: &str| {
            flow.properties
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_string)
        };

        let mut confessions = Vec::new();
        // An ASSEMBLY reads as instructions; every other flow as it always did.
        let flow_type = prop("flow_type");
        let assembly = flow_type.as_deref() == Some(ASSEMBLY);
        let mut unreadable: Vec<String> = Vec::new();

        // Members, via incoming PART_OF_FLOW.
        let mut steps = Vec::new();
        let mut member_names: BTreeMap<String, String> = BTreeMap::new();
        for e in self.incoming(flow_id, Some(edge::PART_OF_FLOW))? {
            let cap = self.get_node(node::CAPABILITY, &e.from_id)?;
            if cap.is_none() {
                // The storage layer accepts an edge to a node that does not
                // exist; rendering the id as if it were a step would be a
                // silent fill-in. Caught live: the smoke test attached a
                // member that was never created and the report said nothing.
                confessions.push(format!(
                    "member '{}' has a PART_OF_FLOW edge but no Capability node — the step \
                     is listed by id only, because there is nothing to read.",
                    e.from_id
                ));
            }
            let name = cap
                .as_ref()
                .and_then(|n| n.properties.get("name"))
                .and_then(Value::as_str)
                .unwrap_or(&e.from_id)
                .to_string();
            let step_order = e.properties.get("step_order").and_then(Value::as_i64);
            member_names.insert(e.from_id.clone(), name.clone());
            let (puts_together, closes) = self.step_parts_and_joins(&e.from_id, &mut unreadable)?;
            let (performed_by, checks, limits) = if assembly {
                self.step_performers_checks_and_limits(&e.from_id)?
            } else {
                (Vec::new(), Vec::new(), Vec::new())
            };
            steps.push(FlowStep {
                capability_id: e.from_id,
                name,
                step_order,
                puts_together,
                closes,
                performed_by,
                checks,
                limits,
            });
        }
        steps.sort_by(|a, b| {
            match (a.step_order, b.step_order) {
                (Some(x), Some(y)) => x.cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
            .then(a.capability_id.cmp(&b.capability_id))
        });
        let unordered = steps.iter().filter(|s| s.step_order.is_none()).count();
        if unordered > 0 && steps.len() > 1 {
            confessions.push(format!(
                "{unordered} of {} step(s) carry no step_order — they are listed after the \
                 ordered ones, in id order, because the graph never said where they go.",
                steps.len()
            ));
        }

        // Entry and exit are COMPUTED from the order, never stored: the steps
        // at the lowest and at the highest stated position.
        let ordered: Vec<(i64, &str)> = steps
            .iter()
            .filter_map(|s| s.step_order.map(|o| (o, s.capability_id.as_str())))
            .collect();
        let at = |pick: Option<i64>| -> Vec<String> {
            pick.map(|o| {
                ordered
                    .iter()
                    .filter(|(x, _)| *x == o)
                    .map(|(_, id)| (*id).to_string())
                    .collect()
            })
            .unwrap_or_default()
        };
        let entry_points = at(ordered.iter().map(|(o, _)| *o).min());
        let exit_points = at(ordered.iter().map(|(o, _)| *o).max());
        let members: BTreeSet<&str> = member_names.keys().map(String::as_str).collect();

        // Transitions: TRIGGERS with both endpoints inside the membership.
        let mut transitions = Vec::new();
        let mut unroled = 0usize;
        for id in member_names.keys() {
            for e in self.outgoing(id, Some(edge::TRIGGERS))? {
                if !members.contains(e.to_id.as_str()) {
                    continue;
                }
                let role = e
                    .properties
                    .get("role")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                if role.is_none() {
                    unroled += 1;
                }
                transitions.push(FlowTransition {
                    from_id: e.from_id,
                    to_id: e.to_id,
                    role,
                });
            }
        }
        transitions.sort_by(|a, b| a.from_id.cmp(&b.from_id).then(a.to_id.cmp(&b.to_id)));
        if unroled > 0 {
            confessions.push(format!(
                "{unroled} of {} transition(s) carry no role — forward and feedback edges \
                 are indistinguishable there, which for a process is the load-bearing fact.",
                transitions.len()
            ));
        }

        // Cycles among members — one representative per strongly-connected
        // cluster, same shape as structure.rs's circular_dependencies, but as
        // a fact of the process rather than a defect of the product.
        let cycles = cycles_among(&transitions);

        // What the instructions could not read: a part or join a step names
        // whose node is not there (a store restored with a dangling edge).
        confessions.extend(unreadable);
        if assembly {
            self.confess_what_instructions_lack(&steps, &mut confessions)?;
        } else {
            let assembling = steps
                .iter()
                .filter(|s| !s.puts_together.is_empty() || !s.closes.is_empty())
                .count();
            if assembling > 0 {
                confessions.push(format!(
                    "{assembling} of {} step(s) put together parts or close joins, but this \
                     flow's flow_type is {}, not `{ASSEMBLY}`, so it is not read as \
                     instructions: who performs each step, its checks and its limits are not \
                     listed, and no join between its parts is asked about. Recorded with \
                     flow_type `{ASSEMBLY}` (add_flow), it is.",
                    steps.len(),
                    flow_type
                        .as_deref()
                        .map_or_else(|| "unstated".to_string(), |t| format!("`{t}`")),
                ));
            }
        }

        Ok(FlowReport {
            flow_id: flow_id.to_string(),
            flow_name: prop("name").unwrap_or_else(|| flow_id.to_string()),
            flow_type,
            entry_points,
            exit_points,
            steps,
            transitions,
            cycles,
            confessions,
        })
    }

    /// An assembly read as instructions, and what they lack: each in plain
    /// words, in the order a reader following them would meet it.
    fn confess_what_instructions_lack(
        &self,
        steps: &[FlowStep],
        confessions: &mut Vec<String>,
    ) -> Result<(), DynoError> {
        let total = steps.len();
        let which = |pick: &dyn Fn(&FlowStep) -> bool| -> Vec<&str> {
            steps
                .iter()
                .filter(|s| pick(s))
                .map(|s| s.capability_id.as_str())
                .collect()
        };
        let idle = which(&|s| s.puts_together.is_empty() && s.closes.is_empty());
        if !idle.is_empty() {
            confessions.push(format!(
                "{} of {total} step(s) put together no part and close no join — {}. Read as \
                 instructions, such a step gives only its name: which parts it handles and \
                 which joins it makes were never recorded.",
                idle.len(),
                idle.join(", ")
            ));
        }
        let unperformed = which(&|s| s.performed_by.is_empty());
        if !unperformed.is_empty() {
            confessions.push(format!(
                "{} of {total} step(s) name no one who performs them — {}: no Actor \
                 INTERACTS_WITH them with `interaction: performs`, so the instructions cannot \
                 say who does each.",
                unperformed.len(),
                unperformed.join(", ")
            ));
        }
        let unchecked = which(&|s| s.checks.is_empty());
        if !unchecked.is_empty() {
            confessions.push(format!(
                "{} of {total} step(s) have no check — {}: no Verification VERIFIES them, so \
                 nothing says how what they do is confirmed.",
                unchecked.len(),
                unchecked.join(", ")
            ));
        }
        let joins = self.joins_between(steps.iter().map(|s| {
            (
                s.capability_id.as_str(),
                s.puts_together.as_slice(),
                s.closes.as_slice(),
            )
        }))?;
        for j in joins.iter().filter(|j| j.closed_by.is_empty()) {
            let quoted = |ids: &[String]| {
                ids.iter()
                    .map(|i| format!("'{i}'"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            confessions.push(format!(
                "join '{}' ({}) is between {} (which provides it) and {} (which consumes it), \
                 parts this flow puts together, and no step of the flow closes it — the \
                 instructions never say when it is made, or check that what it must hold \
                 holds.",
                j.id,
                j.name,
                quoted(&j.provided_by),
                quoted(&j.consumed_by)
            ));
        }
        Ok(())
    }

    /// The parts a step puts together (PUTS_TOGETHER) and the joins it closes
    /// (CLOSES), each sorted by id. A target whose node is not there is listed
    /// by id and said in `unreadable`.
    fn step_parts_and_joins(
        &self,
        step_id: &str,
        unreadable: &mut Vec<String>,
    ) -> Result<(Vec<StepNamed>, Vec<StepJoin>), DynoError> {
        let mut parts = Vec::new();
        for e in self.outgoing(step_id, Some(edge::PUTS_TOGETHER))? {
            let part = self.get_node(node::COMPONENT, &e.to_id)?;
            if part.is_none() {
                unreadable.push(format!(
                    "step '{step_id}' puts together '{}', which names no Component — the part is \
                     listed by id only, because there is nothing to read.",
                    e.to_id
                ));
            }
            parts.push(StepNamed {
                name: name_or_id(part.as_ref(), &e.to_id),
                id: e.to_id,
            });
        }
        parts.sort_by(|a, b| a.id.cmp(&b.id));
        let mut joins = Vec::new();
        for e in self.outgoing(step_id, Some(edge::CLOSES))? {
            let join = self.get_node(node::INTERFACE, &e.to_id)?;
            if join.is_none() {
                unreadable.push(format!(
                    "step '{step_id}' closes '{}', which names no Interface — the join is listed \
                     by id only, because there is nothing to read.",
                    e.to_id
                ));
            }
            let said = |key: &str| {
                join.as_ref()
                    .and_then(|n| n.properties.get(key))
                    .and_then(Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_string)
            };
            joins.push(StepJoin {
                name: name_or_id(join.as_ref(), &e.to_id),
                what_must_hold: said("description").or_else(|| said("spec")),
                id: e.to_id,
            });
        }
        joins.sort_by(|a, b| a.id.cmp(&b.id));
        Ok((parts, joins))
    }

    /// Who performs a step (an Actor INTERACTS_WITH it, `interaction:
    /// performs`), its checks (Verifications that VERIFY it) and its limits
    /// (Constraints and DesignRules that CONSTRAIN it), each sorted by id.
    #[allow(clippy::type_complexity)]
    fn step_performers_checks_and_limits(
        &self,
        step_id: &str,
    ) -> Result<(Vec<StepNamed>, Vec<StepCheck>, Vec<StepNamed>), DynoError> {
        let mut performed_by = Vec::new();
        for e in self.incoming(step_id, Some(edge::INTERACTS_WITH))? {
            if e.properties.get("interaction").and_then(Value::as_str) != Some(PERFORMS) {
                continue;
            }
            let actor = self.get_node(node::ACTOR, &e.from_id)?;
            performed_by.push(StepNamed {
                name: name_or_id(actor.as_ref(), &e.from_id),
                id: e.from_id,
            });
        }
        performed_by.sort_by(|a, b| a.id.cmp(&b.id));
        let mut checks = Vec::new();
        for e in self.incoming(step_id, Some(edge::VERIFIES))? {
            let check = self.get_node(node::VERIFICATION, &e.from_id)?;
            let said = |key: &str| {
                check
                    .as_ref()
                    .and_then(|n| n.properties.get(key))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            };
            checks.push(StepCheck {
                name: name_or_id(check.as_ref(), &e.from_id),
                method: said("method"),
                status: said("status"),
                id: e.from_id,
            });
        }
        checks.sort_by(|a, b| a.id.cmp(&b.id));
        let mut limits = Vec::new();
        for e in self.incoming(step_id, Some(edge::CONSTRAINS))? {
            let limit = match self.get_node(node::CONSTRAINT, &e.from_id)? {
                Some(n) => Some(n),
                None => self.get_node(node::DESIGN_RULE, &e.from_id)?,
            };
            limits.push(StepNamed {
                name: name_or_id(limit.as_ref(), &e.from_id),
                id: e.from_id,
            });
        }
        limits.sort_by(|a, b| a.id.cmp(&b.id));
        Ok((performed_by, checks, limits))
    }

    /// The joins between an assembly's own parts: each Interface one part the
    /// flow puts together PROVIDES and ANOTHER CONSUMES, with the flow's
    /// steps that close it — the join no step closes is what `flow_report`
    /// confesses. Read from what each step names, so it answers for a flow of
    /// any type; only an `assembly` is asked about it.
    pub fn assembly_joins(&self, flow_id: &str) -> Result<Vec<AssemblyJoin>, DynoError> {
        let mut unreadable = Vec::new();
        let mut named = Vec::new();
        for e in self.incoming(flow_id, Some(edge::PART_OF_FLOW))? {
            let (parts, joins) = self.step_parts_and_joins(&e.from_id, &mut unreadable)?;
            named.push((e.from_id, parts, joins));
        }
        self.joins_between(
            named
                .iter()
                .map(|(s, p, j)| (s.as_str(), p.as_slice(), j.as_slice())),
        )
    }

    /// [`Self::assembly_joins`] over what the steps name: `(step, its parts,
    /// its joins)`.
    fn joins_between<'a>(
        &self,
        steps: impl IntoIterator<Item = (&'a str, &'a [StepNamed], &'a [StepJoin])>,
    ) -> Result<Vec<AssemblyJoin>, DynoError> {
        let mut parts: BTreeSet<&str> = BTreeSet::new();
        let mut closed: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for (step, put, made) in steps {
            parts.extend(put.iter().map(|p| p.id.as_str()));
            for j in made {
                closed.entry(j.id.as_str()).or_default().insert(step);
            }
        }
        let mut sides: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
        for p in &parts {
            for e in self.outgoing(p, Some(edge::PROVIDES))? {
                sides.entry(e.to_id).or_default().0.insert((*p).to_string());
            }
            for e in self.outgoing(p, Some(edge::CONSUMES))? {
                sides.entry(e.to_id).or_default().1.insert((*p).to_string());
            }
        }
        let mut joins = Vec::new();
        for (id, (provided_by, consumed_by)) in sides {
            // BETWEEN TWO PARTS: one provides it and ANOTHER consumes it.
            let between = provided_by
                .iter()
                .any(|p| consumed_by.iter().any(|c| c != p));
            if !between {
                continue;
            }
            let join = self.get_node(node::INTERFACE, &id)?;
            joins.push(AssemblyJoin {
                name: name_or_id(join.as_ref(), &id),
                provided_by: provided_by.into_iter().collect(),
                consumed_by: consumed_by.into_iter().collect(),
                closed_by: closed
                    .get(id.as_str())
                    .map(|s| s.iter().map(|x| (*x).to_string()).collect())
                    .unwrap_or_default(),
                id,
            });
        }
        Ok(joins)
    }
}

/// The `flow_type` that reads a flow as instructions.
pub const ASSEMBLY: &str = "assembly";

/// The `INTERACTS_WITH.interaction` that names who performs a step.
pub const PERFORMS: &str = "performs";

/// A node's `name`, or `id` when it has none or is not there.
fn name_or_id(n: Option<&StoredNode>, id: &str) -> String {
    n.and_then(|n| n.properties.get("name"))
        .and_then(Value::as_str)
        .unwrap_or(id)
        .to_string()
}

/// One representative cycle per strongly-connected cluster of the transition
/// graph, plus degenerate self-triggers. Paths are rotated to start at their
/// smallest id so the output is byte-stable run to run.
fn cycles_among(transitions: &[FlowTransition]) -> Vec<FlowCycle> {
    let mut cycles = Vec::new();
    if transitions.is_empty() {
        return cycles;
    }

    let mut builder = GraphBuilder::new();
    for t in transitions {
        builder.add_node(&t.from_id);
        builder.add_node(&t.to_id);
    }
    for t in transitions {
        if t.from_id == t.to_id {
            cycles.push(FlowCycle {
                members: vec![t.from_id.clone()],
                path: vec![t.from_id.clone()],
            });
            continue;
        }
        // Endpoints were added above; a duplicate edge is fine for SCC purposes.
        let _ = builder.add_edge(&t.from_id, &t.to_id, 1.0);
    }
    let graph = builder.build(true);

    let mut meta = vec![String::new(); graph.node_count()];
    for t in transitions {
        for id in [&t.from_id, &t.to_id] {
            if let Some(idx) = graph.idx_of(id) {
                meta[idx] = id.clone();
            }
        }
    }

    for group in strongly_connected_components(&graph).groups() {
        if group.len() < 2 {
            continue;
        }
        let members: BTreeSet<usize> = group.iter().copied().collect();
        let mut sub = GraphBuilder::new();
        let mut sub_ids: Vec<&str> = group.iter().map(|&i| meta[i].as_str()).collect();
        sub_ids.sort_unstable();
        for id in &sub_ids {
            sub.add_node(id);
        }
        for &i in &group {
            for &(j, _) in graph.out_neighbors(i) {
                if members.contains(&j) {
                    let _ = sub.add_edge(&meta[i], &meta[j], 1.0);
                }
            }
        }
        let sub_graph = sub.build(true);
        if let Some(path) = find_cycle(&sub_graph) {
            let mut ids: Vec<String> = path
                .iter()
                .map(|&i| {
                    sub_ids
                        .iter()
                        .find(|id| sub_graph.idx_of(id) == Some(i))
                        .map(|id| id.to_string())
                        .unwrap_or_default()
                })
                .collect();
            if let Some(start) = ids
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.cmp(b.1))
                .map(|(i, _)| i)
            {
                ids.rotate_left(start);
            }
            cycles.push(FlowCycle {
                // The cluster's full membership — the honest answer to "what
                // is caught in this loop?". The walk below may be shorter.
                members: sub_ids.iter().map(|s| (*s).to_string()).collect(),
                path: ids,
            });
        }
    }
    cycles.sort_by(|a, b| a.members.cmp(&b.members));
    cycles
}
