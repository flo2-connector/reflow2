# Upgrading to v0.86.0

🛑 **Upgrade every seat together, before anyone records a proof with this release's new fields.**
This release's schema stamp moves by vocabulary, and by one node type fewer: `Anchor`, announced in
v0.85.0, is removed. No node or edge type is added, so an older binary does not refuse the store. It reads the design, and it MISREADS a proof
that uses the new fields. The schema gains:
- a new enum property, `statement_kind`, on `Requirement` and `Constraint` (the stamp's
  `enum_values` gain both);
- `taken_as_given`, `variables` and `fixes` on `Requirement` and `Constraint`, and `computed`,
  `variables` and `fixes` on `Verification`;
- `discharges`, `generalizes_over`, `independent_of` and `independent_except` on `Inference`;
- `role` (`premise`, `assumed`) on the `PREMISE_OF` edge.

Two properties also loosen: `Question.gap_id` is no longer required (a question an agent put of its
own has none), and `Constraint.direction` has no schema default any more.

| | v0.85.0 | v0.86.0 |
| --- | --- | --- |
| Node types | 29 | 28 (`Anchor` removed) |
| Edge types | 68 | 68 |
| `statement_kind` (Requirement, Constraint) | — | theorem, lemma, corollary, proposition, definition, axiom, conjecture |
| `PREMISE_OF.role` | — | premise, assumed |
| `Question.gap_id` | required | optional |
| `Constraint.direction` default | `maximum` | none (`add_constraint` still writes `maximum` when given a `limit`) |

## What an older binary does

This was measured at the cut with the published v0.85.0 release binary (sha256 `18d54599…`,
checked against the release's `checksums.txt`) and this release's build. The test design was
Euclid's proof by contradiction: two definitions, a supposition assumed by one step and
discharged by the next, a theorem and a corollary, and one question put to a named person with
`ask_question`.

- **v0.85.0 opens a store that v0.86.0 has opened or written, and warns.** It reads everything, and
  says on every call:

  > THIS GRAPH WAS LAST WRITTEN BY reflow2 0.86.0; you are running 0.85.0, which is BEHIND it.
  > Everything in it still reads, but every write THIS binary makes materialises its own schema
  > defaults onto records the newer reflow2 left implicit … Update reflow2 before writing.

  A one-shot `--call` leaves the stamp alone. **An MCP server on v0.85.0 re-stamps the store as
  its own** once it has warned: v0.86.0 then reads it as "written by reflow2 0.85.0 … Additive
  only", and nothing records any more that it holds vocabulary v0.85.0 does not know. Nothing is
  lost: v0.85.0's export of that store carries every node, edge and property, `statement_kind`
  and `discharges` included (the content hash was identical to v0.86.0's own export).
- **v0.85.0 misreads a proof that assumes something.** It ignores `role: assumed` and
  `discharges`:

  | | v0.86.0 | v0.85.0 |
  | --- | --- | --- |
  | the theorem | `shown` | `unproven` |
  | the contradiction (rests on the supposition) | `conditional` | `unproven` |
  | the theorem's foundations | the two definitions | the two definitions **and the discharged supposition** |

  It does not check a generalization over a fixed variable or a stated independence, and it shows
  no `statement_reading`. A question put with `ask_question` is listed by its `open_questions`.
- **v0.85.0 refuses to import an export that v0.86.0 wrote.** Nothing is written ("this document
  was written by reflow2 0.86.0, and you are running 0.85.0, which is BEHIND it").
  - With `accept_newer: true` it still refuses, all or nothing, an export holding a question put
    with `ask_question`: `Question.gap_id: required property is missing`.
  - With no such question, `accept_newer: true` imports it, and writes `direction: maximum` onto
    each definition recorded as a Constraint (its report's `materialized` says
    `Constraint.direction: 2`).
- **v0.86.0 opens a v0.85.0 store untouched.** Its content is unchanged: v0.85.0's export of the
  store before and after v0.86.0 opened it had the same content hash, and v0.86.0's own export of it
  matched as well. v0.86.0 imports v0.85.0's export and re-exports the same content. Nothing needs
  migrating. Opening the store does record that v0.86.0 last wrote it, so v0.85.0 warns on it
  afterwards (above).

## The `Anchor` node type is gone

v0.85.0 announced it; this release removes it (`dec:anchor-retire-or-implement`). Nothing in
reflow2 ever created or read one. Measured with stores the published v0.85.0 binary wrote and this
release's build:

| | v0.86.0 does |
| --- | --- |
| a store holding no Anchor (every store we know of) | opens, and says once: "It names Anchor, which this reflow2 retired, and holds none — everything in it still reads." |
| a store holding one Anchor | refuses to open, naming it: "THE GRAPH ACTUALLY HOLDS Anchor", with the way through below |
| an export holding one | refuses the import, all or nothing, naming `Anchor` as retired with the decision; nothing is written |
| `create_node` with `node_type: "Anchor"` | refuses, naming the type; nothing is written |

**If a design holds an Anchor**, with v0.85.0: re-express what it recorded (a baseline is
`add_epoch` with `epoch_type` `baseline` and its `checksum`; a node's state at it is a Snapshot from
`snapshot_before_change`), then `delete_node` it. Measured: after v0.85.0 deleted the one Anchor in a
store, this release opened that store. v0.85.0 still opens a store this release has opened (with the
warning above), so the order can be undone.

## What you do

1. **Snapshot the store of any server that other people or machines reach** before upgrading it.
   That snapshot is your rollback.
2. **Update every binary that opens the design, together:** every machine, every harness and
   every server (a hosted engine, or the image at `:0.86.0`). The installer's update does it, and a
   hand-built copy needs rebuilding. A seat left on v0.85.0 keeps working, and reads a proof that
   uses the new fields wrongly, so do not leave one behind.
3. **Stop any shared server still running the old binary:**
   `reflow2-mcp --graph-path <path> --stop-shared`. A reconnect alone does not replace it.
4. **Open the design.** Nothing needs migrating, and no export or import is required, unless the
   design holds an `Anchor`: delete it with v0.85.0 first (above).

## Rolling back to v0.85.0

Restore the snapshot from step 1. Without one, v0.85.0 still opens the store (with the warning
above) and keeps every property, but it misreads any proof recorded with `assumed` or
`discharges`, and its first write materialises its own defaults. Do not write to the design from
v0.85.0 once anyone has recorded a proof with the new fields.

## Also in this release, and what it may ask of you

The CHANGELOG's `[0.86.0]` section has the full list.

- **A computed check holds an inference premise only through its kept record.** A check made by
  computation reads `passing` as a premise only while it names a kept record with a checksum. A
  hand-set computed check now reads `unknown` ("asserted, not passing"). Register the
  calculator's record as an Artifact with its checksum and name it in the check's `location`.
- **The search index needs nothing.** Full-text search moves to Tantivy 0.27, whose index format
  is the one 0.26 wrote. A store opened by this release keeps its index as it is (every file
  byte-identical, measured on a 7,513-node design), searches answer with the same hits and
  scores, and `loop_status` reports no `search_index_rebuilt_on_open`. An index this release has
  written to is read in full by v0.85.0, so rolling back loses no search.
- **A claim that loses its support reads unsupported, never false.** `conclusion_after: "fails"`
  from a lost premise alone no longer occurs. A client reading `impact` should read the new
  `unsupported` value and each row's `conclusion_reads`.
- **The `Anchor` node type is removed** (above). A design that never had one needs nothing.
