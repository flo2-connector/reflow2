### Added

- **A claim a named party made reads `claimed`.** `claimed_by` (on a
  statement, on the premise that is the claimant's argument, or on a step)
  names the party: while nothing in the design establishes it and nothing
  refutes it, it and what rests on it read "claimed by OpenAI, unverified" —
  `proof_status` and `statement_reading` `claimed`, `claimed_by` and the `why`
  in `reasoning_report` — never `unproven` (which also means refuted) or
  `open`. A what-if that drops the claimant's argument reads the claim `open`
  again (`conclusion_was: claimed`, `conclusion_reads: open`), and leaves what
  does not rest on it standing. `add_requirement` and `add_constraint` take
  `claimed_by` (req:a-claim-made-by-a-named-party-and-not-yet-verified-reads-
  claimed; the Navier–Stokes map's (C) could only read "unproven").
- **A status that held over a period is read back.** A TemporalFact with
  `fact_type: status`, `valid_from` and `valid_to` on a claim or a step is
  listed under `status_over_time` in `reasoning_report` and on a step's
  `get_node`. A claim or step that does not hold now, and whose every status
  has ended, reads `withdrawn`, with the period and the objection that
  withdrew it: Kempe's 1879 proof reads "withdrawn: it held 1879–1890 …,
  withdrawn by req:obj-heawood-1890-kempe-degree-5". It lived only in dated
  records no report read.
- **A source can say it was named and not read, and carry a grade.**
  `not_read: true` on an Artifact: a DOCUMENTS edge from it never takes a
  claim off `uncited_premises`, what rests on that claim reads uncited, and a
  checksum on it pins nothing. A source can carry evidence's Admiralty grade,
  `source_reliability` (A–F) and `information_credibility` (1–6): graded D–F or
  4–6, it checks no citation, and what it documents reads `cited_unverified`;
  each cited source in `reasoning_report` says its grade and why it pins
  nothing (dec:idea-a-source-says-what-kind-of-support-it-gives, option (c)
  plus a "named, not read" marking; a taxonomy of source kinds stays
  considered). The Navier–Stokes map's NOT READ paper counted as citing.
- **🛑 Upgrade every seat together (no further move for this release): the
  schema stamp widens again.** `Artifact` gains two enum properties,
  `source_reliability` and `information_credibility`; `claimed_by` (string)
  and `not_read` (bool) are added too. PR A1 already moved the stamp for the
  next release, so this costs no second upgrade: every seat upgrades once.

### Fixed

- **A refreshed mirror never deletes what its source withdraws.**
  `mirror_surface` re-mirroring one design refreshes what it holds IN PLACE:
  every local edge into a node the new surface still publishes survives (a
  refresh used to delete and re-create, dropping a consumer's CONSUMES). A
  node the new surface withdraws is KEPT, marked `superseded_by_surface`,
  with a snapshot and a ChangeEvent; the reply lists `superseded`,
  `kept_edges`, `snapshots` and `change_event`. The guard now counts every
  local edge touching a withdrawn node, either way round — a step's
  PREMISE_OF included — and refuses, naming each, unless `accept_withdrawal:
  true`. What rested on a withdrawn node reads REASON LOST:
  `rests_on_withdrawn` in `reasoning_report`, `reasons_lost` and
  `premises_withdrawn` in `loop_status`, until the owner re-grounds the step
  or lets the decision go; `propagate_from` seeds from the withdrawn id (it
  answered unknown_seeds). Measured on the split-capstone trial's two
  KESTREL-ASSESS surfaces (req:a-superseded-mirrored-judgment-is-kept-and-
  what-rested-on-it-reads-reason-lost).
- **A judgment another replaces reads `superseded` and is not published.** A
  requirement `replaces` supersedes reads `superseded` (it read `contested`:
  its own step still held, and the replacement left its truth unknown), and
  `export_surface` no longer publishes it beside its successor.
- **Two lines stated independent are checked against their premises.** The
  check compared steps and cited results only, so three trials read two lines
  "independent" though both rest on one premise: Gettier's two cases on
  Gettier's two points, the Navier–Stokes Lean and natural-language routes on
  the Lean-check claim. A statement neither line derives — taken as given, a
  claim nobody has shown, an assumption nothing discharges — is now named in
  `shared_premises` and makes the statement false, unless
  `independent_except` names it. The vocabulary (definitions, axioms, rules of
  method) and the theorem's own setting (premises both final steps take up)
  are named in `shared_setting` and leave the lines independent; the four
  colour Coq and RSST routes name the 633 configurations there, recorded as a
  definition.
- **`independent_of: []` clears a stated independence.** `add_inference`
  removed a key from the map it MERGED into the stored step, so an empty list
  (or `cases_exhaustive: ""`) left the old value standing.
- **An objection nobody has sourced contests and never defeats.** An
  objection marked `taken_as_given` that no pinned source documents leaves
  what it attacks contested — "contested by an objection nobody has sourced"
  — and never defeats or refutes it; once a pinned source documents it, it
  holds as before. Heawood's recalled, unread objection defeated Kempe's
  degree-5 step outright on the four colour rung (req:an-objection-taken-as-
  given-with-no-pinned-source-contests-and-never-defeats).
