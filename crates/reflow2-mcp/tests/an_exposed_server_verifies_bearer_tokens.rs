//! AN EXPOSED SERVER VERIFIES BEARER TOKENS ITSELF — the OAuth 2.0
//! resource-server half of GitHub issue #616 fix 4.
//!
//! The ruling is the settled
//! `dec:idea-authentication-is-somebody-elses-layer-and-the-line-is-the-contributor-id`
//! (option (e)): an exposed reflow2 with an OIDC issuer configured validates
//! the Bearer JWT itself (signature against the issuer's published keys, iss,
//! aud, exp; refusing `none`, HS* and any alg that does not fit the key), may
//! require a claim, serves `/.well-known/oauth-protected-resource`, answers
//! 401 with `WWW-Authenticate: Bearer resource_metadata=...`, and derives the
//! contributor from a verified claim through an OPERATOR-configured mapping.
//! The contributor must already exist; reflow2 never invents the person.
//!
//! The floor every check here pins is
//! `req:every-oauth-role-reflow2-plays-meets-oauth-2-1-at-a-minimum` ("WHAT
//! THE FLOOR INCLUDES"): Bearer tokens in the Authorization header only, never
//! a query string; every token validated (signature, issuer, audience
//! restricted to THIS resource per RFC 8707, expiry); failures answer with
//! WWW-Authenticate and the right status (400 invalid_request, 401
//! invalid_token, 403 insufficient_scope); protected resource metadata (RFC
//! 9728); TLS stated, never assumed; local use unchanged.
//!
//! EVERYTHING HERE RUNS THE REAL BINARY over real HTTP, against a test issuer
//! served in-process (OIDC discovery plus a JWKS) and tokens really signed
//! with locally generated keys: RSA (a throwaway 2048-bit key in
//! `fixtures/bearer_test_rsa_2048.pk8`, generated for this test, protecting
//! nothing), and P-256 and Ed25519 keys generated per run.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use ring::rand::SystemRandom;
use ring::signature::{self as sig, KeyPair};
use serde_json::{Value, json};

const RSA_PKCS8: &[u8] = include_bytes!("fixtures/bearer_test_rsa_2048.pk8");
/// The name remote sessions reach the server by — not loopback, so the
/// server is EXPOSED in the ruling's sense.
const PUBLIC_HOST: &str = "reflow2.test";
const PUBLIC_URL: &str = "https://reflow2.test/";
const METADATA_URL: &str = "https://reflow2.test/.well-known/oauth-protected-resource";
const REALM_PATH: &str = "/realms/team";

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-bearer-{name}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// ---- keys and tokens, signed for real ---------------------------------------

/// One signing key the test issuer publishes (or deliberately does not).
enum Key {
    Rsa(sig::RsaKeyPair),
    P256(sig::EcdsaKeyPair),
    Ed(sig::Ed25519KeyPair),
}

struct Signer {
    kid: String,
    key: Key,
}

impl Signer {
    fn rsa(kid: &str) -> Signer {
        Signer {
            kid: kid.into(),
            key: Key::Rsa(sig::RsaKeyPair::from_pkcs8(RSA_PKCS8).expect("the RSA fixture")),
        }
    }

    fn p256(kid: &str) -> Signer {
        let rng = SystemRandom::new();
        let alg = &sig::ECDSA_P256_SHA256_FIXED_SIGNING;
        let pkcs8 = sig::EcdsaKeyPair::generate_pkcs8(alg, &rng).unwrap();
        Signer {
            kid: kid.into(),
            key: Key::P256(sig::EcdsaKeyPair::from_pkcs8(alg, pkcs8.as_ref(), &rng).unwrap()),
        }
    }

    fn ed25519(kid: &str) -> Signer {
        let rng = SystemRandom::new();
        let pkcs8 = sig::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        Signer {
            kid: kid.into(),
            key: Key::Ed(sig::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap()),
        }
    }

    fn alg(&self) -> &'static str {
        match self.key {
            Key::Rsa(_) => "RS256",
            Key::P256(_) => "ES256",
            Key::Ed(_) => "EdDSA",
        }
    }

    /// The public half, as the issuer publishes it in its JWKS.
    fn jwk(&self) -> Value {
        match &self.key {
            Key::Rsa(k) => {
                let c: sig::RsaPublicKeyComponents<Vec<u8>> = k.public().into();
                json!({"kty": "RSA", "kid": self.kid, "use": "sig", "alg": "RS256",
                       "n": B64.encode(&c.n), "e": B64.encode(&c.e)})
            }
            Key::P256(k) => {
                let p = k.public_key().as_ref();
                json!({"kty": "EC", "kid": self.kid, "use": "sig", "alg": "ES256", "crv": "P-256",
                       "x": B64.encode(&p[1..33]), "y": B64.encode(&p[33..65])})
            }
            Key::Ed(k) => {
                json!({"kty": "OKP", "kid": self.kid, "use": "sig", "alg": "EdDSA", "crv": "Ed25519",
                       "x": B64.encode(k.public_key().as_ref())})
            }
        }
    }

    fn sign_bytes(&self, msg: &[u8]) -> Vec<u8> {
        let rng = SystemRandom::new();
        match &self.key {
            Key::Rsa(k) => {
                let mut out = vec![0u8; k.public().modulus_len()];
                k.sign(&sig::RSA_PKCS1_SHA256, &rng, msg, &mut out).unwrap();
                out
            }
            Key::P256(k) => k.sign(&rng, msg).unwrap().as_ref().to_vec(),
            Key::Ed(k) => k.sign(msg).as_ref().to_vec(),
        }
    }

    /// A compact JWS over `claims`, with this key's alg and kid.
    fn token(&self, claims: &Value) -> String {
        self.token_with_header(
            &json!({"alg": self.alg(), "kid": self.kid, "typ": "at+jwt"}),
            claims,
        )
    }

    fn token_with_header(&self, header: &Value, claims: &Value) -> String {
        let msg = format!(
            "{}.{}",
            B64.encode(header.to_string()),
            B64.encode(claims.to_string())
        );
        let s = self.sign_bytes(msg.as_bytes());
        format!("{msg}.{}", B64.encode(s))
    }
}

