//! `--read-only` on a CLIENT: a process that forwards an agent's calls to a
//! server refuses every write ITSELF, before the call leaves.
//!
//! `fact:read-only-is-silently-ignored-by-the-remote-and-shared-clients-2026-10-02`.
//!
//! # The class this closes
//!
//! A server honours `--read-only` at its write guard (`ReflowService::write_lock`
//! and `file_write_permitted`). A CLIENT — `--remote URL`, `--shared`, a client
//! attached through a folder's `.reflow2.toml` — opens no store and has no write
//! guard: it forwards whatever line it is given, and the server it forwards to
//! was started by somebody else, writable. So the flag was accepted and changed
//! nothing, and a write through a `--read-only` client landed (measured
//! 2026-10-02 and again on main c4e1cbd for all three).
//!
//! The server cannot be asked to refuse on this client's behalf: it serves other
//! sessions, and a `--shared` daemon keeps the flags of whichever session started
//! it. The only process that knows this session asked for read-only is this one,
//! so this one screens every line before it is sent ([`ReadOnlyClient::screen`]).
//! Every forwarding loop in [`crate::proxy`] takes the flag as a required
//! argument, so a new client mode cannot forward without deciding.
//!
//! # How a call is classified, and why three sources
//!
//! A `tools/call` leaves only when ALL of these say it only reads:
//!
//! 1. **This build's own surface** ([`ReflowService::served_tools`]): the
//!    tool's `read_only_hint` here. A tool this build does not serve cannot be
//!    classified by it — a newer server's new tool, or another server's — and
//!    is refused as a write.
//! 2. **The server's own `tools/list`**: the tool's `readOnlyHint` THERE, read
//!    from the server's replies to this session and fetched by this client when
//!    a call names a tool it has not seen listed. A server that cannot be asked
//!    (unreachable, refused, an unreadable list), a tool it does not list, and a
//!    tool with no hint are all refused as writes.
//! 3. **The file rule** ([`crate::service::writes_a_file`]): `export_graph`
//!    and `export_surface` are annotated read-only because they do not write the
//!    GRAPH, but with `path` they write a FILE on the server. With `path` they are
//!    refused; without it they answer in the reply and pass — exactly what a
//!    read-only server refuses and allows.
//!
//! The first two are the same classification `--call` uses (the served
//! `read_only_hint`, item 1): the server's says what it will do, this build's
//! guards against a server that says less than it should. Neither is a list kept
//! by hand.
//!
//! Every other JSON-RPC method is a read by MCP's own definition or refused by
//! name: initialize, ping, the list methods, `prompts/get`, `resources/read`
//! and `completion/complete` pass; anything else that expects an answer is
//! refused, so a method this client does not know is never sent.
//!
//! # What is forwarded is what was screened
//!
//! A screened line is forwarded RE-SERIALISED from the value that was
//! classified, never as the bytes the client sent, so a server whose parser
//! reads the line differently (a duplicated key, say) cannot be handed a call
//! other than the one that was judged. A line that is not JSON is answered with
//! a parse error and not sent; a batch that carries anything refused is not sent
//! at all.
//!
//! # What it does not cover
//!
//! The refusal is an MCP tool error (`isError: true`, the form MCP gives a tool
//! that declined), naming `--read-only`. A served read-only refusal is a
//! JSON-RPC `invalid_request`; the two say the same thing in the shape each
//! party is given. A tool whose served hint is "write" but whose body would only
//! read (`design_identity` with no `label`) is refused here as `--call` refuses
//! it; a plain read-only server answers it, because its guard is the write
//! itself and not the hint.

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};
use tokio::sync::{Mutex, RwLock};

use crate::service::ReflowService;

/// The phrase every client-side refusal starts with, so a reader — and a
/// test — can tell this client's refusal from the server's.
pub const REFUSED_HERE: &str = "REFUSED BY THIS CLIENT";

/// JSON-RPC methods that read by MCP's own definition, and so pass a read-only
/// client unscreened. `tools/call` is screened per tool; anything not here that
/// expects an answer is refused by name.
const READ_METHODS: [&str; 9] = [
    "initialize",
    "ping",
    "tools/list",
    "prompts/list",
    "prompts/get",
    "resources/list",
    "resources/templates/list",
    "resources/read",
    "completion/complete",
];

