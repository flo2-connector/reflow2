### Added

- **A composed check shows where its designs already touch, before anyone
  declares it.** `compose_and_analyse` (one other design or several) and
  `pair_designs` read the records the designs already hold. A reference one
  design records to a node of another is a `declared` link. Exact evidence is
  a `computed` link: the same file in both (an artifact location in the same
  repository, the repository being the source a pin naming that design
  states, or the same URL; or an equal checksum), or a pin in one design
  naming the other's graph_id or source repository. In a composition each
  link is drawn as a real edge in the throwaway combined graph, so the
  ordinary checks see across it: a design joined to the others by one no
  longer reads as a cut-off cluster beside them. Nothing is written.
- **A similar name is a candidate, never a link.** Interfaces in two designs
  with closely matching names and the same medium are listed under
  `candidates` for a person to confirm; they are never drawn and never read by
  a check. A bare relative path (README.md, LICENSE, Cargo.toml) is never
  evidence, nor is an equal checksum of an empty or stock file; both are
  counted under `set_aside`.
- **Every pair says what was checked.** `link_status` files every pair of
  designs as `linked`, `candidates_only`, `never_touch` (checked, no evidence)
  or `not_checked` (a design of the pair was not read), with a count per
  status, so an empty seam never reads as agreement. The composed reply's note now says that a design with no
  link to the others reads as a cut-off cluster, a finding the composition
  made and the design alone does not have.

- **The hub skill tells an owner to connect each pin to what uses it**
  (`depends_on` from the hub's component or capability to the pin): a pin
  wired to nothing threads only itself, so the composed check reads that
  member as a cut-off cluster, and that finding is true.

### Fixed

- **A composed reply holds its budget.** Over a hub of fifteen designs the
  reply was 58,505 characters at the default budget of 30,000, because every
  list was at its floor of one whole finding and some findings name a whole
  cut-off cluster. When sampling lists is not enough, each finding's ids are
  cut to a few (it keeps `affected_ids_total`) and then whole `by_design` /
  `by_pair` keys are withheld, each named in `withheld_to_fit`; every count
  stays in `counts`. `link_status` is sampled like any list, its counts kept.
  The one-other-design call does the same when it is given `budget_chars`.

### Changed

- **The one-other-design `compose_and_analyse` reply keeps every key it had
  and gains the link fields** (`declared_links`, `computed_links`,
  `candidates`, `link_status`, `set_aside`, `links_note`); its findings now
  include what the checks see across a computed link. `pair_designs` gains
  `declared_links`, `computed_links`, `link_status`, `set_aside` and
  `links_note`; its name pairs are unchanged and are not evidence that the
  designs touch.
