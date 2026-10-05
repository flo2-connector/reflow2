//! A spawned shared daemon inherits nothing it should not.
//!
//! FOUND 2026-10-03
//! (fact:a-spawned-shared-daemon-keeps-every-file-descriptor-its-parent-left-open-2026-10-03),
//! on the shared build box. Builds ran as `flock <lock> cargo test …`; two
//! `--shared` clients from a failed test run had started `--serve-shared`
//! daemons, and `lsof` showed both daemons holding the lock file on fd 3. The
//! descriptor came from `flock(1)` (which opens it without close-on-exec)
//! through cargo, the test binary and the client, and the daemons kept it for
//! about fifteen minutes, until they were killed by hand. Every other guarded
//! build on the machine waited that long.
//!
//! THE CAUSE: `spawn_daemon` set the daemon's stdin, stdout and stderr and its
//! process group, and nothing else. `std::process::Command` closes nothing: it
//! relies on every descriptor being opened close-on-exec, which is true of the
//! ones Rust opens and false of the ones a process INHERITED from whoever
//! launched it. So a daemon meant to outlive its session kept every lock, pipe,
//! socket and file its launcher happened to leave open, for its whole life
//! (`--idle-timeout`, two hours by default). It also stayed in its launcher's
//! session, with its launcher's controlling terminal.
//!
//! THE CLASS IS WHAT A DAEMON KEEPS OF THE PROCESS THAT STARTED IT. A process
//! meant to outlive its parent must take nothing from it but what it was handed
//! on purpose, and leave nothing behind for it. These tests check the three
//! things a launcher can leak into a daemon (descriptors, a lock held through
//! one, the session and terminal) and the one a daemon can leak back into its
//! launcher: the exit status nobody collects, which is a zombie
//! (fact:a-long-lived-shared-proxy-leaks-zombie-children-42-in-seven-days, 42
//! of them under one seven-day-old client).
//!
//! Linux only: `/proc` is the instrument. The fix covers every Unix reflow2
//! ships for; macOS runs the same code with a different way of finding the
//! descriptors, and it is not measured here.
#![cfg(target_os = "linux")]

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_reflow2-mcp")
}

/// A scratch project with a design in it, and a HOME of its own.
struct Rig {
    _dir: tempfile::TempDir,
    root: PathBuf,
    graph: PathBuf,
    home: PathBuf,
}

fn isolate(cmd: &mut Command, home: &Path) {
    cmd.env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("REFLOW2_CONFIG_DIR", home.join("reflow2-config"))
        .env("RUST_LOG", "error")
        .env_remove("REFLOW2_CONTENT_POLICY")
        .env_remove("REFLOW2_TRUSTED_GATEWAY")
        .env_remove("REFLOW2_OIDC_ISSUER")
        .env_remove("REFLOW2_CONTRIBUTOR_ID")
        .env_remove("REFLOW2_CONTRIBUTOR_MAP");
}

fn rig(tag: &str) -> Rig {
    let dir = tempfile::Builder::new()
        .prefix(&format!("reflow2-detach-{tag}-"))
        .tempdir()
        .expect("tempdir");
    let root = dir.path().join("project");
    let home = dir.path().join("home");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&home).unwrap();
    let graph = root.join(".reflow2/graph");
    let r = Rig {
        _dir: dir,
        root,
        graph,
        home,
    };
    let mut cmd = Command::new(bin());
    cmd.current_dir(&r.root).args([
        "--graph-path",
        r.graph.to_str().unwrap(),
        "--call",
        "add_project",
        "--args",
        r#"{"id":"proj:detach","name":"Detach"}"#,
    ]);
    isolate(&mut cmd, &r.home);
    let o = cmd.output().expect("the binary runs");
    assert!(
        o.status.success(),
        "seeding the design failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    r
}

/// Is `pid` a process that has not exited? A zombie has exited: it is only an
/// exit status waiting for its parent.
fn running(pid: u32) -> bool {
    matches!(stat(pid), Some(s) if s.state != 'Z')
}

/// Stops every daemon a test started, even when the test fails part-way: a
/// daemon left behind holds the store for two hours, and these tests are about
/// daemons that hold things they should not.
struct StopsDaemon {
    graph: PathBuf,
    home: PathBuf,
    pids: Vec<u32>,
}

impl StopsDaemon {
    fn new(r: &Rig) -> Self {
        Self {
            graph: r.graph.clone(),
            home: r.home.clone(),
            pids: Vec::new(),
        }
    }
}

