### Fixed

- **`reflow2 update` no longer writes the whole kit back into a project that kept only part
  of it.** A clone that kept only `REFLOW2.md` beside its team's own `AGENTS.md` had
  `update --check` plan about 80 new files and a line appended to that team's `AGENTS.md`.
  The guard added in 0.89.0 only refused when every recorded kit file was gone. Now, when
  every file of one part of the kit (all the skills, all the prompts, a command tree) is gone,
  `update` and `update --check` refuse and change nothing. The refusal names the parts that
  are gone and the files still here. Deleting ONE kit file still means "give me reflow2's
  copy". **What to do:** nothing in a project that carries the whole kit. In a project that
  keeps part of it on purpose, leave it: `update` now leaves it alone. `reflow2 init .` brings
  the whole kit back, and `reflow2 init --remove` takes out the rest. Update cannot yet refresh
  only the files a project keeps.
