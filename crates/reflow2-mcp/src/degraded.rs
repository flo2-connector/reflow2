//! The server that starts when the graph cannot be opened — so a session that
//! has no design brain can find out **why**.
//!
//! From the StoryFlow fleet, 2026-07-25, measured from both sides of a lock. A
//! three-boss fleet pointed every session at one graph; the first to start won
//! the exclusive lock and the rest died at startup, before any tool existed. What
//! the losing sessions saw was not an error — it was *nothing*:
//!
//! ```text
//! reflow2: ✘ Failed to connect — -32000: MCP error -32000: Connection closed
//! ```
//!
//! Zero `reflow2__*` tools, none deferred, and — in the words of the api-boss who
//! wrote it up — *"nothing distinguished this from 'reflow2 was never configured
//! for this project'"*. reflow2's own excellent diagnosis ("another process
//! already has the design graph open… stop that server") went to stderr and died
//! with the process. Recovering it took hand-piping an `initialize` frame into the
//! binary, which is a diagnosis path no ordinary session will ever find.
//!
//! The same silence had already been reached from a different cause on 2026-07-24:
//! a graph refused for schema-version skew also exits at startup. Two causes, one
//! failure class — which is why the fix belongs here, at the plumbing, rather than
//! in each skill.
//!
//! **So: never exit silently.** Complete the MCP handshake, put the diagnosis in
//! the server instructions (which the agent reads as part of its context), and
//! serve exactly one tool whose name is unmistakable. An MCP server that starts
//! and explains itself beats one that dies before it can be asked.
//!
//! What this deliberately is NOT: a read-only mode. Serving the read tools from a
//! locked graph needs RocksDB's secondary-instance open, which lives one layer
//! down in dynograph-storage and is not exposed yet (`req:read-while-held`). This
//! costs nothing to ship and stops the outage being invisible today.
//!
//! ⭐ AND IT IS NOT TERMINAL WHEN ITS CAUSE CAN CLEAR (2026-09-28). A store held
//! by ANOTHER PROCESS is the one cause that clears by itself: the holder stops,
//! and the lock is free. Until 2026-09-28 this surface treated it like a corrupt
//! store — served the one tool forever, never re-tried, and (over HTTP) bound a
//! port that the image's TCP health check read as healthy. On a rolling update
//! the new pod lost the lock to the old one and stayed degraded after the old
//! one was gone (GitHub issue #616;
//! `fact:root-cause-a-server-degraded-by-a-held-lock-never-retries-and-looks-alive-2026-09-28`).
//!
//! So a surface built with [`DegradedService::recovering`] keeps trying to open
//! the store, with bounded backoff, and when it succeeds it serves the design
//! IN PLACE: the same process and port, the full surface for every session
//! (each with its own seat), `notifications/tools/list_changed` to every client
//! already connected, and readiness turned ready (`crate::readiness`). It is the
//! latent surface's promotion (`crate::latent`, 2026-09-14) applied to the other
//! stand-in surface, which is the class both belong to: a surface bound at
//! startup must re-check a cause that can clear.
//!
//! 🛑 WHAT IS NOT POLLED, ON PURPOSE: a cause no amount of waiting fixes — a
//! stamp that will not read, a store written by a newer reflow2, a corrupt or
//! unreadable store, an identity refusal. Those are built with
//! [`DegradedService::new`] and say "restart after fixing the cause", because
//! promising a recovery that cannot happen is its own silent failure. And if the
//! lock frees and the open THEN fails for such a reason, the surface stops
//! retrying and says that instead.

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    InitializeRequestParams, InitializeResult, ListToolsResult, PaginatedRequestParams,
    ServerCapabilities, ServerConfig,
};
use rmcp::service::{NotificationContext, Peer, RequestContext, RoleServer};
use rmcp::{ErrorData as McpError, ServerHandler, tool, tool_router};
use serde_json::json;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::Duration;

use crate::readiness::Readiness;
use crate::service::ReflowService;

