### Added

- **🛑 Upgrade every seat together: the schema stamp moves.** Two edge types
  are added, `PUTS_TOGETHER` and `CLOSES` (69 → 71), `Flow.flow_type` gains
  `assembly`, and `INTERACTS_WITH.interaction` names a fifth value,
  `performs` (free text, as before). Measured with the published 0.98.0
  binary (its release checksum checked) on stores and an export this build
  wrote:
  - 0.98.0 REFUSES to open ANY store this build has written, even one
    holding neither addition, naming the types it lacks ("it has never heard
    of CLOSES, PUTS_TOGETHER"): a new edge type moves both halves of the
    stamp.
  - It refuses to import an export holding an assembly step, all or
    nothing, even with `--accept-newer`, naming each item it cannot read
    (the `assembly` value, each `PUTS_TOGETHER` and `CLOSES`).
  - This build opens a 0.98.0 store unchanged ("Additive only — everything
    in it still reads").
  **What to do:** upgrade every seat, flo2.io included, before anyone
  records a step's parts or joins; stop any shared server still on the old
  binary (`reflow2-mcp --graph-path <path> --stop-shared`). Nothing needs
  migrating.
- **A step of a process names the parts it puts together and the joins it
  closes.** `part_of_flow` takes two optional lists: `puts_together`
  (Component ids; one `PUTS_TOGETHER` from the step to each part it places,
  joins or assembles) and `closes` (Interface ids; one `CLOSES` to each join
  it makes, after which what the Interface says must hold there holds). One
  call records a step with its parts and joins, and `draw_edges` takes the
  same lists. All or nothing: an id that is not a Component, or not an
  Interface, or that names nothing, refuses the whole call naming each one,
  and nothing is written. Re-sending a step keeps the parts and joins it
  leaves out (`delete_edge` takes one away). The reply names every edge the
  call drew in `edges_drawn`, and one that was already there apart. A step
  named without either list reads and replies exactly as before
  (req:a-step-of-a-process-names-the-parts-it-puts-together).
- **An assembly reads back as instructions.** A Flow of `flow_type:
  assembly` (how a thing is made and put together: printed, cast, polished,
  plated, set) reads back in `flow_report` with, on each step:
  `puts_together`, `closes` (each join with `what_must_hold`, the
  Interface's own description), `performed_by` (each Actor that
  INTERACTS_WITH the step with `interaction: performs`), `checks` (each
  Verification that VERIFIES it, with its `method` and `status`) and
  `limits` (each Constraint or DesignRule that CONSTRAINS it). Each list is
  written only when it has something in it. Its confessions say, in plain
  words, a step that puts together no part and closes no join, a step
  nobody performs, a step nothing checks, and a join between two of the
  flow's own parts (an Interface one part PROVIDES and another CONSUMES)
  that no step closes. `derived_report` counts those joins
  (`assembly_instructions`). Every other flow reads byte for byte as it
  did: a step of a process already carries checks, limits and actors, and
  its report does not start listing them. A flow of another type whose
  steps name parts or joins lists those, and says it is not read as
  instructions until it is recorded with `flow_type: assembly`.
- **A changed part or join reaches the step that makes it.**
  `propagate_from` walks `PUTS_TOGETHER` and `CLOSES` as it walks
  `REALIZES`: from a changed bezel seat to the step that sets the stone,
  from a changed band to the step that prints it, and from a changed step
  to the parts and joins it makes.
