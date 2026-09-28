//! A code folder names the design it implements: `.reflow2.toml`.
//!
//! `req:a-code-folder-names-the-design-it-implements-and-its-server`, combined
//! plan position 3, first slice, on @ajs's word ("start on step 3").
//! Settled 2026-09-28 in
//! `dec:the-pointer-file-is-dot-reflow2-toml-and-the-first-slice-attaches-one-design`:
//!
//! ```toml
//! # .reflow2.toml — at the repository root, committed, holding no secret
//! [design]
//! id      = "0dcdeca97f8810ce"
//! address = "https://api.flo2.io/g/0dcdeca97f8810ce/mcp"
//!
//! [[also]]                       # other designs this folder works with
//! id      = "…"
//! address = "…"
//! role    = "consumes"
//! ```
//!
//! # What it changes
//!
//! Until now a folder named its design by PATH: the MCP entry says
//! `--graph-path .reflow2/graph`, and whatever store sits there is opened. That
//! stops being true the moment a design moves to a server. The store left behind
//! by the move still sits there, and a session opened in the folder was served
//! it as if it were the design (`fact:an-agent-opened-in-a-moved-designs-folder-is-served-the-frozen-store-2026-09-27`).
//!
//! With a pointer, the client started in the folder attaches to the design the
//! pointer names — whatever the MCP entry says about a local path — and:
//! · FOLLOWS THE POINTER EVEN WHEN A LOCAL STORE IS ALSO PRESENT, and says so,
//!   on stderr and in the handshake the agent reads. The local store is never
//!   opened.
//! · CHECKS the server's answer names the design the folder expects (the `id`),
//!   and refuses a different one rather than working the wrong design.
//! · LISTS any `[[also]]` designs and does not open them: opening several
//!   designs in one session waits on `dec:idea-a-session-holds-several-graphs`.
//!
//! # Why both an id and an address
//!
//! Servers address a design differently — api.flo2.io at `/g/<id>/mcp`, a
//! reflow2 registry at `/g/<id>/` — so the address is stored whole rather than
//! composed. The id is stored beside it so the pointer can be CHECKED, which is
//! the one thing the prose form of the same idea cannot do
//! (`fact:a-users-agents-md-line-already-names-the-remote-design-2026-09-26`).
//!
//! # Why not inside `.reflow2/`
//!
//! Every consumer project git-ignores the whole of `.reflow2/`
//! (tools/reflow2_init.py), and the pointer has to be committed.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Value, json};

/// The pointer's file name, at the folder a design's `.reflow2/` sits in.
pub const FILE: &str = ".reflow2.toml";

/// A folder's pointer, read and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pointer {
    /// Where it was read from.
    pub path: PathBuf,
    /// The design this folder implements.
    pub design: Named,
    /// Other designs the folder names. Listed, never opened (first slice).
    pub also: Vec<Also>,
}

/// The design a folder implements.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Named {
    pub id: String,
    pub address: String,
}

/// Another design a folder works with, and the part it plays.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Also {
    pub id: String,
    pub address: String,
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OnDisk {
    design: Named,
    #[serde(default)]
    also: Vec<Also>,
}

/// Where the pointer for a design at `graph_path` would be: the folder the
/// store's `.reflow2/` sits in (`<root>/.reflow2/graph` → `<root>/.reflow2.toml`).
pub fn location_for(graph_path: &str) -> PathBuf {
    crate::wall_check::project_root(Some(graph_path), None).join(FILE)
}

/// Read the pointer at `path`. `Ok(None)` when there is none — the folder names
/// its design by path, as it always has. `Err` when there is one and it cannot be
/// used: never silently fall back to a local store, which is the defect this
/// exists to close.
pub fn read(path: &Path) -> Result<Option<Pointer>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(format!(
                "{} exists but could not be read: {e}",
                path.display()
            ));
        }
    };
    parse(&text, path).map(Some)
}

