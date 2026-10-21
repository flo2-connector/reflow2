### Added

- **`import_graph` takes `merge: true`, which keeps what a partial node leaves
  out.** Without it an import still states each node whole, so an export→import
  round trip reproduces a design and a document can drop a property. With it, a
  node the design already holds takes the document's properties and keeps every
  other one, as `create_nodes` does. **What to do:** to edit existing records in
  bulk, pass `merge: true`, or use `create_nodes`.
- **A search on a design that pins others names every pinned design it did not
  look in.** `search_design` adds `not_searched` (each pinned design, where it
  could be read, and why it was not searched) and `not_searched_note`, so
  "nothing found" in a hub never reads as nothing found anywhere. A hub's view
  names only the pinned designs outside it. A design that pins nothing answers
  exactly as before.
- **`upstream_status` shows each member's reviewed revision beside the state of
  its record.** Each entry under `members` carries `reviewed_revision` (the
  pin's `version`) and `record_state` (`unchanged`, `moved`, `missing`,
  `not_watched` …). reflow2 still fetches nothing: to catch up, the hub's own
  tooling re-declares the pin with the new `version` (the hub skill says how).

### Fixed

- **A partial `import_graph` no longer removes stored properties in silence.**
  The reply now counts, by `Type.property`, every property it removed
  (`properties_removed`) or changed (`properties_changed`) on a node the design
  already held, names a few in `removed_from`, and says what to do in
  `removed_note`. A hub that imported 454 Artifacts as `{name, location}` lost
  their `checksum`, `checksum_basis` and `status`, and the reply said only
  `nodes_written: 454`. A value the schema's default replaced, such as an
  `artifact_type` reset from `config` to `code` or an accepted Requirement's
  `status` reset to `proposed`, is now counted as changed, never under
  `materialized` as a default filled in. The served description no longer
  promises that a partial node is refused; only one missing a required property
  is. The adopt skill sends bulk edits of existing records to `create_nodes`.
- **A link into a design pinned by its committed export says when its far node
  is gone.** `upstream_status` (and `loop_status`) report `link_far_end_stale`
  when the export no longer holds the node and `link_far_end_unreachable` when
  the export cannot be read, where before a removed node read only as
  `link_far_end_moved`. `review_relations`, and `related_to` on a decision,
  refuse a link to a node the pinned export does not hold, and write nothing.
