//! A server told to stop drains before it exits.
//!
//! `req:a-server-drains-before-it-stops` (`cap:a-server-drains-before-it-stops`),
//! on @ajs's word, 2026-09-27: "Yes, let's build the now items in reflow2".
//!
//! # What a stop was before this
//!
//! An HTTP server had no handler for SIGTERM or SIGINT, so a stop was the
//! process dying wherever it happened to be: a write cut part-way, every open
//! store closed by the kernel rather than by its owner, and a write-through
//! export still waiting out its quiet period lost with the process. Inside a
//! container it failed the other way round. The image `exec`s the server, so it
//! runs as process 1, and process 1 IGNORES a signal it has no handler for:
//! `docker stop` waited out its full ten seconds and then killed the server
//! outright. A shared server (`--serve-shared`) did catch SIGTERM, but only to
//! remove its rendezvous and `exit(0)` on the spot.
//!
//! # The order, and why it is this order
//!
//! 1. **Stop accepting.** The listener is dropped, so a new connection is
//!    refused at once instead of being accepted into a server that is leaving.
//! 2. **Every open connection finishes the exchange it is in, then closes.** An
//!    idle keep-alive connection closes now; one mid-request closes after its
//!    answer.
//! 3. **Work in progress gets up to the grace period to finish**
//!    (`--shutdown-grace`, [`DEFAULT_GRACE`]). Work is every request that is not
//!    a GET, counted from its arrival until its answer has been written out.
//! 4. **Sessions and streams end** (rmcp's cancellation token), including the
//!    long-lived GET streams step 3 does not wait for.
//! 5. **A write-through export still waiting is written now**, instead of being
//!    lost with the process.
//! 6. **Every open design is closed and its store released**, within
//!    [`CLOSE_BOUND`].
//! 7. **The server exits**, and says what it did.
//!
//! # Why a GET is not work
//!
//! In MCP's streamable HTTP transport a GET opens the standalone stream a server
//! uses to reach a client unprompted. It stays open as long as the session
//! does, so counting it would make every stop wait out the whole grace period
//! for nothing. A tool call is a POST, and a POST's answer stream ends once the
//! answer is sent: that is the work a stop must not cut in half.
//!
//! # The grace belongs below the supervisor's stop timeout
//!
//! Docker allows 10 s between SIGTERM and SIGKILL by default, Kubernetes 30 s,
//! systemd 90 s. The default grace plus [`CLOSE_BOUND`] fits inside Docker's
//! default; an operator whose supervisor waits longer can raise it. A request
//! still running when the grace runs out is cut off, and the server says how
//! many were.

use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Weak};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use anyhow::Context as _;
use bytes::Bytes;
use http::{Method, Request, Response};
use http_body_util::{BodyExt, combinators::BoxBody};
use reflow2_core::DesignGraph;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use tokio::sync::{Notify, RwLock};

use crate::auto_export::{AutoExport, Flushed};
use crate::registry_http::GraphRouter;
use crate::service::ReflowService;

/// How long work in progress may take to finish once a stop begins, unless the
/// operator says otherwise (`--shutdown-grace`). With [`CLOSE_BOUND`] on top it
/// stays inside Docker's default stop timeout of 10 s.
pub const DEFAULT_GRACE: Duration = Duration::from_secs(5);

/// How long closing may take once the grace is over: ending sessions, writing a
/// waiting export, and waiting for every store to be released.
pub const CLOSE_BOUND: Duration = Duration::from_secs(2);

/// What every HTTP surface here answers with.
type BoxResponse = Response<BoxBody<Bytes, Infallible>>;

/// Work in progress on a server: requests being answered and connections open.
///
/// Cloned into every connection and every request; the counts are shared.
#[derive(Clone, Default)]
pub struct Work {
    counts: Arc<Counts>,
}

#[derive(Default)]
struct Counts {
    requests: AtomicUsize,
    connections: AtomicUsize,
    changed: Notify,
}

#[derive(Clone, Copy)]
enum Kind {
    Request,
    Connection,
}

impl Counts {
    fn of(&self, kind: Kind) -> &AtomicUsize {
        match kind {
            Kind::Request => &self.requests,
            Kind::Connection => &self.connections,
        }
    }
}

/// One unit of work, counted for as long as this is held.
pub struct Held {
    counts: Arc<Counts>,
    kind: Kind,
}

