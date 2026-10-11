### Added

- **Every tool that reads another design takes it by graph_id on a server that holds many
  designs.** After `compare_designs` and `compose_and_analyse`, the rest of the family:
  `certify_preservation` takes `other_graph_id` (the live graph is the base, as in
  `compare_designs`), `merge_designs` takes `other_graph_id` for theirs and `base_graph_id` for the
  common ancestor (ours is the live graph), `analyze_alternatives` takes `other_graph_ids` (the live
  graph is the baseline), and `pair_designs` and `seam_report` take `other_graph_id` instead of the
  inline `design`. Each is read from the designs a registry server (`--registry-root`, flo2.io's
  shape) already holds, only inside the set the trusted host hands over in
  `_meta["reflow2/readable_graph_ids"]` on that call; a design outside it is refused naming the id
  and saying nothing about what it holds. The path and inline arguments stay, made optional where
  they were required, so an existing call is unchanged; a graph_id together with a path or an
  inline export is refused naming both
  (`req:every-cross-design-operation-takes-its-other-designs-by-graph-id-under-the-hosts-read-filter`).

### Fixed

- **A registry server no longer opens a design, an export or a store at a path a call hands it,
  through any tool.** Until now a `--registry-root` server opened the path it was given in
  `certify_preservation`, `merge_designs`, `analyze_alternatives`, `fork_point` (`export_path`) and
  `describe_designs` — which told any caller the graph_id and label of a design stored at a path on
  the host — and in three writes: `apply_merge` merged from the paths, `import_graph` imported a
  file from the host into the design, and `external_dependency` read a `design_export` there for
  its baseline. Each now refuses with "a path is not served here", naming what works instead. The
  writes do not take the other design by graph_id yet: send `import_graph` the export as
  `document`, and run `apply_merge` on a server you run yourself. Only the registry server changes;
  over stdio or `--shared` every path works exactly as before. A gateway in front of reflow2 can
  keep its own refusals of these arguments; they are no longer the only thing standing between a
  caller and the host's files for these tools.
