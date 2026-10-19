# Upgrading to v0.88.0

🛑 **Upgrade every seat together, before anyone records a proof with this release's new values.**
This release's schema stamp moves by vocabulary: `statement_kind` (on `Requirement` and
`Constraint`) gains `hypothesis` and `falsum`, so the stamp's `enum_values` move. No node or edge
type is added or removed. An older binary **refuses** a store that holds either new value, and
refuses to import an export that holds one, even with `--accept-newer`. The schema also gains
properties an older binary keeps but does not read:
- `cases` and `cases_exhaustive` on `Inference` (a proof by cases);
- `argued_by` on the `SATISFIES` and `DEPENDS_ON` edges, and `evidence` became writable through
  `satisfies` (it was already declared).

| | v0.87.0 | v0.88.0 |
| --- | --- | --- |
| Node types | 28 | 28 |
| Edge types | 68 | 68 |
| `statement_kind` (Requirement, Constraint) | theorem, lemma, corollary, proposition, definition, axiom, conjecture | the same, plus `hypothesis` and `falsum` |
| `Inference` | — | `cases`, `cases_exhaustive` |
| `SATISFIES`, `DEPENDS_ON` | — | `argued_by` |

## What an older binary does

Measured at the cut with the published v0.87.0 release binary (from the release's
`reflow2-mcp-linux-x86_64.tar.gz`, sha256 `cb08aed7…`, checked against its `checksums.txt`) and
this release's build. The test design was Euclid's proof by contradiction: a definition, a
supposition (`hypothesis`) assumed by one step, the contradiction it yields (`falsum`), and a
reductio that discharges the supposition and concludes the theorem.

- **v0.87.0 refuses to open a store holding `hypothesis` or `falsum`**, naming each value:
  > REFUSED: this graph holds Requirement.statement_kind = "falsum", Requirement.statement_kind =
  > "hypothesis" — 2 value(s) this reflow2 does not know, on a property it DOES know.

  Nothing is opened or changed. One such node is enough: a store with a single `hypothesis` is
  refused the same way.
- **v0.87.0 opens a store v0.88.0 has written that holds neither value, and warns** on every call:
  > THIS GRAPH WAS LAST WRITTEN BY reflow2 0.88.0; you are running 0.87.0, which is BEHIND it.
  > Everything in it still reads, but every write THIS binary makes materialises its own schema
  > defaults … Update reflow2 before writing.

  It keeps the new edge properties: a `SATISFIES` written with `evidence` and `argued_by` reads
  back with both, and its export carries them. It does not read them: no `argument` is shown
  beside the edge, and no `reason_to_exist` on a requirement.
- **v0.87.0 refuses to import an export v0.88.0 wrote.** Nothing is written ("you are running
  0.87.0, which is BEHIND it").
  - With `--accept-newer` it still refuses, all or nothing, an export holding `hypothesis` or
    `falsum` (`invalid enum value 'falsum'`, and every edge to those nodes).
  - With neither value, `--accept-newer` imports it.
- **v0.88.0 opens a v0.87.0 store untouched.** v0.87.0's export of the store before and after
  v0.88.0 opened it had the same content hash, and v0.88.0's own export of it matched. v0.88.0
  imports v0.87.0's export and re-exports the same content. Nothing needs migrating. Opening the
  store does record that v0.88.0 last wrote it, so v0.87.0 warns on it afterwards (above).

**A correction to the v0.87.0 notes.** They said v0.86.0 "imports 0.87.0's export". A binary
behind the export's writer refuses a plain import; it needs `--accept-newer` (`accept_newer: true`
on `import_graph`), as the paragraph above measures for v0.87.0.

## What you do

1. **Snapshot the store of any server that other people or machines reach** before upgrading it.
   That snapshot is your rollback.
2. **Update every binary that opens the design, together:** every machine, every harness and
   every server (a hosted engine, or the image at `:0.88.0`). The installer's update does it, and a
   hand-built copy needs rebuilding. A seat left on v0.87.0 is locked out of the design the moment
   anyone records a `hypothesis` or a `falsum`, so do not leave one behind.
3. **Stop any shared server still running the old binary:**
   `reflow2-mcp --graph-path <path> --stop-shared`. A reconnect alone does not replace it.
4. **Open the design.** Nothing needs migrating, and no export or import is required.

## Rolling back to v0.87.0

Restore the snapshot from step 1. Without one, v0.87.0 still opens the store (with the warning
above) only while it holds no `hypothesis` and no `falsum`. If it holds one, change each such
statement's `statement_kind` with v0.88.0 first: measured, after v0.88.0 re-recorded the
supposition and the contradiction as `proposition`, v0.87.0 opened the store. Its readings of that
proof are then v0.87.0's own. Do not write to the design from v0.87.0 once anyone has recorded a
proof with the new values.

## Also in this release, and what it may ask of you

The CHANGELOG's `[0.88.0]` section has the full list.

- **A theorem, lemma or corollary taken as given now reads `cited`** (or `cited_unverified` when
  no Artifact with a checksum documents it), and what rests on an unverified citation reads
  `conditional`. A proof resting on a recalled theorem moves from shown to conditional until the
  source is registered and drawn with DOCUMENTS.
- **A claim that holds only under an open supposition contests, rather than refutes,** a claim
  outside it.
- **`add_decision` stores `rejected` and `superseded` as given** (they landed `proposed`), and
  an `approver` with no status to sign is not drawn (`approver_not_drawn` on the reply).
- **Re-drawing an edge through a typed helper keeps the properties the call leaves out.** To
  clear one, delete the edge and draw it again, or set the whole bag with `create_edge`.
- **Nothing is deprecated or removed.**