impl Drop for Held {
    fn drop(&mut self) {
        self.counts.of(self.kind).fetch_sub(1, Ordering::SeqCst);
        self.counts.changed.notify_waiters();
    }
}

impl Work {
    fn hold(&self, kind: Kind) -> Held {
        self.counts.of(kind).fetch_add(1, Ordering::SeqCst);
        Held {
            counts: Arc::clone(&self.counts),
            kind,
        }
    }

    /// Count a request until what this returns is dropped.
    pub fn request(&self) -> Held {
        self.hold(Kind::Request)
    }

    /// Count a connection until what this returns is dropped.
    pub fn connection(&self) -> Held {
        self.hold(Kind::Connection)
    }

    /// Requests in progress right now.
    pub fn requests(&self) -> usize {
        self.counts.requests.load(Ordering::SeqCst)
    }

    /// Connections open right now.
    pub fn connections(&self) -> usize {
        self.counts.connections.load(Ordering::SeqCst)
    }

    /// Wait until no request is in progress, or the deadline passes. True when
    /// they all finished.
    pub async fn requests_done_by(&self, deadline: Instant) -> bool {
        self.none_by(Kind::Request, deadline).await
    }

    /// Wait until every connection has closed, or the deadline passes.
    pub async fn connections_done_by(&self, deadline: Instant) -> bool {
        self.none_by(Kind::Connection, deadline).await
    }

    async fn none_by(&self, kind: Kind, deadline: Instant) -> bool {
        loop {
            // Register for the wake-up BEFORE reading the count, so a drop
            // between the read and the wait cannot be missed.
            let changed = self.counts.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.counts.of(kind).load(Ordering::SeqCst) == 0 {
                return true;
            }
            let until = tokio::time::Instant::from_std(deadline);
            if tokio::time::timeout_at(until, changed).await.is_err() {
                return self.counts.of(kind).load(Ordering::SeqCst) == 0;
            }
        }
    }
}

/// A service that counts every request that is not a GET as work in progress,
/// from its arrival until its answer's body has been written out or dropped.
#[derive(Clone)]
pub struct Tracked<S> {
    inner: S,
    work: Work,
}

impl<S> Tracked<S> {
    pub fn new(inner: S, work: Work) -> Self {
        Self { inner, work }
    }
}

impl<S, B> tower_service::Service<Request<B>> for Tracked<S>
where
    S: tower_service::Service<Request<B>, Response = BoxResponse, Error = Infallible>,
    S::Future: Send + 'static,
{
    type Response = BoxResponse;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<BoxResponse, Infallible>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let held = (req.method() != Method::GET).then(|| self.work.request());
        let answer = self.inner.call(req);
        Box::pin(async move {
            let response = answer.await?;
            Ok(match held {
                None => response,
                // The answer counts as work until its body is done: a tool
                // call's result travels in the body, after the headers.
                Some(held) => response.map(|body| {
                    body.map_frame(move |frame| {
                        let _counted = &held;
                        frame
                    })
                    .boxed()
                }),
            })
        })
    }
}

/// What a stopping server has to close, and how.
pub enum Holds {
    /// One design: its sessions, its store, and its write-through export.
    One {
        sessions: Arc<LocalSessionManager>,
        store: Weak<RwLock<DesignGraph>>,
        auto_export: Option<Arc<AutoExport>>,
        graph_path: Option<String>,
    },
    /// Many designs behind `/g/<graph_id>/`, each opened on demand.
    Many(GraphRouter),
    /// No design at all: the degraded surface, which only explains why.
    Nothing { sessions: Arc<LocalSessionManager> },
}

impl Holds {
    /// The single design `service` serves, through `sessions`.
    pub fn one(service: &ReflowService, sessions: Arc<LocalSessionManager>) -> Self {
        Holds::One {
            sessions,
            store: Arc::downgrade(&service.graph),
            auto_export: service.auto_export_handle(),
            graph_path: service.graph_path.clone(),
        }
    }

    /// Every design a registry router holds.
    pub fn many(router: GraphRouter) -> Self {
        Holds::Many(router)
    }

    /// A surface with sessions and no design.
    pub fn nothing(sessions: Arc<LocalSessionManager>) -> Self {
        Holds::Nothing { sessions }
    }

