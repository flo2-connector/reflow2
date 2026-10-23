//! `query` tools — one slice of the MCP surface.
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

/// The output contract for `search_design` — **the one declared schema on the
/// whole surface, and it is an EXPERIMENT rather than a policy.**
///
/// # What it is for
///
/// Alex reported (2026-08-29) that his agent sees only the `content` stub for
/// almost every tool, while his client negotiated MCP `2025-11-25` — the same
/// revision `claude-code` reads `structuredContent` from without difficulty.
/// That killed the leading explanation (gate the duplication on protocol
/// version) and left a candidate nobody had considered: **reflow2 returns
/// `structuredContent` everywhere and declared `outputSchema` nowhere**, and
/// MCP pairs the two. A client that surfaces a structured payload only for
/// tools advertising an output contract would behave exactly as he describes.
///
/// So exactly ONE tool declares one, and every other tool is the control. If
/// his next session reads `search_design` and still not `get_node`, the cause
/// is found and the repair is to declare schemas. If it reads neither, the
/// candidate is refuted and the client ignores `structuredContent` outright.
/// `tests/one_tool_declares_its_output_contract.rs` keeps both arms honest.
///
/// 🛑 DO NOT BROADEN THIS BEFORE THE RESULT IS READ. Declaring schemas on more
/// tools destroys the control arm and the question goes unanswered — which is
/// the whole cost, because nothing else can distinguish the two outcomes.
///
/// # Why declaring it cannot break anything
///
/// rmcp 3.1.2 advertises `outputSchema` in `tools/list` and never validates an
/// outgoing payload against it — checked in the SDK source, not assumed. A
/// wrong schema would misinform a client; it cannot fail a call.
///
/// # Why `additionalProperties` is deliberately absent
///
/// `ok_read` may add `loop_hint`, and `envelope` passes objects through
/// unchanged. Closing the object would make this schema wrong the moment the
/// envelope grows a field, and a wrong contract is worse than a partial one.
fn search_design_output_schema() -> Arc<rmcp::model::JsonObject> {
    let schema = json!({
        "type": "object",
        "description": "A ranked list of design nodes matching the query, with the bounds of \
                        the search stated so a caller can tell a complete answer from a \
                        truncated one.",
        "properties": {
            "hits": {
                "type": "array",
                "description": "Matching nodes, best first. Scores are comparable WITHIN one \
                                result list and not across queries.",
                "items": {
                    "type": "object",
                    "properties": {
                        "node_id": { "type": "string" },
                        "node_type": { "type": "string" },
                        "name": {
                            "type": "string",
                            "description": "The node's name at hit time; empty if it has none."
                        },
                        "score": { "type": "number", "description": "BM25 relevance." },
                        "design": {
                            "type": "string",
                            "description": "Present on a search over several designs \
                                            (`tracked_designs`, or a hub's address): the \
                                            graph_id of the design this hit is in."
                        },
                        "age_days": {
                            "type": "integer",
                            "description": "Present only on a node carrying a date — how old \
                                            its claim is. Absent on an undated node."
                        },
                        "as_of": {
                            "type": "string",
                            "description": "The date the claim was made, when it carries one."
                        },
                        "linked": {
                            "type": "array",
                            "description": "Records directly linked to this hit by a review \
                                            relation or DECOMPOSES, at most 8. LINKED, not \
                                            matched: they need not contain the query. Absent \
                                            when nothing is linked.",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "node_id": { "type": "string" },
                                    "relation": { "type": "string" },
                                    "direction": {
                                        "type": "string",
                                        "description": "`out`: hit RELATION linked. `in`: \
                                                        linked RELATION hit."
                                    }
                                },
                                "required": ["node_id", "relation", "direction"]
                            }
                        }
                    },
                    "required": ["node_id", "node_type", "name", "score"]
                }
            },
            "stale": {
                "type": "array",
                "description": "Ids the index returned but the store no longer holds. NON-EMPTY \
                                MEANS THE INDEX HAS DRIFTED and a reindex is due — it is not a \
                                list of results.",
                "items": { "type": "string" }
            },
            "limit": {
                "type": "integer",
                "description": "The limit that bounded this result. hits.len() == limit means \
                                there may be more; this is the no-silent-caps rule made visible."
            },
            "searched": {
                "type": "integer",
                "description": "How many of the design's nodes the search ran over (the \
                                search index's own count). It says WHICH empty an empty \
                                `hits` is: nothing matched among this many."
            },
            "not_searched": {
                "type": "array",
                "description": "Present only on a design that pins other designs \
                                (external_dependency with a graph_id): each pinned design \
                                this search did NOT look in, with where it could be read \
                                and why it was not searched. A hit missing here says \
                                nothing about them.",
                "items": {
                    "type": "object",
                    "properties": {
                        "design": { "type": "string", "description": "The pinned design's graph_id." },
                        "name": { "type": "string" },
                        "dependency": { "type": "string", "description": "The pin's id here." },
                        "read_at": {
                            "type": "object",
                            "description": "`export` (a path) or `address`; empty when the pin names neither."
                        },
                        "state": {
                            "type": "string",
                            "description": "With `tracked_designs`: why it was not searched, \
                                            in one word — `missing`, `unreadable`, \
                                            `not_reached` (at an address), `nowhere`, \
                                            `not_read` (past the read cap) or `not_followed` \
                                            (pinned by a pinned design)."
                        },
                        "why": { "type": "string" }
                    },
                    "required": ["design", "dependency", "why"]
                }
            },
            "not_searched_note": {
                "type": "string",
                "description": "Beside `not_searched`: how many pinned designs were not looked in."
            },
            "designs_searched": {
                "type": "array",
                "description": "On a search over several designs: the graph_id of each design \
                                searched, this one first. `found_by_design`, \
                                `searched_by_design` and `hits_withheld` give each one's count \
                                and what the one `limit` left out; with `tracked_designs`, \
                                `read_from` names the export each pinned design was read from.",
                "items": { "type": "string" }
            },
            "tracked_note": {
                "type": "string",
                "description": "With `tracked_designs`, when it added nothing: why."
            },
            "loop_hint": {
                "type": "string",
                "description": "Present only when the coherence loop is owed something."
            }
        },
        "required": ["hits", "stale", "limit", "searched"]
    });
    Arc::new(
        schema
            .as_object()
            .expect("the literal above is an object")
            .clone(),
    )
}

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

#[tool_router(router = query_router, vis = "pub")]
impl ReflowService {
    // ---- Generic CRUD (deterministic) ----

    #[tool(
        description = "Create a node of any schema type with a property object. An existing id MERGES: the props you pass overwrite, every stored property you omit survives — so a partial props object edits, it does not reset the rest to defaults. A `null` UNSETS a property: the key is removed and the reply lists it under `unset`; a REQUIRED property refuses instead (a null used to be stored as a value that readers read two ways, 2026-09-07). READ `undeclared` IN THE REPLY: it names any property you sent that the schema does not declare for this type. The write still SUCCEEDS — the store is a property bag on purpose, so a design can record what reflow2 never anticipated — but a typo and a deliberate extension used to be indistinguishable, and this is how you tell them apart. Absent when there is nothing to say. ⚠️ EDITING SOMETHING YOU READ? PASS `expected_content_hash` — the `revision.prior_content_hash` from when you read it. The write then becomes a COMPARE-AND-SWAP and is REFUSED if the node moved in between, naming both hashes, instead of silently overwriting whoever wrote it meanwhile. Without it a shared graph loses updates by luck: measured from both sides of one real collision, the write returned a normal success and THE WINNER WAS NEVER TOLD. The `revision` block reports an overwrite AFTER the fact; this prevents it. Opt-in, because a caller who never read the node has no honest expectation to state. \
                       Ask for this when you want to add an item of a type no specific tool covers.",
        title = "Create a Node",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub async fn create_node(
        &self,
        Parameters(req): Parameters<CreateNodeReq>,
    ) -> Result<CallToolResult, McpError> {
        // A `null` is an UNSET request, not a value: split it out before the
        // bag is parsed, applied after the merge, reported in the reply.
        let (props, unset) = crate::service::split_null_props(req.props);
        let props = parse_props(props)?;
        let mut g = self.write_lock().await?;
        // Computed BEFORE the write, from what the caller actually sent: an
        // upsert merges, so reading the stored node afterwards would also
        // report undeclared properties somebody else wrote earlier, and blame
        // this caller for them.
        let undeclared = g.undeclared_properties(&req.node_type, &props);
        // Read BEFORE the write so the `revision` block can report what moved —
        // and, since 2026-08-17, so it can hand back the `prior_content_hash`
        // that `expected_content_hash` needs.
        //
        // ⚠️ THIS TOOL CARRIED NO `revision` BLOCK AT ALL until the surface
        // probe went looking for it: the block was attached by the TYPED
        // constructors and never by generic `create_node`. So the
        // compare-and-swap shipped, for one commit, with its precondition
        // value unobtainable from the very tool that demanded it. The core
        // tests all passed; only driving the real surface found it, which is
        // AGENTS.md's "compiling is not the finish line" earning its place
        // again.
        let prior = g.get_node(&req.node_type, &req.id).ok().flatten();
        // THE OWNER'S WORD, AT THE GENERIC DOOR TOO (2026-09-29). A settling
        // value inside `props` is held to the same rule as the typed
        // constructors, asked of the core's one table, BEFORE anything is
        // written — so a refusal leaves no half-written node.
        crate::tools::capture::approver_must_exist(&g, req.approver.as_deref(), "create_node")?;
        if let Some(settle) =
            generic_settle(&req.node_type, &req.id, prior.as_ref(), &props, &unset)
        {
            crate::settles::gate(
                "create_node",
                true,
                req.approver.as_deref(),
                &format!("a settle ({})", settle.describe()),
            )?;
        }
        // COMPARE-AND-SWAP when the caller stated what they read, plain upsert
        // when they did not. The refusal is the point: `revision` already told
        // the LOSER of a collision afterwards and told the winner nothing, and
        // reporting a lost update is not the same as not losing it
        // (req:a-write-cannot-silently-lose-someone-elses-work).
        let written = match req.expected_content_hash.as_deref() {
            Some(expected) => g.upsert_node_if_unchanged(&req.node_type, &req.id, props, expected),
            None => g.upsert_node(&req.node_type, &req.id, props),
        };
        let written = match written {
            Ok(_) if !unset.is_empty() => g
                .remove_properties(&req.node_type, &req.id, &unset)
                .map(|n| (n, unset)),
            Ok(n) => Ok((n, Vec::new())),
            Err(e) => Err(e),
        };
        match written {
            Ok((n, unset)) => {
                if reflow2_core::intent::settles_intent(&req.node_type, &n.properties) {
                    crate::tools::capture::sign_as_approver(
                        &mut g,
                        &req.node_type,
                        &req.id,
                        req.approver.as_deref(),
                        req.acted_at.as_deref(),
                    )?;
                }
                let mut dto = NodeDto::from(n);
                dto.undeclared = undeclared;
                dto.unset = unset;
                // The generic write path gets the same protection as the
                // typed constructors, and arguably needs it more: this is
                // where a node type with no constructor of its own gets
                // edited, and where the 2026-09-01 loss that prompted the
                // work actually happened.
                crate::tools::capture::preserve_prior(&mut g, prior.as_ref(), &dto);
                let revision = crate::tools::capture::revision_of(&g, prior.as_ref(), &dto);
                crate::tools::capture::with_capture_notes(
                    dto,
                    "loop: a generic write is still a design change — run detect_gaps \
                     (detect-and-ask) when the batch lands",
                    None,
                    revision,
                    // `create_node` is OUT OF SCOPE by design, not by omission.
                    // Its `props` is one JSON bag declaring no prose
                    // parameters, so "was the named parameter supplied?" has no
                    // meaning — and that scoping is what took the check from 2
                    // false positives to 0. Measured 2026-08-31: zero instances
                    // of the damage shape from any writer across 3,528 nodes,
                    // and this path already carries `undeclared` plus a JSON
                    // parse that refuses structural corruption loudly. Option C
                    // (a CI scan over the export) is the home for this case.
                    None,
                )
            }
            Err(e) => Err(node_error(&g, &req.node_type, e)),
        }
    }

