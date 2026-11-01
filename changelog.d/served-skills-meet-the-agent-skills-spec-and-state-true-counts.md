### Fixed

- **Every served skill's frontmatter now conforms to the Agent Skills specification.** Each skill
  carried `metadata: {composes: [..]}` (and two `demanded_by: [..]`), a list where the
  specification allows only string values, so a strict host could skip all 35 skills. Both are
  now one string of words (`composes: "STANDING WRITES"`). Five descriptions held a `: ` that
  strict YAML reads as a second key (check-health, link-ideas, onboarding, plan-increments,
  where-am-i); they are reworded. Nothing to do: the skills are served, so the next release
  carries them.
- **The counts the skills state are true again.** help said eight skills answer to another word
  (ten do: `/taxonomy` and `/triage` were missing) and spoke of a list of twenty-eight commands
  (forty ship); ci-gate said the build fails on any of four causes while `reflow2_check.py` fails
  on ten, and now names all ten (PHANTOM, INTEGRITY, STORED TWINS, DEPEND and GAPS were
  undocumented); governance-proposal said two findings over a list of three.

### Added

- **`skill_lint` checks every served skill against the specification and against what is
  served.** It fails the build on a frontmatter field the specification does not define, a
  `name` or `description` past its limits (64 and 1,024 characters), a `metadata` value that is
  not a string, or a value strict YAML refuses. It also fails when a count that introduces a list
  is not the list's length, or when a skill's count of something served (renamed skills, slash
  commands, the gate's failure kinds) disagrees with the surface, computed on every run.
