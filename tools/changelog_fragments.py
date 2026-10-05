#!/usr/bin/env python3
"""CHANGELOG fragments: one file per pull request, assembled at the release cut.

Why: every PR used to hand-edit CHANGELOG.md's `[Unreleased]` section, at the
same spot, so parallel PRs collided there (on 2026-09-30, #641 edited it while
three other branches waited to merge main). Anthony settled it on 2026-10-03:
one fragment file per PR, `changelog.d/<slug>.md`, assembled into CHANGELOG.md
at the cut (dec:idea-how-changelog-md-gets-written-once-the-design-is-the-record,
option b). Two PRs never touch one file, and the author writes the entry —
including any step a consumer must take — while they still know it.

A fragment is Markdown in Keep a Changelog sections:

    ### Fixed

    - **What a person notices, in bold.** Why, and what to do about it.

Each `### <Section>` is one of Added, Changed, Deprecated, Removed, Fixed,
Security. A fragment may carry several. Name the file after the change, not the
PR number (the number does not exist until the PR is opened).

    python3 tools/changelog_fragments.py --check            # fragments are well formed; [Unreleased] is empty
    python3 tools/changelog_fragments.py --preview          # what the cut would write
    python3 tools/changelog_fragments.py --cut 0.79.0 [--date 2026-10-04]

`--cut` writes the assembled entries under a new `## [<version>] — <date>`
heading below an empty `[Unreleased]`, and DELETES the fragments it used (git
history keeps them), so the next release starts from an empty changelog.d/. The
cut's author then writes the release's opening paragraph above the entries, as
before. Exit codes: 0 ok · 1 a check failed · 2 could not run.

Standard library only.
"""

from __future__ import annotations

import argparse
import datetime
import os
import re
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SECTIONS = ("Added", "Changed", "Deprecated", "Removed", "Fixed", "Security")
README = "README.md"
_HEADING = re.compile(r"^###\s+(.+?)\s*$")
_UNRELEASED = re.compile(r"^## \[Unreleased\]\s*$", re.M)
_NEXT_RELEASE = re.compile(r"^## \[", re.M)


def fragment_paths(dirpath: str) -> list[str]:
    """Every fragment in `dirpath`, sorted by name; the README is not one."""
    if not os.path.isdir(dirpath):
        return []
    return sorted(
        os.path.join(dirpath, f) for f in os.listdir(dirpath)
        if f.endswith(".md") and f != README and not f.startswith(".")
    )


def parse_fragment(path: str) -> tuple[dict[str, str], list[str]]:
    """`({section: body}, problems)` for one fragment."""
    problems: list[str] = []
    with open(path, encoding="utf-8") as fh:
        text = fh.read()
    sections: dict[str, list[str]] = {}
    current = None
    for n, line in enumerate(text.splitlines(), 1):
        m = _HEADING.match(line)
        if m:
            name = m.group(1)
            if name not in SECTIONS:
                problems.append(f"{os.path.basename(path)}:{n}: '### {name}' is not one of "
                                f"{', '.join(SECTIONS)}")
                current = None
                continue
            current = name
            sections.setdefault(current, [])
            continue
        if line.startswith("## ") or line.startswith("# "):
            problems.append(f"{os.path.basename(path)}:{n}: a fragment holds entries, not "
                            f"release or file headings ('{line.strip()}')")
            continue
        if current is None:
            if line.strip():
                problems.append(f"{os.path.basename(path)}:{n}: text before the first "
                                f"'### <Section>' heading belongs under one")
            continue
        sections[current].append(line)
    out = {}
    for name, lines in sections.items():
        body = "\n".join(lines).strip("\n")
        if not body.strip():
            problems.append(f"{os.path.basename(path)}: '### {name}' has no entry under it")
            continue
        out[name] = body
    if not out and not problems:
        problems.append(f"{os.path.basename(path)}: holds no entry")
    return out, problems


