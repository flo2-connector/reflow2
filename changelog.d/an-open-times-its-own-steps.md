### Added

- **Opening a design now logs how long each of its steps took.** Each open of a
  design on disk writes one `INFO` line, `opened a design`, with the design's
  id, `total_ms`, and every step in milliseconds:
  - schema;
  - identity;
  - the RocksDB store;
  - the provenance check;
  - the type index;
  - each repair that runs on every open, with how many records it changed;
  - the search index.

  A `--call` prints the line on stderr. A server writes it to its log, so a
  hosted reopen can be split without a debugger. Code that opens a design
  reads the same timing from `DesignGraph::opened_in()`. What an open does,
  and in what order, is unchanged. The `read` and `write` verbs stay quiet,
  because they log errors only. **Nothing to do.**
