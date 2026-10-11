### Fixed

- **Revising a change's `affected` list no longer erases the checksums accepted on
  it.** Calling `add_change_event` again on an existing ChangeEvent with one more
  `affected` entry re-drew every CHANGED edge with its `action` alone, so an
  acceptance `set_artifact_checksum(s)` had written on an edge — `accepted_baseline`,
  `accepted_seq`, `checksum_after` — was gone, the reply said only
  `edges_already_present`, and the design gate failed on files accepted minutes
  earlier. An edge already drawn now keeps every property the call did not pass;
  only `action` moves when the call says a different one. **What to do:** nothing.
  If a gate ever told you to "accept it two-sided" for a file you had just
  accepted, this was why; re-accepting it was the right repair, and the order
  "name every artifact first, then accept" is no longer needed.
