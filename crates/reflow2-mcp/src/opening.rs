//! ⭐ "MAY THIS OPEN CREATE A STORE?" — ONE RULE, asked by every mode before it
//! opens the design store at `--graph-path`, and never by opening it.
//!
//! Opening a store where there is none CREATES one: the opener
//! (`DesignGraph::open_rocksdb`) makes the directory and RocksDB's files, and
//! writes the version stamp and the identity beside them. So the question is
//! about the PATH, and it is answered before anything is opened.
//!
//! THE RULE. A store exists at the path, or it does not. `--read-only` creates
//! nothing, so a read-only run opens a store only where one already exists;
//! without it, opening may create one. Whether a directory OPTED IN to a design
//! (`latent::design_present`) is a different question — whether a design was
//! started — and it is not asked here.
//!
//! WHY ONE FUNCTION. Each mode used to carry its own copy of the test, and the
//! mode with no copy is where the defect lived: item 1 gave the one-shot door
//! one, item 1b gave the `--shared` client and the latent surface one each, and
//! every SERVER (stdio, `--http`, `--serve-shared`) still opened the path and
//! created an empty store under `--read-only`, while that flag's help said
//! "no design store is created where there is none"
//! (`fact:a-read-only-server-creates-an-empty-store-where-there-is-none-2026-10-03`).
//!
//! WHO ASKS IT:
//! · the one-shot door — `one_shot::resolve_design` (`--call`, `--export`, …);
//! · the `--shared` client, which starts no server where there is no store;
//! · the latent surface, which opens a design that appears under it only when
//!   it may (`latent::LatentService`);
//! · every serving mode in `main` — stdio, `--http`, `--serve-shared` — which
//!   serves the latent surface where it may not open
//!   (`dec:a-read-only-server-with-no-store-serves-the-latent-surface`);
//! · the registry's discovery, which binds only a store that exists
//!   (`registry::Registry::discover`), so `--registry-root` opens no store that
//!   is missing, read-only or not.
//!
//! A unit test below holds `src/` to it: no other code tests a graph path for a
//! store, so a new mode asks this or is caught asking something else.
//!
//! AND "CAN THIS BUILD OPEN A STORE AT ALL?" — asked here too, by
//! [`refuse_without_on_disk_storage`], first thing in both openers. A build
//! without the crate's `rocksdb` feature refuses every store in a folder.

use std::path::Path;

use reflow2_core::DynoError;

/// Whether a design store exists at `graph_path`, so that opening it creates
/// no store. Asked of the path alone: nothing is opened, locked or written.
pub fn store_exists(graph_path: &str) -> bool {
    Path::new(graph_path).exists()
}

/// Whether a run may open the store at `graph_path`. With `--read-only` only an
/// existing store may be opened, because opening a missing one creates it and
/// `--read-only` creates nothing; without it, opening may create the store.
pub fn may_open(graph_path: &str, read_only: bool) -> bool {
    !read_only || store_exists(graph_path)
}

/// Whether this build can open a design store in a folder at all: the crate's
/// `rocksdb` feature, which default features turn on.
///
/// EVERY BUILD ANYONE INSTALLS OR RUNS HAS IT — a release, `cargo install`, the
/// Docker images, the installer's source-build instructions and the launcher
/// all build with default features. A build without it is the everyday
/// development build (`cargo dev-test`, Anthony, 2026-10-15: "for the vast
/// majority of development, I don't think it's necessary"), which serves
/// in-memory designs only and never compiles RocksDB's C++.
pub const STORES_IN_A_FOLDER: bool = cfg!(feature = "rocksdb");

/// The refusal a build without on-disk storage gives every path that would
/// open a store in a folder — asked FIRST, before anything is opened, locked
/// or written, so a refused open leaves the folder as it found it.
///
/// It NEVER falls back to memory: a design that silently stops persisting
/// reads as a design that works, right up to the session that finds it empty.
/// The core already fails loud when its own `rocksdb` feature is off
/// (`StorageEngine::new_rocksdb`), and this is the same contract — the same
/// error kind — said in this crate's terms, naming the build, the feature and
/// the fix. It comes first because the core's open reads beside the store
/// before it reaches its own refusal, and a store with an identity problem
/// would otherwise be answered with that problem instead of the real one.
///
/// Asked by the two openers every mode goes through
/// (`ReflowService::new_reporting`, `one_shot::open_store`), and by the
/// `--shared` client, which would otherwise start a daemon that can only fail.
pub fn refuse_without_on_disk_storage(graph_path: &str) -> Result<(), DynoError> {
    if STORES_IN_A_FOLDER {
        return Ok(());
    }
    Err(DynoError::Storage(without_on_disk_storage(graph_path)))
}

/// The words of that refusal, for a caller that says them itself.
pub fn without_on_disk_storage(graph_path: &str) -> String {
    format!(
        "this reflow2-mcp was built without on-disk storage (the `rocksdb` feature), so it \
         cannot open the design store at {graph_path}; nothing was opened or created there. \
         Build it with default features (`cargo build -p reflow2-mcp`, or `--features \
         rocksdb`) to open a store in a folder. A build without the feature serves \
         in-memory designs only, for development and tests."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix("reflow2-opening-")
            .tempdir()
            .unwrap()
    }

    #[test]
    fn read_only_may_open_only_a_store_that_exists() {
        let d = scratch();
        let g = d.path().join(".reflow2").join("graph");
        let g = g.to_str().unwrap();
        assert!(!store_exists(g));
        assert!(may_open(g, false), "a writable run may create the store");
        assert!(!may_open(g, true), "--read-only creates nothing");

        // An opted-in folder (`.reflow2/` and nothing in it) still has no store.
        std::fs::create_dir_all(d.path().join(".reflow2")).unwrap();
        assert!(!may_open(g, true), "opting in is not a store");

        std::fs::create_dir_all(g).unwrap();
        assert!(store_exists(g));
        assert!(
            may_open(g, true),
            "an existing store may be opened read-only"
        );
        assert!(!d.path().join(".reflow2").join("graph.meta.json").exists());
    }

    /// ⭐ ONE RULE, NOT A COPY PER MODE. Every test in `src/` of whether a
    /// graph path holds a store goes through this module. The three copies that
    /// stood before it were each written `Path::new(<graph_path>).exists()`, and
    /// the serving modes, which had none, created the store under
    /// `--read-only`. Scans the source rather than trusting a list.
    #[test]
    fn no_other_code_tests_a_graph_path_for_a_store() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut stack = vec![src.clone()];
        let mut copies = Vec::new();
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().is_none_or(|e| e != "rs") || path.ends_with("opening.rs") {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap();
                for (n, line) in text.lines().enumerate() {
                    let squeezed: String = line.split_whitespace().collect();
                    if squeezed.contains("Path::new(")
                        && squeezed.contains("graph_path")
                        && squeezed.contains(").exists()")
                    {
                        copies.push(format!(
                            "{}:{}: {}",
                            path.strip_prefix(&src).unwrap().display(),
                            n + 1,
                            line.trim()
                        ));
                    }
                }
            }
        }
        assert!(
            copies.is_empty(),
            "these test a graph path for a store themselves; ask \
             `crate::opening::store_exists` or `may_open` instead, so every mode answers \
             \"may this open create a store?\" the same way:\n{}",
            copies.join("\n")
        );
    }
}
