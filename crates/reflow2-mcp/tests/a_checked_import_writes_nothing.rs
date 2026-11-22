//! `import_graph` with `check_only` runs the whole import and WRITES NOTHING,
//! and its reply cannot be read as a write.
//!
//! WHY THE REPLY HAS A SHAPE OF ITS OWN. A check that would be accepted and an
//! import that was accepted are the two replies most easily taken for each
//! other. A caller that takes the first for the second goes on believing a
//! design holds what was never written. So a check says what it is
//! (`checked_only: true`, `written: false`, a `note` in plain words), counts
//! under `would_write`, and carries NO field a real import reports its writes
//! in (`nodes_written`, `edges_written`).
//!
//! A check that finds faults is a NORMAL reply: `would_be_refused: true` and
//! every fault in `faults`, in the words the refused import itself uses.
//!
//! What the engine does under each option is tested where the engine is
//! (`reflow2-core/tests/an_import_can_be_checked_added_only_and_held_to_proposals.rs`).
//! Here: the served reply, the served refusal of an unknown field, a store in
//! a folder (the identity an EMPTY store would adopt), and the `--call` door
//! (a check is served as a writing tool and must write no export).
#![cfg_attr(not(feature = "rocksdb"), allow(dead_code, unused_imports))]

use reflow2_mcp::service::*;
use rmcp::ServiceExt;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

macro_rules! j {
    ($call:expr) => {
        $call
            .await
            .expect("tool ok")
            .structured_content
            .expect("structured content present")
    };
}

macro_rules! call {
    ($s:expr, $tool:ident, $args:expr) => {
        j!($s.$tool(Parameters(serde_json::from_value($args).unwrap())))
    };
}

/// The fields a REAL import reports its writes in. A check carries neither.
const A_REAL_IMPORTS: [&str; 2] = ["nodes_written", "edges_written"];

/// Two new records and one relation between them: a document any design takes.
fn two_records() -> Value {
    json!({
        "nodes": [
            {"node_type": "Requirement", "node_id": "req:fed", "properties": {
                "name": "Fed", "statement": "The okapi is fed."}},
            {"node_type": "Requirement", "node_id": "req:watered", "properties": {
                "name": "Watered", "statement": "The okapi is watered."}}
        ],
        "edges": [
            {"edge_type": "DEPENDS_ON", "from_id": "req:fed", "to_id": "req:watered",
             "properties": {}}
        ]
    })
}

/// A document with two things wrong: a node type the schema does not know, and
/// a relation of a type it does not know either.
fn two_faults() -> Value {
    json!({
        "nodes": [
            {"node_type": "Requirement", "node_id": "req:fed", "properties": {
                "name": "Fed", "statement": "The okapi is fed."}},
            {"node_type": "Okapi", "node_id": "okapi:one", "properties": {"name": "One"}}
        ],
        "edges": [
            {"edge_type": "GRAZES_BESIDE", "from_id": "req:fed", "to_id": "req:fed",
             "properties": {}}
        ]
    })
}

/// Every node the design holds, with its properties: what a write would change.
async fn held(s: &ReflowService) -> Value {
    let export = call!(s, export_graph, json!({}));
    json!({"nodes": export["nodes"], "edges": export["edges"], "graph_id": export["graph_id"]})
}

fn says_it_was_a_check(reply: &Value) {
    assert_eq!(reply["checked_only"], true, "{reply}");
    assert_eq!(reply["written"], false, "{reply}");
    let note = reply["note"].as_str().unwrap_or_default();
    assert!(
        note.contains("THIS WAS A CHECK") && note.contains("NOTHING WAS WRITTEN"),
        "the note says in plain words that this was a check and nothing was written: {reply}"
    );
    assert!(reply["would_be_refused"].is_boolean(), "{reply}");
    assert!(
        reply["faults"].is_array(),
        "`faults` is always there: {reply}"
    );
    assert!(
        reply["would_write"]["nodes"].is_u64() && reply["would_write"]["edges"].is_u64(),
        "{reply}"
    );
    for field in A_REAL_IMPORTS {
        assert!(
            reply.get(field).is_none(),
            "a check carries no `{field}`: that is how a real import says what it wrote: {reply}"
        );
    }
}

/// The faults a REFUSED real import lists, read back out of its refusal.
fn listed(refusal: &str) -> Vec<String> {
    assert!(refusal.contains("NOTHING was written"), "{refusal}");
    refusal
        .split("\n  - ")
        .skip(1)
        .map(str::to_string)
        .collect()
}

