//! PROCESS TOOLS: one call that runs, in order, a process the surface already
//! serves as separate tools, with a receipt per step.
//!
//! # Why they exist
//!
//! `dec:idea-a-few-core-tools-each-running-a-process-of-subtools` (settled
//! 2026-10-25, b + c, Anthony: "Can we do b + c?"): a core tool runs its
//! common process of subtools in one call, saying which step refused if one
//! does; and the surface is GROUPED by the five jobs of
//! `dec:idea-reflow2-does-fewer-things-and-does-them-really-well`. His later
//! word the same day: the jobs group the LISTING and are not a call layer. A
//! call stays two steps, the door and then a process tool or a subtool, so no
//! tool is reached through a job, and every new tool names the job it serves.
//!
//! Which processes come first was MEASURED, not guessed
//! (`fact:the-processes-people-and-agents-repeat-most-2026-10-25`): agents ran
//! add_change_event then set_artifact_checksums 129 times in a month, and the
//! whole record-what-was-built sequence (verification, change, checksums, then
//! artifact or mark-built) 16 times. The same week three root causes found one
//! class: what a tool does in the same call gets done, and a separate step
//! mostly does not.
//!
//! # The rules every process tool here keeps
//!
//! - It RUNS THE TOOLS IT COMPOSES, through their own handlers, so each step
//!   checks and writes exactly what that tool does, and those tools stay
//!   served for the unusual case.
//! - ONE WRITE: the whole process is the call's write unit
//!   (`crate::service::unit`), so a refused step takes every earlier step back
//!   with it. Driven directly (a test, a one-shot mode), the tool opens the
//!   unit a served call gets, so the rule holds on every path.
//! - A refusal NAMES ITS STEP: its number, its name and the tool that refused.
//! - A RECEIPT PER STEP, each the composed tool's own reply (shaped as every
//!   write's receipt is, at `call_tool`).

#![allow(unused_imports)]

use rmcp::{
    ErrorData as McpError,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};

use crate::service::*;

/// The job this file's first tool serves, in the owner's words (job 4 of the
/// five, 2026-10-13).
pub(crate) const JOB_4: &str = "Job 4 of reflow2's five: show whether what was built meets what \
     was asked, by checks anyone can re-run.";

/// An optional step's schema: the step's own object, `null` allowed, inline
/// rather than an `anyOf` of a `$ref` and `null` — the keyword subset every
/// served schema is held to (as `include_edges` is), so the argument check
/// reads it.
fn nullable_object<T: JsonSchema>(g: &mut schemars::SchemaGenerator) -> schemars::Schema {
    let mut s = T::json_schema(g);
    if let Some(o) = s.as_object_mut() {
        o.insert("type".into(), json!(["object", "null"]));
    }
    s
}

/// Step 1: the check that PASSED.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BuiltCheckReq {
    /// The Verification's id (`ver:…`). An existing check is revised; a new
    /// one needs `name`.
    pub id: String,
    /// A short name, for a new check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// HOW it was checked, as add_verification takes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "crate::enum_schema::verification_method_opt")]
    pub method: Option<String>,
    /// What the check IS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// What it checks: the ids of the Capabilities, Components, Requirements
    /// or Artifacts it VERIFIES.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verifies: Option<Vec<String>>,
    /// When it ran and passed, as a plain date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run_at: Option<String>,
    /// What the passing run found.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub findings: Option<String>,
}

/// Step 2: the change, as add_change_event takes it.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BuiltChangeReq {
    /// The ChangeEvent's id (`chg:…`).
    pub id: String,
    /// A short name, for a new change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// WHY it changed: `new_feature`, `defect_fix` and the rest
    /// add_change_event takes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "crate::enum_schema::change_event_change_type_opt")]
    pub change_type: Option<String>,
    /// `system` (the thing changed) or `record` (only the design's knowledge
    /// of it did). Absent means nobody said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "crate::enum_schema::change_event_subject_opt")]
    pub subject: Option<String>,
    /// What changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Why, and the lesson.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    /// When it landed, as a plain date. Dates each answer and acceptance too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected_at: Option<String>,
    /// The commits it was made in, comma-separated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commits: Option<String>,
    /// For a repair: `corrected_cause` or `contained_symptom`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "crate::enum_schema::repair_req")]
    pub repair: Option<String>,
    /// What the proper fix would be; required with `contained_symptom`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stands_in_for: Option<String>,
    /// The nodes it changed (a CHANGED edge to each). The files in step 3
    /// are joined to it by their acceptance and need not be listed here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affected: Option<Vec<AffectedNodeReq>>,
    /// The findings it answered (INVALIDATES to each), or `[]` for none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answers: Option<Vec<AnswerReq>>,
}