    #[tool(
        description = "Create or update MANY nodes in one call — the bulk form of create_node. ALL OF IT OR \
                       NONE OF IT: every item is attempted so you learn every failure in one round trip, and if \
                       anything failed nothing is written. Upsert, like create_node, so re-running after a fix \
                       is safe. Ask for this when you want to add many items at once — several nodes in one \
                       call, a bulk create.",
        title = "Create Many Nodes",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub async fn create_nodes(
        &self,
        Parameters(req): Parameters<CreateNodesReq>,
    ) -> Result<CallToolResult, McpError> {
        let mut specs = Vec::with_capacity(req.nodes.len());
        let mut signers = Vec::with_capacity(req.nodes.len());
        for n in req.nodes {
            signers.push((n.approver, n.acted_at));
            specs.push(BulkNodeSpec {
                node_type: n.node_type,
                id: n.id,
                props: parse_props(n.props)?,
            });
        }
        let mut g = self.write_lock().await?;
        // Each item is held to the rule create_node is held to, and the batch's
        // own all-or-nothing shape: every unsigned settle is named at once and
        // NOTHING is written (2026-09-29).
        let mut refusals = Vec::new();
        for (i, (spec, (approver, _))) in specs.iter().zip(&signers).enumerate() {
            if let Err(e) =
                crate::tools::capture::approver_must_exist(&g, approver.as_deref(), "create_nodes")
            {
                refusals.push(format!("nodes[{i}]: {}", e.message));
                continue;
            }
            let prior = g.get_node(&spec.node_type, &spec.id).ok().flatten();
            if let Some(settle) =
                generic_settle(&spec.node_type, &spec.id, prior.as_ref(), &spec.props, &[])
                && let Err(e) = crate::settles::gate(
                    "create_nodes",
                    true,
                    approver.as_deref(),
                    &format!("a settle ({})", settle.describe()),
                )
            {
                refusals.push(format!("nodes[{i}]: {}", e.message));
            }
        }
        if !refusals.is_empty() {
            return Err(McpError::invalid_params(
                format!(
                    "{} item(s) would settle intent with nobody's name on them, and NOTHING                      was written:\n  - {}",
                    refusals.len(),
                    refusals.join("\n  - ")
                ),
                None,
            ));
        }
        let report = g
            .create_nodes_with(&specs, req.check_only)
            .map_err(dyno_err)?;
        if report.applied {
            for (spec, (approver, acted_at)) in specs.iter().zip(&signers) {
                let settles = g
                    .get_node(&spec.node_type, &spec.id)
                    .ok()
                    .flatten()
                    .is_some_and(|n| {
                        reflow2_core::intent::settles_intent(&spec.node_type, &n.properties)
                    });
                if settles {
                    crate::tools::capture::sign_as_approver(
                        &mut g,
                        &spec.node_type,
                        &spec.id,
                        approver.as_deref(),
                        acted_at.as_deref(),
                    )?;
                }
            }
        }
        bulk_result(report, NodeDto::from)
    }

    #[tool(
        description = "Create MANY edges in one call — the bulk form of the GENERIC `create_edge`: \
                       each item names its edge type, both endpoints and free `props`, and runs \
                       only the schema's checks. It runs NO typed helper's own checks (a \
                       `constrains` contribution and unit, a `governed_by` ruling, an \
                       `authored_by` role set), so for many edges a typed helper draws use \
                       `draw_edges`, which runs each helper's own body per item. ALL OF IT OR \
                       NONE OF IT: every item is attempted so you learn every failure at once, \
                       and if anything failed nothing is written. \
                       Ask for this to link many pairs of items in one call with an edge type no \
                       typed helper draws.",
        title = "Create Many Edges",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub async fn create_edges(
        &self,
        Parameters(req): Parameters<CreateEdgesReq>,
    ) -> Result<CallToolResult, McpError> {
        let mut specs = Vec::with_capacity(req.edges.len());
        for e in req.edges {
            let from_type = self
                .resolve_type(e.from_type.as_deref(), &e.from_id, "from_type")
                .await?;
            let to_type = self
                .resolve_type(e.to_type.as_deref(), &e.to_id, "to_type")
                .await?;
            specs.push(BulkEdgeSpec {
                edge_type: e.edge_type,
                from_type,
                from_id: e.from_id,
                to_type,
                to_id: e.to_id,
                props: parse_props(e.props)?,
            });
        }
        let mut g = self.write_lock().await?;
        // An edge that is the derived copy of a property (`reflow2_core::twins`)
        // is the store's to draw. One drawn by hand against its property is
        // refused before anything is written, every such item named at once,
        // the batch's own all-or-nothing rule.
        let mut refusals = Vec::new();
        for (i, spec) in specs.iter().enumerate() {
            if let Some(why) = g
                .twin_edge_refusal(&spec.edge_type, &spec.from_id, &spec.to_id, false)
                .map_err(dyno_err)?
            {
                refusals.push(format!("edges[{i}]: {why}"));
            }
        }
        if !refusals.is_empty() {
            return Err(McpError::invalid_params(
                format!(
                    "{} edge(s) would put a relation stored twice out of step, and NOTHING was \
                     written:\n  - {}",
                    refusals.len(),
                    refusals.join("\n  - ")
                ),
                None,
            ));
        }
        // A CONTRADICTS in the batch that turns a claim refuted says so in the
        // reply (req:a-write-that-makes-a-claim-read-refuted-says-so-in-its-
        // reply), as create_edge's does.
        let drawn: Vec<(String, String, Option<String>)> = specs
            .iter()
            .filter(|e| e.edge_type == "CONTRADICTS")
            .map(|e| {
                (
                    e.from_id.clone(),
                    e.to_id.clone(),
                    e.props
                        .get("alignment")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                )
            })
            .collect();
        let watch = crate::service::watch_attacks(&g, None, &drawn)?;
        let report = g
            .create_edges_with(&specs, req.check_only)
            .map_err(dyno_err)?;
        crate::service::bulk_result_refuting(report, EdgeDto::from, &g, &watch)
    }

    #[tool(
        description = "The bulk form of the typed edge tools. Each item is \
                       `{\"tool\": \"<typed tool>\", \"arguments\": {…its own arguments…}}` and runs \
                       that tool's own body — its checks, its refusal words, its reply — so a batch \
                       can never store what the tool alone would refuse. ALL OF IT OR NONE OF IT: \
                       every item is attempted, every failure is named by position, and if any \
                       failed nothing is written; `check_only` writes nothing. The reply names each \
                       edge drawn, subject first. \
                       Ask for this when you have a whole batch of typed links to record and want \
                       each one checked exactly as it would be alone.",
        title = "Draw Many Typed Edges",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub async fn draw_edges(
        &self,
        Parameters(req): Parameters<crate::bulk_edges::DrawEdgesReq>,
    ) -> Result<CallToolResult, McpError> {
        let indexed: Vec<(usize, crate::bulk_edges::DrawEdgeItem)> =
            req.edges.into_iter().enumerate().collect();
        // EACH ITEM GETS THE ARGUMENT CHECK ITS HELPER WOULD (`crate::arguments`):
        // checked against the helper's own input schema before its body
        // deserialises anything, refused in the very words the helper's own call
        // would receive — the bulk form's whole promise is "its checks, its
        // refusal words".
        let transport = crate::arguments::current_transport();
        let unfit: Vec<Option<String>> = indexed
            .iter()
            .map(|(_, item)| {
                let t = self.tool_router.get(&item.tool)?;
                let args = item.arguments.as_object()?;
                let violations = crate::arguments::check(&t.input_schema, args);
                (!violations.is_empty())
                    .then(|| crate::arguments::refusal(&item.tool, &violations, transport))
            })
            .collect();
        let mut g = self.write_lock().await?;
        // A typed helper in the batch that turns a claim refuted says so in
        // the reply (req:a-write-that-makes-a-claim-read-refuted-says-so-in-
        // its-reply): every record an item names is watched, and only one the
        // batch actually turns refuted is reported.
        let mut named: Vec<String> = Vec::new();
        for (_, item) in &indexed {
            crate::bulk_edges::ids_named(&item.arguments, &mut named);
        }
        named.sort();
        named.dedup();
        named.retain(|id| g.node_types_holding(id).is_ok_and(|t| t.len() == 1));
        // What they attack too: an item that makes an objection hold (a
        // `satisfies` that delivers it) refutes what the objection opposes.
        let mut ids = named.clone();
        for id in &named {
            ids.extend(g.attack_targets_of(id).map_err(dyno_err)?);
        }
        let watch = g.watch_refutations(ids).map_err(dyno_err)?;
        let report = g
            .atomically(
                &indexed,
                |(_, item)| item.tool.clone(),
                |g, (index, item)| match &unfit[*index] {
                    Some(refusal) => Err(refusal.clone()),
                    None => {
                        crate::bulk_edges::draw_one(g, item).map(|r| (*index, item.tool.clone(), r))
                    }
                },
                req.check_only,
            )
            .map_err(dyno_err)?;
        let reply = serde_json::to_value(crate::bulk_edges::reply(report, req.budget_chars))
            .map_err(ser_err)?;
        let applied = reply.get("applied").and_then(|v| v.as_bool()) == Some(true);
        if applied && !req.check_only {
            ok_json(crate::service::with_now_refuted(reply, &g, &watch)?)
        } else {
            ok_json(reply)
        }
    }