#[tokio::test]
async fn a_clean_check_reads_as_a_check_and_the_same_import_then_writes_what_it_counted() {
    let s = ReflowService::in_memory().unwrap();
    call!(
        s,
        add_requirement,
        json!({"id": "req:held", "name": "Held", "statement": "Already here."})
    );
    let before = held(&s).await;

    let reply = call!(
        s,
        import_graph,
        json!({"document": two_records(), "check_only": true})
    );
    says_it_was_a_check(&reply);
    assert_eq!(reply["would_be_refused"], false, "{reply}");
    assert_eq!(reply["faults"], json!([]), "{reply}");
    assert_eq!(
        reply["would_write"],
        json!({"nodes": 2, "edges": 1}),
        "{reply}"
    );
    assert!(
        reply["note"]
            .as_str()
            .unwrap()
            .contains("would be accepted"),
        "{reply}"
    );
    assert_eq!(held(&s).await, before, "a check changed the design");

    // The real import, asked the same way without `check_only`, writes exactly
    // what the check counted, and reads as a write.
    let wrote = call!(s, import_graph, json!({"document": two_records()}));
    assert!(wrote.get("checked_only").is_none(), "{wrote}");
    assert!(wrote.get("written").is_none(), "{wrote}");
    assert!(wrote.get("would_write").is_none(), "{wrote}");
    assert_eq!(wrote["nodes_written"], reply["would_write"]["nodes"]);
    assert_eq!(wrote["edges_written"], reply["would_write"]["edges"]);
    assert_ne!(held(&s).await, before);
}

#[tokio::test]
async fn a_check_with_faults_is_a_normal_reply_that_lists_what_the_import_is_refused_for() {
    let s = ReflowService::in_memory().unwrap();
    let before = held(&s).await;

    let reply = call!(
        s,
        import_graph,
        json!({"document": two_faults(), "check_only": true})
    );
    says_it_was_a_check(&reply);
    assert_eq!(reply["would_be_refused"], true, "{reply}");
    assert_eq!(
        reply["would_write"],
        json!({"nodes": 0, "edges": 0}),
        "an import that would be refused would write nothing: {reply}"
    );
    assert!(
        reply["note"].as_str().unwrap().contains("would be REFUSED"),
        "{reply}"
    );
    let faults: Vec<String> = reply["faults"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap().to_string())
        .collect();
    assert_eq!(faults.len(), 2, "both faults, not the first: {reply}");
    assert!(faults[0].starts_with("nodes[1] okapi:one"), "{faults:?}");
    assert!(
        faults[1].starts_with("edges[0] GRAZES_BESIDE req:fed -> req:fed: "),
        "{faults:?}"
    );
    assert_eq!(held(&s).await, before, "a check changed the design");

    // The same import for real is REFUSED, for the same faults in the same
    // words, and writes nothing either.
    let refused = s
        .import_graph(Parameters(
            serde_json::from_value(json!({"document": two_faults()})).unwrap(),
        ))
        .await
        .expect_err("the real import is refused");
    assert_eq!(listed(&refused.message), faults);
    assert_eq!(held(&s).await, before);
}

#[tokio::test]
async fn the_two_refusing_options_are_served_and_a_check_lists_what_they_refuse() {
    let s = ReflowService::in_memory().unwrap();
    call!(
        s,
        add_requirement,
        json!({"id": "req:fed", "name": "Fed", "statement": "The okapi is fed."})
    );
    let before = held(&s).await;

    // ADD ONLY: `req:fed` is held, `req:watered` is new.
    let reply = call!(
        s,
        import_graph,
        json!({"document": two_records(), "check_only": true, "add_only": true})
    );
    says_it_was_a_check(&reply);
    assert_eq!(
        reply["faults"],
        json!([
            "nodes[0] req:fed: the design already holds this record, and an add-only import \
             never changes one it holds"
        ]),
        "{reply}"
    );

    // PROPOSALS ONLY: a requirement arriving accepted is a settle.
    let settled = json!({"nodes": [
        {"node_type": "Requirement", "node_id": "req:arrives-settled", "properties": {
            "name": "Settled", "statement": "It arrives accepted.", "status": "accepted"}}
    ], "edges": []});
    let reply = call!(
        s,
        import_graph,
        json!({"document": settled, "check_only": true, "proposals_only": true})
    );
    says_it_was_a_check(&reply);
    assert_eq!(
        reply["faults"],
        json!([
            "nodes[0] req:arrives-settled: this import takes proposals only, and this record \
             arrives settled (Requirement req:arrives-settled status=accepted): settling a \
             record is its owner's act on the design, never a file's"
        ]),
        "{reply}"
    );
    assert_eq!(held(&s).await, before, "a check changed the design");

    // Without `check_only` each is a refusal, and nothing is written.
    for args in [
        json!({"document": two_records(), "add_only": true}),
        json!({"document": settled, "proposals_only": true}),
    ] {
        let refused = s
            .import_graph(Parameters(serde_json::from_value(args.clone()).unwrap()))
            .await
            .expect_err("refused");
        assert_eq!(listed(&refused.message).len(), 1, "{args}: {refused:?}");
    }
    assert_eq!(held(&s).await, before);
}