/// An unsigned token: `alg: none`, empty signature.
fn none_token(claims: &Value) -> String {
    format!(
        "{}.{}.",
        B64.encode(json!({"alg": "none", "typ": "JWT"}).to_string()),
        B64.encode(claims.to_string())
    )
}

/// An HS256 token whose HMAC secret is the RSA key's PUBLIC modulus — the
/// classic algorithm-confusion forgery: anyone can compute it.
fn hs256_token(kid: &str, secret: &[u8], claims: &Value) -> String {
    let msg = format!(
        "{}.{}",
        B64.encode(json!({"alg": "HS256", "kid": kid, "typ": "JWT"}).to_string()),
        B64.encode(claims.to_string())
    );
    let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, secret);
    let tag = ring::hmac::sign(&key, msg.as_bytes());
    format!("{msg}.{}", B64.encode(tag.as_ref()))
}

// ---- a test issuer: OIDC discovery and a JWKS, served in-process -----------

struct Issuer {
    url: String,
    jwks: Arc<Mutex<Value>>,
    jwks_fetches: Arc<AtomicUsize>,
}

impl Issuer {
    fn start(keys: Vec<Value>) -> Issuer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}{REALM_PATH}");
        let jwks = Arc::new(Mutex::new(json!({"keys": keys})));
        let jwks_fetches = Arc::new(AtomicUsize::new(0));
        let (j, f, u) = (Arc::clone(&jwks), Arc::clone(&jwks_fetches), url.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut first = String::new();
                if reader.read_line(&mut first).is_err() {
                    continue;
                }
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                }
                let path = first.split_whitespace().nth(1).unwrap_or("/").to_string();
                let (status, body) =
                    if path == format!("{REALM_PATH}/.well-known/openid-configuration") {
                        (
                        "200 OK",
                        json!({
                            "issuer": u,
                            "jwks_uri": format!("{u}/protocol/openid-connect/certs"),
                            "authorization_endpoint": format!("{u}/protocol/openid-connect/auth"),
                            "token_endpoint": format!("{u}/protocol/openid-connect/token"),
                        })
                        .to_string(),
                    )
                    } else if path == format!("{REALM_PATH}/protocol/openid-connect/certs") {
                        f.fetch_add(1, Ordering::SeqCst);
                        ("200 OK", j.lock().unwrap().to_string())
                    } else {
                        ("404 Not Found", "{}".to_string())
                    };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        Issuer {
            url,
            jwks,
            jwks_fetches,
        }
    }

    fn publish(&self, key: Value) {
        self.jwks.lock().unwrap()["keys"]
            .as_array_mut()
            .unwrap()
            .push(key);
    }

    fn fetches(&self) -> usize {
        self.jwks_fetches.load(Ordering::SeqCst)
    }
}

/// The claims a good token for `user` carries.
fn claims(issuer: &str, user: &str) -> Value {
    let t = now();
    json!({
        "iss": issuer,
        "aud": PUBLIC_URL,
        "sub": format!("sub-{user}"),
        "preferred_username": user,
        "scope": "openid profile reflow2",
        "groups": ["reflow2-team"],
        "iat": t,
        "nbf": t - 5,
        "exp": t + 600,
    })
}

// ---- the real server ---------------------------------------------------------

/// A design holding who:alice and who:bob (and NOT who:carol), plus a
/// decision to settle, seeded through the shell door before any server holds it.
fn seeded_design(name: &str) -> PathBuf {
    let dir = tmp_dir(name);
    let store = dir.join(".reflow2").join("graph");
    for (tool, args) in [
        (
            "add_contributor",
            json!({"id": "who:alice", "name": "Alice", "kind": "person"}),
        ),
        (
            "add_contributor",
            json!({"id": "who:bob", "name": "Bob", "kind": "person"}),
        ),
        (
            "add_project",
            json!({"id": "prj:p", "name": "Bearer test project"}),
        ),
        (
            "add_decision",
            json!({"id": "dec:d", "name": "Choose a log format", "decision": "Use JSON lines."}),
        ),
    ] {
        let out = Command::new(bin())
            .arg("--graph-path")
            .arg(&store)
            .arg("--call")
            .arg(tool)
            .arg("--args")
            .arg(args.to_string())
            .output()
            .expect("run --call");
        assert!(
            out.status.success(),
            "seed {tool}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    store
}

struct Server {
    child: Child,
    port: u16,
    stderr: Arc<Mutex<String>>,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Start the real binary over HTTP with `args`; wait for the address it bound.
fn serve(store: &Path, args: &[&str]) -> Server {
    let mut cmd = Command::new(bin());
    cmd.arg("--graph-path")
        .arg(store)
        .arg("--http")
        .arg("127.0.0.1:0")
        .args(args)
        .env_remove("REFLOW2_TRUSTED_GATEWAY")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn the server");
    let stderr = child.stderr.take().expect("stderr piped");
    let log = Arc::new(Mutex::new(String::new()));
    let (tx, rx) = std::sync::mpsc::channel::<u16>();
    let l = Arc::clone(&log);
    std::thread::spawn(move || {
        let mut sent = false;
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            l.lock().unwrap().push_str(&line);
            l.lock().unwrap().push('\n');
            // The SERVING line, not the first loopback URL: the banner also
            // names the test issuer, which listens on loopback too.
            const SERVING: &str = "over HTTP at http://127.0.0.1:";
            if !sent && let Some(i) = line.find(SERVING) {
                let tail = &line[i + SERVING.len()..];
                let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(p) = digits.parse::<u16>() {
                    let _ = tx.send(p);
                    sent = true;
                }
            }
        }
    });
    let port = match rx.recv_timeout(Duration::from_secs(90)) {
        Ok(p) => p,
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "the server did not come up within 90s. Its stderr:\n{}",
                log.lock().unwrap()
            );
        }
    };
    Server {
        child,
        port,
        stderr: log,
    }
}

