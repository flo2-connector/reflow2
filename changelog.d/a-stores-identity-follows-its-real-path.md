### Fixed

- **A store opened through a symlink, or as `--graph-path .`, now opens by its real path too.** Up to
  v0.79.0, reflow2 placed a store's identity file (`graph.id.json`) beside the path as you typed it. So:
  - a store first opened through a symlink kept the file beside the LINK;
  - a store opened as `--graph-path .` from inside itself kept it INSIDE the store, as `..id.json`;
  - in both cases the next open by the store's real path was refused, as having lost its identity file.

  New identity files are now written beside the store's real directory, whatever the spelling. On open,
  reflow2 looks there first, then where an older version may have put the file (beside the link; inside the
  store). An identity found only in an old place is used, and a copy is written beside the store; reflow2
  says so once, on stderr and in `loop_status`. The old file is never moved or deleted, so an older reflow2
  opening the same way still finds it. Every store that opened before still opens. Opening by the plain
  path, the way an MCP configuration does, is unchanged. Tests pin this with stores the released v0.76.0
  binary made through a symlink and as `.`.

  In one case the result changes, and reflow2 says so: when the file beside the store and an older file name
  DIFFERENT designs, the one beside the store is used and the other is reported in `loop_status`'s `next`.

- **A refused open no longer changes anything.** An open that ends in a refusal over the identity file now
  leaves the store and every file beside it byte for byte as it found them. Up to v0.79.0 it still wrote
  `graph.meta.json`, and it rotated the store's own `LOG`, write-ahead log, `MANIFEST` and `OPTIONS` files.
  So a fresh `graph.meta.json` next to a missing `graph.id.json` said nothing about when the file was lost.

- **The refusal says where it looked, which design the store holds, and how to recover.** It lists every
  place it looked for the identity file. It reads the design id out of the store's own keys, without writing,
  and prints the identity file that would open it. It also says to look beside any symlink an older reflow2
  may have used.

  **What to do:** if a store refuses to open because it "has lost its identity file", put the design's id
  file beside the store, at `<store>.id.json` (for example `.reflow2/graph.id.json`). If the store was ever
  opened through a symlink, the file is beside that link as `<link-name>.id.json`: copy it there. If you have
  no copy, the refusal prints one with the id the store holds; write it there. The 2026-10-05 triage measured
  that putting the right id file beside the store recovers it with its data.