    #[tool(
        description = "Create an edge of any schema type between typed endpoints — the generic write for a relationship no typed helper covers. PREFER THE TYPED HELPER when one exists (`satisfies`, `allocate`, `depends_on`, `verifies` …): it supplies the required properties and refuses a wrong endpoint by name. Before reaching for this, call `describe_schema` with `from` and `to`: it names which edge types may join two node types and flags whether each actually models that pair or merely accepts it through a `*` wildcard — several will validate, and the first that does is not necessarily the one that means what you intend. Ask for this when you want to link two items with a relationship the typed tools do not offer.",
        title = "Create an Edge",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub async fn create_edge(
        &self,
        Parameters(req): Parameters<CreateEdgeReq>,
    ) -> Result<CallToolResult, McpError> {
        let props = parse_props(req.props)?;
        let mut g = self.write_lock().await?;
        let to_type =
            crate::service::resolve_node_type(&g, req.to_type.as_deref(), &req.to_id, "to_type")?;
        let from_type = crate::service::resolve_node_type(
            &g,
            req.from_type.as_deref(),
            &req.from_id,
            "from_type",
        )?;
        // The derived copy of a property is the store's to draw; drawn by hand
        // against its property it is refused, naming what to do instead.
        if let Some(why) = g
            .twin_edge_refusal(&req.edge_type, &req.from_id, &req.to_id, false)
            .map_err(dyno_err)?
        {
            return Err(McpError::invalid_params(why, None));
        }
        // A CONTRADICTS that turns a claim refuted says so in the reply
        // (req:a-write-that-makes-a-claim-read-refuted-says-so-in-its-reply).
        let drawn: Vec<(String, String, Option<String>)> = if req.edge_type == "CONTRADICTS" {
            vec![(
                req.from_id.clone(),
                req.to_id.clone(),
                props
                    .get("alignment")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            )]
        } else {
            Vec::new()
        };
        let watch = crate::service::watch_attacks(&g, None, &drawn)?;
        let edge = g.create_edge(
            &req.edge_type,
            &from_type,
            &req.from_id,
            &to_type,
            &req.to_id,
            props,
        );
        match edge {
            Ok(e) => {
                let reply = serde_json::to_value(EdgeDto::from(e)).map_err(ser_err)?;
                ok_json(crate::service::with_now_refuted(reply, &g, &watch)?)
            }
            // Say what would have worked — see `edge_error`.
            Err(e) => Err(edge_error(&g, &from_type, &to_type, e)),
        }
    }

