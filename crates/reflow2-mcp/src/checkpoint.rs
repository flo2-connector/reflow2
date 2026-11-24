//! A server copies every design it serves, consistently, without stopping
//! (`cap:a-server-checkpoints-every-design-it-serves-on-a-signal-without-stopping`).
//!
//! ⭐ WHY IT EXISTS. flo2.io's nightly backup stopped the engine for about two
//! and a half minutes every night (04:14 to 04:17 UTC on 2026-11-22 and 11-23),
//! because only the process holding a RocksDB store can take a consistent copy
//! of it, and the backup's only way to get one was to make that process let go.
//! RocksDB's answer is a CHECKPOINT: the store's files as of one moment,
//! hard-linked into a new folder while the store stays open. Only the process
//! holding the store can take one, so the backup has to ASK the server.
//!
//! ⭐ HOW IT IS ASKED: A SIGNAL (Anthony, 2026-11-23, "A signal"). `SIGUSR1`
//! adds nothing reachable over the network, and only someone who can signal
//! the process (root on the box, or `docker kill -s USR1`) can start a copy.
//! The server answers through the filesystem, where the backup reads it:
//!
//! - the copies land in `<dir>/<stamp>.partial/<design folder>/graph`, one per
//!   design, `<design folder>` being the folder under the registry root that
//!   holds the design's `.reflow2/`;
//! - [`RESULT_FILE`] is written beside them, naming every design, whether it
//!   was open, how long its copy took, and any that could not be copied with
//!   why;
//! - only then is `<stamp>.partial` renamed to `<stamp>`, so a folder without
//!   `.partial` is a whole set, and a reader never takes a half-written one.
//!
//! The server never deletes a set: what to keep is the backup's to decide.
//!
//! 🛑 WITHOUT `--checkpoint-dir` THE SIGNAL CHANGES NOTHING, and says so in the
//! log. Before this, `SIGUSR1` ended the process (its default action), so a
//! stray signal to a server not set up for it now costs a log line rather than
//! an outage.
//!
//! NOT HERE: a single-design server (`--graph-path`) does not answer the
//! signal yet; flo2.io serves a registry.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Serialize;

/// The environment variable `--checkpoint-dir` may also be set by.
pub const ENV: &str = "REFLOW2_CHECKPOINT_DIR";

/// The file written beside a set of copies, last before the set is renamed.
pub const RESULT_FILE: &str = "result.json";

/// The suffix of a set still being written.
pub const PARTIAL_SUFFIX: &str = ".partial";

/// One design's copy, as [`RESULT_FILE`] lists it.
#[derive(Debug, Clone, Serialize)]
pub struct DesignCopied {
    /// The folder under the registry root that holds the design's `.reflow2/`,
    /// which is also the folder its copy is under in the set.
    pub folder: String,
    pub graph_id: String,
    /// Whether this server held the design open when it was copied: then the
    /// copy was taken through the store it holds, otherwise from the store on
    /// disk, with any request that would have opened it held back meanwhile.
    pub was_open: bool,
    pub took_ms: u64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// One whole set, as [`RESULT_FILE`] holds it.
#[derive(Debug, Clone, Serialize)]
pub struct CheckpointRun {
    /// The set's folder name under the checkpoint folder: when it began, UTC.
    pub stamp: String,
    pub took_ms: u64,
    /// How many designs were copied, and how many could not be.
    pub copied: usize,
    pub failed: usize,
    /// The copy of each design is `<folder>/graph` in this set, a RocksDB
    /// store as of the moment it was taken. A design opened from it rebuilds
    /// its search index, which is not copied.
    pub designs: Vec<DesignCopied>,
}

/// A set being written: its stamp and its `.partial` folder.
#[derive(Debug)]
pub struct Pending {
    pub stamp: String,
    pub partial: PathBuf,
    started: Instant,
    designs: Vec<DesignCopied>,
}

impl Pending {
    /// Begin a set under `dir` (created if it is missing), named by the time
    /// now, made unique by a counter if a set of that second already exists.
    pub fn begin(dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(dir).map_err(|e| {
            format!(
                "could not make the checkpoint folder {}: {e}",
                dir.display()
            )
        })?;
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let base = reflow2_core::dates::utc_stamp(secs);
        for n in 1..1000 {
            let stamp = if n == 1 {
                base.clone()
            } else {
                format!("{base}-{n}")
            };
            let whole = dir.join(&stamp);
            let partial = dir.join(format!("{stamp}{PARTIAL_SUFFIX}"));
            if whole.exists() || partial.exists() {
                continue;
            }
            std::fs::create_dir(&partial).map_err(|e| {
                format!("could not make the set's folder {}: {e}", partial.display())
            })?;
            return Ok(Pending {
                stamp,
                partial,
                started: Instant::now(),
                designs: Vec::new(),
            });
        }
        Err(format!(
            "a thousand checkpoint sets are already named {base} in {}",
            dir.display()
        ))
    }

    /// Where the copy of the design held in `folder` goes in this set, its
    /// parent made. The path itself must not exist: RocksDB makes it.
    pub fn place_for(&self, folder: &str) -> Result<PathBuf, String> {
        let parent = self.partial.join(folder);
        std::fs::create_dir_all(&parent)
            .map_err(|e| format!("could not make {}: {e}", parent.display()))?;
        Ok(parent.join("graph"))
    }

