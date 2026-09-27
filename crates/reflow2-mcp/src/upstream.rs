//! Read the designs this one DEPENDS ON, without taking them in.
//!
//! # The finding this exists to fix
//!
//! `req:design-dependencies-declared` (ACCEPTED 2026-07-27) names two checks in
//! its own statement: *"declared-versus-mirrored answers am I actually composing
//! against the version I said, and DECLARED-VERSUS-UPSTREAM answers has what I
//! depend on moved since. Either question is unanswerable with only one of the
//! two halves."* Only the build-facing half shipped —
//! `reconcile_dependencies` compares a declaration against `Cargo.toml` and its
//! siblings. Nothing compared a declaration against the upstream DESIGN.
//!
//! # Why the obvious route was barred
//!
//! [`crate::sync_debt`] already answers "has that file moved", over
//! `provenance::last_synced`. But that record is written by exactly two paths,
//! `export_graph` and `import_graph` — so the only ways to get a path into it
//! are to export to it or import from it. And `import_graph` into a store that
//! already holds a design KEEPS THE HOST'S NAME and upserts the incoming nodes
//! into it (`adopt_on_import`: *"a store already holding a design keeps its own
//! name, because layering an export onto a live design is an upsert, not a
//! restore"*). **So watching a design by importing it absorbs that design** —
//! which is precisely what the hub case
//! (`dec:idea-a-hub-owns-designs-it-does-not-absorb-and-that-is-a-third-relation`)
//! says must not happen.
//!
//! The missing piece was therefore much smaller than a new relation: a way to
//! WATCH a path without importing it. The comparison, the reporting and the
//! child list all already existed.
//!
//! # Why the child list is the manifest
//!
//! Something has to say WHICH designs to watch, and the obvious worry was that
//! this would drag a hub relation in through the back door. It does not: the
//! dependency manifest already holds the list — checked in, version-pinned,
//! naming each design by `graph_id`, reviewable in a diff, and carrying the
//! direction a flat list of ids could not express.
//!
//! # Why the reading lives here and not in the core
//!
//! `reflow2-core` does no file I/O, deliberately and repeatedly — the same
//! reason [`crate::sync_debt`] states for itself. The core holds the
//! declarations and the COMPARISON; this module supplies what was found on
//! disk. That split is `reconcile_dependencies` and `reconcile_artifacts` all
//! over again, one boundary along.
//!
//! ⚠️ IT DOES NOT NAVIGATE. `describe_designs` makes the CALLER find candidate
//! paths because reflow2 does no file navigation, and that rule is intact here:
//! every path read is one this design pointed at ITSELF, in its own committed
//! manifest. Reading a file you were handed is a weaker claim than going
//! looking for one, and the difference is the whole reason this is allowed.
//!
//! # A design watched at the server that holds it
//!
//! Since 2026-09-27 a dependency may name `design_address` instead of an
//! export path (`req:a-design-watches-another-design-at-the-server-that-holds-it`).
//! Under the one-blueprint rules a design IS the store on the server that holds
//! it, and an export is a perishable photocopy nothing tracks: the first design
//! that moved to flo2.io left its last photocopy behind, and watching that file
//! reported the move itself and would then have said "unchanged" forever
//! (`fact:a-moved-design-cannot-be-watched-and-its-frozen-export-reads-as-live-2026-09-27`).
//!
//! So [`observe_everywhere`] asks the server: the MCP handshake, then
//! `export_graph` with no path, and the COMPUTED content hash of what came back
//! — the same fingerprint a file watch takes, so a baseline means the same
//! thing whichever way it was taken. The credential is the one
//! `reflow2-mcp setup` stored for that server, and nothing else.
//!
//! ⚠️ ONLY THIS MODULE'S ASYNC PASS GOES OVER THE NETWORK. [`observe_upstreams`]
//! — the pass `loop_status` runs on every orientation — reads files only, and
//! an address target comes back `not_observed` saying where it IS read. An
//! orientation call that could hang on somebody else's server would be the
//! hang class this project refuses in the surfaces built to detect hangs.
//!
//! 🛑 A SERVER HOLDING OTHER PEOPLE'S DESIGNS NEVER REACHES OUT
//! ([`crate::service::ReflowService::reaches_out`]). A registry that fetched
//! whatever address a caller declared would let any caller make it reach any
//! host its network can see; there, an address watch is recorded and reported
//! `refused`, and the person's own client does the watching.

use reflow2_core::{GraphExport, ObservedUpstream, UpstreamTarget};
use serde_json::{Value, json};

