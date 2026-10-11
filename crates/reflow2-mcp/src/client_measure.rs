//! The remote client MEASURES the folder's own files, because the server
//! cannot see this disk.
//!
//! `epoch:planned-local-files-are-measured-for-a-remote-design`, the accepted
//! intent `req:a-local-client-works-against-a-remote-reflow2-server-by-url`
//! point 3 (combined plan position 7).
//!
//! # The class this closes
//!
//! Since 2026-09-18 a server that holds the project tree measures a registered
//! file itself (`crate::measure`): `link_artifact`, `set_artifact_checksum(s)`
//! and `reconcile_artifacts` called without a checksum hash the file under the
//! root and record `checksum_basis: measured`. A design served from a registry
//! or from flo2.io holds no tree, so the same call there was either refused by
//! name or — when the agent typed a hash — recorded as whatever was asserted.
//! The client forwarded every line unchanged, and the one process that CAN see
//! the folder did nothing with it.
//!
//! # What this does
//!
//! The client parses each `tools/call` the way `proxy::is_initialize` parses
//! the handshake — by method and tool name, never a substring — and for four
//! tools fills in what the server would have measured had it held the tree:
//!
//! 1. `link_artifact` with no `checksum`: the `location` is measured and the
//!    checksum injected. A location that cannot be measured is forwarded as
//!    it came — the server registers the artifact with no baseline, exactly as
//!    it does for its own unmeasurable locations — and the reply names why.
//! 2. `set_artifact_checksum`, and every checksum-less item of
//!    `set_artifact_checksums`: the artifact's stored `location` is read with
//!    an internal `get_node` on the same session, measured, and the checksum
//!    injected. An unmeasurable one is REFUSED HERE with the server's own
//!    wording (`crate::measure::NotMeasured::reason`) and nothing is sent: an
//!    accept must never move a baseline to a guess, and the server's own
//!    `checksum_or_measure` refuses the same way.
//! 3. `reconcile_artifacts` with no `observed`: every Artifact is paged with an
//!    internal `scan_nodes`, measured, and sent as `observed` the way the
//!    server's own `measure_registered` builds it — absent → `present: false`,
//!    any other failure listed in the reply and not sent.
//!
//! Items that already carry a checksum pass through untouched. Internal calls
//! use this client's own ids and their replies never reach the agent's stream.
//!
//! # Measured with the server's own code
//!
//! [`crate::measure::measure`] is called on this machine with the client's
//! root, so hashing and path rules match the server by construction: sha256
//! over raw bytes as `sha256:<hex>`, a URI never looked for on disk, an escape
//! from the root refused unread, a directory never hashed.
//!
//! # The root, and never a guess
//!
//! A client attached through a folder's `.reflow2.toml` measures under the
//! folder that holds the file; a bare `--remote` client measures under
//! `--tree-root` when it was given one. With neither there is NO root: nothing
//! is measured, the calls go as they came, and the reply says so. The current
//! directory is never used — it is whatever the MCP host happened to start the
//! process in, and a checksum taken under the wrong folder reads exactly like
//! a right one.
//!
//! # The truth in the reply
//!
//! The server stores a value it did not measure as `asserted`, and that is
//! what it says. The client rewrites the reply's `measurement` block to say
//! the value was measured ON THIS MACHINE by the client, with what was
//! measured and under which root, and keeps the server's own block beside it
//! under `server`. Nothing on the wire changes for anyone not using this
//! client; the server is untouched.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Map, Value, json};

use crate::measure::{MeasureMemo, Measurement, NotMeasured, measure};
use crate::read_only_client::Forwarder;

/// Where a client's root came from — said on stderr and in the reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootSource {
    /// The folder holding `.reflow2.toml`.
    PointerFolder,
    /// `--tree-root` on the command line.
    TreeRoot,
}

/// The client's measuring half: a root (or none) and the memo the server
/// keeps per process, so a reconcile that measures every file pays once.
#[derive(Debug)]
pub struct ClientMeasure {
    root: Option<(PathBuf, RootSource)>,
    memo: MeasureMemo,
    next_id: AtomicU64,
}

/// What to do with one line from the agent, once this has looked at it.
#[derive(Debug)]
pub enum Prepared {
    /// Send `body`; when `after` is set, pass the server's reply through
    /// [`ClientMeasure::rewrite`] before the agent sees it.
    Forward {
        body: String,
        after: Option<Rewrite>,
    },
    /// Answered here; nothing is sent.
    Answer(Vec<String>),
}

/// How the server's reply to a prepared call is rewritten.
#[derive(Debug)]
pub enum Rewrite {
    /// `link_artifact`: what the client measured, or why it could not.
    Link {
        id: Value,
        outcome: Result<Measurement, NotMeasured>,
    },
    /// `set_artifact_checksum`: the one measurement injected.
    Checksum { id: Value, measured: Measurement },
    /// `set_artifact_checksums`: the measurements injected, by artifact id.
    Checksums {
        id: Value,
        measured: Vec<(String, Measurement)>,
    },
    /// `reconcile_artifacts`: the block `measure_registered` would have made.
    Reconcile { id: Value, block: Value },
    /// Forwarded as it came because this client has no root: the reply is told.
    NoRoot { id: Value, tool: String },
}

/// The tools whose checksum-less calls this fills in.
const LINK: &str = "link_artifact";
const SET_ONE: &str = "set_artifact_checksum";
const SET_MANY: &str = "set_artifact_checksums";
const RECONCILE: &str = "reconcile_artifacts";

