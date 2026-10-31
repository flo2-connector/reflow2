#!/usr/bin/env python3
"""The private-terms guard's own net (hermetic: made-up terms, a scratch repository).

tools/check_private_terms.py keeps a public repository from naming a private
work program. What makes it safe to run in a public CI log is what it does NOT
print, so that is pinned first: a finding names WHERE (path:line, `PR title`,
`PR body`, a commit sha) and never the matched words or the pattern. Then that
it fails on each kind of place, passes a clean tree, reads the pattern from
either source, passes with a notice when there is none (a fork's pull request
has no secrets), and refuses a pattern that does not compile without quoting it.

The terms here are invented and match nothing real.
"""

from __future__ import annotations

import os
import pathlib
import subprocess
import sys
import tempfile

SCRIPT = pathlib.Path(__file__).resolve().parent / "check_private_terms.py"
TERMS = r"\bzorblax\b|quuxcorp|frobnitz[_-]?lab"
WORDS = ("zorblax", "Zorblax", "QUUXCORP", "frobnitz-lab", "quuxcorp", "frobnitz")

failures: list[str] = []


def check(label: str, ok: bool, detail: str = "") -> None:
    print(f"  {'PASS' if ok else 'FAIL'}  {label}" + ("" if ok else f"   {detail}"))
    if not ok:
        failures.append(label)


def git(repo: pathlib.Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, text=True, check=True
    ).stdout.strip()


def run(repo: pathlib.Path, *args: str, env: dict | None = None) -> subprocess.CompletedProcess:
    e = {k: v for k, v in os.environ.items() if k not in ("PRIVATE_TERMS", "PRIVATE_TERMS_FILE")}
    e.update(env or {})
    return subprocess.run(
        [sys.executable, str(SCRIPT), "--root", str(repo), *args],
        capture_output=True, text=True, env=e,
    )


def leaks(out: str) -> list[str]:
    """Any matched word or any piece of the pattern that reached the output."""
    low = out.lower()
    bad = [w for w in WORDS if w.lower() in low]
    if "zorb" in low or "quux" in low or "frob" in low or TERMS.lower() in low:
        bad.append("(a fragment of the pattern)")
    return bad


def scratch_repo() -> pathlib.Path:
    repo = pathlib.Path(tempfile.mkdtemp(prefix="private-terms-"))
    git(repo, "init", "-q", "-b", "main")
    git(repo, "config", "user.email", "t@example.org")
    git(repo, "config", "user.name", "test")
    git(repo, "config", "commit.gpgsign", "false")
    (repo / "clean.md").write_text("A design hub with twenty members.\nNothing to see.\n")
    (repo / "notes.md").write_text("line one\nline two\nfield report from Zorblax\n")
    (repo / "binary.bin").write_bytes(b"\0\0quuxcorp\0")
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "-m", "A first commit with a neutral message")
    return repo


def main() -> int:
    print("== the private-terms guard says where, never what ==")
    repo = scratch_repo()
    env = {"PRIVATE_TERMS": TERMS}

    r = run(repo, env=env)
    out = r.stdout + r.stderr
    check("a tracked file that names a term fails the check", r.returncode == 1, out[-400:])
    check("the finding names the file and the line", "notes.md:3" in out, out[-400:])
    check("the matched words and the pattern are never printed", not leaks(out), str(leaks(out)))
    check("a binary file is not read as text", "binary.bin" not in out, out[-400:])
    check("a clean file is not reported", "clean.md" not in out, out[-400:])

    # A file NAMED after the work is a finding too.
    (repo / "notes.md").write_text("nothing here now\n")
    (repo / "frobnitz_lab-plan.md").write_text("neutral text\n")
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "-m", "Neutral again")
    r = run(repo, env=env)
    out = r.stdout + r.stderr
    check("a path that names a term fails", r.returncode == 1 and "(its path)" in out, out[-400:])
    check("and its finding does not print the path's words", not leaks(out), str(leaks(out)))
    git(repo, "rm", "-q", "frobnitz_lab-plan.md")
    git(repo, "commit", "-q", "-m", "Gone")

    r = run(repo, env=env)
    check("a clean tree passes", r.returncode == 0 and "OK" in r.stdout, r.stdout + r.stderr)

    # The PR's own text, passed through the environment as CI passes it.
    r = run(repo, "--no-files", "--pr-title-env", "T", "--pr-body-env", "B",
            env={**env, "T": "Retire the copy", "B": "Para one.\n\nThe quuxcorp hub asked for it."})
    out = r.stdout + r.stderr
    check("a PR body that names a term fails", r.returncode == 1 and "PR body (line 3)" in out, out)
    check("a clean PR title is not reported", "PR title (line" not in out, out)
    check("the PR text's words are never printed", not leaks(out), str(leaks(out)))
    r = run(repo, "--no-files", "--pr-title-env", "T", env={**env, "T": "Zorblax support"})
    check("a PR title that names a term fails", r.returncode == 1 and "PR title (line 1)" in r.stdout, r.stdout)

    # Commit messages, by range and by sha.
    base = git(repo, "rev-parse", "HEAD")
    (repo / "x.md").write_text("x\n")
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "-m", "Add x\n\nAsked for by the frobnitz-lab.")
    bad = git(repo, "rev-parse", "HEAD")
    r = run(repo, "--no-files", "--commits", f"{base}..HEAD", env=env)
    out = r.stdout + r.stderr
    check("a commit message that names a term fails, by sha", r.returncode == 1 and bad in out, out)
    check("the commit's words are never printed", not leaks(out), str(leaks(out)))
    r = run(repo, "--no-files", "--commits-env", "RANGE", env={**env, "RANGE": f"{'0' * 40}..{bad}"})
    check("a range with no base checks its last commit, and says so",
          r.returncode == 1 and "no base" in r.stdout and bad in r.stdout, r.stdout + r.stderr)
    r = run(repo, "--no-files", "--commits", f"{base}~1..{base}", env=env)
    check("a range of clean messages passes", r.returncode == 0, r.stdout + r.stderr)

    # The pattern from a file, one alternative per line, comments skipped.
    terms = pathlib.Path(tempfile.mkdtemp()) / "terms.regex"
    terms.write_text("# made-up terms\nzorblax\n\nquuxcorp\n")
    r = run(repo, "--no-files", "--pr-body-env", "B", env={"PRIVATE_TERMS_FILE": str(terms), "B": "about QuuxCorp"})
    check("the pattern is read from PRIVATE_TERMS_FILE", r.returncode == 1 and "PR body" in r.stdout, r.stdout + r.stderr)

    # No pattern: a fork's pull request. A notice, and a pass.
    r = run(repo, env={"PRIVATE_TERMS": ""})
    check("with no pattern it passes, with a notice", r.returncode == 0 and "NOTICE" in r.stdout, r.stdout + r.stderr)

    # A pattern that does not compile is refused without being quoted.
    r = run(repo, env={"PRIVATE_TERMS": "zorblax(unclosed"})
    out = r.stdout + r.stderr
    check("a pattern that does not compile is refused (exit 2)", r.returncode == 2, out)
    check("and the refusal does not quote it", "zorblax" not in out and "unclosed" not in out, out)

    print()
    if failures:
        print(f"{len(failures)} check(s) FAILED")
        return 1
    print("All private-terms guard checks passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