/// A bound on the pages of a server's tool list this client will read before it
/// gives up classifying (and refuses). reflow2 serves its list in one page.
const MAX_LIST_PAGES: usize = 32;

/// Where a screened line goes.
pub trait Forwarder: Sync {
    /// Send one JSON-RPC message to the server and return every message it
    /// answered with.
    fn forward(&self, body: String) -> impl Future<Output = anyhow::Result<Vec<String>>> + Send;
}

/// What to do with one line from the client.
#[derive(Debug)]
pub enum Screened {
    /// Send this — the line as it was classified — to the server. `lists_tools`
    /// is set for a `tools/list`, whose reply teaches the screen the server's
    /// classification ([`ReadOnlyClient::learn`]).
    Forward { body: String, lists_tools: bool },
    /// Answer the client with these lines. Nothing is sent.
    Answer(Vec<String>),
}

/// Why a call is refused, said in the refusal.
#[derive(Debug, Clone, PartialEq)]
enum Why {
    /// The call names no tool.
    NoTool,
    /// This build serves the tool as a write.
    WritesHere,
    /// A file writer, given `path`.
    WritesAFile,
    /// This build serves no tool by that name.
    UnknownHere,
    /// The server's list could not be read.
    Unclassified(String),
    /// The server does not list the tool.
    NotOnTheServer,
    /// The server serves the tool as a write, or declares no hint.
    WritesThere,
}

/// The screen a `--read-only` client puts in front of every line it forwards.
pub struct ReadOnlyClient {
    /// Who calls go to, for the refusal text: a URL, or the shared server for a
    /// graph path.
    server: String,
    /// This build's surface: tool name → whether its `read_only_hint` is true.
    ours: HashMap<String, bool>,
    /// The server's surface, as its own `tools/list` said: name → reads.
    /// `None` until a list has been read.
    theirs: RwLock<Option<HashMap<String, bool>>>,
    /// Held while this client fetches the server's list, so concurrent calls
    /// that miss wait for one fetch instead of each making their own.
    fetching: Mutex<()>,
    next_id: AtomicU64,
}

impl ReadOnlyClient {
    /// A screen for calls going to `server` (named in every refusal).
    pub fn new(server: impl Into<String>) -> ReadOnlyClient {
        ReadOnlyClient {
            server: server.into(),
            ours: reads_by_name(&ReflowService::served_tools()),
            theirs: RwLock::new(None),
            fetching: Mutex::new(()),
            next_id: AtomicU64::new(0),
        }
    }

    /// The line to log at start, so an operator reading stderr knows what the
    /// flag does here.
    pub fn banner(&self) -> String {
        format!(
            "--read-only: this session refuses every write itself, before it is sent to {} — a \
             call leaves only when this reflow2 and the server's own tool list both say it only \
             reads. Reads, searches and reports pass through.",
            self.server
        )
    }

