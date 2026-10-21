# Upgrading to v0.90.0

🛑 **Upgrade every seat together, before anyone records an argument, a piece of evidence or a
test with this release's new values.** This release's schema stamp moves by vocabulary. No node
or edge type is added or removed (28 and 68, as in v0.89.0), but the stamp's `enum_values` move:

| | v0.89.0 | v0.90.0 |
| --- | --- | --- |
| Node types | 28 | 28 |
| Edge types | 68 | 68 |
| `statement_kind` (Requirement, Constraint) | theorem, lemma, corollary, proposition, definition, axiom, conjecture, hypothesis, falsum | the same, plus `objection`, `reply`, `unstated_premise`, `method_rule`, `evidence` and `judgement` |
| `Inference.rule` | all, any, k_of_n | the same, plus `immediate` |
| `Constraint.direction` | maximum, minimum | the same, plus `setpoint` |
| `CONTRADICTS.alignment` | opposing, supporting | the same, plus `conceding` |
| `GOVERNED_BY.ruling` | parks | the same, plus `held` |
| New enum properties | — | `source_reliability` (A–F), `information_credibility` (1–6) and `confidence` (low, moderate, high) on Requirement and Constraint; `establishes` and `conclusion_form` on Inference |
| New properties | — | `reported_at`, `silent_on` and `same_formula_as` on Requirement and Constraint; `source` on Constraint; `rule_node` on Inference; `uses` on PREMISE; `moves_to` on CONTRADICTS; `hypothesis_test` and `criteria_at_data` on Verification |

**Every seat** means every binary that opens the design:
- **the flo2.io engine** (and any other hosted server, or the container image at `:0.90.0`);
- **a local `reflow2-mcp`**, the MCP server a harness starts;
- **the `--call` binary** (`reflow2 read` / `reflow2 write`, as in VS Code where MCP is blocked);
- **committed design copies**: an export written by v0.90.0 is refused by an older binary, so a
  CI gate or a teammate on v0.89.0 cannot read a copy this release wrote.

The served tool schemas move with it: nine tools take new arguments or values (below), so a
client that pins tool schemas (a toolsnap, a generated client) moves too.

## What an older binary does

Measured at the cut with the published v0.89.0 release binary (from the release's
`reflow2-mcp-linux-x86_64.tar.gz`, checked against its `checksums.txt`) and this release's build.
Each test design was a premise, a claim and the step between them, written by v0.90.0, plus one
of the new values.

- **v0.89.0 refuses to open a store holding a new value of an indexed property**, naming it:
  > REFUSED: this graph holds Requirement.statement_kind = "evidence" — 1 value(s) this reflow2
  > does not know, on a property it DOES know.

  Measured for `statement_kind: evidence` and `Inference.rule: immediate`; every new
  `statement_kind` is the same property. Nothing is opened or changed.
- **v0.89.0 opens, with its "behind" warning, a store holding `setpoint` or `ruling: held`, and
  reads them wrongly.** These two properties are not checked on opening:
  - a `setpoint` Constraint is judged as a budget: v0.89.0's `budget_report` reads it `within`,
    where v0.90.0 reads it `ungated` (a value held is never a budget);
  - a `GOVERNED_BY` with `ruling: held` keeps its value, but v0.89.0 does not read the held side
    of a dispute.
- **v0.89.0 opens a store holding only the new properties** (`confidence`, `rule_node`,
  `same_formula_as` …) with the warning, keeps them, and does not read them:
  > THIS GRAPH WAS LAST WRITTEN BY reflow2 0.90.0; you are running 0.89.0, which is BEHIND it.
- **v0.89.0 refuses to import an export v0.90.0 wrote** ("you are running 0.89.0, which is
  BEHIND it"). Nothing is written.
  - With `--accept-newer` it still refuses, all or nothing, an export holding any new enum value:
    `statement_kind` `evidence`, `rule` `immediate`, `direction` `setpoint`, `ruling` `held`
    (`invalid enum value …`).
  - With none of them, `--accept-newer` imports it.
- **v0.90.0 opens a v0.89.0 store untouched.** v0.89.0's export of the store before and after
  v0.90.0 opened it had the same content hash, and v0.90.0's own export of it matched. v0.90.0
  imports v0.89.0's export and re-exports the same content. Nothing needs migrating. Opening the
  store does record that v0.90.0 last wrote it, so v0.89.0 warns on it afterwards.

## What you do

1. **Snapshot the store of any server that other people or machines reach** before upgrading it.
   That snapshot is your rollback.
2. **Update every seat above, together.** The installer's update does it, and a hand-built copy
   needs rebuilding. A seat left on v0.89.0 is locked out of the design the moment anyone records
   a piece of evidence, an objection or an immediate step, and misreads a setpoint before that.
3. **Stop any shared server still running the old binary:**
   `reflow2-mcp --graph-path <path> --stop-shared`. A reconnect alone does not replace it.
4. **Re-take committed design copies with the new binary**, and update any pinned tool schemas.
5. **Open the design.** Nothing needs migrating, and no export or import is required.

## Rolling back to v0.89.0

Restore the snapshot from step 1. Without one, v0.89.0 still opens the store (with the warning
above) only while it holds none of the new `statement_kind` values and no `immediate` step.
Re-record any such statement or step with v0.90.0 first. Re-record a `setpoint` Constraint as
`maximum` or `minimum`, or remove it, or v0.89.0 will judge it as a budget. Do not write to the
design from v0.89.0 once anyone has recorded an argument or evidence with the new values.

## Also in this release, and what it may ask of you

The CHANGELOG's `[0.90.0]` section has the full list.

- **A claim resting on a statistical hypothesis test reads `supported` at α, never `shown` or
  `proven`.** A test that does not reject H0 supports nothing.
- **A conclusion held up only by disputes reads `contested`** where it read `unproven`.
- **What rests on an undocumented report reads `conditional`**, and `supported` once a
  checksum-pinned source DOCUMENTS it.
- **A test whose criteria moved after its data reads unknown** until the owner acknowledges the
  `criteria_changed_after_data` gap with a reason.
- **A change walk from a check no longer crosses a GOVERNED_BY**, so a what-if from a check reaches
  less than it did.
- **`loop_status.session` says `counted: false`** on a server that has seen only that call,
  instead of reporting zero writes.
- **Nothing is deprecated or removed.**
