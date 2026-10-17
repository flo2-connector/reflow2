### Changed

- **A computed check holds an inference premise only through its kept record.**
  A Verification made by computation — `computed: true` on `add_verification`
  (new), or a check whose `location` or record names a `.calc.json` file — reads
  `passing` as a premise only while it names a kept record: an Artifact with a
  checksum that DOCUMENTS the check, that the check PRODUCES, or that its
  `location` names (by `art:` id or by the Artifact's own location), with no open
  drift on it, that states the result (the PRODUCES edge's `outcome`, else the
  check's `findings`). With no such record the premise reads `unknown`, and its
  `why` says the check is **asserted, not passing** and names the missing record.
  A record whose `outcome` is `fail` makes the check fail. **Who notices:** a
  proof or argument whose premise is a hand-set computed check. **Every other
  check reads exactly as in 0.85.0**, and nothing changes for a design with no
  Inference. **What to do:** register the calculator's record as an Artifact with
  its checksum and name it in the check's `location` (or link it).
- **A claim that loses its support reads unsupported, never false.** A node an
  Inference concludes now holds when one of its steps holds and is otherwise
  `unknown` ("unsupported: …") — a failing step no longer makes its conclusion
  `fails`. Only an objection that holds refutes a claim. In the what-if
  (`propagate_from`, `propagate_change`), a lost premise counts as unsupported:
  a step that loses its support reads `unsupported` (new value) rather than
  `broken`, which is kept for a step whose premise really fails; and each row
  says what its claim is left with — `conclusion_reads`: `still_supported`
  (with `still_supported_by`, the other lines), `unsupported` or `refuted`.
  **Who notices:** a client reading `impact` or `conclusion_after` on a
  design with Inferences; `conclusion_after: "fails"` from a loss alone no
  longer occurs.
- **A what-if stays inside the reasoning it affects.** Along PREMISE_OF and
  CONCLUDES a radius walks down to what rests on the changed statement and up to
  its own grounds, and never turns: it no longer reaches every statement that
  shares a definition with it, nor an unrelated conjecture as a risk crossing.
  Nothing changes for a design with no Inference.
- **The software-shaped phase questions are not asked of a proof.**
  `concept_without_design` and `quality_target_unstated` stay silent on a
  REASONING DESIGN: one holding at least one Inference, no Capability, Component
  or Flow, and whose every live Requirement takes part in the reasoning (a
  premise or conclusion of a step, or joined by CONTRADICTS to one). One
  statement outside the reasoning brings them back. `unlinked_intent` counts a
  PREMISE_OF or CONCLUDES as a relation.

### Added

- **A refutation reads as refuted, not as a defect.** In a line of reasoning, an
  opposing CONTRADICTS whose source holds — a counterexample doing its job — is
  no longer a `contradiction` defect and no longer counts in `loop_status`'s
  `structural_defects`. It is counted in `detect_defects`'
  `swept.suppressed_by_refutation` (present only when non-zero). A contradiction
  between two claims that hold, or are unknown, is still reported.
- **The loop sees what a reasoning owes.** On a design holding Inferences,
  `loop_status` carries `reasoning`: unconfirmed steps, uncited premises, claims
  that read `shown` but not `proven`, and premises taken as given, with their ids
  (at most 20 each); `next` says it, and it is never counted against `clean`.
  `detect_gaps` raises one low-severity `unconfirmed_reasoning` gap. A design
  with no Inference reads byte for byte as before.
- **A definition or axiom can be taken as given.** `taken_as_given: true` on
  `add_constraint` and `add_requirement` marks a stated foundation:
  `reasoning_report` lists it under `stated_foundations`, not `uncited_premises`.
  A Requirement taken as given holds as a premise.
- **A single point of failure in a line of reasoning is named.**
  `detect_defects` reports `single_point_of_failure` for a lemma every holding
  line of a claim rests on (the nearest such lemma, never a definition or axiom),
  checked by counting it lost.

### Fixed

- **`gaps_to_prompts` and `gap_to_prompt` need only the gap's `id`,** as their
  descriptions said: `{"gap": {"id": "gap:…"}}` works, and a row with no `id` is
  refused naming the field.
- **`describe_schema` keeps to the `budget_chars` it is given.** A reply still
  over it after its prose is cut keeps every edge's type and endpoints and every
  property's name, type and values, and, if need be, shortens its lists while
  `all_names` keeps every name; `budget.compacted` says which. With no
  `budget_chars` it answers as before.
- **A capped `get_instructions` returns its first section whole** with the
  manifest (`budget.detail: first_section_only`, `next_section`), instead of the
  manifest alone, whenever that section fits.
- **A check whose `location` names a kept record counts it as cited** in
  `reasoning_report`, with no `documents` link needed.
- **`add_verification`'s `verifies` field states its shape in its first
  sentence** (`{"target_id": …, "target_type": …}`), so a short describe shows it.
