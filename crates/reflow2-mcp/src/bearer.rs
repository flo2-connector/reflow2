//! AN EXPOSED SERVER VERIFIES BEARER TOKENS ITSELF — reflow2 as an OAuth 2.0
//! RESOURCE SERVER (GitHub issue #616 fix 4, the half #636 left owed).
//!
//! The ruling is the settled
//! `dec:idea-authentication-is-somebody-elses-layer-and-the-line-is-the-contributor-id`,
//! option (e): an exposed server with an OIDC issuer configured validates the
//! Bearer JWT itself — the signature against the issuer's published keys, plus
//! iss, aud and exp — may require a claim, serves
//! `/.well-known/oauth-protected-resource`, answers 401 with a
//! `WWW-Authenticate` header naming it, and derives the person from a verified
//! claim through an OPERATOR-configured mapping. reflow2 verifies; it never
//! issues identity: no sign-in flow, no sessions, no user table.
//!
//! The floor is `req:every-oauth-role-reflow2-plays-meets-oauth-2-1-at-a-minimum`
//! (OAuth 2.1, draft-ietf-oauth-v2-1, with the MCP authorization profile on
//! top where it is stricter — RFC 9728 metadata and RFC 8707 audiences):
//!
//! · A token is read from the `Authorization: Bearer` header ONLY. One in the
//!   URI query string is refused (400 invalid_request) and never used, even
//!   beside a valid header. A form-encoded body is never read: MCP bodies are
//!   JSON, so there is no body method to accept.
//! · Every token is validated before anything behind the gate runs: the
//!   signature against a key the issuer published, `iss` equal to the
//!   configured issuer, `aud` naming THIS resource, `exp` (required) and `nbf`.
//!   `alg: none` and every HS* algorithm are refused before any key is looked
//!   at, and so is an alg that does not fit the key it names.
//! · Failures answer with the status OAuth names and a `WWW-Authenticate:
//!   Bearer` challenge carrying `resource_metadata`: 400 invalid_request, 401
//!   invalid_token (or 401 with no error code when there is no token at all,
//!   RFC 6750 §3.1), 403 insufficient_scope.
//! · The token STOPS AT THIS GATE. The `Authorization` header is removed
//!   before the request reaches the MCP service, so no handler can read it,
//!   log it or pass it to another service.
//! · TLS is STATED, never assumed. reflow2 does not terminate TLS: it serves
//!   plain HTTP, and the public URL clients use must be `https://` (a TLS-
//!   terminating proxy in front) unless it is loopback.
//!
//! WHERE THE KEYS COME FROM — always from the OPERATOR, never from a token:
//! discovery at the configured issuer (`/.well-known/openid-configuration`,
//! then RFC 8414's `/.well-known/oauth-authorization-server`), an explicit
//! `--http-oidc-jwks-uri`, or a pinned `--http-oidc-jwks-file` that needs no
//! network at all. A token's `jku`, `x5u` or embedded `jwk` header is never
//! followed: a server that fetched a key a token pointed at would verify the
//! token against the forger's own key.
//!
//! The key set is cached, looked up by `kid`, refreshed when it is older than
//! [`KEYS_TTL`], and refreshed AT ONCE when a token names a kid it does not
//! hold (the issuer rotated) — at most once per [`UNKNOWN_KID_COOLDOWN`], so a
//! stream of made-up kids cannot turn this server into a fetch amplifier.
//!
//! The crypto is `ring`, the library this binary's TLS already uses, plugged
//! into `jsonwebtoken` as its provider ([`install_crypto`]): one crypto stack,
//! no C toolchain beyond the one rustls needs, and verification only — the
//! provider refuses to sign, because reflow2 issues no tokens.

use std::collections::{BTreeMap, HashSet};
use std::convert::Infallible;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use base64::Engine;
use bytes::Bytes;
use http::{HeaderMap, Request, Response, StatusCode, Uri};
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use jsonwebtoken::jwk::{
    AlgorithmParameters, EllipticCurve, Jwk, KeyOperations, PublicKeyUse, ThumbprintHash,
};
use jsonwebtoken::{Algorithm, AlgorithmFamily, DecodingKey, DecodingKeyKind, Validation};
use serde_json::{Map, Value, json};

type BoxResponse = Response<BoxBody<Bytes, Infallible>>;

/// Where RFC 9728 puts a protected resource's metadata.
pub const METADATA_PATH: &str = "/.well-known/oauth-protected-resource";

/// How long a fetched key set is trusted before the next request refreshes it,
/// so a key the issuer withdrew stops verifying within this bound.
pub const KEYS_TTL: Duration = Duration::from_secs(10 * 60);

/// The least time between two refreshes a token's unknown `kid` triggers.
pub const UNKNOWN_KID_COOLDOWN: Duration = Duration::from_secs(30);

/// The least time between two attempts to load an empty or stale key set, so
/// an issuer that is down is not asked again by every request.
pub const RETRY_WHEN_EMPTY: Duration = Duration::from_secs(5);

/// Clock skew allowed on `exp` and `nbf`, in seconds.
pub const LEEWAY_SECS: u64 = 60;

/// A token longer than this is refused unread. Real access tokens are a few
/// kilobytes; the bound keeps a hostile header from costing a decode.
pub const MAX_TOKEN_BYTES: usize = 32 * 1024;

/// The algorithms a token may be signed with: asymmetric only. `none` and the
/// HS* family are absent BY DESIGN — a key set publishes no shared secret, and
/// an HS* token "verified" against a public key is the classic forgery.
pub const ACCEPTED_ALGS: [Algorithm; 9] = [
    Algorithm::RS256,
    Algorithm::RS384,
    Algorithm::RS512,
    Algorithm::PS256,
    Algorithm::PS384,
    Algorithm::PS512,
    Algorithm::ES256,
    Algorithm::ES384,
    Algorithm::EdDSA,
];

// ---- configuration -----------------------------------------------------------

/// Where the issuer's public keys come from. Always the operator's choice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeySource {
    /// OpenID discovery at the issuer, then RFC 8414 metadata.
    Discover,
    /// The key set at this URL.
    Uri(String),
    /// A pinned key set in this file: no network at all.
    File(PathBuf),
}

/// One claim the operator requires, `NAME=VALUE`: the claim equals VALUE, or
/// is an array holding it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiredClaim {
    pub name: String,
    pub value: String,
}

/// Which Contributor a verified token is — OPERATOR configuration, never a
/// property of the Contributor: any caller can upsert a Contributor through
/// `add_contributor`, and a mapping stored there would let them re-point
/// someone else's sign-in.
///
/// The template renders from verified claims (`who:{preferred_username}`).
/// With no map, the rendering IS the contributor id. With a map, it is the KEY
/// looked up there, and a key the map does not hold names nobody.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContributorMapping {
    template: String,
    pieces: Vec<Piece>,
    map: Option<(String, BTreeMap<String, String>)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Piece {
    Text(String),
    Claim(String),
}