// ---- over the wire --------------------------------------------------------

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "check-probe".to_string();
        cfg
    }
}

struct Session {
    client: rmcp::service::RunningService<rmcp::service::RoleClient, Probe>,
}

async fn session() -> Session {
    let svc = ReflowService::in_memory().expect("in-memory service");
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = svc.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    let client = Probe
        .serve((client_rx, client_tx))
        .await
        .expect("in-process handshake");
    Session { client }
}

impl Session {
    async fn call(&self, tool: &str, args: Value) -> Result<Value, String> {
        let arguments = args.as_object().cloned().unwrap_or_default();
        match self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
        {
            Ok(r) if r.is_error.unwrap_or(false) => Err(r
                .content
                .iter()
                .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                .collect::<Vec<_>>()
                .join("\n")),
            Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
            Err(rmcp::service::ServiceError::McpError(e)) => Err(e.message.to_string()),
            Err(e) => Err(format!("transport failure: {e}")),
        }
    }
}

/// Through a served session, where a call runs inside its write unit: the
/// check's reply, the design unchanged, and the import that follows.
#[tokio::test]
async fn a_served_check_writes_nothing_and_an_unknown_field_is_still_refused() {
    let s = session().await;
    let nodes = |v: &Value| v["nodes"].as_array().map(Vec::len).unwrap_or(usize::MAX);
    let before = s.call("export_graph", json!({})).await.unwrap();

    let reply = s
        .call(
            "import_graph",
            json!({"document": two_records(), "check_only": true}),
        )
        .await
        .expect("a check is a normal reply");
    says_it_was_a_check(&reply);
    assert_eq!(reply["would_write"], json!({"nodes": 2, "edges": 1}));

    let faulted = s
        .call(
            "import_graph",
            json!({"document": two_faults(), "check_only": true}),
        )
        .await
        .expect("a check with faults is a normal reply, never an error");
    says_it_was_a_check(&faulted);
    assert_eq!(faulted["would_be_refused"], true);
    assert_eq!(faulted["faults"].as_array().unwrap().len(), 2);

    let after = s.call("export_graph", json!({})).await.unwrap();
    assert_eq!(nodes(&after), nodes(&before), "a served check wrote a node");
    assert_eq!(after["content_hash"], before["content_hash"]);

    // An option this tool does not serve is REFUSED, not ignored: a caller
    // that misspells `check_only` must never get a real import instead.
    for unknown in ["check_onl", "dry_run", "checkOnly"] {
        let mut args = json!({"document": two_records()});
        args[unknown] = json!(true);
        let refusal = s
            .call("import_graph", args)
            .await
            .expect_err("an unknown field is refused");
        assert!(
            refusal.contains("refused before it ran")
                && refusal.contains(&format!("`{unknown}` is not a parameter of this tool")),
            "`{unknown}`: {refusal}"
        );
        // And the request type itself refuses it, whoever builds one without
        // the served schema in front of it.
        let mut args = json!({"document": two_records()});
        args[unknown] = json!(true);
        let err = serde_json::from_value::<reflow2_mcp::service::ImportGraphReq>(args)
            .err()
            .unwrap_or_else(|| panic!("`{unknown}` was read as a request"))
            .to_string();
        assert!(
            err.contains("unknown field") && err.contains(unknown),
            "`{unknown}`: {err}"
        );
    }
    let after = s.call("export_graph", json!({})).await.unwrap();
    assert_eq!(
        after["content_hash"], before["content_hash"],
        "a refused call wrote the document it carried"
    );

    // And the session a check ran in still writes: the same document, sent
    // without `check_only`, lands, and counts what the check said it would.
    let written = s
        .call("import_graph", json!({"document": two_records()}))
        .await
        .expect("the real import");
    assert!(written.get("checked_only").is_none(), "{written}");
    assert_eq!(written["nodes_written"], reply["would_write"]["nodes"]);
    assert_eq!(written["edges_written"], reply["would_write"]["edges"]);
    let after = s.call("export_graph", json!({})).await.unwrap();
    assert_eq!(nodes(&after), nodes(&before) + 2, "the real import landed");
}

