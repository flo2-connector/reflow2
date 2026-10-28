//! A server holding many designs and its open-file limit: raise it at start,
//! check the three numbers against each other, and say the headroom.
//!
//! ⭐ WHY THIS EXISTS. On 2026-10-28 flo2.io's registry, moving to 0.93.0,
//! opened all 64 of its designs and the last two failed with "Too many open
//! files" (`fact:root-cause-a-registry-at-its-open-limit-runs-out-of-file-handles-2026-10-28`).
//! Three bounds meet in one process and nothing compared them:
//!
//! · `--registry-max-open` (64 there), chosen as a MEMORY bound;
//! · [`reflow2_core::MAX_OPEN_FILES_PER_STORE`] (64 table files a store), chosen
//!   per store;
//! · the process's soft `RLIMIT_NOFILE`, whatever the host gives — Docker's
//!   default soft limit is 1024, under a hard limit of 524288.
//!
//! What this module does, as Anthony chose on 2026-10-28 (options A, B and F of
//! `dec:idea-a-server-checks-its-open-design-limit-and-per-design-file-cap-against-its-open-file-limit`):
//!
//! 1. [`raise_to_hard_limit`] — in registry mode the server raises its soft
//!    limit to the hard one, as Go's runtime has done for every program since
//!    1.19 (`req:a-server-holding-many-designs-starts-with-file-handles-for-all-it-may-open`).
//! 2. [`Budget::warning`] — then checks `max_open × (per-store cap + fixed files
//!    per design) + the process's own` against the soft limit, and when they do
//!    not fit WARNS, naming the three numbers and which one to change. It never
//!    refuses to start: a server whose designs are small (about 16 files each on
//!    flo2.io) works far below the worst case.
//! 3. [`headroom_line`] — `/readyz` reports the files open now against the
//!    limit (`req:a-server-reports-its-open-file-headroom`), so the shortfall
//!    shows before a design is refused. Counted from `/proc/self/fd` on Linux;
//!    anywhere else it says the count is unknown rather than guess.

/// Files one open design keeps open BESIDES its table files, counted with an
/// allowance for the ones it opens for a moment.
///
/// MEASURED 2026-10-28 on the published 0.92.0 build (`reflow2-mcp --http`,
/// one store, then `--registry-root` with four), by listing `/proc/<pid>/fd`
/// after `initialize`, `search_design` and `get_node` on each design:
///
/// · reflow2's own design (8,339 nodes, 66,755 edges): 22 handles for the
///   process, 12 of them in the store — 5 table files (`*.sst`) and 7 others;
/// · a one-node design: 9 in the store — 2 table files and the same 7;
/// · a registry: 10 at start with no design open, then +12, +9, +9, +9 as each
///   design opened, every one exactly its table files plus the same 7.
///
/// The 7 every design holds: RocksDB's write-ahead log (`*.log`), `MANIFEST-*`,
/// `LOCK`, its info `LOG`, two handles on the store directory, and the
/// full-text index's `.tantivy-writer.lock`. The index's segment files are
/// memory-mapped and hold no handle once mapped.
///
/// The other 5 are an allowance, not a measurement: for a moment a design can
/// also hold a second write-ahead log as it rotates, a new table file a flush
/// or compaction is writing, and the index's `meta.json` and new segment files
/// while it commits or its watcher reads.
pub const FIXED_FILES_PER_DESIGN: u64 = 12;

/// Files the server keeps open whatever it serves: 10 measured with no design
/// open (3 `/dev/null`, an eventfd, two epoll handles, the listening socket and
/// its runtime's), plus room for the connections it is answering.
pub const PROCESS_OWN_FILES: u64 = 32;

/// The table files one store may keep open — the number the store is opened
/// with, read from the core so the two cannot drift.
pub fn per_store_cap() -> u64 {
    u64::try_from(reflow2_core::MAX_OPEN_FILES_PER_STORE).unwrap_or(0)
}

/// The process's open-file limit. `u64::MAX` stands for unlimited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileLimit {
    /// The limit in force: no file can be opened past it.
    pub soft: u64,
    /// How far the process may raise `soft` on its own.
    pub hard: u64,
}

/// The process's open-file limit, or `None` where there is none to read
/// (not Unix).
pub fn file_limit() -> Option<FileLimit> {
    #[cfg(unix)]
    {
        read_rlimit().map(|(soft, hard)| FileLimit {
            soft: as_count(soft),
            hard: as_count(hard),
        })
    }
    #[cfg(not(unix))]
    {
        None
    }
}

#[cfg(unix)]
fn read_rlimit() -> Option<(libc::rlim_t, libc::rlim_t)> {
    let mut lim = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: getrlimit writes only the struct it is given.
    (unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut lim) } == 0)
        .then_some((lim.rlim_cur, lim.rlim_max))
}

