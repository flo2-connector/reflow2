# Upgrading to v0.98.0

**Upgrade every seat.** This release's schema stamp moves by vocabulary. No node or edge type is
added or removed (28 and 69, as in v0.97.0), but the stamp's `enum_values` gain one vocabulary:

| | v0.97.0 | v0.98.0 |
| --- | --- | --- |
| Node types | 28 | 28 |
| Edge types | 69 | 69 |
| Enum vocabularies | 83 | 84 |
| `Decision.closing_it` | — | a new vocabulary: `closes`, `narrows`, `restates` |
| New properties | — | `hole` and `serves` on Decision; `refuter`, `waiting`, `promise` and `promise_rank` on Artifact; each optional |

Unlike v0.97.0's move, nothing here locks an older seat out: v0.97.0 does not know these
properties at all, so it keeps them as stored rather than refusing them. What an older seat
cannot do is read a proof search's map, and a round it records cannot be judged.

**Every seat** means every binary that opens the design:
- **the flo2.io engine** (and any other hosted server, or the container image at `:0.98.0`);
- **a local `reflow2-mcp`**, the MCP server a harness starts;
- **the `--call` binary** (`reflow2 read` / `reflow2 write`, as in VS Code where MCP is blocked);
- **anything that imports an export**: an export written by v0.98.0 is refused by an older
  binary's plain import.

The served tool schemas move with it, and no tool is added or removed (199 on both):
`add_decision` takes `hole`, `serves` and `closing_it`; `register_alternative` and `add_artifact`
take `refuter`, `waiting`, `promise` and `promise_rank`; `reasoning_report` takes `map` and
`part_offset`; `record_finding`'s `value` is written by reflow2 on a round. A client that pins
tool schemas (a toolsnap, a generated client) moves too.

## What an older binary does

Measured at the cut with the published v0.97.0 release binary (from the release's
`reflow2-mcp-linux-x86_64.tar.gz`, checked against its `checksums.txt`) and this release's build.
Each test design was a conjecture and a lemma written by v0.98.0, then, in turn: nothing more; a
hole's decision point with `hole` and `serves`; the same with `closing_it`; a charted route
(`refuter`, `waiting`, `promise`, `promise_rank`) and one recorded round.

- **v0.97.0 opens a store v0.98.0 wrote**, every one of the four, with its "behind" warning:
  > THIS GRAPH WAS LAST WRITTEN BY reflow2 0.98.0; you are running 0.97.0, which is BEHIND it.

  It reads the new fields back as stored (`closing_it: "closes"`, `promise_rank: 1`, a round's
  `value`), and a revise it makes of the decision point keeps them. The store's stamp still reads
  0.98.0 after that write.
- **v0.97.0 cannot read the map.** It refuses `reasoning_report` with `map` (exit 2, "`map` is
  not a parameter of this tool"), and `add_decision` and `register_alternative` with the new
  fields, the same way. Nothing is written.
- **A round v0.97.0 records carries no readings of the path.** v0.98.0 lists it under
  `rounds.not_judged` and counts no rounds past it (`since_progress` reads `null`), rather than
  guess.
- **v0.97.0 refuses a plain import of any v0.98.0 export** ("you are running 0.97.0, which is
  BEHIND it"). Nothing is written. With `--accept-newer` it imports each of the four whole, the
  new fields kept.
- **v0.97.0 can still draw an edge from a node to itself** (a second `register_alternative` for
  the same alternative, `create_edge`, `record_finding` naming itself as its cause), which
  v0.98.0 refuses. Measured on a design v0.97.0 wrote with three: v0.98.0 opens it, its import
  keeps all three and names each ("kept as the document holds it; remove it with delete_edge"),
  and `detect_defects` reports each node recorded as contradicting itself.

## What you do

1. **Snapshot the store of any server that other people or machines reach** before upgrading it.
   That snapshot is your rollback.
2. **Update every seat above.** The installer's update does it, and a hand-built copy needs
   rebuilding. A seat left on v0.97.0 works the design as before, but records rounds the map
   cannot judge, and cannot read the map.
3. **Stop any shared server still running the old binary:**
   `reflow2-mcp --graph-path <path> --stop-shared`. A reconnect alone does not replace it.
4. **Re-take committed exports with the new binary**, and update any pinned tool schemas.
5. **Run `reflow2 update` in each project** that `reflow2 init` set up. For the VS Code terminal
   route it changes one file, `.github/skills/work-a-proof-hole/SKILL.md` (the skill's
   description), and nothing else.
6. **Open the design.** Nothing needs migrating, and no export or import is required. A proof
   recorded before this release reads as it did; to use the map, give each hole's decision point
   `hole`, `serves` and `closing_it`, and record each round's end with `record_finding`
   (`fact_type: search_round`, no `value`).

## Rolling back to v0.97.0

Restore the snapshot from step 1. Without one, v0.97.0 opens the store as it stands, with the
warning above, and keeps the new fields. Rounds recorded by v0.98.0 keep their readings, and a
later v0.98.0 judges them again.

## Also in this release, and what it may ask of you

The CHANGELOG's `[0.98.0]` section has the full list.

- **`reasoning_report` reads one claim larger than the budget in parts.** Such a claim comes as
  its head with `in_parts`, and its steps, failed routes and objections in `parts`; read on with
  `part_offset` (the claim's `in_parts.next_part_offset`, with the same `offset`). A reader that
  follows only `next_offset` still reads every claim's head.
- **Seventeen relations can no longer be drawn from a node to itself** on any door (the schema's
  `irreflexive: true`; TRIGGERS is excepted). A script that drew one to stand for "this is its
  own X" gets a refusal naming the relation.