/// Step 3: one file the work changed.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BuiltFileReq {
    /// The Artifact's id (`art:…`). One not yet registered is registered
    /// first, and then needs `name`.
    pub artifact_id: String,
    /// A name, to register a new file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Where it lives, to register a new file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// What kind of file, to register a new one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_type: Option<String>,
    /// Its new checksum, accepted on this call's change. Omitted, a server
    /// that can read the file measures it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    /// The Verification this file RUNS (IMPLEMENTS): a test file names the
    /// check it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runs: Option<String>,
}

/// Step 4: one thing now built.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BuiltItemReq {
    /// A Capability (`cap:…`), or a Decision (`dec:…`) the work carried out.
    pub id: String,
    /// For a Capability: `realized` (the default) or `verified`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Revise its words to say it is built, as replace_text does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "nullable_object::<BuiltReplaceReq>")]
    pub replace: Option<BuiltReplaceReq>,
}

/// replace_text's arguments, for the node of a `built` entry.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BuiltReplaceReq {
    /// The field to revise (`name`, `description` ...).
    pub field: String,
    /// The exact text to replace; omitted, `new` is appended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old: Option<String>,
    /// What it becomes.
    pub new: String,
}

/// Record what was built, in one call.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordWhatWasBuiltReq {
    /// Step 1: the check that passed. Omit it for work no check covers.
    #[serde(default)]
    #[schemars(schema_with = "nullable_object::<BuiltCheckReq>")]
    pub verification: Option<BuiltCheckReq>,
    /// Step 2: the change, with `answers` naming the findings it closed.
    pub change: BuiltChangeReq,
    /// Step 3: the files the work changed, each registered if new and its
    /// checksum accepted on the change.
    #[serde(default)]
    pub files: Option<Vec<BuiltFileReq>>,
    /// Step 4: the capabilities and decisions now built.
    #[serde(default)]
    pub built: Option<Vec<BuiltItemReq>>,
}

/// A step's refusal, naming the step and the tool that refused.
fn refused(n: usize, step: &str, tool: &str, why: &str) -> McpError {
    McpError::invalid_params(
        format!(
            "record_the_build: step {n} ({step}) refused, in {tool}: {why} Nothing was \
             written: the steps are one write, so a refused step takes the others back with it. \
             Correct that step and send the whole call again."
        ),
        None,
    )
}

/// A composed tool's answer as a step's receipt, or its refusal as the step's.
fn receipt_of(
    n: usize,
    step: &str,
    tool: &str,
    answer: Result<CallToolResult, McpError>,
) -> Result<JsonValue, McpError> {
    match answer {
        Err(e) => Err(refused(n, step, tool, &e.message)),
        Ok(r) if r.is_error == Some(true) => {
            let text = r
                .content
                .iter()
                .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                .collect::<Vec<_>>()
                .join(" ");
            Err(refused(n, step, tool, &text))
        }
        Ok(r) => Ok(r.structured_content.unwrap_or(JsonValue::Null)),
    }
}

/// Arguments for a composed tool, read as that tool reads them.
fn args<T: serde::de::DeserializeOwned>(
    n: usize,
    step: &str,
    tool: &str,
    v: JsonValue,
) -> Result<T, McpError> {
    serde_json::from_value(v).map_err(|e| refused(n, step, tool, &e.to_string()))
}

fn row(n: usize, step: &str, tools: &[&str], receipt: JsonValue) -> JsonValue {
    json!({ "n": n, "step": step, "tools": tools, "tool": tools.first(), "receipt": receipt })
}

