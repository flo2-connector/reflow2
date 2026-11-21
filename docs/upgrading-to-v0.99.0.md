# Upgrading to v0.99.0

**Upgrade every seat together.** This release's schema stamp moves by two edge types, and a
v0.98.0 binary refuses any store a v0.99.0 binary has opened, even once and only to read.

| | v0.98.0 | v0.99.0 |
| --- | --- | --- |
| Node types | 28 | 28 |
| Edge types | 69 | 71 |
| Enum vocabularies | 84 | 84 |
| New edge types | — | `PUTS_TOGETHER` (a step to a Component), `CLOSES` (a step to an Interface) |
| `Flow.flow_type` | seven values | gains `assembly` |
| `INTERACTS_WITH.interaction` | names four values | names a fifth, `performs` (free text, as before) |

v0.98.0's move was by vocabulary, and an older seat went on opening the design. This one is
different: a new edge type moves both halves of the stamp, so the older binary cannot tell what
it would be leaving out and refuses to open the store at all.

**Every seat** means every binary that opens the design:
- **the flo2.io engine** (and any other hosted server, or the container image at `:0.99.0`);
- **a local `reflow2-mcp`**, the MCP server a harness starts;
- **the `--call` binary** (`reflow2 read` / `reflow2 write`, as in VS Code where MCP is blocked);
- **anything that imports an export**: an export written by v0.99.0 is refused by an older
  binary's plain import.

The served tool schemas move with it, and no tool is added or removed (199 on both):
`part_of_flow` takes `puts_together` and `closes`, and `add_flow`'s `flow_type` takes `assembly`;
`flow_report`'s description says an assembly reads back as instructions. A client that pins tool
schemas (a toolsnap, a generated client) moves too.

## What an older binary does

Measured at the cut with the published v0.98.0 release binary (from the release's
`reflow2-mcp-linux-x86_64.tar.gz`, checked against its `checksums.txt`) and this release's build,
through the command-line door, each on a fresh design on disk.

- **v0.98.0 refuses to open any store v0.99.0 has written**, to read or to write, even one that
  holds neither new edge:
  > this graph (written by reflow2 0.99.0) names types this reflow2 (0.98.0) cannot read, so
  > opening it could silently show you less of your design than it holds — refused.
  > Your reflow2 is BEHIND: it has never heard of CLOSES, PUTS_TOGETHER

- **One read by v0.99.0 is enough.** v0.99.0 opens a store v0.98.0 wrote and says so ("this graph
  was written by reflow2 0.98.0 (28 node types, 69 edge types); you are running 0.99.0 (28, 71).
  Additive only — everything in it still reads"). That open moves the store's stamp to 0.99.0,
  with `previous_reflow2_version: 0.98.0`, whether the call was a read or a write. Measured: a
  store v0.98.0 wrote, one `get_node` by v0.99.0, and v0.98.0 then refuses it with the message
  above. `reflow2 update` opens the store too, so the same holds after it.
- **v0.98.0 refuses a plain import of any v0.99.0 export** ("you are running 0.98.0, which is
  BEHIND it"). Nothing is written.
- **With `--accept-newer`, v0.98.0 imports a v0.99.0 export only when no step in it names a part
  or a join and no flow is an assembly.** Such an export imports whole, and v0.98.0 then reads and
  writes the copy. An export holding one `PUTS_TOGETHER`, even on a step of an ordinary process,
  is refused whole ("Unknown edge type: PUTS_TOGETHER"), and so is one holding an assembly flow
  ("invalid enum value 'assembly'", then each `CLOSES` and `PUTS_TOGETHER` by name). The import is
  all or nothing, and it lists every item it cannot read.
- **v0.98.0 refuses the new inputs on its own store**, before the call runs (exit 2), and
  nothing is written: `add_flow` with `flow_type: assembly` ("expected one of `process`,
  `data_flow`, `control_flow`, `decision_flow`, `capture`, `retrieval`, `generation`, or null;
  got the string "assembly""), and `part_of_flow` with `puts_together` or `closes` ("is not a
  parameter of this tool").
- **v0.99.0 imports a v0.98.0 export** whole, with nothing to accept.

## What you do

1. **Snapshot the store of any server that other people or machines reach** before upgrading it,
   and keep an export written by v0.98.0. That snapshot is your rollback: once v0.99.0 has opened
   a store, v0.98.0 will not.
2. **Update every seat above, together.** The installer's update does it, and a hand-built copy
   needs rebuilding. A seat left on v0.98.0 is locked out of the design from the first time
   another seat opens it with v0.99.0.
3. **Stop any shared server still running the old binary:**
   `reflow2-mcp --graph-path <path> --stop-shared`. A reconnect alone does not replace it.
4. **Re-take committed exports with the new binary**, and update any pinned tool schemas.
5. **Run `reflow2 update` in each project** that `reflow2 init` set up. For the VS Code terminal
   route it changes no file you commit: no skill and no command changed in this release. It
   backs the design up to `.reflow2/backups/` first, then opens the store, which moves the
   store's stamp.
6. **Open the design.** Nothing needs migrating, and no export or import is required. A flow
   recorded before this release reads exactly as it did. To record how a thing is made, give the
   flow `flow_type: assembly` and name each step's parts (`puts_together`) and joins (`closes`)
   with `part_of_flow`.

## Rolling back to v0.98.0

Restore the snapshot from step 1. Without one, there is one way back, and only while the design
holds no assembly flow and no step that names a part or a join: export the design with v0.99.0,
then import that export into an empty store with v0.98.0 and `--accept-newer`. Measured: the copy
reads and writes under v0.98.0, and its stamp reads 0.98.0. Once the design holds an assembly,
v0.98.0 cannot read it by any route, and the snapshot is the only way back.

## Also in this release, and what it may ask of you

The CHANGELOG's `[0.99.0]` section has the full list.

- **`loop_status` is faster on a large design**, on the first ask after a write and on a repeat
  ask with no write between. What a repeat ask reuses is kept in the running engine, so it
  arrives where one engine stays up between calls (an MCP server, a shared server, flo2.io) and
  not on a one-shot call, which starts a new engine each time. Nothing to do.
- **Building from source needs Rust 1.99 or newer**, and the repo pins 1.100.0 in
  `rust-toolchain.toml`; rustup installs it the first time you run `cargo` there.
- **An exposed `--http` server checks bearer tokens with `jsonwebtoken` 12.0.0.** A token whose
  header names a critical extension the check does not know is now refused.
- **`reflow2 update` refuses on a project that kept only part of the kit**, and changes nothing
  there. `reflow2 init .` brings the whole kit back.