    #[tool(
        description = "Discover the design vocabulary before writing to it: which node types \
                       exist, which properties they require, and which edge types may join two \
                       given types. Call this instead of guessing at create_node / create_edge. \
                       No arguments returns everything; `node_type` focuses one type and the \
                       edges it can carry; `from` + `to` together answer 'what may connect an X \
                       to a Y?'; `tool` alone answers 'how do I call this TOOL?' \u{2014} its \
                       input schema, every nested shape and allowed value, and the lessons this \
                       design holds for it, as tools/list gives them (brief by default, `full` \
                       for the entry unaltered). \
                       THE RANKING HAS FOUR TIERS, and the third is the one that used to be \
                       missing: edges naming BOTH types; then edges that DECLARE they are for \
                       this pair (their endpoint is open by design, not by oversight \u{2014} \
                       `GOVERNED_BY` is open on its source because anything may be governed); \
                       then edges naming ONE side exactly; then the rest, which accept the pair \
                       through a `*` wildcard and say nothing about meaning it. \
                       \u{2b50} READ `note` AND `modelled_open_matches` TOGETHER: \"nothing here \
                       is FOR this pair, ask for a new edge type\" and \"the answer is below, \
                       declared\" are different facts needing opposite actions, and until \
                       2026-09-02 they were rendered identically. Three projects acted on the \
                       wrong one and two drew an edge they themselves called a stand-in. \
                       Ask for this to see which fields each type takes before you use it.",
        title = "Describe the Schema",
        annotations(read_only_hint = true)
    )]
    pub async fn describe_schema(
        &self,
        Parameters(req): Parameters<DescribeSchemaReq>,
    ) -> Result<CallToolResult, McpError> {
        let g = self.graph.read().await;
        // HOW TO CALL ONE TOOL (`crate::describe_tool`): the served list of
        // THIS design, built from the lessons read under this same lock by the
        // function tools/list uses, so the answer is what a session on this
        // design is given — through every door a served tool reaches, the
        // `--call` door included, none of which reads tools/list
        // (req:the-cli-describes-any-tool-with-its-full-schema-and-lessons).
        if let Some(tool) = req.tool.as_deref() {
            if req.node_type.is_some() || req.from.is_some() || req.to.is_some() {
                return Err(McpError::invalid_params(
                    "describe_schema takes `tool` alone (how to call one served tool), or the \
                     vocabulary arguments (`node_type`, or `from` and `to`) \u{2014} not a mix."
                        .to_string(),
                    None,
                ));
            }
            let listing = self.listing_from(crate::lessons::lessons_by_step(&g));
            drop(g);
            return crate::describe_tool::describe_one(
                &listing,
                tool,
                req.full,
                crate::describe_tool::Door::Served,
                &crate::describe_tool::Source::Design("this design".to_string()),
                req.budget_chars
                    .unwrap_or(crate::reply_budget::DEFAULT_REPLY_BUDGET_CHARS),
            )
            .map_err(|why| McpError::invalid_params(why, None))
            .and_then(ok_json);
        }
        if req.full {
            return Err(McpError::invalid_params(
                "`full` only means something with `tool`: it asks for one tool's tools/list \
                 entry unaltered. The vocabulary is narrowed with `node_type`, or `from` and \
                 `to` together."
                    .to_string(),
                None,
            ));
        }
        // The vocabulary's bytes are in its `hint` and `description` prose, and
        // the part a caller acts on is the type and edge NAMES — so trimming
        // prose leaves this tool answering the question it is asked.
        let bound = |v: serde_json::Value| {
            crate::reply_budget::bound_reply(
                v,
                req.budget_chars
                    .unwrap_or(crate::reply_budget::DEFAULT_REPLY_BUDGET_CHARS),
                "Every type and edge name is still listed; narrow with `node_type`, or `from` \
                 and `to` together, to read one part's hints in full.",
            )
        };
        let full = matches!((&req.node_type, &req.from, &req.to), (None, None, None));
        // A FIELD'S WORD GIVEN AS `node_type` — "lemma", "hypothesis",
        // "counterexample" — answers the type that holds it
        // (req:the-vocabulary-is-served-in-a-fields-own-terms). Only when the
        // name is no type of reflow2's own, so every declared type answers
        // byte for byte as before; and only when some field uses the word, so
        // anything else is refused exactly as before.
        let by_term = match &req.node_type {
            Some(t) if !g.schema().node_types.contains_key(t.as_str()) => g.types_called(t),
            _ => Vec::new(),
        };
        let mut reply = match (&req.node_type, &req.from, &req.to) {
            (Some(word), None, None) if !by_term.is_empty() => {
                let served: Vec<_> = self
                    .tool_router
                    .list_all()
                    .into_iter()
                    .map(crate::arguments::published)
                    .collect();
                describe_by_term(&g, word, by_term, req.required_only, &served, &bound)?
            }
            (None, None, None) => {
                bound(serde_json::to_value(g.describe_vocabulary()).map_err(ser_err)?)
            }
            // The writers are added AFTER bounding: they are a handful of tool
            // names and one short sentence, and a trimmed reply is exactly the
            // one whose reader most needs to be told which tool writes the type.
            (Some(t), None, None) if req.required_only => with_writers(
                t,
                bound(
                    serde_json::to_value(g.describe_node_type_required(t).map_err(params_err)?)
                        .map_err(ser_err)?,
                ),
            ),
            (Some(t), None, None) => with_writers(
                t,
                bound(
                    serde_json::to_value(g.describe_node_type(t).map_err(params_err)?)
                        .map_err(ser_err)?,
                ),
            ),
            (None, Some(f), Some(t)) => with_drawn_by(bound(
                serde_json::to_value(g.edge_types_between(f, t).map_err(params_err)?)
                    .map_err(ser_err)?,
            )),
            // A half-given pair is a mistake, not a request for everything.
            _ => {
                return Err(McpError::invalid_params(
                    "describe_schema takes no arguments (the full vocabulary), `node_type` alone, \
                     or `from` and `to` together — not a mix."
                        .to_string(),
                    None,
                ));
            }
        };
        // THE MARK on a deprecated node type, property and edge type, where
        // the vocabulary is read (`crate::deprecation::mark_vocabulary`). An
        // empty registry leaves the answer untouched.
        crate::deprecation::mark_vocabulary(&mut reply, &self.deprecations);
        // THE FULL ANSWER LISTS WHAT IS GOING (`crate::deprecation`): every
        // deprecation with what to do, after bounding, like the writers above —
        // and no key at all while nothing is. AFTER THE MARK, NOT BEFORE: a
        // listed item names its type under `node_type`, so a listing that was
        // already in the reply got marked as if it were the vocabulary, and
        // `deprecations[].item` carried a `deprecated` copy of the mark. The
        // first real entry (the Anchor type, 2026-10-14) showed it.
        if full
            && let (Some(list), Some(m)) = (
                crate::deprecation::listing(&self.deprecations),
                reply.as_object_mut(),
            )
        {
            m.insert("deprecations".into(), list);
        }
        // AND THE WHOLE REPLY KEEPS TO THE BUDGET (crate::schema_budget). The
        // prose bound above cannot shorten the structure, and the writers and
        // deprecations ride after it, so a 4,000-character budget answered
        // up to 99,494 (the primes trial's F3).
        //
        // ONLY A BUDGET THE CALLER NAMES: with none, the reply is bounded as it
        // always was, so a plain call answers byte for byte as before.
        let reply = match req.budget_chars {
            None => reply,
            Some(budget) => crate::schema_budget::keep_to_budget(
                reply,
                budget,
                "Every name is listed; narrow with `node_type`, or `from` and `to` together, to \
                 read one part in full.",
            ),
        };
        ok_json(reply)
    }

    #[tool(
        description = "Fetch one node by id and read everything recorded about it: its \
                       properties and, with `include_edges`, its edges — every link to and from \
                       it, each with its type, direction, the node at the other end (id, type, \
                       name) and the edge's own properties, evidence included. `{node: {...}}` \
                       when present, `{node: null}` when absent; `node_type` is optional. \
                       Carries `discontinued`: true when an ACCEPTED Decision has withdrawn this \
                       node (OBSOLETES). READ IT — the stored `status` still records what was \
                       BUILT, so a withdrawn capability goes on saying `realized`. \
                       A Component also carries `built_reading`: what its evidence reads as \
                       (`built`, `partly_built`, `not_built`) with the evidence counted, \
                       computed beside its stored `status`, which it never moves. \
                       A concluded node also carries `foundations`: the axioms it rests on. \
                       Ask for this when you want everything recorded about one item — pull up \
                       the full record for a requirement or any single node — or a node and its \
                       edges: what it is connected to, its neighbours, which nodes link to it. \
                       Ask for this to show me everything recorded about one item.",
        title = "Get a Node",
        annotations(read_only_hint = true)
    )]
    pub async fn get_node(
        &self,
        Parameters(req): Parameters<GetNodeReq>,
    ) -> Result<CallToolResult, McpError> {
        let g = self.graph.read().await;
        // dec:idea-get-node-by-id-alone-since-the-prefix-names-the-type (Alex,
        // dev_storyflow): the id prefix names the type by convention, so the
        // type may be omitted — resolved by looking the id up under every type.
        // A COLLISION is refused, never guessed: the convention is not a rule,
        // and a silently wrong node is worse than a required field.
        let node_type: String = match req.node_type.clone() {
            Some(t) => t,
            None => {
                let holders = g.node_types_holding(&req.id).map_err(dyno_err)?;
                match holders.len() {
                    0 => {
                        // The same fact this function's own comment below
                        // records for a wrong TYPE name: a bare null is a fact
                        // the server HAS and declines to give. An absent id
                        // reads identically to a node that exists and is
                        // empty, so it says which.
                        let mut reply = json!({
                            "node": JsonValue::Null,
                            "empty_because": format!(
                                "no node with id {:?} exists under ANY declared type, so this is \
                                 \"not in this design\" rather than \"present and blank\". Check \
                                 the id, or find it by its words with search_design.",
                                req.id
                            ),
                        });
                        // Asked for edges: say there is no edges block
                        // because there is no node, rather than leave the key
                        // out and let its absence read as "no edges".
                        if req.include_edges.query().is_some() {
                            reply["edges"] = JsonValue::Null;
                        }
                        return ok_json(reply);
                    }
                    1 => holders.into_iter().next().unwrap_or_default(),
                    _ => {
                        return Err(McpError::invalid_params(
                            format!(
                                "id {:?} is held by MORE THAN ONE node type ({}). Refusing to \
                                 guess — pass `node_type` to say which you mean. (Ids are meant \
                                 to be unique across types by the typed-prefix convention; this \
                                 one is not.)",
                                req.id,
                                holders.join(", ")
                            ),
                            None,
                        ));
                    }
                }
            }
        };
        // A WRONG TYPE NAME and an ABSENT NODE used to answer identically: a
        // bare `null`. dev_storyflow (w-c216679a, 2026-08-09) asked for
        // `get_node("Epoch", …)` — the stored type is `DesignEpoch` — read the
        // null as "it isn't there", and their brief then told them to mint a
        // second epoch it explicitly forbids. They caught it only because they
        // distrusted the null.
        //
        // The type name is checkable against the schema for free, so answering
        // `null` for it is a fact the server HAS and declines to give. Same
        // class as `unmet_needs: 0` meaning "we never said we needed anything"
        // — a zero that cannot be told from an absence.
        if g.describe_node_type(&node_type).is_err() {
            let vocabulary = g.describe_vocabulary();
            let known: Vec<&str> = vocabulary
                .node_types
                .iter()
                .map(|n| n.node_type.as_str())
                .collect();
            // `Epoch` → `DesignEpoch` is the reported case, and containment
            // either way catches the whole family of dropped/added prefixes.
            let asked = node_type.to_ascii_lowercase();
            let near: Vec<&str> = known
                .iter()
                .copied()
                .filter(|n| {
                    let n = n.to_ascii_lowercase();
                    !asked.is_empty() && (n.contains(&asked) || asked.contains(&n))
                })
                .collect();
            let hint = if near.is_empty() {
                String::new()
            } else {
                format!(" Did you mean {}?", near.join(" or "))
            };
            return Err(McpError::invalid_params(
                format!(
                    "`{}` is not a node type in this schema, so `null` here would mean \"no \
                     such TYPE\" rather than \"no such node\" — and those are different facts.\
                     {hint}\n\nKnown node types: {}.\n\nCall `describe_schema` for the full \
                     vocabulary.",
                    node_type,
                    known.join(", ")
                ),
                None,
            ));
        }
        let node = g.get_node(&node_type, &req.id).map_err(dyno_err)?;
        // One named shape both ways (BL-57): `{node: {...}}` when present,
        // `{node: null}` when absent. Before, present returned a bare object
        // and absent returned `{value: null}` (the scalar wrap) — two shapes,
        // so an agent branching on the result read the absent case wrong.
        let node = match node {
            Some(stored) => decorate(
                &g,
                stored_to_value(stored.clone())?,
                &node_type,
                &stored.node_id,
            )?,
            None => JsonValue::Null,
        };
        // `include_edges` (dec:idea-an-edge-reader-returns-one-nodes-edges-and-find-tools-finds-it):
        // off by default, so every caller that does not ask gets the reply it
        // always got, byte for byte.
        //
        // AN ABSENT NODE SAYS WHICH EMPTY IT IS ON EVERY PATH. Until
        // 2026-10-03 the typed path answered a bare `{"node": null}` while the
        // untyped path, for the same absent id, said why
        // (fact:get-node-given-a-node-type-answers-an-absent-id-with-a-bare-null-2026-10-03).
        // The typed path can say one thing more: whether the id is held under
        // a DIFFERENT type, the commonest reason a typed read of a real id
        // comes back empty.
        let query = req.include_edges.query();
        if node.is_null() {
            let elsewhere: Vec<String> = g
                .node_types_holding(&req.id)
                .map_err(dyno_err)?
                .into_iter()
                .filter(|t| *t != node_type)
                .collect();
            let because = if elsewhere.is_empty() {
                format!(
                    "no {node_type} with id {:?} exists, and no node of any other type holds that \
                     id, so this is \"not in this design\" rather than \"present and blank\"{}. \
                     Check the id, or find it by its words with search_design.",
                    req.id,
                    if query.is_some() {
                        ", and there are no edges to read"
                    } else {
                        ""
                    }
                )
            } else {
                format!(
                    "no {node_type} with id {:?} exists, but that id IS held as {}. Read it \
                     without `node_type`, or with that type.",
                    req.id,
                    elsewhere.join(" and ")
                )
            };
            let mut reply = json!({ "node": JsonValue::Null, "empty_because": because });
            // Asked for edges: say there is no edges block because there is
            // no node, rather than leave the key out and let its absence read
            // as "no edges".
            if query.is_some() {
                reply["edges"] = JsonValue::Null;
            }
            return self.ok_read(&g, reply);
        }
        let Some((query, budget_chars)) = query else {
            return self.ok_read(&g, json!({ "node": node }));
        };
        // The node is sent whole, as it always is; the edges get what is left
        // of the budget.
        let spent_chars = node.to_string().len();
        let edges = g
            .node_edges(&node_type, &req.id, &query, budget_chars, spent_chars)
            .map_err(dyno_err)?;
        self.ok_read(&g, json!({ "node": node, "edges": edges }))
    }

    #[tool(
        description = "List nodes of a type. Answers with as many as fit in one reply and says what it left out \
                       — `total` is how many exist, `omitted` how many did not come back, `next_offset` where \
                       to resume, and `capped_by` why it stopped (`size` when the payload was full, `limit` \
                       when you asked for fewer). A cap is never silent, but it is also never a surprise: pass \
                       `brief: true` for id/name/status only when you want the shape of a large type, or \
                       `limit`/`offset` to page deliberately. On a mature design the full properties of one \
                       type can be tens of thousands of characters — read brief first, then fetch the few nodes \
                       you actually need with get_node. Every node carries `discontinued` (brief included): \
                       true when an ACCEPTED Decision has withdrawn it. The stored `status` records what was \
                       BUILT and does not move on withdrawal, so filtering a list on `status` alone will count \
                       things that no longer exist. Every Component also carries `built_reading`: what its \
                       evidence shows built, beside the stored `status`. PASS `level` TO ASK FOR ONE RUNG OF THE DECOMPOSITION \
                       LADDER — `component` / `subsystem` / `system` / `system_of_systems` / `enterprise`. That \
                       is how you ask for \"the top-level boxes\": deriving them from the CONTAINS spine \
                       instead returns leaves nobody wired to a parent, which is a different set and a \
                       confidently wrong one. Ask for this to list all the requirements, or every component, \
                       interface or other kind of item — show all of one type.",
        title = "List Nodes of a Type",
        annotations(read_only_hint = true)
    )]
    pub async fn scan_nodes(
        &self,
        Parameters(req): Parameters<ScanReq>,
    ) -> Result<CallToolResult, McpError> {
        let g = self.graph.read().await;
        let mut nodes = g.scan_nodes(&req.node_type).map_err(dyno_err)?;
        // `level` narrows to one rung of the decomposition ladder. Refused on
        // any other type and on an unknown rung, rather than answered with an
        // empty list: "no Components at that level" and "that is not a level"
        // are different facts, and the second one silently reads as the first.
        if let Some(level) = req.level.as_deref() {
            const LEVELS: [&str; 5] = [
                "component",
                "subsystem",
                "system",
                "system_of_systems",
                "enterprise",
            ];
            if req.node_type != reflow2_core::nodes::node::COMPONENT {
                return Err(McpError::invalid_params(
                    format!(
                        "`level` narrows the decomposition ladder and only `Component`                          carries one, so it cannot filter `{}`. Drop `level`, or scan                          `Component`.",
                        req.node_type
                    ),
                    None,
                ));
            }
            if !LEVELS.contains(&level) {
                return Err(McpError::invalid_params(
                    format!(
                        "`{}` is not a decomposition level. The ladder is: {}.",
                        level,
                        LEVELS.join(" ▸ ")
                    ),
                    None,
                ));
            }
            nodes.retain(|n| {
                n.properties
                    .get("level")
                    .and_then(|v| v.as_str())
                    // The schema defaults an unset level to `component`, so an
                    // older node with no level must still answer to it.
                    .unwrap_or("component")
                    == level
            });
        }
        let nodes = nodes;
        let total = nodes.len();
        let offset = req.offset.unwrap_or(0).min(total);
        let brief = req.brief.unwrap_or(false);
        // The nodes' share of the reply budget: the whole reply must fit, so
        // the paging fields and a read hint are kept back from it.
        let node_budget = req
            .budget_chars
            .unwrap_or(crate::reply_budget::DEFAULT_REPLY_BUDGET_CHARS)
            .saturating_sub(SCAN_REPLY_RESERVE_CHARS);

        // Render one node at a time, stopping at whichever bound bites first:
        // the caller's `limit`, or the payload budget. The budget exists because
        // an unbounded read of a mature type does not fail loudly — it arrives
        // as tens of thousands of characters that the client truncates, which is
        // the silent drop rule 6 forbids, happening outside reflow2 where
        // nothing can name it. Naming it here is the whole point.
        let mut items: Vec<JsonValue> = Vec::new();
        let mut bytes = 0usize;
        let mut capped_by: Option<&'static str> = None;
        for node in nodes.iter().skip(offset) {
            if req.limit.is_some_and(|limit| items.len() >= limit) {
                capped_by = Some("limit");
                break;
            }
            let rendered = if brief {
                brief_node(node)
            } else {
                stored_to_value(node.clone())?
            };
            // Decorated BEFORE the size is measured, so the payload budget
            // accounts for what actually goes out. Measuring the bare node and
            // then growing it is how a "bounded" read quietly exceeds its bound.
            //
            // NOT PINNED BY A TEST, and said so rather than left to look
            // verified: the ordering is reasoning, not measurement. Reversing
            // these two lines kills no test in
            // `discontinued_reaches_the_reader.rs`, because catching it needs a
            // graph sitting within ~22 bytes per node of the budget.
            // The overrun it would cause is bounded and small (one short bool
            // per returned node), which is why this is a note and not a gate.
            let rendered = decorate(&g, rendered, &req.node_type, &node.node_id)?;
            let size = rendered.to_string().len();
            // Always return at least one node: a single node larger than the
            // whole budget must still be readable, or a big node becomes
            // unreachable rather than merely expensive.
            // The comma before it counts too: the budget is the wire's.
            if !items.is_empty() && bytes + size + 1 > node_budget {
                capped_by = Some("size");
                break;
            }
            bytes += size + 1;
            items.push(rendered);
        }

        let returned = items.len();
        let next = offset + returned;
        self.ok_read(
            &g,
            json!({
                // `count` keeps its established meaning — how many came back in
                // this reply — so a caller that only reads {count, items} is
                // unaffected. `total` is the new, larger truth.
                "count": returned,
                "items": items,
                "total": total,
                "offset": offset,
                "returned": returned,
                "omitted": total.saturating_sub(next),
                "next_offset": (next < total).then_some(next),
                "capped_by": capped_by,
                "brief": brief,
            }),
        )
    }

    #[tool(
        description = "Find the reflow2 tool for a job you can describe but cannot name — \
                       'how do I record that a file implements a capability?', 'what shows me \
                       the blast radius?'. Ranked over the served surface itself (name, \
                       description, parameter names, and a field's own words for the type a tool \
                       writes), so it can never drift from the tools that actually exist. The whole surface is too large to hold in context at \
                       once; this is its catalogue. Descriptions come back trimmed; once you \
                       know the name, `describe_schema` with `tool` reads its whole input schema \
                       and the lessons this design holds for it (the reply's `describe` says \
                       how, through any door).",
        title = "Find a Tool",
        annotations(read_only_hint = true)
    )]
    pub async fn find_tools(
        &self,
        Parameters(req): Parameters<FindToolsReq>,
    ) -> Result<CallToolResult, McpError> {
        let query = req.query.to_lowercase();
        let terms: Vec<&str> = query
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|t| !t.is_empty())
            .collect();
        // Each tool AS PUBLISHED: a name the schema keeps only to redirect a
        // mistake (`x-reflow2-not-a-field`) is not offered here either, or the
        // catalogue invites the very call it refuses
        // (fact:the-changeevent-description-decoy-is-listed-by-find-tools-and-passes-the-argument-check-2026-10-05).
        let all: Vec<_> = self
            .tool_router
            .list_all()
            .into_iter()
            .filter(|t| !crate::service::DEPRECATED_TOOLS.contains(&t.name.as_ref()))
            .map(crate::arguments::published)
            .collect();
        let searched = all.len();
        // Document frequency over the whole served surface, computed per call
        // rather than cached: the surface is fixed at startup but small enough
        // that recomputing is cheaper than a cache that could go stale against
        // a router somebody composed differently.
        let corpus: Vec<(String, String)> = all
            .iter()
            .map(|t| {
                (
                    t.name.to_lowercase(),
                    t.description.as_deref().unwrap_or("").to_lowercase(),
                )
            })
            .collect();
        let weighted = term_weights(&terms, &corpus);

        let mut scored: Vec<(f64, JsonValue)> = all
            .iter()
            .filter_map(|tool| {
                let name = tool.name.as_ref();
                let description = tool.description.as_deref().unwrap_or("");
                let params = tool
                    .input_schema
                    .get("properties")
                    .and_then(JsonValue::as_object)
                    .map(|p| p.keys().cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                let score = score_tool(name, description, &params, &weighted);
                (score > 0.0).then(|| {
                    (
                        score,
                        json!({
                            "tool": name,
                            "score": score,
                            "summary": trim_summary(description),
                            "parameters": params,
                        }),
                    )
                })
            })
            .collect();

        // A QUERY IN A FIELD'S OWN WORDS — "record a lemma", "add a
        // hypothesis" — ranks the tools that write the type the word means
        // (`crate::field_terms`, req:the-vocabulary-is-served-in-a-fields-own-terms).
        // Each such item says which word found it, and what the mapping
        // means, under `also_called`. Nothing is added to any description:
        // the terms live in the schema and reach find_tools' index only.
        let index = crate::field_terms::index(&all);
        for (tool, (raw, homes)) in crate::field_terms::hits(&terms, &index, &corpus) {
            let Some(t) = all.iter().find(|t| t.name == tool.as_str()) else {
                continue;
            };
            let description = t.description.as_deref().unwrap_or("");
            let gain = raw / length_norm(description.to_lowercase().len());
            if gain <= 0.0 {
                continue;
            }
            let homes = json!(homes);
            match scored.iter_mut().find(|(_, v)| v["tool"] == tool.as_str()) {
                Some((score, v)) => {
                    *score += gain;
                    v["score"] = json!(*score);
                    v["also_called"] = homes;
                }
                None => {
                    let params = t
                        .input_schema
                        .get("properties")
                        .and_then(JsonValue::as_object)
                        .map(|p| p.keys().cloned().collect::<Vec<_>>())
                        .unwrap_or_default();
                    scored.push((
                        gain,
                        json!({
                            "tool": tool,
                            "score": gain,
                            "summary": trim_summary(description),
                            "parameters": params,
                            "also_called": homes,
                        }),
                    ));
                }
            }
        }

        // A QUERY THAT NAMES A NODE TYPE, spelled as the schema spells it
        // (`TemporalFact`, `Decision`), is answered first with the constructor
        // that creates that type: a writer of it (writers.json) that takes the
        // new node's `id`. Measured on 0.79.0 (2026-10-05): an agent asking for
        // "TemporalFact fact_type valid_from subject_id add fact" got
        // record_finding in no top 8, because the tool's words are "finding"
        // and "observed", not the type's name. The type a constructor creates
        // is a fact the surface already holds, so nothing is added to any
        // description; each such item says which type it creates.
        let words: std::collections::BTreeSet<&str> = req
            .query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect();
        let mut creates: std::collections::BTreeMap<String, String> = Default::default();
        for ty in crate::writers::node_types_written() {
            if !words.contains(ty.as_str()) {
                continue;
            }
            for tool in crate::revise_route::constructors_of(&ty, &all) {
                creates.entry(tool).or_insert_with(|| ty.clone());
            }
        }
        if !creates.is_empty() {
            let top = scored.iter().map(|(s, _)| *s).fold(0.0_f64, f64::max);
            for (tool, ty) in &creates {
                match scored.iter_mut().find(|(_, v)| v["tool"] == tool.as_str()) {
                    Some((score, v)) => {
                        *score += top + 1.0;
                        v["score"] = json!(*score);
                        v["creates"] = json!(ty);
                    }
                    None => {
                        let Some(t) = all.iter().find(|t| t.name == tool.as_str()) else {
                            continue;
                        };
                        let params = t
                            .input_schema
                            .get("properties")
                            .and_then(JsonValue::as_object)
                            .map(|p| p.keys().cloned().collect::<Vec<_>>())
                            .unwrap_or_default();
                        scored.push((
                            top + 1.0,
                            json!({
                                "tool": tool,
                                "score": top + 1.0,
                                "summary": trim_summary(t.description.as_deref().unwrap_or("")),
                                "parameters": params,
                                "creates": ty,
                            }),
                        ));
                    }
                }
            }
        }

        // Ties broken by name so the same query answers the same way twice —
        // a ranking that reshuffles teaches an agent not to trust it.
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1["tool"].as_str().cmp(&b.1["tool"].as_str()))
        });
        let matched = scored.len();
        let limit = req.limit.unwrap_or(DEFAULT_TOOL_SEARCH_RESULTS).max(1);
        let items: Vec<JsonValue> = scored.into_iter().take(limit).map(|(_, v)| v).collect();

        ok_json(json!({
            "count": items.len(),
            "items": items,
            "matched": matched,
            "omitted": matched.saturating_sub(items.len()),
            "searched": searched,
            "query": req.query,
            // A summary and parameter NAMES are not enough to call a tool:
            // the shapes, allowed values and lessons were learned one refusal
            // at a time through the door
            // (fact:root-cause-argument-shapes-are-learned-by-refusal-because-the-door-holds-the-full-schema-and-prints-none-of-it-2026-10-02).
            // So the reply names the route to the rest, in the words every
            // door can act on.
            "describe": crate::describe_tool::ROUTE_FROM_FIND_TOOLS,
        }))
    }

    #[tool(
        description = "Find design nodes by what they say, when you don't know their ids — \
                       'what does the design say about persistence?', 'is there already a \
                       requirement about latency?'. BM25 keyword search over every field a node's \
                       prose is written in (name, statement, description, a rationale, a \
                       change's summary, a check's findings, an interface's spec ...; never an \
                       id, date, hash, path or where a record came from), ranked, optionally \
                       scoped to one node type. \
                       Search BEFORE creating a node that might already exist, and to map the \
                       user's words to the node they mean. Result reports its own bounds: \
                       hits.len() == limit means there may be more, and a non-empty `stale` \
                       list means the index has drifted from the store. Each hit also lists, \
                       under `linked`, the records directly linked to it (a review relation or \
                       DECOMPOSES, with the direction), so a linked family comes back together \
                       even where only one of its records carries your words. On a design that \
                       pins others (a hub), `tracked_designs: true` searches them too, from their \
                       committed exports. \
                       Ask for this to find out whether anyone has written something down about a subject, such as budgets.",
        title = "Search the Design",
        output_schema = search_design_output_schema(),
        annotations(read_only_hint = true)
    )]
    pub async fn search_design(
        &self,
        Parameters(req): Parameters<SearchDesignReq>,
    ) -> Result<CallToolResult, McpError> {
        let g = self.graph.read().await;
        let result = g
            .search_design(
                &req.query,
                req.node_type.as_deref(),
                req.limit.unwrap_or(10),
            )
            .map_err(dyno_err)?;
        // A design that PINS other designs says which of them this search did
        // not look in. One that pins none answers exactly as before.
        let pins = crate::upstream::pins_not_searched(
            &g.declared_dependencies().map_err(dyno_err)?,
            self.served_from_registry(),
        );
        // `tracked_designs` is served where a call is received
        // (`service::tracked`), which takes it out before this handler runs.
        // Reaching here with it means it was not served on this path, and the
        // reply says so rather than let it read as honoured.
        let tracked_note = req.tracked_designs.unwrap_or(false).then(|| {
            if self.served_from_registry() {
                crate::service::TRACKED_ON_A_REGISTRY
            } else {
                crate::service::TRACKED_NOT_AT_THE_CALL_LAYER
            }
        });
        if pins.is_empty() && tracked_note.is_none() {
            return self.ok_read(&g, result);
        }
        let mut reply = serde_json::to_value(result).map_err(ser_err)?;
        if let Some(o) = reply.as_object_mut() {
            if !pins.is_empty() {
                o.insert(
                    "not_searched_note".into(),
                    JsonValue::String(crate::upstream::not_searched_note(pins.len(), false)),
                );
                o.insert("not_searched".into(), JsonValue::Array(pins));
            }
            if let Some(note) = tracked_note {
                o.insert("tracked_note".into(), JsonValue::String(note.to_string()));
            }
        }
        self.ok_read(&g, reply)
    }

    #[tool(
        description = "WHAT THE DESIGN HOLDS ABOUT ONE SUBJECT, read-only, in one call — the \
                       /topic view: 'show me something about X'. Searches the design for the \
                       phrase, groups the hits by node type, and for each hit reads its status, \
                       its connections, and the LATEST DATED change or measurement that touched \
                       it. Computed server-side so two agents render one topic the same way. \
                       ⭐ `not_found` IS MANDATORY: it says what was searched, which populated \
                       node types matched NOTHING, and whether the list was cut — a miss on the \
                       caller's phrasing must never read as the design holding nothing. \
                       Bounded: `budget` says what was withheld; `count` and `by_type` never are. \
                       ⭐ ALSO THE FIRST READ BEFORE YOU ASSERT A PROJECT'S STATUS TO ANYONE — a \
                       progress report or deck for management, \"what has this project achieved\", \
                       \"where does it stand\": call it on the project's name before you write a \
                       word (field report 2026-09-14). Writes nothing. \
                       Ask for this to read everything the design says about one subject in one place.",
        title = "Topic Report",
        annotations(read_only_hint = true)
    )]
    pub async fn topic_report(
        &self,
        Parameters(req): Parameters<TopicReportReq>,
    ) -> Result<CallToolResult, McpError> {
        let g = self.graph.read().await;
        let report = g
            .topic_report(
                &req.query,
                req.limit,
                req.budget_chars
                    .unwrap_or(reflow2_core::detect::DEFAULT_REPLY_BUDGET_CHARS),
            )
            .map_err(dyno_err)?;
        self.ok_read(&g, report)
    }

    #[tool(
        description = "READ BACK A LINE OF REASONING, read-only: an argument for and against, an \
                       experiment's case, a paper's chain of claims, a legal case. Each claim \
                       comes with its COMPUTED truth (holds, conditional, fails or unknown), the steps that \
                       support it (rule, warrant, who confirmed them), the objections against it \
                       and how each stands (`refutes`, `stands`, `answered`), and its cited \
                       sources. Then: what the reasoning rests on and which premises cite no \
                       source, the points still contested, the disputes a Decision set to \
                       `contested` keeps open on purpose, and every claim with no support. \
                       `claim` reads under one claim; omitted, every claim, refuted ones \
                       included. `what_this_means`: structure is not truth. Shown \
                       here, never raised as a gap. Bounded by `budget_chars`: every claim \
                       comes back whole, and a long proof pages with `next_offset`. Writes \
                       nothing. \
                       Ask for this to check an argument point by point: what backs each claim, \
                       what is disputed, and what has no evidence behind it.",
        title = "Reasoning Report",
        annotations(read_only_hint = true)
    )]
    pub async fn reasoning_report(
        &self,
        Parameters(req): Parameters<ReasoningReportReq>,
    ) -> Result<CallToolResult, McpError> {
        let g = self.graph.read().await;
        let report = g
            .reasoning_report_from(req.claim.as_deref(), req.limit, req.offset)
            .map_err(dyno_err)?;
        let full = serde_json::to_value(report).map_err(ser_err)?;
        let bounded = reasoning_page(
            full,
            req.budget_chars
                .unwrap_or(crate::reply_budget::DEFAULT_REPLY_BUDGET_CHARS),
            req.budget_chars.is_none(),
        );
        self.ok_read(&g, bounded)
    }

    #[tool(
        description = "Delete a node by id, returning whether it existed. `node_type` is optional: omitted, the id's prefix is taken as the type you mean (`req:` → Requirement), and the node is deleted only when the one type holding the id is that type — otherwise nothing is deleted and the refusal names what the id holds. THIS REMOVES EVERY EDGE ATTACHED TO IT AND THERE IS NO UNDO — which is why it is for MISTAKES, not for history. A requirement the user dropped, a capability something replaced, a component that shipped and was decommissioned are retired on the record instead (set_requirement_status, OBSOLETES, record_change with deprecation) so the design can still say why its survivors are shaped as they are; the retire-from-design skill forces that question first. Run `propagate_from` on the id before deleting: if the blast radius is not close to empty, this is the wrong tool. Ask for this when you want to remove an item from the design entirely — get rid of a thing that should no longer be in the model — and only for one that never should have existed.",
        title = "Delete a Node",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    pub async fn delete_node(
        &self,
        Parameters(req): Parameters<TypedIdReq>,
    ) -> Result<CallToolResult, McpError> {
        let mut g = self.write_lock().await?;
        if let Some(node_type) = &req.node_type {
            let deleted = g.delete_node(node_type, &req.id).map_err(dyno_err)?;
            return ok_json(json!({ "deleted": deleted }));
        }
        // The type omitted: the id's prefix is the caller's stated type, and
        // the holder must match it (req:delete-node-takes-the-id-prefix-as-the-
        // callers-stated-type). Resolved under the write lock, so nothing can
        // change between the check and the delete.
        let node_type = delete_type_from_prefix(&g, &req.id)?;
        let Some(node_type) = node_type else {
            // Nothing holds the id: reported, not an error, as with a type.
            return ok_json(json!({ "deleted": false }));
        };
        let deleted = g.delete_node(&node_type, &req.id).map_err(dyno_err)?;
        ok_json(json!({ "deleted": deleted, "node_type": node_type }))
    }

    #[tool(
        description = "Delete one edge by type and endpoint ids (true if it existed). For \
                       retracting a link that was drawn in error — a wrongly-asserted SATISFIES, \
                       an allocation that never happened. A link that WAS true and stopped being \
                       true is design history, not an error: record it (record_change) rather \
                       than erasing it. Until this tool existed the only way to remove a wrong \
                       edge over MCP was to delete one of its endpoints. \
                       Ask for this to remove one link between two items.",
        title = "Delete an Edge",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    pub async fn delete_edge(
        &self,
        Parameters(req): Parameters<DeleteEdgeReq>,
    ) -> Result<CallToolResult, McpError> {
        let mut g = self.write_lock().await?;
        // `{deleted}` rather than the bare bool the core returns: a scalar in
        // `structuredContent` is the BL-48 defect (ok_json would wrap it as an
        // anonymous `{value}`, but the field deserves its name).
        // Removing the derived copy while its property still names it would
        // leave the relation half-stored; the property is what moves it.
        if let Some(why) = g
            .twin_edge_refusal(&req.edge_type, &req.from_id, &req.to_id, true)
            .map_err(dyno_err)?
        {
            return Err(McpError::invalid_params(why, None));
        }
        let deleted = g
            .delete_edge(&req.edge_type, &req.from_id, &req.to_id)
            .map_err(dyno_err)?;
        ok_json(json!({ "deleted": deleted }))
    }
}

