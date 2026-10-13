### Fixed

- **Search finds words in a change's rationale, and in the other prose fields it
  used to skip.** `search_design` (and `topic_report`, which searches the same
  way) now reads every field a node's prose is written in. Fifteen fields were
  added:
  - a change's `rationale` and `stands_in_for`;
  - a requirement's and a constraint's `rationale`;
  - a dimension assessment's `description` and `evidence`;
  - a project's `objective`;
  - a decision's `alternatives`;
  - the `name` of a drift event and of an anchor;
  - an interface's `spec`, `operations`, `error_model`, `endpoint` and
    `payload_schema`.

  Before this, a fix whose reason quoted an error could not be found by that
  error, which is the first search the root-cause skill makes. A new test lists
  every text field as either searched or left out with its reason (a date, a
  reference, a hash, a location, a word from a small set, where a record came
  from, or a payload for a program), so a field added later cannot be skipped
  silently. **Nothing to do:** a design opened by this release rebuilds its
  search index once, by itself, because the index now records which fields it
  was built over. A server already rebuilt it at every start. The one-shot
  `--call` door used to leave older nodes unsearchable until they were next
  written.
