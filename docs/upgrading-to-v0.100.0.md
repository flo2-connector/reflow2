# Upgrading to v0.100.0

🛑 **Upgrade every seat together, before anyone closes an increment.** This release's schema
stamp moves by vocabulary. No node or edge type is added or removed (28 and 71, as in v0.99.0),
but the stamp's `enum_values` move:

| | v0.99.0 | v0.100.0 |
| --- | --- | --- |
| Node types | 28 | 28 |
| Edge types | 71 | 71 |
| Enum vocabularies | 84 | 84 |
| `DesignEpoch.status` | `planned`, `arrived` | the same, plus `deferred`, `dropped`, `superseded` |
| New properties | — | `close_reason` and `superseded_by` on DesignEpoch (not in the stamp) |

This move is like v0.92.0's, not like v0.99.0's: a v0.99.0 seat goes on opening the design, with
its "behind" warning, until someone closes an increment with `close_increment`. From then on it is
refused.

**Every seat** means every binary that opens the design:
- **the flo2.io engine** (and any other hosted server, or the container image at `:0.100.0`);
- **a local `reflow2-mcp`**, the MCP server a harness starts;
- **the `--call` binary** (`reflow2 read` / `reflow2 write`, as in VS Code where MCP is blocked);
- **anything that imports an export**: an export written by v0.100.0 is refused by an older
  binary's plain import.

The served tool schemas move with it: the tool count goes from 199 to 200 (`close_increment`
added), and seven tools differ (`graph_report`, `derived_report`, `add_epoch`, `plan_epoch`,
`set_epoch_status`, `create_node`, `create_nodes`; the CHANGELOG's `[0.100.0]` section says how).
A client that pins tool schemas (a toolsnap, a generated client) moves too.

## What an older binary does

Measured at the cut with the published v0.99.0 release binary (from the release's
`reflow2-mcp-linux-x86_64.tar.gz`, checked against its `checksums.txt`) and this release's build,
through the command-line door, each on a fresh design on disk.

- **v0.100.0 opens a v0.99.0 store**, saying "Additive only — everything in it still reads". That
  open moves the store's stamp to 0.100.0, whether the call was a read or a write.
- **v0.99.0 opens, with its "behind" warning, a store v0.100.0 has opened**, and reads and writes
  it, while no increment in it is closed:
  > THIS GRAPH WAS LAST WRITTEN BY reflow2 0.100.0; you are running 0.99.0, which is BEHIND it.
- **v0.99.0 refuses to open a store holding one closed increment**, to read or to write, naming
  the value:
  > REFUSED: this graph holds DesignEpoch.status = "deferred" — 1 value(s) this reflow2 does not
  > know, on a property it DOES know.

  The same holds for `dropped` and `superseded`. Nothing is opened or changed.
- **A deferred increment reopened** with `set_epoch_status` `planned` holds none of the new values,
  and v0.99.0 opens that store again, with its warning. Reopening clears `close_reason` and
  `superseded_by`; the close's ChangeEvent keeps both.
- **v0.99.0 refuses a plain import of any v0.100.0 export** ("you are running 0.99.0, which is
  BEHIND it"). Nothing is written.
  - With `--accept-newer` it imports whole an export that holds no closed increment, and then
    reads and writes the copy.
  - It refuses whole, all or nothing, an export holding a single closed increment ("invalid enum
    value 'deferred', expected one of ["planned", "arrived"]"), and names each record that rests
    on it: the approver's `AUTHORED_BY` and the close's `CHANGED`.
- **v0.99.0 has no `close_increment`** ("no tool named `close_increment` is served"), its
  `loop_status` has no `open_increments`, and its `graph_report` refuses `include_open_increments`
  before the call runs (exit 2).
- **v0.100.0 imports a v0.99.0 export** whole, with nothing to accept.

## What you do

1. **Snapshot the store of any server that other people or machines reach** before upgrading it,
   and keep an export written by v0.99.0. That snapshot is your rollback once an increment is
   closed.
2. **Update every seat above, together, before anyone calls `close_increment`.** The installer's
   update does it, and a hand-built copy needs rebuilding. A seat left on v0.99.0 is locked out of
   the design the first time anyone closes an increment in it.
3. **Stop any shared server still running the old binary:**
   `reflow2-mcp --graph-path <path> --stop-shared`. A reconnect alone does not replace it.
4. **Re-take committed exports with the new binary**, and update any pinned tool schemas.
5. **Run `reflow2 update` in each project** that `reflow2 init` set up. For the VS Code terminal
   route it changes no file you commit: the two skills that changed (plan-increments and
   where-am-i) are served, and their stubs did not change. It backs the design up to
   `.reflow2/backups/` first, then opens the store, which moves the store's stamp.
6. **Open the design.** Nothing needs migrating, and no export or import is required. Every
   increment that is still planned now reads as open in `loop_status` and `graph_report`, oldest
   first. A design with many old planned epochs sees a long count the first time. Close the ones
   that will not be finished with `close_increment`, on their owner's word, or mark the ones that
   happened `arrived`.

## Rolling back to v0.99.0

Restore the snapshot from step 1. Without one, v0.99.0 still opens the store (with the warning
above) while no increment in it is closed. A deferred increment can be reopened with
`set_epoch_status` `planned` first (measured: v0.99.0 then opens the store). Reopening clears
the close from the increment, and the close's ChangeEvent stays on the record.

## Also in this release, and what it may ask of you

The CHANGELOG's `[0.100.0]` section has the full list.

- **`derived_report` takes `budget_chars`** (default 30,000), and a reply over it cuts the
  declarations' text and says so at `budget`. Every relation, count and example id stays. `only`
  reads named relations whole.
- **`set_epoch_status` and `add_epoch` refuse the three closes**, naming `close_increment`.
- **A closed increment is the owner's settle**: one written without an `AUTHORED_BY` approver
  fails the design gate (`tools/check_intent_authority.py`).