/// The type `delete_node` deletes when none is passed: the one type holding
/// `id`, and only when it is the type the id's prefix names by convention
/// ([`reflow2_core::nodes::node::ID_PREFIXES`]). `Ok(None)` when nothing holds
/// the id. Every other case is refused before anything is deleted, naming what
/// the id holds and the way through — the prefix is the caller's stated
/// belief, and a holder that differs from it is the typo the strict form of
/// 2026-09-06 guarded against.
fn delete_type_from_prefix(
    g: &reflow2_core::DesignGraph,
    id: &str,
) -> Result<Option<String>, McpError> {
    let holders = g.node_types_holding(id).map_err(dyno_err)?;
    let edges = || -> Result<usize, McpError> {
        Ok(g.outgoing(id, None).map_err(dyno_err)?.len()
            + g.incoming(id, None).map_err(dyno_err)?.len())
    };
    let refuse = |why: String| Err(McpError::invalid_params(why, None));
    match holders.as_slice() {
        [] => Ok(None),
        [held] => {
            let n = edges()?;
            match reflow2_core::nodes::node::type_named_by_prefix(id) {
                Some((_, Some(named))) if named == held => Ok(Some(held.clone())),
                Some((prefix, Some(named))) => refuse(format!(
                    "{id:?} is a {held} with {n} edge(s), but its prefix `{prefix}:` names a \
                     {named} by convention, so the id does not say you mean this {held}. \
                     Nothing was deleted. If you do, pass node_type: \"{held}\"."
                )),
                Some((prefix, None)) => refuse(format!(
                    "{id:?} is a {held} with {n} edge(s), and its prefix `{prefix}:` names no \
                     type by convention, so the id does not say which type you mean to \
                     delete. Nothing was deleted. To delete it, pass node_type: \"{held}\"."
                )),
                None => refuse(format!(
                    "{id:?} is a {held} with {n} edge(s), and it has no `<prefix>:` naming a \
                     type, so the id does not say which type you mean to delete. Nothing was \
                     deleted. To delete it, pass node_type: \"{held}\"."
                )),
            }
        }
        many => refuse(format!(
            "{id:?} is held by MORE THAN ONE node type ({}), so a bare id cannot say which to \
             delete. Nothing was deleted. Pass node_type to name the one you mean.",
            many.join(", ")
        )),
    }
}

