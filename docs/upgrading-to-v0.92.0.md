# Upgrading to v0.92.0

🛑 **Upgrade every seat together, before anyone records a limit each part meets on its own
(`composition: each`).** This release's schema stamp moves by vocabulary. No node or edge type is
added or removed (28 and 69, as in v0.91.0), but the stamp's `enum_values` move:

| | v0.91.0 | v0.92.0 |
| --- | --- | --- |
| Node types | 28 | 28 |
| Edge types | 69 | 69 |
| `Constraint.composition` | sum, path | the same, plus `each` |
| New edge enum properties | — | `reads_as` (intent, delivery) on PREMISE_OF; `used_basis` (stated, current) on DEPENDS_ON |
| New properties | — | `answered_none` on ChangeEvent; `input`, `value_used`, `unit_used` and `checksum_used` on DEPENDS_ON |

**Every seat** means every binary that opens the design:
- **the flo2.io engine** (and any other hosted server, or the container image at `:0.92.0`);
- **a local `reflow2-mcp`**, the MCP server a harness starts;
- **the `--call` binary** (`reflow2 read` / `reflow2 write`, as in VS Code where MCP is blocked);
- **committed design copies**: an export written by v0.92.0 is refused by an older binary's plain
  import, so a CI gate or a teammate on v0.91.0 cannot read a copy this release wrote.

The served tool schemas move with it: the tool count goes from 200 to 199 (`record_the_build`
added; `record_change` and `manual_work_report` removed), and twelve tools differ (the
CHANGELOG's `[0.92.0]` section lists them), so a client that pins tool schemas (a toolsnap, a
generated client) moves too.

## What an older binary does

Measured at the cut with the published v0.91.0 release binary (from the release's
`reflow2-mcp-linux-x86_64.tar.gz`, checked against its `checksums.txt`) and this release's build.
Each test design was a chain of 56 reasoning steps written by v0.91.0, plus at most one of the new
things written by v0.92.0.

- **v0.92.0 opens a v0.91.0 store**, saying "Additive only — everything in it still reads".
- **v0.91.0 opens, with its "behind" warning, a store v0.92.0 has opened**, and reads it:
  > THIS GRAPH WAS LAST WRITTEN BY reflow2 0.92.0; you are running 0.91.0, which is BEHIND it.
- **v0.91.0 refuses to open a store holding `composition: each`**, naming it:
  > REFUSED: this graph holds Constraint.composition = "each" — 1 value(s) this reflow2 does not
  > know, on a property it DOES know.

  Nothing is opened or changed.
- **v0.91.0 refuses a plain import of any v0.92.0 export** ("you are running 0.91.0, which is
  BEHIND it"). Nothing is written.
  - With `--accept-newer` it imports an export holding a premise marked `reads_as` (measured), and
    has no reading for it.
  - It still refuses, all or nothing, an export holding `composition: each` ("invalid enum value
    'each', expected one of ["sum", "path"]").
- **A reading v0.91.0 gets differently without any new value:** a step concluding a Decision reads
  its requirement, capability and interface premises as their owner's intent in v0.92.0 and by
  their delivery in v0.91.0. A design choice resting on accepted requirements reads `shown` in
  v0.92.0 and `unproven` in v0.91.0. Nothing is stored differently; the two binaries read the same
  records two ways.

## What you do

1. **Snapshot the store of any server that other people or machines reach** before upgrading it.
   That snapshot is your rollback.
2. **Update every seat above, together.** The installer's update does it, and a hand-built copy
   needs rebuilding. A seat left on v0.91.0 is locked out of the design the moment anyone records
   a limit with `composition: each`, and reads design choices differently before that.
3. **Stop any shared server still running the old binary:**
   `reflow2-mcp --graph-path <path> --stop-shared`. A reconnect alone does not replace it.
4. **Re-take committed design copies with the new binary**, and update any pinned tool schemas.
5. **Run `reflow2 update` in each project** that `reflow2 init` set up, and re-run the installer for
   the copies in `~/.claude/commands/`: the slash commands are now pointers to the served skills
   (#794), and an installed copy is not refreshed by the binary.
6. **Open the design.** Nothing needs migrating, and no export or import is required.

## Rolling back to v0.91.0

Restore the snapshot from step 1. Without one, v0.91.0 still opens the store (with the warning
above) only while it holds no `composition: each`. Re-record such a limit as `sum` first, or
remove it. A premise's `reads_as`, a kept output's inputs and a change's `answered_none` stay in
the store, and v0.91.0 does not read them.

## Also in this release, and what it may ask of you

The CHANGELOG's `[0.92.0]` section has the full list.

- **`loop_status` and `detect_gaps` list what is recorded broken** (`breaches`, `open_breach`)
  until something closes it. A design with many old defect findings sees a large count the first
  time; most are fixes that never drew `invalidates`.
- **A `null` now clears a field** on a revise through any `add_*` constructor, `plan_epoch` and
  `record_finding`. A client that sends `null` for every field it leaves empty will clear those
  fields. Send only the fields you mean to change.
- **`record_change` and `manual_work_report` are removed.** Use `snapshot_before_change` and
  `manual_work_ledger`, which take the same arguments.
- **A long reasoning chain no longer crashes the server**, and the server's threads reserve a
  16 MiB stack each (address space; memory is used only by a call that goes deep).
