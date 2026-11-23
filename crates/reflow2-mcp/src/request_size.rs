//! The largest request the HTTP door takes in one call, chosen by its operator.
//!
//! The streamable-HTTP transport holds a whole POST body in memory before it
//! reads the call, and refuses one past its limit with `413`. reflow2 set no
//! limit until 2026-11-22, so the library's 4 MiB held everywhere
//! (`fact:measured-the-http-door-takes-4-mib-in-one-request-2026-11-22`), and a
//! document sent to `import_graph` over HTTP could be no larger. flo2's file
//! route needs larger records files, and Anthony chose (2026-11-22, in flo2's
//! design) that they get in by raising this limit rather than by a file the
//! engine reads or by splitting the file.
//!
//! ⭐ THE DEFAULT DOES NOT MOVE. Unset, the limit is [`DEFAULT_BYTES`], the
//! library's own, so no server changes until its operator says so.
//!
//! ⭐ BOUNDED. A request is held whole for the length of the call, and a server
//! behind a gateway may hold it twice (the gateway's copy and this one), so a
//! typo of a few zeros must not let one call take the box. A value outside
//! [`MIN_BYTES`]..=[`MAX_BYTES`] is refused at startup, saying the bounds.
//!
//! ⭐ THE REFUSAL SAYS WHAT WOULD HAVE WORKED. The library answers an
//! oversized request "Payload Too Large: request body exceeds N bytes", which
//! names the limit and not the setting. [`RequestSizeGate`] keeps the library's
//! status and replaces its words with reflow2's: the limit, in bytes and MiB,
//! the flag and its environment twin, and that nothing was read — where the
//! failure is met, as the host refusal does (`host_gate`).

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

type BoxResponse = Response<BoxBody<Bytes, Infallible>>;

/// The flag that sets the limit.
pub const FLAG: &str = "--http-max-request-bytes";

/// Its environment twin.
pub const ENV: &str = "REFLOW2_HTTP_MAX_REQUEST_BYTES";

/// The limit when nothing sets it: rmcp 3.6's `DEFAULT_MAX_REQUEST_BODY_BYTES`.
/// Kept equal to it on purpose, so leaving the setting alone changes nothing.
pub const DEFAULT_BYTES: usize = 4 * 1024 * 1024;

/// The smallest limit accepted. Below this an ordinary `tools/call` with a
/// paragraph of prose would be refused, which is a misconfiguration, not a
/// choice.
pub const MIN_BYTES: usize = 64 * 1024;

/// The largest limit accepted. A request is held whole in memory for the
/// length of the call; past this a records file belongs on another route
/// (`dec:idea-how-a-records-file-past-one-engine-request-gets-in`, option B).
pub const MAX_BYTES: usize = 512 * 1024 * 1024;

/// Read a limit as an operator writes it: a whole number of bytes, or one
/// followed by `KiB`, `MiB` or `GiB` (case-insensitive, a space allowed), and
/// held to [`MIN_BYTES`]..=[`MAX_BYTES`]. The error says what was wrong and
/// what is accepted, in a sentence clap prints after the flag's name.
pub fn parse(text: &str) -> Result<usize, String> {
    let t = text.trim();
    let split = t.find(|c: char| !c.is_ascii_digit()).unwrap_or(t.len());
    let (digits, unit) = t.split_at(split);
    let unit = unit.trim().to_ascii_lowercase();
    let scale: usize = match unit.as_str() {
        "" | "b" => 1,
        "kib" => 1024,
        "mib" => 1024 * 1024,
        "gib" => 1024 * 1024 * 1024,
        _ => {
            return Err(format!(
                "{text:?} is not a size: write a whole number of bytes, or one followed by KiB, \
                 MiB or GiB (for example 16MiB)"
            ));
        }
    };
    // An empty number is not a size. All digits and too long for a usize IS
    // one, only too large: it reads as past the bound, never as a typo of form.
    if digits.is_empty() {
        return Err(format!(
            "{text:?} is not a size: write a whole number of bytes, or one followed by KiB, MiB \
             or GiB (for example 16MiB)"
        ));
    }
    let n: usize = digits.parse().unwrap_or(usize::MAX);
    let bytes = n.saturating_mul(scale);
    if !(MIN_BYTES..=MAX_BYTES).contains(&bytes) {
        return Err(format!(
            "{text:?} is {bytes} bytes, outside what this server accepts: from {} to {} \
             (64KiB to 512MiB). A request is held whole in memory for the length of the call, so \
             the bound keeps one call from taking the machine",
            MIN_BYTES, MAX_BYTES
        ));
    }
    Ok(bytes)
}

