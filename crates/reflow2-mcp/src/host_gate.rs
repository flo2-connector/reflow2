//! The Host allowlist, owned here — one rule, in front of every HTTP surface.
//!
//! The streamable-HTTP transport answers only requests whose `Host` header is on
//! an allowlist (loopback by default; `--http-allow-host` extends it). That is
//! DNS-rebinding protection: a web page the person visits must not reach their
//! design through a name rebound to 127.0.0.1 (`cap:remote-sessions`).
//!
//! ⭐ WHY REFLOW2 OWNS THE CHECK INSTEAD OF THE MCP LIBRARY. Until 2026-09-28
//! the check lived inside rmcp, which refused in its own words — `403 Forbidden:
//! Host header is not allowed`, no content type — and logged "possible DNS
//! rebinding attempt". Neither named the flag that would admit the host.
//! reflow2's sentence naming `--http-allow-host` printed once, at startup, where
//! a container log scrolls it away. Behind a proxy that reads as a broken
//! ingress (GitHub issue #616;
//! `fact:root-cause-the-host-refusal-is-the-librarys-opaque-403-and-the-fix-is-named-only-at-startup-2026-09-28`).
//! "Say what would have worked" belongs where the failure is MET: the reply,
//! and the log line an operator tails.
//!
//! ⭐ ONE OWNER, NOT TWO. rmcp's copy is switched off (`disable_allowed_hosts`)
//! where the config is built, and this gate runs in front of every service
//! `serve_http` starts: single-design `--http`, the `--serve-shared` daemon,
//! the `--registry-root` router and the degraded surface. Two copies of one rule
//! can disagree, and the day an upstream normalisation changed, some requests
//! would get the opaque 403 back. The gate also runs BEFORE the registry
//! router, which answered a bare path with its design listing (and opened a
//! design for a `/g/<id>/` path) before rmcp's per-design check ever ran.
//!
//! 🛑 THE MATCHING IS rmcp 3.4's, KEPT BYTE-FOR-BYTE IN MEANING so moving the
//! owner changes what a refusal SAYS and nothing about what is refused: the
//! host is compared case-insensitively with brackets trimmed; an allowed entry
//! without a port admits any port, and one with a port admits only that port;
//! a missing Host falls back to the request's `:authority`; a Host that is not
//! UTF-8 or not an authority is a 400. The unit tests below pin each case.

use bytes::Bytes;
use http::{HeaderMap, Request, Response, StatusCode, Uri};
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use std::collections::HashSet;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

type BoxResponse = Response<BoxBody<Bytes, Infallible>>;

/// The hosts every server answers, before any `--http-allow-host`: loopback.
pub const LOOPBACK: [&str; 3] = ["localhost", "127.0.0.1", "::1"];

/// How many distinct refused hosts are logged one by one. Past this, one line
/// says further refusals are not logged individually — a flood of forged Host
/// headers must not fill the log, and the REPLY still names the fix every time.
const LOGGED_HOSTS_CAP: usize = 32;

/// A refused Host is echoed back (in the reply and the log) cut to this many
/// characters, so a caller cannot make either arbitrarily large.
const ECHO_CAP: usize = 200;

/// The allowlist and what has been said about it.
pub struct HostRule {
    allowed: Vec<String>,
    logged: Mutex<LogState>,
}

#[derive(Default)]
struct LogState {
    hosts: HashSet<String>,
    overflow_said: bool,
}

/// A Host header reduced to what the rule compares.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Authority {
    host: String,
    port: Option<u16>,
}

impl Authority {
    fn new(host: &str, port: Option<u16>) -> Authority {
        Authority {
            host: host
                .trim_matches('[')
                .trim_matches(']')
                .to_ascii_lowercase(),
            port,
        }
    }

    /// How a person would write it: `host` or `host:port`, IPv6 bracketed when
    /// a port follows.
    fn display(&self) -> String {
        match (self.port, self.host.contains(':')) {
            (Some(p), true) => format!("[{}]:{p}", self.host),
            (Some(p), false) => format!("{}:{p}", self.host),
            (None, _) => self.host.clone(),
        }
    }
}

impl HostRule {
    /// Loopback plus `extra`, EXTENDED and never replaced, so naming a remote
    /// host cannot lock out the local sessions already using this server.
    pub fn new(extra: &[String]) -> HostRule {
        let mut allowed: Vec<String> = LOOPBACK.iter().map(|h| h.to_string()).collect();
        for h in extra {
            if !allowed.contains(h) {
                allowed.push(h.clone());
            }
        }
        HostRule {
            allowed,
            logged: Mutex::new(LogState::default()),
        }
    }

    /// The hosts this server answers, in the order they were given.
    pub fn allowed(&self) -> &[String] {
        &self.allowed
    }