#[cfg(unix)]
#[allow(clippy::unnecessary_cast)] // rlim_t is u64 on Linux and macOS, not everywhere
fn as_count(raw: libc::rlim_t) -> u64 {
    if raw == libc::RLIM_INFINITY {
        u64::MAX
    } else {
        raw as u64
    }
}

/// What [`raise_to_hard_limit`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Raise {
    /// The soft limit already stood at the hard one.
    AlreadyAtHard(FileLimit),
    /// Raised from `before` to `after`.
    Raised { before: FileLimit, after: FileLimit },
    /// It could not be raised; the sentence says what was tried and why not.
    Failed { before: FileLimit, why: String },
    /// This platform has no open-file limit to raise (not Unix).
    NoLimitHere,
}

impl Raise {
    /// The limit in force after the attempt, if there is one.
    pub fn limit(&self) -> Option<FileLimit> {
        match self {
            Raise::AlreadyAtHard(l) => Some(*l),
            Raise::Raised { after, .. } => Some(*after),
            Raise::Failed { before, .. } => Some(*before),
            Raise::NoLimitHere => None,
        }
    }

    /// The startup log line, or `None` when there is nothing worth saying.
    pub fn sentence(&self) -> Option<String> {
        match self {
            Raise::Raised { before, after } => Some(format!(
                "reflow2: raised this server's open-file limit from {} to {} (the hard limit \
                 this host allows), so the designs it holds open have room for their files.",
                count(before.soft),
                count(after.soft)
            )),
            Raise::Failed { before, why } => Some(format!(
                "reflow2: could not raise this server's open-file limit from {} toward its hard \
                 limit {}: {why}. It keeps {}.",
                count(before.soft),
                count(before.hard),
                count(before.soft)
            )),
            Raise::AlreadyAtHard(_) | Raise::NoLimitHere => None,
        }
    }
}

/// Raise the soft open-file limit as far as the host allows: to the hard
/// limit, or on macOS, where a hard limit of "unlimited" cannot be taken
/// literally, to `kern.maxfilesperproc` and then to `OPEN_MAX` (10240).
pub fn raise_to_hard_limit() -> Raise {
    #[cfg(unix)]
    {
        let Some((soft, hard)) = read_rlimit() else {
            return Raise::NoLimitHere;
        };
        let before = FileLimit {
            soft: as_count(soft),
            hard: as_count(hard),
        };
        if soft == hard {
            return Raise::AlreadyAtHard(before);
        }
        let mut tried = Vec::new();
        for target in raise_targets(hard) {
            if target <= soft {
                continue;
            }
            let lim = libc::rlimit {
                rlim_cur: target,
                rlim_max: hard,
            };
            // SAFETY: setrlimit reads only the struct it is given.
            if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &lim) } == 0 {
                let after = file_limit().unwrap_or(FileLimit {
                    soft: as_count(target),
                    hard: before.hard,
                });
                return Raise::Raised { before, after };
            }
            tried.push(format!(
                "{} ({})",
                count(as_count(target)),
                std::io::Error::last_os_error()
            ));
        }
        if tried.is_empty() {
            return Raise::AlreadyAtHard(before);
        }
        Raise::Failed {
            before,
            why: format!("setrlimit refused {}", tried.join(", then ")),
        }
    }
    #[cfg(not(unix))]
    {
        Raise::NoLimitHere
    }
}

/// The soft limits to try, best first.
#[cfg(unix)]
fn raise_targets(hard: libc::rlim_t) -> Vec<libc::rlim_t> {
    #[allow(unused_mut)] // pushed to on macOS only
    let mut targets = vec![hard];
    #[cfg(target_os = "macos")]
    {
        // <sys/syslimits.h>; the libc crate does not carry it for Apple.
        const OPEN_MAX: libc::rlim_t = 10240;
        if let Some(per_proc) = macos_maxfilesperproc() {
            targets.push(hard.min(per_proc));
        }
        targets.push(hard.min(OPEN_MAX));
    }
    targets
}

#[cfg(target_os = "macos")]
fn macos_maxfilesperproc() -> Option<libc::rlim_t> {
    let mut value: libc::c_int = 0;
    let mut len = std::mem::size_of::<libc::c_int>();
    // SAFETY: the name is NUL-terminated and `value`/`len` describe a buffer
    // of exactly the size sysctl is told.
    let ok = unsafe {
        libc::sysctlbyname(
            c"kern.maxfilesperproc".as_ptr(),
            (&mut value as *mut libc::c_int).cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    } == 0;
    (ok && value > 0).then(|| value as libc::rlim_t)
}

/// The three numbers a server holding many designs must keep consistent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// `--registry-max-open`: the designs it may hold open at once.
    pub max_open: u64,
    /// The table files one store may keep open ([`per_store_cap`]).
    pub per_store_cap: u64,
    /// The files a design keeps besides its table files ([`FIXED_FILES_PER_DESIGN`]).
    pub fixed_per_design: u64,
    /// The server's own files ([`PROCESS_OWN_FILES`]).
    pub process_own: u64,
}

