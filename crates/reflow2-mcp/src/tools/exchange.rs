//! `exchange` tools — one slice of the MCP surface.
//!
//! Split out of `service.rs` under BL-181, which had grown to 6,356 lines and
//! 139 tools in one file: the design distinguished the systems these tools
//! serve and the build did not separate them at all. That mismatch is what
//! `granularity_report` reported, and this is the answer to it.
//!
//! **Function is unchanged by construction.** Every item here moved verbatim;
//! nothing was rewritten. `rmcp` composes routers, so this module declares its
//! own and `ReflowService::new` sums them — the surface a client sees is
//! byte-identical, which `tools/toolsnap.py` is what proves rather than claims.

#![allow(unused_imports)]

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, ContentBlock, Implementation, ProtocolVersion, ServerCapabilities,
        ServerConfig,
    },
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map as JsonMap, Value as JsonValue, json};
use tokio::sync::RwLock;

use reflow2_core::bulk::{
    AskedRecord as BulkAskedRecord, ChecksumAccept as BulkChecksumAccept, EdgeSpec as BulkEdgeSpec,
    GapAck as BulkGapAck, NodeSpec as BulkNodeSpec,
};
use reflow2_core::temporal::ChangeRecord;
use reflow2_core::{
    AgentAnswer, AgentBackend, AskedQuestion, ChangeType, DEFAULT_SCOPE_DEPTH, DesignGraph,
    Dimension, DriftDisposition, DynoError, EpochType, GapCandidate, GenesisOptions, HealOptions,
    HealProposal, HealStrategy, IngestOptions, LinkArtifactOptions, LoopStatus, ObservedArtifact,
    ObservedPath, PromptCollector, PropagateOptions, ReadinessForecast, ReadinessGate,
    ReadinessKind, ReadinessObservation, ReconcileOptions, StoredNode, Value,
};

use crate::dto::{EdgeDto, NodeDto};
use crate::service::*;

/// THE OTHER DESIGN of a seam tool, from exactly one of two sources: inline,
/// as always, or BY GRAPH_ID through the registry this design is served from
/// (`req:every-cross-design-operation-takes-its-other-designs-by-graph-id-under-the-hosts-read-filter`),
/// resolved and authorised by `other_design`, the one door, which drops the
/// other design's guard before it returns — so the caller then reads its own
/// design holding one guard, never two. `verb` finishes "which one was …".
async fn inline_or_by_graph_id(
    service: &ReflowService,
    design: Option<JsonMap<String, JsonValue>>,
    other_graph_id: Option<&str>,
    verb: &str,
) -> Result<reflow2_core::GraphExport, McpError> {
    match (design, other_graph_id) {
        (Some(_), Some(_)) => Err(McpError::invalid_params(
            format!(
                "pass design OR other_graph_id, not both — with two other designs there is no \
                 way to say which one was {verb}."
            ),
            None,
        )),
        (None, None) => Err(McpError::invalid_params(
            "no other design: pass design (an export document) or other_graph_id (another \
             design this server holds)."
                .to_string(),
            None,
        )),
        (Some(design), None) => serde_json::from_value(JsonValue::Object(design))
            .map_err(|e| McpError::invalid_params(format!("not an export document: {e}"), None)),
        (None, Some(other_id)) => service.other_design(other_id).await,
    }
}

