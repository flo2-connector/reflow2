#!/usr/bin/env python3
"""Tests for the AGENTS.md size ceiling (req:agents-md-can-only-hold-or-shrink).

AGENTS.md is the file every agent working on reflow2 reads first (CLAUDE.md
points at it), so its size is spent at the start of every session. It grew from
66,227 bytes at v0.60.0 to 103,893 at v0.101.0 with nothing measuring it
(fact:agents-md-grew-from-66-kb-to-104-kb-between-v0-60-and-v0-101-and-nothing-
bounds-it-2026-11-20). These pin what the check must do: a larger file fails and
says why, an equal or smaller one passes, and a missing ceiling is a failure,
never a pass.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

TOOL = Path(__file__).resolve().parent / "check_agents_md_size.py"
REPO = TOOL.parent.parent
AGENTS = REPO / "AGENTS.md"
CEILING = REPO / "tools" / "agents_md_ceiling.txt"


def run() -> subprocess.CompletedProcess[str]:
    return subprocess.run([sys.executable, str(TOOL)], capture_output=True, text=True, cwd=REPO)


def test_the_repo_as_it_stands_passes() -> None:
    r = run()
    assert r.returncode == 0, r.stdout + r.stderr
    assert "OK" in r.stdout


def test_a_larger_agents_md_fails_and_says_where_text_goes() -> None:
    original = AGENTS.read_bytes()
    try:
        AGENTS.write_bytes(original + b"\nOne more paragraph of standing guidance.\n")
        r = run()
        assert r.returncode == 1, "a file over its ceiling must fail the build"
        out = r.stdout
        assert str(len(original) + 42) in out or "bytes" in out, out
        assert "design" in out, f"the failure must say where the text belongs instead:\n{out}"
    finally:
        AGENTS.write_bytes(original)


def test_a_smaller_agents_md_passes() -> None:
    original = AGENTS.read_bytes()
    try:
        AGENTS.write_bytes(original[: len(original) // 2])
        r = run()
        assert r.returncode == 0, r.stdout
    finally:
        AGENTS.write_bytes(original)


def test_a_missing_ceiling_fails_rather_than_passing() -> None:
    original = CEILING.read_bytes()
    try:
        CEILING.unlink()
        r = run()
        assert r.returncode != 0, "with no ceiling recorded there is nothing to hold the file to"
    finally:
        CEILING.write_bytes(original)


def main() -> int:
    tests = [
        test_the_repo_as_it_stands_passes,
        test_a_larger_agents_md_fails_and_says_where_text_goes,
        test_a_smaller_agents_md_passes,
        test_a_missing_ceiling_fails_rather_than_passing,
    ]
    failed = 0
    for t in tests:
        try:
            t()
            print(f"PASS  {t.__name__}")
        except Exception as e:  # noqa: BLE001 — a crash is a failure too
            print(f"FAIL  {t.__name__}: {type(e).__name__}: {e}")
            failed += 1
    print(f"\n{len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
