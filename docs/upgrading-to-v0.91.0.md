# Upgrading to v0.91.0

🛑 **Upgrade every seat together, before any seat on v0.91.0 opens a design another seat still
reads with v0.90.0.** This release's schema stamp moves by a TYPE: one edge type is added,
`EQUIVALENT_TO`, so the stamp's edge-type count and names move, and with them every store this
release opens. Nothing is removed or narrowed.

| | v0.90.0 | v0.91.0 |
| --- | --- | --- |
| Node types | 28 | 28 |
| Edge types | 68 | 69 (`EQUIVALENT_TO`, with `by` and `evidence`) |
| `statement_kind` (Requirement, Constraint) | theorem … judgement (15 values) | the same, plus `bridge` |
| New enum properties | — | `source_reliability` (A–F) and `information_credibility` (1–6) on Artifact |
| New properties | — | `not_read` on Artifact; `claimed_by` on Requirement, Constraint and Inference |

**Every seat** means every binary that opens the design:
- **the flo2.io engine** (and any other hosted server, or the container image at `:0.91.0`);
- **a local `reflow2-mcp`**, the MCP server a harness starts (Alex's MCP setup is one);
- **the `--call` binary** (`reflow2 read` / `reflow2 write`, as in VS Code where MCP is blocked);
- **committed design copies**: an export written by v0.91.0 is refused by an older binary's plain
  import, so a CI gate or a teammate on v0.90.0 cannot read a copy this release wrote.

The served tool schemas move with it: eleven tools take new arguments or values or read
differently (the CHANGELOG's `[0.91.0]` section lists them), so a client that pins tool schemas
(a toolsnap, a generated client) moves too.

## What an older binary does

Measured at the cut with the published v0.90.0 release binary (sha256 `383551b7…`, the one
flo2's CI pins) and this release's build. Each test design was a premise, a claim and the step
between them, written by v0.91.0, plus at most one of the new things.

- **v0.90.0 refuses to open ANY store v0.91.0 has opened, even one holding nothing new, and even
  if v0.91.0 only read it.** Opening records the stamp, and the stamp now names an edge type
  v0.90.0 lacks:
  > BEHIND: it has never heard of EQUIVALENT_TO — update reflow2 (or rebuild from a current
  > checkout) and reopen

  Measured on a store v0.90.0 wrote, opened once by v0.91.0 with `reflow2 read graph_report`:
  v0.90.0 then refused to open or export it. Nothing in it changed (v0.91.0's export had the
  content hash v0.90.0's export had before). **So the moment a v0.91.0 seat opens a shared
  design, every v0.90.0 seat on that design is locked out.**
- **v0.90.0 refuses a plain import of any v0.91.0 export** ("you are running 0.90.0, which is
  BEHIND it"). Nothing is written.
  - With `--accept-newer` it imports an export holding none of the new types or values; one
    holding `claimed_by`, or `not_read` and a grade on an Artifact, imports too (measured), and
    v0.90.0 has no reading for those values.
  - It still refuses, all or nothing, an export holding an `EQUIVALENT_TO` edge ("Unknown edge
    type: EQUIVALENT_TO") or `statement_kind: bridge` ("invalid enum value 'bridge'").
- **v0.91.0 opens a v0.90.0 store**, saying "Additive only — everything in it still reads", and
  imports a v0.90.0 export. Nothing needs migrating.

## What you do

1. **Snapshot the store of any server that other people or machines reach** before upgrading it.
   That snapshot is your rollback: once v0.91.0 has opened the store, v0.90.0 will not.
2. **Update every seat above, together.** The installer's update does it, and a hand-built copy
   needs rebuilding. Do not let one seat open a shared design on v0.91.0 while another still runs
   v0.90.0.
3. **Stop any shared server still running the old binary:**
   `reflow2-mcp --graph-path <path> --stop-shared`. A reconnect alone does not replace it.
4. **Re-take committed design copies with the new binary**, and update any pinned tool schemas.
5. **Open the design.** Nothing needs migrating, and no export or import is required.

## Rolling back to v0.90.0

Restore the snapshot from step 1. Without one, export the design with v0.91.0 and import that
export into a fresh store with v0.90.0 and `--accept-newer`. That works only while the design
holds no `EQUIVALENT_TO` edge and no `bridge` statement: remove them with v0.91.0 first (record a
proven equivalence as a cited biconditional and a bridge as an ordinary premise). v0.90.0 has
no reading for `claimed_by`, `not_read` or an Artifact's grade.

## Also in this release, and what it may ask of you

The CHANGELOG's `[0.91.0]` section has the full list.

- **A claim a named party made reads "claimed by <party>, unverified"**, where it read `unproven`.
- **Two lines that share a premise no longer read independent** unless `independent_except`
  names it.
- **A source marked `not_read` never cites, and an objection nobody has sourced only contests.**
- **`mirror_surface` refuses a refresh that would withdraw what your design rests on** until you
  pass `accept_withdrawal: true`; what it withdraws is kept, marked superseded.
- **`get_skill` and `propagate_from` stop at 30,000 characters by default**, naming what they
  withheld. Pass a larger `budget_chars` if your client has the room.
- **`--call` and `--describe` print compact JSON on one line.** A script that read the old
  pretty-printed layout line by line must parse the JSON instead (pipe to `jq`).
- **Each premise row's `why` is its reason's leading clause;** read the full reason from
  `premise_whys` by the premise's id.
- **Nothing is deprecated or removed.**