impl ContributorMapping {
    /// Parse `template` (and the map file's contents, if any).
    pub fn new(template: &str, map: Option<(String, String)>) -> Result<Self, String> {
        let mut pieces = Vec::new();
        let mut rest = template;
        while !rest.is_empty() {
            match rest.find('{') {
                None => {
                    if rest.contains('}') {
                        return Err(format!(
                            "--http-contributor-id `{template}` has a `}}` with no `{{`"
                        ));
                    }
                    pieces.push(Piece::Text(rest.to_string()));
                    rest = "";
                }
                Some(open) => {
                    if open > 0 {
                        let text = &rest[..open];
                        if text.contains('}') {
                            return Err(format!(
                                "--http-contributor-id `{template}` has a `}}` with no `{{`"
                            ));
                        }
                        pieces.push(Piece::Text(text.to_string()));
                    }
                    let after = &rest[open + 1..];
                    let close = after.find('}').ok_or_else(|| {
                        format!("--http-contributor-id `{template}` has a `{{` with no `}}`")
                    })?;
                    let name = &after[..close];
                    if name.is_empty()
                        || !name
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || "_-.:".contains(c))
                    {
                        return Err(format!(
                            "--http-contributor-id `{template}`: `{{{name}}}` is not a claim name"
                        ));
                    }
                    pieces.push(Piece::Claim(name.to_string()));
                    rest = &after[close + 1..];
                }
            }
        }
        if !pieces.iter().any(|p| matches!(p, Piece::Claim(_))) {
            return Err(format!(
                "--http-contributor-id `{template}` names no claim, so every caller would be the \
                 same contributor. Put the claim that identifies the person in braces, for \
                 example `who:{{preferred_username}}` or `{{sub}}`."
            ));
        }
        let map = match map {
            None => None,
            Some((path, text)) => Some((path.clone(), parse_map(&path, &text)?)),
        };
        Ok(ContributorMapping {
            template: template.to_string(),
            pieces,
            map,
        })
    }

    /// The mapping in a phrase, for the handshake, the banner and refusals.
    pub fn describe(&self) -> String {
        match &self.map {
            None => format!("`{}`", self.template),
            Some((path, m)) => format!(
                "`{}` looked up in the contributor map {path} ({} entr{})",
                self.template,
                m.len(),
                if m.len() == 1 { "y" } else { "ies" }
            ),
        }
    }

    /// The contributor these verified claims name, or why they name nobody.
    pub fn contributor(&self, claims: &Map<String, Value>) -> Result<String, String> {
        let mut out = String::new();
        for p in &self.pieces {
            match p {
                Piece::Text(t) => out.push_str(t),
                Piece::Claim(name) => match claims.get(name) {
                    Some(Value::String(s))
                        if !s.is_empty()
                            && !s.chars().any(|c| c.is_whitespace() || c.is_control()) =>
                    {
                        out.push_str(s)
                    }
                    Some(Value::String(_)) => {
                        return Err(format!(
                            "the token's `{name}` claim is empty or holds whitespace, so the \
                             operator's mapping {} names nobody",
                            self.describe()
                        ));
                    }
                    Some(_) => {
                        return Err(format!(
                            "the token's `{name}` claim is not a string, so the operator's mapping \
                             {} names nobody",
                            self.describe()
                        ));
                    }
                    None => {
                        return Err(format!(
                            "the token carries no `{name}` claim, so the operator's mapping {} \
                             names nobody",
                            self.describe()
                        ));
                    }
                },
            }
        }
        match &self.map {
            None => Ok(out),
            Some((path, m)) => m.get(&out).cloned().ok_or_else(|| {
                format!(
                    "the token maps to `{out}`, which the operator's contributor map {path} does \
                     not hold. The operator adds it there to let this person sign."
                )
            }),
        }
    }
}

/// The contributor map file: a TOML `[contributors]` table, key → Contributor id.
fn parse_map(path: &str, text: &str) -> Result<BTreeMap<String, String>, String> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct MapFile {
        contributors: BTreeMap<String, String>,
    }
    let parsed: MapFile = toml::from_str(text).map_err(|e| {
        format!(
            "the contributor map {path} is not a `[contributors]` table of \"key\" = \
             \"who:...\" lines: {e}"
        )
    })?;
    for (k, v) in &parsed.contributors {
        if v.trim().is_empty() {
            return Err(format!(
                "the contributor map {path} maps `{k}` to an empty contributor id"
            ));
        }
    }
    Ok(parsed.contributors)
}

/// Everything the operator declared about this resource server.
#[derive(Clone, Debug)]
pub struct BearerConfig {
    /// The issuer identifier; a token's `iss` must equal it exactly.
    pub issuer: String,
    /// The audience a token must name: THIS resource (RFC 8707).
    pub audience: String,
    /// The resource identifier: the https:// URL clients reach this server at.
    pub resource: String,
    pub keys: KeySource,
    pub required_scopes: Vec<String>,
    pub required_claims: Vec<RequiredClaim>,
    pub mapping: ContributorMapping,
}

/// Is this authority's host this machine?
fn host_is_loopback(uri: &Uri) -> bool {
    uri.host()
        .map(|h| crate::mcp_http::is_loopback_host(h.trim_start_matches('[').trim_end_matches(']')))
        .unwrap_or(false)
}

/// An absolute https:// URL (or http:// on loopback) with no query or
/// fragment, as `what` says it. `why_tls` finishes the sentence a cleartext
/// URL is refused with.
fn checked_url(what: &str, raw: &str, why_tls: &str) -> Result<Uri, String> {
    let uri: Uri = raw
        .parse()
        .map_err(|_| format!("{what} `{raw}` is not a URL"))?;
    let scheme = uri.scheme_str().unwrap_or("");
    if uri.authority().is_none() || !(scheme == "https" || scheme == "http") {
        return Err(format!("{what} `{raw}` must be an absolute https:// URL"));
    }
    if uri.query().is_some() || raw.contains('#') {
        return Err(format!("{what} `{raw}` must carry no query or fragment"));
    }
    if scheme == "http" && !host_is_loopback(&uri) {
        return Err(format!(
            "{what} `{raw}` is plain http:// on a host that is not this machine. {why_tls}"
        ));
    }
    Ok(uri)
}

impl BearerConfig {
    /// Validate what the operator declared. Every refusal names the flag.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        issuer: &str,
        audience: Option<&str>,
        public_url: &str,
        jwks_uri: Option<&str>,
        jwks_file: Option<&str>,
        required_scopes: &[String],
        required_claims: &[String],
        mapping: ContributorMapping,
    ) -> Result<BearerConfig, String> {
        checked_url(
            "--http-oidc-issuer",
            issuer,
            "An issuer's keys and discovery document are fetched only over TLS.",
        )?;
        checked_url(
            "--http-public-url",
            public_url,
            "Tokens must only cross TLS, and reflow2 does NOT terminate TLS: put a TLS-terminating \
             proxy in front of this server and give its https:// URL here. Only a loopback URL may \
             be http://, because a token sent to it never leaves this machine.",
        )?;
        let keys = match (jwks_uri, jwks_file) {
            (Some(_), Some(_)) => {
                return Err(
                    "--http-oidc-jwks-uri and --http-oidc-jwks-file both name the issuer's keys; \
                     pass one"
                        .to_string(),
                );
            }
            (Some(u), None) => {
                checked_url(
                    "--http-oidc-jwks-uri",
                    u,
                    "A key set is fetched only over TLS.",
                )?;
                KeySource::Uri(u.to_string())
            }
            (None, Some(f)) => KeySource::File(PathBuf::from(f)),
            (None, None) => KeySource::Discover,
        };
        let mut claims = Vec::new();
        for c in required_claims {
            let (name, value) = c.split_once('=').ok_or_else(|| {
                format!("--http-oidc-required-claim `{c}` must be NAME=VALUE, for example groups=reflow2")
            })?;
            let (name, value) = (name.trim(), value.trim());
            if name.is_empty() || value.is_empty() {
                return Err(format!(
                    "--http-oidc-required-claim `{c}` must be NAME=VALUE with both parts given"
                ));
            }
            claims.push(RequiredClaim {
                name: name.to_string(),
                value: value.to_string(),
            });
        }
        for s in required_scopes {
            if s.is_empty()
                || s.chars()
                    .any(|c| c.is_whitespace() || c == '"' || c == '\\')
            {
                return Err(format!(
                    "--http-oidc-required-scope `{s}` is not one scope token (no spaces or quotes)"
                ));
            }
        }
        Ok(BearerConfig {
            issuer: issuer.to_string(),
            audience: audience
                .map(str::to_string)
                .unwrap_or_else(|| public_url.to_string()),
            resource: public_url.to_string(),
            keys,
            required_scopes: required_scopes.to_vec(),
            required_claims: claims,
            mapping,
        })
    }

    /// The resource's path with no trailing slash ("" for a bare host).
    fn resource_path(&self) -> String {
        let uri: Uri = self.resource.parse().expect("validated in new");
        uri.path().trim_end_matches('/').to_string()
    }

    /// The URL of this resource's metadata (RFC 9728 §3.1: the well-known
    /// suffix goes between the host and the resource's path).
    pub fn metadata_url(&self) -> String {
        let uri: Uri = self.resource.parse().expect("validated in new");
        format!(
            "{}://{}{METADATA_PATH}{}",
            uri.scheme_str().unwrap_or("https"),
            uri.authority().map(|a| a.as_str()).unwrap_or(""),
            self.resource_path()
        )
    }

    /// Is `path` where this server answers its metadata?
    pub fn is_metadata_path(&self, path: &str) -> bool {
        let p = path.trim_end_matches('/');
        p == METADATA_PATH || p == format!("{METADATA_PATH}{}", self.resource_path())
    }

    /// The RFC 9728 document.
    pub fn metadata(&self) -> Value {
        let mut m = json!({
            "resource": self.resource,
            "authorization_servers": [self.issuer],
            "bearer_methods_supported": ["header"],
            "resource_name": "reflow2",
        });
        if !self.required_scopes.is_empty() {
            m["scopes_supported"] = json!(self.required_scopes);
        }
        m
    }

    /// How the keys are obtained, in a phrase.
    pub fn describe_keys(&self) -> String {
        match &self.keys {
            KeySource::Discover => format!("discovered from {}", self.issuer),
            KeySource::Uri(u) => format!("fetched from {u}"),
            KeySource::File(p) => format!("pinned in {} (no network)", p.display()),
        }
    }
}