impl ClientMeasure {
    /// A client measuring under `root`, or none.
    pub fn new(root: Option<(PathBuf, RootSource)>) -> Self {
        let root = root.map(|(p, s)| (std::fs::canonicalize(&p).unwrap_or(p), s));
        Self {
            root,
            memo: MeasureMemo::default(),
            next_id: AtomicU64::new(1),
        }
    }

    /// The root files are measured under, when there is one.
    pub fn root(&self) -> Option<&Path> {
        self.root.as_ref().map(|(p, _)| p.as_path())
    }

    /// One line for stderr saying what this client will and will not measure.
    pub fn banner(&self) -> String {
        match &self.root {
            Some((root, source)) => format!(
                "files named by {LINK}, {SET_ONE}, {SET_MANY} and {RECONCILE} are measured on \
                 this machine under {} ({}); the server cannot see this disk and records each \
                 value as asserted",
                root.display(),
                match source {
                    RootSource::PointerFolder => "the folder holding .reflow2.toml",
                    RootSource::TreeRoot => "--tree-root",
                }
            ),
            None => format!(
                "no root to measure files under: this session was started with no \
                 .reflow2.toml folder and no --tree-root, so {LINK}, {SET_ONE}, {SET_MANY} and \
                 {RECONCILE} called without a checksum are sent as they came and the reply says \
                 nothing was measured here. The current directory is never guessed."
            ),
        }
    }

    /// A sentence for the handshake notice, when there is a root.
    pub fn notice(&self) -> Option<String> {
        self.root.as_ref().map(|(root, _)| {
            format!(
                "📏 FILES ARE MEASURED HERE. {LINK}, {SET_ONE}, {SET_MANY} and {RECONCILE} \
                 called without a checksum hash the folder's own files on this machine under {} \
                 (sha256, never content) and send the result; the server cannot see this disk and \
                 records each value as asserted, and the reply's `measurement` block says so. \
                 There is no need to compute or type a checksum by hand.",
                root.display()
            )
        })
    }

    /// Look at one line from the agent and decide what to send.
    pub async fn prepare<F: Forwarder>(&self, line: &str, up: &F) -> Prepared {
        let unchanged = || Prepared::Forward {
            body: line.to_string(),
            after: None,
        };
        let Ok(mut v) = serde_json::from_str::<Value>(line) else {
            return unchanged();
        };
        if v.get("method").and_then(Value::as_str) != Some("tools/call") {
            return unchanged();
        }
        let Some(name) = v
            .pointer("/params/name")
            .and_then(Value::as_str)
            .map(str::to_string)
        else {
            return unchanged();
        };
        let id = v.get("id").cloned().unwrap_or(Value::Null);
        let args = v
            .pointer("/params/arguments")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        let wants = match name.as_str() {
            LINK | SET_ONE => lacks(&args, "checksum"),
            SET_MANY => args
                .get("accepts")
                .and_then(Value::as_array)
                .is_some_and(|items| {
                    items
                        .iter()
                        .any(|a| a.as_object().is_some_and(|o| lacks(o, "checksum")))
                }),
            RECONCILE => args
                .get("observed")
                .is_none_or(|o| o.is_null() || o.as_array().is_some_and(Vec::is_empty)),
            _ => false,
        };
        if !wants {
            return unchanged();
        }
        let Some((root, _)) = &self.root else {
            return Prepared::Forward {
                body: line.to_string(),
                after: Some(Rewrite::NoRoot { id, tool: name }),
            };
        };

        let (new_args, after) = match name.as_str() {
            LINK => match self.link(root, args, &id, up).await {
                Some(x) => x,
                None => return unchanged(),
            },
            SET_ONE => match self.set_one(root, args, &id, up).await {
                Ok(Some(x)) => x,
                Ok(None) => return unchanged(),
                Err(refusal) => return Prepared::Answer(vec![refusal_message(&id, &refusal)]),
            },
            SET_MANY => match self.set_many(root, args, &id, up).await {
                Ok(x) => x,
                Err(refusal) => return Prepared::Answer(vec![refusal_message(&id, &refusal)]),
            },
            RECONCILE => match self.reconcile(root, args, &id, up).await {
                Ok(x) => x,
                Err(refusal) => return Prepared::Answer(vec![refusal_message(&id, &refusal)]),
            },
            _ => return unchanged(),
        };
        if let Some(params) = v.get_mut("params").and_then(Value::as_object_mut) {
            params.insert("arguments".into(), Value::Object(new_args));
        }
        Prepared::Forward {
            body: v.to_string(),
            after: Some(after),
        }
    }

    /// `link_artifact`: measure the location (given, or the stored one on a
    /// re-link) and inject the checksum. `None` when there is no location at
    /// all — the server then says "nothing to measure" itself.
    async fn link<F: Forwarder>(
        &self,
        root: &Path,
        mut args: Map<String, Value>,
        id: &Value,
        up: &F,
    ) -> Option<(Map<String, Value>, Rewrite)> {
        let location = match args.get("location").and_then(Value::as_str) {
            Some(l) => l.to_string(),
            None => {
                let artifact_id = args.get("artifact_id").and_then(Value::as_str)?;
                self.stored_location(artifact_id, up).await.ok().flatten()?
            }
        };
        let outcome = measure(root, &location, Some(&self.memo));
        if let Ok(m) = &outcome {
            args.insert("checksum".into(), Value::String(m.checksum.clone()));
        }
        Some((
            args,
            Rewrite::Link {
                id: id.clone(),
                outcome,
            },
        ))
    }

