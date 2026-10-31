### Removed

- **The copy of reflow2's own design is no longer committed to the repository.** reflow2's
  design has been hosted on flo2.io since 2026-10-10, where it is private; the copy that CI
  read at `docs/design/reflow2/`, the tool that refreshed it (`tools/reflow2_design_copy.py`,
  with its one-design-PR-at-a-time hold) and the home page's counts of reflow2's design
  (`tools/site_counts.py`) are gone. `/docs/design/` is now git-ignored, so a copy exported
  there for local work cannot be committed by accident. Git history is unchanged. **What to
  do:** nothing, unless you worked on reflow2's own design from the copy: write through
  flo2.io, and claim `proj:reflow2` before a design PR as before (AGENTS.md says how).
- **The CI gates that read that copy are retired until a nightly check replaces them.** The
  design coherence gate (`reflow2_check.py --export`), `self_host_uses_documents`,
  `check_intent_authority`, `check_consumer_reach`, `check_verification_ratchet` and
  `the_served_surface_names_no_real_person` no longer run in CI on reflow2's own design; each
  tool stays, and each hermetic suite still runs. The design is not checked against the code
  until a scheduled check on flo2.io's box runs reflow2's own client against the hosted design
  (accepted, not built yet). Projects that run these tools on their own designs are not
  affected.

### Changed

- **Two surface gates and two core tests read a generated design instead of reflow2's own.**
  `replies_are_bounded` and `a_reply_is_sent_once` need a design big enough to make replies
  large, and the item-layout round trip and the hydrate measurement need one of realistic
  size. `tools/generated_design.py` (and its Rust twin in the core tests) makes one from the
  schema: reflow2's own size and mix, every node type and edge type the schema declares, every
  value shape, and odd ids on purpose. `vocabulary_reach.py --check` now reads reachability
  from the surface alone, which guards 17 more properties than before; `--export` adds the
  adoption columns.
- **Examples and test fixtures use neutral names** (a work hub, a lab instrument, a deploy
  repository, a storage service) in the skills, tool help, source comments and docs.

### Fixed

- **A number saved in a design reads back as the same number.** serde_json parsed some
  shortest-form floats one unit in the last place off (`0.11458333333333333`, which is 11/96,
  came back `0.11458333333333331`), so an untouched design could read as tampered in the item
  layout, or fail its own content hash as a single file. Its exact parser
  (`float_roundtrip`) is now on. Found by the generated design; reflow2's own design never
  held one of the values that change.

### Security

- **CI refuses a change that names a private work program.** `tools/check_private_terms.py`
  checks every tracked file, the pull request's title and body, and its commit messages
  against a pattern held in the repository secret `PRIVATE_TERMS`, and prints only where it
  matched (`path:line`, `PR title`, `PR body`, a commit sha), never what. A pull request from a
  fork has no secrets and passes with a notice; the push to main checks it.