// ---- what the gate hands the service -----------------------------------------

/// What the gate established about the request now arriving, put into its
/// HTTP extensions for the MCP service to read (`ReflowService::call_tool`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedCaller {
    /// The issuer that signed the token.
    pub issuer: String,
    /// The token's `sub`, if it carried one.
    pub subject: Option<String>,
    /// The Contributor the operator's mapping derives, or `None`.
    pub contributor: Option<String>,
    /// Why the mapping named nobody, when it did not.
    pub unmapped: Option<String>,
}

/// Why a request was refused at the gate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// No bearer credentials at all: 401 with no error code.
    NoToken,
    /// 400 invalid_request.
    InvalidRequest(String),
    /// 401 invalid_token.
    InvalidToken(String),
    /// 403 insufficient_scope.
    InsufficientScope(String),
    /// The issuer's keys cannot be had right now: 503. Not the token's fault,
    /// so it is not called invalid — a client would throw a good token away.
    KeysUnavailable(String),
}

// ---- the key set ---------------------------------------------------------------

struct CachedKey {
    kid: Option<String>,
    jwk: Jwk,
}

impl CachedKey {
    /// Does this key fit `alg`? Its type and curve, its declared `alg`, `use`
    /// and `key_ops` must all allow verifying a signature of that algorithm.
    fn fits(&self, alg: Algorithm) -> bool {
        let family_fits = match (&self.jwk.algorithm, alg.family()) {
            (AlgorithmParameters::RSA(_), AlgorithmFamily::Rsa) => true,
            (AlgorithmParameters::EllipticCurve(p), AlgorithmFamily::Ec) => matches!(
                (&p.curve, alg),
                (EllipticCurve::P256, Algorithm::ES256) | (EllipticCurve::P384, Algorithm::ES384)
            ),
            (AlgorithmParameters::OctetKeyPair(p), AlgorithmFamily::Ed) => {
                p.curve == EllipticCurve::Ed25519
            }
            _ => false,
        };
        let declared_alg_fits = match &self.jwk.common.key_algorithm {
            None => true,
            Some(k) => format!("{k:?}") == format!("{alg:?}"),
        };
        let use_fits = matches!(
            self.jwk.common.public_key_use,
            None | Some(PublicKeyUse::Signature)
        );
        let ops_fit = self
            .jwk
            .common
            .key_operations
            .as_ref()
            .is_none_or(|ops| ops.contains(&KeyOperations::Verify));
        family_fits && declared_alg_fits && use_fits && ops_fit
    }
}

/// Parse a key set leniently: a key this server cannot use (an encryption
/// key, a symmetric one, a type it does not know) is skipped, not fatal —
/// Keycloak publishes its RSA-OAEP encryption key beside its signing keys.
fn parse_key_set(doc: &Value) -> Result<Vec<CachedKey>, String> {
    let keys = doc
        .get("keys")
        .and_then(Value::as_array)
        .ok_or("it is not a JSON Web Key Set (no `keys` array)")?;
    let mut out = Vec::new();
    for k in keys {
        let Ok(jwk) = serde_json::from_value::<Jwk>(k.clone()) else {
            continue;
        };
        if matches!(
            jwk.algorithm,
            AlgorithmParameters::OctetKey(_) | AlgorithmParameters::Other(_)
        ) {
            continue;
        }
        out.push(CachedKey {
            kid: jwk.common.key_id.clone(),
            jwk,
        });
    }
    if out.is_empty() {
        return Err("it holds no public key this server can verify a signature with".into());
    }
    Ok(out)
}

#[derive(Default)]
struct KeySet {
    keys: Vec<CachedKey>,
    loaded_at: Option<Instant>,
}

#[derive(Default)]
struct FetchState {
    /// The key set's URL, once discovery found it.
    jwks_uri: Option<String>,
    last_attempt: Option<Instant>,
    last_unknown_kid_refresh: Option<Instant>,
    last_error: Option<String>,
}

/// The verifier: configuration plus the cached key set. One per server.
pub struct Verifier {
    config: BearerConfig,
    keys: RwLock<Arc<KeySet>>,
    fetch: tokio::sync::Mutex<FetchState>,
    sessions: std::sync::Mutex<SessionOwners>,
}

/// How many session-to-sign-in bindings are kept. Past this the oldest is
/// forgotten, and its session falls back to per-request verification alone —
/// which still holds every request to its own token.
pub const SESSION_BINDINGS_CAP: usize = 10_000;

/// Which sign-in opened which MCP session (the MCP authorization spec's
/// "bind session IDs to user-specific information"). A session is never
/// used to AUTHENTICATE — every request carries and proves its own token —
/// but a session holds state (a seat, a declared acting agent), and another
/// sign-in that learned its id must not borrow it.
#[derive(Default)]
struct SessionOwners {
    owner: std::collections::HashMap<String, String>,
    order: std::collections::VecDeque<String>,
}

impl SessionOwners {
    fn bind(&mut self, session: String, who: String) {
        if self.owner.insert(session.clone(), who).is_none() {
            self.order.push_back(session);
            while self.order.len() > SESSION_BINDINGS_CAP {
                if let Some(old) = self.order.pop_front() {
                    self.owner.remove(&old);
                }
            }
        }
    }

    fn unbind(&mut self, session: &str) {
        if self.owner.remove(session).is_some() {
            self.order.retain(|s| s != session);
        }
    }
}

impl VerifiedCaller {
    /// Who this sign-in is, for binding a session: the issuer and subject,
    /// else the issuer and contributor; `None` when the token names neither.
    fn sign_in(&self) -> Option<String> {
        self.subject
            .as_ref()
            .or(self.contributor.as_ref())
            .map(|who| format!("{}\u{1f}{who}", self.issuer))
    }
}