/// A server with no graph, which knows exactly why and says so — and, when the
/// cause can clear, serves the design the moment it does.
#[derive(Clone)]
pub struct DegradedService {
    /// The plain-language reason the graph could not be opened — already
    /// translated for a human by `explain_open_failure`. Shared, because a
    /// recovering surface can learn a NEW reason (the lock freed and the open
    /// then failed for a reason waiting will not fix).
    reason: Arc<RwLock<String>>,
    /// The path that was attempted, so the reader can tell WHICH graph failed
    /// when a machine holds several.
    graph_path: String,
    tool_router: ToolRouter<Self>,
    /// `Some` when the cause can clear by itself and a task is retrying.
    recovery: Option<Arc<Recovery>>,
    /// THIS session's view of the design once it is served: its own `share()`
    /// of the recovered service, so it gets its own seat as every HTTP session
    /// of a healthy server does. Per clone on purpose — the HTTP transport
    /// clones a template per session, and the template is never promoted.
    session: OnceLock<ReflowService>,
}

/// Why an open failed, sorted by whether waiting can fix it.
#[derive(Debug)]
pub enum OpenFailure {
    /// Another process holds the store. Clears when it lets go.
    Held(String),
    /// Anything waiting will not fix: a stamp that will not read, a newer
    /// writer, a corrupt or unreadable store.
    Permanent(String),
}

/// Opens the design, the way the healthy start would have (read-only, tree
/// root, write-through export and all), or says why not.
pub type Opener = Arc<dyn Fn() -> Result<ReflowService, OpenFailure> + Send + Sync>;

/// The shortest and longest wait between attempts. Short first, because a
/// rolling update's old pod usually lets go within seconds; capped, because
/// every attempt is one failed open of a lock another process holds, which
/// costs a syscall and a log line, and a pod waiting an hour should not spin.
const FIRST_RETRY: Duration = Duration::from_millis(500);
const MAX_RETRY: Duration = Duration::from_secs(5);

/// The retrying half of a degraded surface: the design once opened, who to
/// tell, and what readiness says.
pub struct Recovery {
    graph_path: String,
    full: RwLock<Option<ReflowService>>,
    /// Every client connected while degraded, told when the list changes.
    peers: Mutex<Vec<Peer<RoleServer>>>,
    readiness: Arc<Readiness>,
}

impl Recovery {
    /// The design, once served.
    pub fn served(&self) -> Option<ReflowService> {
        self.full.read().ok().and_then(|g| g.clone())
    }

    /// Hand the design back for a stop to close, and forget it here, so
    /// nothing in this surface keeps its store open while the drain waits for
    /// it to be released (`crate::drain`).
    pub fn take(&self) -> Option<ReflowService> {
        self.full.write().ok().and_then(|mut g| g.take())
    }

    fn watch(&self, peer: Peer<RoleServer>) {
        if let Ok(mut peers) = self.peers.lock() {
            peers.retain(|p| !p.is_transport_closed());
            peers.push(peer);
        }
    }

