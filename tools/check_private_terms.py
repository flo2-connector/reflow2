#!/usr/bin/env python3
"""Refuse a change that names a private work program — without naming it.

WHY IT EXISTS. reflow2's repository is public. On 2026-10-30 its owner decided
that it must not name their private work program, the organisation it belongs
to, or the projects in its hub, nor use field words that point at that work
(req:reflow2s-public-repository-names-no-private-work-program-and-ci-refuses-one).
The files were reworded to neutral examples, and this check keeps them so: it
fails on any tracked file, pull request title or body, or commit message that
matches the names.

🛑 THE CHECK NAMES NOTHING. The names live in a repository SECRET, not in this
file, and a CI log on a public repository is public. So a finding is printed as
WHERE only — `path:line`, `PR title`, `PR body`, or a commit sha — and never the
matched text, never the line, and never the pattern. A path that itself
matches is named by its place in `git ls-files` instead. A reader who has the
pattern can find the word; a reader who does not learns nothing.

WHERE THE PATTERN COMES FROM: the environment variable PRIVATE_TERMS, else the
file named by PRIVATE_TERMS_FILE. It is a regular expression, matched without
regard to case (Python's `re`, which reads `\\b`, alternation and character
classes the way PCRE does). A file may hold one per line; blank lines and lines
starting with `#` are skipped, and the rest are joined as alternatives.

WITH NO PATTERN IT PASSES, SAYING SO. A pull request from a fork runs without
the repository's secrets, so the pattern is not there to check against; failing
every outside contribution for that would turn the check off in practice. It
prints a notice and exits 0. The push to main that follows runs with the secret.

Usage:
    PRIVATE_TERMS_FILE=~/path/terms.regex python3 tools/check_private_terms.py
    python3 tools/check_private_terms.py --pr-title-env PR_TITLE --pr-body-env PR_BODY \\
        --commits-env COMMIT_RANGE              # what CI runs (.github/workflows/ci.yml)
    python3 tools/check_private_terms.py --commits origin/main..HEAD --text-file pr.md

Exits 0 when nothing matches (or there is no pattern), 1 on any match, 2 when
the pattern does not compile or an argument is wrong. Standard library only.
"""

from __future__ import annotations

import argparse
import os
import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
ZERO_SHA = re.compile(r"^0+$")


def pattern_text() -> str | None:
    """The pattern, from PRIVATE_TERMS or PRIVATE_TERMS_FILE; None if neither holds one."""
    raw = os.environ.get("PRIVATE_TERMS", "")
    if not raw.strip():
        path = os.environ.get("PRIVATE_TERMS_FILE", "").strip()
        if path:
            try:
                raw = pathlib.Path(path).expanduser().read_text(encoding="utf-8")
            except OSError:
                # The path is the operator's; saying it cannot be read names
                # nothing private, but the path itself is not echoed either.
                print("check_private_terms: PRIVATE_TERMS_FILE is set and cannot be read", file=sys.stderr)
                raise SystemExit(2)
    parts = [ln.strip() for ln in raw.splitlines() if ln.strip() and not ln.strip().startswith("#")]
    return "|".join(f"(?:{p})" for p in parts) if parts else None


def compile_pattern(text: str) -> re.Pattern:
    try:
        return re.compile(text, re.IGNORECASE)
    except re.error as e:
        # The position is safe to print; the message can quote the pattern.
        print(f"check_private_terms: the pattern does not compile (at position {e.pos})", file=sys.stderr)
        raise SystemExit(2)


def tracked_files(root: pathlib.Path) -> list[str]:
    out = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"], capture_output=True, check=True
    ).stdout
    return [p for p in out.decode("utf-8", "surrogateescape").split("\0") if p]


def matching_lines(text: str, rx: re.Pattern) -> list[int]:
    """1-based line numbers holding a match; the match itself is never kept."""
    return [i for i, line in enumerate(text.splitlines(), 1) if rx.search(line)]