    /// `set_artifact_checksum`: measure the artifact's stored location and
    /// inject the checksum; refused by name when it cannot be measured.
    async fn set_one<F: Forwarder>(
        &self,
        root: &Path,
        mut args: Map<String, Value>,
        id: &Value,
        up: &F,
    ) -> Result<Option<(Map<String, Value>, Rewrite)>, String> {
        let Some(artifact_id) = args
            .get("artifact_id")
            .and_then(Value::as_str)
            .map(str::to_string)
        else {
            // No artifact named: the server refuses the shape itself.
            return Ok(None);
        };
        let measured = self.measure_stored(root, &artifact_id, up).await?;
        args.insert("checksum".into(), Value::String(measured.checksum.clone()));
        Ok(Some((
            args,
            Rewrite::Checksum {
                id: id.clone(),
                measured,
            },
        )))
    }

    /// `set_artifact_checksums`: every checksum-less item measured; ALL OF IT
    /// OR NONE OF IT, as the server's own batch — one unmeasurable item refuses
    /// the call, named.
    async fn set_many<F: Forwarder>(
        &self,
        root: &Path,
        mut args: Map<String, Value>,
        id: &Value,
        up: &F,
    ) -> Result<(Map<String, Value>, Rewrite), String> {
        let mut measured = Vec::new();
        let mut accepts = args
            .get("accepts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for item in accepts.iter_mut() {
            let Some(o) = item.as_object_mut() else {
                continue;
            };
            if !lacks(o, "checksum") {
                continue;
            }
            let Some(artifact_id) = o
                .get("artifact_id")
                .and_then(Value::as_str)
                .map(str::to_string)
            else {
                continue;
            };
            // Named as the server names a batch item: `<id>: <why>`.
            let m = self
                .measure_stored(root, &artifact_id, up)
                .await
                .map_err(|e| format!("{artifact_id}: {e}"))?;
            o.insert("checksum".into(), Value::String(m.checksum.clone()));
            measured.push((artifact_id, m));
        }
        args.insert("accepts".into(), Value::Array(accepts));
        Ok((
            args,
            Rewrite::Checksums {
                id: id.clone(),
                measured,
            },
        ))
    }

    /// `reconcile_artifacts` with nothing observed: page every Artifact,
    /// measure each location, and send what the server's `measure_registered`
    /// would have built.
    async fn reconcile<F: Forwarder>(
        &self,
        root: &Path,
        mut args: Map<String, Value>,
        id: &Value,
        up: &F,
    ) -> Result<(Map<String, Value>, Rewrite), String> {
        let artifacts = self.every_artifact(up).await.map_err(|why| {
            format!(
                "nothing to reconcile: the registered artifacts could not be read from the \
                 design to measure them on this machine ({why})"
            )
        })?;
        let mut observed = Vec::new();
        let mut unmeasurable = Vec::new();
        let (mut measured, mut absent, mut without_location) = (0usize, 0usize, 0usize);
        for (artifact_id, location) in &artifacts {
            let Some(location) = location else {
                without_location += 1;
                continue;
            };
            match measure(root, location, Some(&self.memo)) {
                Ok(m) => {
                    measured += 1;
                    observed.push(json!({
                        "artifact_id": artifact_id, "present": true, "checksum": m.checksum
                    }));
                }
                Err(NotMeasured::Absent { .. }) => {
                    absent += 1;
                    observed.push(json!({ "artifact_id": artifact_id, "present": false }));
                }
                Err(why) => unmeasurable.push(json!({
                    "artifact_id": artifact_id,
                    "location": location,
                    "not_measured": why,
                    "reason": why.reason(),
                })),
            }
        }
        if observed.is_empty() {
            // The server would refuse an empty `observed` as "no tree", which
            // is true of it and not the point: say what THIS machine found.
            let mut why = format!(
                "nothing to reconcile: of {} registered artifact(s), none could be measured on \
                 this machine under {}",
                artifacts.len(),
                root.display()
            );
            if without_location > 0 {
                why.push_str(&format!(" — {without_location} carry no location"));
            }
            for u in &unmeasurable {
                why.push_str(&format!(
                    "; {} — {}",
                    u["artifact_id"].as_str().unwrap_or_default(),
                    u["reason"].as_str().unwrap_or_default()
                ));
            }
            return Err(why);
        }
        args.insert("observed".into(), Value::Array(observed));
        let block = json!({
            "basis": "asserted",
            "measured_by": "client",
            "root": root.display().to_string(),
            "measured": measured,
            "absent": absent,
            "without_location": without_location,
            "unmeasurable": unmeasurable,
            "note": self.note(),
        });
        Ok((
            args,
            Rewrite::Reconcile {
                id: id.clone(),
                block,
            },
        ))
    }

    /// Measure the location stored on `artifact_id`, with the server's own
    /// refusals for an artifact that has none or cannot be measured
    /// (`ReflowService::checksum_or_measure`).
    async fn measure_stored<F: Forwarder>(
        &self,
        root: &Path,
        artifact_id: &str,
        up: &F,
    ) -> Result<Measurement, String> {
        let location = self
            .stored_location(artifact_id, up)
            .await
            .map_err(|why| {
                format!(
                    "`{artifact_id}` could not be measured on this machine: its location could \
                     not be read from the design ({why}) — pass `checksum` to assert one"
                )
            })?
            .ok_or_else(|| {
                format!(
                    "`{artifact_id}` has no location, so there is nothing to measure — pass \
                     `checksum` to assert one"
                )
            })?;
        measure(root, &location, Some(&self.memo)).map_err(|why| {
            format!(
                "`{artifact_id}` was not measured: {} — pass `checksum` to assert one. (Measured \
                 on this machine by your reflow2 client under {}; the server cannot see this \
                 disk.)",
                why.reason(),
                root.display()
            )
        })
    }