/// Parse and check a pointer's text.
pub fn parse(text: &str, path: &Path) -> Result<Pointer, String> {
    let where_ = path.display();
    let on_disk: OnDisk = toml::from_str(text).map_err(|e| {
        let msg = e.message().to_string();
        let secret = ["key", "token", "secret", "password", "bearer"]
            .iter()
            .any(|w| msg.contains(&format!("`{w}`")));
        format!(
            "{where_} is not a pointer reflow2 can use: {msg}.{} It takes a [design] table with an \
             `id` and an `address`, and optional [[also]] entries with an `id`, an `address` and a \
             `role`.",
            if secret {
                " It holds NO secret: a key for a server belongs in `reflow2-mcp setup remote \
                 <server>`, which keeps it in the OS keychain."
            } else {
                ""
            }
        )
    })?;
    check(
        "[design]",
        &on_disk.design.id,
        &on_disk.design.address,
        &where_,
    )?;
    for a in &on_disk.also {
        check("[[also]]", &a.id, &a.address, &where_)?;
    }
    Ok(Pointer {
        path: path.to_path_buf(),
        design: on_disk.design,
        also: on_disk.also,
    })
}

fn check(
    table: &str,
    id: &str,
    address: &str,
    where_: &impl std::fmt::Display,
) -> Result<(), String> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "{where_}: {table} `id` must be a design's graph id (letters, digits, `-`, `_`), got \
             {id:?}. `design_identity` on the design says its id."
        ));
    }
    crate::mcp_http::parse_endpoint(address)
        .map_err(|e| format!("{where_}: {table} `address` {address:?} is not a server address reflow2 can reach: {e:#}"))?;
    Ok(())
}

/// What the server at a pointer's address said about which design it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identity {
    /// It holds the design the pointer names.
    Confirmed,
    /// It holds a different design, with this id.
    Different(String),
    /// It refused (no key, a wrong one, or no such design for this caller).
    Refused(String),
    /// It could not be asked (down, unreachable, or it answered with nothing
    /// readable). Not proof of anything about the design.
    Unconfirmed(String),
}

/// Ask the server at the pointer's address which design it holds.
///
/// `design_identity` with no arguments, over its own short MCP session. It is
/// the one tool that says which design a server holds; it is marked as a write
/// because it can rename, so a server that logs writes (flo2.io) records one
/// read-shaped call per session start. A read-only identity call would remove
/// that and is recorded as a follow-up.
pub async fn identity_at(pointer: &Pointer, bearer: Option<&str>) -> Identity {
    use crate::mcp_http::{PROBE_TIMEOUT, ServerAnswered, post_with};
    let address = pointer.design.address.as_str();
    let classify = |e: anyhow::Error| match e.downcast_ref::<ServerAnswered>() {
        Some(a) if matches!(a.status, 401 | 403 | 404) => Identity::Refused(a.to_string()),
        _ => Identity::Unconfirmed(format!("{e:#}")),
    };
    let hello = json!({
        "jsonrpc": "2.0", "id": 0, "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "reflow2-pointer", "version": env!("CARGO_PKG_VERSION")}
        }
    })
    .to_string();
    let session = match post_with(address, None, hello, PROBE_TIMEOUT, bearer).await {
        Ok((_, s)) => s,
        Err(e) => return classify(e),
    };
    let _ = post_with(
        address,
        session.as_deref(),
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.to_string(),
        PROBE_TIMEOUT,
        bearer,
    )
    .await;
    let call = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "design_identity", "arguments": {}}
    })
    .to_string();
    let messages = match post_with(address, session.as_deref(), call, PROBE_TIMEOUT, bearer).await {
        Ok((m, _)) => m,
        Err(e) => return classify(e),
    };
    match graph_id_in(&messages) {
        Some(id) if id == pointer.design.id => Identity::Confirmed,
        Some(id) => Identity::Different(id),
        None => Identity::Unconfirmed(format!(
            "the server at {address} answered design_identity without naming a design"
        )),
    }
}

