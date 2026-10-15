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

### Fixed

- **The installer says why it could not make your project's design record.** When the
  reflow2 binary refused to export a fresh project's design, `reflow2_init.py` printed
  "NOT created — ask your agent to `export_graph` to that path", although the agent's
  server is that same binary and refuses the same way. It now prints the binary's own
  reason, such as a build without on-disk storage and how to get one that has it.