impl Verifier {
    pub fn new(config: BearerConfig) -> Arc<Verifier> {
        install_crypto();
        Arc::new(Verifier {
            config,
            keys: RwLock::new(Arc::new(KeySet::default())),
            fetch: tokio::sync::Mutex::new(FetchState::default()),
            sessions: std::sync::Mutex::new(SessionOwners::default()),
        })
    }

    fn session_owner(&self, session: &str) -> Option<String> {
        self.sessions
            .lock()
            .ok()
            .and_then(|s| s.owner.get(session).cloned())
    }

    fn bind_session(&self, session: &str, who: String) {
        if let Ok(mut s) = self.sessions.lock() {
            s.bind(session.to_string(), who);
        }
    }

    fn unbind_session(&self, session: &str) {
        if let Ok(mut s) = self.sessions.lock() {
            s.unbind(session);
        }
    }

    pub fn config(&self) -> &BearerConfig {
        &self.config
    }

    /// Load the key set now — at startup, so an operator learns at once
    /// whether the issuer answers. A failure is reported, not fatal: the next
    /// request tries again, so an issuer that starts after reflow2 is fine.
    pub async fn warm(&self) -> Result<usize, String> {
        let mut st = self.fetch.lock().await;
        self.refresh(&mut st).await
    }

    fn current(&self) -> Arc<KeySet> {
        match self.keys.read() {
            Ok(k) => Arc::clone(&k),
            Err(poisoned) => Arc::clone(&poisoned.into_inner()),
        }
    }

    /// Fetch (or read) the key set, replace the cache on success.
    async fn refresh(&self, st: &mut FetchState) -> Result<usize, String> {
        st.last_attempt = Some(Instant::now());
        let result = self.load(st).await;
        match result {
            Ok(keys) => {
                let n = keys.len();
                let set = Arc::new(KeySet {
                    keys,
                    loaded_at: Some(Instant::now()),
                });
                match self.keys.write() {
                    Ok(mut k) => *k = set,
                    Err(poisoned) => *poisoned.into_inner() = set,
                }
                st.last_error = None;
                Ok(n)
            }
            Err(e) => {
                // Discover again next time: the issuer may have moved its keys.
                st.jwks_uri = None;
                if st.last_error.as_deref() != Some(e.as_str()) {
                    eprintln!("reflow2: could not load the token issuer's keys — {e}");
                }
                st.last_error = Some(e.clone());
                Err(e)
            }
        }
    }

    async fn load(&self, st: &mut FetchState) -> Result<Vec<CachedKey>, String> {
        let doc = match &self.config.keys {
            KeySource::File(p) => {
                let text = std::fs::read_to_string(p).map_err(|e| {
                    format!("the pinned key set {} could not be read: {e}", p.display())
                })?;
                serde_json::from_str::<Value>(&text)
                    .map_err(|e| format!("the pinned key set {} is not JSON: {e}", p.display()))?
            }
            KeySource::Uri(u) => crate::mcp_http::get_json(u)
                .await
                .map_err(|e| format!("the key set at {u}: {e:#}"))?,
            KeySource::Discover => {
                let uri = match &st.jwks_uri {
                    Some(u) => u.clone(),
                    None => {
                        let u = self.discover().await?;
                        st.jwks_uri = Some(u.clone());
                        u
                    }
                };
                crate::mcp_http::get_json(&uri)
                    .await
                    .map_err(|e| format!("the key set at {uri}: {e:#}"))?
            }
        };
        let where_from = match &self.config.keys {
            KeySource::File(p) => p.display().to_string(),
            KeySource::Uri(u) => u.clone(),
            KeySource::Discover => st.jwks_uri.clone().unwrap_or_default(),
        };
        parse_key_set(&doc).map_err(|e| format!("the key set from {where_from}: {e}"))
    }

    /// The issuer's `jwks_uri`, from OpenID discovery or RFC 8414 metadata. The
    /// document must name the configured issuer exactly (OIDC Discovery §4.3).
    async fn discover(&self) -> Result<String, String> {
        let issuer = self.config.issuer.trim_end_matches('/');
        let uri: Uri = self
            .config
            .issuer
            .parse()
            .map_err(|_| "unparseable issuer")?;
        let origin = format!(
            "{}://{}",
            uri.scheme_str().unwrap_or("https"),
            uri.authority().map(|a| a.as_str()).unwrap_or("")
        );
        let path = uri.path().trim_end_matches('/');
        let candidates = [
            format!("{issuer}/.well-known/openid-configuration"),
            format!("{origin}/.well-known/oauth-authorization-server{path}"),
        ];
        let mut why = Vec::new();
        for url in candidates {
            match crate::mcp_http::get_json(&url).await {
                Ok(doc) => {
                    if doc.get("issuer").and_then(Value::as_str)
                        != Some(self.config.issuer.as_str())
                    {
                        why.push(format!(
                            "{url} names issuer {}, not {}",
                            doc.get("issuer")
                                .map(|v| v.to_string())
                                .unwrap_or("nothing".into()),
                            self.config.issuer
                        ));
                        continue;
                    }
                    let Some(jwks) = doc.get("jwks_uri").and_then(Value::as_str) else {
                        why.push(format!("{url} names no jwks_uri"));
                        continue;
                    };
                    checked_url(
                        "the discovered jwks_uri",
                        jwks,
                        "A key set is fetched only over TLS.",
                    )?;
                    return Ok(jwks.to_string());
                }
                Err(e) => why.push(format!("{e:#}")),
            }
        }
        Err(format!(
            "discovery at {} found no key set: {}. Name it with --http-oidc-jwks-uri, or pin it \
             with --http-oidc-jwks-file.",
            self.config.issuer,
            why.join("; ")
        ))
    }

    /// The key to check a token with `alg` and `kid` against, refreshing the
    /// cache when it is stale, empty, or does not hold `kid`.
    async fn key_for(&self, alg: Algorithm, kid: Option<&str>) -> Result<Jwk, Refusal> {
        let mut set = self.current();
        let stale = set.loaded_at.is_none_or(|t| t.elapsed() >= KEYS_TTL);
        let missing = kid.is_some_and(|k| !set.keys.iter().any(|c| c.kid.as_deref() == Some(k)));
        if stale || missing {
            let mut st = self.fetch.lock().await;
            // Someone else may have refreshed while this request waited.
            set = self.current();
            let stale = set.loaded_at.is_none_or(|t| t.elapsed() >= KEYS_TTL);
            let missing =
                kid.is_some_and(|k| !set.keys.iter().any(|c| c.kid.as_deref() == Some(k)));
            let empty = set.loaded_at.is_none();
            let may_retry = st
                .last_attempt
                .is_none_or(|t| t.elapsed() >= RETRY_WHEN_EMPTY);
            let kid_refresh_allowed = st
                .last_unknown_kid_refresh
                .is_none_or(|t| t.elapsed() >= UNKNOWN_KID_COOLDOWN);
            // An empty or stale set is refreshed — but not by EVERY request
            // while the issuer is down: a stale set keeps serving between
            // attempts, and an empty one answers 503 between them.
            let go = if empty || stale {
                may_retry
            } else {
                missing && kid_refresh_allowed
            };
            if go {
                if missing && !empty && !stale {
                    st.last_unknown_kid_refresh = Some(Instant::now());
                }
                let _ = self.refresh(&mut st).await;
                set = self.current();
            }
            if set.loaded_at.is_none() {
                return Err(Refusal::KeysUnavailable(
                    st.last_error
                        .clone()
                        .unwrap_or_else(|| "the issuer's keys have not loaded yet".to_string()),
                ));
            }
        }
        let fitting: Vec<&CachedKey> = set.keys.iter().filter(|k| k.fits(alg)).collect();
        match kid {
            Some(k) => {
                let named: Vec<&CachedKey> = set
                    .keys
                    .iter()
                    .filter(|c| c.kid.as_deref() == Some(k))
                    .collect();
                if named.is_empty() {
                    return Err(Refusal::InvalidToken(format!(
                        "the token names key {k}, which the issuer does not publish"
                    )));
                }
                named
                    .into_iter()
                    .find(|c| c.fits(alg))
                    .map(|c| c.jwk.clone())
                    .ok_or_else(|| {
                        Refusal::InvalidToken(format!(
                            "the key {k} the token names does not fit its algorithm {alg:?}"
                        ))
                    })
            }
            None => match fitting.as_slice() {
                [only] => Ok(only.jwk.clone()),
                [] => Err(Refusal::InvalidToken(format!(
                    "the issuer publishes no key for algorithm {alg:?}"
                ))),
                many => Err(Refusal::InvalidToken(format!(
                    "the token names no key (no `kid`) and {} published keys fit {alg:?}",
                    many.len()
                ))),
            },
        }
    }

