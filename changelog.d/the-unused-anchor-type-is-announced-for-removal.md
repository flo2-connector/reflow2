### Deprecated

- **The `Anchor` node type is going: this release still serves it, and the
  next release removes it.** It was a checksummed baseline taken when leaving
  P2 for a build round, carried over from the original reflow. Nothing in
  reflow2 has ever created one or read one, and none of the seven designs
  measured on 2026-10-14 holds one. `describe_schema` now marks it
  `deprecated` and lists it under `deprecations`, and a call that writes or
  reads an Anchor gets a `deprecation_notice` in its reply. Nothing breaks in
  this release: an Anchor is still created, read and exported exactly as
  before (`dec:anchor-retire-or-implement`). **What to do:** record a
  checksummed baseline with `add_epoch` (`epoch_type` `baseline`, with its
  `checksum`), and capture a node's state at it with `snapshot_before_change`,
  which records a Snapshot.

### Fixed

- **Two schema hints no longer describe a link nothing makes.** `DriftEvent`'s
  hint said a drift event "References the Anchor it diverged from via
  DEPENDS_ON", and Anchor's said drift events reference it. No code drew that
  link. A drift event DEPENDS_ON the Artifact whose file drifted, or the
  Environment and Release of a deployment drift, and the hint now says so.
  **What to do:** nothing.