    /// Record one design's copy.
    pub fn record(&mut self, copied: DesignCopied) {
        self.designs.push(copied);
    }

    /// Write [`RESULT_FILE`] and rename the set to its whole name. Returns the
    /// whole set's folder and what it holds.
    pub fn finish(self) -> Result<(PathBuf, CheckpointRun), String> {
        let failed = self.designs.iter().filter(|d| !d.ok).count();
        let run = CheckpointRun {
            stamp: self.stamp.clone(),
            took_ms: ms(self.started.elapsed()),
            copied: self.designs.len() - failed,
            failed,
            designs: self.designs,
        };
        let body = serde_json::to_vec_pretty(&run).map_err(|e| e.to_string())?;
        let result = self.partial.join(RESULT_FILE);
        write_synced(&result, &body)
            .map_err(|e| format!("could not write {}: {e}", result.display()))?;
        let whole = self
            .partial
            .with_file_name(self.partial.file_name().map_or_else(
                || self.stamp.clone(),
                |n| {
                    n.to_string_lossy()
                        .trim_end_matches(PARTIAL_SUFFIX)
                        .to_string()
                },
            ));
        std::fs::rename(&self.partial, &whole).map_err(|e| {
            format!(
                "could not rename {} to {}: {e}",
                self.partial.display(),
                whole.display()
            )
        })?;
        Ok((whole, run))
    }
}

fn write_synced(path: &Path, body: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    f.write_all(body)?;
    f.sync_all()
}

/// Milliseconds, saturating.
pub fn ms(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

/// Answer `SIGUSR1` for as long as the process runs: each signal takes one
/// whole set of `router`'s designs into `dir`, one set at a time, and logs
/// what it did. With no `dir`, each signal is logged and nothing is done.
///
/// Must be called inside a tokio runtime. Installing the handler replaces the
/// signal's default action, which ended the process.
#[cfg(unix)]
pub fn answer_signal(router: crate::registry_http::GraphRouter, dir: Option<PathBuf>) {
    use tokio::signal::unix::{SignalKind, signal};
    let mut usr1 = match signal(SignalKind::user_defined1()) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                "could not listen for SIGUSR1, so a backup cannot ask this server for \
                 checkpoints: {e}"
            );
            return;
        }
    };
    match &dir {
        Some(d) => tracing::info!(
            "SIGUSR1 takes a checkpoint of every design this server serves into {} \
             (--checkpoint-dir)",
            d.display()
        ),
        None => tracing::info!("SIGUSR1 is logged and does nothing: no --checkpoint-dir was given"),
    }
    tokio::spawn(async move {
        while usr1.recv().await.is_some() {
            let Some(dir) = &dir else {
                tracing::warn!(
                    "SIGUSR1 received, and nothing was copied: this server was started \
                     without --checkpoint-dir (or {ENV}), so it has nowhere to put a checkpoint"
                );
                continue;
            };
            match router.checkpoint_all(dir).await {
                Ok((whole, run)) => tracing::info!(
                    set = %whole.display(),
                    copied = run.copied,
                    failed = run.failed,
                    took_ms = run.took_ms,
                    "checkpointed the designs this server serves"
                ),
                Err(e) => tracing::error!("SIGUSR1: no checkpoint set was written: {e}"),
            }
        }
    });
}

/// The folder under the registry root that holds the store at `store_path`
/// (`<root>/<folder>/.reflow2/graph`), or the store's own path made safe as a
/// name when it is laid out otherwise.
pub fn folder_of(store_path: &str) -> String {
    let p = Path::new(store_path);
    p.parent()
        .filter(|d| d.file_name().is_some_and(|n| n == ".reflow2"))
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| store_path.replace(['/', '\\'], "_"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_design_is_named_by_the_folder_that_holds_its_dot_reflow2() {
        assert_eq!(
            folder_of("/data/reflow2-a565f7/.reflow2/graph"),
            "reflow2-a565f7"
        );
        assert_eq!(folder_of("/odd/place/graph"), "_odd_place_graph");
    }

    #[test]
    fn a_set_is_partial_until_it_is_finished_and_then_holds_its_result() {
        let dir = tempfile::tempdir().unwrap();
        let mut set = Pending::begin(dir.path()).unwrap();
        assert!(set.partial.to_string_lossy().ends_with(PARTIAL_SUFFIX));
        let place = set.place_for("one").unwrap();
        assert!(!place.exists(), "RocksDB makes the copy's own folder");
        set.record(DesignCopied {
            folder: "one".into(),
            graph_id: "a".into(),
            was_open: false,
            took_ms: 1,
            ok: false,
            error: Some("why".into()),
        });
        let stamp = set.stamp.clone();
        let (whole, run) = set.finish().unwrap();
        assert_eq!(whole, dir.path().join(&stamp));
        assert!(!dir.path().join(format!("{stamp}{PARTIAL_SUFFIX}")).exists());
        assert_eq!((run.copied, run.failed), (0, 1));
        let body: serde_json::Value =
            serde_json::from_slice(&std::fs::read(whole.join(RESULT_FILE)).unwrap()).unwrap();
        assert_eq!(body["designs"][0]["error"], "why");
        // A second set in the same second gets its own name.
        let again = Pending::begin(dir.path()).unwrap();
        assert_ne!(again.stamp, stamp);
    }
}