/// The standard exposed resource server: discovery against `issuer`, the
/// audience defaulting to the public URL, a required scope and claim, and
/// contributors named `who:{preferred_username}`.
fn resource_server(store: &Path, issuer: &str) -> Server {
    serve(
        store,
        &[
            "--http-allow-host",
            PUBLIC_HOST,
            "--http-public-url",
            PUBLIC_URL,
            "--http-oidc-issuer",
            issuer,
            "--http-oidc-required-scope",
            "reflow2",
            "--http-oidc-required-claim",
            "groups=reflow2-team",
            "--http-contributor-id",
            "who:{preferred_username}",
        ],
    )
}

/// One HTTP exchange: (status, headers lowercased, body de-chunked).
struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    fn challenge(&self) -> String {
        self.header("www-authenticate")
            .unwrap_or_default()
            .to_string()
    }
}

/// One request addressed to the server's PUBLIC name — the exposed case.
fn http(port: u16, method: &str, path: &str, extra: &[(&str, &str)], body: &str) -> Reply {
    http_to(port, PUBLIC_HOST, method, path, extra, body)
}

fn http_to(
    port: u16,
    host: &str,
    method: &str,
    path: &str,
    extra: &[(&str, &str)],
    body: &str,
) -> Reply {
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(120))).ok();
    let mut req = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\n\
         Accept: application/json, text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (k, v) in extra {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(body);
    s.write_all(req.as_bytes()).expect("write request");
    let mut raw = Vec::new();
    let _ = s.read_to_end(&mut raw);
    let raw = String::from_utf8_lossy(&raw).to_string();
    let (head, rest) = raw.split_once("\r\n\r\n").unwrap_or((raw.as_str(), ""));
    let mut lines = head.lines();
    let status = lines
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    let chunked = headers
        .iter()
        .any(|(k, v)| k == "transfer-encoding" && v.contains("chunked"));
    let body = if chunked {
        let mut out = String::new();
        let mut rest = rest;
        while let Some((size, tail)) = rest.split_once("\r\n") {
            let Ok(n) = usize::from_str_radix(size.trim(), 16) else {
                break;
            };
            if n == 0 || tail.len() < n {
                out.push_str(&tail[..n.min(tail.len())]);
                break;
            }
            out.push_str(&tail[..n]);
            rest = tail[n..].strip_prefix("\r\n").unwrap_or(&tail[n..]);
        }
        out
    } else {
        rest.to_string()
    };
    Reply {
        status,
        headers,
        body,
    }
}

fn initialize_body() -> String {
    json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
        "protocolVersion": "2025-06-18", "capabilities": {},
        "clientInfo": {"name": "bearer-test", "version": "1"}}})
    .to_string()
}

/// The JSON-RPC message carrying `id` in a reply body (JSON or SSE).
fn rpc_reply(body: &str, id: u64) -> Option<Value> {
    let mut candidates: Vec<Value> = body
        .lines()
        .filter_map(|l| l.strip_prefix("data:"))
        .filter_map(|d| serde_json::from_str(d.trim()).ok())
        .collect();
    if let Ok(v) = serde_json::from_str::<Value>(body.trim()) {
        candidates.push(v);
    }
    candidates.into_iter().find(|v| v["id"] == id)
}

/// An MCP session over the server, every request carrying `token`.
struct Session {
    port: u16,
    sid: String,
    token: String,
    next: u64,
    instructions: String,
}

impl Session {
    fn open(port: u16, token: &str) -> Session {
        let auth = format!("Bearer {token}");
        let r = http(
            port,
            "POST",
            "/mcp",
            &[("Authorization", &auth)],
            &initialize_body(),
        );
        assert_eq!(
            r.status, 200,
            "a valid token opens a session: {} {:?} {}",
            r.status, r.headers, r.body
        );
        let instructions =
            rpc_reply(&r.body, 0).expect("initialize reply")["result"]["instructions"]
                .as_str()
                .unwrap_or_default()
                .to_string();
        let sid = r
            .header("mcp-session-id")
            .unwrap_or_else(|| panic!("the server hands out a session: {:?}", r.headers))
            .to_string();
        let _ = http(
            port,
            "POST",
            "/mcp",
            &[("Authorization", &auth), ("mcp-session-id", &sid)],
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        );
        Session {
            port,
            sid,
            token: token.to_string(),
            next: 1,
            instructions,
        }
    }