    /// Verify `token` and decide who it is. Everything a token claims is
    /// checked here or nowhere.
    pub async fn verify(&self, token: &str) -> Result<VerifiedCaller, Refusal> {
        if token.len() > MAX_TOKEN_BYTES {
            return Err(Refusal::InvalidToken(format!(
                "the token is longer than {MAX_TOKEN_BYTES} bytes"
            )));
        }
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() == 5 {
            return Err(Refusal::InvalidToken(
                "an encrypted token (JWE) is not accepted; send a signed JWT access token".into(),
            ));
        }
        if parts.len() != 3 {
            return Err(Refusal::InvalidToken(
                "the token is not a signed JWT (three dot-separated parts)".into(),
            ));
        }
        let header: Map<String, Value> = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[0])
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .ok_or_else(|| Refusal::InvalidToken("the token's header is not readable".into()))?;
        let alg_name = header.get("alg").and_then(Value::as_str).unwrap_or("");
        if alg_name.eq_ignore_ascii_case("none") || alg_name.is_empty() {
            return Err(Refusal::InvalidToken(
                "an unsigned token (alg none) is refused".into(),
            ));
        }
        if alg_name.to_ascii_uppercase().starts_with("HS") {
            return Err(Refusal::InvalidToken(format!(
                "alg {alg_name} is refused: only the issuer's published PUBLIC keys verify a \
                 token here, and a symmetric algorithm would make the key itself the secret"
            )));
        }
        let alg: Algorithm = alg_name
            .parse()
            .ok()
            .filter(|a| ACCEPTED_ALGS.contains(a))
            .ok_or_else(|| Refusal::InvalidToken(format!("alg {alg_name} is not accepted here")))?;
        if header.contains_key("crit") {
            return Err(Refusal::InvalidToken(
                "the token's header lists critical extensions (crit), which this server does not \
                 implement"
                    .into(),
            ));
        }
        let kid = header.get("kid").and_then(Value::as_str);
        let jwk = self.key_for(alg, kid).await?;
        let key = DecodingKey::from_jwk(&jwk)
            .map_err(|e| Refusal::InvalidToken(format!("the issuer's key is not usable: {e}")))?;

        let mut v = Validation::new(alg);
        v.set_issuer(&[self.config.issuer.as_str()]);
        v.set_audience(&[self.config.audience.as_str()]);
        v.set_required_spec_claims(&["exp", "iss", "aud"]);
        v.validate_exp = true;
        v.validate_nbf = true;
        v.validate_aud = true;
        v.leeway = LEEWAY_SECS;
        let claims = jsonwebtoken::decode::<Map<String, Value>>(token, &key, &v)
            .map_err(|e| Refusal::InvalidToken(describe_jwt_error(&e, &self.config)))?
            .claims;

        if let Some(missing) = self
            .config
            .required_scopes
            .iter()
            .find(|s| !granted_scopes(&claims).contains(s.as_str()))
        {
            return Err(Refusal::InsufficientScope(format!(
                "the token does not grant the scope `{missing}`"
            )));
        }
        for c in &self.config.required_claims {
            let holds = match claims.get(&c.name) {
                Some(Value::String(s)) => s == &c.value,
                Some(Value::Array(a)) => a.iter().any(|x| x.as_str() == Some(c.value.as_str())),
                Some(Value::Bool(b)) => b.to_string() == c.value,
                Some(Value::Number(n)) => n.to_string() == c.value,
                _ => false,
            };
            if !holds {
                return Err(Refusal::InsufficientScope(format!(
                    "the token's `{}` claim does not hold `{}`",
                    c.name, c.value
                )));
            }
        }
        let subject = claims.get("sub").and_then(Value::as_str).map(String::from);
        let (contributor, unmapped) = match self.config.mapping.contributor(&claims) {
            Ok(c) => (Some(c), None),
            Err(why) => (None, Some(why)),
        };
        Ok(VerifiedCaller {
            issuer: self.config.issuer.clone(),
            subject,
            contributor,
            unmapped,
        })
    }

    /// The `WWW-Authenticate` value for `refusal`.
    pub fn challenge(&self, refusal: &Refusal) -> String {
        let mut params = vec![("realm".to_string(), "reflow2".to_string())];
        let (code, desc) = match refusal {
            Refusal::NoToken | Refusal::KeysUnavailable(_) => (None, None),
            Refusal::InvalidRequest(d) => (Some("invalid_request"), Some(d)),
            Refusal::InvalidToken(d) => (Some("invalid_token"), Some(d)),
            Refusal::InsufficientScope(d) => (Some("insufficient_scope"), Some(d)),
        };
        if let Some(c) = code {
            params.push(("error".into(), c.into()));
        }
        if let Some(d) = desc {
            params.push(("error_description".into(), challenge_text(d)));
        }
        if !self.config.required_scopes.is_empty() {
            params.push(("scope".into(), self.config.required_scopes.join(" ")));
        }
        params.push(("resource_metadata".into(), self.config.metadata_url()));
        let joined: Vec<String> = params
            .into_iter()
            .map(|(k, v)| format!("{k}=\"{v}\""))
            .collect();
        format!("Bearer {}", joined.join(", "))
    }

    /// The whole refusal reply: status, challenge, and a plain sentence.
    pub fn refusal_response(&self, refusal: &Refusal) -> BoxResponse {
        let (status, body) = match refusal {
            Refusal::NoToken => (
                StatusCode::UNAUTHORIZED,
                format!(
                    "reflow2 refused this request: it carries no access token. This server is an \
                     OAuth 2.0 resource server: send a token issued by {} in the Authorization \
                     header (`Authorization: Bearer <token>`). How to get one is described at {}.\n",
                    self.config.issuer,
                    self.config.metadata_url()
                ),
            ),
            Refusal::InvalidRequest(d) => (
                StatusCode::BAD_REQUEST,
                format!(
                    "reflow2 refused this request (invalid_request): {d}. A token is accepted in \
                     the Authorization header only, as `Authorization: Bearer <token>`, and never \
                     in the URL.\n"
                ),
            ),
            Refusal::InvalidToken(d) => (
                StatusCode::UNAUTHORIZED,
                format!(
                    "reflow2 refused this request (invalid_token): {d}. Get a fresh token from {} \
                     for {}.\n",
                    self.config.issuer, self.config.audience
                ),
            ),
            Refusal::InsufficientScope(d) => (
                StatusCode::FORBIDDEN,
                format!("reflow2 refused this request (insufficient_scope): {d}.\n"),
            ),
            Refusal::KeysUnavailable(d) => (
                StatusCode::SERVICE_UNAVAILABLE,
                format!(
                    "reflow2 cannot check any token right now, because the issuer's keys could \
                     not be loaded: {d}. Your token was not judged; retry shortly.\n"
                ),
            ),
        };
        let mut b = Response::builder()
            .status(status)
            .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .header("x-content-type-options", "nosniff")
            .header(http::header::CACHE_CONTROL, "no-store");
        if matches!(refusal, Refusal::KeysUnavailable(_)) {
            b = b.header(http::header::RETRY_AFTER, "10");
        } else {
            b = b.header(http::header::WWW_AUTHENTICATE, self.challenge(refusal));
        }
        b.body(Full::new(Bytes::from(body)).boxed())
            .expect("a static text response is always well-formed")
    }

    fn metadata_response(&self, method: &http::Method) -> BoxResponse {
        if *method != http::Method::GET && *method != http::Method::HEAD {
            return Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(http::header::ALLOW, "GET, HEAD")
                .body(Full::new(Bytes::new()).boxed())
                .expect("a static response is always well-formed");
        }
        let body = if *method == http::Method::HEAD {
            Bytes::new()
        } else {
            Bytes::from(self.config.metadata().to_string())
        };
        Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_TYPE, "application/json")
            .header(http::header::CACHE_CONTROL, "max-age=300")
            .body(Full::new(body).boxed())
            .expect("a static response is always well-formed")
    }
}

