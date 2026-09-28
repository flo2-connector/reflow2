//! `/g/<graph_id>/` — the transport half of selecting a design per session.
//!
//! ⭐ WHAT THIS FINISHES. `registry.rs` has been the RESOLUTION half since
//! 2026-08-12: it maps a `graph_id` to a path under one root, refuses an unknown
//! id by name, and refuses a PATH as an unknown id so a filesystem route is not
//! a second way in. Nothing called it outside its own tests, so
//! `cap:select-graph-by-id` sat `in_progress` and two flo2 items were blocked on
//! it — measuring per-store cost needs one process holding two designs, and
//! proving isolation against the published image needs two designs in it.
//!
//! ⭐ THE CARRIER WAS ALREADY CHOSEN, and not by this module.
//! `cap:select-graph-by-id` names an HTTP path prefix as preferred "because the
//! selection is then visible in logs and routable by ordinary proxies", with an
//! MCP initialize parameter second and an attaching tool call worst — that last
//! "makes every session stateful in a way a reconnect silently loses, and it
//! leaves a window in which a session is connected to nothing". This implements
//! the first.
//!
//! ⭐ AND THE SECURITY PROPERTY WAS DECIDED BEFORE THE SURFACE EXISTED, which is
//! what `dec:the-registry-root-is-the-tenant-boundary` is for: **the registry
//! routes WITHIN a root and never across one.** There is no cross-root operation
//! to design, no filtered listing (that would need an identity system reflow2 has
//! twice refused in writing), and an operator serving several tenants gives each
//! its own root. Isolation is a property of WHAT IS THERE, not of a check on who
//! is asking.
//!
//! 🛑 ISOLATION HOLDS BY CONSTRUCTION HERE, NOT BY A CHECK, and that is
//! deliberate — flo2's own tracker warns that "a handler that reaches the wrong
//! graph corrupts a design rather than erroring, so this wants property tests,
//! not examples":
//!
//! - The only way to name a design is the `graph_id` segment, and it is handed
//!   straight to `Registry::attach`, which resolves it against the root. A path
//!   is refused as an unknown id, because `attach` treats its argument as an id
//!   and nothing else.
//! - A segment containing `/`, `\`, or `..` never reaches `attach` at all: it
//!   cannot survive being one path segment, and the traversal check below is
//!   belt to that brace.
//! - Each design gets its OWN `StreamableHttpService` with its OWN session
//!   manager, so a session id minted under `/g/A/` is unknown under `/g/B/`.
//!   Sessions cannot be carried across designs because they are not in the same
//!   table.
//!
//! ## Designs close when nobody is using them (since 2026-09-27)
//!
//! `req:a-hosted-server-closes-idle-designs-and-says-busy-when-full`. Until this
//! change an opened design stayed open until the process ended, so
//! `--registry-max-open` was a budget for the process LIFETIME, not a bound on
//! concurrent use. On flo2.io, with 11 designs and the default limit of 8, one
//! pass over every design (the upgrade script's own check) refused the last 3
//! with 404 until a restart
//! (`fact:flo2-io-met-the-open-design-cap-with-no-idle-eviction-2026-09-27`).
//! What every system serving many small single-owner objects does instead —
//! Orleans, Akka Cluster Sharding, Dapr actors, Durable Objects — is what this does:
//!
//! - **A design with no request for `--registry-idle` (default 15 minutes) is
//!   closed.** A background sweep does it, and so does every request that finds
//!   the table full, so an idle server still frees its memory.
//! - **At the limit, the least recently used design with nothing in flight is
//!   closed to make room**, rather than the newcomer being refused. The refusal
//!   is left for the one case it is right: every open design is serving a request
//!   at this moment.
//! - **That refusal is `503 Service Unavailable` with `Retry-After`, never 404.**
//!   404 tells a client the design does not exist, and a client that believes it
//!   will not come back.
//! - **Closing ends every session held on the design.** A client on an older
//!   MCP revision then meets 404 for its session and re-initializes, which the
//!   spec requires and flo2's gateway already does; a 2026-07-28 client has no
//!   session to lose.
//! - 🛑 **A design is never reopened until its previous copy has released its
//!   store.** Closing drops the router's hold, but a request still streaming its
//!   answer can hold the store a moment longer, and RocksDB refuses a second open
//!   of a store this same process holds. The reopen therefore waits on a `Weak`
//!   to the old store and proceeds only when nothing holds it.
//!
//! ## What this increment does NOT do, named rather than left to be discovered
//!
//! `dec:one-process-many-stores` lists five things that must change for hosted
//! multi-graph. The registry lifecycle, the selection carrier and closing idle
//! designs are done; these are not:
//!
//! - **No per-graph rendezvous.** A multi-graph server does not publish itself
//!   into each held graph's sidecar, so a local `--shared` session opening one of
//!   those directories will not find it and will start its own daemon. That is
//!   the single-graph path working as it always has, not a regression, but it
//!   means the two modes do not yet cooperate.
//! - **`--import` / `--export` still open the store directly** and so still
//!   require the server to release it.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use reflow2_core::DesignGraph;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tokio::sync::{Mutex, OnceCell, RwLock};

