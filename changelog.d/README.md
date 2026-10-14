# changelog.d — one fragment per pull request

Each pull request writes its CHANGELOG entry as **its own file here**, never into
`CHANGELOG.md`. The release cut assembles them. Two pull requests therefore never
edit the same spot, and neither waits for the other to merge
(`dec:idea-how-changelog-md-gets-written-once-the-design-is-the-record`, settled
2026-10-03).

## Writing one

Name the file after the change, in a few words: `changelog.d/item-layout.md`.
Put the entry under one or more [Keep a Changelog](https://keepachangelog.com)
sections — `### Added`, `### Changed`, `### Deprecated`, `### Removed`,
`### Fixed`, `### Security`:

```markdown
### Fixed

- **What a person notices, in bold.** Why it happened, and **what to do** about
  it — every step a consumer must take goes here, while you still know it.
```

Write it for a person, as the entries in `CHANGELOG.md` read.

### Deprecating something a client uses

Anything a client can call or read is removed only after a release that
announced it (`crates/reflow2-mcp/src/deprecation.rs`). Each entry in
`deprecation::REGISTRY` needs its own `### Deprecated` entry that names the item
— every name in backticks: the tool; the tool and the argument's path; the tool
and the reply field's path; the node type; the edge type; the node type and the
property — and carries a **What to do:** line:

```markdown
### Deprecated

- **The `Anchor` node type is going, in the release after this one.** Nothing
  creates it, and a Snapshot records the same baseline. **What to do:** record a
  Snapshot instead.
```

`tests/every_deprecation_is_announced_in_the_changelog.rs` fails without it,
and, once the cut has assembled it, reads it from that release's section of
`CHANGELOG.md`. The release that removes the item says so under `### Removed`.

Check a fragment with:

```bash
python3 tools/changelog_fragments.py --check     # CI runs this
python3 tools/changelog_fragments.py --preview   # what the cut would write
```

## At the cut

```bash
python3 tools/changelog_fragments.py --cut 0.79.0 --date 2026-10-04
```

writes the fragments' entries under `## [0.79.0] — 2026-10-04`, grouped by
section, below an empty `[Unreleased]`, and deletes the fragments it used. The
cut's author then writes the release's opening paragraph above them and commits
`CHANGELOG.md` with the deletions.