    /// The `location` stored on an Artifact, through an internal `get_node`.
    /// `Ok(None)` when the node is absent or has none.
    async fn stored_location<F: Forwarder>(
        &self,
        artifact_id: &str,
        up: &F,
    ) -> Result<Option<String>, String> {
        let payload = self
            .call(
                up,
                "get_node",
                json!({ "id": artifact_id, "node_type": reflow2_core::nodes::node::ARTIFACT }),
            )
            .await?;
        Ok(payload
            .pointer("/node/properties/location")
            .and_then(Value::as_str)
            .map(str::to_string))
    }

    /// Every Artifact's id and stored location, paged through `scan_nodes`.
    async fn every_artifact<F: Forwarder>(
        &self,
        up: &F,
    ) -> Result<Vec<(String, Option<String>)>, String> {
        let mut out = Vec::new();
        let mut offset = 0usize;
        loop {
            let page = self
                .call(
                    up,
                    "scan_nodes",
                    json!({ "node_type": reflow2_core::nodes::node::ARTIFACT, "offset": offset }),
                )
                .await?;
            for item in page
                .get("items")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let Some(node_id) = item.get("node_id").and_then(Value::as_str) else {
                    continue;
                };
                let location = item
                    .pointer("/properties/location")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                out.push((node_id.to_string(), location));
            }
            match page.get("next_offset").and_then(Value::as_u64) {
                Some(next) if (next as usize) > offset => offset = next as usize,
                _ => return Ok(out),
            }
        }
    }

    /// One internal `tools/call` on the same session, under this client's own
    /// id. The payload comes back parsed; an error, a tool error or a reply
    /// with no payload is `Err` with the reason.
    async fn call<F: Forwarder>(&self, up: &F, name: &str, args: Value) -> Result<Value, String> {
        let n = self.next_id.fetch_add(1, Ordering::Relaxed);
        let id = format!("reflow2-client-measure-{n}");
        let body = json!({
            "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": { "name": name, "arguments": args }
        })
        .to_string();
        let messages = up.forward(body).await.map_err(|e| format!("{e:#}"))?;
        let reply = messages
            .iter()
            .filter_map(|m| serde_json::from_str::<Value>(m).ok())
            .find(|v| v.get("id").and_then(Value::as_str) == Some(id.as_str()))
            .ok_or_else(|| format!("the server did not answer `{name}`"))?;
        if let Some(err) = reply.get("error") {
            return Err(format!(
                "the server refused `{name}`: {}",
                err.get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("no message")
            ));
        }
        let result = reply
            .get("result")
            .ok_or_else(|| format!("the server answered `{name}` with no result"))?;
        if result.get("isError") == Some(&Value::Bool(true)) {
            return Err(format!(
                "`{name}` failed: {}",
                result
                    .pointer("/content/0/text")
                    .and_then(Value::as_str)
                    .unwrap_or("no text")
            ));
        }
        payload_of(result).ok_or_else(|| format!("the server answered `{name}` with no payload"))
    }

    fn note(&self) -> String {
        format!(
            "measured on this machine by your reflow2 client under {}, with the server's own \
             measuring code (sha256 over the file's bytes, never content); the server cannot see \
             this disk and records the value as asserted.",
            self.root()
                .map(|r| r.display().to_string())
                .unwrap_or_default()
        )
    }

    /// Rewrite the server's reply to a prepared call so it tells the truth
    /// about where the value came from. Messages that are not that reply pass
    /// through untouched.
    pub fn rewrite(&self, messages: Vec<String>, after: &Rewrite) -> Vec<String> {
        messages
            .into_iter()
            .map(|m| self.rewrite_one(m, after))
            .collect()
    }

    fn rewrite_one(&self, message: String, after: &Rewrite) -> String {
        let Ok(mut v) = serde_json::from_str::<Value>(&message) else {
            return message;
        };
        let (id, is_error) = match after {
            Rewrite::Link { id, .. }
            | Rewrite::Checksum { id, .. }
            | Rewrite::Checksums { id, .. }
            | Rewrite::Reconcile { id, .. } => (id, false),
            Rewrite::NoRoot { id, .. } => (id, v.get("error").is_some()),
        };
        if v.get("id") != Some(id) {
            return message;
        }
        if is_error {
            // The server's refusal stands (it holds no tree); this client adds
            // that nothing was measured here either, and why.
            if let Some(msg) = v
                .pointer("/error/message")
                .and_then(Value::as_str)
                .map(str::to_string)
            {
                let tool = match after {
                    Rewrite::NoRoot { tool, .. } => tool.as_str(),
                    _ => "",
                };
                let joined = format!("{msg} {}", no_root_sentence(tool));
                if let Some(slot) = v.pointer_mut("/error/message") {
                    *slot = Value::String(joined);
                }
            }
            return v.to_string();
        }
        let Some(result) = v.get_mut("result").and_then(Value::as_object_mut) else {
            return message;
        };
        if result.get("isError") == Some(&Value::Bool(true)) {
            return message;
        }
        let root = self
            .root()
            .map(|r| r.display().to_string())
            .unwrap_or_default();
        let note = self.note();
        let edit = |payload: &mut Map<String, Value>| match after {
            Rewrite::Link { outcome, .. } => {
                let server = payload.remove("measurement").unwrap_or(Value::Null);
                let block = match outcome {
                    Ok(m) => json!({
                        "basis": "asserted", "measured_by": "client",
                        "checksum": m.checksum, "bytes": m.bytes, "measured_path": m.measured_path,
                        "root": root, "server": server, "note": note,
                    }),
                    Err(why) => json!({
                        "basis": "none", "measured_by": "client", "not_measured": why,
                        "root": root, "server": server,
                        "note": format!(
                            "no baseline recorded — {} (measured on this machine by your reflow2 \
                             client under {root}; the server cannot see this disk)",
                            why.reason()
                        ),
                    }),
                };
                payload.insert("measurement".into(), block);
            }
            Rewrite::Checksum { measured, .. } => {
                let server = payload.remove("measurement").unwrap_or(Value::Null);
                payload.insert(
                    "measurement".into(),
                    json!({
                        "basis": "asserted", "measured_by": "client",
                        "checksum": measured.checksum, "bytes": measured.bytes,
                        "measured_path": measured.measured_path,
                        "root": root, "server": server, "note": note,
                    }),
                );
            }
            Rewrite::Checksums { measured, .. } => {
                if let Some(list) = payload
                    .get_mut("measurements")
                    .and_then(Value::as_array_mut)
                {
                    for entry in list.iter_mut() {
                        let Some(o) = entry.as_object_mut() else {
                            continue;
                        };
                        let Some(aid) = o.get("artifact_id").and_then(Value::as_str) else {
                            continue;
                        };
                        let Some((_, m)) = measured.iter().find(|(id, _)| id == aid) else {
                            continue;
                        };
                        let server = o.remove("measurement").unwrap_or(Value::Null);
                        o.insert(
                            "measurement".into(),
                            json!({
                                "basis": "asserted", "measured_by": "client",
                                "checksum": m.checksum, "bytes": m.bytes,
                                "measured_path": m.measured_path,
                                "root": root, "server": server, "note": note,
                            }),
                        );
                    }
                }
            }
            Rewrite::Reconcile { block, .. } => {
                let server = payload.remove("measurement").unwrap_or(Value::Null);
                let mut block = block.clone();
                if let Some(b) = block.as_object_mut() {
                    b.insert("server".into(), server);
                }
                payload.insert("measurement".into(), block);
            }
            Rewrite::NoRoot { tool, .. } => {
                let client = json!({
                    "measured": false, "why": "no_root", "note": no_root_sentence(tool),
                });
                match payload
                    .get_mut("measurement")
                    .and_then(Value::as_object_mut)
                {
                    Some(m) => {
                        m.insert("client".into(), client);
                    }
                    None => {
                        payload.insert("measurement".into(), json!({ "client": client }));
                    }
                }
            }
        };
        if let Some(sc) = result
            .get_mut("structuredContent")
            .and_then(Value::as_object_mut)
        {
            edit(sc);
        } else if let Some(text) = result
            .get("content")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("text"))
            .and_then(Value::as_str)
            .map(str::to_string)
            && let Ok(Value::Object(mut payload)) = serde_json::from_str::<Value>(&text)
        {
            // An older server that answered in text only: rewrite that.
            edit(&mut payload);
            if let Some(slot) = result
                .get_mut("content")
                .and_then(Value::as_array_mut)
                .and_then(|c| c.first_mut())
                .and_then(Value::as_object_mut)
            {
                slot.insert(
                    "text".into(),
                    Value::String(Value::Object(payload).to_string()),
                );
            }
        }
        v.to_string()
    }
}

