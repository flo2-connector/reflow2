### Changed

- **A root cause in a family of designs looks in every member, not only where the symptom
  showed.** The `root-cause` skill now walks a design's member designs. It searches past causes in
  each, asks which members moved since this design last looked (`upstream_status`), compares the
  interfaces at the seam with the other side's current version, and walks impact backwards from a
  member that moved (`propagate_from` with `arriving_from`). It names any member it could not reach.
  It records the cause in the design where the cause lives, with a pointer from where the symptom
  showed. The `/hub` skill now says where to record is not where to look. **What to do:** nothing.
  The skills are served, so the next session uses them.
- **A new idea or requirement is asked whether it belongs under a whole already recorded.**
  `capture-intent`, `brainstorm` and `link-ideas` each ask whether the new piece is one facet of a
  larger aim the design holds, such as a vision or an umbrella requirement, and link it there, or
  say none fits.