    /// A tools/call; `Ok(structured)` or `Err(refusal text)`.
    fn call(&mut self, tool: &str, args: Value, meta: Option<Value>) -> Result<Value, String> {
        let id = self.next;
        self.next += 1;
        let mut params = json!({"name": tool, "arguments": args});
        if let Some(m) = meta {
            params["_meta"] = m;
        }
        let body = json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": params});
        let auth = format!("Bearer {}", self.token);
        let r = http(
            self.port,
            "POST",
            "/mcp",
            &[("Authorization", &auth), ("mcp-session-id", &self.sid)],
            &body.to_string(),
        );
        assert_eq!(r.status, 200, "{tool}: {} {}", r.status, r.body);
        let v = rpc_reply(&r.body, id).unwrap_or_else(|| panic!("no reply to {tool}: {}", r.body));
        if let Some(e) = v.get("error") {
            return Err(e.to_string());
        }
        if v["result"]["isError"] == true {
            return Err(v["result"]["content"].to_string());
        }
        Ok(v["result"]
            .get("structuredContent")
            .cloned()
            .unwrap_or(Value::Null))
    }

    fn design(&mut self) -> Value {
        self.call("export_graph", json!({}), None)
            .expect("export_graph")
    }
}

/// The AUTHORED_BY edges on `node`: contributor -> roles.
fn signatures_on(doc: &Value, node: &str) -> BTreeMap<String, Value> {
    doc["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .filter(|e| e["edge_type"] == "AUTHORED_BY" && e["from_id"] == node)
        .map(|e| {
            (
                e["to_id"].as_str().unwrap().to_string(),
                e["properties"]["roles"].clone(),
            )
        })
        .collect()
}

fn has_node(doc: &Value, id: &str) -> bool {
    doc["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .any(|n| n["node_id"] == id)
}

/// The parameters of a `Bearer` challenge, by name.
fn challenge_params(challenge: &str) -> BTreeMap<String, String> {
    let rest = challenge
        .strip_prefix("Bearer")
        .unwrap_or_else(|| panic!("a Bearer challenge: {challenge:?}"));
    let mut out = BTreeMap::new();
    let mut rest = rest.trim();
    while !rest.is_empty() {
        let Some((k, v)) = rest.split_once('=') else {
            break;
        };
        let k = k.trim().trim_start_matches(',').trim().to_string();
        let v = v.trim_start();
        let (value, tail) = if let Some(q) = v.strip_prefix('"') {
            let end = q.find('"').expect("a closed quoted value");
            (q[..end].to_string(), &q[end + 1..])
        } else {
            let end = v.find(',').unwrap_or(v.len());
            (v[..end].trim().to_string(), &v[end..])
        };
        out.insert(k, value);
        rest = tail.trim_start_matches(',').trim();
    }
    out
}

/// A refusal at the door: `status`, a Bearer challenge naming the metadata,
/// the `error` code expected (None = no error code), and no JSON-RPC result.
fn assert_refused(r: &Reply, status: u16, error: Option<&str>, what: &str) {
    assert_eq!(
        r.status, status,
        "{what}: {} {:?} {}",
        r.status, r.headers, r.body
    );
    let c = r.challenge();
    let p = challenge_params(&c);
    assert_eq!(
        p.get("resource_metadata").map(String::as_str),
        Some(METADATA_URL),
        "{what}: the challenge names the protected resource metadata: {c}"
    );
    assert_eq!(
        p.get("error").map(String::as_str),
        error,
        "{what}: the error code: {c}"
    );
    assert!(
        rpc_reply(&r.body, 0).is_none() && !r.body.contains("\"result\""),
        "{what}: a refused request reaches nothing: {}",
        r.body
    );
}

fn post_init(port: u16, auth: Option<&str>) -> Reply {
    match auth {
        Some(a) => http(
            port,
            "POST",
            "/mcp",
            &[("Authorization", a)],
            &initialize_body(),
        ),
        None => http(port, "POST", "/mcp", &[], &initialize_body()),
    }
}

// ---- the floor, item by item --------------------------------------------------

/// No token: 401 with `WWW-Authenticate: Bearer resource_metadata=...` and NO
/// error code (RFC 6750 §3.1: a request that carries no credentials gets none),
/// plus the scope the server needs, so an MCP client can run the sign-in.
#[test]
fn a_request_with_no_token_is_challenged_with_the_resource_metadata() {
    let rsa = Signer::rsa("rsa-1");
    let issuer = Issuer::start(vec![rsa.jwk()]);
    let server = resource_server(&seeded_design("no-token"), &issuer.url);
    let r = post_init(server.port, None);
    assert_refused(&r, 401, None, "no token");
    let p = challenge_params(&r.challenge());
    assert_eq!(p.get("scope").map(String::as_str), Some("reflow2"), "{p:?}");
    // A scheme that is not Bearer carries no bearer credentials either.
    let r = post_init(server.port, Some("Basic YWxpY2U6c2VjcmV0"));
    assert_refused(&r, 401, None, "a Basic credential");
}

/// RFC 9728: the metadata is served without a token, at the well-known path,
/// naming THIS resource, its authorization server and the header method only.
#[test]
fn the_protected_resource_metadata_is_served() {
    let rsa = Signer::rsa("rsa-1");
    let issuer = Issuer::start(vec![rsa.jwk()]);
    let server = resource_server(&seeded_design("metadata"), &issuer.url);
    let r = http(
        server.port,
        "GET",
        "/.well-known/oauth-protected-resource",
        &[],
        "",
    );
    assert_eq!(r.status, 200, "{} {}", r.status, r.body);
    assert!(
        r.header("content-type")
            .is_some_and(|c| c.starts_with("application/json")),
        "{:?}",
        r.headers
    );
    let m: Value = serde_json::from_str(&r.body).expect("metadata is JSON");
    assert_eq!(m["resource"], PUBLIC_URL, "{m}");
    assert_eq!(m["authorization_servers"], json!([issuer.url]), "{m}");
    assert_eq!(m["bearer_methods_supported"], json!(["header"]), "{m}");
    assert!(
        m["scopes_supported"]
            .as_array()
            .is_some_and(|s| s.contains(&json!("reflow2"))),
        "{m}"
    );
}

/// A valid token makes the call the MAPPED contributor's: the author of what it
/// writes, the only approver it may name. Signed with RSA, P-256 and Ed25519.
#[test]
fn a_valid_token_makes_the_call_the_mapped_contributors() {
    let rsa = Signer::rsa("rsa-1");
    let ec = Signer::p256("ec-1");
    let ed = Signer::ed25519("ed-1");
    let issuer = Issuer::start(vec![rsa.jwk(), ec.jwk(), ed.jwk()]);
    let server = resource_server(&seeded_design("valid"), &issuer.url);

    let mut s = Session::open(server.port, &rsa.token(&claims(&issuer.url, "alice")));
    assert!(
        s.instructions.contains("BEARER") && s.instructions.contains(&issuer.url),
        "the handshake says how this server knows who is calling: {}",
        s.instructions
    );
    s.call(
        "add_requirement",
        json!({"id": "req:by-alice", "name": "Alice's need", "statement": "The log must rotate."}),
        None,
    )
    .expect("a proposal as alice");
    s.call(
        "set_decision_status",
        json!({"decision_id": "dec:d", "status": "accepted", "approver": "who:alice"}),
        None,
    )
    .expect("alice settles in her own name");
    let doc = s.design();
    let on_req = signatures_on(&doc, "req:by-alice");
    assert_eq!(
        on_req.keys().collect::<Vec<_>>(),
        vec!["who:alice"],
        "the write is credited to the token's contributor: {on_req:?}"
    );
    let on_dec = signatures_on(&doc, "dec:d");
    assert!(
        on_dec
            .get("who:alice")
            .is_some_and(|roles| roles.to_string().contains("approver")),
        "alice is the approver: {on_dec:?}"
    );

    // Her token cannot sign for bob, by argument or by `_meta`.
    let refused = s.call(
        "add_requirement",
        json!({"id": "req:forged", "name": format!("Need {}", "req:forged"), "statement": format!("The system must do the thing {} names.", "req:forged"), "status": "accepted", "approver": "who:bob"}),
        None,
    );
    assert!(refused.is_err(), "a signature in bob's name: {refused:?}");
    let refused = s.call(
        "add_requirement",
        json!({"id": "req:as-bob", "name": format!("Need {}", "req:as-bob"), "statement": format!("The system must do the thing {} names.", "req:as-bob")}),
        Some(json!({"reflow2/writes_for": "who:bob"})),
    );
    assert!(
        refused
            .as_ref()
            .is_err_and(|t| t.contains("who:alice") && t.contains("who:bob")),
        "a call naming bob in _meta is refused, naming both: {refused:?}"
    );
    let doc = s.design();
    assert!(!has_node(&doc, "req:forged") && !has_node(&doc, "req:as-bob"));
    // And a session cannot declare someone else.
    let refused = s.call("writes_for", json!({"contributor_id": "who:bob"}), None);
    assert!(
        refused.is_err(),
        "writes_for(bob) on a token server: {refused:?}"
    );

    for (signer, user) in [(&ec, "bob"), (&ed, "alice")] {
        let mut s = Session::open(server.port, &signer.token(&claims(&issuer.url, user)));
        let id = format!("req:{}-{user}", signer.alg().to_lowercase());
        s.call(
            "add_requirement",
            json!({"id": id, "name": format!("Need {}", id), "statement": format!("The system must do the thing {} names.", id)}),
            None,
        )
        .unwrap_or_else(|e| panic!("{} token for {user}: {e}", signer.alg()));
        let doc = s.design();
        assert_eq!(
            signatures_on(&doc, &id).keys().cloned().collect::<Vec<_>>(),
            vec![format!("who:{user}")],
            "{}",
            signer.alg()
        );
    }
}

/// Wrong issuer, wrong audience (a token minted for another resource),
/// expired, not yet valid, and a signature by a key the issuer never
/// published: each is 401 invalid_token with the challenge.
#[test]
fn a_token_from_the_wrong_issuer_audience_or_time_is_invalid() {
    let rsa = Signer::rsa("rsa-1");
    let issuer = Issuer::start(vec![rsa.jwk()]);
    let server = resource_server(&seeded_design("invalid"), &issuer.url);
    let good = claims(&issuer.url, "alice");
    let with = |k: &str, v: Value| {
        let mut c = good.clone();
        c[k] = v;
        c
    };
    let impostor = Signer::p256("rsa-1"); // claims the published kid, wrong key
    for (what, token) in [
        (
            "wrong issuer",
            rsa.token(&with("iss", json!("https://evil.example/realms/team"))),
        ),
        (
            "wrong audience",
            rsa.token(&with("aud", json!("https://another-resource.example/"))),
        ),
        (
            "no audience",
            rsa.token(&{
                let mut c = good.clone();
                c.as_object_mut().unwrap().remove("aud");
                c
            }),
        ),
        ("expired", rsa.token(&with("exp", json!(now() - 3600)))),
        (
            "not yet valid",
            rsa.token(&with("nbf", json!(now() + 3600))),
        ),
        (
            "no expiry",
            rsa.token(&{
                let mut c = good.clone();
                c.as_object_mut().unwrap().remove("exp");
                c
            }),
        ),
        (
            "a key the issuer never published",
            impostor.token_with_header(&json!({"alg": "ES256", "kid": "ec-unknown"}), &good),
        ),
        ("a signature that does not verify", {
            let t = rsa.token(&good);
            let (head, _) = t.rsplit_once('.').unwrap();
            format!("{head}.{}", B64.encode([7u8; 256]))
        }),
        ("not a JWT", "abc.def".to_string()),
    ] {
        let r = post_init(server.port, Some(&format!("Bearer {token}")));
        assert_refused(&r, 401, Some("invalid_token"), what);
    }
}

/// `alg: none` and HS256 are refused — including the classic confusion that
/// signs HS256 with the RSA key's PUBLIC modulus as the secret.
#[test]
fn alg_none_and_hs256_are_refused() {
    let rsa = Signer::rsa("rsa-1");
    let issuer = Issuer::start(vec![rsa.jwk()]);
    let server = resource_server(&seeded_design("algs"), &issuer.url);
    let good = claims(&issuer.url, "alice");
    let n = B64.decode(rsa.jwk()["n"].as_str().unwrap()).unwrap();
    for (what, token) in [
        ("alg none", none_token(&good)),
        (
            "HS256 with the public modulus",
            hs256_token("rsa-1", &n, &good),
        ),
        (
            "HS256 with the jwk text",
            hs256_token("rsa-1", rsa.jwk().to_string().as_bytes(), &good),
        ),
        ("RS256 header on an EC signature", {
            let ec = Signer::p256("rsa-1");
            ec.token_with_header(&json!({"alg": "RS256", "kid": "rsa-1"}), &good)
        }),
    ] {
        let r = post_init(server.port, Some(&format!("Bearer {token}")));
        assert_refused(&r, 401, Some("invalid_token"), what);
    }
}

/// A token in the URI query string is refused with 400 invalid_request and
/// NEVER used — not even when a valid header comes with it.
#[test]
fn a_token_in_the_query_string_is_refused_and_never_used() {
    let rsa = Signer::rsa("rsa-1");
    let issuer = Issuer::start(vec![rsa.jwk()]);
    let server = resource_server(&seeded_design("query"), &issuer.url);
    let token = rsa.token(&claims(&issuer.url, "alice"));
    let mut s = Session::open(server.port, &token);
    let call = json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call", "params": {
        "name": "add_requirement",
        "arguments": {"id": "req:via-query", "name": format!("Need {}", "req:via-query"), "statement": format!("The system must do the thing {} names.", "req:via-query")}}})
    .to_string();
    let path = format!("/mcp?access_token={token}");
    let r = http(
        server.port,
        "POST",
        &path,
        &[("mcp-session-id", &s.sid)],
        &call,
    );
    assert_refused(
        &r,
        400,
        Some("invalid_request"),
        "a token in the query string",
    );
    let auth = format!("Bearer {token}");
    let r = http(
        server.port,
        "POST",
        &path,
        &[("Authorization", &auth), ("mcp-session-id", &s.sid)],
        &call,
    );
    assert_refused(
        &r,
        400,
        Some("invalid_request"),
        "a query token beside a header",
    );
    assert!(
        !has_node(&s.design(), "req:via-query"),
        "a request carrying a token in its URL wrote nothing"
    );
}