    /// Screen one line the client sent. Reads pass, re-serialised; everything
    /// else is answered here and never sent.
    pub async fn screen<F: Forwarder>(&self, line: &str, up: &F) -> Screened {
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                return Screened::Answer(vec![
                    json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": {
                            "code": -32700,
                            "message": format!(
                                "{REFUSED_HERE}, AND NOTHING WAS SENT: the line is not JSON ({e}), \
                                 so this --read-only session cannot tell what it asks for, and \
                                 sends nothing it cannot classify."
                            )
                        }
                    })
                    .to_string(),
                ]);
            }
        };
        match value {
            Value::Array(items) => self.screen_batch(items, up).await,
            one => {
                let lists_tools = method_of(&one) == Some("tools/list");
                match self.verdict(&one, up).await {
                    Ok(()) => Screened::Forward {
                        body: one.to_string(),
                        lists_tools,
                    },
                    Err(Some(answer)) => Screened::Answer(vec![answer.to_string()]),
                    Err(None) => Screened::Answer(Vec::new()),
                }
            }
        }
    }

    /// A batch is sent only when every message in it would be; otherwise each
    /// request in it is answered here and the batch is not sent. Sending part of
    /// a batch would split one answer the client is waiting for into two.
    async fn screen_batch<F: Forwarder>(&self, items: Vec<Value>, up: &F) -> Screened {
        let mut verdicts = Vec::with_capacity(items.len());
        for item in &items {
            verdicts.push(self.verdict(item, up).await);
        }
        if verdicts.iter().all(Result::is_ok) {
            let lists_tools = items.iter().any(|i| method_of(i) == Some("tools/list"));
            return Screened::Forward {
                body: Value::Array(items).to_string(),
                lists_tools,
            };
        }
        let answers: Vec<Value> = items
            .iter()
            .zip(verdicts)
            .filter_map(|(item, verdict)| match verdict {
                Err(answer) => answer,
                Ok(()) => item.get("id").map(|id| {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32600,
                            "message": format!(
                                "{REFUSED_HERE}, AND NOTHING WAS SENT: this request was in a batch \
                                 that also carried a call this --read-only session refuses, and a \
                                 batch is sent whole or not at all. Send this request on its own."
                            )
                        }
                    })
                }),
            })
            .collect();
        if answers.is_empty() {
            return Screened::Answer(Vec::new());
        }
        Screened::Answer(vec![Value::Array(answers).to_string()])
    }

    /// `Ok` when the message may be sent; otherwise the answer to give the
    /// client (`None` for a notification, which nobody waits on).
    async fn verdict<F: Forwarder>(&self, msg: &Value, up: &F) -> Result<(), Option<Value>> {
        let id = msg.get("id").cloned();
        let Some(method) = method_of(msg) else {
            if msg.get("method").is_some() {
                // A method that is not a string cannot be classified.
                return Err(
                    id.map(|id| method_refusal(&id, "a method that is not a string", &self.server))
                );
            }
            // A response to something the server asked: it invokes nothing.
            return Ok(());
        };
        let Some(id) = id else {
            // A notification: nobody waits on an answer. MCP's own notifications
            // invoke nothing; anything else is dropped here and said on stderr.
            if method.starts_with("notifications/") {
                return Ok(());
            }
            tracing::warn!(
                "--read-only: not sending a `{method}` notification — it is not one of MCP's own, \
                 so this session cannot tell whether it changes anything"
            );
            return Err(None);
        };
        if method == "tools/call" {
            let params = msg.get("params");
            let tool = params.and_then(|p| p.get("name")).and_then(Value::as_str);
            let args = params.and_then(|p| p.get("arguments"));
            return match self.classify(tool, args, up).await {
                Ok(()) => Ok(()),
                Err(why) => Err(Some(self.tool_refusal(&id, tool.unwrap_or_default(), why))),
            };
        }
        if READ_METHODS.contains(&method) {
            return Ok(());
        }
        Err(Some(method_refusal(&id, method, &self.server)))
    }

    /// Whether `tool`, called with `args`, only reads — by this build, by the
    /// file rule, and by the server's own list, in that order (the first two
    /// cost nothing, so a write never waits on the server).
    async fn classify<F: Forwarder>(
        &self,
        tool: Option<&str>,
        args: Option<&Value>,
        up: &F,
    ) -> Result<(), Why> {
        let Some(tool) = tool.filter(|t| !t.is_empty()) else {
            return Err(Why::NoTool);
        };
        match self.ours.get(tool) {
            None => return Err(Why::UnknownHere),
            Some(false) => return Err(Why::WritesHere),
            Some(true) => {}
        }
        if crate::service::writes_a_file(tool, args) {
            return Err(Why::WritesAFile);
        }
        match self.theirs_for(tool, up).await? {
            None => Err(Why::NotOnTheServer),
            Some(false) => Err(Why::WritesThere),
            Some(true) => Ok(()),
        }
    }

    /// The server's word on `tool`: `Some(reads)` if it lists it, `None` if it
    /// does not. Reads the server's list when it has not been read, or does not
    /// name the tool, once.
    async fn theirs_for<F: Forwarder>(&self, tool: &str, up: &F) -> Result<Option<bool>, Why> {
        if let Some(known) = self.theirs.read().await.as_ref()
            && let Some(reads) = known.get(tool)
        {
            return Ok(Some(*reads));
        }
        let _one_fetch = self.fetching.lock().await;
        // Another call may have fetched while this one waited.
        if let Some(known) = self.theirs.read().await.as_ref()
            && let Some(reads) = known.get(tool)
        {
            return Ok(Some(*reads));
        }
        let listed = self.fetch(up).await.map_err(Why::Unclassified)?;
        let answer = listed.get(tool).copied();
        *self.theirs.write().await = Some(listed);
        Ok(answer)
    }

    /// Read the server's whole tool list, every page, over this session.
    async fn fetch<F: Forwarder>(&self, up: &F) -> Result<HashMap<String, bool>, String> {
        let mut all = HashMap::new();
        let mut cursor: Option<String> = None;
        for _ in 0..MAX_LIST_PAGES {
            let id = format!(
                "reflow2-read-only-tools-{}",
                self.next_id.fetch_add(1, Ordering::Relaxed)
            );
            let params = match &cursor {
                Some(c) => json!({ "cursor": c }),
                None => json!({}),
            };
            let body =
                json!({"jsonrpc": "2.0", "id": id, "method": "tools/list", "params": params})
                    .to_string();
            let messages = up.forward(body).await.map_err(|e| format!("{e:#}"))?;
            let reply = messages
                .iter()
                .filter_map(|m| serde_json::from_str::<Value>(m).ok())
                .find(|v| v.get("id") == Some(&json!(id)))
                .ok_or_else(|| "the server answered tools/list with nothing".to_string())?;
            if let Some(err) = reply.get("error") {
                return Err(format!("the server refused tools/list: {err}"));
            }
            let tools = reply
                .pointer("/result/tools")
                .and_then(Value::as_array)
                .ok_or_else(|| "the server's tools/list reply holds no tool list".to_string())?;
            all.extend(reads_by_name_json(tools));
            cursor = reply
                .pointer("/result/nextCursor")
                .and_then(Value::as_str)
                .map(str::to_string);
            if cursor.is_none() {
                return Ok(all);
            }
        }
        Err(format!(
            "the server's tool list did not end within {MAX_LIST_PAGES} pages"
        ))
    }

    /// Learn from the server's replies to the client's own `tools/list`: they
    /// are the server's classification, and the freshest there is.
    pub async fn learn(&self, replies: &[String]) {
        let mut found = HashMap::new();
        for reply in replies {
            let Ok(v) = serde_json::from_str::<Value>(reply) else {
                continue;
            };
            let each = match v {
                Value::Array(items) => items,
                one => vec![one],
            };
            for m in each {
                if let Some(tools) = m.pointer("/result/tools").and_then(Value::as_array) {
                    found.extend(reads_by_name_json(tools));
                }
            }
        }
        if found.is_empty() {
            return;
        }
        let mut theirs = self.theirs.write().await;
        theirs.get_or_insert_with(HashMap::new).extend(found);
    }

    /// The tool error the client gets for a refused call.
    fn tool_refusal(&self, id: &Value, tool: &str, why: Why) -> Value {
        let version = env!("CARGO_PKG_VERSION");
        let reason = match why {
            Why::NoTool => "the call names no tool.".to_string(),
            Why::WritesHere => format!("`{tool}` writes the design (its read_only_hint is false)."),
            Why::WritesAFile => format!(
                "`{tool}` with `path` writes a file on the server. Call it without `path` to get \
                 the document in the reply."
            ),
            Why::UnknownHere => format!(
                "this reflow2-mcp ({version}) serves no tool named `{tool}`, so it cannot tell \
                 whether `{tool}` writes, and --read-only counts a tool it cannot classify as a \
                 write. If the server runs a newer reflow2, update this one."
            ),
            Why::Unclassified(e) => format!(
                "this client could not read the server's tool list to confirm that `{tool}` only \
                 reads ({e}), and --read-only counts a tool it cannot classify as a write."
            ),
            Why::NotOnTheServer => format!(
                "the server's own tool list does not name `{tool}`, so this client cannot tell \
                 whether it writes there, and --read-only counts it as a write."
            ),
            Why::WritesThere => format!(
                "the server serves `{tool}` as a write (its readOnlyHint there is not true)."
            ),
        };
        let text = format!(
            "{REFUSED_HERE}, AND NOTHING WAS SENT: this session was started with --read-only, \
             which refuses every write. {reason} The call never left this process, so the design \
             at {} is exactly as it was. Reads, searches and reports still work in this session; \
             to write, use a session started without --read-only.",
            self.server
        );
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {"content": [{"type": "text", "text": text}], "isError": true}
        })
    }
}