#[tool_router(router = exchange_router, vis = "pub")]
impl ReflowService {
    #[tool(
        description = "Compute the seam between this design and another by COMPLEMENTARY ROLE, instead of \
                       hand-asserting which boundaries correspond. Each boundary declares a role on \
                       `Interface.designation` and pairing matches COMPLEMENTS — `published`/`both` against \
                       `required`/`both` — never like with like. Two boundaries pair when their NAMES match fuzzily AND they agree \
                       on medium, transport_security and auth. FIVE OUTCOMES, all useful: paired (the seam, \
                       computed); CONFLICTS, where the names match but the axes refuse — reported with EVERY \
                       refusing axis, never dropped as a non-match, because \"you publish this, I need this, \
                       and we cannot connect as either is built\" is the finding worth having; unmet needs (we \
                       require it, nobody publishes it — the loudest signal); dead surface (they publish it, \
                       nobody here needs it); and duplicate providers (two publishers of one need is a \
                       conflict, not a match). Uncertain name matches are CANDIDATES to ask about, never \
                       actions. Boundaries carrying no role are counted and NAMED, because `internal` is the \
                       DEFAULT and cannot tell \"deliberately internal\" from \"never classified\" — otherwise \
                       a design that did no labelling reports a clean seam. Feed `paired` to seam_report to \
                       learn whether the full contracts agree (req:complementary-pairing). On a registry server, \
                       `other_graph_id` instead of `design`. A name pair is not evidence the designs touch; `computed_links` are. Ask for this to match our required contracts against what the other system publishes.",
        title = "Pair Two Designs",
        annotations(read_only_hint = true)
    )]
    pub async fn pair_designs(
        &self,
        Parameters(req): Parameters<PairDesignsReq>,
    ) -> Result<CallToolResult, McpError> {
        let other =
            inline_or_by_graph_id(self, req.design, req.other_graph_id.as_deref(), "paired")
                .await?;
        let g = self.graph.read().await;
        ok_json(g.pair_designs(&other).map_err(dyno_err)?)
    }

    #[tool(
        description = "Compare paired boundaries across a seam and say where two designs \
                       DISAGREE — the check the ordinary detectors cannot do, because they \
                       reason about structure and a contract mismatch is a comparison of \
                       PROPERTIES ACROSS A PAIR. Compares medium, paradigm, payload format, \
                       auth, transport security, operations, error model and payload schema. \
                       THREE RULES WORTH KNOWING: `unspecified` on either side reports as \
                       UNSTATED, never as agreement, so 0 incompatibilities can never be read \
                       as compatible; free-text axes report as DIFFERS for a person to read, \
                       never as incompatible, because a machine cannot tell a real mismatch \
                       from different wording; and the report always names what it did NOT \
                       examine — the types that CROSS these boundaries are part of the contract \
                       and are invisible to it. On a registry server, `other_graph_id` instead of \
                       `design`. \
                       Ask for this to check whether both sides of an interface actually agree.",
        title = "Seam Report",
        annotations(read_only_hint = true)
    )]
    pub async fn seam_report(
        &self,
        Parameters(req): Parameters<SeamReportReq>,
    ) -> Result<CallToolResult, McpError> {
        let other = inline_or_by_graph_id(
            self,
            req.design,
            req.other_graph_id.as_deref(),
            "reported against",
        )
        .await?;
        let pairs: Vec<(String, String)> =
            req.pairs.into_iter().map(|p| (p.ours, p.theirs)).collect();
        let g = self.graph.read().await;
        ok_json(g.seam_report(&other, &pairs).map_err(dyno_err)?)
    }

    #[tool(
        description = "The whole design as one portable document — every node and edge, sorted so two \
                       exports of an unchanged graph are byte-identical. Use it to back the design \
                       up, move it between machines, or migrate it across a reflow2 upgrade. It \
                       carries a stamp saying which reflow2 wrote it, and `taken_at` — the branch and \
                       commit the working tree was at, and whether it was dirty. Pass `path` to write \
                       the document to a file instead of returning it — a large design overflows a \
                       session. A path ending in `/` writes the ITEM LAYOUT: one file per node and \
                       per edge, so git's ordinary merge merges disjoint changes. LINEAGE ANCHORS AT \
                       THE COMMITTED RECORD: each changed item records its hash at the merge-base \
                       with the default branch, so a squash-merge lands ONE hop however often you \
                       export, not once per PR. Outside git it chains from the file already there; \
                       `chained_from` says which anchor was used and `chain_note` why, because a hash \
                       cannot say where it came from. \u{1F6D1} NEVER ISSUE THIS IN THE SAME PARALLEL \
                       BATCH AS WRITES YOU EXPECT IT TO CONTAIN: calls a harness emits together are \
                       unordered and this takes the same lock, so it can run BEFORE them. THE FAILURE \
                       IS SILENT: the export succeeds, it is simply early, and the document you \
                       commit carries artifact hashes that will not match disk. Sequence it after the \
                       writes return. \u{26A0} The unexported-work nudge will NOT catch it — it \
                       compares NODE COUNTS, so a missed PROPERTY change is invisible. Ask for this \
                       to save the design to a file you can commit.",
        title = "Export the Design",
        annotations(read_only_hint = true)
    )]
    pub async fn export_graph(
        &self,
        Parameters(req): Parameters<ExportGraphToReq>,
    ) -> Result<CallToolResult, McpError> {
        // A replaced executable stamps the record with a version that is no
        // longer on disk. Refused before anything is read.
        let (stale, note) = crate::service::exe_replaced_since_start();
        if let Some(refusal) = crate::service::export_stale_refusal(stale, note) {
            return Err(refusal);
        }
        let stamps = req.stamps.unwrap_or(false);
        if stamps
            && let Some(path) = &req.path
            && crate::saved_design::is_item_layout(std::path::Path::new(path))
        {
            return Err(McpError::invalid_params(
                format!(
                    "REFUSED: {path} is an item layout, and the item layout never carries the                      server's stamps (dec:write-stamps-stay-in-the-store-and-travel-in-an-export-                     only-when-asked-2026-10-28): two branches making the same change to one item                      would otherwise conflict on it. Nothing was written. Drop `stamps`, or name a                      .json file for a backup that keeps them."
                ),
                None,
            ));
        }
        let g = self.graph.read().await;
        let mut export = if stamps {
            g.export_graph_with_stamps()
        } else {
            g.export_graph()
        }
        .map_err(dyno_err)?;
        let Some(path) = req.path else {
            return ok_json(export);
        };
        // A read-only server writes no file, whatever this tool's annotation
        // says about the graph.
        self.file_write_permitted("export_graph", &path)?;
        // Refuse to clobber an existing file unless the caller opts in. Graph
        // text is untrusted (the server's own instructions say so), so a stray
        // or injected `path` pointing at a real file must not silently destroy
        // it (BL-57). A new path writes freely.
        let target = std::path::Path::new(&path);
        if target.exists() && !req.overwrite.unwrap_or(false) {
            return Err(McpError::invalid_params(
                format!(
                    "{path} already exists — refusing to overwrite it. Pass overwrite=true \
                     to replace it, or choose a path that does not exist."
                ),
                None,
            ));
        }
        // THE FILE-WRITE SEAM LIVES IN `export_write`, shared with the
        // server's own write-through (`crate::auto_export`). Two callers must
        // agree on where the lineage anchors, whether the write would drop
        // design the file already holds, what `wrote` reports and recording
        // that this seat is in step — a second copy that drifts is the failure
        // this project keeps meeting.
        let written = match crate::export_write::chain_and_write(
            &mut export,
            &path,
            self.graph_path.as_deref(),
            self.sidecar_path_for_writes(),
            req.accept_divergence.unwrap_or(false),
        ) {
            Ok(w) => w,
            // A caller-supplied path that cannot be written, or a write that
            // would lose work, is the caller's to resolve — not a server fault.
            Err(refusal) => {
                return Err(McpError::invalid_params(
                    refusal.message().to_string(),
                    None,
                ));
            }
        };
        let (wrote, chained_from, chain_note, sync_note) = (
            written.wrote,
            written.chained_from,
            written.chain_note,
            written.sync_note,
        );
        // Report where it actually landed: a relative path resolves against the
        // server's cwd, which the calling agent cannot see.
        let resolved = std::fs::canonicalize(target)
            .map(|p| p.display().to_string())
            .unwrap_or(path);
        let mut receipt = json!({
            "path": resolved,
            "bytes": written.bytes,
            "nodes": export.nodes.len(),
            "edges": export.edges.len(),
            "content_hash": export.content_hash,
            "prev_content_hash": export.prev_content_hash,
            // WHAT THE CHAIN IS ANCHORED TO. `prev_content_hash` is a hash and
            // says nothing about where that hash came from, so an export
            // anchored at the committed record and one anchored at whatever was
            // last written to disk look identical in the receipt — and they
            // produce different lineages.
            "chained_from": chained_from,
            "wrote": wrote,
            "layout": if written.items.is_some() { "items" } else { "file" },
            "taken_at": serde_json::to_value(&export.taken_at).map_err(ser_err)?,
            "stamp": serde_json::to_value(&export.stamp).map_err(ser_err)?,
        });
        if wrote == "unchanged" {
            // Only on the case that misleads. A note present on every successful
            // export is the noise `search_first` and the `revision` block both
            // refuse to become.
            receipt["wrote_note"] = json!(
                "NOTHING WAS WRITTEN — the file already held exactly this design, byte for byte. \
                 That is not a failure and not a refusal, and it is worth reading twice on a \
                 shared server: it means somebody else's export already carried your work, so \
                 there was nothing left for yours to add. `content_hash` and `prev_content_hash` \
                 look identical to a successful write and cannot tell you this."
            );
        }
        if let Some(items) = written.items {
            receipt["items"] = serde_json::to_value(items).map_err(ser_err)?;
        }
        if let Some(note) = chain_note {
            receipt["chain_note"] = json!(note);
        }
        if let Some(note) = sync_note {
            receipt["sync_note"] = json!(note);
        }
        ok_json(receipt)
    }

    #[tool(
        description = "Export ONLY the published surface — the contracts others are entitled to \
                       rely on, and nothing internal. Every Interface designated `published`, the \
                       artifacts that specify or realize it (the machine-readable ICD), the \
                       components on each side, and the project. Requirements, capabilities, \
                       decisions, verifications and history stay home, and the result COUNTS what \
                       it withheld — a recipient cannot tell a small design from a filtered one, \
                       so the note says which they are holding. Use it to hand a boundary to \
                       another team or a vendor without handing over the design. Deliberately not \
                       part of the export hash chain: this is a derived view, not a record of the \
                       design, and it is not a backup. A design with no designated boundary gets \
                       an EMPTY SURFACE warning rather than a quietly empty file. \
                       Ask for this to give a vendor just our public contracts.",
        title = "Export the Published Surface",
        annotations(read_only_hint = true)
    )]
    pub async fn export_surface(
        &self,
        Parameters(req): Parameters<ExportSurfaceReq>,
    ) -> Result<CallToolResult, McpError> {
        let g = self.graph.read().await;
        let surface = g.export_surface().map_err(dyno_err)?;
        match req.path.as_deref() {
            // ⚠️ WITHHELD WHOLE, NEVER TRIMMED — the same rule get_instructions
            // and export_graph follow. This branch returns a published SURFACE
            // DOCUMENT that a consumer designs against; half of one is not a
            // smaller answer, it is a corrupt contract, and it would be
            // indistinguishable from a real surface that happens to be small.
            // The escape already exists and is better: write it to `path`.
            None => {
                let full = serde_json::to_value(&surface).map_err(ser_err)?;
                let budget = req
                    .budget_chars
                    .unwrap_or(crate::reply_budget::DEFAULT_REPLY_BUDGET_CHARS);
                let chars = crate::reply_budget::chars_of(&full);
                if chars > budget {
                    return ok_json(json!({
                        "published": surface.published,
                        "evidence": surface.evidence,
                        "nodes": surface.document.nodes.len(),
                        "edges": surface.document.edges.len(),
                        "withheld_nodes": surface.withheld_nodes,
                        "withheld_edges": surface.withheld_edges,
                        "content_hash": surface.document.content_hash,
                        "note": surface.note,
                        "budget": {
                            "applied": true,
                            "budget_chars": budget,
                            "full_chars": chars,
                            "detail": "whole_document_withheld",
                            "note": format!(
                                "WITHHELD WHOLE, NOT TRIMMED. The surface is {chars} characters \
                                 against a budget of {budget}. A published surface is a CONTRACT \
                                 somebody builds against, so a truncated one is corrupt rather \
                                 than short — and nothing in it would say so. Every count and the \
                                 content_hash are here; call again with `path` to write the whole \
                                 document to a file, or raise `budget_chars` if this client has \
                                 the room."
                            ),
                        },
                    }));
                }
                ok_json(surface)
            }
            Some(path) => {
                // A read-only server writes no file, whatever this tool's
                // annotation says about the graph.
                self.file_write_permitted("export_surface", path)?;
                let rendered = serde_json::to_string_pretty(&surface.document).map_err(ser_err)?;
                if !req.overwrite.unwrap_or(false) && std::path::Path::new(path).exists() {
                    return Err(McpError::invalid_params(
                        format!(
                            "{path} already exists — pass overwrite: true to replace it. A \
                             published surface is meant to be shared, so clobbering one silently \
                             could replace what a consumer is building against."
                        ),
                        None,
                    ));
                }
                std::fs::write(path, format!("{rendered}\n")).map_err(|e| {
                    McpError::internal_error(format!("failed to write {path}: {e}"), None)
                })?;
                ok_json(json!({
                    "path": path,
                    "published": surface.published,
                    "evidence": surface.evidence,
                    "nodes": surface.document.nodes.len(),
                    "edges": surface.document.edges.len(),
                    "withheld_nodes": surface.withheld_nodes,
                    "withheld_edges": surface.withheld_edges,
                    "content_hash": surface.document.content_hash,
                    "note": surface.note,
                }))
            }
        }
    }

    #[tool(
        description = "Mirror ANOTHER design's published surface into this graph as foreign nodes \
                       carrying the coordinate that says whose they are — which design, at what \
                       content hash, when. The composition step of dec:nested-graphs option (c): \
                       designs are separate graphs at ownership boundaries and link by mirroring, \
                       because an edge cannot cross a store. Afterwards your own components \
                       provides/consumes the mirrored Interface with ORDINARY local edges, so the \
                       golden thread, propagate and every detector work unchanged, and foreignness \
                       is a property of the node rather than of the link. COLLISIONS ARE REFUSED, \
                       never merged: an id that already exists here is left untouched and \
                       reported, because upsert would otherwise overwrite your design with \
                       somebody else's node, and two designs using one id for different things is \
                       a naming conversation between owners. A REFRESH (the same design again) \
                       updates the nodes it already holds and never deletes: a node the new surface \
                       withdraws is KEPT, marked superseded, with a snapshot and a change event; \
                       when your design rests on it the refresh is refused naming your edges, \
                       unless you pass accept_withdrawal, and what rested on it then reads \
                       reason-lost. \
                       Ask for this when you want to bring in a pinned or frozen copy of what another team's design publishes — its public interfaces.",
        title = "Mirror Another Design's Surface",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub async fn mirror_surface(
        &self,
        Parameters(req): Parameters<MirrorSurfaceReq>,
    ) -> Result<CallToolResult, McpError> {
        let doc: reflow2_core::GraphExport =
            serde_json::from_value(JsonValue::Object(req.document)).map_err(|e| {
                McpError::invalid_params(format!("not a reflow2 surface document: {e}"), None)
            })?;
        let mut g = self.write_lock().await?;
        // A mirrored node that asserts settled intent is another design's word,
        // signed THERE; here it carries nobody's name, and the intent gate reads
        // it like any other node. Written — a mirror is a dated copy — and
        // NAMED, the rule every document-carrying writer follows (2026-09-29).
        let watch = crate::settles::SettleWatch::before(
            &g,
            doc.nodes
                .iter()
                .map(|n| (n.node_type.as_str(), n.node_id.as_str())),
        );
        let mirrored = g
            .mirror_surface_with(
                &doc,
                req.at.as_deref(),
                req.accept_withdrawal.unwrap_or(false),
            )
            .map_err(dyno_err)?;
        let mut reply = serde_json::to_value(mirrored).map_err(ser_err)?;
        crate::settles::report_unsigned(&mut reply, watch.unsigned(&g));
        ok_json(reply)
    }

    #[tool(
        description = "The designs this one is composed with, and the version each was pinned to: \
                       project id, source graph, surface content hash, and when the mirror was \
                       taken. A mirror is a dated claim about a VERSION of another design, never a \
                       live truth, so this is the list to re-check when a partner publishes again.",
        title = "Mirrored Designs",
        annotations(read_only_hint = true)
    )]
    pub async fn mirrors(&self) -> Result<CallToolResult, McpError> {
        let g = self.graph.read().await;
        self.ok_read_or_why(
            &g,
            g.mirrors().map_err(dyno_err)?,
            "no other design has been composed with this one (mirror_surface takes the pinned copy)",
        )
    }

    #[tool(
        description = "Load an exported design into this graph. THE DOCUMENT SHAPE: \
                       {\"nodes\":[{\"node_type\":\"Requirement\",\"node_id\":\"req:x\",\"properties\":{...}}],\"edges\":[{\"edge_type\":\"SATISFIES\",\"from_id\":\"cap:x\",\"to_id\":\"req:x\",\"properties\":{}}]}. \
                       Only `nodes` is required: `graph_id`, `stamp` and both hashes are OPTIONAL on the way \
                       in, and `edges` may be omitted. Endpoint \
                       types are recovered from the nodes in the document or from this graph; describe_schema \
                       lists each node_type's properties. EACH NODE IS STATED WHOLE: a node already here is \
                       replaced, so a property the document leaves out is removed; the reply counts each \
                       removal and change in `properties_removed` / `properties_changed`. A node is refused only \
                       when it lacks a required property. To EDIT existing nodes, pass `merge: true` (what the \
                       document omits is kept) or use create_nodes. Anything not in the document is left \
                       alone. Atomic: a document that fails validation leaves the graph untouched, and EVERY \
                       invalid item is reported in one response; an edge whose endpoint is missing is reported, \
                       not dropped. IDENTITY: an EMPTY store adopts the document's `graph_id` \
                       (`adopted_identity`); a store already holding one keeps its name. A document from a NEWER \
                       reflow2 is REFUSED unless `accept_newer`: it would write this binary's defaults onto \
                       what the newer one left implicit. `materialized` names every \
                       Type.property written that the document did not state. Ask for this when you want to \
                       load a design or model file that someone exported or sent you.",
        title = "Import a Design",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    pub async fn import_graph(
        &self,
        Parameters(req): Parameters<ImportGraphReq>,
    ) -> Result<CallToolResult, McpError> {
        // A WRITE FROM A FILE: on a registry the path names a file on the
        // host, possibly another person's design, so it is refused; the
        // document itself, sent in the call, still imports.
        self.refuse_a_path_on_a_registry(
            &[("path", req.path.as_deref())],
            "send the export itself as `document`; another design held here is not imported by \
             graph_id yet.",
        )?;
        let read: crate::saved_design::ReadDesign = match (req.document, &req.path) {
            (Some(document), None) => crate::saved_design::ReadDesign {
                export: parse_struct_param(document, "reflow2 export")?,
                items: None,
            },
            (None, Some(path)) => crate::saved_design::read_design(path)
                .map_err(|e| McpError::invalid_params(e, None))?,
            (Some(_), Some(_)) => {
                return Err(McpError::invalid_params(
                    "pass document OR path, not both — with two sources there is no way to say                      which one was imported."
                        .to_string(),
                    None,
                ));
            }
            (None, None) => {
                return Err(McpError::invalid_params(
                    "nothing to import: pass document (an export payload) or path (a file)."
                        .to_string(),
                    None,
                ));
            }
        };
        crate::saved_design::refuse_newer_schema(&read, req.accept_newer.unwrap_or(false))
            .map_err(|e| McpError::invalid_params(e, None))?;
        let doc = &read.export;
        let mut g = self.write_lock().await?;
        let watch = crate::settles::SettleWatch::before(
            &g,
            doc.nodes
                .iter()
                .map(|n| (n.node_type.as_str(), n.node_id.as_str())),
        );
        let mut report = g
            .import_graph_with(
                doc,
                reflow2_core::export::ImportOptions {
                    accept_newer: req.accept_newer.unwrap_or(false),
                    merge: req.merge.unwrap_or(false),
                },
            )
            .map_err(dyno_err)?;
        crate::saved_design::annotate_import(&read, &mut report);
        // Absorbing a file puts this seat in step with it, which is exactly
        // what the stale-seat refusal tells people to do — so record it, or the
        // remedy would not clear the condition it names (req:stale-seat-knows).
        if let (Some(graph_path), Some(path), Some(hash)) =
            (self.graph_path.as_deref(), &req.path, &doc.content_hash)
        {
            reflow2_core::provenance::record_sync(
                graph_path,
                &crate::saved_design::sync_key(path),
                hash,
            );
        }
        let mut reply = serde_json::to_value(report).map_err(ser_err)?;
        crate::settles::report_unsigned(&mut reply, watch.unsigned(&g));
        ok_json(reply)
    }

    #[tool(
        description = "Compare two as-designed records — the design-vs-design sibling of the \
                       reconcile family, which only ever compares design against reality. \
                       Findings are directional relative to the named base: `added` / `removed` \
                       / `changed` (property-level), banded into design content vs the \
                       supporting layer (change events, questions, provenance). Pass base_path \
                       alone to compare the live graph against a committed export ('has this \
                       session diverged from the record?'); pass other_path too to compare two \
                       export files (branches, machines, alternatives). Reports divergence, \
                       never judges which side is right. On a server that holds many designs (a \
                       registry behind a trusted host) pass other_graph_id instead of a path: the \
                       live graph is the base and the other design is read by graph_id from the \
                       one the server holds, inside the set of designs the host says you may \
                       read; a path is not served there. Over `budget_chars` the long lists are \
                       shortened and named in `budget.lists_shortened`; read any one in full, a \
                       page at a time, with `list` and `offset`.",
        title = "Compare Two Designs",
        annotations(read_only_hint = true)
    )]
    pub async fn compare_designs(
        &self,
        Parameters(req): Parameters<CompareDesignsReq>,
    ) -> Result<CallToolResult, McpError> {
        // A divergence report is a LIST OF WHAT MOVED, not a document — so
        // unlike export_surface above, trimming its prose leaves it answering
        // the question it was asked. Every count survives; the long statements
        // and rationales are what made it unusable on two real 9-16 MB exports,
        // which is the friction this was reported as. Two unrelated designs
        // have several long id lists (measured 2026-10-11 on flo2.io: a 4,000
        // budget answered 1,421,523 characters while only the longest list was
        // shortened), so the lists give by the characters they cost, each one
        // named, and `list` + `offset` read any one of them in full.
        let budget = req
            .budget_chars
            .unwrap_or(crate::reply_budget::DEFAULT_REPLY_BUDGET_CHARS);
        let page = match req.list.as_deref() {
            Some(list) => Some((list_path(list)?, req.offset.unwrap_or(0))),
            None => {
                if req.offset.is_some() {
                    return Err(McpError::invalid_params(
                        "`offset` pages one list, so it goes with `list`: name the list to \
                         read (design.added, edges_removed, …)."
                            .to_string(),
                        None,
                    ));
                }
                None
            }
        };
        let bound = |diff: serde_json::Value| -> Result<serde_json::Value, McpError> {
            match page {
                Some((list, offset)) => page_of_diff(diff, list, offset, budget),
                None => Ok(crate::reply_budget::bound_reply_sampling(
                    diff,
                    budget,
                    "Every count is still here (`summary`). To read a shortened list in full, \
                     call compare_designs again with the same arguments plus `list` (its `path` \
                     from `lists_shortened`) and `offset` (its `shown`): the reply is that list \
                     from there under `items`, every other list as a count, and \
                     `page.next_offset` for the page after. Read one node in full with get_node.",
                )),
            }
        };
        // THE OTHER DESIGN BY GRAPH_ID
        // (`req:every-cross-design-operation-takes-its-other-designs-by-graph-id-under-the-hosts-read-filter`):
        // the live graph is the base, labelled by this design's graph_id, and
        // the other design is whatever the registry holds under that id —
        // resolved and authorised by `other_design`, the one door.
        if let Some(other_id) = req.other_graph_id.as_deref() {
            if req.other_path.is_some() {
                return Err(McpError::invalid_params(
                    "pass other_graph_id OR other_path, not both — with two other sides there \
                     is no way to say which one was compared."
                        .to_string(),
                    None,
                ));
            }
            if req.base_path.is_some() {
                return Err(McpError::invalid_params(
                    "with other_graph_id the live graph is the base, so base_path is not \
                     combined with it: compare two files with base_path and other_path, or this \
                     design against another with other_graph_id alone."
                        .to_string(),
                    None,
                ));
            }
            // THEIRS FIRST, under their guard, which `other_design` drops before
            // it returns; THEN ours. Never both at once — tokio's RwLock queues
            // a writer ahead of new readers, so A→B against B→A holding two
            // guards each would deadlock.
            let other = self.other_design(other_id).await?;
            let (live, own) = {
                let g = self.graph.read().await;
                (
                    g.export_graph().map_err(dyno_err)?,
                    g.graph_id().to_string(),
                )
            };
            return ok_json(bound(
                serde_json::to_value(reflow2_core::compare_designs(&live, &other, &own, other_id))
                    .map_err(ser_err)?,
            )?);
        }
        let Some(base_path) = req.base_path.as_deref() else {
            return Err(McpError::invalid_params(
                "nothing to compare: pass base_path (an export file, what every finding is \
                 relative to) or other_graph_id (another design this server holds)."
                    .to_string(),
                None,
            ));
        };
        // 🛑 A PATH IS NOT SERVED ON A REGISTRY SERVER. It serves designs for
        // other people under one root; a path names a file on ITS host, which
        // is not a design it serves — and until 2026-10-10 a bare registry read
        // any path it was given here (measured in
        // `tests/a_cross_design_read_takes_the_other_design_by_graph_id.rs`).
        self.refuse_a_path_on_a_registry(
            &[
                ("base_path", Some(base_path)),
                ("other_path", req.other_path.as_deref()),
            ],
            "name the design by graph_id (other_graph_id).",
        )?;
        let base = read_export_document(base_path)?;
        match &req.other_path {
            Some(other_path) => {
                let other = read_export_document(other_path)?;
                ok_json(bound(
                    serde_json::to_value(reflow2_core::compare_designs(
                        &base, &other, base_path, other_path,
                    ))
                    .map_err(ser_err)?,
                )?)
            }
            None => {
                let g = self.graph.read().await;
                ok_json(bound(
                    serde_json::to_value(g.compare_with_base(&base, base_path).map_err(dyno_err)?)
                        .map_err(ser_err)?,
                )?)
            }
        }
    }

    #[tool(
        description = "Decide whether a restructuring PRESERVED FUNCTION — compare_designs' \
                       verdict-bearing sibling. A maturity restructuring holds the function set \
                       invariant and moves everything else (allocation, packaging, which \
                       functions live in which component, which seams are declared), and it is \
                       safe exactly when function is provably unchanged. That is computable, so \
                       this CERTIFIES rather than asserts: every divergence is classified \
                       function / structure / supporting and the verdict is `preserved`, \
                       `not_preserved` or `indeterminate`. NOTHING IS WAVED THROUGH — a node \
                       type, an edge endpoint or a property edit the rules cannot place lands in \
                       `unclassified` and forces `indeterminate`, because a classifier that has \
                       not been taught part of the vocabulary must not certify a design it never \
                       examined. A reworded capability is undecidable by construction (a rename \
                       and a scope change are the same bytes) and comes back with both values \
                       for a human. `not_certified_about` is on every certificate INCLUDING a \
                       clean one: this reads two design records and has read no code, so it \
                       never claims the implementation preserved behaviour. On a registry server, \
                       give `other_graph_id` instead of a path. \
                       Ask for this when you want to know whether a restructuring or reorganisation changed what the system does or only how it is organised — proof that no function was lost.",
        title = "Certify Function Preserved",
        annotations(read_only_hint = true)
    )]
    pub async fn certify_preservation(
        &self,
        Parameters(req): Parameters<CertifyPreservationReq>,
    ) -> Result<CallToolResult, McpError> {
        // THE RESTRUCTURED DESIGN BY GRAPH_ID, exactly as compare_designs
        // takes it: the live graph is the base, labelled by this design's
        // graph_id, and the other design is resolved and authorised by
        // `other_design`, the one door — theirs exported first under their
        // guard, which is dropped before ours is read.
        if let Some(other_id) = req.other_graph_id.as_deref() {
            if req.other_path.is_some() {
                return Err(McpError::invalid_params(
                    "pass other_graph_id OR other_path, not both — with two restructured sides \
                     there is no way to say which one was certified."
                        .to_string(),
                    None,
                ));
            }
            if req.base_path.is_some() {
                return Err(McpError::invalid_params(
                    "with other_graph_id the live graph is the base, so base_path is not \
                     combined with it: certify two files with base_path and other_path, or \
                     another design against this one with other_graph_id alone."
                        .to_string(),
                    None,
                ));
            }
            let other = self.other_design(other_id).await?;
            let (live, own) = {
                let g = self.graph.read().await;
                (
                    g.export_graph().map_err(dyno_err)?,
                    g.graph_id().to_string(),
                )
            };
            let diff = reflow2_core::compare_designs(&live, &other, &own, other_id);
            return ok_json(reflow2_core::certify_preservation(&diff, &live, &other));
        }
        self.refuse_a_path_on_a_registry(
            &[
                ("base_path", req.base_path.as_deref()),
                ("other_path", req.other_path.as_deref()),
            ],
            "name the restructured design by graph_id (other_graph_id).",
        )?;
        let Some(base_path) = req.base_path.as_deref() else {
            return Err(McpError::invalid_params(
                "nothing to certify against: pass base_path (the export before the \
                 restructuring) or other_graph_id (the restructured design, another one this \
                 server holds)."
                    .to_string(),
                None,
            ));
        };
        let base = read_export_document(base_path)?;
        match &req.other_path {
            Some(other_path) => {
                let other = read_export_document(other_path)?;
                let diff = reflow2_core::compare_designs(&base, &other, base_path, other_path);
                ok_json(reflow2_core::certify_preservation(&diff, &base, &other))
            }
            None => {
                let g = self.graph.read().await;
                ok_json(
                    g.certify_preservation_against(&base, base_path)
                        .map_err(dyno_err)?,
                )
            }
        }
    }

    #[tool(
        description = "Propose a three-way merge of two divergent designs against their common \
                       ancestor — compare's write-side sibling (BL-80). Runs git's trivial-merge \
                       case table per node and per property over typed values: only one side \
                       changed → take it; both changed the same way → take it; both changed \
                       differently → a conflict, surfaced as a Question for the human, never \
                       guessed. A node one side deleted and the other changed is retained and \
                       asked (deletion must be re-justified); edges get the identical rule. Pass \
                       base_path (the ancestor — e.g. git merge-base + the committed export at \
                       that commit), ours_path (merge into) and theirs_path (merge in). This is a \
                       PROPOSAL: it writes nothing. Applying the resolved merge is a separate, \
                       explicit step. On a registry server, give `other_graph_id` (theirs) and \
                       `base_graph_id` instead of paths; ours is then this design.",
        title = "Propose a Three-Way Merge",
        annotations(read_only_hint = true)
    )]
    pub async fn merge_designs(
        &self,
        Parameters(req): Parameters<MergeDesignsReq>,
    ) -> Result<CallToolResult, McpError> {
        let paths = [
            ("base_path", req.base_path.as_deref()),
            ("ours_path", req.ours_path.as_deref()),
            ("theirs_path", req.theirs_path.as_deref()),
        ];
        // THE OTHER DESIGNS BY GRAPH_ID: theirs and the common ancestor, each
        // through `other_design`, the one door, one after the other — each
        // guard dropped before the next is taken — and ours, the live graph,
        // read last. Never two guards at once.
        if req.other_graph_id.is_some() || req.base_graph_id.is_some() {
            if let Some((argument, _)) = paths.iter().find(|(_, p)| p.is_some()) {
                return Err(McpError::invalid_params(
                    format!(
                        "name the designs by graph_id OR by path, not both: with other_graph_id \
                         and base_graph_id ours is the live graph, so `{argument}` has nothing \
                         to name."
                    ),
                    None,
                ));
            }
            let (Some(theirs_id), Some(base_id)) =
                (req.other_graph_id.as_deref(), req.base_graph_id.as_deref())
            else {
                return Err(McpError::invalid_params(
                    "a merge by graph_id names two designs: other_graph_id (theirs, merged in) \
                     and base_graph_id (the common ancestor both diverged from). Ours is the \
                     live graph."
                        .to_string(),
                    None,
                ));
            };
            let base = self.other_design_named("base_graph_id", base_id).await?;
            let theirs = self.other_design(theirs_id).await?;
            let (ours, own) = {
                let g = self.graph.read().await;
                (
                    g.export_graph().map_err(dyno_err)?,
                    g.graph_id().to_string(),
                )
            };
            return ok_json(reflow2_core::merge_designs(
                &base, &ours, &theirs, base_id, &own, theirs_id,
            ));
        }
        self.refuse_a_path_on_a_registry(
            &paths,
            "name theirs by graph_id (other_graph_id) and the common ancestor by base_graph_id; \
             ours is the live graph.",
        )?;
        let (Some(base_path), Some(ours_path), Some(theirs_path)) =
            (paths[0].1, paths[1].1, paths[2].1)
        else {
            return Err(McpError::invalid_params(
                "nothing to merge: pass base_path, ours_path and theirs_path (three export \
                 files), or other_graph_id and base_graph_id (designs this server holds; ours is \
                 the live graph)."
                    .to_string(),
                None,
            ));
        };
        let base = read_export_document(base_path)?;
        let ours = read_export_document(ours_path)?;
        let theirs = read_export_document(theirs_path)?;
        ok_json(reflow2_core::merge_designs(
            &base,
            &ours,
            &theirs,
            base_path,
            ours_path,
            theirs_path,
        ))
    }

    #[tool(
        description = "Apply a resolved three-way merge into the live design — the write side of \
                       merge_designs (BL-80). `ours` is the live graph at --graph-path; this \
                       merges `theirs` into it against the common ancestor `base`, making the live \
                       design equal the merged result, atomically. Pass `resolutions` — one \
                       decision per conflict (its `merge:…` id → base/ours/theirs, from a prior \
                       merge_designs run). It REFUSES and writes nothing if any conflict is \
                       undecided, or a decision names no conflict. This is the explicit commit the \
                       proposal is designed around: run merge_designs first, decide the conflicts, \
                       then apply.",
        title = "Apply a Merge",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    pub async fn apply_merge(
        &self,
        Parameters(req): Parameters<ApplyMergeReq>,
    ) -> Result<CallToolResult, McpError> {
        // A WRITE FROM ANOTHER DESIGN takes it by path only, for now: by
        // graph_id it would hold theirs while writing ours, which needs a
        // two-lock order this server does not have yet. On a registry the path
        // is refused, as every path is there.
        self.refuse_a_path_on_a_registry(
            &[
                ("base_path", Some(&req.base_path)),
                ("theirs_path", Some(&req.theirs_path)),
            ],
            "apply_merge does not take the other design by graph_id yet, so apply a merge on a \
             server you run yourself, where the path is yours.",
        )?;
        let base = read_export_document(&req.base_path)?;
        let theirs = read_export_document(&req.theirs_path)?;
        let mut resolutions = std::collections::BTreeMap::new();
        for (id, choice) in &req.resolutions {
            let parsed = reflow2_core::Resolution::parse(choice).ok_or_else(|| {
                dyno_err(reflow2_core::DynoError::Validation {
                    node_type: "merge".into(),
                    property: "resolutions".into(),
                    message: format!(
                        "conflict '{id}' has resolution '{choice}', which is not one of \
                         base/ours/theirs"
                    ),
                })
            })?;
            resolutions.insert(id.clone(), parsed);
        }
        let mut g = self.write_lock().await?;
        let watch = crate::settles::SettleWatch::before(
            &g,
            theirs
                .nodes
                .iter()
                .map(|n| (n.node_type.as_str(), n.node_id.as_str())),
        );
        let applied = g
            .apply_merge(&base, &theirs, &resolutions, req.use_recorded)
            .map_err(dyno_err)?;
        let mut reply = serde_json::to_value(applied).map_err(ser_err)?;
        crate::settles::report_unsigned(&mut reply, watch.unsigned(&g));
        ok_json(reply)
    }

    #[tool(
        description = "Recall recorded conflict resolutions (rerere) by their content keys — the \
                       advisory half of merge (BL-80 #5). Pass the `resolution_key`s (`rr:…`) that \
                       merge_designs put on its conflicts; returns, for each one previously \
                       resolved, the recorded decision (base/ours/theirs). Because the key is the \
                       conflict's *content* (values, not location), one recorded decision is \
                       recalled for every node with the identical conflict — resolve the shape \
                       once, then apply_merge with use_recorded, or feed these suggestions back as \
                       explicit resolutions. A suggestion, never an auto-decision. \
                       Ask for this to see how we settled the same conflict last time.",
        title = "Recall Conflict Resolutions",
        annotations(read_only_hint = true)
    )]
    pub async fn recall_resolutions(
        &self,
        Parameters(req): Parameters<RecallResolutionsReq>,
    ) -> Result<CallToolResult, McpError> {
        let g = self.graph.read().await;
        let recalled = g
            .recall_resolutions(&req.resolution_keys)
            .map_err(dyno_err)?;
        // A BARE `{}` ECHOED NOTHING — not the keys asked about, not a count,
        // so "no resolution recorded" and "I did not understand your keys"
        // shared one reply. Both are named now.
        let not_recorded: Vec<&String> = req
            .resolution_keys
            .iter()
            .filter(|k| !recalled.contains_key(*k))
            .collect();
        ok_json(serde_json::json!({
            "count": recalled.len(),
            "asked_about": req.resolution_keys.len(),
            "recalled": recalled,
            "not_recorded": not_recorded,
            "empty_because": (recalled.is_empty()).then(|| format!(
                "none of the {} key(s) asked about has a recorded resolution. A resolution is \
                 written when apply_merge settles a conflict, and the key is the conflict's \
                 CONTENT — so a key from a different conflict shape will never match.",
                req.resolution_keys.len()
            )),
        }))
    }

    #[tool(
        description = "Compare parallel design alternatives on the same measures — an analysis of \
                       alternatives (BL-70). Pass the paths to two or more alternative export \
                       documents (branch-by-file); the first is the baseline. Returns each branch's \
                       measures side by side — design nodes, open gaps, structural defects, \
                       allocation modularity, capabilities verified — plus every non-baseline \
                       branch's structural divergence from the baseline (added/removed/changed). \
                       Makes alternatives comparable on measures, not advocacy; it opens its own \
                       throwaway graphs, so it never touches and is never blocked by the live one. \
                       Collapse the winner with merge_designs/apply_merge and retire the losers. \
                       On a registry server, give `other_graph_ids` instead of paths, measured \
                       against this design. \
                       Ask for this to line candidate designs up on the same numbers.",
        title = "Analyze Alternatives",
        annotations(read_only_hint = true)
    )]
    pub async fn analyze_alternatives(
        &self,
        Parameters(req): Parameters<AnalyzeAlternativesReq>,
    ) -> Result<CallToolResult, McpError> {
        // THE ALTERNATIVES BY GRAPH_ID: the live graph is the baseline,
        // labelled by this design's graph_id, and each alternative comes
        // through `other_design`, the one door, one at a time — each guard
        // dropped before the next is taken, ours read last.
        if let Some(ids) = req.other_graph_ids.as_deref() {
            if req.paths.is_some() {
                return Err(McpError::invalid_params(
                    "pass other_graph_ids OR paths, not both — the baseline is the live graph by \
                     graph_id and the first file by path, and one comparison has one baseline."
                        .to_string(),
                    None,
                ));
            }
            if ids.is_empty() {
                return Err(McpError::invalid_params(
                    "other_graph_ids is empty: name at least one alternative design by graph_id \
                     to measure against this one."
                        .to_string(),
                    None,
                ));
            }
            let mut others = Vec::with_capacity(ids.len());
            for id in ids {
                others.push((
                    id.clone(),
                    self.other_design_named("other_graph_ids", id).await?,
                ));
            }
            let baseline = {
                let g = self.graph.read().await;
                (
                    g.graph_id().to_string(),
                    g.export_graph().map_err(dyno_err)?,
                )
            };
            let mut alternatives = Vec::with_capacity(others.len() + 1);
            alternatives.push(baseline);
            alternatives.extend(others);
            return ok_json(reflow2_core::analyze_alternatives(&alternatives).map_err(dyno_err)?);
        }
        let Some(paths) = req.paths.as_deref() else {
            return Err(McpError::invalid_params(
                "nothing to compare: pass paths (two or more export files; the first is the \
                 baseline) or other_graph_ids (designs this server holds; the live graph is the \
                 baseline)."
                    .to_string(),
                None,
            ));
        };
        let named: Vec<(&str, Option<&str>)> =
            paths.iter().map(|p| ("paths", Some(p.as_str()))).collect();
        self.refuse_a_path_on_a_registry(
            &named,
            "name the alternatives by graph_id (other_graph_ids); the live graph is the baseline.",
        )?;
        let mut alternatives = Vec::with_capacity(paths.len());
        for p in paths {
            alternatives.push((p.clone(), read_export_document(p)?));
        }
        ok_json(reflow2_core::analyze_alternatives(&alternatives).map_err(dyno_err)?)
    }

    #[tool(
        description = "Set a Decision's lifecycle status — proposed / accepted / deferred / \
                       contested / superseded / rejected (BL-70). Setting it to `proposed` opens it \
                       as a decision point: an undecided fork you can register alternatives under. \
                       `deferred` SETS IT ASIDE — sound but untimely — and it stops counting as debt \
                       in loop_status and what_next; it is the owner's word and wants `approver`, \
                       like `accepted`. `contested` KEEPS IT OPEN ON PURPOSE: both sides kept, no \
                       verdict owed, also the owner's word and no debt. \
                       Every other property is preserved. CARRIES `settled_question_prose` WHEN A DECISION REACHES \
                       `accepted` AND SOMETHING IT GOVERNS STILL SAYS THE QUESTION IS OPEN: the \
                       block names each such node, the phrase that matched, and quotes the prose \
                       around it, so you can judge it here rather than in another call. It never \
                       says the text is wrong — prose that QUOTES an old question is a correct \
                       node and looks identical. Silent on `rejected`/`superseded`, which retire \
                       rather than settle, and on `accepted` -> `accepted`. From a 2026-09-09 \
                       incident where a requirement said \"NOT YET DECIDED … his call to make\" \
                       three weeks after the call had been made, and the owner was asked the \
                       settled question a second time. \
                       PASS `chose` TO SAY WHICH OPTION WON, in your own words: the settling act \
                       then records what it settled without rewriting the deliberation that \
                       produced it. PASS `name` TO RETITLE IT in the same call. \
                       Ask for this to mark a decision accepted with the owner's name.",
        title = "Set Decision Status",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub async fn set_decision_status(
        &self,
        Parameters(req): Parameters<SetDecisionStatusReq>,
    ) -> Result<CallToolResult, McpError> {
        let mut g = self.write_lock().await?;
        crate::tools::capture::approver_must_exist(
            &g,
            req.approver.as_deref(),
            "set_decision_status",
        )?;
        // WHOSE WORD IS THIS? Declared in crate::settles: `accepted` and
        // `deferred` are the owner's act — settling a question and setting one
        // aside — while superseding or rejecting retires rather than decides.
        let owners_word =
            crate::settles::rule("set_decision_status").settles_str(Some(req.status.as_str()));
        let note = crate::settles::gate(
            "set_decision_status",
            owners_word,
            req.approver.as_deref(),
            "a Decision status",
        )?;
        // A new name rides the settle, so the heading that asked the question
        // can stop asking it in the act that answered it. An empty one is
        // refused BEFORE anything is written: a nameless decision is a worse
        // record than a stale name.
        if let Some(n) = req.name.as_deref()
            && n.trim().is_empty()
        {
            return Err(McpError::invalid_params(
                "`set_decision_status`: `name` was passed empty. Pass the decision's new \
                 heading, or leave `name` out to keep the one it has. The status was NOT \
                 moved — nothing about this call was written."
                    .to_string(),
                None,
            ));
        }
        // Read BEFORE the write: afterwards there is nothing left to compare
        // against, and `accepted` -> `accepted` must stay silent.
        let was_accepted = crate::tools::capture::prior_status(
            &g,
            reflow2_core::nodes::node::DECISION,
            &req.decision_id,
        )
        .as_deref()
            == Some("accepted");
        let node = NodeDto::from(
            g.set_decision_status(&req.decision_id, &req.status)
                .map_err(dyno_err)?,
        );
        crate::tools::capture::sign_as_approver(
            &mut g,
            reflow2_core::nodes::node::DECISION,
            &req.decision_id,
            req.approver.as_deref(),
            req.acted_at.as_deref(),
        )?;
        // What the settlement CHOSE, when the settler said so. Written only
        // when supplied, so a status move that names no outcome is unchanged
        // and absence keeps meaning nobody said.
        let node = crate::tools::capture::set_optional_props(
            &mut g,
            reflow2_core::nodes::node::DECISION,
            &req.decision_id,
            &[
                ("chose", req.chose.as_deref()),
                ("name", req.name.as_deref().map(str::trim)),
            ],
        )?
        .map_or(node, NodeDto::from);
        // TWO DIFFERENT QUESTIONS, and they used to share one flag. Whose word
        // this is was answered above, from the declaration. DID A QUESTION JUST
        // CLOSE? Only `accepted` closes one. A deferred decision's governed
        // prose saying "still open" is TRUE, so the settled-question check
        // below must not fire for it.
        let settles = req.status == "accepted";
        // A question just closed. Anything this decision governs whose prose
        // still says it is open is now a paragraph the next reader will believe
        // over the decision — the 2026-09-09 incident, fired at the moment the
        // divergence is created rather than found by a sweep later.
        let hits = if settles && !was_accepted {
            let mut hits = crate::tools::capture::governed_open_prose(&g, &req.decision_id);
            // AND THE DECISION'S OWN TEXT (item 15 of req:small-read-back-
            // faults-from-the-cogito-trial-are-fixed): "PROPOSED…, AWAITING THE
            // OWNER", written while it was proposed, reads wrong the moment it
            // is accepted, and only governed nodes were read.
            if let Ok(Some(own)) = g.get_node(reflow2_core::nodes::node::DECISION, &req.decision_id)
                && let Some(hit) = crate::prose_currency::open_prose(
                    &req.decision_id,
                    reflow2_core::nodes::node::DECISION,
                    |f| own.properties.get(f).and_then(|v| v.as_str()),
                )
            {
                hits.push(hit);
            }
            hits
        } else {
            Vec::new()
        };
        crate::tools::capture::with_approval_and_settled_question(
            node,
            note,
            &req.decision_id,
            &hits,
        )
    }

    #[tool(
        description = "WHICH NODES MIGHT THIS ONE RELATE TO \u{2014} offered, never drawn. The missing \
                       half of half-idea linking: `unreviewed_ideas` counts the ideas connected to \
                       nothing, `review_relations` records the judgement, and until now NOTHING \
                       answered the question a person working that backlog actually has \u{2014} which of \
                       these belong together? \u{2b50} EVERY CANDIDATE CARRIES THE WALK THAT PRODUCED IT \
                       in `because`: a shared neighbour (two nodes that both relate to a third are \
                       related in the graph\u{2019}s own terms) or distinctive shared terms, each \
                       weighted by rarity ACROSS THE POOL: one true of nearly everything (the \
                       author, the project) counts for nothing. With no `pool_type`, \
                       intent is ranked against requirements AND decisions. A candidate whose reason cannot be stated is not offered. \
                       \u{1f6d1} IT NEVER WRITES AND NEVER PROPOSES AN EDGE. Ranking is the machine\u{2019}s half; \
                       drawing is yours, through `review_relations`, and a false neighbour is worse \
                       than a missing one because anything searching by neighbourhood repeats it \
                       forever. Nodes ALREADY related are excluded from the ranking and returned in \
                       `already_related`, so \u{2018}not offered because already linked\u{2019} is distinguishable \
                       from \u{2018}not offered because nothing matched\u{2019}. \u{26a0} AN EMPTY ANSWER SAYS WHICH \
                       EMPTY IT IS \u{2014} `empty_because` separates \u{2018}nothing to compare against\u{2019}, \u{2018}this \
                       node carries no comparable text\u{2019} and \u{2018}ranked N and none matched\u{2019}, because only \
                       the last means the node may be genuinely new. Needs no search index and no \
                       database, so it works in an index-less build.",
        title = "Relation Candidates",
        annotations(read_only_hint = true)
    )]
    pub async fn relation_candidates(
        &self,
        Parameters(req): Parameters<RelationCandidatesReq>,
    ) -> Result<CallToolResult, McpError> {
        let g = self.graph.read().await;
        let node_type = crate::service::resolve_node_type(
            &g,
            req.node_type.as_deref(),
            &req.node_id,
            "node_type",
        )?;
        ok_json(
            g.relation_candidates(
                &node_type,
                &req.node_id,
                req.pool_type.as_deref(),
                req.limit.unwrap_or(5),
            )
            .map_err(dyno_err)?,
        )
    }

    #[tool(
        description = "Declare that this Decision states the quality attribute the design is \
                       BUILT FOR — the answer to \"what is this system for?\". A TARGET, not a \
                       score: DimensionAssessment records what a system IS on an axis, this \
                       records what it is aiming at. ⭐ IT MATTERS BECAUSE THE ATTRIBUTE DECIDES \
                       WHICH GROUPING IS RIGHT, and the four disagree — performance wants least \
                       chatter across boundaries, reliability wants no articulation point and may \
                       deliberately DUPLICATE a function across parts, maintainability wants \
                       what-changes-together-to-live-together, security wants boundaries \
                       following trust rather than coupling. Allocating without this answer \
                       silently picks performance. ASK IT EARLY: asking is cheap and shapes \
                       everything downstream, while the cost of asking late is reworking \
                       services already built — which is why genesis asks it and allocation, \
                       correctly, still waits for the last responsible moment. 🛑 THERE IS NO \
                       `none`: absence means nobody was asked, a different fact from a design \
                       that weighed it and chose, and `quality_target_unstated` reads exactly \
                       that difference. Every other property is preserved; link what the choice \
                       shapes with governed_by.",
        title = "Set a Quality Target",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub async fn set_quality_target(
        &self,
        Parameters(req): Parameters<SetQualityTargetReq>,
    ) -> Result<CallToolResult, McpError> {
        let mut g = self.write_lock().await?;
        ok_json(NodeDto::from(
            g.set_quality_target(&req.decision_id, &req.quality_target)
                .map_err(dyno_err)?,
        ))
    }

    #[tool(
        description = "Register an alternative under a proposed decision point (BL-70): a \
                       lightweight Artifact pointer that names where the alternative's design \
                       export lives (branch-by-file), GOVERNED_BY the Decision and CONTRADICTS its \
                       siblings. Refuses unless the Decision is `proposed` — you fork an open \
                       choice, not a settled one. Compare the registered alternatives with \
                       analyze_alternatives, then collapse_decision to choose.",
        title = "Register an Alternative",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub async fn register_alternative(
        &self,
        Parameters(req): Parameters<RegisterAlternativeReq>,
    ) -> Result<CallToolResult, McpError> {
        let mut g = self.write_lock().await?;
        ok_json(
            g.register_alternative(&req.decision_id, &req.artifact_id, &req.name, &req.location)
                .map_err(dyno_err)?,
        )
    }

    #[tool(
        description = "List the alternatives registered under a decision point (BL-70) — the Artifact pointers GOVERNED_BY the Decision, each with its export location, ready to feed to analyze_alternatives. AN EMPTY LIST SAYS WHICH EMPTY: the reply distinguishes a decision that exists but has no alternatives registered, a decision that is not `proposed` (only an open decision point can carry forks), and an id that names no Decision at all — three different facts that used to share one bare zero. Ask for this when you want to see the roads recorded under an open choice before comparing them. \
                       Ask for this to see which candidate designs we did register against a decision.",
        title = "Alternatives for a Decision",
        annotations(read_only_hint = true)
    )]
    pub async fn alternatives_for(
        &self,
        Parameters(req): Parameters<AlternativesForReq>,
    ) -> Result<CallToolResult, McpError> {
        let g = self.graph.read().await;
        let exists = g
            .get_node(reflow2_core::nodes::node::DECISION, &req.decision_id)
            .map_err(dyno_err)?
            .is_some();
        if !exists {
            return Err(McpError::invalid_params(
                format!(
                    "no Decision {:?} in this design, so it can hold no alternatives. An empty \
                     list would read as \"this decision point has none\", which is a different \
                     fact.",
                    req.decision_id
                ),
                None,
            ));
        }
        ok_json_or_why(
            g.alternatives_for(&req.decision_id).map_err(dyno_err)?,
            "this Decision exists and no Artifact is GOVERNED_BY it, so no alternative has been \
             registered under it (register_alternative does that) — the decision point is real \
             and unforked",
        )
    }

    #[tool(
        description = "Collapse a decision point (BL-70): choose the winning alternative. The \
                       Decision moves to `accepted`, the losing alternatives are superseded \
                       (OBSOLETES — retired on the record, not deleted), and the outcome is \
                       written into the Decision's own `alternatives` field with the rationale. \
                       This records the choice; merge the winner's design content into the \
                       baseline separately with apply_merge. \
                       Ask for this to pick the winner and retire the others.",
        title = "Collapse a Decision Point",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub async fn collapse_decision(
        &self,
        Parameters(req): Parameters<CollapseDecisionReq>,
    ) -> Result<CallToolResult, McpError> {
        let mut g = self.write_lock().await?;
        crate::tools::capture::approver_must_exist(
            &g,
            req.approver.as_deref(),
            "collapse_decision",
        )?;
        // Choosing a winner moves the Decision to `accepted` — settled intent,
        // declared in crate::settles like every other settle path. Until
        // 2026-09-29 this path took no approver and said nothing
        // (fact:collapse-decision-settles-a-decision-with-no-approver-and-says-nothing-2026-09-28).
        let settles = crate::settles::rule("collapse_decision").settles_value(None);
        let note = crate::settles::gate(
            "collapse_decision",
            settles,
            req.approver.as_deref(),
            "a Decision accepted by choosing its winner",
        )?;
        let report = g
            .collapse_decision(&req.decision_id, &req.winner_id, req.note.as_deref())
            .map_err(dyno_err)?;
        crate::tools::capture::sign_as_approver(
            &mut g,
            reflow2_core::nodes::node::DECISION,
            &req.decision_id,
            req.approver.as_deref(),
            req.acted_at.as_deref(),
        )?;
        let mut out = serde_json::to_value(report).map_err(crate::service::ser_err)?;
        if let (Some(note), Some(obj)) = (note, out.as_object_mut()) {
            obj.insert("carries_nobodys_name".into(), JsonValue::String(note));
        }
        ok_json(out)
    }

    #[tool(
        description = "Analyse THIS design together with other designs — a dependency, a partner system, a hub's members — and report the problems at the join: what only shows up when they are present together. It imports theirs beside yours and runs reflow2's ORDINARY checks over the whole, so a seam problem arrives as the gap it already is: a contract with no provider, a requirement nothing satisfies across the join. NOTHING IS WRITTEN: the combined graph is built in memory and thrown away, and each design's ids are namespaced, so none overwrites another. ONE other design: `design` (an export, with `namespace`), or `other_graph_id` on a registry server behind a trusted host; findings come back OURS / THEIRS / SEAM. SEVERAL at once, for an effort across a hub's designs: `other_graph_ids`, or `tracked_designs: true` for every design this one pins (through a hub's address, every member). Served alone (the --call door) each is read from its committed export, on a registry from its live store; findings come back `by_design`, `by_pair` ('a×b') and `across`, and each design not read is named in `not_composed`. Run it over a hub before changing what several of its designs share. Where they touch is read from their records on EXACT evidence only (the same file, or a pin naming the other) and drawn as a `computed` link the checks see across; a similar name is only a `candidate` to confirm. ⚠ `link_status` says, per pair: linked, candidates_only, never_touch or not_checked, never that they agree.",
        title = "Compose and Analyse Designs Together",
        annotations(read_only_hint = true)
    )]
    pub async fn compose_and_analyse(
        &self,
        Parameters(req): Parameters<ComposeReq>,
    ) -> Result<CallToolResult, McpError> {
        // SEVERAL DESIGNS AT ONCE (`other_graph_ids`, `tracked_designs`):
        // `service::composed`. Every other call is the one-other-design path
        // below, byte for byte.
        if crate::service::composed_asks_for_several(&req) {
            return self.compose_several(req).await;
        }
        let budget = req.budget_chars;
        // ONE OTHER DESIGN, from exactly one of two sources: inline, as always,
        // or BY GRAPH_ID through the registry this design is served from
        // (`req:every-cross-design-operation-takes-its-other-designs-by-graph-id-under-the-hosts-read-filter`),
        // resolved and authorised by `other_design`, the one door. Theirs is
        // exported first (that guard is dropped inside), ours read after.
        let (doc, namespace): (reflow2_core::GraphExport, String) =
            match (req.design, req.other_graph_id.as_deref()) {
                (Some(_), Some(_)) => {
                    return Err(McpError::invalid_params(
                        "pass design OR other_graph_id, not both — with two other designs there \
                         is no way to say which one was composed in."
                            .to_string(),
                        None,
                    ));
                }
                (None, None) => {
                    return Err(McpError::invalid_params(
                        "nothing to compose: pass design (an export document) or other_graph_id \
                         (another design this server holds)."
                            .to_string(),
                        None,
                    ));
                }
                (Some(design), None) => {
                    let Some(namespace) = req.namespace else {
                        return Err(McpError::invalid_params(
                            "a namespace is required with an inline design: without it the two \
                             designs' ids would collide, which is the whole reason this is not \
                             import_graph. (With other_graph_id it defaults to that graph_id.)"
                                .to_string(),
                            None,
                        ));
                    };
                    let doc = serde_json::from_value(JsonValue::Object(design)).map_err(|e| {
                        McpError::invalid_params(format!("invalid design document: {e}"), None)
                    })?;
                    (doc, namespace)
                }
                (None, Some(other_id)) => {
                    let doc = self.other_design(other_id).await?;
                    (doc, req.namespace.unwrap_or_else(|| other_id.to_string()))
                }
            };
        let g = self.graph.read().await;
        let report = g.compose_and_analyse(&doc, &namespace).map_err(dyno_err)?;
        drop(g);
        match budget {
            None => ok_json(report),
            Some(budget) => ok_json(crate::service::composed::bound_composed(
                serde_json::to_value(report).map_err(crate::service::ser_err)?,
                budget,
                "Every count is still here. A shortened finding list is a sample: raise \
                 `budget_chars` if this client has the room, or open one finding's id with \
                 get_node.",
                &[
                    "seam_findings",
                    "our_findings",
                    "their_findings",
                    "skipped_edges",
                    "declared_links",
                    "computed_links",
                    "candidates",
                ],
                &[],
                &["seam_findings", "our_findings", "their_findings"],
            )),
        }
    }
}