/// How many upstream designs one pass will actually open, on disk or at an
/// address together.
///
/// The same bound, for the same measured reason, as
/// [`crate::sync_debt::MAX_RECORDS_CHECKED`]: every target costs a full document
/// read and parse, and one seat had accumulated 16 sync targets totalling
/// 102 MB before anybody noticed. A declared-dependency list grows more slowly
/// than a sync-target list — nobody adds one by accident — so the bound is
/// higher, but it is not absent, and what it skips is NAMED rather than
/// dropped.
pub const MAX_UPSTREAMS_READ: usize = 16;

/// How long the handshake with a watched design's server may take.
const WATCH_HELLO_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// How long the server may take to hand over the design. Generous, because a
/// large design is a large document; bounded, because a wedged server must
/// come back as `unreachable` rather than hang the caller.
const WATCH_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// What a bounded pass left unopened.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpstreamNotRead {
    pub count: usize,
    pub dependencies: Vec<String>,
    pub note: String,
}

fn not_read_note(skipped: &[&UpstreamTarget]) -> Option<UpstreamNotRead> {
    if skipped.is_empty() {
        return None;
    }
    Some(UpstreamNotRead {
        count: skipped.len(),
        dependencies: skipped.iter().map(|t| t.name.clone()).collect(),
        note: format!(
            "{} declared upstream design(s) were NOT opened by this pass, which reads at most \
             {MAX_UPSTREAMS_READ} because every one costs a full document read. Named here rather \
             than dropped: a pass that quietly checks fewer than it knows about is the silent \
             truncation this project refuses. They come back as `not_observed`, never as \
             agreement.",
            skipped.len()
        ),
    })
}

/// Read each upstream watched ON DISK and say what is there.
///
/// A target watched at an ADDRESS is not read here: that goes over the network,
/// which this synchronous pass — the one `loop_status` runs — never does. It
/// gets no observation, so the judgement reports it `not_observed` and says
/// `upstream_status` is where it is read.
///
/// Returns one [`ObservedUpstream`] per target opened, plus whatever the bound
/// left out. Pass the result to `DesignGraph::reconcile_upstream`, which holds
/// the judgement.
///
/// 🛑 IT NEVER WRITES. Not the baseline hash, not a sync record, nothing. A
/// check that refreshed its own baseline on read would report `moved` exactly
/// once and then be permanently quiet, which is worse than no check at all.
/// Taking a new baseline is `external_dependency`'s job, and it is a deliberate
/// act by the person who read what changed (`dec:ask-not-repair`).
pub fn observe_upstreams(
    targets: &[UpstreamTarget],
) -> (Vec<ObservedUpstream>, Option<UpstreamNotRead>) {
    let on_disk: Vec<&UpstreamTarget> = targets
        .iter()
        .filter(|t| t.design_export.is_some())
        .collect();
    let (read, skipped) = on_disk.split_at(on_disk.len().min(MAX_UPSTREAMS_READ));
    let observed = read.iter().map(|t| observe_one(t)).collect();
    (observed, not_read_note(skipped))
}

/// Read every declared upstream — files from disk, designs at an address from
/// the server that holds them — and say what is there.
///
/// This is the pass `upstream_status` runs. `reaches_out` is the server's own
/// [`crate::service::ReflowService::reaches_out`]: when it is false, an address
/// target is not fetched and comes back `refused`, naming why.
pub async fn observe_everywhere(
    targets: &[UpstreamTarget],
    reaches_out: bool,
) -> (Vec<ObservedUpstream>, Option<UpstreamNotRead>) {
    let all: Vec<&UpstreamTarget> = targets.iter().collect();
    let (read, skipped) = all.split_at(all.len().min(MAX_UPSTREAMS_READ));
    let mut observed = Vec::with_capacity(read.len());
    for t in read {
        observed.push(match t.design_address.as_deref() {
            None => observe_one(t),
            Some(_) if !reaches_out => ObservedUpstream {
                detail: Some(
                    "This reflow2 serves other people's designs and does not reach out to another \
                     server on a caller's behalf. Watch it from your own machine, where reflow2 \
                     carries your key for that server."
                        .into(),
                ),
                ..bare(t, "refused")
            },
            Some(address) => match fingerprint(address).await {
                Ok(f) => ObservedUpstream {
                    id: t.id.clone(),
                    state: "read".into(),
                    content_hash: Some(f.content_hash),
                    graph_id: Some(f.graph_id),
                    nodes: Some(f.nodes),
                    detail: None,
                },
                Err(n) => ObservedUpstream {
                    detail: Some(n.detail),
                    ..bare(t, n.state)
                },
            },
        });
    }
    (observed, not_read_note(skipped))
}

fn bare(t: &UpstreamTarget, state: &str) -> ObservedUpstream {
    ObservedUpstream {
        id: t.id.clone(),
        state: state.to_string(),
        content_hash: None,
        graph_id: None,
        nodes: None,
        detail: None,
    }
}

