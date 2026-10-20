# AGENTS.md — orientation for AI agents working **on** Reflow 2.0

> ## Which are you doing?
>
> | You are… | Read | You are in the wrong place if… |
> |---|---|---|
> | **Building or improving reflow2 itself** — changing the Rust, the schema, the tool surface, the skills | **this file**, then [docs/sharpening.md](docs/sharpening.md) | — |
> | **Using reflow2 to design your own project** | **[getting-started/AGENTS.md](getting-started/AGENTS.md)** — and you should not be reading it here. Run `python3 tools/reflow2_init.py /path/to/your/project`, which installs it into your repo along with the skills and MCP config | you are following the build commands below. They build *reflow2*; they have nothing to do with your project, and reflow2 deliberately installs no `src/` layout or build file, because your project type is a design *output* |
>
> Everything after this line is for the first case. The consumer kit lives in
> [getting-started/](getting-started/) and is a separate document with a separate audience — it
> never mentions cargo, and this file never mentions your design.
>
> **This is the primary instruction file for this repo** — it follows the
> [agents.md](https://agents.md) convention, so every agent reads the same thing.
>
> **Before you start:** run **`git pull --rebase`**, then run **`claim_report`** and claim what you
> take with **`claim_region`** — the board moved into the graph on 2026-08-04
> (`dec:coord-board-in-graph`), so claims travel in the committed design (`docs/design/reflow2/`) and a graph
> you haven't pulled is out of date. **[COORD.md](COORD.md)** keeps the handles, the conventions, and
> resolving merge conflicts without discarding anyone's work.
> **The graph** has what is open and why: the graph — `loop_status` for what the loop owes, `detect_gaps` for the open questions, `search_design` to find a past finding by its words. `docs/backlog.md` was retired 2026-08-07 (`dec:backlog-is-retired`); its open rows are nodes now.

Read this first. It tells you what this project is, how it's organized, and the rules to
follow so your changes stay coherent with the design.

> **If you are here to improve reflow2, read [docs/sharpening.md](docs/sharpening.md) before you
> pick up an item.** It is the standing method for finding this project's own gaps — where findings
> actually come from, how much to trust one, and the specific way this work goes wrong (shaping the
> model until the tool goes quiet, then reporting the tool is fine). It also names the
> instruments that measure whether the loop works, and their current baselines. **reflow2's silence
> is never evidence that reflow2 is healthy.**

**Per-crate files exist and the closest one wins**, per the convention. If you are editing inside
a crate, read its file too — the build commands genuinely differ, and getting that wrong costs
ten minutes of C++ compile:

| Editing | Also read | Why it differs |
|---|---|---|
| `crates/reflow2-core/**` | [its AGENTS.md](crates/reflow2-core/AGENTS.md) | sub-second test path; core-only invariants |
| `crates/reflow2-mcp/**` | [its AGENTS.md](crates/reflow2-mcp/AGENTS.md) | RocksDB is a default feature, off for development; four test programs; needs the smoke test |

## Commands

Everything runs from the repo root. The core crate is `crates/reflow2-core`.

```bash
# Build / test — THE EVERYDAY COMMAND, for development and every local gate:
# both crates WITHOUT RocksDB, on the in-memory backend (a cargo alias in
# .cargo/config.toml for `cargo test --workspace --no-default-features
# --no-fail-fast`). It never compiles librocksdb-sys' C++ (10-19 min cold,
# 2 GB on disk), and it skips only the MCP crate's `on_disk` test program —
# the tests that need a store in a folder — which CI's full job runs on every
# pull request. Anthony, 2026-10-15: "for the vast majority of development,
# I don't think it's necessary".
#
# IT LEAVES target/debug/reflow2-mcp BUILT WITHOUT ON-DISK STORAGE, which
# refuses every store in a folder and says so. Before anything that drives the
# binary on a store (smoke_mcp.py, toolsnap.py and the other instruments, the
# installer from this checkout), run `cargo build -p reflow2-mcp`.
cargo dev-test

# The core alone, with no features at all (no Tantivy either): the sub-second
# edit loop once built.
cargo test -p reflow2-core --no-default-features
cargo test -p reflow2-core --no-default-features --test in_memory heal::  # one integration-test file
cargo test -p reflow2-core --no-default-features golden_thread_round_trips  # one test by name
cargo test -p reflow2-core --no-default-features --lib                # unit + doctests only

# BOTH CRATES' test files are compiled into a few test PROGRAMS, not one each
# (tests/in_memory.rs, plus feature-gated programs — Cargo.toml in each crate
# says which and why). So one file's tests are its program and its module:
cargo test -p reflow2-core --no-default-features --test in_memory heal::
cargo test -p reflow2-mcp --no-default-features --test in_memory a_budget_says_how_it_composes::
cargo test -p reflow2-mcp --test on_disk design_identity::            # needs RocksDB

cargo clippy -p reflow2-core --no-default-features --all-targets      # keep clippy-clean
cargo clippy -p reflow2-mcp --no-default-features --all-targets       # the MCP crate, in memory
cargo fmt                                               # and fmt-clean (cargo fmt --check in CI)

# THE FULL RUN, WITH RocksDB: every test program, `on_disk` included — the
# alias for `cargo test --workspace --no-fail-fast`, exactly CI's full job.
# CI runs it on every pull request, so run it locally only when your change
# touches the store on disk: opening one, its identity file, its lock, its
# version stamp, the shared daemon, importing or exporting a store. Pays the
# RocksDB compile once, then it is cached.
#
# This is the heaviest command in the file. On a machine with ~8-12 GB of RAM it
# is the one that gets your terminal killed — see "Building with limited RAM"
# below, and prefer the memory-capped form there.
cargo full-test

# Schema validation (Python, no Rust toolchain needed). Must print "OK" after any
# schema/*.yaml edit. Needs PyYAML; use whatever python3 has it.
python3 tools/validate_schema.py

# The installer's own suite (stdlib only, hermetic — no network, no binary).
# Run after any tools/reflow2_init.py change; it is what lets the self-model
# call cap:kit verified.
python3 tools/test_init.py

# Call ONE tool from the shell — the door a build script or a Makefile uses.
# Prints the reply's JSON on stdout; a refusal goes to stderr with exit 1; a
# reply the tool marked as an error exits 2. Arguments that do not fit the
# tool's published schema are such a reply (exit 2, naming the tool and the
# field path); exit 1 is a tool's own rule refusing. Read-only tools work while
# a server holds the graph (best-effort snapshot, stderr says so); writers
# refuse then. A writer that succeeds writes the export before exiting —
# `--export-to`, else the one the project's MCP config names — and stderr says
# where; exit 3 means the write landed and the export could not be written.
#
# IN THIS REPOSITORY, `.reflow2/graph` IS REFUSED: since 2026-10-10 `.reflow2.toml`
# names reflow2's design on flo2.io, and a one-shot command never opens the store
# the move left behind. Point --graph-path at a store you imported from
# docs/design/reflow2/ (the import below), or reach the design itself over MCP.
./target/release/reflow2-mcp --graph-path .reflow2/graph --call graph_report
./target/release/reflow2-mcp --graph-path .reflow2/graph --call budget_report --args '{"constraint_id":"con:mass"}'
# The same door with the read/write split in the command text, for a terminal
# that asks before each command: `read` runs only a tool that changes nothing
# (graph or disk) and refuses any other by name before opening anything, so one
# rule `^reflow2 read ` approves every read and no write; `write` runs anything.
# `read --list` prints the split (crates/reflow2-mcp/src/verbs.rs).
./target/release/reflow2-mcp read get_node '{"id":"req:x"}'
./target/release/reflow2-mcp write add_requirement --args - <<'EOF'
{"id": "req:x", "name": "…", "statement": "…"}
EOF
# `--args @path` (or `read TOOL @path`) reads the object from a file instead, so
# prose never passes through the command text.
./target/release/reflow2-mcp write add_requirement --args @req.json

# Read how to call a tool before calling it through that door: its input schema
# (nested $defs shapes, allowed values) and the lessons THIS design holds for it
# — the tools/list entry a session on the design gets, which --call never reads.
# Brief by default (under the reply budget); --full is the entry unaltered.
# --list-tools --full is the whole served list, for a generator. Read-only; the
# same answer is served as describe_schema with `tool`.
./target/release/reflow2-mcp --graph-path .reflow2/graph --describe add_decision
./target/release/reflow2-mcp --graph-path .reflow2/graph --list-tools --full

# Load a design into a graph without speaking MCP — the sibling of --export.
# Upsert, so it layers onto whatever is there. Takes `-` for stdin, so an export
# on one machine pipes into an import on another. The graph is single-writer:
# stop any running MCP server first, and the error says so if you forget. In
# this repository, import into a path outside `.reflow2/` (see above).
./target/release/reflow2-mcp --graph-path /tmp/reflow2-local/.reflow2/graph --import docs/design/reflow2/
# (--import, import_graph, --diff and compare_designs take either saved form: the
# item layout's directory, or a single-file export such as docs/design/reflow2.json.)

# reflow2's own functional design, as a reflow2 graph. The committed design at
# docs/design/reflow2/ (one file per node and per edge) is the durable record — .reflow2/ is gitignored, so
# the committed files are what gets reviewed and diffed. Rebuild it after a design
# change; --analyse-only re-imports the committed design and re-runs the analysis.
python3 tools/build_design_graph.py
python3 tools/build_design_graph.py --analyse-only

# Phase-coverage trial — does the design still carry weight after P2? Seeds a
# realistic graph, injects the divergences P3/P4/P5 are each supposed to catch,
# and scores whether the graph noticed. 13/13 as of 2026-07-19 (BL-30 and BL-9
# closed the last two probes) — fully green and exits 0, so it now works as a
# regression gate. It remains the standing measurement for the failure that
# sank the original reflow — the early phases going well and the later ones
# proceeding as if they hadn't.
python3 tools/phase_trial.py

# Seat-identity probe — does ONE client keep ONE seat? Per transport, per
# protocol version, AND is a claim with no seat refused exactly where the session
# cannot supply one. test_shared_sessions proves the other half (two clients get
# two DIFFERENT seats) and nothing proved the complement, which is why the whole
# suite stayed green through the rmcp v3 upgrade while seat identity was already
# broken: every test client here negotiates a pre-2026-07-28 version. Green since
# 2026-07-30 (mint_seat + the claim_region guard) and now a CI gate in the `full`
# job — req:seat-identity-survives-stateless-mcp, dec:stateless-seat-handle.
python3 tools/stateless_seat_probe.py

# Erosion trial — the sharper question. Not "did a file change?" but: after N
# rounds of test-fails/fix-code/accept, does the design still describe what
# shipped? 7/8 (was 2/7): the one remaining miss is the semantic
# description-vs-history judgement, deliberately not built (the ledger
# reports; the human judges). Non-zero exit by design until that decision
# changes.
python3 tools/erosion_trial.py

# The same cycle done right — the constructive counterpart. Proves designed ==
# released is reachable today with axis-Z discipline (original intent survives in
# a Snapshot), and that reflow2 gives the SAME verdict for the coherent graph and
# the eroded one. That gap is BL-35.
python3 tools/coherent_erosion_trial.py

# Project the design as viewpoints — functional / operational-flow / structural /
# traceability / as-released / decisions, DoDAF-informed (docs/viewpoints.md).
# Pure projections: anything a view needs that the graph cannot supply is
# CONFESSED, and every confession is a finding (BL-40). Also a probe.
python3 tools/render_views.py                        # the committed design export
python3 tools/render_views.py --graph-path <dir>     # a live graph (stop the server first)

# Process-model probe — can reflow2 hold its own operating model, a Flow whose
# feedback loops are the subject? Started as BL-37's friction log (no Flow write
# side, roles lost, cycles invisible, product-shaped nudge); all four are fixed
# and this now exits non-zero on any regression. Needs the built MCP binary.
python3 tools/model_the_loop.py

# End-to-end smoke test of the MCP *binary* (stdio JSON-RPC, real RocksDB graph).
# Covers what cargo test can't: the shipped surface, tool schemas, and the JSON an
# agent actually receives. Needs `cargo build -p reflow2-mcp` first (RocksDB, ~10 min
# cold). Stdlib-only Python; exits non-zero on any failed check.
python3 tools/smoke_mcp.py

# Install or update reflow2 in a consumer project (the design environment only —
# never a src/ layout or build file; project type is a design output, not an input).
python3 tools/reflow2_init.py /path/to/project           # set up, or update in place
python3 tools/reflow2_init.py /path/to/project --check   # what would change
```

Tests live beside the code as `crates/reflow2-core/tests/*.rs` (one file per module/concern) plus
unit tests in `src/schema.rs` and doctests in `src/lib.rs`/`src/nodes.rs`.

### Building with limited RAM

**If your terminal window vanishes mid-build — shell, agent and MCP server all dying at once —
that is not a crash, and nothing will be in `dmesg`.** It is `systemd-oomd`, which watches
*memory pressure* rather than the kernel OOM killer watching allocation failure. Your terminal,
the agent driving it and the MCP server all live in one VTE cgroup scope, so oomd takes the
whole scope and the symptom presents as "the terminal crashed". Find it with:

```bash
journalctl --user --since "-3 days" | grep -iE 'oomd|vte-spawn'
```

`cargo test --workspace` (`cargo full-test`) is what usually triggers it: RocksDB's C++ build
plus parallel rustc jobs is the peak, and a full workspace run has been measured at **9.3 GB** on
a 4-core/11 GiB box. `cargo dev-test` never compiles RocksDB, so the C++ half of that peak is not
in it. Two mitigations that help but do **not** prevent a kill on their own — `[build] jobs = 2`
and `[profile.dev] debug = "line-tables-only"` — belong in your own `~/.cargo/config.toml`,
**not in this repo**: they are properties of one machine, and committing them would slow builds
and degrade debug info for every other developer and for CI.

**The reliable fix is to give cargo its own cgroup, so a runaway build dies instead of your
session:**

```bash
systemd-run --user --scope -p MemoryMax=6G -p MemorySwapMax=2G \
    cargo test --workspace -- --test-threads=2
```

**Disabling `systemd-oomd` outright is a legitimate option, and is what this repo's maintainer
runs.** It does not remove OOM handling — the kernel OOM killer is built in and unremovable, and
that is the point: oomd kills by *cgroup* and takes your whole session, whereas the kernel kills
the single worst process, which under a build is `rustc` and not your terminal. The cost is that
the kernel only acts at true exhaustion rather than early on pressure, so a runaway build can
thrash the machine first — which is exactly what the capped scope above prevents. Use both, or
neither; the capped scope alone is the more targeted of the two.

Two habits worth keeping regardless. Write test output to a file and grep it, rather than
running the suite twice to get failures and then a pass count — that doubling is pure peak for
no information. And for dev iteration use `cargo dev-test`, or the core-only command at the top
of this section, which skip the RocksDB compile entirely; the full run is CI's, not an edit-loop
command.

**Disk.** cargo never deletes an old build: every rebuild mints a new copy of each test program
and keeps the last one. Since 2026-10-15 the MCP crate's ~170 test files are four test programs
rather than one program each, and `cargo dev-test` links no RocksDB into any of them; the core's
test files are still one program each. When the disk gets tight, `rm -rf target/debug/incremental`
and `cargo clean -p reflow2-core -p reflow2-mcp` — never a bare `cargo clean`, which also
destroys the cached RocksDB build. The root `Cargo.toml`'s notes have the measurements.

None of this affects people who merely *use* reflow2: `tools/install.sh` fetches a prebuilt,
checksum-verified binary and never compiles. It matters for contributors, and for anyone on a
platform with no prebuilt asset, whom the installer deliberately refuses and redirects to a
source build.

**reflow2's own design is on flo2.io since 2026-10-10, and `.reflow2.toml` names it.** Reach it
with an `http` entry in your `.mcp.json`, `{"type": "http", "url": "https://api.flo2.io/g/reflow2/mcp"}`.
Your MCP client signs in through flo2.io, and you must be a member of the design; Anthony adds
members. reflow2's own client cannot sign in to flo2.io yet
(`epoch:planned-reflow2-signs-in-as-an-oauth-client`). So the launcher below, started in this
folder, attaches nothing, opens no local store, and serves one tool that says why. It remains the
way to serve a LOCAL store, which is the break-glass path in "The design record" below.

**The MCP server this repo ran on its own design graph is launched via
[`tools/reflow2-mcp-launch.sh`](tools/reflow2-mcp-launch.sh)** (wired in `.mcp.json`), not a raw
binary. **Since 2026-09-12 it serves `target/release/reflow2-mcp` and does not build it**
(`req:a-session-runs-a-release-binary`). Measured on the same bytes with only the profile
differing: a `propagate_from` walk 5,851 ms → 62 ms warm — 93× — and `design_regions` 12.5 s →
2.0 s warm. Every cost number in the 2026-09-10/11 tool sweep was taken on the debug binary,
because that is what this wrapper served.

**Rebuilding is now a deliberate act: [`tools/reflow2-rebuild.sh`](tools/reflow2-rebuild.sh).**
A release build here runs ~34 minutes cold, dominated by `librocksdb-sys` compiling C++ from
source, so a wrapper that rebuilt on every source change would make any edit a half-hour cold
start on the next session. The launcher therefore **refuses** when the release binary is missing
(naming the command) and **serves-with-a-banner** when the binary is older than your source. For
the edit-compile loop, `REFLOW2_PROFILE=debug` restores the old behaviour exactly: content-hash
gate, automatic `cargo build`, refusal to serve a broken build.

> ⚠️ **NOT REBUILDING BRINGS BACK THE FAILURE THE AUTO-REBUILD WAS ADDED FOR**, which is why the
> staleness banner is loud and unconditional. On 2026-08-16 a stale binary answered
> `describe_schema` with the pre-#199 `change_type` enum while main already declared `defect_fix`,
> and the session nearly recorded the exact fiction #199 was merged to end. Nothing in a tool's
> reply says which build answered it — `graph_report`'s `served_by.stale` is the second guard, and
> it reads the running process rather than this document.

The content hash covers sources, `schema/*.yaml` and `getting-started/skills/**/SKILL.md`, not
mtimes, on purpose: `cargo build` keys on mtimes, and `git pull --rebase`/checkout can leave
sources *older* than the last-built binary — which silently served stale code for whole sessions
until this was added. All wrapper output goes to stderr; stdout is the JSON-RPC channel and must
stay clean.

**A cut ENDS by rebuilding the binary this box serves — step 9 of `flow:release-cut`.** The
launcher's banner only fires inside this repo. Every *other* project on this machine reaches
reflow2 through the user-scope registration in `~/.claude.json`, which points straight at
`target/release/reflow2-mcp` with no wrapper in the path, so nothing tells those sessions the
binary is behind. Measured 2026-09-18: a full adopt of a real building design ran on 0.62.0 for a
whole session while 0.64.0 was tagged and published, because the two cuts in between rebuilt the
debug binary for the export and never touched release. So after the tag's assets are verified
(step 8), run `tools/reflow2-rebuild.sh`, then stop every shared daemon (`--stop-shared` in each
project, or a full restart of those sessions — a `/mcp` reconnect does not do it, see below) and
confirm with `pgrep -fa target/release/reflow2-mcp` plus `ls -l /proc/<pid>/exe`: `(deleted)`
means that server is still the old build. The rule is delivered on `release_report`, the step
before it (`rule:a-cut-ends-by-rebuilding-the-served-release-binary`).

> ⚠️ **`/mcp` RECONNECT DOES NOT PICK UP YOUR REBUILD, AND THIS PARAGRAPH SAID IT DID UNTIL
> 2026-08-17.** The old wording — *"you must `/mcp` reconnect reflow2 (or restart the session) for
> the live tools to pick it up"* — is wrong wherever a **shared server** is running, which is the
> normal case here. `--shared` re-attaches to the long-lived `--serve-shared` daemon, and the
> daemon keeps executing the image it started from. Reconnecting spawns a fresh *client* against a
> stale *server*, so it looks like it worked and changes nothing.
>
> **MEASURED THE DAY THIS WAS CORRECTED.** `heal.rs` was widened, the binary rebuilt at 22:13, and
> a `/mcp` reconnect at 22:16 spawned a new `--shared` client — while the `--serve-shared` daemon
> from 18:18 went on serving. `detect_defects` answered `total: 20` from code no longer on disk,
> and said nothing about it. Anthony reconnected *specifically* to make the change live.
>
> **THE ACTUAL REFRESH** is to stop the daemon, then make any tool call (it respawns from whatever
> is at the path):
>
> ```bash
> ./target/release/reflow2-mcp --graph-path ./.reflow2/graph --stop-shared
> ```
>
> `crates/reflow2-mcp/src/service.rs`'s own `STALE_NOTE` has said this correctly since 2026-08-11
> — *"A SESSION RESTART ALONE, WITHOUT `--stop-shared`, CHANGES NOTHING"* — and this file
> contradicted it. **A running server is the authority on its own currency, not this document:**
> `graph_report`'s `served_by.stale` is the check (`true` = every computed number came from a
> replaced binary; `null` means it could not tell, which is never `false`). Note that
> `loop_status` and `detect_defects` do NOT carry that block, so the debt and defect numbers a
> session actually reads can be stale with nothing saying so — open under
> `req:a-report-says-what-it-swept-and-whether-its-checks-ran`.

## Working on this repo

**Order of operations.** `git pull --rebase`, then `claim_report` to see what is held, then
`mint_seat` once and `claim_region` to take your item — *before* the work, because a claim nobody
can see is not a claim. Release it with `release_claim` when you finish; if you forget, liveness
is computed from your session, so the claim reads `gone` rather than sitting there looking taken.
Then ask the graph what is open and why: the graph — `loop_status` for what the loop owes, `detect_gaps` for the open questions, `search_design` to find a past finding by its words. [COORD.md](COORD.md) covers
resolving merge conflicts on the shared records without discarding anyone's work; read that before
you hit one.

**Branches.** `feat/<short-name>` off `main`, one per claimed item where practical.

**No merge driver to set up.** The committed design is the item layout — one file per node and
one per edge — so git's ORDINARY merge merges it: two people editing different parts of the
design touch different files and merge with no conflict, on GitHub as anywhere
(`dec:how-the-saved-design-is-laid-out-so-git-merges-it`). The `.gitattributes` line that pointed
the single file at `merge=reflow2` came out on 2026-10-03. `reflow2-mcp --merge-driver` still
exists for a project whose design is still a single file, and is optional there. A real conflict
is one item both sides changed, shown as that item's file. **Never resolve a design conflict
with `--ours`/`--theirs` without reading both sides**: for code that drops a hunk, for a design
it drops what someone wrote and nothing will tell you it is gone.

> **`--no-fail-fast` is not optional and not a nicety.** `cargo test` ABORTS EVERY REMAINING TEST
> BINARY the moment one fails, and this workspace had ~155 of them behind a ~13-minute run (the
> core's test files are still one program each; the MCP crate's are four programs since 2026-10-15). So a
> change with three independent failures costs three full cycles — you fix one, wait, discover the
> next, wait again — and on CI that is three push-and-wait rounds for information one run already
> had. **Measured 2026-08-25 while adding one detector**: successive runs reached 31, then 47, then
> 114 suites, surfacing exactly one new failure each time; all four were in `reflow2-core` and one
> `--no-fail-fast` run would have shown all of them at once. The cost is that a RED run now takes
> full wall-clock instead of stopping early, which is the right trade: a red run is already a
> failure, and what you want from it is every fact it has, not the first one.

⭐ **TO RUN THE WHOLE SET, DO NOT WRITE A SCRIPT — RUN `python3 tools/run_ci_gates.py`.** It
executes exactly what `ci.yml` runs, in that order, reading the list through the same parser
`skill_lint.py` uses rather than keeping a fourth copy of it. `--list` shows them without running
anything, and a keyword argument narrows it (`run_ci_gates.py reflow2_check.py` re-runs just the
export-dependent gates after an export).

🛑 **THE REASON IT EXISTS IS THAT THREE SESSIONS HAND-WROTE THIS AND GOT THREE DIFFERENT SUBSETS.**
The worst ran 17 of 48, called `render_skills_and_tools` without `--check`, ran both clippy
invocations without `-D warnings`, and piped `cargo test` through `head -40` — which truncates the
report at the fortieth of roughly two hundred test binaries and can kill the run through the closed
pipe. Forty consecutive passes and no failures is indistinguishable from a clean full run, and is
the more reassuring of the two readings. The block below is the everyday subset a person picks
from; the runner is how you check the lot.

**A change is done when all of these are clean** — the everyday subset, with the flags CI
actually uses. **Every clippy line carries `-D warnings`, because that is what turns a local
warning into a red build.** **Locally, `cargo dev-test` is the test gate** — both crates without RocksDB, one command. The two
lines marked *with RocksDB* are CI's full job, which runs them on every pull request; run them
locally (`cargo full-test`) only when your change touches the store on disk. Touched checks
locally, everything once in CI.

```bash
cargo test --workspace --no-fail-fast                    # both crates, with RocksDB — CI's full job
cargo test -p reflow2-core --no-default-features --no-fail-fast   # the in-memory backend, alone
cargo test -p reflow2-mcp --no-default-features --no-fail-fast    # the MCP crate in memory, no RocksDB
cargo clippy -p reflow2-core --no-default-features --all-targets -- -D warnings
cargo clippy -p reflow2-mcp --no-default-features --all-targets -- -D warnings
cargo clippy -p reflow2-mcp --all-targets -- -D warnings          # with RocksDB — CI's full job
cargo fmt --check
python3 tools/validate_schema.py                         # after any schema/*.yaml edit
python3 tools/check_discrimination_rules.py               # the schema's discrimination rules vs capture-intent's routing table — one contract, two records
python3 tools/smoke_mcp.py                               # after any tool-surface change
python3 tools/empty_speaks.py                            # an empty answer says WHICH empty — no bare zero on the wire
python3 tools/refusal_speaks.py                          # every argument refusal names the TOOL and the field path — probes generated from the schemas
python3 tools/toolsnap.py                                # tool schemas vs committed goldens; --update to bless
python3 tools/every_tool_carries_a_title.py              # every served tool carries a title; every writer says whether it can destroy
python3 tools/tool_confusability.py --validate           # the offline find_tools replica ranks as the live one does
python3 tools/replies_are_bounded.py                     # a reply that outgrows the budget must offer a bound — and this FAILS if nothing overflowed
python3 tools/a_reply_is_sent_once.py                    # every structured reply carries ONE payload and ONE signpost, asked of the whole surface
python3 tools/skill_lint.py                              # after any skill or tool-surface edit
python3 tools/test_run_ci_gates.py                       # the gate runner's own net — it must never report a pass having run nothing
python3 tools/test_wall_check.py                         # the wall-check instrument's own net
python3 tools/test_run_to_files.py                      # a real test run becomes per-file outcomes reconcile_verification reads
python3 tools/test_latent_promotion.py                  # the latent server promotes itself in place, on any client (real binary)
python3 tools/test_content_policy.py                    # a reply takes the shape its client can read (per-client content policy, on the wire)
python3 tools/test_opencode_plugin.py                   # the OpenCode loop-nudge plugin, driven as OpenCode drives it (node)
python3 tools/test_call_door.py                         # the --call door and the VS Code terminal route, driven as an agent drives them (real binary)
python3 tools/reflow2_check.py --export docs/design/reflow2/   # design vs build per change, and per-item integrity and lineage
python3 tools/check_intent_authority.py docs/design/reflow2/    # settled intent carries the owner's name
python3 tools/check_consumer_reach.py docs/design/reflow2/     # a capability claiming reach beyond this repo is realized by something a consumer gets
python3 tools/check_verification_ratchet.py docs/design/reflow2/ # a capability does not NEWLY claim a passing check nothing can re-run
python3 tools/vocabulary_reach.py --check                # a NEW declared property the surface cannot write
python3 tools/check_command_surface.py                   # the skill/command copies still agree
python3 tools/the_served_surface_names_no_real_person.py   # no served text names a real person from this design
python3 tools/changelog_fragments.py --check             # every CHANGELOG fragment is well formed, and nobody wrote into [Unreleased] directly
python3 tools/test_item_layout_merges.py                 # two PRs' design records merge with plain git in either order, lineage stays one hop (real binary, real branches)
python3 tools/launch_serves_release.py                   # the launcher serves release and builds nothing there
python3 tools/render_skills_and_tools.py --check         # docs/skills-and-tools.md rows and counts vs the served surface
```

> **This list is a SUBSET and `ci.yml` is the authority.** The full job also runs the instruments
> and the Python suites — `phase_trial`, `coherent_erosion_trial`, `model_the_loop`,
> `stateless_seat_probe`, `test_init`, `test_shared_sessions`, `test_merge_driver`,
> `test_degraded_server`, `test_nudge_path`, `test_loop_nudge`, `test_render_views`,
> `test_stale_seat`, `test_reflow2_check`, `test_impact_of_diff`, `test_why_history`, `check_doc_versions`, `test_check_doc_versions`, `test_site_counts`,
> `test_skill_lint`, `self_host_uses_documents`, `test_check_intent_authority`, `test_check_consumer_reach`, `test_check_verification_ratchet`, `test_vocabulary_reach`,
> `test_export_to_reaches_the_daemon`, `test_shared_attach_from_another_folder`, `test_one_meaning_for_an_artifact_location`, `test_feedback_is_a_computed_tally`,
> `test_a_lesson_is_served_at_the_step`, `test_release_workflow`, `test_dependency_currency`, `test_changelog_fragments`,
> `test_reflow2_design_copy` — so **green here is not green
> there**, and *"believe CI"* below is not a figure of speech. Run the ones your change touches;
> [docs/sharpening.md](docs/sharpening.md) says which instrument covers what.
>
> **KEEPING THESE TWO LISTS IN AGREEMENT IS NO LONGER MANUAL (BL-159).** `skill_lint.py` now fails
> the build if any `cargo`/`python3` gate in `ci.yml` appears in neither this block nor the
> sentence above, if a listed command is spelled differently from the way CI runs it (**flags
> included** — that is the exact defect that filed BL-159: the `-p reflow2-mcp` clippy line was
> missing entirely and the `-p reflow2-core` one lacked `-D warnings`, so following this block
> exactly still produced a red build), or if either list names a gate CI has stopped running.
> **So edit these two lists together with `ci.yml`, or the lint will say so.** Adding a gate to
> `ci.yml` means adding it here — to the block if a developer should run it every time, to the
> sentence above if not. Both answers are honest; silence is the one that is not.

**The design record: one file per node and per edge, exported as often as you like.** Since
2026-10-03 (`dec:how-the-saved-design-is-laid-out-so-git-merges-it`,
`dec:the-designs-lineage-is-kept-per-item`, Anthony) the committed design is the ITEM LAYOUT at
`docs/design/reflow2/`: `nodes/<Type>/<escaped-id>.json`, `edges/<xx>/<hash>.json`, and a small
`design.json` holding only `graph_id`, `schema_version` and `migrated_from`. It replaced
`docs/design/reflow2.json`, whose whole-file header (`content_hash`, `prev_content_hash`,
`taken_at`) every export rewrote — so every pair of concurrently open PRs conflicted on it (46 of
46 in #640–#668, 39 of them on nothing else), and a queue of approved PRs landed one at a time.

What that means for a pull request:

- **Export as often as you like, in any commit.** `export_graph` with a path ending in `/`, or
  `--export-to docs/design/reflow2/` on each `--call` write, rewrites only the files whose item
  changed. There is no "once, last" rule and no `chained_from` to read.
- **Lineage is per item, and anchored for you.** A changed item records, in `prev_item_hash`, its
  hash at the merge-base with the default branch; an unchanged item keeps main's file byte for
  byte. A squash-merge lands each changed item one hop, and an export taken mid-merge anchors
  where the merge will land (the merge-base of main with `HEAD` and `MERGE_HEAD`), so it cannot
  mis-chain the way #571's fourth CI failure did. Every export also re-checks the lineage of the
  items the branch has already changed, so re-exporting repairs one a merge left stale. The gate
  checks it (`LINEAGE`, per item).
- **To bring main in: `git merge origin/main`.** No rebuild, no replay onto main's export. Two PRs
  that changed different items merge in either order with no conflict. A conflict is one small
  file named after an item both sides changed — a real conflict. A file with conflict markers in
  it is not an item, so first take one side of THAT file (`git checkout --theirs -- <file>`, after
  reading both), then import the layout into your store, write what the item should say, and
  export — before or after committing the merge.
- **An accepted checksum rides the change that accepted it** — `checksum_after` on the change's
  CHANGED edge, numbered by `accepted_seq` — so two PRs that edit one file each write their OWN
  edge (decision 3 of `dec:item-13-checksums-move-to-change-edges-and-main-converts-in-one-pr`).
  `set_artifact_checksums` is called exactly as before. The gate's design-vs-build check is
  git-aware: on a PR, every registered file the PR changed must be covered by an acceptance in
  the PR that matches the file at the PR head; on main, every registered file a commit changed
  must be covered by an acceptance in that commit. A file two PRs both edited is the merge of two
  accepted changes: the gate notes it, and the server's own reconcile reports it until somebody
  accepts the merged content.
- **A PR need not be up to date with main to merge** (decision 4). Main's push CI catches a bad
  combination, as it does for code.
- **`taken_at` lives in `docs/design/reflow2/taken_at.json`, which the layout git-ignores**
  (decision 5). The whole-design content hash is computed on read and equals what the single file
  stated for the same design, so watch baselines and release pins survive the move.
- **The CHANGELOG entry is a fragment**, `changelog.d/<slug>.md` ([changelog.d/README.md](changelog.d/README.md)).
  The cut assembles them with `tools/changelog_fragments.py --cut`.

**One release of overlap.** Every reader takes either form — `--import`, `import_graph`,
`--diff`/`compare_designs`, `--merge`, the upstream watch, `fork_point`, and every Python gate
and tool through the one loader, `tools/design_io.py`. A `.json` path is still written as the
single file, whose chain anchors at the same merge-base and whose gate checks are unchanged.

> ⚠️ **The claim board's old tension is gone with the layout.** A claim is graph state, and the
> single file could only be written once per PR, last — so a claim made before the work was
> invisible until the PR merged (`dec:a-graph-claim-cannot-be-published-before-its-pr`). In the
> item layout a claim is one more item: export it in the PR's first commit and push.

**Since 2026-09-12 the server also keeps the committed design current on its own** —
`--export-to <PATH>` (a directory ending in `/` for the item layout, a `.json` file for the single
form), which this repo's `.mcp.json` and every project `reflow2_init.py` installs now pass. After each
change the write-through waits for two seconds of quiet (ten at the outside, so a steady stream of
writes cannot starve it) and exports once, so a burst of thirty writes costs one export and a
forgotten export stops being a class of loss. The guarantee lives in the server because **the
session is the thing that might not come back**: the Stop hook that used to carry it fires once and
exists in one harness.

**Since 2026-10-03 a writing `--call` does the same before it exits**
(`req:a-writing-call-keeps-the-committed-export-current`): the door installs the same
write-through with no background task and runs it once after a successful write, on the file
`--export-to` names or, without it, the one the project's MCP configuration names for that store.
A worktree has no MCP configuration (it is git-ignored), so pass `--export-to docs/design/reflow2/`
on each `--call` write there, or `export_graph` to that path when you are done.

> 🛑 **IT WILL NOT OVERWRITE A HAND EDIT, AND THAT MEANS IT CAN STOP.** If the file no longer
> matches what reflow2 last wrote — you edited it, or a merge left it unparseable — the
> write-through declines and **`loop_status` says so in `next`**, not in a log line nobody reads.
> Resolve the file and it resumes. Your own `export_graph` never looks like tampering: the baseline
> moves at the shared file-write seam, so a deliberate export is still yours to make whenever you
> want one. A read-only server is refused the write-through outright.

**Since 2026-10-10 reflow2's design is on flo2.io: write there, copy last, one design PR at a
time.** It moved in phase 2 of `dec:idea-reflow2s-own-design-moves-onto-flo2-io-or-stays-local`
(option c1, Anthony 2026-10-09). The one live design is `reflow2` on flo2.io. The paragraphs above
still describe the committed copy: its layout, its per-item lineage, and how it merges. The local
store and write-through they describe are now the break-glass path at the end of this section.
`docs/design/reflow2/` is a copy of the live design for CI, refreshed by
`tools/reflow2_design_copy.py`:

1. **Hold the design before your first design write.** Run `python3 tools/reflow2_design_copy.py hold`
   on your branch. It compares the hosted design with main's copy and says whether you may begin.
   Then claim it as it tells you: `claim_region` on `proj:reflow2`, depth 0, note
   `design PR: <your branch>`. While you hold it, every other branch's `hold` and `copy` refuse.
2. **Write through flo2.io.** Your writes are live at once, before anyone reviews the PR. Undoing
   one is a new write, not a closed PR. What a ship records: the capability's status, the new
   files as Artifacts, the check that passed, the ChangeEvent. **And when the shipped work answers
   a recorded idea** (a `proposed` exploratory Decision), draw `EVOLVES_INTO` from that idea to the
   requirement or capability it became, and leave the idea's status for the owner: closing it is
   Anthony's word, and the edge is what lets `decision_overtaken_by_promotion` put it to him.
   Measured 2026-10-13, 47 of the 59 open ideas found already built carried no such edge
   (`fact:root-cause-built-work-stays-open-because-its-record-is-kept-by-hand-and-the-ship-moves-only-the-capability-2026-10-13`).
3. **Copy last.** Run `git merge origin/main`, then `python3 tools/reflow2_design_copy.py copy`, and
   commit `docs/design/reflow2/`. It prints what your copy carries; read it.
   - It refuses while another branch holds the design.
   - It refuses while writes nobody holds are in flight. `--adopt` takes them into your PR
     deliberately.
   - It refuses until your branch contains main.
   - If you write again after copying (a CI fix, say), copy again.
4. **After the merge there is nothing to release.** Nothing is in flight, so your hold is stale
   and the next PR may hold.

What the choice costs, accepted with it:

- **One design-writing PR at a time.** No stacked design work. Code-only PRs are unaffected.
- **A write nobody holds rides in the next copy.** Intent written outside a PR (a hub session's
  ideas, a requirement added from claude.ai) belongs to no branch and is harmless to carry.
  `hold` and `copy` name it, so it is carried knowingly. Implementation facts, such as an accepted
  checksum or an artifact's location, belong to one branch's files; those are what the hold
  protects.
- **The guard binds only whoever runs it.** Claims are advisory (`crates/reflow2-core/src/claims.rs`),
  and the design records who wrote, not on which branch. A session that writes without holding
  is the one gap the guard cannot close.
- **A record that needs schema only your branch's build knows** cannot be written on flo2.io until
  a release carrying that schema reaches flo2.io. Record that part afterwards, or locally (below).
- **flo2.io does not serve every tool.** `wall_check` and the tools that open another design by
  path run locally, against the committed copy. Two of the cross-design tools no longer need a
  path there — see the next section.
- **An outside contributor cannot write the hosted design.** Their PR changes code. A maintainer
  records its design change on flo2.io and copies it in a follow-up PR.

### Cross-design reads by graph_id, and what a registry server refuses

**Since 2026-10-10 `compare_designs` and `compose_and_analyse` take the other design by
`other_graph_id`** — the first step of the cross-design layer
(`req:every-cross-design-operation-takes-its-other-designs-by-graph-id-under-the-hosts-read-filter`).
**Since 2026-10-11 every other READ of another design does too**: `certify_preservation`
(`other_graph_id`; the live graph is the base, as in `compare_designs`), `merge_designs`
(`other_graph_id` for theirs and `base_graph_id` for the common ancestor; ours is the live graph),
`analyze_alternatives` (`other_graph_ids`; the live graph is the baseline), and `pair_designs` /
`seam_report` (`other_graph_id` instead of the inline `design`, exactly one of the two). Each keeps
its path or inline argument, made optional where it was required, and refuses a graph_id together
with one. Each resolves through the same door, theirs before ours and one guard at a time — a merge
reads the ancestor, then theirs, then the live graph
(`tests/every_path_taking_read_takes_the_other_design_by_graph_id.rs`). `seam_coverage` reads only
this design and was never in the family. On a `--registry-root` server
the other design is read from the one the process already holds (`registry_http::RegistryHandle`,
opened on demand like any request, closed by the ordinary idle rules), `compare_designs` makes the
live graph the base labelled by this design's graph_id, and `compose_and_analyse` namespaces the
other design by its graph_id unless `namespace` says otherwise. The core did not change: both
operations needed only a `GraphExport`.

**The read is bounded by the set the host hands over, not by the root**
(`dec:a-cross-design-read-is-bounded-by-the-readable-set-the-host-hands-over-not-by-the-root-alone`,
which re-opened `dec:the-registry-root-is-the-tenant-boundary` for this case alone, because on
flo2.io one root holds every tenant). `ReflowService::other_design` is the one door, and its
refusals come in a fixed order: no registry ("served only by a registry server; pass a path
here"); this design's own graph_id; no trusted gateway or bearer gate (a bare registry reaches no
other design, whatever a call claims); and an id outside `_meta["reflow2/readable_graph_ids"]` on
THIS call — refused naming the id and the key, never saying whether such a design is here or what
it holds. **A gateway must send `reflow2/readable_graph_ids` on reads as well as writes**; the
flo2 gateway sends `_meta` only on writes today, so until it changes every cross-design read
through it is refused. The other design is exported under its own read guard, which is dropped
before this design's is taken — never two guards at once, because tokio's `RwLock` queues a writer
ahead of new readers and A→B against B→A would otherwise deadlock
(`tests/a_cross_design_read_takes_the_other_design_by_graph_id.rs` holds all of this against the
real binary, the deadlock case included).

**A registry server refuses a path.** Until 2026-10-10 a `--registry-root` server given
`compare_designs` with `base_path`/`other_path` opened that path on its own host and answered the
diff — the flo2 gateway's `REFUSED_TOOLS` was the only thing standing between a caller and the
droplet's filesystem. Since 2026-10-11 EVERY tool that opens a design, an export or a store at a
path a call names refuses it there, through one function
(`ReflowService::refuse_a_path_on_a_registry`, "a path is not served here; …" plus what works
instead): the reads above, `fork_point`'s `export_path` (this design's own record in git, so it has
no graph_id form — left out, the version comes from the design itself) and `describe_designs`'
`paths` (which named any design stored on the host, its graph_id and label, to whoever asked).
**The WRITES that take another design stay path-only, refused on a registry, until the two-lock
step**: `apply_merge` (`base_path`, `theirs_path`), `import_graph` (`path`; an inline `document`
still imports) and `external_dependency` (`design_export`, read for the baseline; `""`, which stops
a watch, is not refused). Taking theirs by graph_id while writing ours needs a lock order across
two designs that does not exist yet. A server that is not a registry — stdio, `--shared` — is
unchanged: the path there is the caller's own.

**What a registry still opens on the host** (open; read from the code on 2026-10-11, not yet
measured, and still behind the gateway's refused lists). Paths a call names that are not another
design: `export_graph`/`export_surface` WRITE this design to the `path` given, and `wall_check`
reads the checkout at `root` (or the store's folder). Paths the design itself stores:
`upstream_status` and `loop_status` read an `ExternalDependency.design_export` recorded before
this change (and `external_dependency` re-reads it on a re-declare that leaves it out), and
`loop_status { since_export }` reads the export paths in the sync sidecar.

### A hub's address serves a view over its members

**Since 2026-10-11 the hub's address IS the view** — layer 2 of the cross-design work, at the
tool layer, with no tool and no argument added (`crates/reflow2-mcp/src/service/view.rs`). The
host passes **`_meta["reflow2/view_graph_ids"]`** (`VIEW_GRAPH_IDS_META`, beside
`READABLE_GRAPH_IDS_META`) on every call to a hub's own design: a JSON array of graph_ids, the
hub's members ∩ the caller's readable set. `call_tool` reads it per call into the
`VIEW_GRAPH_IDS` task-local; **a call without the key goes straight to `call_tool_unshaped`, the
path every call took before, and its reply is byte-identical** (asserted against a registry that
never admits a view). A view is honoured only where `other_design` is — a registry server
(`served_from_registry()`) behind a trusted gateway or a bearer gate; anywhere else the hub
answers alone and `view_note` says why the view was not served. Each call is then one of:

- **Routed.** If any string anywhere in the arguments is `"<graph_id>:<rest>"` with `<graph_id>`
  one of the view's ids (an exact id, then `:`), the call is a MEMBER CALL: the prefix is stripped
  from every such string, the same tool runs on that member's own service
  (`RegistryHandle::member`, a fresh session on it for this one call), and the member's ids in the
  reply are prefixed back. The caller's `_meta` — `reflow2/writes_for`, `reflow2/acting_agent`,
  the readable set — and the verified token ride along, so a write lands in the member credited
  to the caller. **Ids of TWO members in one call are refused before anything runs:** *a write
  lands in one design; a relation across designs is a typed link record made in one of them — use
  `review_relations` with `other_design` and `other_id`.* A member outside the readable set is
  refused with `other_design`'s own sentence (`service::not_readable`): the id and the key,
  nothing it holds.
- **Fan-out read.** A tool whose served annotation says `readOnlyHint: true`, naming no member,
  runs on the hub and then on each readable member IN TURN, and answers
  `{"view": [...], "by_design": {"<hub>": …, "<member>": …}, "unreachable": [...], "view_note"}`
  with a text block saying it is a view. A member that fails is `{"error": …}` under its id, never
  dropped. **`search_design` is the one MERGED reply:** every design's hits ranked together by
  score, each member's `node_id` prefixed (the hub's own unprefixed), each hit naming its
  `design`, and `designs_searched` listed.
- **A write naming no member** writes the hub's own design exactly as before.
- **Not viewed** (`service::view::NOT_VIEWED`, one const with the reason per group): export and
  import, releases and the cut, claims and holds, seats, served text (`get_instructions`,
  `get_skill`, `list_skills`, `find_tools`, `find_skills`, `describe_schema`), and the
  cross-design tools that already name their other design. Naming no member, these act on the hub
  alone and say so in `view_note`. **An id naming a member routes them like any tool** (since
  2026-10-19: the routing step comes first, so `add_release {id: "<member>:rel:…"}` lands in that
  member instead of being written into the hub under its literal prefixed id).
- **Aimed: `member` runs ANY tool on one design** (`view::MEMBER_ARGUMENT`,
  `req:through-a-hub-every-tool-can-run-on-one-named-member`). Through a hub's address every call
  may pass `member`, the graph_id of a member or of the hub itself; the view takes it out of the
  arguments and runs the tool on that design alone — the tools that take no node id
  (`loop_status`, `reasoning_report` with no claim, `detect_gaps` with no scope, `describe_schema`,
  `get_skill`, `usage_report` …) and the not-viewed ones (`claim_region`, `release_claim`) included,
  and a `search_design` or `scan_nodes` then covers that design only. `tools/list` declares
  `member` on every tool ONLY when the listing request carries `reflow2/view_graph_ids` and the
  server admits a view; every other listing is byte-identical to what it was, so the toolsnap
  goldens do not move. `member` naming no design of the view, beside an id of another member, or
  on a call that names no view is refused before anything runs. `aim` is the one routing rule, and
  `every_served_tool_can_be_aimed_at_one_member` holds it over the whole served list.

**The prefix rule.** In a member's reply, a string under a key named `id`
or ending in `_id` (`node_id`, `from_id`, `to_id`, `target_id`, `subject_id`, `source_id`,
`other_id`, `artifact_id`, `decision_id`, `verification_id`, `capability_id`, …) gets
`"<graph_id>:"`; so does every string item of an array under a key ending in `_ids`; under
`drawn`, `edges_drawn` and `edges_already_present` each item is an edge sentence `"A REL B"`, and
A and B get it. A key ending in `graph_id`/`graph_ids` names a design and is left alone, as is all
free text; so is an id a cross-design record places in ANOTHER design — `<p>_node_id`
(`design_node_id`, `to_node_id`, `from_node_id`) beside a non-empty `<p>`, `<p>_design` or
`<p>_graph_id`. (Merged search also prefixes its `stale` ids.) **Since 2026-10-19, by value as
well as by key:** any whole string value that is an id the call passed with the member's prefix,
or a node id the member holds (looked up on the member, under its guard alone, after its call),
gets the prefix whatever its key — a step's `assumes`, `conclusion`, `discharges`, `cases`, a
summary list; and a refusal's text (an `isError` reply or a protocol error) gets the same ids
prefixed where they stand in its words. A successful reply's prose is still never rewritten: it
may be stored content, and a prefix pasted back inside prose would be stored with it.

**One design's guard at a time.** Every call the view makes is `call_tool_unshaped` on ONE design,
awaited to the end before the next starts, and it takes and drops that design's guard inside —
never two at once, never the hub's while a member's is taken. Two hubs viewing the same members in
opposite orders cannot deadlock on tokio's writer-preferring `RwLock`
(`tests/a_hubs_address_serves_a_view_over_its_members.rs` runs H1 over [A, B] against H2 over
[B, A], routed writes included, under a deadline).

**Reasoning crosses designs, at the same layer** (since 2026-10-19,
`req:reasoning-crosses-designs-within-a-hub`; `crates/reflow2-core/src/far.rs` and
`crates/reflow2-mcp/src/service/far.rs`). The core holds one design and opens no other, so the
registry is the assembler, and a local `--registry-root` behind a gateway or bearer gate gets it
exactly as flo2.io does:

- **A premise in another design reads as it reads there.** Before each call on a design whose
  reasoning names a reference `xref:<design>:<node>` (a premise, a conclusion or either end of an
  objection), `call_tool_unshaped` reads each far node IN ITS OWN DESIGN
  (`RegistryHandle::read_other_design`, that design's guard alone, before this design's is
  taken) and hands the readings to the call through `FarScoped`, which installs them in the
  core's per-call scope (`reflow2_core::far::scoped`) on every poll. The premise then carries
  the far truth; a far proof status short of `proven` keeps a confirmed line here from
  `proven`; what the far node holds only under there makes the claim here `conditional`. A far
  node that is gone reads `stale`, and a far design the caller may not read (the readable set on
  THIS call), or any server that reaches no other design, reads `unreachable`: unknown, never
  holding. Readings are per call and never stored: two callers may read different designs.
- **Through a hub, a change walk continues.** `propagate_from` and `propagate_change` called
  through a hub's address carry the radius on into each design of the view (`WALK_AMONG`, set by
  `view::serve`: the hub and the readable members) holding a reference to a node the walk
  reached, and on from there, each design entered once. The references are named in
  `boundary_crossings` with their design's prefix and each radius comes back under
  `across_designs`, whose ids already carry their own design's prefix and are skipped by the
  member prefixer (`view::ACROSS_DESIGNS`). A call that does not come through a hub's address is
  unchanged.

**What flo2 must send:** on EVERY call to a hub's own design, reads included,
`reflow2/view_graph_ids` (the hub's members ∩ what the caller may read) together with
`reflow2/readable_graph_ids` and `reflow2/writes_for` — and the same `reflow2/view_graph_ids` on a
`tools/list` of a hub's own design, which is what makes the listing declare `member`. A member it leaves out of the view is
simply not reached; one it names but leaves out of the readable set is reported `unreachable`
and refused when routed to.

> 🧯 **BREAK GLASS: WHEN flo2.io CANNOT SERVE reflow2's DESIGN.** flo2.io's engine IS reflow2, so a
> release that breaks opening designs also locks you out of the design you would read to fix it.
> Bringing flo2.io back is flo2's operator work (flo2's `ops/README.md`). Meanwhile:
>
> 1. **Work locally from the newest committed copy**, as before the move:
>    `./target/release/reflow2-mcp --graph-path <a fresh path> --import docs/design/reflow2/`.
>    Everything merged is there.
> 2. **Writes made on flo2.io since the last merged copy are not in git.** They are in flo2.io's
>    nightly backup, which archives every hosted design's store and is pulled hourly to the
>    operator's machine. Restore the store from the tarball (`designs/reflow2/` in flo2's
>    `ops/README.md`, "Restore"), export it with the binary flo2.io last ran, and
>    `python3 tools/reflow2_design_copy.py copy --from <that export>` brings it into a branch.
> 3. **When flo2.io serves the design again, local writes are not on it.** Write them again
>    through flo2.io, using the local record's ChangeEvents as the list. Replacing the hosted
>    store with a local one stops flo2.io's engine, so it is an operator act and needs Anthony's go.

**Since 2026-09-12 the server also keeps a usage ledger** — `<graph-path>.usage.jsonl`, beside the
store with the other sidecars — one line per tool call: the tool, the outcome class (`ok` /
`refused` / `error`), the refusal class when refused, the duration, the harness that connected, the
seat, and for `get_skill` alone the skill's name. **The verb, never the object**: no argument, node
id, statement or message is ever written (`req:telemetry-carries-usage-never-design-content`), and
`tools/test_feedback_is_a_computed_tally.py` proves it on the real binary with a leak mutation. The
unit is the project, not the session. `usage_report` renders the window since the previous report
(and leaves the marker that closes it); the served `feedback` skill — `/feedback` — is how a person
gets that tally plus the agent's per-row dispositions into the project's own `reflow2_feedback.md`,
which goes nowhere unless they carry it. An in-memory design has no ledger and says so.

**Since 2026-09-12 a lesson is served at the step it concerns** — a `DesignRule` or `TemporalFact`
may name the served skills and tools it is about in `steps` (`record_finding` / `add_design_rule`;
a name nothing serves is refused with the nearest). `get_skill` then carries the design's lessons
for that skill beside its body, and `tools/list` appends them to the named tool's description — the
moment before the call. An empty design serves the surface unchanged (the toolsnap goldens pin it).
The reason is measured: the same trap was written down three times and repeated; what stopped it
each time arrived at the step (`req:a-lesson-is-served-at-the-step-it-concerns`).

**CI enforces these on every push** (`.github/workflows/ci.yml`): a fast core job
(core tests, clippy `-D warnings`, fmt, schema, installer suite, skill lint) and a full job
(workspace tests, the smoke test and the exit-zero instruments against the real binary). Green
locally but red in CI means your local run skipped a gate — believe CI. The skill lint checks
the skills' *contract* (served tool names, mirrors byte-identical, frontmatter, the standing
rule); their semantic quality stays evidenced by trials, per docs/sharpening.md — deliberately
no LLM evals in CI.

### Branch, then PR — nothing lands on `main` directly

**Standing practice from 2026-07-31 (Anthony), starting after PR #3.** Work goes to a branch and
reaches `main` only through a pull request that CI has passed. No direct commits to `main`, and
no merge on a red or unrun build.

**The wrinkle, because the trigger config decides the workflow.** `ci.yml` runs `on: push` for
**`main` only**, plus `on: pull_request` for anything. So *a feature branch gets no CI at all
until a PR exists* — "wait for CI, then open the PR" cannot happen in that order here. The
sequence that gets the intent is:

1. Commit to the branch and push. (No CI yet. This is expected, not a problem.)
2. **Open the PR as a draft** — that is what starts CI.
3. Run the local gates meanwhile; a red CI on a branch you gated locally is a finding about the
   gates, not just about the branch (*"believe CI"*, above).
4. Mark it ready and merge **only when both jobs are green**.

A draft PR is the trigger, not a claim that the work is finished — which is exactly why the
draft state exists. Opening one early costs nothing and is the only way to learn what CI thinks.

**When a PR merges, its branch is deleted.** Pushing more commits to that branch afterwards
recreates it *with no PR and therefore no CI*, and they will sit there looking merged when they
are not. Follow-up work after a merge starts a new branch and a new PR.

**If you changed a detector, a phase capability, or anything in the coherence loop**, run the
instruments in [docs/sharpening.md](docs/sharpening.md) before and after, and add a probe for what
you built. A number that moves is your claim; a number that moves the wrong way is a finding. They
are not pass/fail gates yet — they record baselines that are failing on purpose.

Compiling is not the finish line, and neither is a green unit test. Drive the thing you changed:
the surface a user actually touches is the MCP binary, and three home-grown test layers once
agreed with each other and were all wrong because each was a client we wrote.

**Update the records in the same change, not afterwards** — this is the rule most often skipped,
and the records are the project's memory:

| Record | Update when |
|---|---|
| a fragment in [changelog.d/](changelog.d/README.md) (never `CHANGELOG.md` itself; the cut assembles them) | a user would notice |
| [docs/requirements-coverage.md](docs/requirements-coverage.md) | a status moves |
| the graph (a TemporalFact `defect`, a `planned` Capability, a `proposed` Decision, a DesignRule) | an item is discovered — see `dec:backlog-is-retired` for which shape |
| [docs/trials/](docs/trials/) | a real session went wrong — verbatim, append-only |
| `claim_region` / `release_claim` (the graph) | you start, and again when you finish |

**Claims made in the records must be evidence-backed.** If a backlog entry asserts a consequence,
it should be traceable to something someone observed — not inferred while writing the entry. When
you cannot source it, say so or strike it.

## Architecture

Reflow 2.0 is a graph-backed engine that keeps a design coherent across its lifecycle. Two
crates: **`reflow2-core`** is the deterministic, LLM-free coherence engine; **`reflow2-mcp`**
is the thin agent-native surface over it — the `reflow2-mcp` stdio binary a consumer actually
runs (the surface decision was made and built; see below). The core stays neutral to the
interaction surface (MCP / CLI / hosted) and to any LLM provider — those plug in at the seam,
not the centre.

### The store and schema (the foundation)

- The graph store is **in this tree**, at `src/foundation/` — the schema and `Value`
  vocabulary (`foundation/core/`), the RocksDB-backed store (`foundation/store/`), and the
  full-text index (`foundation/text.rs`). RocksDB is opt-in behind reflow2-core's own
  `rocksdb` feature, so the core runs on the in-memory backend for dev and tests.
  Graph-theory algorithms live beside it in `src/graphalg/`.
- It came from **[dynograph-foundation](https://github.com/sligara7/dynograph-foundation)**
  and was **absorbed from it at `v0.12.0`** — reflow2 no longer links anything from that
  repository (`dec:absorb-the-foundation-subset-and-end-the-dependency`, 2026-08-24).
  **Every absorbed module carries a provenance header naming the tag, files and commits it
  took**, because the recorded objection to absorbing anything is that vendoring turns a
  visible dependency into an invisible one — the pin carried a written reason for every bump
  and in-tree code has no successor to that record. Those headers are that successor, and
  `tools/check_doc_versions.py` reads the tag out of them rather than out of prose.
- The **schema is the vocabulary** (28 node types, 68 edge types across 11 `schema/*.yaml`
  domains): the node/edge names are load-bearing. `src/schema.rs` embeds all ten YAML files
  via `include_str!` and merges them with `Schema::from_multiple_yamls` — the same files
  `tools/validate_schema.py` checks, so there is one source of truth. Terminology in code
  must match the schema; `src/nodes.rs` holds the `node::`/`edge::` name constants.

### The design graph handle

`src/graph.rs` — `DesignGraph` wraps a `foundation::store::StorageEngine` scoped to one
logical graph id. It is the single handle everything else hangs off: generic
schema-validated CRUD (`create_node`/`get_node`/`create_edge`/`outgoing`/`incoming`/
`scan_nodes`/`delete_*`), typed golden-thread constructors (`add_project`,
`add_requirement`, `add_capability`, `add_component`, `satisfies`, `allocate`, `contains`),
and `pub(crate)` batch controls for atomic apply. Each coherence-loop step is a set of
methods on `DesignGraph` implemented in its own module (Rust lets `impl DesignGraph` span
files):

| Loop step | Module | Entry points |
|---|---|---|
| **CHANGE** (axis Z — never overwrite the past) | `src/temporal.rs` | `add_epoch`, `snapshot_node`, `add_change_event`, `record_change` |
| **PROPAGATE** (blast radius along the golden thread) | `src/propagate.rs` | `propagate_change` (reactive), `propagate_from` (speculative) |
| **DETECT** (find gaps to ask the human) | `src/detect.rs` | `detect_gaps` → `GapCandidate`s; `GapCandidate::to_prompt` (PROMPT half, via `LlmBackend`) |
| **HEAL** (fix structure the machine can) | `src/heal.rs` (+ `src/structure.rs`) | `detect_defects`, `propose_heal`, `apply_heal` |
| **LLM seam** | `src/llm.rs` | `LlmBackend` trait, `MockLlmBackend`, `complete_json` |

`src/structure.rs` builds a `graphalg` view (the "design network" — design nodes
joined by *traceability* edges) for HEAL's topology detectors.

### Load-bearing invariants (do not regress these)

- **No silent fallbacks / no silent drops** (AGENTS.md rule 4). This is enforced concretely:
  CRUD fails loud on unknown types / missing required props; PROPAGATE bounds depth but
  *reports* `truncated_beyond_depth`; HEAL moves un-appliable ops to `skipped_operations`
  with a reason; DETECT surfaces unknown seeds; the LLM PROMPT step degrades to raw wording
  with `rephrase_degraded = true`. New code must keep this bar; tests assert it explicitly.
- **HEAL is propose-then-apply.** `propose_heal` never mutates; `apply_heal` mutates
  atomically, is mode-aware (`rigid` project mode = propose-only), gates generated content
  behind `requires_human_review`, and does post-repair verification. Keep detection,
  proposal, and mutation separate.
- **PROPAGATE / structure exclude `CONTAINS`.** Decomposition is not traceability; including
  it makes the Project a hub that short-circuits distances. Impact and topology traverse the
  shared traceability set (`nodes::is_traceability_edge`). `INCLUDES` (Release → Artifact/
  Component) *is* in that set as of the v0.5.0 as-released work — a changed artifact reaches
  the releases that ship it, and a Release+Environment pair is no longer a disconnected island.
- **A release records everything it actually ships — including the documentation.** When cutting a
  release, `release_includes` must list every component that goes out, not a hand-maintained
  roll-call: anything built since the last cut raises `unreleased_component` until its release
  records it, and that gap is the reminder working rather than noise. `cmp:docs` is called out
  because documentation is part of the deliverable and it is the one people forget (Anthony,
  2026-07-24: "it'd be like if I designed a fighter jet using reflow2, but then didn't deliver any
  documentation on it to the user"). The nine releases up to v0.10.1 do **not** record documentation
  and are deliberately left that way — they genuinely shipped a README and a setup guide, but the
  design did not model documentation as a deliverable until 2026-07-25, and back-filling would
  reconstruct history rather than record it (`dec:intent-preserved`). Starts from the next release,
  on the user's call.
- **A release must be pinned to its epoch, and the design now says so itself.** `AT_EPOCH` is
  what puts a Release on the time axis; without it `changelog_point` resolves the release to a
  position with no sequence, so any window computed from it loses its lower bound and silently
  widens to the beginning of the design. This was a memory-only step and it failed twice —
  `rel:v0190` was cut without the edge four hours before `changelog_view` needed it, and
  `v0.17.0` still lacks one while `v0.18.0`'s commit message boasted of not repeating the
  fault. `release_without_epoch` now raises it per release (BL-122). **Do not "fix" a release
  by renaming things to match**: a matching name plus an existing epoch node is exactly what
  made the missing edge invisible, and only the edge counts.
- **A release must record what it shipped, and the cut ENDS by reading that record back.** An
  `INCLUDES` edge per artifact and component is what makes the as-released view exist; the
  schema has always said so (*"a Release with none is a version number, not a manifest"*) and
  nothing enforced it until 2026-08-21. `release_without_manifest` now raises it per release
  when a Release is pinned to an epoch, deployed, and includes nothing, **at severity 0.85 —
  which STOPS THE BUILD** (Anthony's call, 2026-08-21). It sits with the findings that say the
  design asserts something untrue rather than with the questions that wait for a human, because
  a note was demonstrably not enough: the v0.38.0 cut passed `reflow2_check` green with 96 notes
  scrolling past it. A genuinely contentless release — a re-tag, a docs-only republish — is
  still a real state, and stays acknowledgeable with a reason on the record. **The rule never
  consults `Release.status`** — `rel:v0380` was tagged, published, deployed and asset-verified
  while its status still read `planned`, so a status-based exemption would have excused the one
  case it exists for. The same clause was retrofitted to `release_without_epoch`, which had
  exempted any `planned` release since BL-122 and now exempts one only while it is undeployed.
  **The trap this closes:** `release_includes_all` defaults to a DRY RUN and answers
  `{"added": 304, "applied": false}` — pass `apply: true`, and finish the cut with
  `release_report`, which is the authoritative object. v0.38.0 was published with an empty
  manifest because `isError` was false, `reflow2_check` was green, and nobody read the object.
- **The whole cut is `flow:release-cut` in the graph — read it with `flow_report`, do not
  reconstruct it.** Eight ordered steps with their transitions, including the two backward
  "forces resync" edges that say what sends a cut back a step. It satisfies
  `req:a-project-records-how-it-ships`, and it exists because five separate cuts each shipped a
  different step wrong and recorded it only afterwards. **Order is load-bearing in three
  places**: accept the `reflow2.toml` version drift BEFORE freezing the manifest, rebuild the
  binary after the version bump and before the export, and export LAST because it costs the
  session its MCP access.
  - **Since reflow2's design moved onto flo2.io (2026-10-10), the cut writes its record there,**
    while flo2.io still runs the PREVIOUS release. Step 6 becomes the copy, taken by the cut's own freshly built binary,
    so the stamp and each item's lineage are the new release's:
    `REFLOW2_BIN=target/release/reflow2-mcp python3 tools/reflow2_design_copy.py copy`. Nothing is
    stopped and no session loses MCP. The cut is a design PR like any other, so hold the design
    first. Measured 2026-10-10 on the real design: an export from 0.81.0, imported and exported by
    0.81.1, came back with the identical content hash.
  - **The release notes say what is going and what has gone** (step 2, beside the changelog's
    assembly, `cap:a-cuts-release-notes-say-what-is-going-and-what-has-gone`; the retirement
    rule is `cap:a-served-item-is-removed-only-after-a-release-that-announced-it`).
    Before tagging, the release's opening paragraph and its `### Deprecated` and `### Removed`
    sections list every entry `deprecation::REGISTRY` deprecates in this release, each with its
    **What to do:** line, and every item this release removes (a tool removed now answers its
    old name with a refusal saying what to do, for this one release). Set each new entry's
    `deprecated_in` to this release's version; `every_deprecation_is_announced_in_the_changelog`
    checks the assembled section. **Telling the people who rely on the MCP server is owed before
    the release ships** — a deprecation nobody hears about is a removal with extra steps.
    Anthony decided how on 2026-10-14
    (`dec:anthony-tells-the-mcp-users-of-a-deprecation-before-the-release-ships`):
    - The agent cutting the release drafts a short note for Alex from this release's
      `### Deprecated` and `### Removed` entries and their **What to do:** lines, in plain words,
      and gives it to Anthony.
    - Anthony sends it, however he normally reaches Alex. On his request the agent may put it
      in his Gmail as a draft; the agent never sends it.
    - **The release is not tagged until Anthony says he has told Alex.** A release that
      deprecates and removes nothing owes no note.
- **Structural topology detectors are selective.** A design's golden thread is tree-shaped,
  where every internal node is a naive articulation point — so `single_point_of_failure`
  only fires when a node separates ≥2 real subsystems (see `structure.rs`).
- **Dependencies stay at or near their latest releases, and a bump is proven, not assumed.**
  This replaced the 2026-08 rule "do not bump the `rocksdb` pin as housekeeping", which held
  every dependency to "only when a change needs it". Anthony accepted the replacement on
  2026-10-02 (`req:reflow2-keeps-its-dependencies-at-or-near-their-latest-releases`). What it
  asks of you:
  - **Patch and minor releases** inside a declared range are taken promptly, with a lock refresh.
  - **A new major or new 0.x series** gets its own PR that adapts reflow2's code. "Adapting is
    work" is never a reason to stay behind.
  - **A held-back dependency is recorded in [`dependency-holds.toml`](dependency-holds.toml)**,
    with a reason and a date to look again. Real reasons: an upstream regression; a protocol
    revision adopted deliberately rather than inherited by a bump
    (`req:reflow2-conforms-to-the-mcp-specification-revision-it-serves`); a platform pin made on
    purpose, such as the `ubuntu-22.04` runner and image base kept for glibc reach. A bump that is
    merely in flight is a hold too, naming its PR. Remove the entry when the bump lands.
  - **A bump that touches stored data or the release machinery proves it before merge.**
    - For stored data, existing stores open unchanged under the new build, AND the last release
      opens what the new build wrote. Rollback is checked, not assumed.
    - For the release machinery, a `release.yml` `dry_run` dispatch passes.
  - **The Rust floor (`rust-version`) is stable minus one**, Anthony's choice of 2026-10-02.
    Release binaries are always built on current stable; the floor is the oldest compiler a
    source build may use, and raising it is what lets the code use a newer language or library
    feature.

  **It is checked by `.github/workflows/dependencies.yml`, monthly and before every cut** (a
  cut's step 1 changes `Cargo.toml`, which triggers it).
  - Its `currency` job (`tools/dependency_currency.py`) reads every Cargo.toml dependency, every
    GitHub Action and the floor against their latest releases. It FAILS on anything behind a new
    major or 0.x series with no hold, on a hold past its date, and on a lookup it could not make.
    It reports a newer release inside a declared range as a lock refresh owed.
  - Its `floor` job builds the whole workspace at the declared `rust-version`, so the number is
    true rather than claimed.

  The cost the old rule rested on is still real, and it is now a reason to batch, not to stay
  behind: moving `rocksdb` forces a full `librocksdb-sys` C++ rebuild (~10 min) on every machine
  that pulls. `rocksdb` is the same crate it was absorbed as. Upstream resumed releasing, so the
  "historically unmaintained" premise for switching to `rust-rocksdb` has moved
  (`fact:the-unmaintained-premise-under-the-rocksdb-switch-has-moved`). That switch stays parked
  (`dec:the-rust-rocksdb-switch-is-parked-until-the-store-question-settles`).
- **A storage-format change is a data-migration question, not just a code change — and
  absorbing the store made it EASIER to make one by accident.** Nothing is stamped on the graph
  directory — not a schema version, not a foundation tag — and validation runs on write, never
  on read. So a change to `foundation/store/keys.rs` or value serialization could misread an
  existing store with nothing to detect it, and an additive schema change leaves mixed-vintage
  nodes rather than backfilling (`foundation/store/engine/tests.rs` pins that behaviour:
  defaults apply on create, not retroactively). This used to be gated by the friction of
  bumping someone else's pin; now it is an ordinary edit in this repo. Before touching either,
  ask what happens to a graph written by the previous version. See **BL-19**.
- **A schema NARROWING ships its migration, and a test fails until it does**
  (`dec:idea-stored-data-is-rechecked-against-the-current-schema`, Anthony 2026-10-02). A
  narrowing is anything the write rule now refuses that the last release accepted: an endpoint
  dropped, an enum value removed, a type retired, a property newly required with no default.
  `tests/a_schema_narrowing_ships_its_migration.rs` diffs the schema against
  `schema/accepted-at-last-release.json` and fails until `narrowing::NARROWINGS` names each one
  with its migration — `Rewritten` (an `EDGE_REWRITES` row, applied on open and on import),
  `RefusedByName` (the import and `detect_defects`' `refused_by_schema` name what fits), or
  `Retired`. **At a cut, after the version bump, re-bless the snapshot**:
  `REFLOW2_BLESS_ACCEPTANCE=1 cargo test -p reflow2-core --no-default-features --test
  a_schema_narrowing_ships_its_migration` — it refuses while anything is unaccounted for. Stored
  data is now re-judged by the same write rule on demand: `detect_defects` reports every stored
  item the current schema refuses, before the import that would fail.
- **The MCP surface shrinks only through a deprecating release** (Anthony, 2026-10-05: *"don't
  break it for how it was originally used/designed as an mcp server"*; amended on his word
  2026-10-13: *"never removed" becomes "never removed WITHOUT A RELEASE OF NOTICE"*,
  `req:the-mcp-server-keeps-working-as-is-and-door-improvements-are-additive-or-opt-in`).
  `tests/the_mcp_surface_only_grows.rs` (name kept for its history) compares the build with
  the LAST RELEASE's tools, argument schemas, the reply shapes of a fixed scenario, and its
  vocabulary — node types, their properties, edge types
  (`tests/fixtures/mcp_surface_at_last_release.json`). Anything taken away fails — a tool, an
  argument, an allowed value or type, a reply field, a call the release answered, a node type,
  a property or an edge type — **unless the record marks it deprecated**, which means that
  release announced it. An argument un-advertised but still accepted, or a value set newly
  published where the release refused other values anyway, is listed there with its proof.
  **At a cut, after the tag, re-bless it from the release binary**, which writes the release's
  deprecations into the record so the next release may remove them:
  `REFLOW2_BLESS_SURFACE=target/release/reflow2-mcp cargo test -p reflow2-mcp --test in_memory
  the_mcp_surface_only_grows::` (default features: it drives the release binary on a store).
  - **To retire anything a client can call or read, it takes two releases.** First add its
    entry to `deprecation::REGISTRY` (`crates/reflow2-mcp/src/deprecation.rs`, the one place
    a deprecation is declared: the item, the version that deprecates it, what to do instead
    in one sentence, and the `dec:` that retires it) and a `changelog.d/` fragment that names
    it under `### Deprecated` with a **What to do:** line —
    `tests/every_deprecation_is_announced_in_the_changelog.rs` fails without it. That release
    still serves it, and says it is going: the tool's or argument's description opens with
    *"DEPRECATED in <version>, removed in the next release: <what to do>"*, describe_schema
    marks a node type, property or edge type `deprecated` and lists every entry under
    `deprecations`, and a call that uses it gets a `deprecation_notice` in its reply. Only the
    NEXT release may remove it.
  - **A tool goes one step further** (`dec:a-tool-is-never-deleted-without-usage-evidence-across-projects-and-never-without-a-refusing-stub-first`):
    in the release that removes it, keep its registry entry, so its old name is refused with
    what to do, and take the entry out in the release after. The surface test asserts the
    refusal. Anything else leaves the registry with the item.
- **Deterministic ids.** Gap/heal issue ids are a stable FNV-1a hash of
  `source + sorted affected ids` (not `std` `DefaultHasher`) so they're reproducible for
  dedup/caching.
- **`LlmBackend` is sync and object-safe.** The core holds `&dyn LlmBackend` and never names
  a provider. Typed JSON parsing is the free function `complete_json`, kept off the trait to
  preserve object safety. Build and test new LLM-reasoning ops against `MockLlmBackend`; do
  not add an async runtime or a provider dependency to the core.

### What's deliberately not here yet

One decision remains deferred — **real LLM provider backends** (unneeded on the agent-native
route: the ambient coding agent *is* the LLM). Still unbuilt: **SME augmentation**,
**generative HEAL content** (proposals stay review-gated stubs), and the optional **embedding
seam** (semantic dedup/retrieval). **SP-3b landed 2026-07-27** — `ingest_step` drives the
extraction pipeline with the calling agent as the model, so INGEST is finally reachable from a
session rather than only from a test. See the coverage
matrix for the exact deferral list. Everything else in the loop — the MCP surface, GENESIS,
INGEST's core, the consumer kit, search, the reconcile family — is built and shipping as of
v0.89.0.

---

## What Reflow 2.0 is

A graph-backed system that partners with an LLM agent to **design and build anything** —
software, hardware, a document, a full acquisition program. It captures the **entire
lifecycle of a design (concept → operations) in one knowledge graph**, tied together by
the systems-engineering *golden thread* (traceability from every artifact back to the
intent it serves).

The payoff: when **anything changes in any phase**, the ripple effects are automatically
found, surfaced to the user as plain questions, and healed back to coherence — so concept
through operations always stays in agreement. **The user never needs to know systems
engineering; the graph does.**

This is a clean-room rebuild ([github.com/flo2-connector/reflow2](https://github.com/flo2-connector/reflow2))
of ideas from the author's earlier projects (all under
[github.com/sligara7](https://github.com/sligara7)): `reflow`, `storyflow`,
`chain_reflow`, and the graph engine `dynograph-foundation`.

### What it is for, and what it is not — **reflow2 for *why*, tests for *whether*.**

The sharpest statement of this came from outside, and it is worth keeping in the owner's words.
A project designed end to end through reflow2 (2026-07-31) ran sixteen sessions of physics that
kept overturning itself, and ended with a six-deep chain of superseded Decisions — five
abandoned positions, each carrying why it failed — which they judged the payoff: *"no code
review, git history or comment thread would give a reader that. This is what the graph is for."*

And the counterpart, stated just as plainly: *"the one thing the graph could not do at any point
was tell me a number was wrong. That needed sources and executable checks."* Their division held
across the whole arc — **reflow2 is where reasoning persists; tests are what stop you being
wrong. They are complementary, and it would be a mistake for either to try to be the other.**

Carry it as a design constraint, not a slogan. It is the reason reflow2 reasons from graph state
and never from run history (`dec:loop-status-state-not-history`), and the reason a detector
should say what it *cannot* see rather than grow toward becoming a test runner.

## The one mental model to hold: the coherence loop

```
CHANGE → PROPAGATE → DETECT → SURFACE → RESOLVE/HEAL → COHERENCE
```

- **CHANGE** — any edit becomes a `ChangeEvent` at a `DesignEpoch` (the old state is snapshotted, never overwritten).
- **PROPAGATE** — walk the traceability edges to compute the blast radius.
- **DETECT** — re-diagnose the touched region for new gaps/contradictions.
- **SURFACE** — turn those into constructive, plain-language questions for the user.
- **RESOLVE/HEAL** — the user answers (re-ingested) or HEAL proposes structural fixes.

Three complementary lenses on the graph: **phases** (P0–P5 lifecycle), **three axes**
(X = network, Y = decomposition, Z = change-over-time), and this **loop** (behavior).

## Current state (important)

**Shipping at v0.89.0.** The deterministic core, the agent-native MCP surface, and the consumer
kit are all built, released as prebuilt binaries, and cold-start-verified. As of v0.12.0 the kit
is *served* rather than installed: a project holds a pointer file and the MCP config, and both the
skills and the working instructions come from the binary (`req:thin-install`). The interaction
surface — once an open question — was **decided (agent-native MCP, 2026-07-18)** and built;
`docs/requirements-coverage.md` is the living status matrix. Do not re-litigate the surface
decision or assume the surface doesn't exist — it is the binary a consumer runs, and
`tools/smoke_mcp.py` drives it end to end.

- `crates/reflow2-core/` — the deterministic, LLM-free coherence engine. Each coherence-loop
  step and each analysis is its own module (30 files); the load-bearing ones: `schema` (merges
  the 11 domains), `graph` (`DesignGraph`: schema-validated CRUD + typed golden-thread
  constructors + `upsert_node`), `nodes` (the `node::`/`edge::` name constants + the
  traceability-edge table), `temporal` (**CHANGE** — epochs, snapshots, `record_change`),
  `propagate` (**PROPAGATE** — direction-classified bounded BFS → an explained `BlastRadius`,
  summary-by-default with `full` for the dump), `detect` (**DETECT** — ranked `GapCandidate`s),
  `heal` + `structure` (**HEAL** — propose-never-mutate → atomic `apply_heal`; topology defects
  over the design network), `llm` (the sync object-safe `LlmBackend` seam + `MockLlmBackend`),
  `ingest` (**INGEST** — freeform text → graph via `LlmBackend`), `genesis` (**GENESIS** —
  bootstrap from a brief), `allocate`/`hierarchy`/`dimensions`/`budget`/`surprises`
  (graph-analysis), `confirm`/`drift`/`fielded`/`verify` (the reconcile + confirmation-ledger
  family), `artifact`/`operate`/`flow`/`provenance`/`report`/`search`/`vocabulary`/`export`
  (as-built linking, releases/environments, process flows, provenance, the rollup report,
  BM25 search, schema discovery, portable export/import). Fast dev/test build:
  `cargo test -p reflow2-core --no-default-features`.
- `crates/reflow2-mcp/` — the agent-native MCP stdio server (`service.rs`, ~78 tools; `dto.rs`;
  `main.rs`). Thin: every tool locks the graph, calls one core op, returns. It carries the
  `rocksdb` feature ON BY DEFAULT, so every installed build has on-disk storage, and development
  builds without it (`cargo dev-test`); see its own AGENTS.md.

Still unbuilt (see "What's deliberately not here yet" above and the coverage matrix): external
LLM provider backends (deferred — unneeded agent-native), SME, generative HEAL content, and the
embedding seam. The `ingest` MCP handshake (SP-3b) shipped in v0.16.0.

- `schema/*.yaml` — 11 composable schema domains (28 node types, 68 edge types), in the
  format defined by `src/foundation/core/schema.rs`. This is the foundation everything builds on.
- `docs/*.md` — the vision, design, and process specifications; `docs/overview.md` maps them.
- `getting-started/` — the consumer kit installed into a project being designed (never a build
  file). `tools/reflow2_init.py` installs it; `install.sh` fetches the released binaries.
- `tools/validate_schema.py` — validates the schema against `foundation/core`'s rules.

## Where to look

**Always start with [docs/overview.md](docs/overview.md)** — it maps every document and the
reading order (Vision → Design → Process → Heritage). Then:

| You want to… | Read |
|---|---|
| understand the "why" | [docs/vision.md](docs/vision.md) |
| understand the graph structure | [docs/three-axes.md](docs/three-axes.md), `schema/` |
| know how content gets into the graph | [docs/extraction-plan.md](docs/extraction-plan.md), [docs/sme-augmentation.md](docs/sme-augmentation.md), [docs/artifact-linking.md](docs/artifact-linking.md) |
| know how change is handled | [docs/impact-propagation.md](docs/impact-propagation.md), [docs/gap-surfacing.md](docs/gap-surfacing.md), [docs/heal-process.md](docs/heal-process.md) |
| understand the operating environment/ruleset | [docs/operating-environment.md](docs/operating-environment.md) |
| know how a human drives it (and the LLM-sourcing tradeoff) | [docs/interaction-surfaces.md](docs/interaction-surfaces.md) |
| confirm the build meets the docs (traceability) | [docs/requirements-coverage.md](docs/requirements-coverage.md) |
| use the graph to *drive* design decisions (allocation, weights, analysis crates) | [docs/graph-analysis.md](docs/graph-analysis.md) |
| make reflow2 drivable by a coding agent (the next build phase) | [docs/surface-plan.md](docs/surface-plan.md) |
| see where ideas came from | [docs/reflow-v3-nuggets.md](docs/reflow-v3-nuggets.md), [docs/chain-reflow-nuggets.md](docs/chain-reflow-nuggets.md) |

## Rules for changing this project

1. **Schema-first.** The node/edge vocabulary is load-bearing. After any `schema/*.yaml`
   edit, run `python3 tools/validate_schema.py` (needs PyYAML — on this machine use
   `~/miniconda3/bin/python`). It must print "OK".
2. **Keep docs cohesive.** Every doc carries a breadcrumb to `overview.md`; new docs must
   too, and must be added to the overview's document map and reading order.
3. **Terminology matches the schema.** Use the real node/edge names (e.g. `Capability`,
   `Component`, `Artifact`, `Verification`, `Environment`, `EnvironmentRule`,
   `ChangeEvent`). Do not reintroduce retired names from old Reflow (e.g. `PhaseEvent`,
   `ContractedFunction`, `APIEndpoint`).
4. **Honor the disciplines** the process docs call "non-negotiable" — most importantly
   **no silent fallbacks / no silent drops**: surface failures and skipped items loudly;
   never let data loss or an unstated assumption pass as success.
5. **References are the author's own** under `github.com/sligara7`. The only third-party
   pieces are dependencies (RocksDB/Tantivy/serde, declared directly since the foundation
   was absorbed; LLM providers like OpenRouter) — never conceptual content.
6. **Don't touch the sibling source repos** (`../../storyflow`, etc.) — mine them for
   ideas, but all new work lands here.
7. **A self-host finding owes a served fix.** reflow2 designing itself is a test harness, not
   the point. A lesson recorded only in reflow2's own graph reaches nobody who uses reflow2:
   graph nodes are project-local, and a consumer has their own graph and their own rules. So
   before a finding is filed, ask **which served surface carries it** — a skill, a tool's
   behaviour or output, the served instructions, or the kit. A finding that lands only in our
   own record is unfinished work, not a completed capture. Measured 2026-08-14: three gate
   lessons were captured as DesignRules here while `ci-gate` — served to every project — was
   silent on all three *and* documented two of its four failure modes, and `parallel-work` told
   users to export before every commit, which is exactly what the lineage check fails on.
   The mirror error is real too: don't push our internal discipline into a served skill, where
   it becomes a rule for somebody else's project that nobody there chose.
8. **A new sentence claiming the code REFUSES something is asserted before it ships.** When you
   write, in a tool description, skill or message, that reflow2 *refuses*, *cannot* or *never*
   does something — make sure something asserts it before you commit. Usually that is checking
   an existing test covers it; sometimes it is one line. **New and reworded sentences only,
   never retroactive.**
   Because the failure is not that prose goes stale — it is **born wrong**. Measured 2026-09-02
   over the six known-false sentences: **four were false the day they were written.** In
   `loop_status` the counter and the claim it cannot support sit in ONE COMMIT, 57 LINES APART;
   in `unresolved_setup` the message announced "nothing follows through" while the loop had no
   condition at all. The authors had the code in front of them, so care is not the missing
   ingredient — a comparison is, and nothing forces one at the moment of writing. (The session
   that discovered this came within one edit of adding a seventh, while fixing the sixth.)
   ⚠️ **Advice, not a gate** — `enforced: false`, chosen deliberately. A check could only find
   these with a keyword classifier, and the one used to size this rule over-counted about 2:1;
   a gate that misfires half the time is one people learn to skim. If the classifier can be made
   precise, the thing to revisit is **warn-in-CI, never fail** — not a red build.
   ⚠️ Do NOT backfill the ~100 such claims already standing: they are ~98% true, and a test
   written today cannot catch a sentence that was already false when committed.
   `rule:a-new-refusal-claim-is-asserted-before-it-ships`.
9. **Field feedback is processed by skill, not by reading.** When feedback arrives from a project
   that uses reflow2 — a friction log, an appended `reflow2_feedback.md`, a first-use write-up —
   run **`/root-cause` on every issue or gap it names** and **`/brainstorm` on every idea for an
   improvement or new feature**. Automatically; this is the standing way feedback is read, not a
   step to be reminded of. Set by Anthony 2026-09-05 after three consecutive reminders.
   Because reading carefully is not the same thing: the day this was set, a triage that read the
   code and wrote a cause down without the skill's steps got **two of four causes wrong**, and
   running the skipped steps (competing candidates, a measurement that could refute) overturned
   both. What compliance leaves behind: a dated `TemporalFact` per issue with a `CAUSES` edge, and
   one exploratory `Decision` per idea with its counter-argument. A triage that leaves neither did
   not follow this rule. `enforced` deliberately unstated — the detector that would notice absence
   is not built. `rule:field-feedback-issues-are-root-caused-and-ideas-are-brainstormed`.
   **Registering the report itself takes THREE writes, not one:** `add_artifact` (+ a first
   baseline), `governed_by` with `ruling: parks` to `dec:a-dated-field-report-is-registered-and-parked`,
   AND an `OBSOLETES` edge from `dec:field-reports-are-untracked-because-the-repository-is-public`
   to the artifact — the file is gitignored, so without that retirement CI's coherence gate fails
   `missing_artifact` on a build that never had it. Missed on 2026-09-07, one CI run after the
   decision text had already said so.
10. **Every failure gets its raw error searched; the full `/root-cause` runs the moment a CAUSE is
   about to be written.** Two tiers, set by Anthony 2026-09-06, ruling on his own ask to make
   `/root-cause` "more automatic … probably some caveats … might not be a blanket rule".
   **Tier 1, on ANY failure, always:** `search_design` on the exact error text and read what comes
   back (0.08 s cold — cost is never the reason to skip it). A recorded cause is applied and its
   recurrence COUNTED, never re-derived. **Tier 2, the whole skill (`get_skill root-cause`):** the
   moment you have formed an explanation for something you did not predict — an unpredicted test
   failure, a count you cannot account for, a surprise in your own build, any "because" written
   before a measurement. The trigger is the sentence, not the failure.
   **Exempt from tier 2, never from tier 1:** an error whose remedy is in the error itself (a
   compile error naming the line, a refusal listing the legal fields), a known-class friction tier 1
   found recorded (count it), a typo. Bounded by construction — errors you fix without ever writing
   a cause. "This one is obvious" is not an exemption; it is the case the skill exists for.
   Because tier 1 alone would have prevented both measured failures here: the 2026-08-22 error that
   reached **eleven occurrences** with its cause in the graph, and the 2026-09-06 re-fix of an
   already-fixed defect. And because a blanket rule becomes a ritual — five argument-shape refusals
   in one triage on 2026-09-05 were evidence, not five investigations.
   `enforced` deliberately unstated; the detector that notices a fix with no `CAUSES` edge is the
   next item. `rule:every-failure-is-searched-and-every-cause-is-root-caused-before-it-is-written`.
11. **Every failure of a reflow2 tool met while working here is captured on the record as a
   to-do.** A refusal whose remedy you found by hand, a warning on every write, a reply that did
   not say what you needed: record it before moving on, as a dated defect `TemporalFact` on the
   capability or artifact the tool realizes, with the tool, the refusal (ids masked), what you
   expected and the workaround. Rule 10's tier 1 first: search the refusal text, and a precedent
   gets a COUNTED recurrence, not a second record. A refusal that is the loop working (a
   near-duplicate guard, a required field the reply names correctly) is not a failure.
   Set by Anthony 2026-09-06: *"if we are not at least capturing these failures as to-do items,
   then this is a big issue … it'll come out when reflow2 is used in somebody else's design."*
   Because reflow2's own sessions are its first field: three refusals were met in the session
   this was set in and none was written down until he asked — one of them a recurrence, on the
   sibling tool, of a fact recorded the day before. `enforced` deliberately unstated; the
   detector leg (the server counting its own refusals) is an open idea beside the rule.
   `rule:a-reflow2-tool-failure-met-while-working-is-captured-as-a-to-do`.

### What carries across sessions, and what does not

Taken from the StoryFlow fleet's boss charter, which draws the line explicitly and puts the
highest-consequence permission on the far side of it: *"prior-session GOs — merge authority,
deploy windows — do not carry into your session. Get fresh ones."* Absent a written list, both
errors are available: asking permission for the routine, and assuming it for the irreversible.
The line is **reversible → standing; outward-facing → per-session.**

**Standing** — do these without asking again:

- Accept checksum drift on a registered artifact you just edited, with a `note` saying which it
  was (standing policy, 2026-07-23). The accept is recorded either way, so it is reviewable.
- Record intent in the graph, run the detectors, commit and push to `main` with the gate green.
- Acknowledge a gap or a defect **only** with the user's reason on the record — the
  acknowledgement is a Decision, and `dec:certainty-derived` says a status change carries the
  *user's* word, not yours.

**Per-session** — ask every time, however recently it was granted:

- **Cutting a release.** Tagging and publishing is this project's deploy: it reaches other
  people's machines and cannot be recalled.
- **Filing an issue or anything else in a repository the user does not control** (the
  `report-friction` skill already enforces this).
- **Moving a Requirement off `proposed`, or a Decision to `accepted`.** That write *is* the
  user's signature; making it on their behalf forges it.

## Engineering principles (adapted from storyflow's `PROTOCOL.md ⭐`)

These are the author's hard-won code-quality principles, carried over and adapted to
reflow2 (a single Rust core, no fleet). They **override speed — timing bends to correctness**.

1. **Right long-term fix — no patches/stopgaps.** Find the *root cause* first (reproduce,
   trace, prove the mechanism), then fix it at the root, not the symptom. If you can't name
   the root cause, say so and keep digging. If the correct fix needs something that isn't
   there yet, **stop and report the gap** — a reported gap is honest; a papered-over one
   re-breaks later.
2. **No silent fallbacks / no silent drops — an integrity line, not a style preference.**
   Never swallow an error into a "looks fine" state (no catch-returns-default, no atomic op
   that drops the bad part, no empty-on-failure). A swallowed failure makes broken code
   report success — it *lies* to the user. Fail loud, or don't write it. This is rule 4
   above; it is the project's first principle. (Enforced concretely across the core — see
   the "load-bearing invariants" under Architecture above.)
3. **Record every deferral — no silent stubs.** When you defer work, write it down as
   Deferred in [docs/requirements-coverage.md](docs/requirements-coverage.md) **in the same
   change**, and annotate the code site (an unused field, a stubbed branch) with a pointer
   back. A deferral nobody wrote down is a silent stub — the same integrity breach as a
   silent drop. "Partial and recorded" is fine; "looks done but quietly isn't" is not.
4. **Verify your own claims before stating them.** Run the real check yourself (foreground
   `cargo test --no-default-features`, clippy, fmt) and confirm any symbol/field/API you
   reference actually exists. "Tests pass" means you watched them pass; a green that only
   passed because an error was swallowed is a false report.
5. **Real-path tests.** A test must exercise the path callers actually use, end to end — not
   an unchanged inner helper. "Done" = the real behavior is observable and tested, not "it
   compiles."
6. **No silent caps/truncation.** If you bound coverage (top-N, a subset of passes, a depth
   cap), say so loudly in the code and the report — silent truncation reads as "covered
   everything" when it didn't. (This is why PROPAGATE reports `truncated_beyond_depth` and
   INGEST records `dropped_edges`.)
7. **Modular, composable code — no monoliths.** Keep files focused and single-responsibility;
   split along natural seams (each coherence-loop step is its own module). Prefer small
   composable pieces and dependency injection (e.g. `&dyn LlmBackend` passed in) over
   sprawling files or deep inheritance.

## Provenance of the ideas (so you can trace any decision)

- **storyflow** → the extraction pipeline, the six universal processes, the operating-
  environment ruleset (its "cosmology"), SME/supplementary analysis, the note layer, the three axes.
- **chain_reflow** → matryoshka/missing-intermediate detection, correlation-vs-causation
  rigor, creative linking, system-of-systems.
- **reflow (v3)** → the phase spine, as-designed/as-built/as-fielded fidelity views,
  framework packs, root-cause change classification.
- **dynograph-foundation** → the schema-driven graph store (RocksDB + BM25 + fuzzy
  matching). The subset reflow2 actually called was absorbed into this tree at `v0.12.0`;
  what reflow2 never called (HNSW/vector resolution, pagerank, the entity resolver) stayed
  behind.

> **reflow2 is installed here.** The design graph is this project's memory — read [REFLOW2.md](REFLOW2.md) and consult it before writing or changing code.
