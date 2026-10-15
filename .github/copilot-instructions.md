# Working on Reflow 2.0

This repo already carries agent instructions in portable form. Read, in order:

0. **`git pull --rebase`** — before reading anything else. Two people work this repo through
   git; a stale checkout means claiming work someone already holds.
1. **[../COORD.md](../COORD.md)** — the claim board. Claim what you take, and follow its rules
   for resolving merge conflicts (short version: never resolve by discarding the other side).
2. **[../AGENTS.md](../AGENTS.md)** — the primary instruction file: commands, architecture, and
   the non-negotiable rules for changing this project.
3. **[../docs/backlog.md](../docs/backlog.md)** — what is open, and why.

A change is done only when `cargo dev-test`, `cargo clippy --workspace --no-default-features
--all-targets -- -D warnings`, `cargo fmt --check`, `python3 tools/validate_schema.py` (after a
schema edit) and `python3 tools/smoke_mcp.py` (after a tool-surface change) are clean.

`cargo dev-test` is a cargo alias (`.cargo/config.toml`) that builds and tests both crates
WITHOUT RocksDB, on the in-memory backend, so it never pays the ten-minute C++ build. It skips
only the MCP crate's `on_disk` test program; CI's full job runs that, with RocksDB, on every pull
request, and `cargo full-test` runs it locally when a change touches the store on disk. For the
fastest loop, `cargo test -p reflow2-core --no-default-features` tests the core alone.

> **reflow2 is installed here.** The design graph is this project's memory — read [REFLOW2.md](REFLOW2.md) and consult it before writing or changing code.