    async fn promote(&self, service: ReflowService) {
        if let Ok(mut g) = self.full.write() {
            *g = Some(service);
        }
        self.readiness.set_ready();
        eprintln!(
            "reflow2: the design at {} is open now — the process that held it has let go. This \
             server serves it from here on, with no restart; connected sessions are told their \
             tool list changed.",
            self.graph_path
        );
        let peers: Vec<Peer<RoleServer>> = self
            .peers
            .lock()
            .map(|mut p| p.drain(..).collect())
            .unwrap_or_default();
        for peer in peers {
            if peer.is_transport_closed() {
                continue;
            }
            // Best-effort: a client that ignores notifications still gets the
            // full surface on its next tools/list.
            if let Err(e) = peer.notify_tool_list_changed().await {
                tracing::warn!("could not tell a client its tool list changed: {e}");
            }
        }
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NoArgs {}

#[tool_router(router = tool_router)]
impl DegradedService {
    /// A degraded surface whose cause cannot clear by itself: it explains, and
    /// says a restart after fixing the cause is what serves the design.
    pub fn new(reason: String, graph_path: String) -> Self {
        Self {
            reason: Arc::new(RwLock::new(reason)),
            graph_path,
            tool_router: Self::tool_router(),
            recovery: None,
            session: OnceLock::new(),
        }
    }

    /// A degraded surface whose cause CAN clear (another process holds the
    /// store): it explains, keeps trying `open` with bounded backoff, and
    /// serves the design in place when the open succeeds. Must be called on a
    /// tokio runtime — the retrying task starts here.
    pub fn recovering(
        reason: String,
        graph_path: String,
        readiness: Arc<Readiness>,
        open: Opener,
    ) -> Self {
        let recovery = Arc::new(Recovery {
            graph_path: graph_path.clone(),
            full: RwLock::new(None),
            peers: Mutex::new(Vec::new()),
            readiness,
        });
        let reason = Arc::new(RwLock::new(reason));
        tokio::spawn(retry(
            Arc::clone(&recovery),
            Arc::clone(&reason),
            graph_path.clone(),
            open,
        ));
        Self {
            reason,
            graph_path,
            tool_router: Self::tool_router(),
            recovery: Some(recovery),
            session: OnceLock::new(),
        }
    }

    /// A degraded surface whose cause CAN clear, but whose recovery is done by
    /// the CALLER rather than by opening the store here: the `--shared` client
    /// that found no shared server (a non-shared process holds the lock) keeps
    /// re-electing and switches the session over itself
    /// (`crate::proxy::run_waiting`). This surface only says so truthfully —
    /// the reason, that it serves the design in this session once it can, and
    /// the `list_changed` capability the caller will honour.
    pub fn awaiting(reason: String, graph_path: String) -> Self {
        Self {
            reason: Arc::new(RwLock::new(reason)),
            graph_path: graph_path.clone(),
            tool_router: Self::tool_router(),
            recovery: Some(Arc::new(Recovery {
                graph_path,
                full: RwLock::new(None),
                peers: Mutex::new(Vec::new()),
                readiness: Readiness::not_ready(crate::readiness::HELD_ELSEWHERE),
            })),
            session: OnceLock::new(),
        }
    }

    /// The retrying half, for a stop to close what it opened.
    pub fn recovery(&self) -> Option<Arc<Recovery>> {
        self.recovery.clone()
    }

    fn reason(&self) -> String {
        self.reason
            .read()
            .map(|r| r.clone())
            .unwrap_or_else(|_| "the design graph could not be opened".to_string())
    }

    /// This session's share of the design, once it is served.
    fn promoted(&self) -> Option<ReflowService> {
        if let Some(s) = self.session.get() {
            return Some(s.clone());
        }
        let full = self.recovery.as_ref()?.served()?;
        let _ = self.session.set(full.share());
        self.session.get().cloned()
    }

    /// The whole point: a tool whose NAME is the diagnosis.
    ///
    /// A session listing tools sees `reflow2_unavailable` and cannot mistake it
    /// for a configuration problem. Calling it returns the reason and the
    /// remedies — in-band, where an agent can act on them.
    #[tool(
        description = "reflow2 is UNAVAILABLE in this session and this is the only tool served. \
                       Call it for the reason and what to do about it. The design graph could not \
                       be opened — most often because another session holds it (the store is \
                       single-writer), or because it was written by a different reflow2. This tool \
                       existing at all means reflow2 IS configured here: do not report the design \
                       brain as missing or unconfigured.",
        annotations(read_only_hint = true)
    )]
    pub async fn reflow2_unavailable(
        &self,
        Parameters(_): Parameters<NoArgs>,
    ) -> Result<CallToolResult, McpError> {
        let payload = json!({
            "available": false,
            "graph_path": self.graph_path,
            "reason": self.reason(),
            "recovers_by_itself": self.recovery.is_some(),
            "what_this_means": "reflow2 is configured for this project but this session has no \
                                design graph. Every other reflow2 tool is absent for that reason \
                                alone — not because the project has no design.",
            "remedies": [
                "If another session holds the graph: that session is the writer. Either work \
                 through it, or take your own seat — copy the committed design export into your \
                 own graph path and merge back with the git merge driver (see the parallel-work \
                 skill and AGENTS.md).",
                "If the reason names a version or type mismatch: the graph was written by a \
                 different reflow2. Export it with the build that wrote it, or import a committed \
                 export into a fresh path — and move the sidecar `<graph-path>.meta.json` too, \
                 because the version gate is read from there.",
                "Either way: tell the user the reason above verbatim. It is the one thing they \
                 cannot get from inside this session."
            ],
            "do_not": "Do not report that reflow2 is not installed, not configured, or that the \
                       project has no design graph. All three would be false, and a design-first \
                       process would then be skipped for a reason nobody recorded."
        });
        let text = serde_json::to_string_pretty(&payload)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let mut result = CallToolResult::structured(payload);
        result.content = vec![ContentBlock::text(text)];
        Ok(result)
    }
}