    /// `Ok` when the request may pass; otherwise the reply that refuses it.
    pub fn check(&self, uri: &Uri, headers: &HeaderMap) -> Result<(), Box<BoxResponse>> {
        let host = parse_host(uri, headers)?;
        if self.admits(&host) {
            return Ok(());
        }
        self.log_refusal(&host);
        Err(Box::new(self.refusal(&host)))
    }

    fn admits(&self, host: &Authority) -> bool {
        self.allowed
            .iter()
            .filter_map(|a| parse_allowed(a))
            .any(|a| a.host == host.host && a.port.is_none_or(|p| host.port == Some(p)))
    }

    /// The reply: what was refused, what is allowed, and the flag that would
    /// admit it. Plain text, because the reader is a person looking at a proxy's
    /// error page or a curl, not an MCP client.
    fn refusal(&self, host: &Authority) -> BoxResponse {
        let seen = echo(&host.display());
        let admit = echo(&host.host);
        let body = format!(
            "reflow2 refused this request: it is addressed to Host \"{seen}\", and this server \
             answers only the hosts it was told to: {allowed}.\n\n\
             To let requests addressed to that name in, restart the server with:\n\n\
             \x20   --http-allow-host {admit}\n\n\
             The flag is repeatable, one per name clients use, and it EXTENDS the list, so local \
             sessions keep working. (`--http-allow-host {admit}:<port>` admits that port only.)\n\n\
             Why the check exists: anything that can reach this port can use the design, and \
             without it a web page could reach a server on this machine through a DNS name \
             rebound to it.\n",
            allowed = self.allowed.join(", "),
        );
        Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
            // The body echoes a caller-supplied header; no client may sniff it
            // into anything but text.
            .header("x-content-type-options", "nosniff")
            .body(Full::new(Bytes::from(body)).boxed())
            .expect("a static text response is always well-formed")
    }

    /// One stderr line per distinct refused host, then one line saying the rest
    /// are not logged one by one.
    fn log_refusal(&self, host: &Authority) {
        let Ok(mut state) = self.logged.lock() else {
            return;
        };
        if state.hosts.contains(&host.display()) {
            return;
        }
        if state.hosts.len() >= LOGGED_HOSTS_CAP {
            if !state.overflow_said {
                state.overflow_said = true;
                eprintln!(
                    "reflow2: {LOGGED_HOSTS_CAP} different Host headers have now been refused; \
                     further ones are not logged one by one. Each refused request's reply still \
                     names the host it used and the --http-allow-host flag that would admit it."
                );
            }
            return;
        }
        state.hosts.insert(host.display());
        eprintln!(
            "reflow2: refused a request addressed to Host \"{seen}\" (403) — this server answers \
             only {allowed}. If clients reach it by that name, restart with --http-allow-host {admit}. \
             Further requests to the same host are refused without logging this again.",
            seen = echo(&host.display()),
            admit = echo(&host.host),
            allowed = self.allowed.join(", "),
        );
    }
}

/// An allowed entry as an authority: `host` or `host:port`.
fn parse_allowed(allowed: &str) -> Option<Authority> {
    let allowed = allowed.trim();
    if allowed.is_empty() {
        return None;
    }
    if let Ok(a) = http::uri::Authority::try_from(allowed) {
        return Some(Authority::new(a.host(), a.port_u16()));
    }
    Some(Authority::new(allowed, None))
}

/// The request's Host, or the 400 a malformed one gets.
fn parse_host(uri: &Uri, headers: &HeaderMap) -> Result<Authority, Box<BoxResponse>> {
    if let Some(raw) = headers.get(http::header::HOST) {
        let text = raw.to_str().map_err(|_| {
            bad_request("reflow2 refused this request: its Host header is not valid text.\n")
        })?;
        let a = http::uri::Authority::try_from(text).map_err(|_| {
            bad_request(&format!(
                "reflow2 refused this request: its Host header \"{}\" is not a host name or \
                 host:port.\n",
                echo(text)
            ))
        })?;
        return Ok(Authority::new(a.host(), a.port_u16()));
    }
    // HTTP/2 carries the host in `:authority`, and a proxy may drop the `Host`
    // header hyper would synthesise from it.
    let a = uri
        .authority()
        .ok_or_else(|| bad_request("reflow2 refused this request: it carries no Host header.\n"))?;
    Ok(Authority::new(a.host(), a.port_u16()))
}

fn bad_request(body: &str) -> Box<BoxResponse> {
    Box::new(
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .header("x-content-type-options", "nosniff")
            .body(Full::new(Bytes::from(body.to_string())).boxed())
            .expect("a static text response is always well-formed"),
    )
}

