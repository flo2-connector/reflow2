### Added

- **A proof step can assume something and later discharge it.** `add_inference`
  takes `assumed`: premises the step takes up as hypotheses — a reductio's
  "suppose the primes are finite", an induction hypothesis — drawn as PREMISE_OF
  with `role: assumed`. An assumed premise holds inside the argument whatever its
  own truth, and everything resting on the step carries it until a later step
  names it in `discharges` (refused unless something beneath that step assumes
  it). A discharged assumption is gone from the conclusion's truth, its
  `foundations` and the what-if: losing it reaches no step. A claim that holds
  only under an undischarged assumption reads `conditional` (a new
  `proof_status`), and `reasoning_report` reads each assumption "assumed,
  discharged by …" or "assumed, never discharged". **Who notices:** nobody whose
  design records no `assumed` premise; every existing PREMISE_OF reads as before.
- **A step can generalize over a declared variable, and reflow2 checks it.**
  `add_inference` takes `generalizes_over` (`L`, `n`), and `add_requirement`,
  `add_constraint` and `add_verification` take `variables` (what a record
  mentions) and `fixes` (what it binds to one value, as `n=40`). A step
  generalizing over a variable that anything beneath it fixes does not hold: its
  verdict reads `unknown` with the reason, so its conclusion is unsupported, never
  refuted. Structural, not a logic: the same name is the same variable.
- **A statement can say what it is.** `statement_kind` on `add_requirement` and
  `add_constraint`: `theorem`, `lemma`, `definition`, `axiom` or `conjecture`, a
  field of its own beside `kind`, whose values stay software's. A definition or
  axiom reads taken as given. Reads carry `statement_reading`: a conjecture reads
  `open`, then `refuted` once an objection that holds refutes it; a theorem reads
  `stated` until a step concludes it.
- **Two lines of reasoning can be stated independent, and the statement is
  checked.** `independent_of` (the other step) and `independent_except` (the
  lemmas both may share) on `add_inference`. The lines may share no step other
  than those under the named lemmas; a step's reading carries `independence`.
- **The reasoning says where it is not as stated.** A new gap source,
  `reasoning_not_as_stated`, one finding per assumption no step discharges, per
  generalization over a fixed variable, and per stated independence the lines do
  not bear out. `loop_status.reasoning` counts each (`undischarged_assumptions`,
  `generalizations_not_holding`, `independence_not_borne_out`, with ids), and
  `next` says it. The `unconfirmed_reasoning` gap is unchanged.
- **An agent can put its own question to a person.** `ask_question`: a question
  no detected gap raised, `asked_of` a named Contributor and linked (ASKS_ABOUT)
  to the nodes it is `about`, every name checked before anything is written. It
  waits like any question: `open_questions` lists it (rows now carry `origin`,
  `gap` or `asker`), `loop_status` counts it and lists it for the person it was
  put to, and `answer_question` records who answered. Its wording never changes
  once asked.
- **A served skill teaches designing a proof**: `design-a-proof` (`/design-a-proof`),
  which `find_skills` finds from "design a proof" and "prove a theorem".

### Changed

- **A definition or axiom recorded as a Constraint carries no budget words.**
  `Constraint.direction` has no schema default any more: `add_constraint` writes
  `maximum` itself when it is given a `limit` and no direction, so a budget reads
  and exports exactly as before, and every reader still reads an absent direction
  as `maximum`. A Constraint stored with a direction keeps it. **Who notices:** a
  client creating a Constraint with no `limit` through `add_constraint` or
  `create_node` no longer sees `direction: maximum` on it.
- **`Question.gap_id` is optional**: a question an agent put of its own has none.
- **Upgrade every seat together before recording a proof with these fields.**
  The schema stamp moves by vocabulary an older binary does not police: the new
  enum property `statement_kind` (Requirement, Constraint), plus edge and
  `list:string` properties; no node or edge type is added. MEASURED with the
  published v0.85.0 binary (checksum verified) on a store this build wrote:
  - **v0.85.0 opens the store with no warning** ("additive only") and re-stamps
    it, dropping the new keys from the stamp, so nothing records that it holds
    vocabulary v0.85.0 does not know.
  - **It then MISREADS a proof that assumes something**: it ignores `role:
    assumed` and `discharges`, so Euclid's reductio reads `unproven` ("no
    derivation holds") where this build reads `shown`, and it does not check a
    generalization or a stated independence. Nothing is lost: v0.85.0 keeps
    every property and its export carries them back.
  - **v0.85.0 refuses to import an export holding an `ask_question` question**
    (`Question.gap_id: required property is missing`), all or nothing; a
    release also refuses any newer document by version unless `accept_newer`,
    and v0.85.0 would then write `direction: maximum` onto each definition
    recorded as a Constraint.
  - **This build opens and imports v0.85.0's stores and exports unchanged.**
