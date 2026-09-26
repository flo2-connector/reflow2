//! One setup command chooses, once, whether new designs live locally or on a
//! remote server, and where that server's key is.
//!
//! req:one-setup-command-chooses-local-or-remote-once. These drive the real
//! binary with its settings in a temporary folder (REFLOW2_CONFIG_DIR) and the
//! key in an environment variable (`--no-keychain`), so no test ever touches a
//! real keychain. The keychain path itself is covered in-process by
//! client_setup's unit tests against a stand-in store, and was round-tripped by
//! hand on a Linux desktop keyring on 2026-09-26.
//!
//! What must hold:
//! · with nothing set up, reflow2 is local;
//! · a remote is saved only after the server has answered the key — a refused
//!   key or a plain-http key to another machine saves nothing;
//! · the settings file names where the key is, never the key;
//! · `--remote` then carries the key for THAT server, and no other server gets it;
//! · going local keeps the setting's server; forget removes the key's source.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};

const KEY: &str = "flo2_setup_test_do_not_print";

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

/// A stand-in server: every request answered with `status`; a 200 answers
/// `list_my_designs` with three designs. Records each request's Authorization.
fn stand_in(status: u16) -> (String, Arc<Mutex<Vec<Option<String>>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&seen);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let (mut len, mut auth) = (0usize, None);
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line.trim().is_empty() {
                    break;
                }
                let (k, v) = line.split_once(':').unwrap_or(("", ""));
                match k.trim().to_ascii_lowercase().as_str() {
                    "content-length" => len = v.trim().parse().unwrap_or(0),
                    "authorization" => auth = Some(v.trim().to_string()),
                    _ => {}
                }
            }
            let mut body = vec![0; len];
            reader.read_exact(&mut body).unwrap();
            record.lock().unwrap().push(auth);
            let msg: serde_json::Value = serde_json::from_slice(&body).unwrap();
            let reply = match (status, msg.get("id")) {
                (200, Some(id)) if msg["method"] == "tools/call" => serde_json::json!({"jsonrpc": "2.0", "id": id,
                    "result": {"structuredContent": {"your_designs": [{}, {}, {}]}}})
                .to_string(),
                (200, Some(id)) => serde_json::json!({"jsonrpc": "2.0", "id": id, "result": {"serverInfo": {"name": "stand-in"}}}).to_string(),
                _ => String::new(),
            };
            let code = if status == 200 && msg.get("id").is_none() {
                202
            } else {
                status
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {code} X\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
                reply.len()
            );
        }
    });
    (origin, seen)
}

fn run(dir: &Path, args: &[&str], key_in_env: bool, stdin: &str) -> Output {
    let mut cmd = Command::new(bin());
    cmd.args(args)
        .env("REFLOW2_CONFIG_DIR", dir)
        .env_remove("SETUP_TEST_KEY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if key_in_env {
        cmd.env("SETUP_TEST_KEY", KEY);
    }
    let mut child = cmd.spawn().expect("the binary runs");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

const HELLO: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#;

fn setup_env_key(dir: &Path, origin: &str) -> Output {
    run(
        dir,
        &[
            "setup",
            "remote",
            origin,
            "--api-key-env",
            "SETUP_TEST_KEY",
            "--no-keychain",
        ],
        true,
        "",
    )
}

#[test]
fn with_nothing_set_up_reflow2_is_local() {
    let dir = tempfile::tempdir().unwrap();
    let o = run(dir.path(), &["setup"], false, "");
    assert!(o.status.success(), "{}", text(&o));
    assert!(text(&o).contains("reflow2 client: local"), "{}", text(&o));
}

#[test]
fn a_remote_is_saved_after_the_server_answers_and_the_file_names_the_variable_not_the_key() {
    let (origin, seen) = stand_in(200);
    let dir = tempfile::tempdir().unwrap();
    let o = setup_env_key(dir.path(), &origin);
    assert!(o.status.success(), "{}", text(&o));
    let said = text(&o);
    assert!(said.contains("Connected: 3 designs"), "{said}");
    assert!(said.contains("read from $SETUP_TEST_KEY"), "{said}");
    assert!(!said.contains(KEY), "the key was printed: {said}");

    let file = std::fs::read_to_string(dir.path().join("client.json")).unwrap();
    assert!(
        file.contains("SETUP_TEST_KEY") && !file.contains(KEY),
        "{file}"
    );
    let seen = seen.lock().unwrap().clone();
    assert!(
        !seen.is_empty()
            && seen
                .iter()
                .all(|a| a.as_deref() == Some(&format!("Bearer {KEY}")[..])),
        "{seen:?}"
    );

    let shown = run(dir.path(), &["setup"], true, "");
    assert!(
        text(&shown).contains(&format!(
            "reflow2 client: remote — new designs go to {origin}"
        )),
        "{}",
        text(&shown)
    );
}

#[test]
fn remote_mode_carries_the_key_to_its_own_server_and_to_no_other() {
    let (origin, seen) = stand_in(200);
    let (other, other_seen) = stand_in(200);
    let dir = tempfile::tempdir().unwrap();
    assert!(setup_env_key(dir.path(), &origin).status.success());
    seen.lock().unwrap().clear();

    let o = run(
        dir.path(),
        &["--remote", &format!("{origin}/g/abc/mcp")],
        true,
        &format!("{HELLO}\n"),
    );
    assert!(o.status.success(), "{}", text(&o));
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        &[Some(format!("Bearer {KEY}"))]
    );

    let o = run(
        dir.path(),
        &["--remote", &format!("{other}/g/abc/mcp")],
        true,
        &format!("{HELLO}\n"),
    );
    assert!(o.status.success(), "{}", text(&o));
    assert_eq!(
        other_seen.lock().unwrap().as_slice(),
        &[None],
        "another server gets no key"
    );
}