/// Render a stored node the way the read tools have always rendered it.
fn stored_to_value(node: StoredNode) -> Result<JsonValue, McpError> {
    serde_json::to_value(NodeDto::from(node)).map_err(ser_err)
}

/// Add the one fact a stored node cannot carry: **has this been withdrawn?**
///
/// # Why this exists, measured 2026-08-12
///
/// `dec:idea-discontinued-is-a-first-class-state` gave `OBSOLETES` four readers
/// — three capability detectors and delivery arithmetic — and every one of them
/// is a COMPUTATION. None was a READ. So `cap:content-store`, discontinued on
/// Anthony's word on 2026-08-09 with its code deleted, still came back from
/// `scan_nodes` as `status: "realized"`, and a session believed it. It then
/// recommended he build a surface for a feature he had personally removed three
/// days earlier.
///
/// The graph was right and the detectors were right. The reader was told
/// nothing — the same class of defect as `get_node` answering a bare `null` for
/// an unknown TYPE: a fact the server holds and declines to give.
///
/// # Derived, never stored
///
/// Computed from the edge on every read, exactly as the detectors compute it,
/// so a reader and a detector can never disagree about the same node. Nothing
/// is written back: `Capability.status` keeps recording what was BUILT, and
/// `dec:idea-does-a-capability-need-a-cancelled-state` — open, marked, and
/// Anthony's — is not settled by implementation here.
///
/// # Present and false, never absent
///
/// Emitted on every node including live ones. "Not discontinued" and "this
/// build does not report discontinuation" must not share one answer, which is
/// the rule `severed_containment` and `not_observed_about` already follow.
///
/// # And a Component's `built_reading`, since 2026-10-13
///
/// The second fact a stored node cannot carry, for the same reason: the
/// detectors computed it and no read said it. 107 of 109 components on
/// reflow2's own design read `planned` while 88 had built work behind them
/// (`fact:root-cause-built-work-stays-open-because-its-record-is-kept-by-hand-and-the-ship-moves-only-the-capability-2026-10-13`),
/// and a reader of this reply saw `planned` and nothing beside it. Computed
/// on every read by [`DesignGraph::component_build`], the one computation
/// `understated_status` reads too; `status` is never touched. Only on a
/// Component: present on every one, absent on every other type.
fn decorate(
    g: &DesignGraph,
    mut rendered: JsonValue,
    node_type: &str,
    node_id: &str,
) -> Result<JsonValue, McpError> {
    let discontinued = g.is_discontinued(node_id).map_err(dyno_err)?;
    let built_reading = if node_type == reflow2_core::nodes::node::COMPONENT {
        let reading = g.component_build(node_id).map_err(dyno_err)?.reading();
        Some(serde_json::to_value(reading).map_err(ser_err)?)
    } else {
        None
    };
    // A STATED INFERENCE STEP CARRIES ITS VERDICT, computed on the read like
    // `built_reading` (crate::inference): its rule over each premise's truth,
    // holds / fails / unknown, with every premise and why. Only on an
    // Inference; absent on every other type.
    let inference_reading = if node_type == reflow2_core::nodes::node::INFERENCE {
        g.inference_reading(node_id)
            .map_err(dyno_err)?
            .map(serde_json::to_value)
            .transpose()
            .map_err(ser_err)?
    } else {
        None
    };
    // FOUNDATIONS (requirement E): a concluded node's terminal premises — the
    // axioms its inference chain ultimately rests on. Omitted when the node is
    // not concluded by any Inference (no foundations to show, and an absent
    // field is "not a conclusion", not "rests on nothing").
    let foundations = g
        .foundations_reading(node_id)
        .map_err(dyno_err)?
        .map(serde_json::to_value)
        .transpose()
        .map_err(ser_err)?;
    // PROOF STATUS (requirement B): proven, shown, or unproven. Omitted when
    // the node is not concluded by any Inference. A conclusion is PROVEN only
    // when every step in at least one holding derivation is confirmed
    // (confirmed_how is person or checker); SHOWN when it holds but some
    // steps are unconfirmed; UNPROVEN when no derivation holds. Its
    // `total_steps` counts the distinct steps under the holding lines, each
    // once, and its `counts` says so on every reply (`ProofReading`).
    let proof_status = g
        .proof_reading(node_id)
        .map_err(dyno_err)?
        .map(serde_json::to_value)
        .transpose()
        .map_err(ser_err)?;
    // WHAT A STATEMENT SAYS IT IS (`statement_kind`) and how it reads now —
    // a conjecture `open` or `refuted`, a theorem `stated` until a step
    // concludes it. Omitted when the node says nothing.
    let says_its_kind = rendered
        .get("properties")
        .and_then(|p| p.get("statement_kind"))
        .is_some();
    let statement_reading = if says_its_kind {
        g.statement_reading(node_id)
            .map_err(dyno_err)?
            .map(serde_json::to_value)
            .transpose()
            .map_err(ser_err)?
    } else {
        None
    };
    // A REQUIREMENT'S REASON TO EXIST (req:an-edge-names-the-claim-that-
    // argues-for-it-and-a-governed-requirement-reads-its-reason-to-exist,
    // item 1): the Decision(s) it is GOVERNED_BY and each one's support,
    // read apart from whether it is delivered. Omitted on a requirement no
    // Decision governs, and on every other type.
    let reason_to_exist = if node_type == reflow2_core::nodes::node::REQUIREMENT {
        g.reason_to_exist(node_id)
            .map_err(dyno_err)?
            .map(serde_json::to_value)
            .transpose()
            .map_err(ser_err)?
    } else {
        None
    };
    // A STATISTICAL TEST'S READING (reflow2_core::hypothesis): supported at α,
    // not rejected, or unknown, with its assumptions, what it spends and any
    // criterion changed after its data. Only on a Verification that carries
    // one.
    let test_reading = if node_type == reflow2_core::nodes::node::VERIFICATION {
        g.test_reading(node_id)
            .map_err(dyno_err)?
            .map(serde_json::to_value)
            .transpose()
            .map_err(ser_err)?
    } else {
        None
    };
    // A CHECK'S CONDITIONS (req:a-check-resting-on-a-claim-that-loses-
    // support-reads-stale): the reasoned claims that constrain it, read, and
    // whether it is stale. Only on a Verification a reasoned claim constrains.
    let check_conditions = if node_type == reflow2_core::nodes::node::VERIFICATION {
        g.check_conditions(node_id)
            .map_err(dyno_err)?
            .map(serde_json::to_value)
            .transpose()
            .map_err(ser_err)?
    } else {
        None
    };
    if let Some(obj) = rendered.as_object_mut() {
        obj.insert("discontinued".to_string(), json!(discontinued));
        if let Some(c) = check_conditions {
            obj.insert("check_conditions".to_string(), c);
        }
        if let Some(t) = test_reading {
            obj.insert("test_reading".to_string(), t);
        }
        if let Some(r) = reason_to_exist {
            obj.insert("reason_to_exist".to_string(), r);
        }
        if let Some(sr) = statement_reading {
            obj.insert("statement_reading".to_string(), sr);
        }
        if let Some(reading) = built_reading {
            obj.insert("built_reading".to_string(), reading);
        }
        if let Some(reading) = inference_reading {
            obj.insert("inference_reading".to_string(), reading);
        }
        // EVERY FOUNDATION HOLDS AND NO DERIVATION DOES: both are true, and
        // the reply says why, rather than showing "holds" beside "no
        // derivation holds" (item 13 of req:small-read-back-faults-from-the-
        // cogito-trial-are-fixed, the capstone).
        let all_foundations_hold = foundations.as_ref().is_some_and(|f| {
            f["foundations"]
                .as_array()
                .is_some_and(|a| !a.is_empty() && a.iter().all(|x| x["truth"] == "holds"))
        });
        let not_derived = proof_status
            .as_ref()
            .is_some_and(|p| matches!(p["status"].as_str(), Some("unproven" | "contested")));
        if all_foundations_hold && not_derived {
            obj.insert(
                "foundations_and_proof".to_string(),
                json!(
                    "Every foundation holds, and no derivation does: both are true. What fails is \
                     between them — a STEP: its rule over its premises, a premise it concludes on \
                     the way, an objection to the step or to a claim on the way, or a gate (a case \
                     split not declared exhaustive, a generalization, the step's rule node). \
                     reasoning_report on this claim reads each step."
                ),
            );
        }
        if let Some(f) = foundations {
            obj.insert("foundations".to_string(), f);
        }
        if let Some(ps) = proof_status {
            obj.insert("proof_status".to_string(), ps);
        }
    }
    Ok(rendered)
}