    async fn close(self, work: &Work, deadline: Instant) -> Closed {
        let mut closed = Closed::default();
        let mut stores: Vec<(String, Weak<RwLock<DesignGraph>>)> = Vec::new();
        match self {
            Holds::One {
                sessions,
                store,
                auto_export,
                graph_path,
            } => {
                if let (Some(auto), Some(graph)) = (auto_export, store.upgrade()) {
                    let until = tokio::time::Instant::from_std(deadline);
                    closed.export = Some(
                        match tokio::time::timeout_at(
                            until,
                            auto.flush(&graph, graph_path.as_deref()),
                        )
                        .await
                        {
                            Ok(flushed) => flushed,
                            Err(_) => Flushed::Declined(
                                "the design was still busy when closing ran out of time"
                                    .to_string(),
                            ),
                        },
                    );
                }
                closed.sessions = end_sessions(&sessions).await;
                closed.designs = 1;
                stores.push(("this design".to_string(), store));
            }
            Holds::Many(router) => {
                for (id, store, sessions) in router.close_all("the server is stopping").await {
                    closed.designs += 1;
                    closed.sessions += sessions;
                    stores.push((id, store));
                }
            }
            Holds::Nothing { sessions } => {
                closed.sessions = end_sessions(&sessions).await;
            }
        }
        if !work.connections_done_by(deadline).await {
            closed.connections_left = work.connections();
        }
        for (name, store) in stores {
            if !released_by(&store, deadline).await {
                closed.still_held.push(name);
            }
        }
        closed
    }
}

/// End every session a session manager holds. Returns how many it ended.
pub(crate) async fn end_sessions(sessions: &LocalSessionManager) -> usize {
    let handles: Vec<_> = sessions
        .sessions
        .write()
        .await
        .drain()
        .map(|(_, handle)| handle)
        .collect();
    for handle in &handles {
        // A worker that already exited has nothing left to end.
        let _ = handle.close().await;
    }
    handles.len()
}