/// The `graph_id` in the reply to request 1, from structured content or text.
fn graph_id_in(messages: &[String]) -> Option<String> {
    let reply = messages
        .iter()
        .filter_map(|m| serde_json::from_str::<Value>(m).ok())
        .find(|v| v.get("id") == Some(&json!(1)))?;
    let result = reply.get("result")?;
    if result.get("isError") == Some(&json!(true)) {
        return None;
    }
    let from_structured = result
        .pointer("/structuredContent/graph_id")
        .and_then(Value::as_str)
        .map(str::to_string);
    from_structured.or_else(|| {
        result
            .pointer("/content/0/text")
            .and_then(Value::as_str)
            .and_then(|t| serde_json::from_str::<Value>(t).ok())
            .and_then(|v| {
                v.get("graph_id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
    })
}

/// What the agent is told at the handshake, before the server's own words.
pub fn notice(pointer: &Pointer, local_store: bool, identity: &Identity) -> String {
    let mut s = format!(
        "📍 THIS FOLDER NAMES ITS DESIGN. {} says this folder implements design {} at {}, and \
         this session is attached to that design.",
        FILE, pointer.design.id, pointer.design.address
    );
    if let Identity::Unconfirmed(why) = identity {
        s.push_str(&format!(
            " ⚠️ Which design that server holds could not be confirmed when this session started \
             ({why}); if calls fail, that is why."
        ));
    }
    if local_store {
        s.push_str(
            " A design store also sits in this folder's .reflow2/ — it was NOT opened. It is not \
             the design this folder names; do not read it, copy it or write to it expecting it to \
             be current.",
        );
    }
    if !pointer.also.is_empty() {
        let list: Vec<String> = pointer
            .also
            .iter()
            .map(|a| match &a.role {
                Some(r) => format!("{} ({r}) at {}", a.id, a.address),
                None => format!("{} at {}", a.id, a.address),
            })
            .collect();
        s.push_str(&format!(
            " The folder also names {} — NOT opened in this session; each can be added as its own \
             MCP connection.",
            list.join("; ")
        ));
    }
    s
}

/// Why a session attached to nothing rather than to the wrong design.
pub fn refusal(pointer: &Pointer, identity: &Identity) -> Option<String> {
    match identity {
        Identity::Different(held) => Some(format!(
            "{} says this folder implements design {}, but the server at {} holds design {held}. \
             Nothing was attached and nothing local was opened in its place. Correct the `id` or \
             the `address` in {} (`design_identity` on the design you mean says its id).",
            pointer.path.display(),
            pointer.design.id,
            pointer.design.address,
            FILE
        )),
        Identity::Refused(why) => Some(format!(
            "{} names design {} at {}, and that server refused this machine: {why}. Nothing was \
             attached and nothing local was opened in its place. If this machine has no key for \
             it, `reflow2-mcp setup remote <server>` stores one; then start the session again.",
            pointer.path.display(),
            pointer.design.id,
            pointer.design.address
        )),
        Identity::Confirmed | Identity::Unconfirmed(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
# the design this repo implements
[design]
id      = "0dcdeca97f8810ce"
address = "https://api.flo2.io/g/0dcdeca97f8810ce/mcp"

[[also]]
id      = "1a97fc1f1de4d5d2"
address = "https://api.flo2.io/g/1a97fc1f1de4d5d2/mcp"
role    = "consumes"
"#;

    fn at() -> PathBuf {
        PathBuf::from("/repo/.reflow2.toml")
    }

    #[test]
    fn a_pointer_names_its_design_and_the_designs_beside_it() {
        let p = parse(GOOD, &at()).expect("a good pointer parses");
        assert_eq!(p.design.id, "0dcdeca97f8810ce");
        assert_eq!(
            p.design.address,
            "https://api.flo2.io/g/0dcdeca97f8810ce/mcp"
        );
        assert_eq!(p.also.len(), 1);
        assert_eq!(p.also[0].role.as_deref(), Some("consumes"));
    }

    #[test]
    fn it_sits_beside_the_folders_reflow2_directory() {
        let root = std::env::temp_dir().join(format!("reflow2-pointer-loc-{}", std::process::id()));
        std::fs::create_dir_all(root.join(".reflow2")).unwrap();
        let graph = root.join(".reflow2").join("graph");
        let want = std::fs::canonicalize(&root).unwrap().join(FILE);
        assert_eq!(location_for(graph.to_str().unwrap()), want);
    }

    #[test]
    fn no_file_is_no_pointer_and_the_folder_names_its_design_by_path_as_before() {
        assert_eq!(
            read(Path::new("/definitely/not/here/.reflow2.toml")),
            Ok(None)
        );
    }

    #[test]
    fn a_pointer_that_cannot_be_used_is_refused_by_name_never_skipped() {
        for (text, says) in [
            ("[design]\naddress = \"https://x.example/g/a/mcp\"\n", "id"),
            ("[design]\nid = \"a\"\n", "address"),
            (
                "[design]\nid = \"a b\"\naddress = \"https://x.example/\"\n",
                "graph id",
            ),
            (
                "[design]\nid = \"a\"\naddress = \"not a url\"\n",
                "not a server address",
            ),
            (
                "[design]\nid = \"a\"\naddress = \"https://x.example/\"\nkey = \"flo2_abc\"\n",
                "NO secret",
            ),
            ("design = 3\n", "not a pointer"),
            (
                "[design]\nid = \"a\"\naddress = \"https://x.example/\"\n[[also]]\nid = \"\"\naddress = \"https://x.example/\"\n",
                "[[also]]",
            ),
        ] {
            let err = parse(text, &at()).expect_err(text);
            assert!(err.contains(says), "{text:?} → {err}");
            assert!(err.contains(".reflow2.toml"), "the file is named: {err}");
        }
    }

    #[test]
    fn the_notice_says_which_design_and_that_the_local_store_was_left_alone() {
        let p = parse(GOOD, &at()).unwrap();
        let n = notice(&p, true, &Identity::Confirmed);
        assert!(
            n.contains("0dcdeca97f8810ce") && n.contains("api.flo2.io"),
            "{n}"
        );
        assert!(n.contains("was NOT opened"), "{n}");
        assert!(
            n.contains("1a97fc1f1de4d5d2 (consumes)"),
            "the other design is listed: {n}"
        );
        let quiet = notice(
            &parse(
                "[design]\nid = \"a\"\naddress = \"https://x.example/\"\n",
                &at(),
            )
            .unwrap(),
            false,
            &Identity::Confirmed,
        );
        assert!(
            !quiet.contains("NOT opened") && !quiet.contains("also names"),
            "{quiet}"
        );
        let unsure = notice(&p, false, &Identity::Unconfirmed("down".into()));
        assert!(unsure.contains("could not be confirmed"), "{unsure}");
    }

    #[test]
    fn a_different_design_or_a_refusal_attaches_nothing_and_says_why() {
        let p = parse(GOOD, &at()).unwrap();
        let wrong = refusal(&p, &Identity::Different("ffff".into())).expect("refused");
        assert!(
            wrong.contains("holds design ffff") && wrong.contains("nothing local was opened"),
            "{wrong}"
        );
        let no_key = refusal(&p, &Identity::Refused("401".into())).expect("refused");
        assert!(no_key.contains("reflow2-mcp setup remote"), "{no_key}");
        assert_eq!(refusal(&p, &Identity::Confirmed), None);
        assert_eq!(
            refusal(&p, &Identity::Unconfirmed("down".into())),
            None,
            "down is not wrong"
        );
    }

    #[test]
    fn the_graph_id_is_read_from_structured_content_or_from_text() {
        let structured = vec![json!({"jsonrpc":"2.0","id":1,"result":{"structuredContent":{"graph_id":"g1"},"content":[]}}).to_string()];
        assert_eq!(graph_id_in(&structured).as_deref(), Some("g1"));
        let text = vec![json!({"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"{\"graph_id\":\"g2\"}"}]}}).to_string()];
        assert_eq!(graph_id_in(&text).as_deref(), Some("g2"));
        let error = vec![json!({"jsonrpc":"2.0","id":1,"result":{"isError":true,"content":[{"type":"text","text":"{\"graph_id\":\"g3\"}"}]}}).to_string()];
        assert_eq!(graph_id_in(&error), None, "an error reply names no design");
    }
}
