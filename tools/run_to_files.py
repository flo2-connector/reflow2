#!/usr/bin/env python3
"""Turn a real test run into per-FILE outcomes that reconcile_verification reads.

A test runner reports in files and test names; the design records checks. This
is the bridge between the two, so a run can be fed back with
`reconcile_verification(observed_by_file=..., record_events=true,
detected_at=...)` without hand-mapping a single id. It is the first half of
step 4 (dec:step-4-is-built-on-real-runs-with-no-threshold-and-unscored-good-news):
the doubt layer is only worth building once real failures actually flow.

Three sources, any mix, one JSON list out (worst outcome per file wins):

  --cargo FILE      the text output of `cargo test --no-fail-fast` (`-` = stdin).
                    Each test TARGET's "test result:" line is its file's
                    outcome. A target cargo lists as failed that never STARTED
                    (no "Running" line: it did not compile) is reported as
                    `blocked`, not dropped and not `failed`; one that started
                    and printed no result (it crashed while running) is failed.
                    A test PROGRAM whose root declares its files as modules
                    (`mod one;`, `#[path = "..."] mod two;` — one program for
                    many test files) is read per test instead: `test one::x
                    ... ok` is an outcome of the file `one` names, so each file
                    is still placed on its own. Such a program that did not
                    compile leaves every file it declares `blocked`; one that
                    crashed leaves each file `blocked` unless a test in it
                    failed, and its root file `failed`, because which file the
                    crash came from is not in the log.
  --junit FILE      JUnit XML — what pytest (--junitxml), cargo-nextest, jest,
                    go-junit-report and most CI runners write. Grouped by each
                    testcase's `file` attribute; testcases with none are
                    counted and named in stderr, never guessed. An `<error>`
                    that says the test body never ran — pytest's "collection
                    failure" and "failed on setup" — is `blocked`; any other
                    `<error>` (an exception raised while the test ran, as JUnit
                    in Java reports one) is failed.

`blocked` means the run could not run the check, so it says nothing about the
thing checked. reconcile_verification takes it since 2026-10-03; before that a
collection error could only be fed back as `failed`, and detect_gaps then said
the part "did not work as designed"
(fact:root-cause-a-check-that-did-not-run-reads-did-not-work-as-designed-because-only-failing-is-loud-2026-10-02).
  --run-python GLOB run each matching python test script and record its exit
                    code (0 = passed). This converter is named run_to_files.py,
                    NOT test_*.py, so that glob never runs it as a test. reflow2's own tools/test_*.py are plain
                    unittest scripts with no shared runner, so their exit code
                    IS their result.

Paths come out relative to --root (default: the current directory), the form a
design records `location` in. Nothing here reads or writes the design.
"""

from __future__ import annotations

import argparse
import glob
import os
import json
import pathlib
import re
import subprocess
import sys
import xml.etree.ElementTree as ET

# Worst wins, and the order matches the core's (verify.rs `outcome_rank`): a
# failure is evidence the thing is broken; a blocked run is evidence of
# nothing, so it outranks a pass but never hides a failure.
RANK = {"passed": 0, "skipped": 1, "blocked": 2, "failed": 3}

# What pytest writes in an `<error message=...>` when the test BODY never ran.
# Matched at the start of the message, case-insensitively.
NEVER_RAN = ("collection failure", "failed on setup")

RUNNING = re.compile(r"^\s*Running (unittests )?(\S+) \((\S+)\)")
RESULT = re.compile(r"^test result: (ok|FAILED)\.")
# One test's line, as libtest prints it: `test one::x ... ok`.
TEST_LINE = re.compile(r"^test (\S+) \.\.\. (ok|FAILED)$")
# A module a test program's root declares out of line, optionally with the
# `#[path = "..."]` that says where its file is.
MOD_DECL = re.compile(r'^\s*(?:#\[path\s*=\s*"([^"]+)"\]\s*)?(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;')
PATH_ATTR = re.compile(r'^\s*#\[path\s*=\s*"([^"]+)"\]\s*$')
FAILED_TARGET = re.compile(r"^\s*`-p (\S+) --(test|lib|bin) ?(\S*)`")
# CI runners colour cargo's output; a raw log carries the escapes.
ANSI = re.compile(r"\x1b\[[0-9;]*m")


