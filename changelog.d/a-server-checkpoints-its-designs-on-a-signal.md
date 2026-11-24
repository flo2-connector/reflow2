### Added

- **A registry server copies every design it serves on `SIGUSR1`, without
  stopping.** Start it with `--checkpoint-dir DIR` (or
  `REFLOW2_CHECKPOINT_DIR`). On the signal it takes a RocksDB checkpoint of
  each design under its root while it goes on answering calls:
  - a design it holds open is copied through the store it holds;
  - one it does not hold is copied from its store on disk, and a request that
    would open that design meanwhile waits for the copy instead of failing.

  The set lands in `DIR/<UTC stamp>.partial/<design folder>/graph`, each
  design's identity file beside its copy. `result.json` names every design,
  whether it was open, how long its copy took, and any design that could not
  be copied with why. The set is renamed to `DIR/<UTC stamp>` only when it is
  whole, so a folder without `.partial` is always complete. On the same
  filesystem as the designs a copy is hard links, and costs almost no disk
  until the live store compacts. The server never deletes a set.

  A copy does not include the design's kept search index, so a design restored
  from one rebuilds its index on its first open. A backup that used to stop the
  server for a consistent copy can send the signal and read the newest whole
  set instead. **To use it, add `--checkpoint-dir`. Nothing to do otherwise.**

### Changed

- **`SIGUSR1` no longer ends a registry server.** Without `--checkpoint-dir` it
  is logged, saying there is nowhere to put a copy, and the server goes on
  serving. A single-design server (`--graph-path`) does not answer the signal
  yet and keeps the old behaviour. **Nothing to do.**
