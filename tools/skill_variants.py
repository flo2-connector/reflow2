#!/usr/bin/env python3
"""A skill's local and hosted lines, and the repository's own skill mirrors.

`req:a-served-skill-says-what-is-true-where-it-is-served`, governed by
`dec:a-skill-line-that-differs-when-hosted-has-a-local-and-a-hosted-version`.
A line of a SKILL.md that differs between a local install and an engine served
for others is written both ways, each marker alone on its line:

    <!-- reflow2:local -->
    lines true where the engine runs on the reader's own machine
    <!-- reflow2:hosted -->
    lines true where it is served for others (flo2.io, or any host)
    <!-- reflow2:end -->

The engine serves the side for how it runs (crates/reflow2-mcp/src/skill_variants.rs
renders it, and build.rs refuses a malformed marking). This file is its twin
for what Python reads: skill_lint (which checks both sides) and the mirrors.

THE MIRRORS hold the LOCAL version. `.claude/skills/` and `.grok/skills/` are
what this repository's own local sessions load, so they carry what a local
engine serves, rendered from `getting-started/skills/` (the one source). Any
other file in a skill directory is copied byte for byte. The Rust test
`a_served_skill_says_what_is_true_where_it_is_served.rs` holds each mirror to
the Rust rendering, so the two renderers cannot disagree unseen.

Run:  python3 tools/skill_variants.py --write-mirrors    (stdlib only)
      python3 tools/skill_variants.py --check            (exit 1 if a mirror is stale)
"""

from __future__ import annotations

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
SOURCE = REPO / "getting-started/skills"
MIRRORS = (REPO / ".claude/skills", REPO / ".grok/skills")

LOCAL = "<!-- reflow2:local -->"
HOSTED = "<!-- reflow2:hosted -->"
END = "<!-- reflow2:end -->"
SIDES = ("local", "hosted")


def render(text: str, served: str) -> str:
    """`text` as it reads where it is `served` ("local" or "hosted"): each
    marked block's lines for that side, the marker lines gone. Unmarked text
    is returned unchanged."""
    if served not in SIDES:
        raise ValueError(f"served must be one of {SIDES}, not {served!r}")
    out, at = [], "both"
    # Split on "\n" alone, keeping it, exactly as the Rust twin does
    # (`split_inclusive('\n')`): splitlines() would also break on characters
    # Rust does not.
    for line in re.findall(r"[^\n]*\n|[^\n]+\Z", text):
        bare = line.rstrip("\r\n")
        if bare == LOCAL:
            at = "local"
        elif bare == HOSTED:
            at = "hosted"
        elif bare == END:
            at = "both"
        elif at in ("both", served):
            out.append(line)
    return "".join(out)


def has_variants(text: str) -> bool:
    return any(line.rstrip("\r") == LOCAL for line in text.split("\n"))


def problems(text: str) -> list[str]:
    """Every way the marking in `text` is malformed, each naming its line, in
    the same terms as the Rust twin."""
    out: list[str] = []
    lines = text.split("\n")
    frontmatter_end = 0
    if lines[:1] == ["---"]:
        frontmatter_end = next((k + 1 for k in range(1, len(lines)) if lines[k] == "---"), 0)
    open_at, side, counts = None, None, [0, 0]
    for i, line in enumerate(lines):
        n = i + 1
        marker = line in (LOCAL, HOSTED, END)
        if not marker and "<!-- reflow2:" in line:
            out.append(f"line {n}: a reflow2 marker must stand alone on its line, exactly as written")
            continue
        if marker and i < frontmatter_end:
            out.append(f"line {n}: a marker inside the frontmatter")
            continue
        if line == LOCAL:
            if open_at is None:
                open_at, side, counts = n, "local", [0, 0]
            else:
                out.append(f"line {n}: a block opened inside the one opened at line {open_at}")
        elif line == HOSTED:
            if side == "local":
                side = "hosted"
            else:
                out.append(f"line {n}: `{HOSTED}` with no `{LOCAL}` open before it")
        elif line == END:
            if side == "hosted":
                if counts == [0, 0]:
                    out.append(f"line {open_at}: a block with nothing on either side")
                open_at, side = None, None
            else:
                out.append(f"line {n}: `{END}` with no `{LOCAL}` … `{HOSTED}` before it")
        elif side and line.strip():
            counts[SIDES.index(side)] += 1
    if open_at is not None:
        out.append(f"line {open_at}: a block that is never closed with `{END}`")
    return out


def mirror_bytes(source: pathlib.Path) -> bytes:
    """What a mirror holds for `source`, a file under getting-started/skills:
    the local version of a SKILL.md, any other file as it is."""
    if source.name == "SKILL.md":
        return render(source.read_text(encoding="utf-8"), "local").encode("utf-8")
    return source.read_bytes()


def stale_mirrors() -> dict[str, list[str]]:
    """Mirror -> its paths that are missing, extra, or not what mirror_bytes says."""
    sources = {f.relative_to(SOURCE): f for f in SOURCE.rglob("*") if f.is_file()}
    out: dict[str, list[str]] = {}
    for mirror in MIRRORS:
        held = {f.relative_to(mirror) for f in mirror.rglob("*") if f.is_file()} if mirror.exists() else set()
        bad = sorted(str(p) for p in held - sources.keys())
        for rel, src in sources.items():
            if rel not in held or (mirror / rel).read_bytes() != mirror_bytes(src):
                bad.append(str(rel))
        if bad:
            out[str(mirror.relative_to(REPO))] = sorted(bad)
    return out


def write_mirrors() -> list[str]:
    """Bring every mirror to the source's local version; return what changed."""
    changed = []
    sources = {f.relative_to(SOURCE): f for f in SOURCE.rglob("*") if f.is_file()}
    for mirror in MIRRORS:
        for rel, src in sources.items():
            dest = mirror / rel
            want = mirror_bytes(src)
            if not dest.exists() or dest.read_bytes() != want:
                dest.parent.mkdir(parents=True, exist_ok=True)
                dest.write_bytes(want)
                changed.append(str(dest.relative_to(REPO)))
        for f in sorted(mirror.rglob("*")):
            if f.is_file() and f.relative_to(mirror) not in sources:
                f.unlink()
                changed.append(f"{f.relative_to(REPO)} (removed)")
    return changed


def main(argv: list[str]) -> int:
    if argv == ["--write-mirrors"]:
        for path in write_mirrors():
            print(f"wrote {path}")
        return 0
    if argv == ["--check"]:
        stale = stale_mirrors()
        for mirror, paths in stale.items():
            print(f"{mirror}: not the local version of the source: {paths}")
        broken = {
            str(p.relative_to(REPO)): problems(p.read_text(encoding="utf-8"))
            for p in sorted(SOURCE.glob("*/SKILL.md"))
        }
        for path, found in broken.items():
            for problem in found:
                print(f"{path}: {problem}")
        return 1 if stale or any(broken.values()) else 0
    print(__doc__.split("Run:")[1].strip(), file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