def worst(into: dict[str, str], location: str, outcome: str) -> None:
    """Keep the worst outcome seen for a file: one failure fails the file."""
    if location not in into or RANK[outcome] > RANK[into[location]]:
        into[location] = outcome


SKIP_DIRS = {"target", ".git", "node_modules", ".venv"}


def crate_dirs(root: pathlib.Path) -> list[pathlib.Path]:
    """Every directory holding a Cargo.toml, found once, never descending into
    build output — a `target/` tree is the bulk of a Rust checkout."""
    found = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        if "Cargo.toml" in filenames:
            found.append(pathlib.Path(dirpath))
    return found


def resolve_cargo_path(
    printed: str, binary: str, crates: list[pathlib.Path], root: pathlib.Path
) -> str | None:
    """cargo prints a target's path relative to ITS crate; find which crate."""
    # the deps binary is `<crate_or_stem>-<hash>`; crate names use `_`.
    stem = pathlib.Path(binary).name.rsplit("-", 1)[0]
    candidates = [c / printed for c in crates if (c / printed).exists()]
    if len(candidates) > 1:
        # a unittests src/lib.rs exists in every crate: disambiguate by name.
        depth = len(pathlib.Path(printed).parts)
        named = [
            c for c in candidates if c.parents[depth - 1].name.replace("-", "_") == stem
        ]
        candidates = named or candidates
    if len(candidates) != 1:
        return None
    return str(candidates[0].resolve().relative_to(root.resolve()))


def program_modules(program: pathlib.Path, root: pathlib.Path) -> dict[str, str]:
    """The files a test program's root declares as modules: {module: file}.

    A crate root resolves `mod one;` beside itself (`one.rs`, else
    `one/mod.rs`), and `#[path = "p"] mod one;` at `p` relative to its own
    directory — so a program can gather many test files that each keep their
    own path, and their tests come out named `one::x`. Only declarations whose
    file exists are returned; a root with none is an ordinary one-file target.
    """
    try:
        lines = program.read_text(encoding="utf-8").splitlines()
    except OSError:
        return {}
    found: dict[str, str] = {}
    pending_path: str | None = None
    for line in lines:
        attr = PATH_ATTR.match(line)
        if attr:
            pending_path = attr.group(1)
            continue
        m = MOD_DECL.match(line)
        if m:
            given = m.group(1) or pending_path
            name = m.group(2)
            if given:
                candidates = [program.parent / given]
            else:
                candidates = [program.parent / f"{name}.rs", program.parent / name / "mod.rs"]
            for c in candidates:
                if c.is_file():
                    found[name] = str(c.resolve().relative_to(root.resolve()))
                    break
        if line.strip() and not line.lstrip().startswith(("#[", "//")):
            pending_path = None
    return found


def from_cargo(text: str, root: pathlib.Path, out: dict[str, str]) -> list[str]:
    unresolved: list[str] = []
    current: str | None = None
    modules: dict[str, str] = {}
    failed_inside = False
    started: set[str] = set()
    completed: set[str] = set()
    # Each multi-file program's modules, by its root file, for the pass below.
    programs: dict[str, dict[str, str]] = {}
    crates = crate_dirs(root)
    text = ANSI.sub("", text)
    for line in text.splitlines():
        m = RUNNING.match(line)
        if m:
            current = resolve_cargo_path(m.group(2), m.group(3), crates, root)
            modules, failed_inside = {}, False
            if current is None:
                unresolved.append(m.group(2))
            else:
                started.add(current)
                # Unit tests name their crate's modules too, but are reported
                # as the target they always were; only a test PROGRAM's
                # modules are separate test files.
                if not m.group(1):
                    modules = program_modules(root / current, root)
                    if modules:
                        programs[current] = modules
            continue
        t = TEST_LINE.match(line.strip())
        if t and current and modules:
            # A multi-file program: each test's outcome is its own file's.
            # `ignored` says nothing, as a target's ignored count never did.
            owner = modules.get(t.group(1).split("::", 1)[0], current)
            outcome = "passed" if t.group(2) == "ok" else "failed"
            failed_inside = failed_inside or outcome == "failed"
            worst(out, owner, outcome)
            continue
        r = RESULT.match(line.strip())
        if r and current:
            completed.add(current)
            if not modules:
                worst(out, current, "passed" if r.group(1) == "ok" else "failed")
            elif r.group(1) == "FAILED" and not failed_inside:
                # The program failed and no test line said which file: the
                # failure is never dropped, so the program's root carries it.
                worst(out, current, "failed")
            current = None
    # Targets cargo lists as failed after the run. One that STARTED (it has a
    # Running line) and printed no result crashed while running: failed. One
    # that never started did not compile, so none of its tests ran: blocked.
    for line in text.splitlines():
        f = FAILED_TARGET.match(line)
        if f and f.group(2) == "test" and f.group(3):
            for c in crates:
                p = c / "tests" / f"{f.group(3)}.rs"
                if p.exists():
                    rel = str(p.resolve().relative_to(root.resolve()))
                    ran = rel in started
                    declared = programs.get(rel) or program_modules(p, root)
                    if not declared:
                        worst(out, rel, "failed" if ran else "blocked")
                    elif rel not in completed:
                        # A multi-file program that did not compile ran none
                        # of its files; one that crashed did not finish them,
                        # and the log does not say which file the crash came
                        # from. So each file is blocked unless a test in it
                        # already failed, and the root carries the crash.
                        worst(out, rel, "failed" if ran else "blocked")
                        for module_file in declared.values():
                            worst(out, module_file, "blocked")
                    # A program that ran to its result line has already said,
                    # test by test, which of its files failed.
    return unresolved


