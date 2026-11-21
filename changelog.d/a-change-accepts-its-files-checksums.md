### Changed

- **`add_change_event` takes `checksums`, and accepts those files' checksums against the change in
  the same call.** Each item is `{artifact_id, checksum?}`. The file is drawn as CHANGED, so it
  need not be listed in `affected` too. Its checksum is accepted as `design_updated`, and one
  left out is measured where the file can be read, including by a local client. Every file is
  checked before anything is written, and an acceptance that is refused takes the whole call
  back. Measured on flo2.io from 10-30 to 11-20: in 137 of 163 runs, recording a change was
  followed by accepting its checksums, a median of 16 seconds later. That second call is now
  part of the first. A change that touches registered files and does not accept them still says
  so under `next_step`, which now names `checksums` first.

### Deprecated

- **`record_the_build`** is going in the next release. Agents called it 8 times in three weeks
  while doing its steps by hand, so its job moves into the tool they already call. It still
  works in this release, and it is marked as deprecated in the tool list. **What to do:** call
  `add_change_event` with `checksums`, which accepts the files' checksums against the change in the
  same call. Record the check with `add_verification`, and mark the work built with
  `set_capability_status`.