/// The scopes a token grants: `scope` (a space-delimited string, RFC 9068),
/// or `scp` (a string or an array, as some issuers send it).
fn granted_scopes(claims: &Map<String, Value>) -> HashSet<&str> {
    let mut out = HashSet::new();
    for key in ["scope", "scp"] {
        match claims.get(key) {
            Some(Value::String(s)) => out.extend(s.split_whitespace()),
            Some(Value::Array(a)) => out.extend(a.iter().filter_map(Value::as_str)),
            _ => {}
        }
    }
    out
}

fn describe_jwt_error(e: &jsonwebtoken::errors::Error, c: &BearerConfig) -> String {
    use jsonwebtoken::errors::ErrorKind as K;
    match e.kind() {
        K::ExpiredSignature => "the token has expired".into(),
        K::ImmatureSignature => "the token is not valid yet (nbf is in the future)".into(),
        K::InvalidIssuer => format!("the token was not issued by {}", c.issuer),
        K::InvalidAudience => format!(
            "the token is not for this resource: its audience does not name {}",
            c.audience
        ),
        K::MissingRequiredClaim(claim) => format!("the token carries no `{claim}` claim"),
        K::InvalidSignature => "the token's signature does not verify".into(),
        K::InvalidAlgorithm => "the token's algorithm does not fit the key it names".into(),
        K::InvalidClaimFormat(claim) => format!("the token's `{claim}` claim is malformed"),
        _ => "the token could not be read".into(),
    }
}

/// A value safe inside a quoted challenge parameter: RFC 6750 §3 allows
/// %x20-21 / %x23-5B / %x5D-7E, so no quote, no backslash, nothing else.
fn challenge_text(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '"' | '\\' => '\'',
            c if (' '..='~').contains(&c) => c,
            _ => '?',
        })
        .take(300)
        .collect()
}

// ---- the gate --------------------------------------------------------------------

/// Does `query` carry an access token (RFC 6750 §2.3's `access_token`)?
fn query_carries_token(query: Option<&str>) -> bool {
    let Some(q) = query else { return false };
    q.split('&').any(|pair| {
        let key = pair.split('=').next().unwrap_or("");
        percent_decode(key).eq_ignore_ascii_case("access_token")
    })
}

fn percent_decode(s: &str) -> String {
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(hi), Some(lo)) = (hex(b[i + 1]), hex(b[i + 2]))
        {
            out.push(hi * 16 + lo);
            i += 3;
            continue;
        }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// RFC 6750's b64token: `1*( ALPHA / DIGIT / "-" / "." / "_" / "~" / "+" / "/" ) *"="`.
fn is_b64token(t: &str) -> bool {
    let body = t.trim_end_matches('=');
    !body.is_empty()
        && body
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-._~+/".contains(c))
}

/// The bearer token in the request's headers, or why there is none.
pub fn bearer_token(headers: &HeaderMap) -> Result<String, Refusal> {
    let mut values = headers.get_all(http::header::AUTHORIZATION).iter();
    let Some(first) = values.next() else {
        return Err(Refusal::NoToken);
    };
    if values.next().is_some() {
        return Err(Refusal::InvalidRequest(
            "the request carries more than one Authorization header".into(),
        ));
    }
    let text = first.to_str().map_err(|_| {
        Refusal::InvalidRequest("the Authorization header is not valid text".into())
    })?;
    let text = text.trim();
    let (scheme, rest) = match text.split_once(' ') {
        Some((s, r)) => (s, r),
        None => (text, ""),
    };
    if !scheme.eq_ignore_ascii_case("bearer") {
        // Another scheme carries no bearer credentials (RFC 6750 §3.1).
        return Err(Refusal::NoToken);
    }
    let token = rest.trim_start_matches(' ');
    if token.is_empty() {
        return Err(Refusal::InvalidRequest(
            "the Authorization header names Bearer and carries no token".into(),
        ));
    }
    if token.contains(char::is_whitespace) {
        return Err(Refusal::InvalidRequest(
            "the Authorization header carries more than one token".into(),
        ));
    }
    if !is_b64token(token) {
        return Err(Refusal::InvalidRequest(
            "the bearer token holds characters a token cannot (RFC 6750 b64token)".into(),
        ));
    }
    Ok(token.to_string())
}

/// The MCP streamable-HTTP session header.
const SESSION_HEADER: &str = "mcp-session-id";

/// A request quoting a session another sign-in opened: answered as the
/// transport answers a session it does not know (404), so the client starts
/// its own, and nothing behind the gate runs.
fn not_your_session() -> BoxResponse {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(http::header::CACHE_CONTROL, "no-store")
        .body(
            Full::new(Bytes::from_static(
                b"reflow2: no such session for this sign-in. A session belongs to the sign-in \
                  that opened it; start your own with initialize.\n",
            ))
            .boxed(),
        )
        .expect("a static text response is always well-formed")
}

/// `inner`, reached only by requests carrying a token the verifier accepts —
/// and reached WITHOUT the token: the `Authorization` header stops here.
/// With no verifier (no issuer declared) it passes every request through
/// untouched, so one service type serves both.
#[derive(Clone)]
pub struct BearerGate<S> {
    inner: S,
    verifier: Option<Arc<Verifier>>,
}

impl<S> BearerGate<S> {
    pub fn new(inner: S, verifier: Arc<Verifier>) -> BearerGate<S> {
        BearerGate {
            inner,
            verifier: Some(verifier),
        }
    }

    /// A gate when `verifier` is given, a pass-through when it is not.
    pub fn optional(inner: S, verifier: Option<Arc<Verifier>>) -> BearerGate<S> {
        BearerGate { inner, verifier }
    }
}

