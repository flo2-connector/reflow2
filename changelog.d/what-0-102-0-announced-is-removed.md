### Removed

These were announced in 0.102.0, with what to do instead, and go in this release.

- **`record_the_build`.** It is no longer listed. For this one release, a call to the old name is
  refused with what to do instead. **What to do:** call `add_change_event` with `checksums`, which
  accepts the files' checksums against the change in the same call. Record the check with
  `add_verification`, and mark the work built with `set_capability_status`.
- **`search_design`'s `view`, `searched_by_design` and `found_by_design`**, the three per-design
  lists in a search over several designs (through a hub's address, or with `tracked_designs`). The
  reply no longer carries them, or a `deprecation_notice` about them. **What to do:** read
  `designs_searched` in place of `view`, `searched` (the one total) in place of
  `searched_by_design`, and `found_in` (the designs with hits, with their counts) in place of
  `found_by_design`.
