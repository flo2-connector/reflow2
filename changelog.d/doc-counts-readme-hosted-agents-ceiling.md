### Fixed

- **The README, AGENTS.md and the core crate's header state the right counts again** (200 tools,
  36 served skills, 99 core modules, 71 edge types). They said 165, 25, 59 and 60 while the doc
  check stayed green, because it checked only the sentences written into its own list.
- **The README no longer says there is no service at all.** It says nothing leaves your machine
  with the default install, and adds a short paragraph on serving designs over the network
  (`--http`, the Docker image, flo2.io).

### Changed

- **The doc check now finds counts itself.** Any count of tools, served skills, node types, edge
  types or modules in a current-state doc (README, AGENTS.md, SKILLS.md, the consumer kit's
  AGENTS.md, docs/overview.md, docs/interaction-surfaces.md, the crates' lib.rs headers) must
  match the build, or CI fails naming the file, what it says and what the build says. Prose about
  the past is listed by its exact words in the check. **What to do:** when you add a tool or a
  skill, update the counts the failure names.
- **AGENTS.md has a size ceiling.** CI fails a change that makes it larger than the byte count in
  `tools/agents_md_ceiling.txt`. **What to do:** put new standing guidance in the design or at the
  step it bears on; lower the ceiling when the file shrinks; raise it only as its own reviewed
  change that says why.
