### Changed

- **The first `loop_status` after a write is about a third faster on a large design.** On
  reflow2's own design (8,916 nodes) it took 8.7 s and now takes 5.5 to 5.6 s, measured
  2026-11-13 on one machine, the engine before this change beside the engine after it. Two
  parts of the check read the same thing again for every item they looked at. Asking whether
  each check ever came back with a real result read every file record once per check to find
  the one at the check's location (all 833, for each of 200 checks, 1.8 s); the file records
  are now read once per state of the design. The question "was this defect fixed by a later
  change?" counted everything each nearby change had changed, for every defect, to rank the
  few changes it then offered (7,751 counts for 550 defects, 1.2 s); it now counts only for
  the changes it offers. The answers are the same. `detect_gaps` alone after a write went
  from 3.8 s to 2.3 s on that design. A small design sees no difference, and a repeat ask
  with no write between was already 0.1 s. **This is not "check only what changed"**, which
  is recorded and not built: the rest of the time is each rule reading the whole design once.
  **What to do:** nothing.