/// The JSON-RPC error for a method a read-only client does not send.
fn method_refusal(id: &Value, method: &str, server: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": -32600,
            "message": format!(
                "{REFUSED_HERE}, AND NOTHING WAS SENT: this session was started with --read-only, \
                 and `{method}` is not a method it knows to be a read, so it was not sent to \
                 {server}. A read-only session sends initialize, ping, the list methods, \
                 prompts/get, resources/read, completion/complete, and tools/call for a tool that \
                 only reads."
            )
        }
    })
}

/// The `method` of a JSON-RPC message, when it is a string.
fn method_of(msg: &Value) -> Option<&str> {
    msg.get("method").and_then(Value::as_str)
}

/// name → reads, from this build's own tool list.
fn reads_by_name(tools: &[rmcp::model::Tool]) -> HashMap<String, bool> {
    tools
        .iter()
        .map(|t| {
            let reads = t
                .annotations
                .as_ref()
                .and_then(|a| a.read_only_hint)
                .unwrap_or(false);
            (t.name.to_string(), reads)
        })
        .collect()
}

/// name → reads, from a server's `tools/list` JSON. No hint is not a read.
fn reads_by_name_json(tools: &[Value]) -> HashMap<String, bool> {
    tools
        .iter()
        .filter_map(|t| {
            let name = t.get("name")?.as_str()?.to_string();
            let reads = t.pointer("/annotations/readOnlyHint") == Some(&Value::Bool(true));
            Some((name, reads))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::FILE_WRITING_TOOLS;

    /// A server that answers `tools/list` with `tools`, and counts what it is sent.
    struct Fake {
        tools: Value,
        sent: std::sync::Mutex<Vec<String>>,
        fails: bool,
    }

    impl Fake {
        fn new(tools: Value) -> Fake {
            Fake {
                tools,
                sent: std::sync::Mutex::new(Vec::new()),
                fails: false,
            }
        }
    }

    impl Forwarder for Fake {
        fn forward(
            &self,
            body: String,
        ) -> impl Future<Output = anyhow::Result<Vec<String>>> + Send {
            self.sent.lock().unwrap().push(body.clone());
            let reply = if self.fails {
                Err(anyhow::anyhow!("connection refused"))
            } else {
                let v: Value = serde_json::from_str(&body).unwrap();
                Ok(vec![
                    json!({"jsonrpc": "2.0", "id": v["id"], "result": {"tools": self.tools}})
                        .to_string(),
                ])
            };
            async move { reply }
        }
    }

    fn call(id: u64, tool: &str, args: Value) -> String {
        json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":tool,"arguments":args}})
            .to_string()
    }

    fn served(name: &str, reads: bool) -> Value {
        json!({"name": name, "annotations": {"readOnlyHint": reads}})
    }

    /// THE SAME CLASSIFICATION `--call` USES, AND NO LIST KEPT BY HAND: a tool
    /// this build serves as a write is refused without asking the server.
    #[tokio::test]
    async fn a_write_is_refused_without_asking_the_server() {
        let fake = Fake::new(json!([]));
        let screen = ReadOnlyClient::new("http://server/");
        let out = screen
            .screen(&call(1, "add_requirement", json!({"id": "req:x"})), &fake)
            .await;
        let Screened::Answer(lines) = out else {
            panic!("a write was forwarded");
        };
        assert!(lines[0].contains(REFUSED_HERE) && lines[0].contains("--read-only"));
        assert!(fake.sent.lock().unwrap().is_empty());
    }

    /// A read the server also lists as a read is forwarded, re-serialised, and
    /// the server's list is read once.
    #[tokio::test]
    async fn a_read_both_sides_agree_on_is_forwarded_and_the_list_is_read_once() {
        let fake = Fake::new(json!([
            served("get_node", true),
            served("graph_report", true)
        ]));
        let screen = ReadOnlyClient::new("http://server/");
        for (i, tool) in ["get_node", "graph_report", "get_node"].iter().enumerate() {
            match screen.screen(&call(i as u64, tool, json!({})), &fake).await {
                Screened::Forward { body, lists_tools } => {
                    assert!(!lists_tools);
                    let v: Value = serde_json::from_str(&body).unwrap();
                    assert_eq!(v["params"]["name"], *tool);
                }
                Screened::Answer(a) => panic!("{tool} was refused: {a:?}"),
            }
        }
        assert_eq!(
            fake.sent.lock().unwrap().len(),
            1,
            "one tools/list, then the cache"
        );
    }

    /// Where the server cannot be asked, a read is refused as a write.
    #[tokio::test]
    async fn a_read_the_server_cannot_confirm_is_refused() {
        let mut fake = Fake::new(json!([]));
        fake.fails = true;
        let screen = ReadOnlyClient::new("http://server/");
        let Screened::Answer(lines) = screen.screen(&call(1, "get_node", json!({})), &fake).await
        else {
            panic!("an unclassified read was forwarded");
        };
        assert!(lines[0].contains("could not read the server's tool list"));
    }

    /// The file rule: `path` makes a file writer a write; without it, a read.
    #[tokio::test]
    async fn a_file_writer_is_a_write_only_with_a_path() {
        let fake = Fake::new(json!([served("export_graph", true)]));
        let screen = ReadOnlyClient::new("http://server/");
        assert!(matches!(
            screen
                .screen(&call(1, "export_graph", json!({"path": "/x"})), &fake)
                .await,
            Screened::Answer(_)
        ));
        assert!(matches!(
            screen
                .screen(&call(2, "export_graph", json!({"path": null})), &fake)
                .await,
            Screened::Forward { .. }
        ));
        assert!(matches!(
            screen
                .screen(&call(3, "export_graph", json!({})), &fake)
                .await,
            Screened::Forward { .. }
        ));
    }

    /// The server's own `tools/list` replies teach the screen, so a client that
    /// listed first never makes a list request of its own.
    #[tokio::test]
    async fn the_servers_reply_to_the_clients_own_list_is_learned() {
        let fake = Fake::new(json!([]));
        let screen = ReadOnlyClient::new("http://server/");
        let list = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
        assert!(matches!(
            screen.screen(list, &fake).await,
            Screened::Forward {
                lists_tools: true,
                ..
            }
        ));
        screen
            .learn(&[
                json!({"jsonrpc":"2.0","id":2,"result":{"tools":[served("get_node", true)]}})
                    .to_string(),
            ])
            .await;
        assert!(matches!(
            screen.screen(&call(3, "get_node", json!({})), &fake).await,
            Screened::Forward { .. }
        ));
        assert!(fake.sent.lock().unwrap().is_empty());
    }

    /// EVERY FILE WRITER TAKES ITS FILE AS `path`, the argument the file rule
    /// reads — held against the served schema, so renaming the argument on one
    /// tool cannot leave the rule reading a field that no longer exists.
    #[test]
    fn every_file_writer_is_served_and_takes_its_file_as_path() {
        let tools = ReflowService::served_tools();
        for name in FILE_WRITING_TOOLS {
            let tool = tools
                .iter()
                .find(|t| t.name == name)
                .unwrap_or_else(|| panic!("`{name}` is in FILE_WRITING_TOOLS and is not served"));
            assert!(
                tool.input_schema
                    .get("properties")
                    .and_then(|p| p.get("path"))
                    .is_some(),
                "`{name}` takes no `path`, so the file rule reads nothing"
            );
            assert_eq!(
                tool.annotations.as_ref().and_then(|a| a.read_only_hint),
                Some(true),
                "`{name}` is a graph read; if it became a graph write, the hint alone refuses it"
            );
        }
    }

    /// This build's half of the classification is the served surface itself:
    /// every served tool is classified, and the split matches the hints.
    #[test]
    fn this_builds_half_is_the_whole_served_surface() {
        let screen = ReadOnlyClient::new("x");
        let tools = ReflowService::served_tools();
        assert!(tools.len() > 50, "only {} tools served", tools.len());
        assert_eq!(screen.ours.len(), tools.len());
        assert_eq!(screen.ours.get("add_requirement"), Some(&false));
        assert_eq!(screen.ours.get("get_node"), Some(&true));
    }
}