#[tool_router(router = process_router, vis = "pub")]
impl ReflowService {
    #[tool(
        description = "Record what you built in one call, at the end of a piece of work: its passing \
                       verification, its change event and the findings it answered, its files' \
                       checksums, and the work marked built. It serves job 4 of reflow2's five, \
                       whether what was built meets what was asked. Steps, in order, each with its \
                       own receipt: `verification` (recorded as passing); `change` (with `answers`, \
                       which draws INVALIDATES); `files` (each file registered if it is not yet, its \
                       checksum accepted against the change, and a test file joined to the \
                       verification it `runs`); `built` (each one's status set to realized, or a dec: \
                       joined to them; `replace` revises its words). ALL OR NOTHING: a refused step \
                       names itself, and nothing is written. The tools it runs (add_verification, \
                       add_change_event, set_artifact_checksums, set_capability_status, replace_text) \
                       stay served. Ask for it to record what you built.",
        title = "Record the Build",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub async fn record_the_build(
        &self,
        Parameters(req): Parameters<RecordWhatWasBuiltReq>,
    ) -> Result<CallToolResult, McpError> {
        if CallScope::current().is_some() {
            return self.record_the_build_steps(req).await;
        }
        // Driven directly, outside a served call: open the unit a served call
        // gets, so the steps are one write here too.
        let unit = CallUnit::for_graph(&self.graph);
        let scope = CallScope {
            tool: "record_the_build".into(),
            unit: Some(unit.clone()),
        };
        let answer = scope.run(self.record_the_build_steps(req)).await;
        let succeeded = matches!(&answer, Ok(r) if r.is_error != Some(true));
        match unit.settle(succeeded).await {
            Settled::Committed(wrote) => {
                if wrote > 0 {
                    self.announce_write();
                }
                answer
            }
            Settled::Untouched | Settled::Discarded => answer,
            Settled::CommitFailed(why) => Err(McpError::internal_error(
                format!(
                    "record_the_build: every step succeeded but the write could not be \
                     committed, so NOTHING WAS WRITTEN. Send the same call again. ({why})"
                ),
                None,
            )),
        }
    }
}

