### Added

- **`compare_designs` and `compose_and_analyse` take the other design by graph_id on a server
  that holds many designs.** Pass `other_graph_id` instead of a path or an inline export: on a
  registry server (`--registry-root`, flo2.io's shape) the other design is read from the one the
  server already holds, `compare_designs` makes the live graph the base (labelled by this design's
  graph_id) and `compose_and_analyse` namespaces the other design by its graph_id unless you pass
  `namespace`. `base_path` and `design` are optional now, so an existing call is unchanged. The
  read is bounded by the host: the trusted gateway or bearer-token server in front hands reflow2
  the set of designs the caller may read in `_meta["reflow2/readable_graph_ids"]` on every call,
  and a graph_id outside that set — or a call carrying no set — is refused naming the id and
  saying nothing about whether such a design is here or what it holds. A registry server with no
  trusted host reaches no other design, and a single-design server says `other_graph_id` is served
  only by a registry server
  (`req:every-cross-design-operation-takes-its-other-designs-by-graph-id-under-the-hosts-read-filter`,
  `dec:a-cross-design-read-is-bounded-by-the-readable-set-the-host-hands-over-not-by-the-root-alone`).
  **What a host must do:** send `reflow2/readable_graph_ids` in `_meta` on reads as well as writes;
  a gateway that names the caller only on writes gets every cross-design read refused.

### Fixed

- **A registry server no longer reads a file path handed to `compare_designs`.** Until now a
  `--registry-root` server given `base_path` or `other_path` opened that path on its own host and
  answered the diff; a path is not a design it serves. It now refuses, naming the route that works
  (`other_graph_id`). Only the registry server changes; a server you run yourself over stdio or
  `--shared` compares files exactly as before.