impl Drop for StopsDaemon {
    fn drop(&mut self) {
        if let Some(pid) = rendezvous_pid(&self.graph) {
            self.pids.push(pid);
        }
        let mut cmd = Command::new(bin());
        cmd.args([
            "--graph-path",
            self.graph.to_str().unwrap(),
            "--stop-shared",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
        isolate(&mut cmd, &self.home);
        let _ = cmd.status();
        let start = Instant::now();
        while self.pids.iter().any(|p| running(*p)) && start.elapsed() < Duration::from_secs(15) {
            std::thread::sleep(Duration::from_millis(50));
        }
        for pid in &self.pids {
            if running(*pid) {
                let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
            }
        }
    }
}

/// A `--shared` client, launched through `launcher` (a command that ends by
/// running the client), driven over stdio, and stopped when dropped.
struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    stderr: Receiver<String>,
}

impl Drop for Session {
    fn drop(&mut self) {
        drop(self.stdin.take());
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Session {
    fn start(r: &Rig, launcher: &[&str]) -> Session {
        let (program, before) = launcher.split_first().expect("a launcher");
        let mut cmd = Command::new(program);
        cmd.current_dir(&r.root).args(before).arg(bin()).args([
            "--graph-path",
            r.graph.to_str().unwrap(),
            "--shared",
        ]);
        isolate(&mut cmd, &r.home);
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("could not start the client through {program}: {e}"));
        let stdin = child.stdin.take();
        let err = child.stderr.take().unwrap();
        let (tx, stderr) = channel::<String>();
        std::thread::spawn(move || {
            for line in BufReader::new(err).lines().map_while(Result::ok) {
                let _ = tx.send(line);
            }
        });
        Session {
            child,
            stdin,
            stderr,
        }
    }

    /// Wait for the client to say it is sharing a server, and return that
    /// server's pid — the daemon this client spawned.
    fn daemon(&self, r: &Rig, guard: &mut StopsDaemon) -> u32 {
        let start = Instant::now();
        let mut said = Vec::new();
        while start.elapsed() < Duration::from_secs(120) {
            match self.stderr.recv_timeout(Duration::from_millis(200)) {
                Ok(line) => {
                    let up = line.contains("reflow2: sharing the design at");
                    said.push(line);
                    if up {
                        let pid = rendezvous_pid(&r.graph)
                            .expect("a client that is sharing a server has a rendezvous to read");
                        guard.pids.push(pid);
                        return pid;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        panic!(
            "the --shared client never said it was sharing a server; it said:\n{}",
            said.join("\n")
        );
    }
}

fn rendezvous_pid(graph: &Path) -> Option<u32> {
    let raw =
        std::fs::read_to_string(reflow2_mcp::shared::rendezvous_path(graph.to_str()?)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    v["pid"].as_u64().map(|p| p as u32)
}

/// The fields of `/proc/<pid>/stat` these tests read.
struct Stat {
    state: char,
    ppid: u32,
    pgrp: u32,
    session: u32,
    tty_nr: i64,
}

fn stat(pid: u32) -> Option<Stat> {
    let raw = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The command name is in parentheses and may hold spaces; every field
    // after the LAST `)` is plain.
    let rest = &raw[raw.rfind(')')? + 1..];
    let f: Vec<&str> = rest.split_whitespace().collect();
    Some(Stat {
        state: f.first()?.chars().next()?,
        ppid: f.get(1)?.parse().ok()?,
        pgrp: f.get(2)?.parse().ok()?,
        session: f.get(3)?.parse().ok()?,
        tty_nr: f.get(4)?.parse().ok()?,
    })
}

/// Every descriptor `pid` holds, as `fd -> what it points at`.
fn descriptors(pid: u32) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(format!("/proc/{pid}/fd"))
        .unwrap_or_else(|e| panic!("cannot read /proc/{pid}/fd: {e}"))
        .flatten()
        .map(|e| {
            let target = std::fs::read_link(e.path())
                .map(|t| t.to_string_lossy().into_owned())
                .unwrap_or_default();
            (e.file_name().to_string_lossy().into_owned(), target)
        })
        .collect();
    out.sort();
    out
}

fn holds(pid: u32, file: &Path) -> Vec<String> {
    let file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let want = file.to_string_lossy();
    descriptors(pid)
        .into_iter()
        .filter(|(_, t)| t == &*want)
        .map(|(fd, _)| fd)
        .collect()
}

/// THE CLASS: a descriptor the launcher left open, not close-on-exec, reaches
/// the client (that is the launcher's business) and must NOT reach the daemon
/// the client starts. `sh`'s `exec 7>>file` is the plainest way to hold one: it
/// opens fd 7 without close-on-exec and keeps it across the `exec`.
#[test]
fn a_daemon_holds_no_descriptor_its_launcher_left_open() {
    let r = rig("fd");
    let marker = r.root.join("held-by-the-launcher");
    std::fs::write(&marker, b"").unwrap();
    let mut guard = StopsDaemon::new(&r);
    let script = r#"exec 7>>"$0"; exec "$@""#;
    let session = Session::start(&r, &["sh", "-c", script, marker.to_str().unwrap()]);
    let daemon = session.daemon(&r, &mut guard);

    // The instrument works: the client holds it, because its launcher passed it.
    assert_eq!(
        holds(session.child.id(), &marker),
        vec!["7".to_string()],
        "the client should hold the launcher's fd 7 (the test's own premise): {:?}",
        descriptors(session.child.id())
    );
    let held = holds(daemon, &marker);
    assert!(
        held.is_empty(),
        "the daemon (pid {daemon}) holds its launcher's file on fd {held:?}; every descriptor it \
         holds: {:?}",
        descriptors(daemon)
    );
}

/// THE MEASURED INSTANCE: a client run under `flock(1)`, as the build box ran
/// every test. Once the client and `flock` have gone, the lock is free, even
/// though the daemon the client started is still running.
#[test]
fn a_lock_its_launcher_held_is_free_once_the_launcher_has_gone() {
    let r = rig("flock");
    let lock = r.root.join("heavy-build.lock");
    std::fs::write(&lock, b"").unwrap();
    let mut guard = StopsDaemon::new(&r);
    let mut session = Session::start(&r, &["flock", lock.to_str().unwrap()]);
    let daemon = session.daemon(&r, &mut guard);

    // The client is flock's child. Closing its stdin ends the MCP session, the
    // client exits, and flock exits with it, releasing its own descriptor.
    drop(session.stdin.take());
    let start = Instant::now();
    while session.child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(30) {
            panic!("the client under flock did not exit when its stdin closed");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        running(daemon),
        "the daemon outlives the session that started it (that is what it is for)"
    );

    let free = Command::new("flock")
        .args(["-n", lock.to_str().unwrap(), "true"])
        .status()
        .expect("flock(1) runs (util-linux)");
    assert!(
        free.success(),
        "the lock flock(1) held for the client is still held after the client and flock exited: \
         the daemon (pid {daemon}) holds it on fd {:?}; every descriptor it holds: {:?}",
        holds(daemon, &lock),
        descriptors(daemon)
    );
}

/// A daemon leads a session of its own, with no controlling terminal: a
/// terminal that closes does not take it down, and it cannot take the terminal.
#[test]
fn a_daemon_leads_its_own_session_with_no_terminal() {
    let r = rig("session");
    let mut guard = StopsDaemon::new(&r);
    let session = Session::start(&r, &["env"]);
    let daemon = session.daemon(&r, &mut guard);
    let s = stat(daemon).expect("the daemon is running");
    let client = stat(session.child.id()).expect("the client is running");
    assert_eq!(
        s.session, daemon,
        "the daemon leads its own session (its launcher's session is {})",
        client.session
    );
    assert_eq!(s.pgrp, daemon, "the daemon leads its own process group");
    assert_eq!(s.tty_nr, 0, "the daemon has no controlling terminal");
}

/// A daemon that exits while the client that started it is still running is
/// collected, not left as a zombie under that client for as long as it lives.
#[test]
fn a_daemon_that_exits_is_not_left_a_zombie_of_the_client_that_started_it() {
    let r = rig("zombie");
    let mut guard = StopsDaemon::new(&r);
    let session = Session::start(&r, &["env"]);
    let daemon = session.daemon(&r, &mut guard);
    assert_eq!(
        stat(daemon).map(|s| s.ppid),
        Some(session.child.id()),
        "the client started this daemon, so it is the client's child"
    );

    let mut cmd = Command::new(bin());
    cmd.args(["--graph-path", r.graph.to_str().unwrap(), "--stop-shared"]);
    isolate(&mut cmd, &r.home);
    let o = cmd.output().expect("the binary runs");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));

    let start = Instant::now();
    while running(daemon) {
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "the daemon did not stop when asked"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    // It has exited. Collected means /proc no longer has it.
    let start = Instant::now();
    while stat(daemon).is_some() && start.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(50));
    }
    let left = stat(daemon);
    assert!(
        left.is_none(),
        "the daemon (pid {daemon}) exited and is still in the process table in state {:?}, a \
         zombie of the client (pid {}) that started it and never collected its exit status",
        left.map(|s| s.state),
        session.child.id()
    );
    assert!(
        session.child.id() > 0 && stat(session.child.id()).is_some(),
        "the client was alive throughout, so nobody else could have collected it"
    );
}
