### Added

- **🛑 Upgrade every seat together: the schema stamp moves.** One edge type is
  added, `EQUIVALENT_TO` (68 → 69), and `statement_kind` (on Requirement and
  Constraint) gains `bridge`. Measured with the published 0.90.0 binary on
  stores and an export this build wrote:
  - 0.90.0 REFUSES to open ANY store this build has written, even one holding
    neither addition, naming the type it lacks ("it has never heard of
    EQUIVALENT_TO"): a new edge type moves both halves of the stamp.
  - It refuses to import an export holding either addition, all or nothing,
    even with `--accept-newer`.
  - This build opens a 0.90.0 store unchanged ("Additive only — everything in
    it still reads").
  **What to do:** upgrade every
  seat, flo2.io included, before anyone draws `EQUIVALENT_TO` or records a
  `bridge`; stop any shared server still on the old binary
  (`reflow2-mcp --graph-path <path> --stop-shared`). Nothing needs migrating.
- **A bridge premise says a formal statement models a claim of another
  field.** `statement_kind: bridge` ("the Lean statement F expresses (C)",
  "the Coq statement models the theorem"). It reads `bridge`; what rests on it
  reads `conditional` on it (`rests_on_bridges` on a claim and a step, and
  `proof_status` says so); an objection aimed at it contests every claim
  resting on it while the theorem under it stands; `reasoning_report` lists
  it under `bridges`, apart from cited theorems, definitions and uncited
  premises; and `propagate_from` names it under `bridges_lost` on each row it
  costs. The Lucas–Penrose trial tried the three encodings there were: a
  plain premise read "not delivered", an assumed one was immune to objection,
  and a premise taken as given was filed beside Gödel's theorem as a
  foundation (dec:idea-a-bridge-premise-says-a-formal-statement-models-a-claim,
  option b).
- **Two statements can be recorded as proven equivalent.** `create_edge`
  `EQUIVALENT_TO`, naming the biconditional in `by`. It never reads as a
  duplicate: an asserted DUPLICATES beside it is counted under
  `swept.suppressed_as_equivalent` and offered for no merge, HEAL refuses the
  merge in propose and apply, and `possible_duplicate` stays quiet. Neither
  side's reading moves (an open conjecture stays open). `reasoning_report`
  names each claim's `equivalent_to` with how the biconditional reads, and
  `propagate_from` names `equivalents_reached`. The Riemann map had to join RH
  and Robin's inequality with DUPLICATES, and was offered their merge.
- **A step that only restates its gap reads `circular`.** A step whose premise
  is proven equivalent to its conclusion (through any chain of EQUIVALENT_TO
  edges) or is the same formula (`same_formula_as`) carries `circular` on
  `get_node`, is listed under `reasoning_report`'s `circular_steps`, and is put
  to the owner as a `reasoning_not_as_stated` gap. A flag, never a verdict:
  the step reads by its rule as before
  (dec:idea-speculate-the-bridge-that-would-close-a-gap-then-test-it,
  option c).

### Fixed

- **A proof is read as a proof again, whatever its Project's `domain` says.**
  Measured on the heat-equation rung (`domain: document`), detect_gaps asked
  11 questions, nine of them software-shaped: a structure, a quality target,
  a closure criterion, an artifact standard, tests, conventions, a
  deployment, a requirement `kind`, and "unlinked" hypotheses. It now asks 2.
  Three causes (fact:root-cause-proof-statements-fall-through-to-software-
  predicates-and-assumed-premises-stopped-counting-2026-10-22):
  - an ASSUMED premise counts as taking part in the reasoning again. A
    2026-10-17 change narrowed the membership test with the what-if's own
    helper, so a hypothesis only ever assumed read as unlinked intent and a
    proof stopped reading as one;
  - a hypothesis has a truth of its own: it holds only under itself
    (`conditional`), where it read "not delivered", disagreeing with its own
    `statement_reading`;
  - `encoding_undecided` no longer asks a statement (one carrying
    `statement_kind`) for a software `kind`.
  Whether a design is a proof or an argument is now read from what it holds:
  a requirement belongs to the reasoning when it is a statement, takes part
  in a step, or is joined by a CONTRADICTS to one that does. The `domain`
  hint is read only when that fails. A reasoning design is no longer asked
  for tests, conventions, a deployment or a registered agent-instructions
  file; a design that builds something and also holds a proof is no longer
  asked what capability satisfies each of its statements.
- **A conclusion reached through a contested contradiction reads
  `contested`, not `unproven`.** A reductio whose ⊥ rests on a contested
  premise is a contested route, and the dispute is carried through the ⊥. The
  Lucas–Penrose conclusion read "unproven, no derivation holds" while every
  objection against it stood on a translation step under the ⊥.
- **A what-if walks through a contradiction.** `propagate_from` reads a ⊥ by
  its derivation, so a change that costs the derivation reaches the reductio
  resting on it and what rests on its conclusion; it stopped at the ⊥, and
  the row deriving it read `refuted` (the Lucas–Penrose and four colour
  trials).
- **A falsum carries no proof status.** It read `unproven` beside its
  statement reading `false`; its `statement_reading` (`false`,
  `inconsistent` or `stated`) is what it says.