use crate::registry::Registry;
use crate::service::ReflowService;

/// What `StreamableHttpService` answers with. Spelled out because rmcp keeps its
/// own `BoxResponse` alias `pub(crate)`; the shape is part of its public
/// `tower::Service` impl either way.
type BoxResponse = Response<BoxBody<Bytes, std::convert::Infallible>>;

type GraphService = StreamableHttpService<ReflowService, LocalSessionManager>;

/// How long a design may go without a request before it is closed, unless the
/// operator says otherwise (`--registry-idle`). Fifteen minutes is Orleans'
/// default and suits a design whose reopen rebuilds a full-text index: short
/// enough that memory follows the designs actually in use, long enough that a
/// person pausing to think does not pay a reopen on their next call.
pub const DEFAULT_IDLE: Duration = Duration::from_secs(15 * 60);

/// How long a failed open is remembered before the next request tries again.
///
/// A FAILED OPEN IS STILL CACHED, as it always was, so a design whose store is
/// held by another process does not pay the full open on every call — but no
/// longer forever: another process's lock is usually released, and a design
/// that could not open at 09:00 should not stay refused until a restart.
const FAILED_OPEN_RETRY: Duration = Duration::from_secs(30);

/// How long a reopen waits for the previous copy of the same design to release
/// its store before it gives up and answers "busy".
const RELEASE_WAIT: Duration = Duration::from_secs(30);

/// What a 503 tells the client to wait before trying again, in seconds.
const RETRY_AFTER_SECS: u64 = 5;

/// Why the router did not hand a request to a design. Each maps to one status,
/// because the status is what a client acts on and the body is what a person reads.
#[derive(Clone, Debug)]
pub enum Refusal {
    /// No design by that id under this root, or the id is not an id — 404.
    NotADesign(String),
    /// Every slot is serving a request, or the previous copy has not yet
    /// released its store — 503 with `Retry-After`. Transient by construction.
    Busy(String),
    /// The store would not open (held by another process, unreadable) — 503.
    /// Remembered for [`FAILED_OPEN_RETRY`], then tried again.
    CouldNotOpen(String),
}

