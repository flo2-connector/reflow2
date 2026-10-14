### Fixed

- **`unresolved_setup` no longer reports a setup whose target was already acted on.** The rule
  said "nothing follows through" while reading arrival from a list of its own: a Requirement only
  at `met`, which almost no design sets, and a finding only by `EVOLVES_INTO`. So it went on
  reporting a requirement whose thread already delivers it, a finding a change had closed
  (`INVALIDATES` or a `valid_to`), a finding an accepted Decision answers (`MITIGATES`), a finding
  that led to a decision, change or requirement (`CAUSES` or `TRIGGERS`), and a ChangeEvent or
  DesignRule. On reflow2's own design that was 20 of its 47 findings; it now reports 27. The
  message names what it checked: that the target evolved into nothing and has not arrived, and
  what arriving means for that target's type.
- **`decision_overtaken_by_promotion` and `unresolved_setup` now read "arrived" the same way.**
  They kept two lists that disagreed. Both now read one, which also counts an arrived epoch and a
  realized Artifact for both. The promotion detector now also finds an idea whose requirement is
  delivered by its thread but still reads `proposed`: 4 more on reflow2's own design, 13 in all.
  Each detector keeps the one clause its own definition adds: for the follow-through rule, a
  target that evolved into something; for the promotion detector, a requirement the owner
  accepted.

  **What to do:** nothing. Fewer `unresolved_setup` findings and a few more
  `decision_overtaken_by_promotion` gaps may appear on your next `detect_defects` and
  `detect_gaps`. No reply changed shape. A link between two findings where the later one is
  still open is reported as before.
