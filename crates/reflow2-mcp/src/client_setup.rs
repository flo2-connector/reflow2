//! `reflow2-mcp setup` — req:one-setup-command-chooses-local-or-remote-once.
//!
//! One setting, chosen once per person, not per folder: whether NEW designs live
//! on this machine (`local`, the default) or on a remote reflow2 server, and the
//! key that server takes. Settled 2026-09-26 with its terms
//! (dec:idea-one-command-switches-reflow2-between-local-and-remote):
//!
//! · A folder that already holds a design follows THAT design, never this
//!   setting — its local store, or (later) its pointer file. The setting only
//!   decides for a folder with no design yet.
//! · Switching moves no design. Moving one is the deliberate import.
//! · The key is never a literal on the command line. It is typed at a prompt
//!   that does not echo, piped in, or read from a variable the person NAMES —
//!   then kept in the OS keychain, or, where there is none, read from that
//!   variable at each start.
//! · Local needs no network, no account and no sign-in
//!   (req:reflow2-keeps-working-entirely-on-one-machine).
//!
//! WHAT READS IT TODAY: `--remote <url>` with no key flag carries the key set up
//! for that url's server, and ONLY that server's — a key is never sent to a host
//! it was not given for. The empty-folder behaviour (a folder with no design
//! following the setting) arrives with the pointer file and /genesis.
//!
//! The settings file holds no secret: the mode, the server's origin, and WHERE
//! the key is (`keychain`, or the name of a variable). Browser sign-in is a
//! later step; the shape leaves room for it as another `KeySource`.

use crate::mcp_http::{PROBE_TIMEOUT, ServerAnswered, parse_endpoint, post_with};
use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Where new designs go.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Local,
    Remote,
}

/// Where the remote server's key is. Never the key itself.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "from", rename_all = "lowercase")]
pub enum KeySource {
    /// The server takes no key (a private network, a VPN).
    #[default]
    None,
    /// In the OS keychain, under service `reflow2` and the server's origin.
    Keychain,
    /// Read from this environment variable at each start: for a machine with
    /// no keychain. Only the variable's NAME is stored.
    Env { var: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientConfig {
    pub mode: Mode,
    /// The remote server's origin, e.g. `https://api.flo2.io`. Kept when the
    /// mode goes back to local, so the key stays tied to the server it is for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    #[serde(default)]
    pub key: KeySource,
}

const FILE: &str = "client.json";

/// The settings folder: `$REFLOW2_CONFIG_DIR` if set, else the platform's
/// per-user config folder (`$XDG_CONFIG_HOME/reflow2`, `~/.config/reflow2`,
/// `%APPDATA%\reflow2`).
pub fn config_dir() -> anyhow::Result<PathBuf> {
    if let Some(d) = std::env::var_os("REFLOW2_CONFIG_DIR").filter(|d| !d.is_empty()) {
        return Ok(PathBuf::from(d));
    }
    #[cfg(windows)]
    if let Some(d) = std::env::var_os("APPDATA").filter(|d| !d.is_empty()) {
        return Ok(PathBuf::from(d).join("reflow2"));
    }
    if let Some(d) = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|d| d.is_absolute())
    {
        return Ok(d.join("reflow2"));
    }
    let home = std::env::var_os("HOME").filter(|h| !h.is_empty()).context(
        "cannot find the settings folder: HOME is not set (set REFLOW2_CONFIG_DIR to choose one)",
    )?;
    Ok(PathBuf::from(home).join(".config").join("reflow2"))
}

/// The saved settings, or the default (local) when none were ever saved.
pub fn load(dir: &Path) -> anyhow::Result<ClientConfig> {
    let path = dir.join(FILE);
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).with_context(|| {
            format!(
                "{} is not a reflow2 client settings file; fix or delete it",
                path.display()
            )
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ClientConfig::default()),
        Err(e) => Err(e).with_context(|| format!("could not read {}", path.display())),
    }
}

/// Save atomically (write aside, then rename), readable only by its owner.
pub fn save(dir: &Path, cfg: &ClientConfig) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(dir).with_context(|| format!("could not create {}", dir.display()))?;
    let path = dir.join(FILE);
    let tmp = dir.join(format!(".{FILE}.{}", std::process::id()));
    std::fs::write(&tmp, serde_json::to_string_pretty(cfg)? + "\n")
        .with_context(|| format!("could not write {}", tmp.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&tmp, &path)
        .with_context(|| format!("could not replace {}", path.display()))?;
    Ok(path)
}