/// The lists a divergence report holds, by the path `compare_designs`'s `list`
/// takes — published as that argument's enum, and the same spelling
/// `budget.lists_shortened` uses to name one.
use crate::enum_schema::DIFF_LISTS;

/// `list` as one of the diff's list paths, or a refusal naming them all.
fn list_path(list: &str) -> Result<&'static str, McpError> {
    DIFF_LISTS
        .iter()
        .copied()
        .find(|p| *p == list)
        .ok_or_else(|| {
            McpError::invalid_params(
                format!(
                    "`list` must name one of the diff's lists — {} — not {list:?}.",
                    DIFF_LISTS.join(", ")
                ),
                None,
            )
        })
}

/// One page of one of the diff's lists, bounded: `items` holds `list` from
/// `offset` for as far as the budget reaches, `counts` holds every list's
/// length, and `page` says where this page ends and whether another follows.
///
/// The budget shortens `items` — the only list on the page — and `page` is
/// settled AFTER that, from what the page actually holds, so `next_offset` is
/// always the entry the shortened page stopped before. The fields are written
/// first at their widest (every entry shown, `complete: false`) so settling
/// them never grows the reply past the budget it was bounded to.
fn page_of_diff(
    mut diff: serde_json::Value,
    list: &'static str,
    offset: usize,
    budget: usize,
) -> Result<serde_json::Value, McpError> {
    let pointer = |path: &str| format!("/{}", path.replace('.', "/"));
    let mut counts = serde_json::Map::new();
    for path in DIFF_LISTS.iter().copied() {
        let n = diff
            .pointer(&pointer(path))
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len);
        counts.insert(path.to_string(), serde_json::Value::from(n));
    }
    let items: Vec<serde_json::Value> = diff
        .pointer_mut(&pointer(list))
        .and_then(|v| v.as_array_mut())
        .map(std::mem::take)
        .unwrap_or_default();
    let total = items.len();
    if offset > total {
        return Err(McpError::invalid_params(
            format!(
                "`offset` {offset} is past the end of `{list}`, which holds {total} entries; a \
                 page starts at most at {total}."
            ),
            None,
        ));
    }
    let rest: Vec<serde_json::Value> = items.into_iter().skip(offset).collect();
    let mut out = serde_json::Map::new();
    for key in ["base", "other", "ancestry", "summary", "provenance_note"] {
        if let Some(v) = diff.get(key) {
            out.insert(key.to_string(), v.clone());
        }
    }
    out.insert(
        "page".to_string(),
        json!({
            "list": list,
            "offset": offset,
            "total": total,
            "shown": rest.len(),
            "next_offset": total,
            "complete": false,
        }),
    );
    out.insert("items".to_string(), serde_json::Value::Array(rest));
    out.insert("counts".to_string(), serde_json::Value::Object(counts));
    let mut bounded = crate::reply_budget::bound_reply_sampling(
        serde_json::Value::Object(out),
        budget,
        "This is one page of one list: `page.next_offset` is where the next page starts — pass \
         it as `offset` with the same arguments while `page.complete` is false. `counts` holds \
         every list's length.",
    );
    let shown = bounded["items"].as_array().map_or(0, Vec::len);
    let next = offset + shown;
    bounded["page"]["shown"] = serde_json::Value::from(shown);
    bounded["page"]["next_offset"] = serde_json::Value::from(next);
    bounded["page"]["complete"] = serde_json::Value::Bool(next >= total);
    // The block measured the page before `page` was settled; it only shrank.
    if bounded.get("budget").is_some() {
        for _ in 0..3 {
            let now = crate::reply_budget::chars_of(&bounded);
            if bounded["budget"]["reply_chars"] == now {
                break;
            }
            bounded["budget"]["reply_chars"] = serde_json::Value::from(now);
        }
    }
    Ok(bounded)
}