def assemble(dirpath: str) -> tuple[str, list[str], list[str]]:
    """`(markdown, fragments_used, problems)` — every fragment's entries grouped
    by section in Keep a Changelog order, fragments in file-name order within
    a section."""
    by_section: dict[str, list[str]] = {s: [] for s in SECTIONS}
    used, problems = [], []
    for path in fragment_paths(dirpath):
        sections, issues = parse_fragment(path)
        problems.extend(issues)
        if sections:
            used.append(path)
        for name, body in sections.items():
            by_section[name].append(body)
    parts = []
    for name in SECTIONS:
        if by_section[name]:
            parts.append(f"### {name}\n\n" + "\n\n".join(by_section[name]))
    return "\n\n".join(parts), used, problems


def unreleased_body(changelog: str) -> str | None:
    """What sits under `## [Unreleased]`, or None when there is no such heading."""
    m = _UNRELEASED.search(changelog)
    if not m:
        return None
    rest = changelog[m.end():]
    nxt = _NEXT_RELEASE.search(rest)
    return (rest[:nxt.start()] if nxt else rest).strip()


def cut(changelog: str, version: str, date: str, entries: str) -> str:
    """CHANGELOG.md with a new release section below an empty `[Unreleased]`."""
    m = _UNRELEASED.search(changelog)
    if not m:
        raise ValueError("CHANGELOG.md has no '## [Unreleased]' heading to cut below")
    rest = changelog[m.end():]
    nxt = _NEXT_RELEASE.search(rest)
    tail = rest[nxt.start():] if nxt else ""
    section = f"## [{version}] — {date}\n\n{entries}\n\n" if entries else f"## [{version}] — {date}\n\n"
    return changelog[:m.start()] + "## [Unreleased]\n\n" + section + tail


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    mode = ap.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true",
                      help="every fragment is well formed and CHANGELOG.md's [Unreleased] is empty")
    mode.add_argument("--preview", action="store_true", help="print what the cut would write")
    mode.add_argument("--cut", metavar="VERSION", help="assemble the fragments under VERSION")
    ap.add_argument("--date", default=None, help="the release date (default: today)")
    ap.add_argument("--root", default=REPO, help="the repository root")
    opts = ap.parse_args(argv)

    dirpath = os.path.join(opts.root, "changelog.d")
    changelog_path = os.path.join(opts.root, "CHANGELOG.md")
    try:
        with open(changelog_path, encoding="utf-8") as fh:
            changelog = fh.read()
    except OSError as e:
        print(f"changelog_fragments: cannot read {changelog_path}: {e}", file=sys.stderr)
        return 2
    entries, used, problems = assemble(dirpath)

    if opts.check:
        body = unreleased_body(changelog)
        if body is None:
            problems.append("CHANGELOG.md has no '## [Unreleased]' heading")
        elif body:
            problems.append(
                "CHANGELOG.md's [Unreleased] section has entries written into it directly. Since "
                "2026-10-03 each pull request writes its entry as a fragment in changelog.d/ "
                "(see changelog.d/README.md) and the cut assembles them; move these into a "
                "fragment, so two PRs never edit one spot."
            )
        for p in problems:
            print(f"  FAIL  {p}")
        print(f"\nchangelog fragments: {len(used)} fragment(s), "
              + ("OK" if not problems else f"FAILED — {len(problems)} problem(s)"))
        return 1 if problems else 0

    if problems:
        for p in problems:
            print(f"  FAIL  {p}", file=sys.stderr)
        print("changelog_fragments: fix the fragments first; nothing was written", file=sys.stderr)
        return 1

    if opts.preview:
        print(entries or "(no fragments — the release would carry no entries)")
        return 0

    date = opts.date or datetime.date.today().isoformat()
    try:
        updated = cut(changelog, opts.cut, date, entries)
    except ValueError as e:
        print(f"changelog_fragments: {e}", file=sys.stderr)
        return 2
    with open(changelog_path, "w", encoding="utf-8") as fh:
        fh.write(updated)
    for path in used:
        os.remove(path)
    print(f"changelog_fragments: wrote [{opts.cut}] — {date} from {len(used)} fragment(s) and "
          f"removed them: {', '.join(os.path.basename(p) for p in used) or 'none'}. Write the "
          f"release's opening paragraph above its entries, then commit (git add -A changelog.d "
          f"CHANGELOG.md).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