// ---- a store in a folder --------------------------------------------------

/// AN EMPTY STORE ADOPTS THE NAME OF THE DESIGN IT RESTORES, and that is a
/// write: the identity file beside the store is rewritten. A check must not
/// make it. It says which name the import would adopt and leaves the store
/// under the one it minted, in memory and on disk.
#[cfg(feature = "rocksdb")]
#[test]
fn a_check_on_an_empty_store_leaves_the_stores_own_name_alone() {
    use reflow2_core::{DesignGraph, ImportOptions};

    let dir = tempfile::Builder::new()
        .prefix("reflow2-checked-import-")
        .tempdir()
        .unwrap();
    let store = dir.path().join("graph");
    let store_path = store.to_str().unwrap();

    // A design that knows its own name, exported from a store of its own.
    let document = {
        let src = dir.path().join("src-graph");
        let mut g = DesignGraph::open_rocksdb(src.to_str().unwrap()).unwrap();
        g.add_project("proj:x", "X").unwrap();
        g.export_graph().unwrap()
    };
    let theirs = document.graph_id.clone();
    assert_ne!(theirs, "", "the fixture carries a name");

    let files = |root: &std::path::Path| {
        let mut found = Vec::new();
        for entry in std::fs::read_dir(root).unwrap().flatten() {
            let p = entry.path();
            if p.is_file() {
                found.push((p.clone(), std::fs::read(&p).unwrap()));
            }
        }
        found.sort();
        found
    };

    let minted = {
        let mut g = DesignGraph::open_rocksdb(store_path).unwrap();
        let minted = g.graph_id().to_string();
        assert_ne!(minted, theirs, "the store starts under a name of its own");
        // The identity file lives beside the store: every plain file in the
        // folder, byte for byte.
        let beside = files(dir.path());
        assert!(
            beside
                .iter()
                .any(|(_, bytes)| String::from_utf8_lossy(bytes).contains(&minted)),
            "the fixture must hold the store's identity file, or this proves nothing: {:?}",
            beside.iter().map(|(p, _)| p).collect::<Vec<_>>()
        );

        let report = g
            .import_graph_with(
                &document,
                ImportOptions {
                    check_only: true,
                    ..Default::default()
                },
            )
            .expect("a check");
        assert!(
            report.checked_only && report.faults.is_empty(),
            "{report:?}"
        );
        assert_eq!(
            report.would_adopt_identity.as_deref(),
            Some(theirs.as_str()),
            "the check says which name the import would adopt"
        );
        assert!(
            report.adopted_identity.is_none(),
            "and does not say it adopted one"
        );
        assert_eq!(g.graph_id(), minted, "the open store kept its name");
        assert!(!g.holds_a_design(), "and holds nothing");
        assert_eq!(
            files(dir.path()),
            beside,
            "a check rewrote a file beside the store"
        );
        minted
    };

    // Reopened: the name on disk is the one the store minted. Then the real
    // import adopts, as it always has.
    let mut g = DesignGraph::open_rocksdb(store_path).unwrap();
    assert_eq!(g.graph_id(), minted, "the name on disk moved");
    let report = g.import_graph(&document).unwrap();
    assert_eq!(report.adopted_identity.as_deref(), Some(theirs.as_str()));
    assert_eq!(g.graph_id(), theirs);
}

// ---- the `--call` door ----------------------------------------------------