/// A malformed Authorization header is 400 invalid_request.
#[test]
fn a_malformed_authorization_header_is_an_invalid_request() {
    let rsa = Signer::rsa("rsa-1");
    let issuer = Issuer::start(vec![rsa.jwk()]);
    let server = resource_server(&seeded_design("malformed"), &issuer.url);
    let token = rsa.token(&claims(&issuer.url, "alice"));
    for (what, value) in [
        ("an empty bearer", "Bearer".to_string()),
        ("an empty bearer with a space", "Bearer ".to_string()),
        ("two tokens", format!("Bearer {token} {token}")),
        (
            "characters outside b64token",
            "Bearer abc!def{}".to_string(),
        ),
    ] {
        let r = post_init(server.port, Some(&value));
        assert_refused(&r, 400, Some("invalid_request"), what);
    }
    let auth = format!("Bearer {token}");
    let r = http(
        server.port,
        "POST",
        "/mcp",
        &[("Authorization", &auth), ("Authorization", &auth)],
        &initialize_body(),
    );
    assert_refused(
        &r,
        400,
        Some("invalid_request"),
        "two Authorization headers",
    );
}

/// A valid token without the scope (or claim) the operator requires is 403
/// insufficient_scope, naming the scope.
#[test]
fn a_token_without_the_required_scope_or_claim_is_insufficient_scope() {
    let rsa = Signer::rsa("rsa-1");
    let issuer = Issuer::start(vec![rsa.jwk()]);
    let server = resource_server(&seeded_design("scope"), &issuer.url);
    let mut no_scope = claims(&issuer.url, "alice");
    no_scope["scope"] = json!("openid profile");
    let mut no_group = claims(&issuer.url, "alice");
    no_group["groups"] = json!(["another-team"]);
    let mut no_claim = claims(&issuer.url, "alice");
    no_claim.as_object_mut().unwrap().remove("groups");
    for (what, c) in [
        ("without the scope", no_scope),
        ("in another group", no_group),
        ("with no groups claim", no_claim),
    ] {
        let r = post_init(server.port, Some(&format!("Bearer {}", rsa.token(&c))));
        assert_refused(&r, 403, Some("insufficient_scope"), what);
        let p = challenge_params(&r.challenge());
        assert_eq!(
            p.get("scope").map(String::as_str),
            Some("reflow2"),
            "{what}: {p:?}"
        );
    }
}

