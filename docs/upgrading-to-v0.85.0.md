# Upgrading to v0.85.0

🛑 **Upgrade every seat together.** This release's schema stamp moves. A design can now hold
inference steps, so the schema gains:
- one node type (`Inference`);
- two edge types (`PREMISE_OF`, `CONCLUDES`);
- three optional `Inference` properties;
- a new `Decision.status` value, `contested`.

| | v0.84.0 | v0.85.0 |
| --- | --- | --- |
| Node types | 28 | **29** (+ `Inference`) |
| Edge types | 66 | **68** (+ `PREMISE_OF`, `CONCLUDES`) |
| `Inference` | — | `rule`, `k`, `rule_name`, `warrant`, and the optional `confirmed_by`, `confirmed_how`, `confirmed_at` |
| `Decision.status` | proposed, accepted, deferred, superseded, rejected | + **contested** |

## What an older binary does

This was measured at the cut with the published v0.84.0 release binary, checked against the
release's `checksums.txt`.

- **v0.84.0 refuses a store that v0.85.0 has opened, even one that holds nothing new.**
  - v0.85.0 re-stamps a store when it opens it.
  - v0.84.0 then refuses the whole store before reading anything, and names the types it lacks:

    > this graph (written by reflow2 0.85.0) names types this reflow2 (0.84.0) cannot read, so
    > opening it could silently show you less of your design than it holds — refused.
    > • Your reflow2 is BEHIND: it has never heard of Inference, CONCLUDES, PREMISE_OF — update
    > reflow2 (or rebuild from a current checkout) and reopen.

- **v0.84.0 refuses to import an export that v0.85.0 wrote.** Nothing is written ("this document
  was written by reflow2 0.85.0, and you are running 0.84.0, which is BEHIND it").
- **v0.85.0 opens a v0.84.0 store untouched.** Nothing needs migrating.

The refusals are loud and partial reads cannot happen. A seat that has not been upgraded stops
working on the design until it is.

## What you do

1. **Snapshot the store of any server that other people or machines reach** before upgrading it.
   That snapshot is your rollback; an export taken afterwards is not (see above).
2. **Update every binary that opens the design:** every machine, every harness and every server
   (a hosted engine, or the image at `:0.85.0`). The installer's update does it, and a hand-built
   copy needs rebuilding.
3. **Stop any shared server still running the old binary:**
   `reflow2-mcp --graph-path <path> --stop-shared`. A reconnect alone does not replace it.
4. **Open the design.** Nothing needs migrating, and no export or import is required.

## Rolling back to v0.84.0

Restore the snapshot from step 1. A store that v0.85.0 has opened will not open in v0.84.0, and
its export will not import there.

## Also in this release, and what it may ask of you

The CHANGELOG's `[0.85.0]` section has the full list.

- **A merge nobody picked a survivor for is refused.** `apply_heal` refuses a proposal whose merge
  carries `survivor.needs_choice: true` until that operation names `chosen_survivor`. A script
  that passes `propose_heal`'s proposal straight back must ask the person which node to keep.
- **The `Anchor` node type is deprecated, and the next release removes it.** Nothing has created
  one. Record a checksummed baseline with `add_epoch` (`epoch_type` `baseline`, with its
  `checksum`) and capture a node's state with `snapshot_before_change`.
- **An objection counts as far as it holds.** A `CONTRADICTS` used as a counterexample refutes its
  target only when the counterexample itself holds, such as a passing check or a met requirement.
  One not yet established leaves the target `unknown`, contested.
- **For contributors to reflow2:**
  - `cargo dev-test` builds and tests both crates without RocksDB;
  - a `reflow2-mcp` built without the `rocksdb` feature refuses a store in a folder, naming the
    fix.