/// Wait until nothing holds `store` any more, or the deadline passes.
async fn released_by(store: &Weak<RwLock<DesignGraph>>, deadline: Instant) -> bool {
    loop {
        if store.strong_count() == 0 {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// What closing did.
#[derive(Debug, Default)]
pub struct Closed {
    /// Sessions ended.
    pub sessions: usize,
    /// Designs closed.
    pub designs: usize,
    /// What happened to the write-through export, when this server keeps one.
    pub export: Option<Flushed>,
    /// Designs whose store was still held when closing ran out of time. They
    /// close as the process exits, not cleanly.
    pub still_held: Vec<String>,
    /// Connections still open when closing ran out of time.
    pub connections_left: usize,
}

/// What a stop did, from the signal to the last store.
#[derive(Debug)]
pub struct Report {
    /// Why the server stopped: the signal, or the reason it gave itself.
    pub why: String,
    /// The grace work in progress was given.
    pub grace: Duration,
    /// Requests in progress when the stop began.
    pub in_flight: usize,
    /// Requests still running when the grace ran out, and so cut off.
    pub cut_off: usize,
    pub closed: Closed,
}

impl Report {
    /// The account a person reads on stderr.
    pub fn sentence(&self) -> String {
        let grace = describe(self.grace);
        let mut parts = vec![format!("reflow2: stopped ({}).", self.why)];
        parts.push(match (self.in_flight, self.cut_off) {
            (0, _) => "No request was in progress.".to_string(),
            (n, 0) => format!("{n} request(s) in progress finished within the {grace} grace."),
            (n, cut) => format!(
                "{n} request(s) were in progress; {cut} were still running when the {grace} grace \
                 ran out and were cut off. Raise --shutdown-grace if that is too short."
            ),
        });
        if self.closed.sessions > 0 {
            parts.push(format!("{} session(s) ended.", self.closed.sessions));
        }
        match &self.closed.export {
            None | Some(Flushed::NothingWaiting) => {}
            Some(Flushed::Wrote(how)) => parts.push(format!(
                "A change was still waiting to be written through to the export, and was written \
                 ({how})."
            )),
            Some(Flushed::Declined(why)) => parts.push(format!(
                "A change was waiting to be written through to the export and was NOT written: \
                 {why}"
            )),
        }
        if self.closed.designs > 0 {
            parts.push(if self.closed.still_held.is_empty() {
                format!(
                    "{} design(s) closed, every store released cleanly.",
                    self.closed.designs
                )
            } else {
                format!(
                    "{} design(s) closed; still held after {}, so closed as the process exits \
                     rather than cleanly: {}.",
                    self.closed.designs,
                    describe(CLOSE_BOUND),
                    self.closed.still_held.join(", ")
                )
            });
        }
        if self.closed.connections_left > 0 {
            parts.push(format!(
                "{} connection(s) were still open and are dropped.",
                self.closed.connections_left
            ));
        }
        parts.join(" ")
    }
}

/// A duration the way the flags take it.
pub fn describe(d: Duration) -> String {
    let ms = d.as_millis();
    if !ms.is_multiple_of(1000) {
        format!("{ms}ms")
    } else if ms.is_multiple_of(60_000) && ms > 0 {
        format!("{}m", ms / 60_000)
    } else {
        format!("{}s", ms / 1000)
    }
}

/// Resolves when the process is asked to stop: SIGTERM or SIGINT (Ctrl-C).
///
/// The handlers are installed on the first poll, and from then on the signal no
/// longer kills the process by default: whoever awaits this decides what a stop
/// means.
pub async fn stop_signal() -> String {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        match (
            signal(SignalKind::terminate()),
            signal(SignalKind::interrupt()),
        ) {
            (Ok(mut term), Ok(mut int)) => tokio::select! {
                _ = term.recv() => "SIGTERM".to_string(),
                _ = int.recv() => "SIGINT".to_string(),
            },
            _ => {
                eprintln!(
                    "reflow2: WARNING — could not install the stop handlers, so a stop signal \
                     ends this server without draining."
                );
                std::future::pending().await
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
        "Ctrl-C".to_string()
    }
}

/// What the caller wants run at the edges of a stop.
pub struct Hooks {
    /// Every accepted connection, before it is served.
    pub on_accept: Box<dyn Fn() + Send + Sync>,
    /// The moment the server stops accepting, before anything drains. A shared
    /// server takes its rendezvous down here, so no session attaches to a
    /// server on its way out.
    pub on_stop: Box<dyn FnOnce(&str) + Send>,
    /// Ends every session and stream: rmcp's cancellation token, cancelled.
    pub end_streams: Box<dyn FnOnce() + Send>,
}

impl Default for Hooks {
    fn default() -> Self {
        Self {
            on_accept: Box::new(|| {}),
            on_stop: Box::new(|_| {}),
            end_streams: Box::new(|| {}),
        }
    }
}

/// Serve `http` on `listener` until `stop` resolves, then drain and close
/// everything in `holds`, in the order the module comment sets out.
pub async fn serve_until_stopped<Svc>(
    listener: tokio::net::TcpListener,
    http: Svc,
    holds: Holds,
    grace: Duration,
    stop: impl Future<Output = String>,
    hooks: Hooks,
) -> anyhow::Result<Report>
where
    Svc: tower_service::Service<
            Request<hyper::body::Incoming>,
            Response = BoxResponse,
            Error = Infallible,
        > + Clone
        + Send
        + 'static,
    Svc::Future: Send + 'static,
{
    let Hooks {
        on_accept,
        on_stop,
        end_streams,
    } = hooks;
    let work = Work::default();
    let tracked = Tracked::new(http, work.clone());
    let (stopping_tx, stopping_rx) = tokio::sync::watch::channel(false);
    tokio::pin!(stop);

    let why = loop {
        tokio::select! {
            why = &mut stop => break why,
            accepted = listener.accept() => {
                let (stream, peer) = accepted.context("failed to accept an HTTP connection")?;
                on_accept();
                let io = hyper_util::rt::TokioIo::new(stream);
                let svc = hyper_util::service::TowerToHyperService::new(tracked.clone());
                let mut stopping = stopping_rx.clone();
                let open = work.connection();
                // One task per connection: a slow or stuck client must never
                // hold up the others, which is the whole reason several
                // sessions can share this.
                tokio::spawn(async move {
                    let _open = open;
                    let conn = hyper::server::conn::http1::Builder::new()
                        .serve_connection(io, svc)
                        .with_upgrades();
                    tokio::pin!(conn);
                    // The watch borrow is dropped inside, so it never lives
                    // across an await of this task.
                    let stop_asked = async move {
                        let _ = stopping.wait_for(|s| *s).await;
                    };
                    tokio::pin!(stop_asked);
                    let ended = tokio::select! {
                        ended = conn.as_mut() => ended,
                        () = &mut stop_asked => {
                            // Finish the exchange in progress, then close.
                            conn.as_mut().graceful_shutdown();
                            conn.as_mut().await
                        }
                    };
                    if let Err(e) = ended {
                        tracing::debug!("connection from {peer} ended: {e}");
                    }
                });
            }
        }
    };

    drop(listener);
    on_stop(&why);
    let in_flight = work.requests();
    eprintln!(
        "reflow2: stopping ({why}) — no new connections; {in_flight} request(s) in progress get \
         up to {} to finish.",
        describe(grace)
    );
    let _ = stopping_tx.send(true);
    let cut_off = if work.requests_done_by(Instant::now() + grace).await {
        0
    } else {
        work.requests()
    };
    end_streams();
    // Our own copy of the service holds the design too; only the connections'
    // copies may outlive this line, and they end with their connections.
    drop(tracked);
    let closed = holds.close(&work, Instant::now() + CLOSE_BOUND).await;
    let report = Report {
        why,
        grace,
        in_flight,
        cut_off,
        closed,
    };
    eprintln!("{}", report.sentence());
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::Full;
    use tower_service::Service;

    /// A service whose answer takes `delay`, and whose body is one frame.
    #[derive(Clone)]
    struct Slow {
        delay: Duration,
    }

    impl<B: Send + 'static> tower_service::Service<Request<B>> for Slow {
        type Response = BoxResponse;
        type Error = Infallible;
        type Future = Pin<Box<dyn Future<Output = Result<BoxResponse, Infallible>> + Send>>;
        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, _: Request<B>) -> Self::Future {
            let delay = self.delay;
            Box::pin(async move {
                tokio::time::sleep(delay).await;
                Ok(Response::new(Full::new(Bytes::from("done")).boxed()))
            })
        }
    }

    fn get() -> Request<Full<Bytes>> {
        Request::get("/").body(Full::new(Bytes::new())).unwrap()
    }

    fn post() -> Request<Full<Bytes>> {
        Request::post("/").body(Full::new(Bytes::new())).unwrap()
    }

    #[tokio::test]
    async fn a_request_counts_as_work_until_its_body_is_done_and_a_get_never_does() {
        let work = Work::default();
        let mut svc = Tracked::new(
            Slow {
                delay: Duration::ZERO,
            },
            work.clone(),
        );

        let answer = svc.call(post()).await.unwrap();
        assert_eq!(
            work.requests(),
            1,
            "the answer's body has not been read yet"
        );
        let _ = answer.into_body().collect().await.unwrap();
        assert_eq!(work.requests(), 0, "a finished body is no longer work");

        let stream = svc.call(get()).await.unwrap();
        assert_eq!(
            work.requests(),
            0,
            "a GET is a stream, never work to wait for"
        );
        drop(stream);
    }

    #[tokio::test]
    async fn an_answer_dropped_unread_stops_counting() {
        let work = Work::default();
        let mut svc = Tracked::new(
            Slow {
                delay: Duration::ZERO,
            },
            work.clone(),
        );
        let answer = svc.call(post()).await.unwrap();
        drop(answer);
        assert_eq!(work.requests(), 0, "a client that went away is not work");
    }

    #[tokio::test]
    async fn waiting_ends_when_the_work_does_or_at_the_deadline() {
        let work = Work::default();
        let held = work.request();
        let waited = Instant::now();
        assert!(
            !work
                .requests_done_by(Instant::now() + Duration::from_millis(50))
                .await,
            "work still held is not done"
        );
        assert!(waited.elapsed() >= Duration::from_millis(50));

        let w = work.clone();
        let done = tokio::spawn(async move {
            w.requests_done_by(Instant::now() + Duration::from_secs(5))
                .await
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        let released = Instant::now();
        drop(held);
        assert!(done.await.unwrap(), "done as soon as the work ends");
        assert!(
            released.elapsed() < Duration::from_secs(1),
            "and not at the deadline"
        );
    }

    /// Start a server over `Slow`, and hand back its port and a trigger that
    /// stops it.
    async fn serve_slow(
        delay: Duration,
        grace: Duration,
    ) -> (
        u16,
        tokio::sync::oneshot::Sender<()>,
        tokio::task::JoinHandle<anyhow::Result<Report>>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(serve_until_stopped(
            listener,
            Slow { delay },
            Holds::nothing(Arc::new(LocalSessionManager::default())),
            grace,
            async move {
                let _ = rx.await;
                "told to stop".to_string()
            },
            Hooks::default(),
        ));
        (port, tx, server)
    }

    /// One raw HTTP/1.1 POST; the whole answer as text.
    async fn raw_post(port: u16) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        s.write_all(b"POST / HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 0\r\n\r\n")
            .await
            .unwrap();
        let mut raw = String::new();
        let _ = s.read_to_string(&mut raw).await;
        raw
    }

    #[tokio::test]
    async fn a_request_in_progress_when_the_stop_comes_is_answered_in_full() {
        let (port, stop, server) =
            serve_slow(Duration::from_millis(400), Duration::from_secs(5)).await;
        let client = tokio::spawn(raw_post(port));
        tokio::time::sleep(Duration::from_millis(100)).await;
        let _ = stop.send(());

        let answer = client.await.unwrap();
        assert!(
            answer.starts_with("HTTP/1.1 200")
                && answer.contains("\r\ndone\r\n")
                && answer.ends_with("0\r\n\r\n"),
            "the request that was running finished with its whole answer, down to the \
             chunked body's closing chunk: {answer:?}"
        );
        assert!(
            answer.to_ascii_lowercase().contains("connection: close"),
            "and the connection closed after it rather than waiting for another: {answer:?}"
        );
        let report = server.await.unwrap().unwrap();
        assert_eq!(report.in_flight, 1);
        assert_eq!(report.cut_off, 0);
        assert!(
            tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .is_err(),
            "a stopped server accepts nothing"
        );
    }

    #[tokio::test]
    async fn work_that_outlasts_the_grace_is_cut_off_and_said() {
        let (port, stop, server) =
            serve_slow(Duration::from_secs(30), Duration::from_millis(200)).await;
        let _client = tokio::spawn(raw_post(port));
        tokio::time::sleep(Duration::from_millis(100)).await;
        let began = Instant::now();
        let _ = stop.send(());
        let report = server.await.unwrap().unwrap();
        assert!(
            began.elapsed() < Duration::from_millis(200) + CLOSE_BOUND + Duration::from_secs(1),
            "the stop is bounded by the grace and the close, not by the slow request"
        );
        assert_eq!(report.cut_off, 1);
        assert!(
            report.sentence().contains("cut off"),
            "{}",
            report.sentence()
        );
    }

    /// A body that never ends: the standalone stream a session keeps open.
    struct Endless;

    impl http_body::Body for Endless {
        type Data = Bytes;
        type Error = Infallible;
        fn poll_frame(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
        ) -> Poll<Option<Result<http_body::Frame<Bytes>, Infallible>>> {
            Poll::Pending
        }
    }

    /// Answers a GET with a stream that never ends.
    #[derive(Clone)]
    struct Streams;

    impl<B: Send + 'static> tower_service::Service<Request<B>> for Streams {
        type Response = BoxResponse;
        type Error = Infallible;
        type Future = Pin<Box<dyn Future<Output = Result<BoxResponse, Infallible>> + Send>>;
        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, _: Request<B>) -> Self::Future {
            Box::pin(async { Ok(Response::new(Endless.boxed())) })
        }
    }

    #[tokio::test]
    async fn an_open_stream_does_not_make_the_stop_wait_out_the_grace() {
        use tokio::io::AsyncWriteExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let grace = Duration::from_secs(30);
        let server = tokio::spawn(serve_until_stopped(
            listener,
            Streams,
            Holds::nothing(Arc::new(LocalSessionManager::default())),
            grace,
            async move {
                let _ = rx.await;
                "told to stop".to_string()
            },
            Hooks::default(),
        ));
        let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        s.write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;

        let began = Instant::now();
        let _ = tx.send(());
        let report = server.await.unwrap().unwrap();
        assert!(
            began.elapsed() < CLOSE_BOUND + Duration::from_secs(1),
            "a stream is not work: the stop took {:?} of a {grace:?} grace",
            began.elapsed()
        );
        assert_eq!(report.in_flight, 0);
        assert_eq!(
            report.closed.connections_left, 1,
            "a stream nothing ended is reported as dropped, not waited for"
        );
    }

    #[test]
    fn a_duration_reads_the_way_the_flag_takes_it() {
        assert_eq!(describe(Duration::from_secs(5)), "5s");
        assert_eq!(describe(Duration::from_secs(120)), "2m");
        assert_eq!(describe(Duration::from_millis(250)), "250ms");
        assert_eq!(describe(Duration::ZERO), "0s");
    }
}