impl ReflowService {
    async fn record_the_build_steps(
        &self,
        req: RecordWhatWasBuiltReq,
    ) -> Result<CallToolResult, McpError> {
        let mut steps = Vec::new();
        let change_id = req.change.id.clone();
        let change_at = req.change.detected_at.clone();

        // ---- 1. verification ----------------------------------------------
        let check_id = req.verification.as_ref().map(|v| v.id.clone());
        match req.verification {
            Some(v) => {
                let (n, step, tool) = (1, "verification", "add_verification");
                let mut a = json!({
                    "id": v.id,
                    "status": "passing",
                });
                for (k, val) in [
                    ("name", v.name),
                    ("method", v.method),
                    ("description", v.description),
                    ("last_run_at", v.last_run_at),
                    ("findings", v.findings),
                ] {
                    if let Some(val) = val {
                        a[k] = json!(val);
                    }
                }
                if let Some(targets) = v.verifies {
                    a["verifies"] = targets
                        .into_iter()
                        .map(|t| json!({ "target_id": t }))
                        .collect();
                }
                let r = receipt_of(
                    n,
                    step,
                    tool,
                    self.add_verification(Parameters(args(n, step, tool, a)?))
                        .await,
                )?;
                steps.push(row(n, step, &[tool], r));
            }
            None => {
                steps.push(json!({ "n": 1, "step": "verification", "skipped": "no check given" }))
            }
        }

        // ---- 2. change ----------------------------------------------------
        {
            let (n, step, tool) = (2, "change", "add_change_event");
            let a = serde_json::to_value(&req.change).map_err(ser_err)?;
            let r = receipt_of(
                n,
                step,
                tool,
                self.add_change_event(Parameters(args(n, step, tool, a)?))
                    .await,
            )?;
            steps.push(row(n, step, &[tool], r));
        }

        // ---- 3. files -----------------------------------------------------
        match req.files.filter(|f| !f.is_empty()) {
            Some(files) => {
                let (n, step) = (3, "files");
                let mut registered = Vec::new();
                let mut new_files = Vec::new();
                for f in &files {
                    let exists = {
                        let g = self.graph.read().await;
                        g.get_node(reflow2_core::nodes::node::ARTIFACT, &f.artifact_id)
                            .map_err(dyno_err)?
                            .is_some()
                    };
                    if exists {
                        continue;
                    }
                    let tool = "add_artifact";
                    let mut a = json!({ "id": f.artifact_id });
                    for (k, val) in [
                        ("name", &f.name),
                        ("location", &f.location),
                        ("artifact_type", &f.artifact_type),
                    ] {
                        if let Some(val) = val {
                            a[k] = json!(val);
                        }
                    }
                    registered.push(receipt_of(
                        n,
                        step,
                        tool,
                        self.add_artifact(Parameters(args(n, step, tool, a)?)).await,
                    )?);
                    new_files.push(f.artifact_id.clone());
                }
                let tool = "set_artifact_checksums";
                let accepts: Vec<JsonValue> = files
                    .iter()
                    .map(|f| {
                        let mut a = json!({
                            "artifact_id": f.artifact_id,
                            "disposition": "design_updated",
                            "design_change_event_id": change_id,
                        });
                        if let Some(c) = &f.checksum {
                            a["checksum"] = json!(c);
                        }
                        if let Some(at) = &change_at {
                            a["at"] = json!(at);
                        }
                        a
                    })
                    .collect();
                let accepted = receipt_of(
                    n,
                    step,
                    tool,
                    self.set_artifact_checksums(Parameters(args(
                        n,
                        step,
                        tool,
                        json!({ "accepts": accepts }),
                    )?))
                    .await,
                )?;
                // A file this call registered has no earlier content, so its
                // acceptance is its FIRST BASELINE (the house rule: nothing
                // moved, and no change can claim a movement). The change still
                // says it added the file: CHANGED, `added`.
                {
                    let mut g = self.write_lock().await?;
                    for id in &new_files {
                        g.draw_edge_keeping(
                            reflow2_core::nodes::edge::CHANGED,
                            reflow2_core::nodes::node::CHANGE_EVENT,
                            &change_id,
                            reflow2_core::nodes::node::ARTIFACT,
                            id,
                            reflow2_core::nodes::Props::new().set("action", "added"),
                        )
                        .map_err(|e| refused(n, step, "CHANGED", &e.to_string()))?;
                    }
                }
                let mut implements = Vec::new();
                for f in files.iter().filter(|f| f.runs.is_some()) {
                    let check = f.runs.as_deref().unwrap_or_default();
                    let mut g = self.write_lock().await?;
                    if g.get_node(reflow2_core::nodes::node::VERIFICATION, check)
                        .map_err(dyno_err)?
                        .is_none()
                    {
                        return Err(refused(
                            n,
                            step,
                            "IMPLEMENTS",
                            &format!(
                                "{} runs {check:?}, and no Verification has that id{}.",
                                f.artifact_id,
                                match &check_id {
                                    Some(id) => format!(" (this call's check is {id:?})"),
                                    None => String::new(),
                                }
                            ),
                        ));
                    }
                    g.draw_edge_keeping(
                        reflow2_core::nodes::edge::IMPLEMENTS,
                        reflow2_core::nodes::node::ARTIFACT,
                        &f.artifact_id,
                        reflow2_core::nodes::node::VERIFICATION,
                        check,
                        reflow2_core::nodes::Props::new(),
                    )
                    .map_err(|e| refused(n, step, "IMPLEMENTS", &e.to_string()))?;
                    implements.push(json!({ "artifact_id": f.artifact_id, "runs": check }));
                }
                steps.push(row(
                    n,
                    step,
                    &["add_artifact", "set_artifact_checksums", "IMPLEMENTS"],
                    json!({
                        "registered": registered,
                        "added_by_the_change": new_files,
                        "accepted": accepted,
                        "implements": implements,
                    }),
                ));
            }
            None => steps.push(json!({ "n": 3, "step": "files", "skipped": "no files given" })),
        }

        // ---- 4. built -----------------------------------------------------
        match req.built.filter(|b| !b.is_empty()) {
            Some(items) => {
                let (n, step) = (4, "built");
                // Resolve every entry first: a type this step cannot mark is
                // refused before it marks anything (and the unit takes back
                // the earlier steps).
                let mut typed = Vec::new();
                {
                    let g = self.graph.read().await;
                    for it in &items {
                        let ty = crate::service::resolve_node_type(&g, None, &it.id, "built[].id")
                            .map_err(|e| refused(n, step, "built", &e.message))?;
                        if g.get_node(&ty, &it.id).map_err(dyno_err)?.is_none() {
                            return Err(refused(
                                n,
                                step,
                                "built",
                                &format!("{} {:?} does not exist.", ty, it.id),
                            ));
                        }
                        if ty != reflow2_core::nodes::node::CAPABILITY
                            && ty != reflow2_core::nodes::node::DECISION
                        {
                            return Err(refused(
                                n,
                                step,
                                "built",
                                &format!(
                                    "{} is a {ty}; this step marks a Capability or a Decision \
                                     built.",
                                    it.id
                                ),
                            ));
                        }
                        typed.push(ty);
                    }
                }
                let capabilities: Vec<&str> = items
                    .iter()
                    .zip(&typed)
                    .filter(|(_, t)| t.as_str() == reflow2_core::nodes::node::CAPABILITY)
                    .map(|(it, _)| it.id.as_str())
                    .collect();
                let mut marked = Vec::new();
                for (it, ty) in items.iter().zip(&typed) {
                    let mut receipt = json!({ "id": it.id, "node_type": ty });
                    if ty == reflow2_core::nodes::node::CAPABILITY {
                        let tool = "set_capability_status";
                        let a = json!({
                            "capability_id": it.id,
                            "status": it.status.as_deref().unwrap_or("realized"),
                        });
                        receipt["status"] = receipt_of(
                            n,
                            step,
                            tool,
                            self.set_capability_status(Parameters(args(n, step, tool, a)?))
                                .await,
                        )?;
                    } else {
                        if it.status.is_some() {
                            return Err(refused(
                                n,
                                step,
                                "built",
                                &format!(
                                    "{}: `status` is a Capability's; a Decision's status is \
                                     its owner's word (set_decision_status).",
                                    it.id
                                ),
                            ));
                        }
                        if capabilities.is_empty() && it.replace.is_none() {
                            return Err(refused(
                                n,
                                step,
                                "built",
                                &format!(
                                    "{}: nothing would show this decision built. List the \
                                     capability that carries it out in the same `built`, or \
                                     pass `replace` to revise its words.",
                                    it.id
                                ),
                            ));
                        }
                        let mut g = self.write_lock().await?;
                        let exploratory = g
                            .get_node(reflow2_core::nodes::node::DECISION, &it.id)
                            .map_err(dyno_err)?
                            .and_then(|d| {
                                d.properties
                                    .get("kind")
                                    .and_then(|k| k.as_str())
                                    .map(|k| k == "exploratory")
                            })
                            .unwrap_or(false);
                        let mut drawn = Vec::new();
                        for cap in &capabilities {
                            let e = if exploratory {
                                g.draw_edge_keeping(
                                    reflow2_core::nodes::edge::EVOLVES_INTO,
                                    reflow2_core::nodes::node::DECISION,
                                    &it.id,
                                    reflow2_core::nodes::node::CAPABILITY,
                                    cap,
                                    reflow2_core::nodes::Props::new(),
                                )
                            } else {
                                g.draw_edge_keeping(
                                    reflow2_core::nodes::edge::GOVERNED_BY,
                                    reflow2_core::nodes::node::CAPABILITY,
                                    cap,
                                    reflow2_core::nodes::node::DECISION,
                                    &it.id,
                                    reflow2_core::nodes::Props::new(),
                                )
                            }
                            .map_err(|e| refused(n, step, "built", &e.to_string()))?;
                            drawn.push(format!("{} {} {}", e.from_id, e.edge_type, e.to_id));
                        }
                        receipt["drawn"] = json!(drawn);
                        receipt["status_unchanged"] = json!(
                            "A Decision's status is its owner's word: this step draws what \
                             built it and never moves its status."
                        );
                    }
                    if let Some(rep) = &it.replace {
                        let tool = "replace_text";
                        let mut a = json!({ "node_id": it.id, "field": rep.field, "new": rep.new });
                        if let Some(old) = &rep.old {
                            a["old"] = json!(old);
                        }
                        receipt["replaced"] = receipt_of(
                            n,
                            step,
                            tool,
                            self.replace_text(Parameters(args(n, step, tool, a)?)).await,
                        )?;
                    }
                    marked.push(receipt);
                }
                steps.push(row(
                    n,
                    step,
                    &[
                        "set_capability_status",
                        "replace_text",
                        "GOVERNED_BY / EVOLVES_INTO",
                    ],
                    json!({ "marked": marked }),
                ));
            }
            None => {
                steps.push(json!({ "n": 4, "step": "built", "skipped": "nothing marked built" }))
            }
        }

        let ran = steps.iter().filter(|s| s.get("receipt").is_some()).count();
        ok_json(json!({
            "recorded": format!("{ran} of 4 steps, in one write"),
            "job": JOB_4,
            "steps": steps,
        }))
    }
}
