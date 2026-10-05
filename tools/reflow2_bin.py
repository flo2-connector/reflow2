#!/usr/bin/env python3
"""Which reflow2-mcp binary a gate runs: one answer, for every gate.

⭐ WHY THIS EXISTS. Until 2026-10-03 each gate that drives the binary chose it
by itself, and they disagreed. `reflow2_check.py` read `$REFLOW2_BIN`, then
`target/debug`, then `target/release`. `replies_are_bounded.py`,
`a_reply_is_sent_once.py` and about twenty others hard-coded
`target/debug/reflow2-mcp`. `test_content_policy.py` read `$REFLOW2_MCP` and
`test_latent_promotion.py` read `$REFLOW2_MCP_BIN`. So `run_ci_gates.py`, which
passes its environment to every gate, could not point the whole set at one
binary: with only a release build, or a build kept outside this checkout,
`REFLOW2_BIN=… python3 tools/run_ci_gates.py` passed `reflow2_check` and failed
the rest for a binary they never looked for (fix program item 8, 2026-10-03).

The order, the same as reflow2_check's:
  1. `$REFLOW2_BIN`, when set — used as given, never second-guessed;
  2. this checkout's `target/debug/reflow2-mcp` — what `cargo build` and CI make;
  3. this checkout's `target/release/reflow2-mcp`, SAID on stderr, because a
     release build is often older than the source and a gate that silently
     tested it would report on code that is not the code in front of you.
With none of them, the debug path is returned so the gate's own "not found"
names the build to make.

`reflow2_check.py` keeps its own copy of this order, plus a last look on PATH,
because it is installed into consumer projects alone, where there is no
`target/`. `test_run_ci_gates.py` holds every other gate to this module.

Standard library only.
"""

from __future__ import annotations

import os
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
ENV = "REFLOW2_BIN"
DEBUG = REPO / "target" / "debug" / "reflow2-mcp"
RELEASE = REPO / "target" / "release" / "reflow2-mcp"


def default_bin() -> str:
    """The binary a gate runs when it is not told otherwise. See the module doc."""
    env = os.environ.get(ENV)
    if env:
        return env
    if DEBUG.exists():
        return str(DEBUG)
    if RELEASE.exists():
        print(
            f"note: no debug build at {DEBUG}; running {RELEASE}, which may be older than "
            f"the source. Set ${ENV} to choose.",
            file=sys.stderr,
        )
        return str(RELEASE)
    return str(DEBUG)


if __name__ == "__main__":
    print(default_bin())