def from_junit(path: str, root: pathlib.Path, out: dict[str, str]) -> int:
    no_file = 0
    tree = ET.parse(path)
    for case in tree.iter("testcase"):
        f = case.get("file")
        if not f:
            no_file += 1
            continue
        error = case.find("error")
        if case.find("failure") is not None:
            outcome = "failed"
        elif error is not None:
            message = (error.get("message") or "").strip().lower()
            outcome = "blocked" if message.startswith(NEVER_RAN) else "failed"
        elif case.find("skipped") is not None:
            outcome = "skipped"
        else:
            outcome = "passed"
        p = pathlib.Path(f)
        rel = str(p.resolve().relative_to(root.resolve())) if p.is_absolute() else str(p)
        worst(out, rel.removeprefix("./"), outcome)
    return no_file


def from_python(pattern: str, root: pathlib.Path, out: dict[str, str]) -> None:
    for script in sorted(glob.glob(str(root / pattern))):
        rc = subprocess.run(
            [sys.executable, script], cwd=root, capture_output=True, text=True
        ).returncode
        rel = str(pathlib.Path(script).resolve().relative_to(root.resolve()))
        worst(out, rel, "passed" if rc == 0 else "failed")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--cargo", action="append", default=[])
    ap.add_argument("--junit", action="append", default=[])
    ap.add_argument("--run-python", action="append", default=[])
    ap.add_argument("--root", default=".")
    a = ap.parse_args(argv)
    if not (a.cargo or a.junit or a.run_python):
        ap.error("give at least one of --cargo, --junit, --run-python")
    root = pathlib.Path(a.root)
    out: dict[str, str] = {}
    for c in a.cargo:
        text = sys.stdin.read() if c == "-" else pathlib.Path(c).read_text()
        # A log with no test targets in it is not a clean run. MEASURED
        # 2026-09-23: `gh run view --log` exited 0 with an EMPTY log, and this
        # printed "files: 0" — the reading an all-clear and an empty input
        # share. Refuse it by name instead.
        if not any(RUNNING.match(line) for line in ANSI.sub("", text).splitlines()):
            print(
                f"refused: {c} holds no cargo test target at all (no 'Running' line) — "
                "an empty or truncated log is not a run. For a GitHub Actions job, "
                "`gh api repos/<owner>/<repo>/actions/jobs/<id>/logs` returns the log "
                "when `gh run view --log` comes back empty.",
                file=sys.stderr,
            )
            return 2
        for u in from_cargo(text, root, out):
            print(f"note: could not place cargo target {u}", file=sys.stderr)
    for j in a.junit:
        n = from_junit(j, root, out)
        if n:
            print(f"note: {n} JUnit testcase(s) name no file and were not placed", file=sys.stderr)
    for g in a.run_python:
        from_python(g, root, out)
    json.dump(
        [{"location": k, "outcome": v} for k, v in sorted(out.items())],
        sys.stdout,
        indent=1,
    )
    print()
    counts = {o: sum(1 for v in out.values() if v == o) for o in RANK}
    print(f"files: {len(out)} — {counts}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