/// A server's origin as the setting stores it: scheme and host, lowercased,
/// with the port only when it is not the scheme's default. Whatever path the
/// person typed is dropped — which DESIGN a folder uses is not this setting.
pub fn origin_of(url: &str) -> anyhow::Result<String> {
    let ep = parse_endpoint(url)?;
    let scheme = if ep.tls { "https" } else { "http" };
    let default = if ep.tls { ":443" } else { ":80" };
    let authority = ep.connect.to_ascii_lowercase();
    Ok(format!(
        "{scheme}://{}",
        authority.strip_suffix(default).unwrap_or(&authority)
    ))
}

/// The OS keychain, or a stand-in for tests.
pub trait KeyStore {
    fn get(&self, server: &str) -> anyhow::Result<Option<String>>;
    fn set(&self, server: &str, key: &str) -> anyhow::Result<()>;
    /// True when there was a key to delete.
    fn delete(&self, server: &str) -> anyhow::Result<bool>;
}

/// macOS Keychain, Windows Credential Manager, or the Linux desktop's Secret
/// Service — whichever this platform has. Entries are service `reflow2`,
/// account = the server's origin.
pub struct OsKeychain;

const SERVICE: &str = "reflow2";

fn keychain_entry(server: &str) -> anyhow::Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, server).map_err(|e| {
        anyhow::anyhow!(
            "no keychain is available on this machine ({e}). Keep the key in an environment \
             variable instead and run: reflow2-mcp setup remote {server} --api-key-env FLO2_API_KEY --no-keychain"
        )
    })
}

impl KeyStore for OsKeychain {
    fn get(&self, server: &str) -> anyhow::Result<Option<String>> {
        match keychain_entry(server)?.get_password() {
            Ok(k) => Ok(Some(k)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow::anyhow!(
                "could not read the key for {server} from the keychain: {e}"
            )),
        }
    }
    fn set(&self, server: &str, key: &str) -> anyhow::Result<()> {
        keychain_entry(server)?.set_password(key).map_err(|e| {
            anyhow::anyhow!("could not store the key for {server} in the keychain: {e}")
        })
    }
    fn delete(&self, server: &str) -> anyhow::Result<bool> {
        match keychain_entry(server)?.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(anyhow::anyhow!(
                "could not remove the key for {server} from the keychain: {e}"
            )),
        }
    }
}

/// The key `--remote <url>` carries when no key flag was given: the one set up
/// for THAT url's server. Another server gets nothing — a key never travels to
/// a host it was not given for.
pub fn stored_credential(
    url: &str,
    cfg: &ClientConfig,
    store: &dyn KeyStore,
) -> anyhow::Result<Option<String>> {
    let Some(server) = cfg.server.as_deref() else {
        return Ok(None);
    };
    if origin_of(url)? != origin_of(server)? {
        return Ok(None);
    }
    match &cfg.key {
        KeySource::None => Ok(None),
        KeySource::Keychain => store.get(server)?.map(Some).ok_or_else(|| {
            anyhow::anyhow!(
                "the key for {server} is not in the keychain any more. Run `reflow2-mcp setup remote {server}` again."
            )
        }),
        KeySource::Env { var } => match std::env::var(var) {
            Ok(k) if !k.trim().is_empty() => Ok(Some(k.trim().to_string())),
            _ => bail!(
                "reflow2 was set up to read the key for {server} from ${var}, and ${var} is not set here."
            ),
        },
    }
}

/// What `setup` was asked to do.
pub enum Action {
    Show,
    Local,
    Remote {
        url: String,
        key: Option<String>,
        source: KeySource,
    },
    Forget,
}

/// What the server said when setup tried the key.
#[derive(Debug, PartialEq, Eq)]
pub enum Check {
    /// Answered at its account address; the number of designs when it says.
    Connected { designs: Option<usize> },
    /// Answered, but has no account address at /mcp (a plain reflow2 server):
    /// the key will be tried when a design is opened.
    NoAccountAddress,
}

