#!/usr/bin/env python3
"""AGENTS.md can only hold or shrink (req:agents-md-can-only-hold-or-shrink).

AGENTS.md is the file every agent working on reflow2 reads first: CLAUDE.md
points at it, and so does every harness that follows the agents.md convention.
Its size is therefore spent at the start of every session. It grew from 66,227
bytes at v0.60.0 to 103,893 at v0.101.0 with nothing measuring it, which an
outside review of v0.101.0 named and Anthony settled on 2026-11-20
(dec:idea-the-maintainer-agents-md-has-a-size-budget): a ceiling, recorded in
tools/agents_md_ceiling.txt, that the file may not exceed. Nothing is moved out
by this check; it only stops the growth.

Usage:  python3 tools/check_agents_md_size.py
Exit 0 = AGENTS.md is at or under its ceiling.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
AGENTS = REPO / "AGENTS.md"
CEILING = REPO / "tools" / "agents_md_ceiling.txt"


def ceiling() -> int:
    if not CEILING.exists():
        sys.exit(f"FAIL: {CEILING.relative_to(REPO)} is missing, so AGENTS.md has no ceiling to be held to")
    for line in CEILING.read_text().splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            try:
                return int(line)
            except ValueError:
                sys.exit(f"FAIL: {CEILING.relative_to(REPO)} must hold a byte count, found {line!r}")
    sys.exit(f"FAIL: {CEILING.relative_to(REPO)} holds no byte count")


def main() -> int:
    limit = ceiling()
    size = AGENTS.stat().st_size
    if size > limit:
        print(
            f"FAIL: AGENTS.md is {size} bytes, over its ceiling of {limit} by {size - limit}.\n"
            "Every agent working on reflow2 reads this file first, so each byte is spent at the\n"
            "start of every session. Put the new text where it is needed instead: a rule or its\n"
            "reasoning in the design (a DesignRule or a Decision, which search_design finds), or\n"
            "repo know-how at the step it bears on. Then make room here, or, if this file really\n"
            "must grow, raise the ceiling in tools/agents_md_ceiling.txt as its own reviewed\n"
            "change that says why."
        )
        return 1
    note = f" ({limit - size} bytes of room; lower the ceiling to {size} to keep it)" if size < limit else ""
    print(f"check_agents_md_size: OK — AGENTS.md is {size} bytes, ceiling {limit}{note}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