/// Keep trying to open the store until it opens, or until the reason stops
/// being one that waiting can fix.
async fn retry(
    recovery: Arc<Recovery>,
    reason: Arc<RwLock<String>>,
    graph_path: String,
    open: Opener,
) {
    let mut wait = FIRST_RETRY;
    loop {
        tokio::time::sleep(wait).await;
        let attempt = Arc::clone(&open);
        // An open builds the search index, which on a large design takes
        // seconds: never on the runtime's own threads.
        match tokio::task::spawn_blocking(move || attempt()).await {
            Ok(Ok(service)) => {
                recovery.promote(service).await;
                return;
            }
            Ok(Err(OpenFailure::Held(_))) => {
                wait = (wait * 2).min(MAX_RETRY);
            }
            Ok(Err(OpenFailure::Permanent(why))) => {
                let now = format!(
                    "the process that held the design graph at {graph_path} let go, and opening \
                     it then failed for a reason waiting will not fix:\n\n{why}\n\nFix that \
                     and restart this server."
                );
                eprintln!("reflow2: {now}");
                if let Ok(mut r) = reason.write() {
                    *r = now;
                }
                recovery
                    .readiness
                    .set_not_ready(crate::readiness::CANNOT_OPEN);
                return;
            }
            Err(join) => {
                eprintln!(
                    "reflow2: stopped retrying the design at {graph_path} — the attempt itself \
                     failed ({join}). Restart this server to try again."
                );
                recovery
                    .readiness
                    .set_not_ready(crate::readiness::CANNOT_OPEN);
                return;
            }
        }
    }
}

/// The degraded tools until the design is served; the design's own surface
/// for every session from then on.
impl ServerHandler for DegradedService {
    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        // A session that connects AFTER the design is served is the design's
        // session from its first message, handshake record included.
        if let Some(full) = self.promoted() {
            return full.initialize(request, context).await;
        }
        context.peer.set_peer_info(request);
        Ok(self.get_info())
    }

    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        if let Some(recovery) = &self.recovery {
            recovery.watch(context.peer.clone());
        }
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        if let Some(full) = self.promoted() {
            return full.list_tools(request, context).await;
        }
        Ok(crate::tool_listing::tools_result(
            self.tool_router.list_all(),
            &context,
        ))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        if let Some(full) = self.promoted() {
            return full.call_tool(request, context).await;
        }
        let tcc = ToolCallContext::new(self, request, context);
        self.tool_router.call(tcc).await
    }

    fn get_info(&self) -> ServerConfig {
        if let Some(full) = self.promoted() {
            return full.get_info();
        }
        // The instructions are the real fix. A client puts them in the agent's
        // context at handshake time, so the reason arrives BEFORE the agent
        // wonders where the tools went — which is the difference between a
        // self-explaining outage and an invisible one.
        //
        // `list_changed` is DECLARED only by a surface that will send it: one
        // whose cause can clear, and which then serves the design.
        let capabilities = if self.recovery.is_some() {
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build()
        } else {
            ServerCapabilities::builder().enable_tools().build()
        };
        let what_next = if self.recovery.is_some() {
            "Another process holds this design, and that clears by itself: this server keeps \
             trying, and serves the full design IN THIS SESSION as soon as that process lets go \
             — the client is told its tool list changed, and nothing needs restarting. Until \
             then, tell the user the reason above verbatim, and either work through the session \
             that holds the graph or wait for it to finish."
        } else {
            "Tell the user the reason above verbatim, and if another session holds the graph, \
             either work through that session or take your own seat (own graph path + the git \
             merge driver; see the parallel-work skill)."
        };
        ServerConfig::new(capabilities)
            .with_server_info({
                let mut info = Implementation::from_build_env();
                info.name = env!("CARGO_PKG_NAME").to_string();
                info.version = env!("CARGO_PKG_VERSION").to_string();
                info
            })
            .with_instructions(format!(
                "reflow2 IS CONFIGURED FOR THIS PROJECT BUT UNAVAILABLE IN THIS SESSION. The \
                 design graph at {} could not be opened:\n\n{}\n\nOnly one tool is served \
                 (`reflow2_unavailable`); every other reflow2 tool is absent for this reason \
                 alone. DO NOT conclude that this project has no design graph, or that reflow2 is \
                 not installed — both would be false, and a design-first process would be skipped \
                 on a false premise. {}",
                self.graph_path,
                self.reason(),
                what_next
            ))
    }
}