/// Whether `key` is absent or null in a call's arguments.
fn lacks(args: &Map<String, Value>, key: &str) -> bool {
    args.get(key).is_none_or(Value::is_null)
}

/// What a reply is told when this client has no root.
fn no_root_sentence(tool: &str) -> String {
    format!(
        "Nothing was measured on this machine either: your reflow2 client was started with no \
         .reflow2.toml folder and no --tree-root, so it has no root to measure `{tool}`'s files \
         under, and it never guesses the current directory. Start it in the folder that holds \
         .reflow2.toml, or pass --tree-root, and the checksum is measured here."
    )
}

/// A tool result's payload: `structuredContent`, or the text content parsed.
fn payload_of(result: &Value) -> Option<Value> {
    if let Some(sc) = result.get("structuredContent")
        && sc.is_object()
    {
        return Some(sc.clone());
    }
    result
        .pointer("/content/0/text")
        .and_then(Value::as_str)
        .and_then(|t| serde_json::from_str::<Value>(t).ok())
        .filter(Value::is_object)
}

/// A local refusal in the shape the server's own refusal takes: a JSON-RPC
/// `invalid params` error (-32602), the code `McpError::invalid_params` uses.
fn refusal_message(id: &Value, why: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": -32602, "message": why }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// A stand-in server: answers get_node / scan_nodes from a table, records
    /// every body it was sent.
    struct Table {
        artifacts: Vec<(&'static str, Option<&'static str>)>,
        sent: Mutex<Vec<String>>,
    }

    impl Forwarder for Table {
        fn forward(
            &self,
            body: String,
        ) -> impl std::future::Future<Output = anyhow::Result<Vec<String>>> + Send {
            let v: Value = serde_json::from_str(&body).unwrap();
            self.sent.lock().unwrap().push(body);
            let id = v["id"].clone();
            let name = v["params"]["name"].as_str().unwrap_or_default();
            let payload = match name {
                "get_node" => {
                    let want = v["params"]["arguments"]["id"].as_str().unwrap();
                    match self.artifacts.iter().find(|(a, _)| *a == want) {
                        Some((a, Some(l))) => {
                            json!({ "node": { "node_id": a, "properties": { "location": l } } })
                        }
                        Some((a, None)) => json!({ "node": { "node_id": a, "properties": {} } }),
                        None => json!({ "node": null }),
                    }
                }
                "scan_nodes" => {
                    // Two pages, to pin the paging.
                    let offset = v["params"]["arguments"]["offset"].as_u64().unwrap_or(0) as usize;
                    let items: Vec<Value> = self
                        .artifacts
                        .iter()
                        .skip(offset)
                        .take(1)
                        .map(|(a, l)| json!({ "node_id": a, "properties": { "location": l } }))
                        .collect();
                    let next = offset + items.len();
                    json!({
                        "count": items.len(), "items": items, "total": self.artifacts.len(),
                        "next_offset": (next < self.artifacts.len()).then_some(next),
                    })
                }
                _ => json!({ "ok": true, "measurement": { "basis": "asserted" } }),
            };
            let reply = json!({ "jsonrpc": "2.0", "id": id, "result": {
                "structuredContent": payload, "content": [{ "type": "text", "text": "see structuredContent" }]
            } })
            .to_string();
            async move { Ok(vec![reply]) }
        }
    }

    fn tree() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("src")).unwrap();
        std::fs::write(d.path().join("src/a.rs"), b"fn a() {}\n").unwrap();
        d
    }

    fn call(name: &str, args: Value) -> String {
        json!({ "jsonrpc": "2.0", "id": 7, "method": "tools/call",
                "params": { "name": name, "arguments": args } })
        .to_string()
    }

    fn table(artifacts: Vec<(&'static str, Option<&'static str>)>) -> Table {
        Table {
            artifacts,
            sent: Mutex::new(Vec::new()),
        }
    }

    #[tokio::test]
    async fn a_link_without_a_checksum_gets_the_files_sha256_and_the_reply_says_who_measured() {
        let d = tree();
        let cm = ClientMeasure::new(Some((d.path().to_path_buf(), RootSource::TreeRoot)));
        let up = table(vec![]);
        let line = call(
            LINK,
            json!({ "artifact_id": "art:a", "location": "src/a.rs", "target_id": "comp:x" }),
        );
        let Prepared::Forward { body, after } = cm.prepare(&line, &up).await else {
            panic!("forwarded");
        };
        let sent: Value = serde_json::from_str(&body).unwrap();
        let want = measure(d.path(), "src/a.rs", None).unwrap().checksum;
        assert_eq!(sent["params"]["arguments"]["checksum"], want);
        assert_eq!(sent["id"], 7, "the agent's own id is kept");
        assert!(
            up.sent.lock().unwrap().is_empty(),
            "a location in the call needs no lookup"
        );
        let reply = json!({ "jsonrpc": "2.0", "id": 7, "result": { "structuredContent": {
            "artifact": {}, "measurement": { "basis": "asserted", "not_measured": { "why": "no_tree" } }
        } } })
        .to_string();
        let out = cm.rewrite(vec![reply], after.as_ref().unwrap());
        let v: Value = serde_json::from_str(&out[0]).unwrap();
        let m = &v["result"]["structuredContent"]["measurement"];
        assert_eq!(m["basis"], "asserted");
        assert_eq!(m["measured_by"], "client");
        assert_eq!(m["checksum"], want);
        assert_eq!(m["server"]["not_measured"]["why"], "no_tree");
        assert!(
            m["note"]
                .as_str()
                .unwrap()
                .contains("records the value as asserted"),
            "{m}"
        );
    }

    #[tokio::test]
    async fn a_link_with_a_checksum_passes_through_untouched() {
        let d = tree();
        let cm = ClientMeasure::new(Some((d.path().to_path_buf(), RootSource::TreeRoot)));
        let line = call(
            LINK,
            json!({ "artifact_id": "art:a", "location": "src/a.rs", "target_id": "comp:x", "checksum": "sha256:abc" }),
        );
        let Prepared::Forward { body, after } = cm.prepare(&line, &table(vec![])).await else {
            panic!("forwarded");
        };
        assert_eq!(body, line);
        assert!(after.is_none());
    }

    #[tokio::test]
    async fn a_link_outside_the_root_is_forwarded_without_a_checksum_and_the_reply_names_why() {
        let d = tree();
        let cm = ClientMeasure::new(Some((d.path().to_path_buf(), RootSource::TreeRoot)));
        let line = call(
            LINK,
            json!({ "artifact_id": "art:a", "location": "/etc/hosts", "target_id": "comp:x" }),
        );
        let Prepared::Forward { body, after } = cm.prepare(&line, &table(vec![])).await else {
            panic!("forwarded");
        };
        let sent: Value = serde_json::from_str(&body).unwrap();
        assert!(sent["params"]["arguments"].get("checksum").is_none());
        let reply = json!({ "jsonrpc": "2.0", "id": 7, "result": { "structuredContent": {
            "measurement": { "basis": "none" } } } })
        .to_string();
        let out = cm.rewrite(vec![reply], after.as_ref().unwrap());
        let v: Value = serde_json::from_str(&out[0]).unwrap();
        let m = &v["result"]["structuredContent"]["measurement"];
        assert_eq!(m["basis"], "none");
        assert_eq!(m["not_measured"]["why"], "outside_root");
        assert!(
            m["note"].as_str().unwrap().contains("refused unread"),
            "{m}"
        );
    }

    #[tokio::test]
    async fn an_accept_without_a_checksum_looks_the_location_up_and_injects_the_measurement() {
        let d = tree();
        let cm = ClientMeasure::new(Some((d.path().to_path_buf(), RootSource::PointerFolder)));
        let up = table(vec![("art:a", Some("src/a.rs"))]);
        let line = call(
            SET_ONE,
            json!({ "artifact_id": "art:a", "disposition": "baseline_established" }),
        );
        let Prepared::Forward { body, after } = cm.prepare(&line, &up).await else {
            panic!("forwarded");
        };
        let sent: Value = serde_json::from_str(&body).unwrap();
        let want = measure(d.path(), "src/a.rs", None).unwrap().checksum;
        assert_eq!(sent["params"]["arguments"]["checksum"], want);
        let internal = up.sent.lock().unwrap().clone();
        assert_eq!(internal.len(), 1);
        let lookup: Value = serde_json::from_str(&internal[0]).unwrap();
        assert_eq!(lookup["params"]["name"], "get_node");
        assert!(
            lookup["id"]
                .as_str()
                .unwrap()
                .starts_with("reflow2-client-measure-"),
            "an internal call carries the client's own id: {lookup}"
        );
        assert!(matches!(after, Some(Rewrite::Checksum { .. })));
    }

    #[tokio::test]
    async fn an_accept_that_cannot_be_measured_is_refused_here_with_the_servers_words() {
        let d = tree();
        let cm = ClientMeasure::new(Some((d.path().to_path_buf(), RootSource::PointerFolder)));
        for (artifacts, says) in [
            (
                vec![("art:a", Some("../../etc/passwd"))],
                "resolves outside the project root",
            ),
            (vec![("art:a", Some("src/nope.rs"))], "nothing exists at"),
            (
                vec![("art:a", None)],
                "has no location, so there is nothing to measure",
            ),
            (vec![], "has no location, so there is nothing to measure"),
            (
                vec![("art:a", Some("https://x.example/spec"))],
                "is a URI, not a path",
            ),
        ] {
            let up = table(artifacts);
            let line = call(
                SET_ONE,
                json!({ "artifact_id": "art:a", "disposition": "baseline_established" }),
            );
            let Prepared::Answer(replies) = cm.prepare(&line, &up).await else {
                panic!("refused locally: {says}");
            };
            let v: Value = serde_json::from_str(&replies[0]).unwrap();
            assert_eq!(v["id"], 7);
            assert_eq!(v["error"]["code"], -32602);
            let msg = v["error"]["message"].as_str().unwrap();
            assert!(msg.contains(says), "{msg}");
            assert!(msg.contains("pass `checksum` to assert one"), "{msg}");
            assert!(
                up.sent
                    .lock()
                    .unwrap()
                    .iter()
                    .all(|b| b.contains("get_node")),
                "nothing but the lookup was sent"
            );
        }
    }

    #[tokio::test]
    async fn a_batch_measures_only_the_items_without_a_checksum_and_one_failure_refuses_it_all() {
        let d = tree();
        let cm = ClientMeasure::new(Some((d.path().to_path_buf(), RootSource::PointerFolder)));
        let up = table(vec![
            ("art:a", Some("src/a.rs")),
            ("art:b", Some("src/a.rs")),
        ]);
        let line = call(
            SET_MANY,
            json!({ "accepts": [
                { "artifact_id": "art:a", "disposition": "baseline_established" },
                { "artifact_id": "art:b", "disposition": "baseline_established", "checksum": "sha256:given" },
            ] }),
        );
        let Prepared::Forward { body, after } = cm.prepare(&line, &up).await else {
            panic!("forwarded");
        };
        let sent: Value = serde_json::from_str(&body).unwrap();
        let want = measure(d.path(), "src/a.rs", None).unwrap().checksum;
        assert_eq!(sent["params"]["arguments"]["accepts"][0]["checksum"], want);
        assert_eq!(
            sent["params"]["arguments"]["accepts"][1]["checksum"], "sha256:given",
            "an item with a checksum is untouched"
        );
        assert_eq!(
            up.sent.lock().unwrap().len(),
            1,
            "one lookup, for the one item"
        );
        let reply = json!({ "jsonrpc": "2.0", "id": 7, "result": { "structuredContent": {
            "applied": true,
            "measurements": [
                { "artifact_id": "art:a", "measurement": { "basis": "asserted" } },
                { "artifact_id": "art:b", "measurement": { "basis": "asserted" } },
            ] } } })
        .to_string();
        let out = cm.rewrite(vec![reply], after.as_ref().unwrap());
        let v: Value = serde_json::from_str(&out[0]).unwrap();
        let ms = &v["result"]["structuredContent"]["measurements"];
        assert_eq!(ms[0]["measurement"]["measured_by"], "client");
        assert_eq!(ms[0]["measurement"]["checksum"], want);
        assert_eq!(ms[1]["measurement"], json!({ "basis": "asserted" }));

        let up = table(vec![
            ("art:a", Some("src/a.rs")),
            ("art:gone", Some("src/gone.rs")),
        ]);
        let line = call(
            SET_MANY,
            json!({ "accepts": [
                { "artifact_id": "art:a", "disposition": "baseline_established" },
                { "artifact_id": "art:gone", "disposition": "baseline_established" },
            ] }),
        );
        let Prepared::Answer(replies) = cm.prepare(&line, &up).await else {
            panic!("the batch is refused whole");
        };
        let v: Value = serde_json::from_str(&replies[0]).unwrap();
        let msg = v["error"]["message"].as_str().unwrap();
        assert!(msg.starts_with("art:gone: "), "the item is named: {msg}");
        assert!(msg.contains("nothing exists at"), "{msg}");
    }

    #[tokio::test]
    async fn a_reconcile_with_nothing_observed_pages_every_artifact_and_sends_what_it_measured() {
        let d = tree();
        let cm = ClientMeasure::new(Some((d.path().to_path_buf(), RootSource::PointerFolder)));
        let up = table(vec![
            ("art:a", Some("src/a.rs")),
            ("art:gone", Some("src/gone.rs")),
            ("art:web", Some("https://x.example/spec")),
            ("art:none", None),
        ]);
        let line = call(RECONCILE, json!({}));
        let Prepared::Forward { body, after } = cm.prepare(&line, &up).await else {
            panic!("forwarded");
        };
        let sent: Value = serde_json::from_str(&body).unwrap();
        let observed = sent["params"]["arguments"]["observed"].as_array().unwrap();
        assert_eq!(observed.len(), 2, "{observed:?}");
        assert_eq!(observed[0]["artifact_id"], "art:a");
        assert_eq!(observed[0]["present"], true);
        assert!(
            observed[0]["checksum"]
                .as_str()
                .unwrap()
                .starts_with("sha256:")
        );
        assert_eq!(observed[1]["artifact_id"], "art:gone");
        assert_eq!(observed[1]["present"], false);
        assert!(observed[1].get("checksum").is_none());
        let pages = up.sent.lock().unwrap().len();
        assert_eq!(
            pages, 4,
            "one scan_nodes page per artifact, paged by next_offset"
        );

        let reply = json!({ "jsonrpc": "2.0", "id": 7, "result": { "structuredContent": {
            "missing": [], "measurement": { "basis": "asserted", "note": "supplied by the caller" }
        } } })
        .to_string();
        let out = cm.rewrite(vec![reply], after.as_ref().unwrap());
        let v: Value = serde_json::from_str(&out[0]).unwrap();
        let m = &v["result"]["structuredContent"]["measurement"];
        assert_eq!(m["basis"], "asserted");
        assert_eq!(m["measured_by"], "client");
        assert_eq!(m["measured"], 1);
        assert_eq!(m["absent"], 1);
        assert_eq!(m["without_location"], 1);
        assert_eq!(m["unmeasurable"][0]["artifact_id"], "art:web");
        assert_eq!(m["unmeasurable"][0]["not_measured"]["why"], "remote");
        assert_eq!(m["server"]["note"], "supplied by the caller");
    }

    #[tokio::test]
    async fn a_reconcile_that_was_given_observations_passes_through() {
        let d = tree();
        let cm = ClientMeasure::new(Some((d.path().to_path_buf(), RootSource::PointerFolder)));
        let line = call(
            RECONCILE,
            json!({ "observed": [{ "artifact_id": "art:a", "present": true }] }),
        );
        let Prepared::Forward { body, after } = cm.prepare(&line, &table(vec![])).await else {
            panic!("forwarded");
        };
        assert_eq!(body, line);
        assert!(after.is_none());
    }

    #[tokio::test]
    async fn with_no_root_nothing_is_measured_and_the_reply_says_so() {
        let cm = ClientMeasure::new(None);
        let up = table(vec![("art:a", Some("src/a.rs"))]);
        let line = call(
            SET_ONE,
            json!({ "artifact_id": "art:a", "disposition": "baseline_established" }),
        );
        let Prepared::Forward { body, after } = cm.prepare(&line, &up).await else {
            panic!("forwarded as it came");
        };
        assert_eq!(body, line);
        assert!(
            up.sent.lock().unwrap().is_empty(),
            "no lookup without a root"
        );
        let after = after.expect("the reply is told");
        // The server refuses (no tree); the client adds that it measured nothing.
        let refused = json!({ "jsonrpc": "2.0", "id": 7, "error": { "code": -32602,
            "message": "`art:a` was not measured: this server does not hold the project tree" } })
        .to_string();
        let out = cm.rewrite(vec![refused], &after);
        let v: Value = serde_json::from_str(&out[0]).unwrap();
        let msg = v["error"]["message"].as_str().unwrap();
        assert!(msg.starts_with("`art:a` was not measured"), "{msg}");
        assert!(
            msg.contains("Nothing was measured on this machine either"),
            "{msg}"
        );
        assert!(msg.contains("never guesses the current directory"), "{msg}");
        // A link that succeeded without a baseline is told the same.
        let line = call(
            LINK,
            json!({ "artifact_id": "art:a", "location": "src/a.rs", "target_id": "comp:x" }),
        );
        let Prepared::Forward { after, .. } = cm.prepare(&line, &up).await else {
            panic!("forwarded as it came");
        };
        let ok = json!({ "jsonrpc": "2.0", "id": 7, "result": { "structuredContent": {
            "measurement": { "basis": "none" } } } })
        .to_string();
        let out = cm.rewrite(vec![ok], &after.unwrap());
        let v: Value = serde_json::from_str(&out[0]).unwrap();
        let m = &v["result"]["structuredContent"]["measurement"];
        assert_eq!(m["basis"], "none", "the server's block is kept");
        assert_eq!(m["client"]["measured"], false);
        assert_eq!(m["client"]["why"], "no_root");
    }

    #[tokio::test]
    async fn other_calls_and_other_methods_are_not_looked_at() {
        let d = tree();
        let cm = ClientMeasure::new(Some((d.path().to_path_buf(), RootSource::PointerFolder)));
        let up = table(vec![]);
        for line in [
            call(
                "add_artifact",
                json!({ "id": "art:a", "location": "src/a.rs" }),
            ),
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#.to_string(),
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.to_string(),
            "not json".to_string(),
        ] {
            let Prepared::Forward { body, after } = cm.prepare(&line, &up).await else {
                panic!("forwarded: {line}");
            };
            assert_eq!(body, line);
            assert!(after.is_none());
        }
        assert!(up.sent.lock().unwrap().is_empty());
    }

    #[test]
    fn the_banner_and_notice_say_where_files_are_measured_or_that_they_are_not() {
        let d = tree();
        let with = ClientMeasure::new(Some((d.path().to_path_buf(), RootSource::PointerFolder)));
        let root = std::fs::canonicalize(d.path()).unwrap();
        assert!(with.banner().contains(&root.display().to_string()));
        assert!(with.banner().contains("the folder holding .reflow2.toml"));
        assert!(with.notice().unwrap().contains("FILES ARE MEASURED HERE"));
        let without = ClientMeasure::new(None);
        assert!(without.banner().contains("never guessed"));
        assert_eq!(
            without.notice(),
            None,
            "no root is said per call, not at the handshake"
        );
    }
}