impl Budget {
    /// The budget of a registry that may hold `max_open` designs open.
    pub fn for_registry(max_open: usize) -> Budget {
        Budget {
            max_open: u64::try_from(max_open).unwrap_or(u64::MAX),
            per_store_cap: per_store_cap(),
            fixed_per_design: FIXED_FILES_PER_DESIGN,
            process_own: PROCESS_OWN_FILES,
        }
    }

    /// The most files one open design may keep.
    pub fn per_design(&self) -> u64 {
        self.per_store_cap.saturating_add(self.fixed_per_design)
    }

    /// The most files the server may need with every design it may hold open.
    pub fn needed(&self) -> u64 {
        self.max_open
            .saturating_mul(self.per_design())
            .saturating_add(self.process_own)
    }

    /// How many designs fit under `limit` at their worst.
    pub fn designs_that_fit(&self, limit: u64) -> u64 {
        limit.saturating_sub(self.process_own) / self.per_design().max(1)
    }

    /// The startup warning when the designs this server may hold open could
    /// need more files than `limit.soft` allows; `None` when they fit. The
    /// remedy it names depends on whether `limit.hard` leaves room to raise.
    pub fn warning(&self, limit: FileLimit) -> Option<String> {
        let needed = self.needed();
        if needed <= limit.soft {
            return None;
        }
        let fit = self.designs_that_fit(limit.soft);
        let raise = if limit.hard >= needed {
            format!(
                "Raise the open-file limit to at least {needed} — its hard limit here is {}, so \
                 whatever starts this server can (`ulimit -n {needed}`, Docker's `--ulimit \
                 nofile={needed}:{needed}`, or LimitNOFILE={needed} in a systemd unit)",
                count(limit.hard)
            )
        } else {
            format!(
                "Raise the open-file limit to at least {needed} — its hard limit here is {}, \
                 which only whoever runs the host can raise (Docker's `--ulimit \
                 nofile={needed}:{needed}`, LimitNOFILE={needed} in a systemd unit, or \
                 /etc/security/limits.conf)",
                count(limit.hard)
            )
        };
        Some(format!(
            "reflow2: WARNING — this server may run out of file handles before it runs out of \
             designs to open. It may hold {max_open} designs open at once (--registry-max-open \
             {max_open}); each may keep up to {per_design} files open ({cap} table files, the \
             per-store cap, plus {fixed} for its write-ahead log, manifest, locks and full-text \
             index); and the server keeps about {own} of its own. That is up to {needed} files \
             against an open-file limit of {soft}. Past the limit a design fails to open with \
             \"Too many open files\" and its requests are answered 503. {raise}; or lower \
             --registry-max-open to {fit} or fewer. It keeps running: designs with few table \
             files use far fewer than the worst case (about 16 each on flo2.io on 2026-10-28), \
             and GET /readyz reports the files open now against the limit.",
            max_open = self.max_open,
            per_design = self.per_design(),
            cap = self.per_store_cap,
            fixed = self.fixed_per_design,
            own = self.process_own,
            soft = count(limit.soft),
        ))
    }
}

/// A limit as a person reads it.
fn count(n: u64) -> String {
    if n == u64::MAX {
        "unlimited".to_string()
    } else {
        n.to_string()
    }
}

/// How many files this process has open now, or `None` where that cannot be
/// counted without guessing (anywhere but Linux).
pub fn open_file_count() -> Option<u64> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // The directory listing holds one handle of its own while it is read;
        // it is not one the server keeps, so it is taken off.
        let n = std::fs::read_dir("/proc/self/fd").ok()?.count() as u64;
        Some(n.saturating_sub(1))
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        None
    }
}

/// What `/readyz` says about the files open, as numbers: (open now, soft
/// limit), each `None` when it is not known here.
pub fn headroom() -> (Option<u64>, Option<u64>) {
    (open_file_count(), file_limit().map(|l| l.soft))
}

