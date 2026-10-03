//! Readiness that tells the truth: `GET /readyz` and `GET /healthz` on every
//! HTTP surface reflow2 serves.
//!
//! ⭐ WHY THIS EXISTS. Until 2026-09-28 the only health signal an orchestrator
//! had was "the port is listening" — the image's HEALTHCHECK was a bare TCP
//! connect. A server that could not open its design still binds its port (it
//! serves the one-tool degraded surface, `req:never-silently-absent`), so the
//! signal could not tell a served design from a degraded one. On a rolling
//! update the new pod lost the store's lock to the old one, looked healthy, and
//! stayed degraded after the old pod was removed (GitHub issue #616;
//! `fact:root-cause-a-server-degraded-by-a-held-lock-never-retries-and-looks-alive-2026-09-28`).
//!
//! · `/readyz` answers 200 only while the design is served, and 503 with ONE
//!   path-free sentence otherwise. A server that recovers in place (a held lock
//!   that frees, `crate::degraded`) turns ready the moment it serves.
//! · `/healthz` answers 200 while the process serves HTTP at all — liveness. A
//!   degraded server is alive: restarting it would not open a store another
//!   process holds, and would not fix a stamp that will not read.
//! · A registry (`--registry-root`) is ready when it is up. It serves many
//!   designs, each opened on demand; one it cannot serve is named on `GET /`
//!   and in its startup log, and must not take every other design out of
//!   service.
//!
//! 🛑 WHY THE PROBES SIT IN FRONT OF THE HOST GATE (`crate::host_gate`), AND WHY
//! THAT IS SAFE. An orchestrator probes by pod IP or pod name, neither of which
//! an operator puts in `--http-allow-host`, so a gated probe is a 403 and the
//! pod is never ready. The Host allowlist exists so a web page cannot reach the
//! DESIGN through a DNS name rebound to this machine. These two paths expose no
//! design: they say ready or not, alive or not, and (when not ready) one
//! sentence with no path, id or design content in it. Everything else — every
//! MCP request, the registry listing, every `/g/<id>/` — stays behind the gate.

use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};

type BoxResponse = Response<BoxBody<Bytes, Infallible>>;

/// The readiness path: 200 while the design is served.
pub const READY_PATH: &str = "/readyz";
/// The liveness path: 200 while the process serves HTTP.
pub const LIVE_PATH: &str = "/healthz";

/// Whether this server is serving what it was started to serve.
#[derive(Debug)]
pub struct Readiness {
    /// `None` when ready; otherwise the one sentence a probe is told.
    not_ready: RwLock<Option<String>>,
}

impl Readiness {
    /// Ready from the start: a server that opened its design before it bound.
    pub fn ready() -> Arc<Readiness> {
        Arc::new(Readiness {
            not_ready: RwLock::new(None),
        })
    }

    /// Not ready, for `why` — a sentence with NO path, id or design content,
    /// because anyone who can reach the port can read it.
    pub fn not_ready(why: &str) -> Arc<Readiness> {
        Arc::new(Readiness {
            not_ready: RwLock::new(Some(why.to_string())),
        })
    }

    /// The design is served now.
    pub fn set_ready(&self) {
        if let Ok(mut g) = self.not_ready.write() {
            *g = None;
        }
    }

    /// The design is not served, for `why` (path-free; see `not_ready`).
    pub fn set_not_ready(&self, why: &str) {
        if let Ok(mut g) = self.not_ready.write() {
            *g = Some(why.to_string());
        }
    }

    pub fn is_ready(&self) -> bool {
        self.not_ready.read().map(|g| g.is_none()).unwrap_or(false)
    }

    fn answer(&self) -> (StatusCode, String) {
        match self.not_ready.read().ok().and_then(|g| g.clone()) {
            None => (StatusCode::OK, "ready: the design is served\n".to_string()),
            Some(why) => (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("not ready: {why}\n"),
            ),
        }
    }
}

/// What a probe of a server that cannot serve its design YET is told.
pub const HELD_ELSEWHERE: &str = "the design is held by another process; this server serves it as \
                                  soon as that process lets go, with no restart";

/// What a probe of a server that cannot serve its design by itself is told.
pub const CANNOT_OPEN: &str = "the design could not be opened, and waiting will not fix it; the \
                               server log says why, and a restart after fixing the cause serves it";

/// What a probe of a server serving the latent surface is told: there is no
/// design to serve yet (`crate::latent`). Turned ready when one appears.
pub const NO_DESIGN_HERE: &str = "no design store exists where this server was pointed, so there \
                                  is no design to serve yet; it serves one as soon as one exists \
                                  there, with no restart";

/// `inner`, with `/readyz` and `/healthz` answered in front of it.
#[derive(Clone)]
pub struct Probes<S> {
    inner: S,
    readiness: Arc<Readiness>,
}

impl<S> Probes<S> {
    pub fn new(inner: S, readiness: Arc<Readiness>) -> Probes<S> {
        Probes { inner, readiness }
    }
}

fn plain(status: StatusCode, body: String) -> BoxResponse {
    Response::builder()
        .status(status)
        .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header("x-content-type-options", "nosniff")
        // A probe's answer is about NOW; no cache may keep an old one.
        .header(http::header::CACHE_CONTROL, "no-store")
        .body(Full::new(Bytes::from(body)).boxed())
        .expect("a static text response is always well-formed")
}

impl<S, B> tower_service::Service<Request<B>> for Probes<S>
where
    S: tower_service::Service<Request<B>, Response = BoxResponse, Error = Infallible>,
    S::Future: Send + 'static,
{
    type Response = BoxResponse;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<BoxResponse, Infallible>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        // Only GET and HEAD are probes. Anything else on these paths goes on to
        // the gate and the transport like any other request, so the exemption is
        // exactly as wide as the two read-only answers and no wider.
        let probe = matches!(*req.method(), Method::GET | Method::HEAD);
        match req.uri().path() {
            READY_PATH if probe => {
                let (status, body) = self.readiness.answer();
                Box::pin(async move { Ok(plain(status, body)) })
            }
            LIVE_PATH if probe => {
                Box::pin(async move { Ok(plain(StatusCode::OK, "alive\n".to_string())) })
            }
            _ => Box::pin(self.inner.call(req)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_moves_both_ways_and_says_why_only_when_not_ready() {
        let r = Readiness::not_ready(HELD_ELSEWHERE);
        assert!(!r.is_ready());
        let (status, body) = r.answer();
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(body.contains("held by another process"), "{body}");
        r.set_ready();
        assert!(r.is_ready());
        assert_eq!(r.answer().0, StatusCode::OK);
        r.set_not_ready(CANNOT_OPEN);
        assert_eq!(r.answer().0, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn the_sentences_a_probe_can_read_carry_no_path() {
        for s in [HELD_ELSEWHERE, CANNOT_OPEN] {
            assert!(
                !s.contains('/'),
                "a probe sentence must not carry a path: {s}"
            );
        }
    }
}