#[cfg(feature = "rocksdb")]
mod door {
    use serde_json::{Value, json};
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output, Stdio};

    fn bin() -> &'static str {
        env!("CARGO_BIN_EXE_reflow2-mcp")
    }

    const EXPORT: &str = "docs/design/proj.json";

    /// A project folder whose `.mcp.json` names the export a writing call keeps.
    struct Project {
        folder: tempfile::TempDir,
        outside: tempfile::TempDir,
    }

    impl Project {
        fn configured() -> Self {
            let folder = tempfile::Builder::new()
                .prefix("reflow2-checked-door-")
                .tempdir()
                .unwrap();
            let outside = tempfile::Builder::new()
                .prefix("reflow2-checked-door-home-")
                .tempdir()
                .unwrap();
            std::fs::write(
                folder.path().join(".mcp.json"),
                format!(
                    r#"{{"mcpServers":{{"reflow2":{{"command":"{}","args":["--graph-path","./.reflow2/graph","--export-to","{EXPORT}","--shared"]}}}}}}"#,
                    bin()
                ),
            )
            .unwrap();
            std::fs::create_dir_all(folder.path().join("docs/design")).unwrap();
            Project { folder, outside }
        }

        fn path(&self, rel: &str) -> PathBuf {
            self.folder.path().join(rel)
        }

        fn call(&self, tool: &str, args: &str) -> Output {
            let home = self.outside.path();
            Command::new(bin())
                .current_dir(self.folder.path())
                .args(["--call", tool, "--args", args])
                .env("HOME", home)
                .env("XDG_CONFIG_HOME", home.join(".config"))
                .env("REFLOW2_CONFIG_DIR", home.join("reflow2-config"))
                .env("RUST_LOG", "error")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env_remove("REFLOW2_CONTENT_POLICY")
                .env_remove("REFLOW2_TRUSTED_GATEWAY")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("the binary runs")
        }
    }

    fn err(o: &Output) -> String {
        String::from_utf8_lossy(&o.stderr).to_string()
    }

    fn json_of(o: &Output) -> Value {
        let text = String::from_utf8_lossy(&o.stdout);
        serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("stdout is not JSON ({e}):\n{text}\nstderr:\n{}", err(o)))
    }

    /// Bytes and modification time, so "untouched" means not rewritten at all.
    fn fingerprint(p: &Path) -> (Vec<u8>, std::time::SystemTime) {
        (
            std::fs::read(p).unwrap(),
            std::fs::metadata(p).unwrap().modified().unwrap(),
        )
    }

    /// `import_graph` is served as a WRITING tool, and the door writes the
    /// committed export after every writing call that succeeded. A check
    /// succeeded and wrote nothing: the export is not rewritten, one line on
    /// stderr says why, and stdout is the check's own reply. The same call
    /// without `check_only` then writes, and the export follows it.
    #[test]
    fn a_check_through_the_door_writes_no_export_and_says_so() {
        let p = Project::configured();
        let o = p.call("add_project", r#"{"id":"proj:p","name":"P"}"#);
        assert_eq!(o.status.code(), Some(0), "{}", err(&o));
        let export = p.path(EXPORT);
        assert!(
            export.exists(),
            "the first write keeps an export: {}",
            err(&o)
        );
        let before = fingerprint(&export);
        // Coarse mtimes would hide a rewrite made in the same tick.
        std::thread::sleep(std::time::Duration::from_millis(1100));

        let document = json!({"nodes": [
            {"node_type": "Requirement", "node_id": "req:through-the-door", "properties": {
                "name": "Through the door", "statement": "It arrives in a file."}}
        ], "edges": []});

        for (args, would_be_refused) in [
            (json!({"document": document, "check_only": true}), false),
            (
                json!({"document": {"nodes": [
                    {"node_type": "Okapi", "node_id": "okapi:one", "properties": {}}
                ], "edges": []}, "check_only": true}),
                true,
            ),
        ] {
            let o = p.call("import_graph", &args.to_string());
            assert_eq!(
                o.status.code(),
                Some(0),
                "a check is a normal reply, faults or none: {}",
                err(&o)
            );
            let reply = json_of(&o);
            assert_eq!(reply["checked_only"], true, "{reply}");
            assert_eq!(reply["written"], false, "{reply}");
            assert_eq!(reply["would_be_refused"], would_be_refused, "{reply}");
            assert!(reply.get("nodes_written").is_none(), "{reply}");
            let e = err(&o);
            assert!(
                e.contains("only CHECKED and wrote nothing") && !e.contains("export written"),
                "{e}"
            );
            assert_eq!(
                fingerprint(&export),
                before,
                "a check rewrote the committed export"
            );
        }
        let held = p.call("get_node", r#"{"id":"req:through-the-door"}"#);
        assert!(
            !String::from_utf8_lossy(&held.stdout).contains("It arrives in a file."),
            "a check wrote the record: {}",
            String::from_utf8_lossy(&held.stdout)
        );

        let o = p.call("import_graph", &json!({"document": document}).to_string());
        assert_eq!(o.status.code(), Some(0), "{}", err(&o));
        assert_eq!(json_of(&o)["nodes_written"], 1);
        assert!(err(&o).contains("export written to"), "{}", err(&o));
        assert_ne!(fingerprint(&export).0, before.0, "the real import's export");
        assert!(
            String::from_utf8_lossy(&std::fs::read(&export).unwrap())
                .contains("req:through-the-door")
        );
    }
}