impl Refusal {
    fn status(&self) -> StatusCode {
        match self {
            Refusal::NotADesign(_) => StatusCode::NOT_FOUND,
            Refusal::Busy(_) | Refusal::CouldNotOpen(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn reason(&self) -> &str {
        match self {
            Refusal::NotADesign(r) | Refusal::Busy(r) | Refusal::CouldNotOpen(r) => r,
        }
    }
}

/// One design, open.
struct OpenDesign {
    http: GraphService,
    /// Kept so that closing the design can end every session held on it.
    sessions: Arc<LocalSessionManager>,
    /// Whether this copy still holds its store. Upgrading fails once every
    /// request and session that held it has let go — which is the moment the
    /// same design may safely be opened again in this process.
    store: Weak<RwLock<DesignGraph>>,
}

/// One design, open or being opened, or the reason it could not be.
type Opened = Result<Arc<OpenDesign>, Refusal>;

/// A design's place in the table.
struct Slot {
    /// ⭐ A CELL PER ID RATHER THAN A LOCK ACROSS THE MAP. A cold open builds
    /// the full-text index and takes SECONDS on a large design, so holding one
    /// lock across it would stall every OTHER design's requests behind an
    /// unrelated open. `OnceCell` also collapses concurrent first requests for
    /// the SAME id into one open, which `dec:one-process-many-stores` asks for
    /// by name: "concurrent first requests for the same graph must collapse into
    /// one open rather than racing".
    cell: OnceCell<Opened>,
    /// When a request last finished (or started) here, in ms since the router
    /// started. What "idle" and "least recently used" are measured by.
    last_used_ms: AtomicU64,
    /// Requests in progress here. A slot with any is never closed.
    in_flight: AtomicUsize,
    /// When the open failed, in ms since the router started; 0 if it has not.
    failed_ms: AtomicU64,
}

impl Slot {
    fn new(now_ms: u64) -> Self {
        Slot {
            cell: OnceCell::new(),
            last_used_ms: AtomicU64::new(now_ms),
            in_flight: AtomicUsize::new(0),
            failed_ms: AtomicU64::new(0),
        }
    }
}

/// Held for the length of one request: keeps its design from being closed
/// under it, and marks the design used when the request ends.
struct InFlight {
    slot: Arc<Slot>,
    epoch: Instant,
}

impl Drop for InFlight {
    fn drop(&mut self) {
        self.slot
            .last_used_ms
            .store(ms_since(self.epoch), Ordering::SeqCst);
        self.slot.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

fn ms_since(epoch: Instant) -> u64 {
    u64::try_from(epoch.elapsed().as_millis()).unwrap_or(u64::MAX)
}

struct Inner {
    root: String,
    read_only: bool,
    max_open: usize,
    /// `None` means designs are never closed for idleness (`--registry-idle 0`).
    idle: Option<Duration>,
    config: StreamableHttpServerConfig,
    epoch: Instant,
    /// `graph_id` -> its slot. Held only for bookkeeping, never across an open.
    open: Mutex<HashMap<String, Arc<Slot>>>,
    /// Designs closed whose previous copy may still hold its store, by id.
    releasing: std::sync::Mutex<HashMap<String, Weak<RwLock<DesignGraph>>>>,
    /// Set once the server has begun to stop: nothing is opened after that.
    stopping: AtomicBool,
}

/// Serves many designs under one root, selected by `/g/<graph_id>/`.
#[derive(Clone)]
pub struct GraphRouter {
    inner: Arc<Inner>,
}

impl GraphRouter {
    /// A router over `root`. `idle` of `None` never closes a design for
    /// idleness; the limit still closes the least recently used one when full.
    ///
    /// Inside a tokio runtime this also starts the idle sweep, which holds the
    /// router only weakly and so ends with it.
    pub fn new(
        root: String,
        read_only: bool,
        max_open: usize,
        idle: Option<Duration>,
        config: StreamableHttpServerConfig,
    ) -> Self {
        let router = Self {
            inner: Arc::new(Inner {
                root,
                read_only,
                max_open,
                idle,
                config,
                epoch: Instant::now(),
                open: Mutex::new(HashMap::new()),
                releasing: std::sync::Mutex::new(HashMap::new()),
                stopping: AtomicBool::new(false),
            }),
        };
        router.spawn_idle_sweep();
        router
    }

    fn spawn_idle_sweep(&self) {
        let Some(idle) = self.inner.idle else { return };
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let weak = Arc::downgrade(&self.inner);
        let period = (idle / 4).clamp(Duration::from_secs(1), Duration::from_secs(60));
        runtime.spawn(async move {
            let mut tick = tokio::time::interval(period);
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tick.tick().await;
                let Some(inner) = weak.upgrade() else { break };
                GraphRouter { inner }.sweep().await;
            }
        });
    }

    /// The designs under this root, rediscovered on each call.
    ///
    /// NOT CACHED, on purpose: a design created after the server started should
    /// be reachable without a restart, and `discover` is a directory read.
    pub fn graph_ids(&self) -> Vec<String> {
        Registry::discover(&self.inner.root).graph_ids()
    }

    /// Resolve `graph_id` against a fresh read of the root. `attach` is the ONLY
    /// way in; when the id is unknown and the root holds stores it cannot serve,
    /// the refusal says so, so a design that is here and broken is never
    /// reported as simply "not here".
    fn resolve(&self, graph_id: &str) -> Result<crate::registry::Binding, Refusal> {
        let registry = Registry::discover(&self.inner.root);
        registry.attach(graph_id).map_err(|e| {
            let mut why = e.to_string();
            if matches!(e, crate::registry::AttachError::UnknownGraphId { .. })
                && let Some(note) = registry.unserved_note()
            {
                why.push(' ');
                why.push_str(&note);
            }
            Refusal::NotADesign(why)
        })
    }

    /// How many designs this router holds a place for right now.
    pub async fn open_count(&self) -> usize {
        self.inner.open.lock().await.len()
    }

    /// Split `/g/<id>/rest` into the id and what the inner service should see.
    ///
    /// Returns `None` for any path that is not under `/g/`, so the router can
    /// say what would have worked instead of serving a design nobody named.
    fn split(path: &str) -> Option<(String, String)> {
        let rest = path.strip_prefix("/g/")?;
        let (id, tail) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        if id.is_empty() {
            return None;
        }
        Some((
            id.to_string(),
            if tail.is_empty() {
                "/".to_string()
            } else {
                tail.to_string()
            },
        ))
    }

    fn now_ms(&self) -> u64 {
        ms_since(self.inner.epoch)
    }

    /// Close every design idle past the timeout. Returns the ids it closed.
    pub async fn sweep(&self) -> Vec<String> {
        let now = self.now_ms();
        let victims = {
            let mut map = self.inner.open.lock().await;
            self.take_idle(&mut map, now)
        };
        let ids = victims.iter().map(|(id, _)| id.clone()).collect();
        self.close(victims, "idle").await;
        ids
    }

    /// Remove, and hand back for closing, every slot with nothing in flight
    /// that has been idle past the timeout, or whose failed open is old enough
    /// to be tried again.
    fn take_idle(
        &self,
        map: &mut HashMap<String, Arc<Slot>>,
        now: u64,
    ) -> Vec<(String, Arc<Slot>)> {
        let idle_ms = self
            .inner
            .idle
            .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        let retry_ms = u64::try_from(FAILED_OPEN_RETRY.as_millis()).unwrap_or(u64::MAX);
        let due: Vec<String> = map
            .iter()
            .filter(|(_, s)| s.in_flight.load(Ordering::SeqCst) == 0)
            .filter(|(_, s)| {
                let failed = s.failed_ms.load(Ordering::SeqCst);
                if failed > 0 {
                    return now.saturating_sub(failed) >= retry_ms;
                }
                idle_ms.is_some_and(|limit| {
                    now.saturating_sub(s.last_used_ms.load(Ordering::SeqCst)) >= limit
                })
            })
            .map(|(id, _)| id.clone())
            .collect();
        due.into_iter()
            .filter_map(|id| map.remove(&id).map(|s| (id, s)))
            .collect()
    }

    /// Close every design this router holds and open nothing more: what a
    /// stopping server calls (`req:a-server-drains-before-it-stops`).
    ///
    /// Returns each design closed, the store to wait on, and how many sessions
    /// it ended. A request that arrives from here on is answered "busy" rather
    /// than reopening a design behind the stop.
    pub async fn close_all(&self, why: &str) -> Vec<(String, Weak<RwLock<DesignGraph>>, usize)> {
        self.inner.stopping.store(true, Ordering::SeqCst);
        let victims: Vec<_> = self.inner.open.lock().await.drain().collect();
        self.close(victims, why).await
    }

    /// End every session on each design and let go of it. The store closes when
    /// the last request holding it finishes; until then a reopen waits.
    ///
    /// Returns each design it closed, with its store and the sessions it ended.
    async fn close(
        &self,
        victims: Vec<(String, Arc<Slot>)>,
        why: &str,
    ) -> Vec<(String, Weak<RwLock<DesignGraph>>, usize)> {
        let mut closed = Vec::new();
        for (id, slot) in victims {
            let Some(Ok(open)) = slot.cell.get() else {
                continue;
            };
            let ended = crate::drain::end_sessions(&open.sessions).await;
            self.inner
                .releasing
                .lock()
                .expect("the releasing table is never poisoned: nothing panics holding it")
                .insert(id.clone(), open.store.clone());
            eprintln!("reflow2: closed design {id} ({why}); {ended} session(s) ended.");
            closed.push((id, open.store.clone(), ended));
        }
        closed
    }

    /// The design a request names, opened if it is not, and a guard that keeps
    /// it open until the request ends.
    async fn service_for(&self, graph_id: &str) -> Result<(Arc<OpenDesign>, InFlight), Refusal> {
        // ⚠️ BELT TO THE BRACE. A `graph_id` arrives as ONE path segment, so it
        // cannot contain `/` and `..` alone names no design under the root. This
        // refuses the shapes anyway rather than relying on that argument holding
        // forever — the cost of being wrong here is a handler reaching another
        // design, which corrupts rather than errors.
        if graph_id.contains('/') || graph_id.contains('\\') || graph_id == ".." || graph_id == "."
        {
            return Err(Refusal::NotADesign(format!(
                "'{graph_id}' is not a design id. Designs are named by graph_id, never by path — \
                 the registry maps the id to a path under its root, and a path is refused exactly \
                 as an unknown id is."
            )));
        }

        if self.inner.stopping.load(Ordering::SeqCst) {
            return Err(Refusal::Busy(
                "this server is stopping. Try again in a few seconds, when it or its replacement \
                 is up."
                    .to_string(),
            ));
        }

        let now = self.now_ms();
        let (slot, guard, victims) = {
            let mut map = self.inner.open.lock().await;
            let mut victims = self.take_idle(&mut map, now);
            let slot = match map.get(graph_id) {
                Some(slot) => Arc::clone(slot),
                None => {
                    // Resolve the id BEFORE taking a place, so a request naming
                    // no design can never push a real one out.
                    self.resolve(graph_id)?;
                    if map.len() >= self.inner.max_open {
                        let Some(lru) = least_recently_used(&map) else {
                            return Err(Refusal::Busy(format!(
                                "this server holds its limit of {} open design(s) and every one \
                                 is serving a request right now — '{graph_id}' was not opened. \
                                 Try again in a few seconds; if this keeps happening, raise \
                                 --registry-max-open or run a second server.",
                                self.inner.max_open
                            )));
                        };
                        if let Some(s) = map.remove(&lru) {
                            victims.push((lru, s));
                        }
                    }
                    let slot = Arc::new(Slot::new(now));
                    map.insert(graph_id.to_string(), Arc::clone(&slot));
                    slot
                }
            };
            slot.in_flight.fetch_add(1, Ordering::SeqCst);
            slot.last_used_ms.store(now, Ordering::SeqCst);
            let guard = InFlight {
                slot: Arc::clone(&slot),
                epoch: self.inner.epoch,
            };
            (slot, guard, victims)
        };
        if !victims.is_empty() {
            self.close(victims, &format!("to make room for {graph_id}"))
                .await;
        }

        let opened = slot
            .cell
            .get_or_init(|| self.open_design(graph_id))
            .await
            .clone();
        match opened {
            Ok(open) => Ok((open, guard)),
            Err(Refusal::Busy(why)) => {
                // Transient: do not remember it. Drop this slot so the next
                // request starts a fresh open.
                drop(guard);
                let mut map = self.inner.open.lock().await;
                if map.get(graph_id).is_some_and(|s| Arc::ptr_eq(s, &slot)) {
                    map.remove(graph_id);
                }
                Err(Refusal::Busy(why))
            }
            Err(other) => {
                let _ = slot.failed_ms.compare_exchange(
                    0,
                    self.now_ms().max(1),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                );
                Err(other)
            }
        }
    }

    async fn open_design(&self, graph_id: &str) -> Opened {
        // 🛑 NEVER TWO COPIES OF ONE STORE. If this design was closed a moment
        // ago, a request still finishing may hold the old copy's store, and
        // RocksDB refuses a second open of a store this process holds. Wait for
        // the old copy to let go.
        let previous = self
            .inner
            .releasing
            .lock()
            .expect("the releasing table is never poisoned: nothing panics holding it")
            .get(graph_id)
            .cloned();
        if let Some(previous) = previous {
            let deadline = Instant::now() + RELEASE_WAIT;
            while previous.strong_count() > 0 {
                if Instant::now() >= deadline {
                    return Err(Refusal::Busy(format!(
                        "design '{graph_id}' was closed and a request is still finishing on it; \
                         it will reopen once that request ends. Try again in a few seconds."
                    )));
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            self.inner
                .releasing
                .lock()
                .expect("the releasing table is never poisoned: nothing panics holding it")
                .remove(graph_id);
        }

        // Resolve id -> path against the root. `attach` is the ONLY way in.
        let binding = self.resolve(graph_id)?;
        let path = binding.graph_path().to_string();

        // ⚠️ OPENING IS BLOCKING AND SLOW — RocksDB plus a full-text index
        // rebuild, seconds on a large design. On the async runtime that
        // would stall every other connection this process is serving, which
        // is the whole point of holding many designs in one process.
        let opened = tokio::task::spawn_blocking(move || {
            ReflowService::new_reporting(&path).map(|(svc, _prov)| svc)
        })
        .await
        .map_err(|e| Refusal::CouldNotOpen(format!("the open task failed: {e}")))?
        .map_err(|e| Refusal::CouldNotOpen(format!("{e}")))?;

        // A registry holds DESIGNS, not checkouts: the directory a store
        // sits under is not the tree its artifacts describe, so a
        // measurement here would find every file absent and call it
        // missing. Say "not on this machine" instead (`crate::measure`).
        //
        // And it holds OTHER PEOPLE'S designs, so it never goes out over
        // the network on a caller's say-so: a watch declared at an address
        // is recorded, and fetched only by the person's own client
        // (`ReflowService::reaches_out`).
        let opened = opened.without_tree().without_reaching_out();
        let svc = if self.inner.read_only {
            opened.into_read_only()
        } else {
            opened
        };
        let store = Arc::downgrade(&svc.graph);
        let sessions = Arc::new(LocalSessionManager::default());
        Ok(Arc::new(OpenDesign {
            http: StreamableHttpService::new(
                move || Ok(svc.share()),
                Arc::clone(&sessions),
                self.inner.config.clone(),
            ),
            sessions,
            store,
        }))
    }
}

/// The open design used least recently that has nothing in flight, if any.
fn least_recently_used(map: &HashMap<String, Arc<Slot>>) -> Option<String> {
    map.iter()
        .filter(|(_, s)| s.in_flight.load(Ordering::SeqCst) == 0)
        .min_by_key(|(_, s)| s.last_used_ms.load(Ordering::SeqCst))
        .map(|(id, _)| id.clone())
}

/// The answer to a request the router would not hand to a design: the status a
/// client acts on, a `Retry-After` when waiting will help, and a sentence a
/// person can act on.
fn refused(graph_id: &str, refusal: &Refusal) -> BoxResponse {
    let status = refusal.status();
    let body = match refusal {
        Refusal::NotADesign(why) => {
            format!("reflow2 could not attach to design '{graph_id}': {why}\n")
        }
        Refusal::Busy(_) | Refusal::CouldNotOpen(_) => format!(
            "reflow2 could not serve design '{graph_id}' right now: {}\n",
            refusal.reason()
        ),
    };
    let mut response = text(status, body);
    if status == StatusCode::SERVICE_UNAVAILABLE {
        let wait = match refusal {
            Refusal::CouldNotOpen(_) => FAILED_OPEN_RETRY.as_secs(),
            _ => RETRY_AFTER_SECS,
        };
        response
            .headers_mut()
            .insert(http::header::RETRY_AFTER, http::HeaderValue::from(wait));
    }
    response
}

fn text(status: StatusCode, body: String) -> BoxResponse {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from(body)).boxed())
        .expect("a static text response is always well-formed")
}

impl<B> tower_service::Service<Request<B>> for GraphRouter
where
    B: http_body::Body + Send + 'static,
    B::Error: std::fmt::Display,
    B::Data: Send + 'static,
{
    type Response = BoxResponse;
    type Error = std::convert::Infallible;
    type Future = std::pin::Pin<
        Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>,
    >;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let this = self.clone();
        Box::pin(async move {
            let path = req.uri().path().to_string();

            let Some((graph_id, tail)) = GraphRouter::split(&path) else {
                // RULE 4 — SAY WHAT WOULD HAVE WORKED. A bare 404 here is the
                // failure this repo keeps meeting: a session that cannot tell
                // "reflow2 is not here" from "you addressed it wrongly".
                let registry = Registry::discover(&this.inner.root);
                let ids = registry.graph_ids();
                let unserved = registry.unserved();
                // "No designs" is said only when nothing at all is here: a
                // store found and not served is named with why, never
                // discarded into an empty listing (GitHub issue #616).
                let mut known = match (ids.is_empty(), unserved.is_empty()) {
                    (true, true) => "This server's registry root holds no designs.".to_string(),
                    (true, false) => {
                        "This server's registry root holds no design it can serve.".to_string()
                    }
                    (false, _) => format!("Designs under this root: {}.", ids.join(", ")),
                };
                if !unserved.is_empty() {
                    known.push_str(&format!(
                        "\n\nFound under this root and NOT served ({}):",
                        unserved.len()
                    ));
                    for u in unserved {
                        known.push_str("\n  · ");
                        known.push_str(&u.sentence());
                    }
                }
                return Ok(text(
                    StatusCode::NOT_FOUND,
                    format!(
                        "reflow2 is serving SEVERAL designs here, so a request must name which \
                         one: POST to /g/<graph_id>/ rather than to {path}.\n\n{known}\n\nA design \
                         is named by its graph_id, never by a filesystem path.\n"
                    ),
                ));
            };

            match this.service_for(&graph_id).await {
                Ok((open, in_flight)) => {
                    // Strip the prefix so the inner service sees the path it
                    // would have seen as a single-graph server. Everything else
                    // about the request — method, headers, body, the session id
                    // — passes through untouched.
                    let (mut parts, body) = req.into_parts();
                    let query = parts
                        .uri
                        .query()
                        .map(|q| format!("?{q}"))
                        .unwrap_or_default();
                    parts.uri = format!("{tail}{query}")
                        .parse()
                        .unwrap_or_else(|_| "/".parse().expect("'/' is a valid URI"));
                    let inner = Request::from_parts(parts, body);
                    let mut svc = open.http.clone();
                    let answer = tower_service::Service::call(&mut svc, inner).await;
                    // The design counts as in use until its answer is on its
                    // way — only then may it be closed.
                    drop(in_flight);
                    answer
                }
                Err(refusal) => Ok(refused(&graph_id, &refusal)),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{GraphRouter, Refusal, refused};
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    /// A registry root holding `n` real designs, and their ids in order.
    ///
    /// Each design is minted by opening its store once and letting go, which is
    /// what writes the identity sidecar `Registry::discover` reads.
    fn root_with(n: usize) -> (PathBuf, Vec<String>) {
        let root = std::env::temp_dir().join(format!(
            "reflow2-registry-lifecycle-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut ids = Vec::new();
        for i in 0..n {
            let store = root
                .join(format!("design-{i}"))
                .join(".reflow2")
                .join("graph");
            std::fs::create_dir_all(&store).unwrap();
            let path = store.to_str().unwrap().to_string();
            drop(crate::service::ReflowService::new_reporting(&path).expect("mint a design"));
            ids.push(graph_id_of(&root.join(format!("design-{i}"))));
        }
        (root, ids)
    }

    fn graph_id_of(dir: &Path) -> String {
        let raw = std::fs::read_to_string(dir.join(".reflow2").join("graph.id.json"))
            .expect("an opened design has an identity sidecar");
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        v["graph_id"].as_str().unwrap().to_string()
    }

    fn router(root: &Path, max_open: usize, idle: Option<Duration>) -> GraphRouter {
        GraphRouter::new(
            root.to_str().unwrap().to_string(),
            false,
            max_open,
            idle,
            Default::default(),
        )
    }

    async fn holds(router: &GraphRouter, id: &str) -> bool {
        router.inner.open.lock().await.contains_key(id)
    }

    #[tokio::test]
    async fn a_full_router_closes_the_least_recently_used_design_instead_of_refusing() {
        let (root, ids) = root_with(3);
        let r = router(&root, 2, None);
        for id in &ids {
            let opened = r.service_for(id).await;
            assert!(
                opened.is_ok(),
                "{id} must open, not be refused: {:?}",
                opened.err()
            );
        }
        assert_eq!(
            r.open_count().await,
            2,
            "the limit still bounds what is held"
        );
        assert!(
            !holds(&r, &ids[0]).await,
            "the least recently used design was closed"
        );
        assert!(holds(&r, &ids[2]).await, "and the newcomer took its place");
        // Closed is not gone: it reopens on its next request, in this same
        // process, which is the collision RocksDB would refuse if the old copy
        // were still holding the store.
        assert!(
            r.service_for(&ids[0]).await.is_ok(),
            "a closed design reopens"
        );
    }

    #[tokio::test]
    async fn busy_is_said_only_when_every_open_design_is_serving_a_request() {
        let (root, ids) = root_with(2);
        let r = router(&root, 1, None);
        let held = r
            .service_for(&ids[0])
            .await
            .expect("the first design opens");
        let refusal = r
            .service_for(&ids[1])
            .await
            .err()
            .expect("with the only slot serving a request, the second must wait");
        assert!(matches!(refusal, Refusal::Busy(_)), "{refusal:?}");
        let answer = refused(&ids[1], &refusal);
        assert_eq!(answer.status(), http::StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            answer
                .headers()
                .get(http::header::RETRY_AFTER)
                .map(|v| v.to_str().unwrap()),
            Some("5"),
            "a busy server says when to come back"
        );
        drop(held);
        assert!(
            r.service_for(&ids[1]).await.is_ok(),
            "once the request ends, the slot is reused rather than refused"
        );
    }

    #[tokio::test]
    async fn a_design_is_not_reopened_until_its_previous_copy_lets_go() {
        let (root, ids) = root_with(2);
        let r = router(&root, 1, None);
        let (old_copy, in_flight) = r.service_for(&ids[0]).await.expect("opens");
        // The request is over, but its answer still holds the design — as a
        // streaming response does after its headers are sent.
        drop(in_flight);
        drop(
            r.service_for(&ids[1])
                .await
                .expect("closes the first to make room"),
        );
        assert!(!holds(&r, &ids[0]).await);

        let released_at = Instant::now() + Duration::from_millis(300);
        tokio::spawn(async move {
            tokio::time::sleep_until(released_at.into()).await;
            drop(old_copy);
        });
        let reopened = r.service_for(&ids[0]).await;
        assert!(
            reopened.is_ok(),
            "the reopen waits for the old copy and then succeeds: {:?}",
            reopened.err()
        );
        assert!(
            Instant::now() >= released_at,
            "it must not have opened a second copy while the first still held the store"
        );
    }

    #[tokio::test]
    async fn an_idle_design_is_closed_and_one_serving_a_request_is_not() {
        let (root, ids) = root_with(2);
        let r = router(&root, 8, Some(Duration::from_millis(100)));
        drop(r.service_for(&ids[0]).await.expect("opens"));
        let busy = r.service_for(&ids[1]).await.expect("opens");
        tokio::time::sleep(Duration::from_millis(250)).await;
        let closed = r.sweep().await;
        assert_eq!(closed, vec![ids[0].clone()], "only the idle one is closed");
        assert!(
            holds(&r, &ids[1]).await,
            "a design serving a request is never closed"
        );
        drop(busy);
    }

    #[tokio::test]
    async fn a_stopping_router_closes_every_design_and_opens_no_more() {
        let (root, ids) = root_with(2);
        let r = router(&root, 8, None);
        for id in &ids {
            drop(r.service_for(id).await.expect("opens"));
        }
        let closed = r.close_all("the server is stopping").await;
        let mut names: Vec<_> = closed.iter().map(|(id, _, _)| id.clone()).collect();
        names.sort();
        let mut want = ids.clone();
        want.sort();
        assert_eq!(names, want, "every open design is closed");
        let deadline = Instant::now() + Duration::from_secs(5);
        for (id, store, _) in &closed {
            while store.strong_count() > 0 {
                assert!(
                    Instant::now() < deadline,
                    "{id}'s store is released once nothing is serving it"
                );
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
        let refusal = r
            .service_for(&ids[0])
            .await
            .err()
            .expect("a stopping server opens nothing");
        assert!(matches!(refusal, Refusal::Busy(_)), "{refusal:?}");
        assert_eq!(
            refused(&ids[0], &refusal).status(),
            http::StatusCode::SERVICE_UNAVAILABLE,
            "busy, so a client tries again rather than believing the design is gone"
        );
    }

    #[tokio::test]
    async fn no_idle_timeout_means_nothing_is_closed_for_idleness() {
        let (root, ids) = root_with(1);
        let r = router(&root, 8, None);
        drop(r.service_for(&ids[0]).await.expect("opens"));
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(r.sweep().await.is_empty());
        assert!(holds(&r, &ids[0]).await);
    }

    #[tokio::test]
    async fn a_name_that_is_no_design_takes_no_place_and_is_not_found() {
        let (root, ids) = root_with(1);
        let r = router(&root, 1, None);
        let held = r.service_for(&ids[0]).await.expect("opens");
        let refusal = r
            .service_for("no-such-design")
            .await
            .err()
            .expect("an unknown id is refused");
        assert!(matches!(refusal, Refusal::NotADesign(_)), "{refusal:?}");
        assert_eq!(
            refused("no-such-design", &refusal).status(),
            http::StatusCode::NOT_FOUND,
            "a design that does not exist is still 404 — only capacity is 503"
        );
        assert!(holds(&r, &ids[0]).await, "and it pushed no real design out");
        drop(held);
    }

    #[test]
    fn a_design_is_named_by_one_segment_and_the_rest_is_passed_through() {
        assert_eq!(
            GraphRouter::split("/g/reflow2/"),
            Some(("reflow2".into(), "/".into()))
        );
        assert_eq!(
            GraphRouter::split("/g/reflow2"),
            Some(("reflow2".into(), "/".into())),
            "a bare id with no trailing slash still names the design"
        );
        assert_eq!(
            GraphRouter::split("/g/flo2/message"),
            Some(("flo2".into(), "/message".into())),
            "and the inner service sees the path it would have seen alone"
        );
    }

    #[test]
    fn a_path_that_names_no_design_is_not_routed_to_one() {
        assert_eq!(GraphRouter::split("/"), None);
        assert_eq!(GraphRouter::split("/message"), None);
        assert_eq!(GraphRouter::split("/g/"), None, "an empty id names nothing");
        assert_eq!(
            GraphRouter::split("/gg/reflow2/"),
            None,
            "the prefix is exact — a near miss must not resolve"
        );
    }

    #[test]
    fn traversal_cannot_be_smuggled_through_the_id_segment() {
        // `..` as a whole segment is the only traversal that survives being one
        // segment at all, and it names no design under the root. The service
        // refuses it before `attach` ever sees it; this pins the parse half.
        assert_eq!(
            GraphRouter::split("/g/../etc/passwd"),
            Some(("..".into(), "/etc/passwd".into())),
            "it parses as an id of '..' — which service_for refuses by name, \
             rather than being silently resolved as a path"
        );
    }
}
