### Added

- **`--args @path` reads a call's arguments from a file.** Beside `--args -`
  (stdin), `--call TOOL --args @args.json`, `reflow2 write TOOL --args @args.json`
  and `reflow2 read TOOL @args.json` read the one JSON object from the named file,
  so design prose reaches the call with no shell quoting and never enters the
  command text a terminal's guard hook reads. A missing file, a file that is not
  JSON, or one holding anything but one object is refused in plain words, naming
  the file, before anything is opened. **What to do:** MCP users, nothing. Door
  users, put long arguments in a file and pass `@file`.

### Changed

- **Every constructor that revises on a repeated id says so in its first
  sentence.** The 22 that do (the `add_*` constructors, `plan_epoch` and
  `record_finding`) now begin "Create or revise …" — the line `find_tools`
  shows. A refused tool name that guesses at an update
  (`update_node`, `update`, `set_node`) is answered with that route and the
  nearest served names, through a session (the same `invalid_params` code, still
  starting "tool not found") and through `--call`. No setter tool was added.
- **A node type's constructor is found by the type's name.** A `find_tools` query
  that names a node type as the schema spells it (`TemporalFact`, `Decision`)
  ranks the tool that creates it first, and that item says `creates`. A guessed
  tool name that names a type (`add_temporal_fact`, `record_fact`) is answered
  with the tool that creates it (`record_finding`).
- **Value sets the argument check could not see are published as enums:**
  `add_requirement.provenance`, and the `disposition` of `set_artifact_checksum`
  and of each `set_artifact_checksums` item. A value outside the set was already
  refused, one call later, by the handler; it is now listed beside the call's
  other problems, so "every one is listed" is true. The toolsnap guard now
  recognises a value set however it is written (commas, quotes, "or"), not only
  `a` / `b` / `c`.
- **`add_change_event`'s `description` is no longer advertised.** It was a decoy
  that existed only to redirect the commonest mistake, and `find_tools` listed it
  as a real field. It is still ACCEPTED and answered with the same redirect to
  `summary` / `rationale` — now beside the call's other problems.
- **A revision receipt counts the edges the call drew.** `revision.changed` keeps
  its meaning (the node's own properties moved); the new `revision.edges_changed`
  counts the edges this call drew, and an edge-only revise no longer says
  "nothing moved" beside its `edges_drawn`.
- **A standing pin that the MCP surface only grows**
  (`tests/the_mcp_surface_only_grows.rs`): the build is compared with the last
  release's tools, argument schemas and the reply shapes of a fixed scenario, and
  anything taken away fails it. Re-bless at a cut from the release binary.

**What to do:** MCP users, nothing — every change is additive: descriptions,
new optional reply fields (`revision.edges_changed`, a `find_tools` item's
`creates`), and earlier refusals of
values that were already refused. Door users, nothing required; `@file` is
available.
