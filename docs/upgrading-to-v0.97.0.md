# Upgrading to v0.97.0

🛑 **Upgrade every seat together, before anyone records a failed route
(`statement_kind: failed_route`).** This release's schema stamp moves by vocabulary. No node or
edge type is added or removed (28 and 69, as in v0.96.0), but the stamp's `enum_values` move:

| | v0.96.0 | v0.97.0 |
| --- | --- | --- |
| Node types | 28 | 28 |
| Edge types | 69 | 69 |
| Enum vocabularies | 83 | 83 |
| `Requirement.statement_kind` | 16 values | the same, plus `failed_route` |
| New properties | — | `route_to`, `fails_at`, `fails_at_step`, `fails_because`, `route_rests_on` and `would_hold_if` on Requirement, each optional |

**Every seat** means every binary that opens the design:
- **the flo2.io engine** (and any other hosted server, or the container image at `:0.97.0`);
- **a local `reflow2-mcp`**, the MCP server a harness starts;
- **the `--call` binary** (`reflow2 read` / `reflow2 write`, as in VS Code where MCP is blocked);
- **anything that imports an export**: an export written by v0.97.0 is refused by an older
  binary's plain import.

The served tool schemas move with it, and no tool is added or removed (199 on both):
`add_requirement` takes the six fields above, `loop_status` takes `time_budget_ms`,
`search_design`'s output declares `route_to`, and `reasoning_report`'s description names failed
routes. A client that pins tool schemas (a toolsnap, a generated client) moves too.

## What an older binary does

Measured at the cut with the published v0.96.0 release binary (from the release's
`reflow2-mcp-linux-x86_64.tar.gz`, checked against its `checksums.txt`) and this release's build.
Each test design was a few records written by v0.97.0: a conjecture, a capability, a file
registered against it with its checksum accepted by a change, and, in the second design, one
failed route toward the conjecture.

- **v0.96.0 opens a store v0.97.0 wrote**, with its "behind" warning, and reads it, while the
  store holds no failed route:
  > THIS GRAPH WAS LAST WRITTEN BY reflow2 0.97.0; you are running 0.96.0, which is BEHIND it.
- **v0.96.0 refuses to open a store holding a failed route**, naming it:
  > REFUSED: this graph holds Requirement.statement_kind = "failed_route" — 1 value(s) this
  > reflow2 does not know, on a property it DOES know.

  Nothing is opened or changed.
- **v0.96.0 refuses a plain import of any v0.97.0 export** ("you are running 0.96.0, which is
  BEHIND it"). Nothing is written.
  - With `--accept-newer` it imports an export holding no failed route. It ignores the export's
    new `reading_notes` and restores the accepted checksum onto the Artifact, as before.
  - It still refuses, all or nothing, an export holding a failed route ("invalid enum value
    'failed_route'").

## What you do

1. **Snapshot the store of any server that other people or machines reach** before upgrading it.
   That snapshot is your rollback.
2. **Update every seat above, together.** The installer's update does it, and a hand-built copy
   needs rebuilding. A seat left on v0.96.0 is locked out of the design the moment anyone records
   a failed route.
3. **Stop any shared server still running the old binary:**
   `reflow2-mcp --graph-path <path> --stop-shared`. A reconnect alone does not replace it.
4. **Re-take committed exports with the new binary**, and update any pinned tool schemas.
5. **Run `reflow2 update` in each project** that `reflow2 init` set up. For the VS Code terminal
   route it adds the new skill's two files (`.github/skills/work-a-proof-hole/` and
   `.github/prompts/work-a-proof-hole.prompt.md`) and changes nothing else.
6. **Open the design.** Nothing needs migrating, and no export or import is required.

## Rolling back to v0.96.0

Restore the snapshot from step 1. Without one, v0.96.0 still opens the store (with the warning
above) only while it holds no failed route. Delete each failed route first, with this release
(`reasoning_report` lists them under `failed_routes`), after re-recording what it says as prose
on another record if you want to keep it.

## Also in this release, and what it may ask of you

The CHANGELOG's `[0.97.0]` section has the full list.

- **`loop_status` answers within a time budget** (20 s by default) and names in `not_computed`
  what it did not reach. Its `unsurfaced_gaps`, `answered_with_open_gap`,
  `answered_naming_their_record`, `structural_defects`, `unexamined_claims` and `loop_closure`
  read `null` when not computed, never `0`. A script that adds them up should treat `null` as
  "not known from this answer".
- **A client that gives up on a long read should send `notifications/cancelled`.** On an HTTP
  session (before MCP 2026-07-28) a dropped connection does not stop the call; the notification
  does.
- **A single-file export may carry `reading_notes`**, outside its content hash. A script that
  reads checksums from an export should read them with the kit's `tools/design_io.py`
  `current_acceptances`, or import the export and read the records.