/// A token whose mapped contributor the design does not hold authenticates,
/// can read, and cannot write: reflow2 never invents the person.
#[test]
fn a_contributor_the_design_does_not_hold_is_refused_and_never_invented() {
    let rsa = Signer::rsa("rsa-1");
    let issuer = Issuer::start(vec![rsa.jwk()]);
    let server = resource_server(&seeded_design("carol"), &issuer.url);
    let mut s = Session::open(server.port, &rsa.token(&claims(&issuer.url, "carol")));
    s.call("get_node", json!({"id": "dec:d"}), None)
        .expect("a read works for a verified caller");
    let refused = s.call(
        "add_requirement",
        json!({"id": "req:by-carol", "name": format!("Need {}", "req:by-carol"), "statement": format!("The system must do the thing {} names.", "req:by-carol")}),
        None,
    );
    assert!(
        refused.as_ref().is_err_and(|t| t.contains("who:carol")),
        "carol's write is refused, naming her: {refused:?}"
    );
    let doc = s.design();
    assert!(
        !has_node(&doc, "who:carol") && !has_node(&doc, "req:by-carol"),
        "nothing was invented and nothing written"
    );
}

/// A session belongs to the sign-in that opened it: another valid token
/// quoting its id is turned away (as an unknown session, 404) and nothing
/// behind the gate runs. The session is never what authenticates — each
/// request still proves its own token.
#[test]
fn a_session_is_bound_to_the_sign_in_that_opened_it() {
    let rsa = Signer::rsa("rsa-1");
    let issuer = Issuer::start(vec![rsa.jwk()]);
    let server = resource_server(&seeded_design("session"), &issuer.url);
    let mut alice = Session::open(server.port, &rsa.token(&claims(&issuer.url, "alice")));
    let bob_token = rsa.token(&claims(&issuer.url, "bob"));
    let call = json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": {
        "name": "add_requirement",
        "arguments": {"id": "req:in-alices-session", "name": "Borrowed", "statement": "Bob writes through Alice's session."}}})
    .to_string();
    let bob_auth = format!("Bearer {bob_token}");
    let r = http(
        server.port,
        "POST",
        "/mcp",
        &[
            ("Authorization", bob_auth.as_str()),
            ("mcp-session-id", alice.sid.as_str()),
        ],
        &call,
    );
    assert_eq!(r.status, 404, "{} {}", r.status, r.body);
    assert!(!has_node(&alice.design(), "req:in-alices-session"));
    // Alice's own next request in her session still works.
    alice
        .call("get_node", json!({"id": "dec:d"}), None)
        .expect("the owner keeps her session");
}