fn observe_one(t: &UpstreamTarget) -> ObservedUpstream {
    let Some(path) = t.design_export.as_deref() else {
        // Only reachable if a caller hands an address target to the file
        // reader. Reported, never read as a successful read.
        return ObservedUpstream {
            detail: Some("this target names no export path to read".into()),
            ..bare(t, "unreadable")
        };
    };
    let path = std::path::Path::new(path);
    if !path.exists() {
        return bare(t, "missing");
    }
    let Some(doc) = std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<GraphExport>(&raw).ok())
        .and_then(identified)
    else {
        return bare(t, "unreadable");
    };
    // ⚠️ COMPUTED, NEVER the hash the file states about itself. `sync_debt`
    // learned this the hard way: `effective_content_hash` TRUSTS the embedded
    // stamp and computes only when it is absent, so a document edited by
    // anything other than `export_graph` — a merge, a hand-fix, another tool —
    // keeps its old stamp and reads as unmoved while its content has moved.
    // The document is already parsed, so computing costs nothing extra.
    ObservedUpstream {
        id: t.id.clone(),
        state: "read".to_string(),
        content_hash: Some(doc.compute_content_hash()),
        graph_id: Some(doc.graph_id.clone()),
        nodes: Some(doc.nodes.len()),
        detail: None,
    }
}

/// Read the current content hash of one export, for taking a BASELINE.
///
/// Used by `external_dependency` when a declaration names an export to watch:
/// the hash recorded is what the declarer saw AT THAT MOMENT, which is the only
/// thing a later "has it moved?" can honestly be measured against. Returns
/// `None` when there is nothing readable there — a declaration whose pointer is
/// wrong must still be recordable, and it comes back as `missing` on the next
/// read rather than being refused now.
pub fn baseline_hash(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<GraphExport>(&raw).ok())
        .and_then(identified)
        .map(|doc| doc.compute_content_hash())
}

/// A document is a WATCHABLE design record only if it says which design it is.
///
/// `GraphExport` defaults every field on purpose, so a hand-authored document
/// can be imported (BL-138) — which means ANY JSON object parses as an empty,
/// unidentified "export". A watch compares one design over time, and a record
/// naming no design would be fingerprinted as whatever arrived: `{}` in a
/// watched file read as a design with no nodes, and so did a server answering
/// with something else. Found 2026-09-27 by the address watch's own test, and
/// the file watch had carried it since it was built.
fn identified(doc: GraphExport) -> Option<GraphExport> {
    (!doc.is_unidentified()).then_some(doc)
}

/// What a watched design's server gave: the design's computed fingerprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
    /// Computed from the design the server handed over, never a hash it stated.
    pub content_hash: String,
    pub graph_id: String,
    pub nodes: usize,
}

/// Why a watched design's server gave no fingerprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotFingerprinted {
    /// `unreachable` | `refused` | `unreadable` — the states
    /// `DesignGraph::reconcile_upstream` reports for an address watch.
    pub state: &'static str,
    /// What happened, in the failing party's words. Never carries a credential.
    pub detail: String,
}

impl NotFingerprinted {
    fn unreachable(detail: String) -> Self {
        Self {
            state: "unreachable",
            detail,
        }
    }
    fn refused(detail: String) -> Self {
        Self {
            state: "refused",
            detail,
        }
    }
    fn unreadable(detail: String) -> Self {
        Self {
            state: "unreadable",
            detail,
        }
    }
}

/// Ask the server at `address` for the design it holds, carrying the key
/// `reflow2-mcp setup` stored for that server (and no other), and return the
/// design's fingerprint.
pub async fn fingerprint(address: &str) -> Result<Fingerprint, NotFingerprinted> {
    let bearer = credential_for(address)?;
    let result = fingerprint_at(address, bearer.as_deref()).await;
    match (result, bearer.is_none()) {
        // Refused and we sent nothing: say how a key gets here, because "the
        // key is missing, wrong, expired or revoked" cannot tell a person that
        // this machine never had one.
        (Err(mut n), true) if n.state == "refused" => {
            if let Ok(origin) = crate::client_setup::origin_of(address) {
                n.detail.push_str(&format!(
                    " No key for {origin} is set up on this machine: `reflow2-mcp setup remote \
                     {origin}` stores one in the OS keychain (then `reflow2-mcp setup local` if new \
                     designs should still be created here)."
                ));
            }
            Err(n)
        }
        (other, _) => other,
    }
}

/// The key set up for `address`'s server, if any. A failure to READ a key that
/// was set up (the keychain entry gone, a named variable unset) is reported as
/// `refused`, never swallowed into "no key": sending nothing would draw a 401
/// whose message blames the key rather than the setup.
fn credential_for(address: &str) -> Result<Option<String>, NotFingerprinted> {
    let dir = crate::client_setup::config_dir().map_err(|e| {
        NotFingerprinted::refused(format!(
            "could not find reflow2's client settings to look up a key for {address}: {e:#}"
        ))
    })?;
    let cfg =
        crate::client_setup::load(&dir).map_err(|e| NotFingerprinted::refused(format!("{e:#}")))?;
    crate::client_setup::stored_credential(address, &cfg, &crate::client_setup::OsKeychain)
        .map_err(|e| NotFingerprinted::refused(format!("{e:#}")))
}

