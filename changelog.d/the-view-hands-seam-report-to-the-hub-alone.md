### Fixed

- **A hub's view no longer fans out `seam_report`, and now fans out `seam_coverage`.** 0.83.0's
  list of tools a view serves on the hub alone named `seam_coverage`, which reads only its own
  design's seams, and left out `seam_report`, which names its other design. So at a hub, a
  `seam_report` call ran once per member against that same other design, and `seam_coverage`
  answered for the hub alone. Both were reads, and the readable set held throughout. The two are
  swapped, and a test now reads every served schema so a read that names another design cannot
  be left off the list again. **What to do:** nothing.