impl<S, B> tower_service::Service<Request<B>> for BearerGate<S>
where
    S: tower_service::Service<Request<B>, Response = BoxResponse, Error = Infallible>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = BoxResponse;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<BoxResponse, Infallible>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        let Some(verifier) = self.verifier.clone() else {
            return Box::pin(self.inner.call(req));
        };
        if verifier.config.is_metadata_path(req.uri().path()) {
            let r = verifier.metadata_response(req.method());
            return Box::pin(async move { Ok(r) });
        }
        // Checked BEFORE the header, and refused even beside a valid one: a
        // token in a URL has already been written into logs and histories,
        // and accepting the request would teach the client it works.
        if query_carries_token(req.uri().query()) {
            let r = verifier.refusal_response(&Refusal::InvalidRequest(
                "the request carries an access token in its URL query string".into(),
            ));
            return Box::pin(async move { Ok(r) });
        }
        let token = match bearer_token(req.headers()) {
            Ok(t) => t,
            Err(refusal) => {
                let r = verifier.refusal_response(&refusal);
                return Box::pin(async move { Ok(r) });
            }
        };
        // The service that was polled ready takes this request; a clone takes
        // its place for the next (the tower idiom for an async gate).
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let session = req
            .headers()
            .get(SESSION_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let ends_session = *req.method() == http::Method::DELETE;
        Box::pin(async move {
            let caller = match verifier.verify(&token).await {
                Ok(c) => c,
                Err(refusal) => return Ok(verifier.refusal_response(&refusal)),
            };
            let who = caller.sign_in();
            if let Some(sid) = &session
                && let Some(owner) = verifier.session_owner(sid)
                && who.as_ref() != Some(&owner)
            {
                return Ok(not_your_session());
            }
            req.headers_mut().remove(http::header::AUTHORIZATION);
            req.extensions_mut().insert(caller);
            let res = inner.call(req).await?;
            match (&session, &who) {
                (None, Some(who)) => {
                    if let Some(new_sid) = res
                        .headers()
                        .get(SESSION_HEADER)
                        .and_then(|v| v.to_str().ok())
                    {
                        verifier.bind_session(new_sid, who.clone());
                    }
                }
                (Some(sid), _) if ends_session && res.status().is_success() => {
                    verifier.unbind_session(sid);
                }
                _ => {}
            }
            Ok(res)
        })
    }
}

// ---- ring as jsonwebtoken's crypto provider -----------------------------------

struct RingVerifier {
    alg: Algorithm,
    key: DecodingKey,
}

impl jsonwebtoken::signature::Verifier<Vec<u8>> for RingVerifier {
    fn verify(
        &self,
        msg: &[u8],
        signature: &Vec<u8>,
    ) -> Result<(), jsonwebtoken::signature::Error> {
        use ring::signature as rs;
        let bad = || jsonwebtoken::signature::Error::new();
        let rsa: Option<&'static rs::RsaParameters> = match self.alg {
            Algorithm::RS256 => Some(&rs::RSA_PKCS1_2048_8192_SHA256),
            Algorithm::RS384 => Some(&rs::RSA_PKCS1_2048_8192_SHA384),
            Algorithm::RS512 => Some(&rs::RSA_PKCS1_2048_8192_SHA512),
            Algorithm::PS256 => Some(&rs::RSA_PSS_2048_8192_SHA256),
            Algorithm::PS384 => Some(&rs::RSA_PSS_2048_8192_SHA384),
            Algorithm::PS512 => Some(&rs::RSA_PSS_2048_8192_SHA512),
            _ => None,
        };
        if let Some(params) = rsa {
            return match self.key.kind() {
                DecodingKeyKind::RsaModulusExponent { n, e } => rs::RsaPublicKeyComponents { n, e }
                    .verify(params, msg, signature)
                    .map_err(|_| bad()),
                DecodingKeyKind::SecretOrDer(der) => rs::UnparsedPublicKey::new(params, der)
                    .verify(msg, signature)
                    .map_err(|_| bad()),
            };
        }
        let other: &'static dyn rs::VerificationAlgorithm = match self.alg {
            Algorithm::ES256 => &rs::ECDSA_P256_SHA256_FIXED,
            Algorithm::ES384 => &rs::ECDSA_P384_SHA384_FIXED,
            Algorithm::EdDSA => &rs::ED25519,
            _ => return Err(bad()),
        };
        match self.key.kind() {
            DecodingKeyKind::SecretOrDer(bytes) => rs::UnparsedPublicKey::new(other, bytes)
                .verify(msg, signature)
                .map_err(|_| bad()),
            DecodingKeyKind::RsaModulusExponent { .. } => Err(bad()),
        }
    }
}

impl jsonwebtoken::crypto::JwtVerifier for RingVerifier {
    fn algorithm(&self) -> Algorithm {
        self.alg
    }
}

fn ring_verifier(
    alg: &Algorithm,
    key: &DecodingKey,
) -> jsonwebtoken::errors::Result<Box<dyn jsonwebtoken::crypto::JwtVerifier>> {
    use jsonwebtoken::errors::ErrorKind;
    if !ACCEPTED_ALGS.contains(alg) || key.family() != alg.family() {
        return Err(ErrorKind::InvalidAlgorithm.into());
    }
    Ok(Box::new(RingVerifier {
        alg: *alg,
        key: key.clone(),
    }))
}

fn ring_signer(
    _: &Algorithm,
    _: &jsonwebtoken::EncodingKey,
) -> jsonwebtoken::errors::Result<Box<dyn jsonwebtoken::crypto::JwtSigner>> {
    Err(jsonwebtoken::errors::ErrorKind::Signing(
        "reflow2 verifies tokens and never issues them".into(),
    )
    .into())
}

fn no_private_keys(_: &[u8]) -> jsonwebtoken::errors::Result<(Vec<u8>, Vec<u8>)> {
    Err(jsonwebtoken::errors::ErrorKind::InvalidKeyFormat.into())
}

fn no_private_ec_keys(
    _: &[u8],
    _: Algorithm,
) -> jsonwebtoken::errors::Result<(EllipticCurve, Vec<u8>, Vec<u8>)> {
    Err(jsonwebtoken::errors::ErrorKind::InvalidKeyFormat.into())
}

fn no_private_ed_keys(_: &[u8], _: &EllipticCurve) -> jsonwebtoken::errors::Result<Vec<u8>> {
    Err(jsonwebtoken::errors::ErrorKind::InvalidKeyFormat.into())
}

fn ring_digest(data: &[u8], hash: ThumbprintHash) -> jsonwebtoken::errors::Result<Vec<u8>> {
    let alg = match hash {
        ThumbprintHash::SHA256 => &ring::digest::SHA256,
        ThumbprintHash::SHA384 => &ring::digest::SHA384,
        ThumbprintHash::SHA512 => &ring::digest::SHA512,
        _ => return Err(jsonwebtoken::errors::ErrorKind::UnsupportedAlgorithm.into()),
    };
    Ok(ring::digest::digest(alg, data).as_ref().to_vec())
}

/// `ring` behind `jsonwebtoken`: verification only.
static RING_PROVIDER: jsonwebtoken::crypto::CryptoProvider = jsonwebtoken::crypto::CryptoProvider {
    signer_factory: ring_signer,
    verifier_factory: ring_verifier,
    key_utils: jsonwebtoken::crypto::KeyUtils {
        rsa_pub_components_from_private_key: no_private_keys,
        rsa_pub_components_from_public_key: no_private_keys,
        ec_pub_components_from_private_key: no_private_ec_keys,
        ed_pub_components_from_private_key: no_private_ed_keys,
        compute_digest: ring_digest,
    },
};