/// Try the key against the server's account address (`<origin>/mcp`).
/// A refusal or no answer is an error: nothing gets saved on either.
pub async fn check_server(origin: &str, key: Option<&str>) -> anyhow::Result<Check> {
    let account = format!("{origin}/mcp");
    let hello = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "protocolVersion": "2025-06-18", "capabilities": {},
                    "clientInfo": { "name": "reflow2-mcp setup", "version": env!("CARGO_PKG_VERSION") } }
    })
    .to_string();
    let sid = match post_with(&account, None, hello, PROBE_TIMEOUT, key).await {
        Ok((_, sid)) => sid,
        Err(e) => {
            if let Some(answered) = e.downcast_ref::<ServerAnswered>() {
                return match answered.status {
                    404 | 405 => Ok(Check::NoAccountAddress),
                    _ => Err(e),
                };
            }
            return Err(e.context(format!("could not reach {origin}")));
        }
    };
    if sid.is_some() {
        let ready = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.to_string();
        let _ = post_with(&account, sid.as_deref(), ready, PROBE_TIMEOUT, key).await;
    }
    let list = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"list_my_designs","arguments":{}}}"#;
    let designs = post_with(
        &account,
        sid.as_deref(),
        list.to_string(),
        PROBE_TIMEOUT,
        key,
    )
    .await
    .ok()
    .and_then(|(messages, _)| messages.into_iter().find_map(|m| count_designs(&m)));
    Ok(Check::Connected { designs })
}

/// How many designs a `list_my_designs` reply names, if it is one.
fn count_designs(message: &str) -> Option<usize> {
    let v: serde_json::Value = serde_json::from_str(message).ok()?;
    let result = v.get("result")?;
    if result.get("isError").and_then(|e| e.as_bool()) == Some(true) {
        return None;
    }
    let payload = result.get("structuredContent").cloned().or_else(|| {
        let text = result.get("content")?.get(0)?.get("text")?.as_str()?;
        serde_json::from_str(text).ok()
    })?;
    payload.get("your_designs")?.as_array().map(|a| a.len())
}

fn describe_key(cfg: &ClientConfig, store: &dyn KeyStore) -> String {
    let Some(server) = cfg.server.as_deref() else {
        return "none".into();
    };
    match &cfg.key {
        KeySource::None => "none (the server takes no key)".into(),
        KeySource::Env { var } => format!("read from ${var} at each start (nothing stored)"),
        KeySource::Keychain => match store.get(server) {
            Ok(Some(_)) => "in the OS keychain".into(),
            Ok(None) => {
                format!("MISSING from the keychain — run `reflow2-mcp setup remote {server}` again")
            }
            Err(e) => format!("could not check the keychain: {e}"),
        },
    }
}