/// A size in MiB for a sentence, to two places when it is not whole.
pub fn mib(bytes: usize) -> String {
    let m = bytes as f64 / (1024.0 * 1024.0);
    if (m.fract()).abs() < f64::EPSILON {
        format!("{m:.0} MiB")
    } else {
        format!("{m:.2} MiB")
    }
}

/// What a request over the limit is answered with.
pub fn refusal_text(limit: usize) -> String {
    format!(
        "Request too large: this server takes at most {limit} bytes ({}) in one request, and \
         this one was larger, so nothing in it was read. An operator raises the limit with \
         {FLAG} <SIZE> or {ENV}=<SIZE> (for example 16MiB, up to 512MiB). A caller sends less \
         at once: a smaller document, or the same records in parts.",
        mib(limit)
    )
}

/// Answers an oversized request in reflow2's words.
///
/// It reads nothing of the request: the transport inside it enforces the limit
/// while it streams the body, and this only rewrites the `413` it returns,
/// keeping the status, so a client that acts on the code acts as before.
#[derive(Clone)]
pub struct RequestSizeGate<S> {
    inner: S,
    limit: usize,
}

impl<S> RequestSizeGate<S> {
    pub fn new(inner: S, limit: usize) -> RequestSizeGate<S> {
        RequestSizeGate { inner, limit }
    }
}

/// The reply to a request over `limit`, as [`RequestSizeGate`] sends it.
pub fn refusal(limit: usize) -> BoxResponse {
    Response::builder()
        .status(StatusCode::PAYLOAD_TOO_LARGE)
        .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from(refusal_text(limit))).boxed())
        .expect("a fixed status, header and body make a valid response")
}

impl<S, B> tower_service::Service<Request<B>> for RequestSizeGate<S>
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
        let limit = self.limit;
        let fut = self.inner.call(req);
        Box::pin(async move {
            let response = fut.await?;
            if response.status() == StatusCode::PAYLOAD_TOO_LARGE {
                return Ok(refusal(limit));
            }
            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_size_is_read_in_bytes_or_with_a_binary_unit() {
        assert_eq!(parse("65536"), Ok(65536));
        assert_eq!(parse("16MiB"), Ok(16 * 1024 * 1024));
        assert_eq!(parse(" 16 mib "), Ok(16 * 1024 * 1024));
        assert!(parse("1GiB").unwrap_err().contains("outside"));
        assert_eq!(parse("512MiB"), Ok(MAX_BYTES));
        assert_eq!(parse("64KiB"), Ok(MIN_BYTES));
        assert_eq!(parse("4194304"), Ok(DEFAULT_BYTES));
    }

    #[test]
    fn a_size_outside_the_bounds_or_not_a_size_is_refused_saying_what_is_accepted() {
        for bad in ["0", "1024", "63KiB", "513MiB", "99999999999999999999999GiB"] {
            let e = parse(bad).unwrap_err();
            assert!(e.contains("64KiB to 512MiB"), "{bad}: {e}");
        }
        for bad in ["", "16MB", "sixteen", "-1", "1.5MiB"] {
            let e = parse(bad).unwrap_err();
            assert!(e.contains("KiB, MiB or GiB"), "{bad}: {e}");
        }
    }

    #[test]
    fn the_refusal_names_the_limit_the_flag_and_its_environment_twin() {
        let t = refusal_text(DEFAULT_BYTES);
        assert!(t.contains("4194304 bytes (4 MiB)"), "{t}");
        assert!(t.contains(FLAG) && t.contains(ENV), "{t}");
        assert!(t.contains("nothing in it was read"), "{t}");
        assert_eq!(
            refusal(DEFAULT_BYTES).status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    #[test]
    fn the_default_is_the_transport_librarys_own() {
        let lib = rmcp::transport::streamable_http_server::StreamableHttpServerConfig::default();
        assert_eq!(lib.max_request_body_bytes, DEFAULT_BYTES);
    }
}
