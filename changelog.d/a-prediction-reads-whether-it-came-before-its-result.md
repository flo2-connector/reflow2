### Added

- **A prediction reads whether it was stated before the result that tests it.**
  `reasoning_report` carries `prediction_order`: for each result (a statement of
  kind `evidence`) that is a premise of a step beside a prediction, or that
  CONTRADICTS a claim, how the prediction stands against it —
  `stated_before_the_result`, `stated_after_the_result` (post hoc),
  `revised_after_the_result` (with `earlier_wording_in`, the Snapshot that keeps
  the wording it had when the result was recorded), `changed_after_the_result`,
  `order_not_recorded` or `imported_not_recorded_here` — with the times and a
  sentence that says it. It is read only from times the server recorded, never
  from `reported_at`, `acted_at` or any date a caller typed. It is said, never
  enforced: a late prediction is not refused, and no support, contest or
  refutation reading moves because of it. Absent when no result tests anything,
  so a design with no results reads as before. **What to do:** nothing; a prediction
  or result recorded before this release has no stamp and reads
  `order_not_recorded`, and every one recorded after it is stamped.
- **reflow2 stamps the server time on each node's creation and content change.**
  Every node write records, from the server's own clock, when the node was
  created and when its content last changed, sealed (HMAC-SHA256) under a key
  kept inside the store. A write that names a stamp (`server_created_at`,
  `server_changed_at`, `server_seal`) is REFUSED, on every write path. Reads
  leave the stamps out, so every reply, receipt, comparison and content hash
  reads as before. A stamp reads as recorded here only in the store that sealed
  it, against the content it was sealed over; one carried in from elsewhere, or
  left on content edited outside reflow2, reads "imported, not recorded here".
  The store's format does not change, and an older reflow2 still opens it.
  **What to do:** nothing. A backup or move by store directory keeps the stamps
  and the key.
- **`export_graph` takes `stamps: true`** to carry each node's stamps (in its
  properties, which every older reader accepts), for a backup or a move by
  export that should keep them. Off by default, so two exports of one design
  stay byte-identical and two branches making the same change to one item still
  merge; refused with an item-layout path, which never carries them.