/// Run one `setup` action, reporting to `out`. The key never reaches `out`.
pub async fn run(
    action: Action,
    dir: &Path,
    store: &dyn KeyStore,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    let mut cfg = load(dir)?;
    let file = dir.join(FILE);
    match action {
        Action::Show => {
            match (cfg.mode, cfg.server.as_deref()) {
                (Mode::Local, _) => writeln!(
                    out,
                    "reflow2 client: local — designs live on this machine; no network, no sign-in."
                )?,
                (Mode::Remote, Some(server)) => {
                    writeln!(out, "reflow2 client: remote — new designs go to {server}")?;
                    writeln!(out, "  key: {}", describe_key(&cfg, store))?;
                }
                (Mode::Remote, None) => writeln!(
                    out,
                    "reflow2 client: remote, but no server is named — run `reflow2-mcp setup remote <url>`"
                )?,
            }
            writeln!(
                out,
                "  A folder that already holds a design follows that design, not this setting."
            )?;
            writeln!(out, "  settings: {}", file.display())?;
        }
        Action::Local => {
            cfg.mode = Mode::Local;
            save(dir, &cfg)?;
            writeln!(
                out,
                "reflow2 client: local — new designs live on this machine. No design was moved."
            )?;
            if let (Some(server), KeySource::Keychain) = (cfg.server.as_deref(), &cfg.key) {
                writeln!(
                    out,
                    "  Your key for {server} stays in the keychain; `reflow2-mcp setup forget` removes it."
                )?;
            }
        }
        Action::Remote { url, key, source } => {
            let origin = origin_of(&url)?;
            crate::proxy::check_remote_url(&origin, key.is_some())?;
            if !parse_endpoint(&url)?.path.trim_end_matches('/').is_empty() {
                writeln!(
                    out,
                    "Using the server {origin}. Which design a folder works on is chosen per folder, not here."
                )?;
            }
            let check = check_server(&origin, key.as_deref())
                .await
                .map_err(|e| anyhow::anyhow!("{e:#}\nNothing was saved."))?;
            // The key goes where the setting says it lives. Nothing else
            // keeps it: not the settings file, not the output.
            if source == KeySource::Keychain {
                let k = key.as_deref().context("a keychain key needs a key")?;
                store.set(&origin, k)?;
            } else if cfg.server.as_deref() == Some(origin.as_str())
                && cfg.key == KeySource::Keychain
            {
                // Moving this server off the keychain: do not leave the old key behind.
                let _ = store.delete(&origin);
            }
            cfg = ClientConfig {
                mode: Mode::Remote,
                server: Some(origin.clone()),
                key: source,
            };
            save(dir, &cfg)?;
            writeln!(
                out,
                "reflow2 client: remote — new designs go to {origin}. No design was moved."
            )?;
            match check {
                Check::Connected { designs: Some(n) } => writeln!(
                    out,
                    "  Connected: {n} design{} there.",
                    if n == 1 { "" } else { "s" }
                )?,
                Check::Connected { designs: None } => writeln!(out, "  Connected.")?,
                Check::NoAccountAddress => writeln!(
                    out,
                    "  The server answered but has no account address at /mcp; the key will be tried when a design is opened."
                )?,
            }
            writeln!(out, "  key: {}", describe_key(&cfg, store))?;
        }
        Action::Forget => {
            let Some(server) = cfg.server.clone() else {
                writeln!(
                    out,
                    "No remote server is set up, so there is no key to forget."
                )?;
                return Ok(());
            };
            let removed = match cfg.key {
                KeySource::Keychain => store.delete(&server)?,
                _ => false,
            };
            let was = std::mem::take(&mut cfg.key);
            save(dir, &cfg)?;
            match was {
                KeySource::Keychain if removed => {
                    writeln!(out, "Removed the key for {server} from the keychain.")?
                }
                KeySource::Env { var } => writeln!(
                    out,
                    "reflow2 no longer reads ${var} for {server}. The variable itself is yours to unset."
                )?,
                _ => writeln!(out, "No key was stored for {server}.")?,
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{BufRead, BufReader, Read};
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryKeys(Mutex<HashMap<String, String>>);
    impl KeyStore for MemoryKeys {
        fn get(&self, s: &str) -> anyhow::Result<Option<String>> {
            Ok(self.0.lock().unwrap().get(s).cloned())
        }
        fn set(&self, s: &str, k: &str) -> anyhow::Result<()> {
            self.0.lock().unwrap().insert(s.into(), k.into());
            Ok(())
        }
        fn delete(&self, s: &str) -> anyhow::Result<bool> {
            Ok(self.0.lock().unwrap().remove(s).is_some())
        }
    }

    /// A stand-in account address answering every request with `status`,
    /// and `list_my_designs` with `designs` designs. Returns its origin.
    fn account_server(status: u16, designs: usize) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut len = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line.trim().is_empty() {
                        break;
                    }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                }
                let mut body = vec![0; len];
                reader.read_exact(&mut body).unwrap();
                let msg: serde_json::Value = serde_json::from_slice(&body).unwrap();
                let reply = match (status, msg.get("id")) {
                    (200, Some(id)) if msg["method"] == "tools/call" => serde_json::json!({"jsonrpc": "2.0", "id": id,
                        "result": {"structuredContent": {"your_designs": vec![serde_json::json!({}); designs]}}})
                    .to_string(),
                    (200, Some(id)) => serde_json::json!({"jsonrpc": "2.0", "id": id, "result": {}}).to_string(),
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
        origin
    }

    #[test]
    fn an_origin_drops_the_path_the_default_port_and_the_case() {
        assert_eq!(
            origin_of("https://API.flo2.io/g/abc/mcp").unwrap(),
            "https://api.flo2.io"
        );
        assert_eq!(
            origin_of("https://api.flo2.io:443").unwrap(),
            "https://api.flo2.io"
        );
        assert_eq!(
            origin_of("http://127.0.0.1:8080/").unwrap(),
            "http://127.0.0.1:8080"
        );
        assert_eq!(
            origin_of("https://r.example.org:8443/x").unwrap(),
            "https://r.example.org:8443"
        );
    }

    #[test]
    fn no_settings_file_means_local() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()).unwrap(), ClientConfig::default());
        assert_eq!(ClientConfig::default().mode, Mode::Local);
    }

    #[test]
    fn a_key_goes_only_to_the_server_it_was_set_up_for() {
        let keys = MemoryKeys::default();
        keys.set("https://api.flo2.io", "flo2_k").unwrap();
        let cfg = ClientConfig {
            mode: Mode::Remote,
            server: Some("https://api.flo2.io".into()),
            key: KeySource::Keychain,
        };
        assert_eq!(
            stored_credential("https://api.flo2.io/g/abc/mcp", &cfg, &keys)
                .unwrap()
                .as_deref(),
            Some("flo2_k")
        );
        assert_eq!(
            stored_credential("https://evil.example/g/abc/mcp", &cfg, &keys).unwrap(),
            None
        );
        assert_eq!(
            stored_credential("http://api.flo2.io/g/abc/mcp", &cfg, &keys).unwrap(),
            None,
            "a different scheme is a different server"
        );
        keys.delete("https://api.flo2.io").unwrap();
        assert!(
            stored_credential("https://api.flo2.io/mcp", &cfg, &keys).is_err(),
            "a missing key is said, not skipped"
        );
    }

    #[tokio::test]
    async fn setting_up_a_remote_checks_the_key_then_keeps_it_in_the_keychain_only() {
        let origin = account_server(200, 4);
        let dir = tempfile::tempdir().unwrap();
        let keys = MemoryKeys::default();
        let mut out = Vec::new();
        run(
            Action::Remote {
                url: format!("{origin}/g/x/mcp"),
                key: Some("flo2_secret".into()),
                source: KeySource::Keychain,
            },
            dir.path(),
            &keys,
            &mut out,
        )
        .await
        .unwrap();
        let said = String::from_utf8(out).unwrap();
        assert!(said.contains("Connected: 4 designs"), "{said}");
        assert!(said.contains("in the OS keychain"), "{said}");
        assert!(!said.contains("flo2_secret"));
        assert_eq!(keys.get(&origin).unwrap().as_deref(), Some("flo2_secret"));
        let file = std::fs::read_to_string(dir.path().join(FILE)).unwrap();
        assert!(
            !file.contains("flo2_secret"),
            "the settings file holds no secret: {file}"
        );
        assert_eq!(
            load(dir.path()).unwrap(),
            ClientConfig {
                mode: Mode::Remote,
                server: Some(origin),
                key: KeySource::Keychain
            }
        );
    }

    #[tokio::test]
    async fn a_refused_key_is_not_saved_anywhere() {
        let origin = account_server(401, 0);
        let dir = tempfile::tempdir().unwrap();
        let keys = MemoryKeys::default();
        let err = run(
            Action::Remote {
                url: origin.clone(),
                key: Some("flo2_bad".into()),
                source: KeySource::Keychain,
            },
            dir.path(),
            &keys,
            &mut Vec::new(),
        )
        .await
        .unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("401") && msg.contains("Nothing was saved"),
            "{msg}"
        );
        assert!(!msg.contains("flo2_bad"));
        assert_eq!(keys.get(&origin).unwrap(), None);
        assert!(!dir.path().join(FILE).exists());
    }

    #[tokio::test]
    async fn a_server_with_no_account_address_is_saved_and_said_so() {
        let origin = account_server(404, 0);
        let dir = tempfile::tempdir().unwrap();
        let mut out = Vec::new();
        run(
            Action::Remote {
                url: origin,
                key: None,
                source: KeySource::None,
            },
            dir.path(),
            &MemoryKeys::default(),
            &mut out,
        )
        .await
        .unwrap();
        assert!(
            String::from_utf8(out)
                .unwrap()
                .contains("no account address")
        );
    }

    #[tokio::test]
    async fn going_local_keeps_the_key_and_forget_removes_it() {
        let origin = account_server(200, 1);
        let dir = tempfile::tempdir().unwrap();
        let keys = MemoryKeys::default();
        run(
            Action::Remote {
                url: origin.clone(),
                key: Some("flo2_k".into()),
                source: KeySource::Keychain,
            },
            dir.path(),
            &keys,
            &mut Vec::new(),
        )
        .await
        .unwrap();
        let mut out = Vec::new();
        run(Action::Local, dir.path(), &keys, &mut out)
            .await
            .unwrap();
        assert_eq!(load(dir.path()).unwrap().mode, Mode::Local);
        assert!(
            keys.get(&origin).unwrap().is_some(),
            "switching moves nothing, the key included"
        );
        assert!(String::from_utf8(out).unwrap().contains("setup forget"));

        let mut out = Vec::new();
        run(Action::Forget, dir.path(), &keys, &mut out)
            .await
            .unwrap();
        assert!(String::from_utf8(out).unwrap().contains("Removed the key"));
        assert_eq!(keys.get(&origin).unwrap(), None);
        assert_eq!(load(dir.path()).unwrap().key, KeySource::None);
    }

    #[tokio::test]
    async fn a_key_is_refused_over_plain_http_to_another_machine_before_anything_is_sent() {
        let dir = tempfile::tempdir().unwrap();
        let err = run(
            Action::Remote {
                url: "http://reflow2.example.org".into(),
                key: Some("k".into()),
                source: KeySource::Keychain,
            },
            dir.path(),
            &MemoryKeys::default(),
            &mut Vec::new(),
        )
        .await
        .unwrap_err();
        assert!(format!("{err:#}").contains("https"), "{err:#}");
        assert!(!dir.path().join(FILE).exists());
    }
}
