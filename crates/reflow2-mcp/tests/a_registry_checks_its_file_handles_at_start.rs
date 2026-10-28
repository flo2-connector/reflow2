//! A server holding many designs raises its open-file limit at start, warns
//! when the designs it may hold open could need more files than the limit
//! allows, keeps running, and reports the files open on `/readyz`.
//!
//! ⭐ WHY THIS EXISTS. On 2026-10-28 flo2.io's registry opened all 64 designs
//! and the last two failed with "Too many open files": 64 designs, 64 table
//! files a store, and Docker's soft limit of 1024 were three numbers nobody
//! compared (`fact:root-cause-a-registry-at-its-open-limit-runs-out-of-file-handles-2026-10-28`,
//! `req:a-server-holding-many-designs-starts-with-file-handles-for-all-it-may-open`,
//! `req:a-server-reports-its-open-file-headroom`). The arithmetic and the
//! wording are unit-tested in `file_handles`; this drives the real binary,
//! started under a limit the test chooses, because only the running server
//! shows that the check runs, where its words go, and that it does not refuse.
//!
//! Unix only: the limit is set in the child before it runs (`setrlimit`).
//! An empty registry root, so no store is opened and no RocksDB is needed.

#![cfg(unix)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn bin() -> PathBuf {
    let mut p = std::env::current_exe().expect("test binary has a path");
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("reflow2-mcp")
}

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-file-handles-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct Server {
    child: Child,
    port: u16,
    stderr: Arc<Mutex<Vec<String>>>,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Server {
    /// A registry on an empty root, started with soft and hard open-file
    /// limits of `soft`/`hard`, holding at most `max_open` designs.
    fn registry(soft: libc::rlim_t, hard: libc::rlim_t, max_open: usize) -> Server {
        let root = tmp_dir("root");
        let mut cmd = Command::new(bin());
        cmd.arg("--registry-root")
            .arg(&root)
            .args(["--http", "127.0.0.1:0"])
            .args(["--registry-max-open", &max_open.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        // SAFETY: the closure runs in the child between fork and exec, and
        // calls only setrlimit, which is async-signal-safe.
        unsafe {
            cmd.pre_exec(move || {
                let lim = libc::rlimit {
                    rlim_cur: soft,
                    rlim_max: hard,
                };
                if libc::setrlimit(libc::RLIMIT_NOFILE, &lim) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = cmd.spawn().expect("spawn the registry server");
        let stderr = child.stderr.take().expect("stderr piped");
        let lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = Arc::clone(&lines);
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                sink.lock().unwrap().push(line);
            }
        });
        let deadline = Instant::now() + Duration::from_secs(90);
        let port = loop {
            if let Some(p) = lines.lock().unwrap().iter().find_map(|l| {
                let i = l.find("http://127.0.0.1:")?;
                l[i + "http://127.0.0.1:".len()..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u16>()
                    .ok()
            }) {
                break p;
            }
            assert!(
                Instant::now() < deadline,
                "the registry printed no address within 90s: {:?}",
                lines.lock().unwrap()
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        Server {
            child,
            port,
            stderr: lines,
        }
    }

    fn said(&self) -> String {
        self.stderr.lock().unwrap().join("\n")
    }

    /// `GET /readyz`: (status, headers in lower case, body).
    fn readyz(&self) -> (u16, String, String) {
        let mut s = TcpStream::connect(("127.0.0.1", self.port)).expect("connect");
        s.set_read_timeout(Some(Duration::from_secs(30))).unwrap();
        write!(
            s,
            "GET /readyz HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
            self.port
        )
        .unwrap();
        let mut raw = Vec::new();
        let _ = s.read_to_end(&mut raw);
        let text = String::from_utf8_lossy(&raw).to_string();
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
        let status = head
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        (status, head.to_ascii_lowercase(), body.to_string())
    }
}

#[test]
fn a_registry_under_a_hard_limit_too_low_warns_names_the_numbers_and_keeps_serving() {
    // 8 designs (the default) × (64 + 12) + 32 = 640 files, under a hard limit
    // of 256 that the server cannot raise past.
    let server = Server::registry(200, 256, 8);
    let said = server.said();
    assert!(
        said.contains("raised this server's open-file limit from 200 to 256"),
        "the soft limit is raised to the hard one, and said:\n{said}"
    );
    for needle in [
        "WARNING",
        "8 designs open at once (--registry-max-open 8)",
        "up to 76 files open (64 table files, the per-store cap",
        "up to 640 files against an open-file limit of 256",
        "hard limit here is 256, which only whoever runs the host can raise",
        "lower --registry-max-open to 2 or fewer",
    ] {
        assert!(
            said.contains(needle),
            "the warning says `{needle}`:\n{said}"
        );
    }

    // It keeps running, and readiness says the headroom.
    let (status, head, body) = server.readyz();
    assert_eq!(status, 200, "a warning is not a refusal: {body}\n{said}");
    assert!(
        body.starts_with("ready: "),
        "the first line is unchanged: {body}"
    );
    assert!(
        head.contains("x-reflow2-open-file-limit: 256"),
        "the limit after the raise: {head}"
    );
    #[cfg(target_os = "linux")]
    {
        let line = body
            .lines()
            .find(|l| l.starts_with("open files: "))
            .unwrap_or_else(|| panic!("a headroom line: {body}"));
        assert!(
            line.ends_with("left)") && line.contains(" of 256 allowed"),
            "{line}"
        );
        let open: u64 = head
            .lines()
            .find_map(|l| l.strip_prefix("x-reflow2-open-files: "))
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or_else(|| panic!("a counted header on Linux: {head}"));
        assert!(open > 0 && open < 256, "{open}");
    }
}

#[test]
fn a_registry_whose_designs_fit_says_nothing_about_file_handles() {
    // 2 × 76 + 32 = 184 files, under a hard limit of 1024 raised to.
    let server = Server::registry(100, 1024, 2);
    // Readiness answers once the server is up, so its startup lines are in.
    let (status, _, _) = server.readyz();
    assert_eq!(status, 200);
    let said = server.said();
    assert!(
        said.contains("raised this server's open-file limit from 100 to 1024"),
        "{said}"
    );
    assert!(
        !said.contains("run out of file handles"),
        "no warning when the designs fit:\n{said}"
    );
}
