### Fixed

- **A skill served from flo2.io (or any host) no longer tells its reader the design is on their
  own machine.** Every skill was served unchanged wherever reflow2 ran, written for a local
  install: on flo2.io, `help` said "a program that runs on your own machine … Nothing leaves your
  machine" about a design kept on flo2.io, and 33 skills pointed a chat at an AGENTS.md it does
  not have. The lines that really differ are now written twice in the SKILL.md, a local version
  and a hosted one, and reflow2 serves the one for how it runs: a local engine the local side,
  an engine served for others (behind a trusted gateway, as an OAuth resource server, or exposed
  with no caller rule) the hosted side. help, parallel-work, report-friction and ci-gate carry
  both; the AGENTS.md pointer is reworded once in every skill ("The standing rule is in the
  server's instructions"). Nothing to do: the skills are served.
- **Every skill reaches the agent at connect time.** Claude Code keeps 4,096 characters of a
  server's initialize instructions. 0.95.0's ran 10,327 characters locally (9,961 where a
  session-end nudge is installed) and 10,656 behind a trusted gateway, 11,154 with flo2.io's
  preamble in front, so 27 of the 35 skills fell past the cut with nothing to say so. The
  handshake now states the standing rules first, shortened, and names every skill with a "use
  when" line of at most eight words (a new `use_when` metadata field);
  the full triggers stay in `list_skills` and `get_skill`, and the long form of the rules in
  `get_instructions`. Measured on the built binary, it now runs 3,018 characters locally (2,862
  with a nudge) and 3,276 behind a trusted gateway, 3,774 with flo2.io's preamble.

### Added

- **Two tests hold this.** One serves every skill both ways and fails on a claim only a local
  install can make in what a hosted engine serves; the other measures the handshake on every
  door, with room for a host's 496-character preamble, against the 4,096-character cap.