/// The line `/readyz` adds: the files open now against the limit.
pub fn headroom_line(open: Option<u64>, limit: Option<u64>) -> String {
    match (open, limit) {
        (Some(open), Some(limit)) if limit != u64::MAX => format!(
            "open files: {open} of {limit} allowed ({} left)",
            limit.saturating_sub(open)
        ),
        (Some(open), _) => format!("open files: {open}, with no open-file limit"),
        (None, Some(limit)) => format!(
            "open files: unknown on this platform (the open-file limit is {})",
            count(limit)
        ),
        (None, None) => {
            "open files: unknown on this platform, and there is no open-file limit to read"
                .to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limit(soft: u64, hard: u64) -> FileLimit {
        FileLimit { soft, hard }
    }

    /// flo2.io's numbers on 2026-10-28, before the raise: 64 designs, the
    /// per-store cap of 64, Docker's soft 1024 under a hard 524288.
    #[test]
    fn flo2_ios_numbers_do_not_fit_under_dockers_default_and_the_warning_says_so() {
        let b = Budget::for_registry(64);
        assert_eq!(b.per_store_cap, 64, "the cap the store is opened with");
        assert_eq!(b.per_design(), 64 + FIXED_FILES_PER_DESIGN);
        assert_eq!(b.needed(), 64 * 76 + 32);
        assert_eq!(b.needed(), 4896);
        assert_eq!(b.designs_that_fit(1024), (1024 - 32) / 76);
        assert_eq!(b.designs_that_fit(1024), 13);

        let w = b
            .warning(limit(1024, 524_288))
            .expect("4896 files cannot fit under 1024");
        for needle in [
            "WARNING",
            "64 designs open at once (--registry-max-open 64)",
            "up to 76 files open (64 table files, the per-store cap, plus 12",
            "about 32 of its own",
            "up to 4896 files against an open-file limit of 1024",
            "Too many open files",
            "Raise the open-file limit to at least 4896",
            "its hard limit here is 524288, so whatever starts this server can",
            "ulimit -n 4896",
            "--ulimit nofile=4896:4896",
            "lower --registry-max-open to 13 or fewer",
            "It keeps running",
        ] {
            assert!(w.contains(needle), "the warning says `{needle}`:\n{w}");
        }
        assert!(!w.contains('\n'), "one log line: {w}");
    }

    #[test]
    fn after_the_raise_the_same_numbers_fit_and_nothing_is_said() {
        let b = Budget::for_registry(64);
        assert_eq!(b.warning(limit(524_288, 524_288)), None);
        assert_eq!(b.warning(limit(4896, 4896)), None, "exactly enough fits");
        assert!(b.warning(limit(4895, 4895)).is_some(), "one short does not");
        assert_eq!(b.warning(limit(u64::MAX, u64::MAX)), None, "unlimited");
    }

    #[test]
    fn a_hard_limit_too_low_to_raise_names_who_can_raise_it() {
        // macOS launchd's 256, or a small VM's systemd unit.
        let b = Budget::for_registry(8);
        assert_eq!(b.needed(), 8 * 76 + 32);
        let w = b
            .warning(limit(256, 256))
            .expect("640 cannot fit under 256");
        assert!(
            w.contains("up to 640 files against an open-file limit of 256"),
            "{w}"
        );
        assert!(
            w.contains("hard limit here is 256, which only whoever runs the host can raise"),
            "{w}"
        );
        assert!(w.contains("lower --registry-max-open to 2 or fewer"), "{w}");
    }

    #[test]
    fn a_limit_too_small_for_one_design_says_zero_rather_than_underflowing() {
        let b = Budget::for_registry(1);
        assert_eq!(b.designs_that_fit(16), 0);
        let w = b.warning(limit(16, 16)).expect("108 cannot fit under 16");
        assert!(w.contains("lower --registry-max-open to 0 or fewer"), "{w}");
    }

    #[test]
    fn the_headroom_line_says_unknown_rather_than_guessing() {
        assert_eq!(
            headroom_line(Some(1013), Some(1024)),
            "open files: 1013 of 1024 allowed (11 left)"
        );
        assert_eq!(
            headroom_line(None, Some(1024)),
            "open files: unknown on this platform (the open-file limit is 1024)"
        );
        assert!(headroom_line(None, None).contains("unknown"));
        assert!(headroom_line(Some(5), Some(u64::MAX)).contains("no open-file limit"));
        for line in [
            headroom_line(Some(1013), Some(1024)),
            headroom_line(None, Some(1024)),
            headroom_line(None, None),
        ] {
            assert!(!line.contains('/'), "a probe line carries no path: {line}");
        }
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn the_count_on_linux_is_this_processes_own() {
        let before = open_file_count().expect("Linux counts /proc/self/fd");
        let held: Vec<std::fs::File> = (0..5)
            .map(|_| std::fs::File::open("/proc/self/stat").unwrap())
            .collect();
        let during = open_file_count().unwrap();
        drop(held);
        // Other tests share this process and open and close files of their
        // own, so only a floor can be asserted: the five held are counted.
        assert!(before >= 3, "stdin, stdout and stderr at least: {before}");
        assert!(
            during >= 5,
            "the five held are counted: {before} -> {during}"
        );
    }
}