/// A caller-supplied value, cut to a length that keeps replies and logs small.
fn echo(s: &str) -> String {
    let cut: String = s.chars().take(ECHO_CAP).collect();
    if cut.len() < s.len() {
        format!("{cut}…")
    } else {
        cut
    }
}

/// `inner`, reached only by requests whose Host the rule admits.
#[derive(Clone)]
pub struct HostGate<S> {
    inner: S,
    rule: Arc<HostRule>,
}

impl<S> HostGate<S> {
    pub fn new(inner: S, rule: Arc<HostRule>) -> HostGate<S> {
        HostGate { inner, rule }
    }
}

impl<S, B> tower_service::Service<Request<B>> for HostGate<S>
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
        match self.rule.check(req.uri(), req.headers()) {
            Ok(()) => Box::pin(self.inner.call(req)),
            Err(refused) => Box::pin(async move { Ok(*refused) }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(host: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(http::header::HOST, host.parse().unwrap());
        h
    }

    fn admits(rule: &HostRule, host: &str) -> bool {
        rule.check(&Uri::from_static("/"), &headers(host)).is_ok()
    }

    fn body_of(r: BoxResponse) -> String {
        let bytes = futures_block(r.into_body().collect())
            .expect("an Infallible body cannot fail")
            .to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn futures_block<F: Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(f)
    }

    #[test]
    fn loopback_is_answered_on_any_port_and_in_any_case() {
        let rule = HostRule::new(&[]);
        for h in [
            "localhost",
            "LOCALHOST:8080",
            "127.0.0.1:40233",
            "[::1]:9",
            "[::1]",
        ] {
            assert!(admits(&rule, h), "{h}");
        }
        assert!(!admits(&rule, "team.example.org"));
    }

    #[test]
    fn a_named_host_extends_the_list_and_never_replaces_it() {
        let rule = HostRule::new(&["team.example.org".to_string()]);
        assert!(admits(&rule, "team.example.org"));
        assert!(
            admits(&rule, "Team.Example.Org:443"),
            "any port when none was named"
        );
        assert!(admits(&rule, "localhost"), "loopback is kept");
        assert!(!admits(&rule, "other.example.org"));
    }

    #[test]
    fn a_named_host_with_a_port_admits_only_that_port() {
        let rule = HostRule::new(&["team.example.org:8443".to_string()]);
        assert!(admits(&rule, "team.example.org:8443"));
        assert!(!admits(&rule, "team.example.org:443"));
        assert!(
            !admits(&rule, "team.example.org"),
            "no port is not that port"
        );
    }

    #[test]
    fn a_malformed_or_missing_host_is_a_bad_request_not_a_pass() {
        let rule = HostRule::new(&[]);
        let r = rule
            .check(&Uri::from_static("/"), &headers("bad host name"))
            .unwrap_err();
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        let r = rule
            .check(&Uri::from_static("/"), &HeaderMap::new())
            .unwrap_err();
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        // With no Host header, the request's own authority is what is checked.
        assert!(
            rule.check(&Uri::from_static("http://127.0.0.1:5/"), &HeaderMap::new())
                .is_ok()
        );
    }

    #[test]
    fn the_refusal_names_the_host_the_allowed_list_and_the_flag() {
        let rule = HostRule::new(&["a.example".to_string()]);
        let r = rule
            .check(&Uri::from_static("/"), &headers("team.example.org:9000"))
            .unwrap_err();
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        let body = body_of(*r);
        assert!(body.contains("\"team.example.org:9000\""), "{body}");
        assert!(
            body.contains("localhost, 127.0.0.1, ::1, a.example"),
            "{body}"
        );
        assert!(
            body.contains("--http-allow-host team.example.org\n"),
            "{body}"
        );
    }

    #[test]
    fn an_echoed_host_is_bounded() {
        let rule = HostRule::new(&[]);
        let long = format!("{}.example.org", "a".repeat(5000));
        let r = rule
            .check(&Uri::from_static("/"), &headers(&long))
            .unwrap_err();
        let body = body_of(*r);
        assert!(
            body.len() < 2000,
            "a caller cannot grow the reply: {}",
            body.len()
        );
    }

    #[test]
    fn each_refused_host_is_logged_once_and_the_log_is_capped() {
        let rule = HostRule::new(&[]);
        for i in 0..(LOGGED_HOSTS_CAP + 10) {
            let h = format!("h{i}.example.org");
            let _ = rule.check(&Uri::from_static("/"), &headers(&h));
            let _ = rule.check(&Uri::from_static("/"), &headers(&h));
        }
        let state = rule.logged.lock().unwrap();
        assert_eq!(state.hosts.len(), LOGGED_HOSTS_CAP);
        assert!(state.overflow_said);
    }
}