def check_files(root: pathlib.Path, rx: re.Pattern) -> list[str]:
    found: list[str] = []
    for index, rel in enumerate(tracked_files(root), 1):
        path = root / rel
        # The PATH is checked too: a file can be named after what it is about.
        # Such a path is itself the thing not to print, so the file is named by
        # its place in `git ls-files` instead: `git ls-files | sed -n <N>p`.
        where = rel
        if rx.search(rel):
            where = f"tracked file #{index} of `git ls-files`"
            found.append(f"{where} (its path)")
        try:
            data = path.read_bytes()
        except OSError:
            continue  # a tracked file deleted in the working tree, or a submodule
        if b"\0" in data[:8192]:
            continue  # binary: a chance byte sequence is not a name
        text = data.decode("utf-8", "replace")
        found.extend(f"{where}:{n}" for n in matching_lines(text, rx))
    return found


def check_text(label: str, text: str, rx: re.Pattern) -> list[str]:
    lines = matching_lines(text, rx)
    return [f"{label} (line {n})" for n in lines]


def commit_messages(root: pathlib.Path, rng: str) -> tuple[list[tuple[str, str]], str | None]:
    """[(sha, message)] for a range `A..B`, and a notice when it had to narrow it."""
    notice = None
    if ".." in rng:
        base, head = rng.split("..", 1)
        if not base or ZERO_SHA.match(base):
            rng, notice = head, "the range has no base (a new branch); checking its last commit only"
    args = ["git", "-C", str(root), "log", "--format=%H%x00%B%x1e"]
    args += [rng] if ".." in rng else ["-1", rng]
    run = subprocess.run(args, capture_output=True)
    if run.returncode != 0 and ".." in rng:
        head = rng.split("..", 1)[1]
        notice = "the range's base is not in this clone; checking its last commit only"
        run = subprocess.run(["git", "-C", str(root), "log", "--format=%H%x00%B%x1e", "-1", head], capture_output=True)
    if run.returncode != 0:
        print("check_private_terms: the commit range cannot be read", file=sys.stderr)
        raise SystemExit(2)
    out = []
    for rec in run.stdout.decode("utf-8", "replace").split("\x1e"):
        rec = rec.strip("\n")
        if "\0" in rec:
            sha, msg = rec.split("\0", 1)
            out.append((sha.strip(), msg))
    return out, notice


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="Refuse a change that names a private work program.")
    ap.add_argument("--root", default=str(REPO), help="the repository to check (default: this one)")
    ap.add_argument("--no-files", action="store_true", help="skip the tracked files")
    ap.add_argument("--pr-title-env", help="an environment variable holding the PR title")
    ap.add_argument("--pr-body-env", help="an environment variable holding the PR body")
    ap.add_argument("--text-file", action="append", default=[], help="a file of text to check (a drafted PR body)")
    ap.add_argument("--commits", help="a commit range A..B (or one commit) whose messages to check")
    ap.add_argument("--commits-env", help="an environment variable holding the commit range")
    a = ap.parse_args(argv)

    text = pattern_text()
    if text is None:
        print(
            "check_private_terms: NOTICE — no pattern (PRIVATE_TERMS and PRIVATE_TERMS_FILE are "
            "unset or empty), so nothing was checked. A pull request from a fork runs without the "
            "repository's secrets; the push to main runs with them."
        )
        return 0
    rx = compile_pattern(text)
    root = pathlib.Path(a.root)

    found: list[str] = []
    checked: list[str] = []
    if not a.no_files:
        found += check_files(root, rx)
        checked.append("tracked files")
    for env, label in ((a.pr_title_env, "PR title"), (a.pr_body_env, "PR body")):
        if env:
            found += check_text(label, os.environ.get(env, ""), rx)
            checked.append(label)
    for f in a.text_file:
        found += check_text(f"text file {pathlib.Path(f).name}", pathlib.Path(f).read_text(encoding="utf-8"), rx)
        checked.append(f"text file {pathlib.Path(f).name}")
    rng = a.commits or (os.environ.get(a.commits_env, "").strip() if a.commits_env else "")
    if rng:
        commits, notice = commit_messages(root, rng)
        if notice:
            print(f"check_private_terms: notice — {notice}")
        for sha, msg in commits:
            if rx.search(msg):
                found.append(f"commit {sha} (its message)")
        checked.append(f"{len(commits)} commit message(s)")

    if found:
        print(f"check_private_terms: FAIL — {len(found)} place(s) name a private work program:")
        for where in found:
            print(f"  {where}")
        print(
            "Reword each to a neutral example (a work hub, a lab instrument, a deploy repository). "
            "The matched words are not printed, because this log is public."
        )
        return 1
    print(f"check_private_terms: OK — nothing matched ({', '.join(checked) or 'nothing asked'}).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
