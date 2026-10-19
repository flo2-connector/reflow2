### Added

- **🛑 Upgrade every seat together: the schema stamp moves.** `statement_kind`
  (on Requirement and Constraint) gains two values, `hypothesis` and
  `falsum`, so the stamp's `enum_values` move. No node or edge type is added.
  Measured with the published 0.87.0 binary on a store and an export this
  build wrote:
  - 0.87.0 REFUSES to open a store holding either value, naming it
    (`Requirement.statement_kind = "falsum"`).
  - It opens a store this build wrote that holds neither, with its usual
    note.
  - It refuses to import an export holding either value, all or nothing,
    even with `--accept-newer`.
  - This build opens a 0.87.0 store unchanged.
  Update every machine, harness and server that opens a design together,
  before anyone records a proof with the new values. **What to do:** upgrade every
  seat; stop any shared server still on the old binary
  (`reflow2-mcp --graph-path <path> --stop-shared`). Nothing needs migrating.
- **A contradiction is a statement of its own, and it reads false.** A
  `falsum` (statement_kind `falsum`) is the contradiction a reductio derives.
  It reads `false`, never `holds`, and a reductio resting on it reads it by
  whether it is derived, so the reductio holds and discharges its
  supposition. A falsum derived under no open assumption reads
  `inconsistent`: the foundations contradict each other, and `loop_status`
  and `detect_gaps` say so. On the π+e trial the contradiction had to be
  drawn inside the reductio's warrant.
- **A hypothesis has a kind of its own.** `statement_kind: hypothesis` is a
  supposition, an induction hypothesis or a case. It reads `assumed` while
  open and `discharged` once a step discharges it, where it read "stated, not
  shown" (the Arrow and π+e trials).
- **A proof by cases names its cases and what makes them all the cases.**
  `add_inference` takes `cases` (the case hypotheses, discharged by the step
  and refused unless assumed beneath it) and `cases_exhaustive` (the record
  saying the cases cover every possibility, refused unless it names a node,
  and drawn as a premise). The step reads `cases` with `declared` and that
  record's truth, and holds only while it holds. Exhaustiveness is the
  owner's word or a checker's, never reflow2's. A split nothing declares
  exhaustive is counted (`cases_not_declared_exhaustive`) and raised as a
  `reasoning_not_as_stated` gap.

### Changed

- **A result cited from elsewhere reads `cited`, and a recalled one reads
  `cited_unverified`.** A theorem, lemma or corollary taken as given is a
  citation, never an axiom. (A proposition taken as given stays what it was: a
  stated position an argument starts from.) It reads `cited` when an Artifact with
  a checksum DOCUMENTS it (its source, read and pinned), and
  `cited_unverified` otherwise. What rests on an unverified citation reads
  `conditional` on it (`rests_on_unverified` on the claim and the step),
  never plain `holds`; `loop_status` counts `unverified_citations`. **What to
  do:** a proof resting on a recalled theorem now reads conditional rather
  than shown. Register the source you checked as an Artifact with its
  checksum and draw DOCUMENTS to the citation; the served `design-a-proof`
  skill says how. Measured on the trial designs: π+e's theorem moves from
  shown to conditional on Lindemann and the algebraic-closure theorem.
- **The design-a-proof skill has a recipe for each of these, and agrees with
  the tools.** It said a theorem is never taken as given while a cited
  theorem is exactly that; it now says one way to record a citation, how a
  recalled reference reads, and how it becomes checked.

### Fixed

- **A claim that holds only under an open supposition no longer refutes a
  claim outside it.** An objection holding only under an assumption no step
  discharges (or only if an unverified citation is right) used to refute its
  target outright. Measured on the cogito trial (flo2.io 989598dd02246a81):
  `req:ai-not-res-cogitans` read refuted by a verdict that held only under
  "suppose an AI authentically doubts". Such an objection now CONTESTS the
  target and names the supposition (`holds_only_under` on the objection); it
  refutes only a target that holds under the same assumptions. The same
  class, fixed at each place it was found:
  - a conditional reply no longer answers an objection (so it no longer
    reinstates the claim the objection attacked);
  - `detect_defects`' reading of an attack follows the scoped standing;
  - a requirement concluded only by a conditional step is no longer DELIVERED
    by the roll-up;
  - a `met` or `accepted` status resting only on a conditional result is
    reported as `conclusion_does_not_follow`.
  **What to do:** where a claim now reads contested, discharge the
  supposition into a statement of its own ("if S, then not T") and let that
  object, or keep the dispute open on purpose.

- **A generalization no longer holds over a variable an open assumption
  names.** A step that generalized over F while a hypothesis about F was still
  open (undischarged) read `holds`, which the eigenvariable rule of natural
  deduction forbids: reflow2 would certify an invalid universal
  generalization (the Arrow trial). The step now does not hold while an
  assumption open at it lists the variable in its `variables` or `fixes`, or
  declares no `variables` at all; the reading names it under
  `generalization.named_by_open_assumptions` or
  `undeclared_open_assumptions`. Discharging and generalizing on one step (a
  conditional proof over F) is read discharge first and still holds. **What
  to do:** give every hypothesis its `variables` (an empty list when it names
  none), and discharge a hypothesis before the step that generalizes over a
  variable it mentions.
