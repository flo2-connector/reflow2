### Added

- **A saved design says where an Artifact's accepted checksum is.** Since 0.79.0
  an accepted checksum is written once, on the CHANGED edge from the change that
  accepted it, and left off the Artifact's own record while the two are equal;
  reflow2's import puts it back. Nothing in the file said so, and a person or a
  script that opened a committed export with plain JSON read the missing
  `checksum` as a dropped one: twice in one project, which then concluded its
  design gate could not see those files change. It could: one byte added to one
  of them failed the gate. Now a single-file export that leaves at least one
  checksum off a record carries a top-level `reading_notes.artifact_checksums`
  saying which edge holds it, which fields, which acceptance is current (the
  highest `accepted_seq`), and that the kit's `tools/design_io.py`
  `current_acceptances` applies the rule. The note is outside the content hash,
  so no design's hash changes and two exports of one design stay byte-identical;
  a design with no accepted checksums gets no note. An agent that calls
  `export_graph` gets the note in the document it is handed. **What to do:**
  nothing. A script that reads checksums from an export should read them with
  `current_acceptances`, or import the export and read the records. The item
  layout's `design.json` does not carry the note yet: it refuses fields it does
  not know, so adding one would break older binaries reading a newer layout.
