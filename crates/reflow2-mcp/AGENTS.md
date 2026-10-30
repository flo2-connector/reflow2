# AGENTS.md — `reflow2-mcp`

The agent-native surface: an MCP stdio server exposing the coherence-loop ops as tools. Thin by
intent — every tool locks the graph, calls one deterministic core op, and returns. Logic belongs
in `reflow2-core`, not here.

**Read the [root AGENTS.md](../../AGENTS.md) first** for orientation, architecture and the
project-wide rules. This file covers only what differs when you are editing *this* crate.

## Commands — read this before you start

**This crate has a `rocksdb` feature, ON by default** — on-disk storage, a design store in a
folder — because a design must survive across agent sessions. Every build anyone installs or runs
has it (a release, `cargo install`, the Docker images, the launcher), and with it the crate pays
the RocksDB C++ build: ~10 minutes cold, then cached.

**For development, build without it** (Anthony, 2026-10-15). `--no-default-features` builds and
tests this crate on the in-memory backend and never compiles RocksDB; `cargo dev-test` does that
for both crates. A build without the feature refuses every store in a folder, naming the feature
and the fix (`opening::refuse_without_on_disk_storage`) — it never serves memory in a store's
place.

**The ~170 files in `tests/` are compiled into four test programs**, not one each:
`tests/in_memory.rs` (everything that needs no store in a folder), `tests/on_disk.rs` (everything
that does; `required-features = ["rocksdb"]`, so a build without the feature skips it whole), and
two files that need a process of their own (Cargo.toml says why). Each file stays where it was and
is a module, so name the program and filter by the file:

```bash
cargo test -p reflow2-mcp --no-default-features              # in memory: everything but on_disk
cargo test -p reflow2-mcp --no-default-features --test in_memory a_budget_says_how_it_composes::
cargo test -p reflow2-mcp                                    # with RocksDB: every program
cargo test -p reflow2-mcp --test on_disk design_identity::   # one on-disk file
cargo check -p reflow2-mcp --no-default-features --all-targets   # type-check only, no link
cargo clippy -p reflow2-mcp --no-default-features --all-targets -- -D warnings
cargo clippy -p reflow2-mcp --all-targets -- -D warnings     # with RocksDB, as CI's full job

# End-to-end over the real stdio JSON-RPC binary, against a real RocksDB graph.
# Covers what cargo test cannot: the shipped surface, the tool schemas, and the
# JSON an agent actually receives. Stdlib-only Python; non-zero exit on failure.
# The binary it drives is the default build, with RocksDB.
cargo build -p reflow2-mcp
python3 ../../tools/smoke_mcp.py
```

**A new test file goes into one program's root** (`mod <file>;` in `tests/in_memory.rs`, or in
`tests/on_disk.rs` if its tests open a store in a folder): with automatic discovery off, a file
declared nowhere compiles nowhere, and `every_test_file_runs_in_exactly_one_program` fails until
it is declared. **The tests in a program share one process**, on its threads: a test that sets an
environment variable or the current directory, or flips process-wide state, is its own program.

**Run `smoke_mcp.py` for any change to the tool surface.** The Grok trial found a bug that three
home-grown test layers missed — every one of them was a client we wrote, so all three agreed with
each other and with the server, and all three were wrong. The smoke test drives the real binary
over real stdio. It is the only layer that would have caught it.

## Adding a tool

There is **no central registration list**. `#[tool_router]` derives the tool set from every
`#[tool]`-annotated method in the single `impl ReflowService` block, and `schemars` generates the
input schema from the request struct. So a new tool is:

1. A `#[derive(Debug, Deserialize, JsonSchema)] struct FooReq` — one doc comment per field; the
   agent reads them in `tools/list`, so they are user-facing text, not internal notes.
2. A `#[tool(description = "…")]` method on `ReflowService`. Describe the *use case*, not the
   mechanism — an agent picks tools by matching its problem, not by reading signatures.
3. Optional args go through `#[serde(default)]`. A half-specified argument set should fail loud
   rather than silently defaulting to "return everything".
4. **Classify it: `annotations(read_only_hint = …)` on the `#[tool]` attribute.** `true` if the
   method takes the shared lock (`let g = self.graph.lock()`), `false` if it takes `let mut g`
   or can otherwise mutate the graph — the borrow *is* the classification, so read it off the
   body rather than the name (`gap_to_prompt` and the `reconcile_*` family read like queries but
   record, so they are `false`). `smoke_mcp.py` fails if any served tool omits the hint, so a
   tool cannot ship unclassified (BL-76).
5. **Regenerate the toolsnap.** `cargo build -p reflow2-mcp && python3 ../../tools/toolsnap.py
   --update`, then commit the new `tools/toolsnaps/<name>.json`. CI diffs the served surface
   against these goldens; a surface change is a reviewed diff, never a silent one (BL-76).

## Conventions that are easy to break

- **Return through `ok_json`.** It wraps bare arrays as `{count, items}`, because MCP defines
  `structuredContent` as an object and a spec-compliant client rejects an array outright. That is
  the bug the Grok trial found. Wrapping lives at the one choke point precisely so a new list tool
  cannot reintroduce it by forgetting.
- **Never hold the lock across an `await`.** `let g = self.graph.lock().await;` then call the sync
  core op and let the guard drop.
- **Distinguish caller errors from server faults.** `params_err` / `McpError::invalid_params` for
  a bad argument; `dyno_err` (internal error) for a genuine failure. A typo in a type name is not
  a server fault.
- **A rejection should say what would have worked.** `create_node` / `create_edge` list the valid
  alternatives on failure — a blind trial burned twenty minutes because `Unknown edge type:
  PACKAGES` "tells me I'm wrong without telling me what's right". Keep that property when you
  touch the write path, and keep it failing loud: the rejection is better, not softer.
- **`service.rs` is large and the macro forces one `impl` block** — but request structs, DTOs and
  error helpers do not have to live in it. Factor along that seam rather than growing the file.
- **Nothing a client can call or read is removed without a release of notice.** A tool, an
  argument, a reply field, a node type, an edge type or a property is retired in two releases:
  an entry in `deprecation::REGISTRY` (`src/deprecation.rs`) plus a `### Deprecated` changelog
  fragment with a **What to do:** line first, the removal in the release after. The registry
  generates the marks in tools/list and describe_schema and the `deprecation_notice` on a call,
  so never hand-write "deprecated" into a description. A tool keeps its entry for the release
  that removes it, so its old name answers with what to do. `tests/the_mcp_surface_only_grows.rs`
  fails a removal the last release did not announce. See the root AGENTS.md.
- **A COUNTED REFUSING STUB** (`Deprecation::stub`, since 2026-10-30) is a tool still LISTED whose
  handler is `stub_answer`: it changes nothing, refuses with the call that does its job, and every
  call lands in the usage ledger under its own name (`usage_report`'s `retired_stubs`). Its
  `#[tool]` description stays the tool's own (the mark is added at list time, so find_tools'
  word weights do not move), find_tools never offers it, and served text names its survivor
  (`no_served_text_names_a_tool_that_is_not_served`). The old body is kept beside it as
  `<tool>_kept`: at the re-count (`deprecation::RE_COUNT`) a stub nobody called is deleted with
  its `_kept` body, and one somebody called is restored by pointing the tool back at `_kept`.
