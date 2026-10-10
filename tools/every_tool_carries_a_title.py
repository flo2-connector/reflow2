#!/usr/bin/env python3
"""Every served tool carries a title, and every writing tool says whether it can destroy.

THE CLASS, ON THE WIRE. `tools/list` is what a client binds to, and two of its
annotation fields were never stated here until 2026-10-10
(`req:every-served-tool-carries-a-title-and-says-whether-it-destroys`):

  · `title` — MCP's short human name for a tool, distinct from its snake_case
    id. Measured on main before this gate: 197 tools, 0 titles.
  · `destructiveHint` — on a tool whose `readOnlyHint` is false, whether a call
    can delete or overwrite what a person made (true) or only adds, or revises
    with the prior state preserved (false). Measured: 7 of 119 writers said.

Why it matters: flo2 serves reflow2's tools unchanged through its chat door,
and the Claude Connectors Directory requires a clear title and read/write
hints on every tool. A new tool added without them is a clean toolsnap diff
and a green build, which is why this is a gate over the whole surface and not
a golden per tool.

Three assertions, each over EVERY tool the real binary lists over stdio:

  1. `title` is a non-empty string, is not the tool's name, and reads as a
     human name (no underscore in it);
  2. `readOnlyHint` is stated (a boolean) — the class smoke_mcp pins, restated
     here because assertion 3 keys on it;
  3. where `readOnlyHint` is false, `destructiveHint` is stated (a boolean).

A read-only tool is NOT asked for `destructiveHint`: MCP says the field is
meaningful only when `readOnlyHint` is false.

The gate refuses to pass having measured nothing: an empty tool list is a
failure, not a vacuous success.

Usage (from the repo root, after `cargo build -p reflow2-mcp`):

    python3 tools/every_tool_carries_a_title.py
    python3 tools/every_tool_carries_a_title.py --bin target/release/reflow2-mcp

Exits 0 when every tool passes, 1 otherwise, naming each offender and what it
lacks. Standard library only.
"""
from __future__ import annotations

import argparse
import pathlib
import shutil
import sys
import tempfile

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from smoke_mcp import Server  # noqa: E402  (path set above)
from reflow2_bin import default_bin  # noqa: E402  (one binary for every gate)


def judge(tools: list[dict]) -> tuple[list[str], dict]:
    """The verdict on one served list: (offender lines, counts)."""
    offenders: list[str] = []
    titled = 0
    writers = 0
    writers_stated = 0
    destructive: list[str] = []
    for t in sorted(tools, key=lambda t: t["name"]):
        name = t["name"]
        title = t.get("title")
        if not isinstance(title, str) or not title.strip():
            offenders.append(f"{name}: no title")
        elif title == name:
            offenders.append(f"{name}: title is the tool's name, not a human name")
        elif "_" in title:
            offenders.append(f"{name}: title {title!r} reads as an identifier, not a name")
        else:
            titled += 1
        ann = t.get("annotations") or {}
        ro = ann.get("readOnlyHint")
        if not isinstance(ro, bool):
            offenders.append(f"{name}: readOnlyHint not stated")
            continue
        if ro:
            continue
        writers += 1
        dh = ann.get("destructiveHint")
        if not isinstance(dh, bool):
            offenders.append(f"{name}: writes (readOnlyHint=false) but destructiveHint not stated")
            continue
        writers_stated += 1
        if dh:
            destructive.append(name)
    counts = {
        "tools": len(tools),
        "titled": titled,
        "writers": writers,
        "writers_with_destructive_hint": writers_stated,
        "destructive": destructive,
    }
    return offenders, counts


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--bin",
        default=None,
        help="reflow2-mcp binary (default: $REFLOW2_BIN, else this checkout's debug build)",
    )
    a = ap.parse_args()
    a.bin = a.bin or default_bin()

    tmp = pathlib.Path(tempfile.mkdtemp(prefix="every-tool-carries-a-title-"))
    try:
        s = Server(a.bin, str(tmp / "graph"))
        try:
            tools = s.rpc("tools/list", {})["result"]["tools"]
        finally:
            s.close()
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    if not tools:
        print(f"FAIL: {a.bin} listed no tools — nothing was measured")
        return 1

    offenders, c = judge(tools)
    print(
        f"{c['tools']} tools served by {a.bin}: {c['titled']} titled; "
        f"{c['writers']} writers, {c['writers_with_destructive_hint']} state destructiveHint; "
        f"destructive=true on {len(c['destructive'])}: {', '.join(c['destructive']) or '(none)'}"
    )
    if offenders:
        print(f"\nFAIL: {len(offenders)} problem(s) on the served surface:")
        for line in offenders:
            print(f"  - {line}")
        print(
            "\nEvery #[tool(...)] in crates/reflow2-mcp/src carries `title = \"...\"`, and every "
            "one with `read_only_hint = false` carries `destructive_hint = true|false` beside it."
        )
        return 1
    print("OK: every served tool carries a title, and every writing tool says whether it can destroy")
    return 0


if __name__ == "__main__":
    sys.exit(main())