/// A node type's description says which served tool writes it (I14b). Read
/// from the one map `tools/vocabulary_reach.py` also reads
/// (`crate::writers`), so the two cannot disagree.
fn with_writers(node_type: &str, mut v: serde_json::Value) -> serde_json::Value {
    let writers = crate::writers::node_type_writers(node_type);
    if let Some(obj) = v.as_object_mut() {
        if writers.is_empty() {
            obj.insert(
                "no_typed_writer".to_string(),
                serde_json::Value::String(crate::writers::NO_TYPED_WRITER.to_string()),
            );
        }
        obj.insert("written_by".to_string(), serde_json::json!(writers));
    }
    v
}

/// `describe_schema`'s answer to a FIELD's word given as `node_type`: the type
/// that holds it, answered exactly as its own name would be, with
/// `found_by_term` naming where the word lives and what the mapping means and
/// does not, and a `note` saying the answer is for that type.
///
/// WHY `node_type` AND NOT A NEW ARGUMENT. An agent holding "a lemma" that
/// guesses at a type asks `describe_schema {"node_type": "Lemma"}` — and was
/// refused "Unknown node type: Lemma". Answering that very call meets the agent
/// where it already is, adds nothing to tools/list (whose schemas every client
/// pays for on every session, and which the requirement keeps the terms out
/// of), and turns a refusal into an answer, so no call that was answered
/// changes. A term is never spelled as a type's name
/// (`tests/a_fields_own_terms_have_one_home.rs`), so the lookup cannot shadow
/// a type.
///
/// A word some field uses for an EDGE type answers that edge, whole, with the
/// tools that draw it — its typed helpers, or the constructor of the record it
/// is pinned to (`premise` → PREMISE_OF, drawn by `add_inference`; see
/// `crate::field_terms::edge_tools`). A word that names different types in different fields
/// answers none of them: every home is listed, to be asked for by name.
fn describe_by_term(
    g: &DesignGraph,
    word: &str,
    homes: Vec<reflow2_core::vocabulary::TermHome>,
    required_only: bool,
    served: &[rmcp::model::Tool],
    bound: &dyn Fn(serde_json::Value) -> serde_json::Value,
) -> Result<serde_json::Value, McpError> {
    let node_types: std::collections::BTreeSet<&str> = homes
        .iter()
        .filter_map(|h| h.node_type.as_deref())
        .collect();
    let edge_types: std::collections::BTreeSet<&str> = homes
        .iter()
        .filter_map(|h| h.edge_type.as_deref())
        .collect();
    let word = word.trim();
    let fields: Vec<&str> = {
        let mut f: Vec<&str> = homes.iter().map(|h| h.field.as_str()).collect();
        f.dedup();
        f
    };
    let (mut reply, note) = match (
        node_types.iter().next(),
        edge_types.iter().next(),
        node_types.len() + edge_types.len(),
    ) {
        (Some(t), None, 1) => {
            let v = if required_only {
                serde_json::to_value(g.describe_node_type_required(t).map_err(params_err)?)
            } else {
                serde_json::to_value(g.describe_node_type(t).map_err(params_err)?)
            }
            .map_err(ser_err)?;
            (
                with_writers(t, bound(v)),
                format!(
                    "`{word}` is not one of reflow2's type names. In {} it names what reflow2 \
                     calls {t}, so this is {t}'s own answer. `found_by_term` says what that \
                     mapping means and what it does not.",
                    fields.join(" and ")
                ),
            )
        }
        (None, Some(e), 1) => {
            let mut v = bound(
                serde_json::to_value(g.describe_edge_type(e).map_err(params_err)?)
                    .map_err(ser_err)?,
            );
            if let Some(m) = v.as_object_mut() {
                m.insert(
                    "drawn_by".into(),
                    json!(crate::field_terms::edge_tools(g.schema(), e, served)),
                );
            }
            (
                v,
                format!(
                    "`{word}` is not one of reflow2's type names. In {} it names what reflow2 \
                     draws as the edge type {e}, so this is {e}'s answer, whole, with the tools \
                     that draw it. \
                     `found_by_term` says what that mapping means and what it does not.",
                    fields.join(" and ")
                ),
            )
        }
        _ => (
            json!({}),
            format!(
                "`{word}` names different types in different fields, so no one type is \
                 answered: each is listed in `found_by_term` with what it means there. Ask \
                 again with the type's own name."
            ),
        ),
    };
    if let Some(m) = reply.as_object_mut() {
        m.insert("found_by_term".into(), json!(homes));
        m.insert("note".into(), json!(note));
    }
    Ok(reply)
}

