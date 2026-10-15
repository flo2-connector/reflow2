### Changed

- **For contributors to reflow2: everyday builds and tests run without RocksDB, and the MCP
  crate's tests are four test programs instead of one per file.** `reflow2-mcp` has a `rocksdb`
  feature, on by default, so a release, `cargo install`, the Docker images, the installer and the
  launcher build exactly as before. `cargo dev-test`, a cargo alias, builds and tests both crates
  without it, on the in-memory backend: it never compiles RocksDB's C++, and it skips only the
  `on_disk` test program, which CI's full job runs on every pull request (`cargo full-test` runs
  it locally). A `reflow2-mcp` built without the feature refuses every store in a folder, naming
  the feature and the fix, and never serves an in-memory design in its place. The crate's test
  files stay where they were and are modules of `in_memory`, `on_disk`, and two programs that
  need a process of their own, so one file's tests run with
  `cargo test -p reflow2-mcp --test in_memory <file>::`; `tools/run_to_files.py` still reports
  each file on its own. Nothing changes for anyone using reflow2.