/// Make `ring` the process's JWT crypto. Idempotent; called by
/// [`Verifier::new`], so no token is checked before it is in place.
pub fn install_crypto() {
    let _ = RING_PROVIDER.install_default();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_template_renders_from_claims_and_refuses_what_it_cannot_name() {
        let m = ContributorMapping::new("who:{preferred_username}", None).unwrap();
        let claims: Map<String, Value> =
            serde_json::from_value(json!({"preferred_username": "alice", "sub": "1"})).unwrap();
        assert_eq!(m.contributor(&claims).unwrap(), "who:alice");
        let none: Map<String, Value> = serde_json::from_value(json!({"sub": "1"})).unwrap();
        assert!(
            m.contributor(&none)
                .unwrap_err()
                .contains("preferred_username")
        );
        let spaced: Map<String, Value> =
            serde_json::from_value(json!({"preferred_username": "a b"})).unwrap();
        assert!(m.contributor(&spaced).is_err());
        assert!(
            ContributorMapping::new("who:alice", None).is_err(),
            "no claim"
        );
        assert!(ContributorMapping::new("who:{", None).is_err());
        assert!(ContributorMapping::new("who:{a b}", None).is_err());
    }

    #[test]
    fn a_map_names_only_the_keys_it_holds() {
        let m = ContributorMapping::new(
            "{sub}",
            Some((
                "m.toml".into(),
                "[contributors]\n\"s-1\" = \"who:alice\"\n".into(),
            )),
        )
        .unwrap();
        let c: Map<String, Value> = serde_json::from_value(json!({"sub": "s-1"})).unwrap();
        assert_eq!(m.contributor(&c).unwrap(), "who:alice");
        let c: Map<String, Value> = serde_json::from_value(json!({"sub": "s-2"})).unwrap();
        assert!(m.contributor(&c).unwrap_err().contains("does not hold"));
        assert!(ContributorMapping::new("{sub}", Some(("m".into(), "x = 1".into()))).is_err());
    }

    #[test]
    fn the_authorization_header_is_read_strictly() {
        let h = |v: &[&str]| {
            let mut m = HeaderMap::new();
            for x in v {
                m.append(http::header::AUTHORIZATION, x.parse().unwrap());
            }
            m
        };
        assert_eq!(bearer_token(&h(&[])), Err(Refusal::NoToken));
        assert_eq!(bearer_token(&h(&["Basic abc"])), Err(Refusal::NoToken));
        assert_eq!(bearer_token(&h(&["bearer a.b.c"])), Ok("a.b.c".into()));
        assert_eq!(
            bearer_token(&h(&["Bearer  a.b-c_d~e+f/g=="])),
            Ok("a.b-c_d~e+f/g==".into())
        );
        for bad in ["Bearer", "Bearer a b", "Bearer a!b"] {
            assert!(
                matches!(bearer_token(&h(&[bad])), Err(Refusal::InvalidRequest(_))),
                "{bad}"
            );
        }
        assert!(matches!(
            bearer_token(&h(&["Bearer a", "Bearer a"])),
            Err(Refusal::InvalidRequest(_))
        ));
    }

    #[test]
    fn a_query_token_is_seen_however_it_is_spelled() {
        assert!(query_carries_token(Some("access_token=x")));
        assert!(query_carries_token(Some("a=1&access%5Ftoken=x")));
        assert!(query_carries_token(Some("ACCESS_TOKEN=x")));
        assert!(!query_carries_token(Some("token=x")));
        assert!(!query_carries_token(None));
    }

    #[test]
    fn a_challenge_value_cannot_break_its_quotes() {
        let t = challenge_text("say \"hi\" \\ and \u{7}");
        assert!(
            !t.contains('"') && !t.contains('\\') && !t.contains('\u{7}'),
            "{t}"
        );
    }

    fn config(resource: &str) -> BearerConfig {
        BearerConfig::new(
            "https://sso.example.org/realms/team",
            None,
            resource,
            None,
            Some("/nonexistent/jwks.json"),
            &["reflow2".to_string()],
            &[],
            ContributorMapping::new("who:{sub}", None).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn the_metadata_url_puts_the_well_known_suffix_before_the_path() {
        let c = config("https://reflow2.example.org/");
        assert_eq!(
            c.metadata_url(),
            "https://reflow2.example.org/.well-known/oauth-protected-resource"
        );
        assert!(c.is_metadata_path("/.well-known/oauth-protected-resource"));
        let c = config("https://example.org/reflow2/mcp");
        assert_eq!(
            c.metadata_url(),
            "https://example.org/.well-known/oauth-protected-resource/reflow2/mcp"
        );
        assert!(c.is_metadata_path("/.well-known/oauth-protected-resource/reflow2/mcp"));
        assert!(c.is_metadata_path("/.well-known/oauth-protected-resource"));
        assert_eq!(
            c.audience, "https://example.org/reflow2/mcp",
            "RFC 8707 default"
        );
    }

    #[test]
    fn a_cleartext_public_url_off_this_machine_is_refused() {
        let m = ContributorMapping::new("who:{sub}", None).unwrap();
        let e = BearerConfig::new(
            "https://sso.example.org/realms/team",
            None,
            "http://reflow2.example.org/",
            None,
            None,
            &[],
            &[],
            m.clone(),
        )
        .unwrap_err();
        assert!(e.contains("TLS"), "{e}");
        assert!(
            BearerConfig::new(
                "https://sso.example.org/realms/team",
                None,
                "http://127.0.0.1:8080/",
                None,
                None,
                &[],
                &[],
                m,
            )
            .is_ok()
        );
    }

    /// The token stops at the gate: the inner service never sees the
    /// Authorization header, and does see who the gate established.
    #[test]
    fn the_token_is_never_passed_through() {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
        use ring::signature::KeyPair;

        let rng = ring::rand::SystemRandom::new();
        let alg = &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING;
        let pkcs8 = ring::signature::EcdsaKeyPair::generate_pkcs8(alg, &rng).unwrap();
        let kp = ring::signature::EcdsaKeyPair::from_pkcs8(alg, pkcs8.as_ref(), &rng).unwrap();
        let p = kp.public_key().as_ref();
        let dir = std::env::temp_dir().join(format!("reflow2-bearer-unit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let jwks = dir.join("jwks.json");
        std::fs::write(
            &jwks,
            json!({"keys": [{"kty": "EC", "crv": "P-256", "kid": "k", "x": B64.encode(&p[1..33]), "y": B64.encode(&p[33..65])}]})
                .to_string(),
        )
        .unwrap();
        let cfg = BearerConfig::new(
            "https://sso.example.org/realms/team",
            None,
            "https://reflow2.example.org/",
            None,
            Some(jwks.to_str().unwrap()),
            &[],
            &[],
            ContributorMapping::new("who:{preferred_username}", None).unwrap(),
        )
        .unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let msg = format!(
            "{}.{}",
            B64.encode(json!({"alg": "ES256", "kid": "k"}).to_string()),
            B64.encode(
                json!({"iss": "https://sso.example.org/realms/team", "aud": "https://reflow2.example.org/",
                       "exp": now + 60, "sub": "s", "preferred_username": "alice"})
                .to_string()
            )
        );
        let sig = kp.sign(&rng, msg.as_bytes()).unwrap();
        let token = format!("{msg}.{}", B64.encode(sig.as_ref()));

        #[derive(Clone, Default)]
        struct Probe(Arc<std::sync::Mutex<Option<(bool, Option<VerifiedCaller>)>>>);
        impl tower_service::Service<Request<()>> for Probe {
            type Response = BoxResponse;
            type Error = Infallible;
            type Future =
                Pin<Box<dyn Future<Output = Result<BoxResponse, Infallible>> + Send + 'static>>;
            fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
                Poll::Ready(Ok(()))
            }
            fn call(&mut self, req: Request<()>) -> Self::Future {
                *self.0.lock().unwrap() = Some((
                    req.headers().contains_key(http::header::AUTHORIZATION),
                    req.extensions().get::<VerifiedCaller>().cloned(),
                ));
                Box::pin(async { Ok(Response::new(Full::new(Bytes::new()).boxed())) })
            }
        }
        let probe = Probe::default();
        let mut gate = BearerGate::new(probe.clone(), Verifier::new(cfg));
        let req = Request::builder()
            .uri("/mcp")
            .header("authorization", format!("Bearer {token}"))
            .body(())
            .unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let res = rt
            .block_on(tower_service::Service::call(&mut gate, req))
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let (saw_header, caller) = probe.0.lock().unwrap().clone().expect("reached");
        assert!(
            !saw_header,
            "the Authorization header must not reach the service"
        );
        assert_eq!(
            caller.and_then(|c| c.contributor).as_deref(),
            Some("who:alice")
        );
    }
}
