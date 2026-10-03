# Upgrading to v0.78.0

🛑 **Upgrade every seat together, before anyone feeds a run back with the outcome `blocked`.**
This release's schema stamp moves: `Verification.last_reconciled_outcome` gains the value
`blocked` (#662). No type or edge count changes with it.

## What an older binary does: not what v0.65.0's note led you to expect

Since v0.59.0, a binary is meant to refuse a design that stores an enum value it does not know.
**That guard does not hold for this value.** It was measured at this cut with the published
v0.77.0 release binary, on a design where v0.78.0 had recorded `blocked`. v0.77.0 did three
things:

- **It opened the design** with exit 0 and no warning, and read the value back as `"blocked"`.
- **It re-stamped the store as 0.77.0.** The stamp no longer records that the store holds a value
  this binary does not know.
- **Its `--export` succeeded, but importing that export into v0.77.0 was refused**, all or nothing:
  `invalid enum value 'blocked', expected one of ["passed", "failed", "skipped"]`. v0.78.0 imports
  the same file.

**Why the guard missed it.** The guard asks the store whether any node holds the unknown value. It
asks through the property index, and `last_reconciled_outcome` is not indexed, so the answer is
always "none". The two earlier value moves, `Decision.status` (v0.59.0) and
`ChangeEvent.change_type` (v0.65.0), are on indexed properties, which is why those were refused as
promised. A fix cannot reach a binary that has already shipped, so upgrading is what protects you.
The finding is recorded as
`fact:an-older-binary-opens-a-design-storing-a-non-indexed-enum-value-it-does-not-know-and-restamps-it-2026-10-03`.

**When the value appears.** It is written only when `reconcile_verification` records the outcome
`blocked`, which means a check that could not run, such as a collection error or a test file that
did not compile. `tools/run_to_files.py` now writes it for those cases. Until that happens once, a
seat that has not been upgraded sees nothing different.

| | v0.77.0 | v0.78.0 |
| --- | --- | --- |
| Node types | 28 | 28 |
| Edge types | 66 | 66 |
| `Verification.last_reconciled_outcome` | passed, failed, skipped | + **blocked** |

## What you do

1. **Update every binary that opens the design:** every machine, every harness, and every server
   (a hosted engine, or the image at `:0.78.0`). The installer's update does it; a hand-built copy
   needs rebuilding.
2. **Stop any shared server still running the old binary:**
   `reflow2-mcp --graph-path <path> --stop-shared`. A reconnect alone does not replace it.
3. **Open the design.** A v0.78.0 binary opens a v0.77.0 design untouched and re-stamps it.
4. **Nothing needs migrating.** No export and no import are required.

## If you cannot upgrade a seat yet

**Do not feed a run back with the outcome `blocked` until every seat can read it.** Send `failed`
instead, or leave that check out of the run.

**If an old seat already exported a design that holds `blocked`,** import that export with
v0.78.0.

## Rolling back to v0.77.0

**The store format rolls back.** RocksDB 11.8.1 writes table `format_version` 7. v0.77.0, on RocksDB
10.4.2, reads it: measured on 20 real stores (#652).

**A design that holds `blocked` also opens in v0.77.0, as described above.** Its export will not
import back into v0.77.0.

## Also in this release, and what it may ask of you

The CHANGELOG's `[0.78.0]` section has the full list.

- **Refused instead of ignored.** These changes may ask something of a script: drop what the
  refusal names.
  - A wrong-typed, unknown or out-of-set argument is refused before the tool runs, naming the tool
    and the field path.
  - Twelve tools that take no arguments refuse one.
  - A one-shot mode refuses a flag it does not read.
  - `--read-only` is honoured by `--call` and by every forwarding client.
- **`external_dependency` replies with a receipt.** Read the whole manifest with
  `reconcile_dependencies`.
- **A writing `--call` updates the committed export before it exits.** Exit 3 means the write
  landed and the export did not.
- **Source builds need Rust 1.98.** The first build rebuilds `librocksdb-sys`.
