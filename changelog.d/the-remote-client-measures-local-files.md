### Added

- **Your own reflow2 client, attached to a design on a server, now measures the
  folder's files for you.** `link_artifact`, `set_artifact_checksum`,
  `set_artifact_checksums` and `reconcile_artifacts` called without a checksum
  used to be refused by a server that holds no tree (flo2.io, a registry), or
  recorded whatever hash the agent typed. The client started in a folder whose
  `.reflow2.toml` names the design — or a bare `--remote` client given
  `--tree-root` — now hashes the folder's own files on your machine with the
  server's own measuring code (sha256, never content; a URI unmeasured, a path
  outside the folder refused unread) and sends the result, so checksums no
  longer have to be typed by hand. The server records them as **asserted**,
  because it did not measure them, and the reply's `measurement` block says
  the value was measured on this machine by the client and under which root;
  the server's own block rides beside it under `server`. An accept whose file
  cannot be measured is refused on your machine with the server's own words
  and nothing is sent. A client with no root — no `.reflow2.toml` folder and no
  `--tree-root` — measures nothing, never guesses the current directory, and
  says so in the reply. Nothing changes on the wire for anyone not using the
  remote client; the server is untouched. `loop_status`'s `artifacts` block is
  not measured this way yet.