/// Each edge that accepts a pair says which served tool draws it, so a caller
/// who found the right edge is not left to find its tool by a second search.
fn with_drawn_by(mut v: serde_json::Value) -> serde_json::Value {
    if let Some(matches) = v
        .get_mut("matches")
        .and_then(serde_json::Value::as_array_mut)
    {
        for m in matches {
            let edge = m
                .get("edge_type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            if let Some(obj) = m.as_object_mut() {
                obj.insert(
                    "drawn_by".to_string(),
                    serde_json::json!(crate::writers::edge_type_writers(&edge)),
                );
            }
        }
    }
    v
}

/// The settle a generic node write would make: what is stored, overlaid with
/// what the write sends, less what it unsets — asked of the core's one table.
pub(crate) fn generic_settle(
    node_type: &str,
    id: &str,
    prior: Option<&StoredNode>,
    props: &std::collections::HashMap<String, Value>,
    unset: &[String],
) -> Option<reflow2_core::intent::NewSettle> {
    reflow2_core::intent::settling_property(node_type)?;
    let mut after = prior.map(|p| p.properties.clone()).unwrap_or_default();
    for (k, v) in props {
        after.insert(k.clone(), v.clone());
    }
    for k in unset {
        after.remove(k);
    }
    reflow2_core::intent::newly_settles(node_type, id, prior.map(|p| &p.properties), &after)
}

/// Hold one page of `reasoning_report` to `budget` characters by returning
/// FEWER CLAIMS, each WHOLE, never every claim with its words cut
/// (req:the-reasoning-read-back-keeps-the-words-of-the-claims-it-returns).
///
/// MEASURED BEFORE, proof trial rung 2 on flo2.io (0.86.0, held to 27,267
/// characters): 152 text fields of the claims cut to "…", then 25 of the 60
/// claims shown, so the statements and warrants, which are the proof, were
/// readable only through scan_nodes.
///
/// The claims give from the tail of the page, whole, to a floor of one; the
/// reply's `next_offset` and `remaining` then say where the next page starts,
/// and `page` names the budget it was held to, so a budget lowered on the way
/// shows in the result itself. Only if ONE claim and the summary still pass
/// the budget is prose cut, and then only outside `claims`.
///
/// EACH PREMISE'S REASON IS SAID ONCE (crate::said_once): a premise row
/// carries a short reading, the page cites each full reason once in
/// `premise_whys`, and an advisory a reason would repeat for every unverified
/// citation is said once in `advisories` with the ids it applies to — built
/// from the rows the page keeps, so it is counted against the budget too.
fn reasoning_page(mut full: JsonValue, budget: usize, default_budget: bool) -> JsonValue {
    use crate::reply_budget::chars_of;
    let mut rows: Vec<JsonValue> = full
        .get_mut("claims")
        .and_then(|c| c.as_array_mut())
        .map(std::mem::take)
        .unwrap_or_default();
    let moved: Vec<std::collections::BTreeMap<String, String>> = rows
        .iter_mut()
        .map(crate::said_once::premise_rows)
        .collect();
    let offset = full["offset"].as_u64().unwrap_or(0) as usize;
    let total = full["total_claims"].as_u64().unwrap_or(rows.len() as u64) as usize;
    let page_len = rows.len();
    let held = if default_budget {
        format!("budget_chars {budget} (the default)")
    } else {
        format!("budget_chars {budget}")
    };
    let build = |keep: usize| -> JsonValue {
        let mut v = full.clone();
        let end = offset + keep;
        let remaining = total.saturating_sub(end);
        let next = (end < total).then_some(end);
        let fewer = if keep < page_len {
            format!(
                " To fit, it returns {keep} of the {page_len} this page's `limit` allows; no \
                 claim is cut."
            )
        } else {
            String::new()
        };
        let on = match next {
            Some(n) => format!(" {remaining} remain: read on with `offset: {n}`."),
            None => " This is the last page.".to_string(),
        };
        let reads = format!(
            "{keep} of {total} claims, from offset {offset}, each WHOLE (its words in `says`, \
             each step's rule and warrant, each premise's name and its reason in short; \
             each premise's full reason once, in `premise_whys`), in proof order.{on} This reply \
             was held to {held}.{fewer} The summary lists cover the whole walk, not this page."
        );
        if let Some(o) = v.as_object_mut() {
            o.insert("claims".into(), JsonValue::Array(rows[..keep].to_vec()));
            o.insert("remaining".into(), json!(remaining));
            match next {
                Some(n) => o.insert("next_offset".into(), json!(n)),
                None => o.remove("next_offset"),
            };
            o.insert(
                "page".into(),
                json!({
                    "offset": offset,
                    "returned": keep,
                    "total": total,
                    "remaining": remaining,
                    "next_offset": next,
                    "budget_chars": budget,
                    "reads": reads,
                }),
            );
        }
        let mut cited = std::collections::BTreeMap::new();
        for m in &moved[..keep] {
            crate::said_once::merge_moved(&mut cited, m);
        }
        crate::said_once::cite_once(&mut v, &cited);
        v
    };
    // Estimate from what each claim costs, then confirm on the real reply:
    // the page block's numbers move its own size by a few characters.
    let costs: Vec<usize> = rows.iter().map(chars_of).collect();
    let base = chars_of(&build(0));
    let mut keep = page_len;
    let mut spent: usize = costs.iter().sum::<usize>() + page_len.saturating_sub(1);
    while keep > 1 && base + spent > budget {
        keep -= 1;
        spent -= costs[keep] + usize::from(keep > 0);
    }
    let mut out = build(keep);
    while keep > 1 && chars_of(&out) > budget {
        keep -= 1;
        out = build(keep);
    }
    if chars_of(&out) <= budget {
        return out;
    }
    // One whole claim and the summary still pass the budget: cut prose
    // OUTSIDE the claims, and say so in the usual budget block.
    let Some(o) = out.as_object_mut() else {
        return out;
    };
    let claims = o.remove("claims").unwrap_or(JsonValue::Null);
    let page = o.remove("page").unwrap_or(JsonValue::Null);
    // The claims' own reasons, said once, stay whole with the claims.
    let with_claims: Vec<(&str, JsonValue)> = ["premise_whys", "premise_whys_note", "advisories"]
        .into_iter()
        .filter_map(|k| o.remove(k).map(|v| (k, v)))
        .collect();
    let summary = JsonValue::Object(std::mem::take(o));
    let hint = "The claims are whole; what was cut is outside them. Read one claim with \
                reasoning_report on its id, or pass a larger budget_chars.";
    let whole = |room: usize| -> JsonValue {
        let mut rest = crate::reply_budget::bound_reply(summary.clone(), room, hint);
        if let Some(r) = rest.as_object_mut() {
            r.insert("claims".into(), claims.clone());
            r.insert("page".into(), page.clone());
            for (k, v) in &with_claims {
                r.insert((*k).into(), v.clone());
            }
        }
        rest
    };
    // THE SUMMARY GETS EVERY CHARACTER THE CLAIMS LEAVE (items 5 and 7 of
    // req:small-read-back-faults-from-the-cogito-trial-are-fixed): the room was
    // estimated once, the budget block's own size came out of it, and with
    // budget_chars 8,000 the cogito trial got a 4,800-character reply with 17
    // prose fields blanked — and a note naming an internal budget, down to "a
    // budget of 0". The room now grows by the slack the reply leaves, and the
    // note names the caller's budget.
    let kept_whole: usize = with_claims
        .iter()
        .map(|(k, v)| k.len() + 4 + chars_of(v))
        .sum();
    let mut room = budget.saturating_sub(chars_of(&claims) + chars_of(&page) + kept_whole + 32);
    let mut out = whole(room);
    for _ in 0..6 {
        let used = chars_of(&out);
        if used >= budget || room >= budget {
            break;
        }
        let grown = (room + (budget - used)).min(budget);
        let tried = whole(grown);
        if chars_of(&tried) > budget {
            break;
        }
        room = grown;
        out = tried;
    }
    while room > 0 && chars_of(&out) > budget {
        room = room.saturating_sub((chars_of(&out) - budget).max(64));
        out = whole(room);
    }
    let reply_chars = chars_of(&out);
    if let Some(b) = out.get_mut("budget").and_then(|b| b.as_object_mut()) {
        let inner = b
            .get("budget_chars")
            .and_then(|v| v.as_u64())
            .unwrap_or(room as u64);
        b.insert("budget_chars".into(), json!(budget));
        b.insert("summary_room".into(), json!(inner));
        b.insert("reply_chars".into(), json!(reply_chars));
        if let Some(note) = b.get("note").and_then(|n| n.as_str()) {
            let note = note.replacen(
                &format!("against a budget of {inner},"),
                &format!(
                    "outside its whole claims, against the {inner} characters the claims leave of \
                     budget_chars {budget},"
                ),
                1,
            );
            b.insert("note".into(), json!(note));
        }
    }
    out
}