/// Ask the server at `address` for the design it holds and return its
/// fingerprint, carrying `bearer` if given. Public so a test can drive it
/// against a stand-in with a key of its own; everything else goes through
/// [`fingerprint`].
///
/// The read is the MCP handshake and then `export_graph` with no path, which
/// returns the whole design. That costs a full document on every read — the
/// price of needing nothing new on the far side, flo2.io's gateway included.
/// A cheap fingerprint read on the server would replace it.
pub async fn fingerprint_at(
    address: &str,
    bearer: Option<&str>,
) -> Result<Fingerprint, NotFingerprinted> {
    use crate::mcp_http::post_with;

    // A key never crosses a network in the clear: refused before anything is
    // sent, the same rule `--remote` applies.
    crate::proxy::check_remote_url(address, bearer.is_some())
        .map_err(|e| NotFingerprinted::refused(format!("{e:#}")))?;

    let hello = json!({
        "jsonrpc": "2.0", "id": 0, "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "reflow2-watch", "version": env!("CARGO_PKG_VERSION")}
        }
    })
    .to_string();
    let (_, session) = post_with(address, None, hello, WATCH_HELLO_TIMEOUT, bearer)
        .await
        .map_err(failed)?;
    // The transport's courtesy. A server that does not want it may answer it
    // with anything; what matters is the reply to the read below.
    let _ = post_with(
        address,
        session.as_deref(),
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.to_string(),
        WATCH_HELLO_TIMEOUT,
        bearer,
    )
    .await;
    let call = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "export_graph", "arguments": {}}
    })
    .to_string();
    let (messages, _) = post_with(
        address,
        session.as_deref(),
        call,
        WATCH_READ_TIMEOUT,
        bearer,
    )
    .await
    .map_err(failed)?;

    let reply = messages
        .iter()
        .filter_map(|m| serde_json::from_str::<Value>(m).ok())
        .find(|v| v.get("id") == Some(&json!(1)))
        .ok_or_else(|| {
            NotFingerprinted::unreadable(format!(
                "the server at {address} answered, but with no reply to the request for the design"
            ))
        })?;
    if let Some(err) = reply.get("error") {
        let why = err
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| err.to_string());
        return Err(NotFingerprinted::refused(format!(
            "the server at {address} refused to hand over the design: {why}"
        )));
    }
    let result = reply.get("result").ok_or_else(|| {
        NotFingerprinted::unreadable(format!(
            "the server at {address} answered the request for the design with neither a result nor an error"
        ))
    })?;
    let text: String = result
        .get("content")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|c| c.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    if result.get("isError") == Some(&Value::Bool(true)) {
        return Err(NotFingerprinted::refused(format!(
            "the server at {address} would not hand over the design: {text}"
        )));
    }
    // Structured content is where reflow2 puts it; a client whose content
    // policy asks for text gets the same document as text. Either is accepted,
    // and neither is trusted until it parses as a reflow2 export.
    let doc = match result.get("structuredContent") {
        Some(v) if v.is_object() => v.clone(),
        _ => serde_json::from_str::<Value>(&text).map_err(|_| {
            NotFingerprinted::unreadable(format!(
                "the server at {address} answered, but not with a reflow2 design"
            ))
        })?,
    };
    let export: GraphExport = serde_json::from_value(doc).map_err(|e| {
        NotFingerprinted::unreadable(format!(
            "the server at {address} answered, but not with a reflow2 design export ({e})"
        ))
    })?;
    let export = identified(export).ok_or_else(|| {
        NotFingerprinted::unreadable(format!(
            "the server at {address} answered with a document that names no design, so it is not a \
             reflow2 design export"
        ))
    })?;
    Ok(Fingerprint {
        content_hash: export.compute_content_hash(),
        graph_id: export.graph_id.clone(),
        nodes: export.nodes.len(),
    })
}

/// One transport failure, sorted into the state a reader acts on. A server
/// that ANSWERED with 401, 403 or 404 refused; one that answered with any
/// other error, timed out, or never answered is unreachable.
fn failed(e: anyhow::Error) -> NotFingerprinted {
    if let Some(answer) = e.downcast_ref::<crate::mcp_http::ServerAnswered>() {
        return match answer.status {
            401 | 403 | 404 => NotFingerprinted::refused(answer.to_string()),
            _ => NotFingerprinted::unreachable(answer.to_string()),
        };
    }
    NotFingerprinted::unreachable(format!("{e:#}"))
}