#[test]
fn a_refused_key_saves_nothing() {
    let (origin, _) = stand_in(401);
    let dir = tempfile::tempdir().unwrap();
    let o = setup_env_key(dir.path(), &origin);
    assert!(!o.status.success());
    let said = text(&o);
    assert!(
        said.contains("401") && said.contains("Nothing was saved"),
        "{said}"
    );
    assert!(!said.contains(KEY));
    assert!(!dir.path().join("client.json").exists());
}

#[test]
fn a_key_over_plain_http_to_another_machine_is_refused_and_nothing_saved() {
    let dir = tempfile::tempdir().unwrap();
    let o = setup_env_key(dir.path(), "http://reflow2.example.org");
    assert!(!o.status.success());
    assert!(text(&o).contains("https"), "{}", text(&o));
    assert!(!dir.path().join("client.json").exists());
}

#[test]
fn a_server_that_takes_no_key_gets_no_header() {
    let (origin, seen) = stand_in(200);
    let dir = tempfile::tempdir().unwrap();
    let o = run(
        dir.path(),
        &["setup", "remote", &origin, "--no-key"],
        false,
        "",
    );
    assert!(o.status.success(), "{}", text(&o));
    let o = run(
        dir.path(),
        &["--remote", &format!("{origin}/g/abc/mcp")],
        false,
        &format!("{HELLO}\n"),
    );
    assert!(o.status.success(), "{}", text(&o));
    assert!(seen.lock().unwrap().iter().all(Option::is_none));
}

#[test]
fn going_local_moves_nothing_and_forget_drops_the_keys_source() {
    let (origin, _) = stand_in(200);
    let dir = tempfile::tempdir().unwrap();
    assert!(setup_env_key(dir.path(), &origin).status.success());

    let o = run(dir.path(), &["setup", "local"], false, "");
    assert!(o.status.success(), "{}", text(&o));
    assert!(text(&o).contains("No design was moved"));
    assert!(text(&run(dir.path(), &["setup"], false, "")).contains("reflow2 client: local"));

    let o = run(dir.path(), &["setup", "forget"], false, "");
    assert!(o.status.success(), "{}", text(&o));
    assert!(
        text(&o).contains("no longer reads $SETUP_TEST_KEY"),
        "{}",
        text(&o)
    );
    let file = std::fs::read_to_string(dir.path().join("client.json")).unwrap();
    assert!(!file.contains("SETUP_TEST_KEY"), "{file}");
}

#[test]
fn an_empty_piped_key_is_refused() {
    let (origin, _) = stand_in(200);
    let dir = tempfile::tempdir().unwrap();
    let o = run(dir.path(), &["setup", "remote", &origin], false, "\n");
    assert!(!o.status.success());
    assert!(text(&o).contains("--no-key"), "{}", text(&o));
    assert!(!dir.path().join("client.json").exists());
}