/// A key the server has not seen (the issuer rotated) is fetched once, by kid;
/// an unknown kid inside the refresh cooldown is refused without a fetch.
#[test]
fn an_unknown_key_id_refreshes_the_key_set() {
    let rsa = Signer::rsa("rsa-1");
    let issuer = Issuer::start(vec![rsa.jwk()]);
    let server = resource_server(&seeded_design("rotate"), &issuer.url);
    let _ = Session::open(server.port, &rsa.token(&claims(&issuer.url, "alice")));
    let before = issuer.fetches();
    assert!(before >= 1, "the key set was fetched");

    let rotated = Signer::p256("ec-rotated");
    issuer.publish(rotated.jwk());
    let mut s = Session::open(server.port, &rotated.token(&claims(&issuer.url, "bob")));
    s.call("get_node", json!({"id": "dec:d"}), None)
        .expect("the rotated key verifies");
    assert_eq!(issuer.fetches(), before + 1, "one fetch for the new kid");

    let stranger = Signer::p256("never-published");
    let r = post_init(
        server.port,
        Some(&format!(
            "Bearer {}",
            stranger.token(&claims(&issuer.url, "bob"))
        )),
    );
    assert_refused(&r, 401, Some("invalid_token"), "an unknown kid");
    assert_eq!(
        issuer.fetches(),
        before + 1,
        "an unknown kid inside the cooldown does not make the server fetch again"
    );
}

/// A pinned key set from a file: no network at all — the issuer's host is
/// never contacted, and a token still verifies.
#[test]
fn a_pinned_key_set_needs_no_network() {
    let ec = Signer::p256("ec-pinned");
    let dir = tmp_dir("pinned");
    let jwks = dir.join("jwks.json");
    std::fs::write(&jwks, json!({"keys": [ec.jwk()]}).to_string()).unwrap();
    // A host that does not resolve: any attempt to reach it would fail.
    let issuer = "https://sso.unreachable.invalid/realms/team";
    let store = seeded_design("pinned-design");
    let server = serve(
        &store,
        &[
            "--http-allow-host",
            PUBLIC_HOST,
            "--http-public-url",
            PUBLIC_URL,
            "--http-oidc-issuer",
            issuer,
            "--http-oidc-jwks-file",
            jwks.to_str().unwrap(),
            "--http-contributor-id",
            "who:{preferred_username}",
        ],
    );
    let mut c = claims(issuer, "alice");
    c["scope"] = json!("openid");
    let mut s = Session::open(server.port, &ec.token(&c));
    s.call(
        "add_requirement",
        json!({"id": "req:pinned", "name": format!("Need {}", "req:pinned"), "statement": format!("The system must do the thing {} names.", "req:pinned")}),
        None,
    )
    .expect("a pinned key verifies");
    assert_eq!(
        signatures_on(&s.design(), "req:pinned")
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["who:alice".to_string()]
    );
}

