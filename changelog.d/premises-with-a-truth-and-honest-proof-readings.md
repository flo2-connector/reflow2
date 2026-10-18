### Changed

- **A measured finding counts as a premise.** A TemporalFact used as a premise of
  a step used to read "reflow2 computes no truth", so a choice resting on a
  measurement could never hold. A finding now reads `holds` when it is measured
  (or states no basis, which the schema reads as measured) and still stands:
  nothing INVALIDATES it, it states no end (`valid_to`, or a VALID_TO epoch),
  and no accepted decision discontinues it. A `forecast`, an ended or an
  invalidated finding reads `unknown`, with the reason. **What to do:** nothing;
  steps resting on a standing finding may now hold where they read unknown.
- **An accepted Decision a step concludes reads its owner's acceptance and its
  chain side by side.** It used to read only its chain's verdict, so a first,
  incomplete chain made an accepted decision read unsupported. It now holds on
  its acceptance, and `reasoning_report` carries `acceptance` and `chain` (the
  chain's own verdict and why) on it. The change walk still shows a lost chain:
  the row reads `conclusion_reads: accepted_without_a_holding_chain`, and the
  `conclusion_does_not_follow` finding still asks about the chain. A settled
  Decision's `says` is now its `chose`, not the deliberation's text.
- **A ChangeEvent never reads false.** It is history: an objection to it no
  longer makes it `fails`. A counter-argument is recorded as a statement of its
  own (a Requirement with `statement_kind`) that CONTRADICTS the claim, and
  answered there. **What to do:** a ChangeEvent drawn as the objection to a
  claim now leaves that claim contested until the argument is re-recorded as a
  statement and answered; the served skills say how.
- **The served skills agree that a Decision may be a chain's conclusion** when
  its premises are checkable. `capture-intent` used to say an Inference never
  concludes a Decision, while `brainstorm` and the served instructions said it
  could.
- **`graph_report` takes `budget_chars`** (default 30,000) and trims prose to
  fit, every count and id kept, with a budget block saying what it withheld. On
  reflow2's own design the report had grown to 31,137 characters with no way
  to narrow it. **What to do:** pass a larger `budget_chars` if your client has
  the room and you want every word.

### Fixed

- **A discharged assumption is no longer listed under `unsupported`** in
  `reasoning_report`; it is read under `assumptions` ("assumed, discharged by
  …"). Measured on two proof trials: π+e's supposition and all 8 of Arrow's
  assumptions read unsupported. An assumption that is also an ordinary premise
  somewhere is still listed.
- **A claim that holds only under an undischarged assumption reads
  `truth: conditional`** in `reasoning_report`, naming the assumption, where it
  read `holds` beside `proof_status: conditional`.
- **A settled or deliberate objection is no longer a structural contradiction.**
  In a line of reasoning (a step's premise or conclusion, or a statement), an
  opposing CONTRADICTS whose objection is answered, or that stands undecided and
  states its reason in `evidence`, is counted under
  `swept.suppressed_as_objection` instead of being reported by `detect_defects`
  and counted by `loop_status`. reasoning_report reads it as answered or still
  contested. A CONTRADICTS outside any line of reasoning, or a standing one
  that states no reason, is still reported.