/// A contributor map: the template renders a KEY (here the stable `sub`), and
/// the operator's file says which Contributor it is. A key not in the map
/// names nobody: it reads, and cannot sign or write in anyone's name.
#[test]
fn an_operator_map_names_the_contributor() {
    let rsa = Signer::rsa("rsa-1");
    let issuer = Issuer::start(vec![rsa.jwk()]);
    let dir = tmp_dir("map");
    let map = dir.join("contributors.toml");
    std::fs::write(
        &map,
        "# which Contributor a verified sign-in is\n[contributors]\n\"sub-al\" = \"who:alice\"\n",
    )
    .unwrap();
    let store = seeded_design("map-design");
    let server = serve(
        &store,
        &[
            "--http-allow-host",
            PUBLIC_HOST,
            "--http-public-url",
            PUBLIC_URL,
            "--http-oidc-issuer",
            &issuer.url,
            "--http-contributor-id",
            "{sub}",
            "--http-contributor-map",
            map.to_str().unwrap(),
        ],
    );
    let mut c = claims(&issuer.url, "someone-else");
    c["sub"] = json!("sub-al");
    let mut s = Session::open(server.port, &rsa.token(&c));
    s.call(
        "set_decision_status",
        json!({"decision_id": "dec:d", "status": "accepted", "approver": "who:alice"}),
        None,
    )
    .expect("the mapped contributor signs");
    let mut unmapped = claims(&issuer.url, "alice");
    unmapped["sub"] = json!("sub-not-in-the-map");
    let mut s = Session::open(server.port, &rsa.token(&unmapped));
    s.call("get_node", json!({"id": "dec:d"}), None)
        .expect("an unmapped caller still reads");
    let refused = s.call(
        "add_requirement",
        json!({"id": "req:unmapped", "name": format!("Need {}", "req:unmapped"), "statement": format!("The system must do the thing {} names.", "req:unmapped"), "status": "accepted", "approver": "who:alice"}),
        None,
    );
    assert!(
        refused.is_err(),
        "an unmapped caller cannot sign: {refused:?}"
    );
}

// ---- what stays the same, and what is refused at startup ----------------------

/// Local serving takes no OAuth: loopback `--http` with no issuer answers with
/// no token at all, exactly as before.
#[test]
fn loopback_http_without_an_issuer_takes_no_token() {
    let server = serve(&seeded_design("local"), &[]);
    let r = http_to(
        server.port,
        "127.0.0.1",
        "POST",
        "/mcp",
        &[],
        &initialize_body(),
    );
    assert_eq!(r.status, 200, "{} {}", r.status, r.body);
    assert!(r.header("www-authenticate").is_none());
}

fn startup_refusal(args: &[&str]) -> String {
    let out = Command::new(bin())
        .args(args)
        .env_remove("REFLOW2_TRUSTED_GATEWAY")
        .stdin(Stdio::null())
        .output()
        .expect("run reflow2-mcp");
    let text = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        !out.status.success(),
        "{args:?} must be refused at startup, and it ran: {text}"
    );
    text
}

/// A token can only arrive over HTTP: the issuer flags on stdio or `--shared`
/// are refused at startup rather than silently ignored, so stdio and
/// `--shared` stay exactly as they are. And a server declares ONE way of
/// knowing who is calling: an issuer beside a trusted gateway is refused.
#[test]
fn the_issuer_is_refused_where_a_token_cannot_arrive_or_beside_a_gateway() {
    let store = seeded_design("refusals");
    let s = store.to_str().unwrap();
    let issuer = "https://sso.example.org/realms/team";
    let base = [
        "--graph-path",
        s,
        "--http-oidc-issuer",
        issuer,
        "--http-public-url",
        PUBLIC_URL,
        "--http-contributor-id",
        "who:{sub}",
    ];
    let text = startup_refusal(&base);
    assert!(text.contains("--http"), "stdio: {text}");
    let text = startup_refusal(&[&base[..], &["--shared"]].concat());
    assert!(text.contains("--shared"), "--shared: {text}");
    let text = startup_refusal(
        &[
            &base[..],
            &[
                "--http",
                "127.0.0.1:0",
                "--http-trusted-gateway",
                "gw.example",
            ],
        ]
        .concat(),
    );
    assert!(
        text.contains("--http-trusted-gateway") && text.contains("--http-oidc-issuer"),
        "both declared: {text}"
    );
    // An issuer with no way of naming the contributor is a misconfiguration.
    let text = startup_refusal(&[
        "--graph-path",
        s,
        "--http",
        "127.0.0.1:0",
        "--http-oidc-issuer",
        issuer,
        "--http-public-url",
        PUBLIC_URL,
    ]);
    assert!(text.contains("--http-contributor-id"), "no mapping: {text}");
}

/// TLS is STATED, never assumed: reflow2 does not terminate TLS, the help and
/// the startup banner say how it is met, and a public URL that would carry
/// tokens in the clear off this machine is refused.
#[test]
fn tls_is_stated_and_a_cleartext_public_url_is_refused() {
    let help = Command::new(bin()).arg("--help").output().expect("--help");
    let help = String::from_utf8_lossy(&help.stdout).to_string();
    assert!(
        help.contains("does not terminate TLS") || help.contains("does NOT terminate TLS"),
        "--help states how TLS is met: {help}"
    );
    let store = seeded_design("tls");
    let text = startup_refusal(&[
        "--graph-path",
        store.to_str().unwrap(),
        "--http",
        "127.0.0.1:0",
        "--http-allow-host",
        "reflow2.example.org",
        "--http-oidc-issuer",
        "https://sso.example.org/realms/team",
        "--http-public-url",
        "http://reflow2.example.org/",
        "--http-contributor-id",
        "who:{sub}",
    ]);
    assert!(text.contains("TLS"), "a cleartext public URL: {text}");

    let rsa = Signer::rsa("rsa-1");
    let issuer = Issuer::start(vec![rsa.jwk()]);
    let server = resource_server(&store, &issuer.url);
    let started = Instant::now();
    loop {
        let log = server.stderr.lock().unwrap().clone();
        if log.contains("terminate TLS") {
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the startup banner states how TLS is met: {log}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}
