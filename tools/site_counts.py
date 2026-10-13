#!/usr/bin/env python3
"""Count reflow2's own design at its releases, for the site (site/counts.json).

WHY IT EXISTS. req:the-site-shows-why-linked-decisions-are-more-than-a-record asks
the site to show that reflow2 is more than a record of decisions, with reflow2's
own design as the evidence: one small decision at a time, adding up to a design
nobody could hold in their head. By
dec:the-sites-setup-pages-are-hand-written-and-its-design-pages-stay-generated,
anything on the site that shows a real design is generated, never typed. So the
numbers are counted here from the design committed at each release tag, when the
site is published (.github/workflows/pages.yml), and the page draws them.

⚠️ A RELEASE'S MANIFEST IS COUNTED APART. Every cut lists the files it ships as
INCLUDES links: 35,421 of the 53,466 links at v0.83.5. They are bookkeeping, not
decisions linked to each other, so `links` leaves them out and `manifest_links`
carries them. Counting them in would make the design look three times as linked
as it is.

TWO LAYOUTS. Up to 0.78.0 the design was one file, docs/design/reflow2.json. From
0.79.0 it is one file per record under docs/design/reflow2/ (nodes/<Type>/...,
edges/xx/...). A tag holding neither is skipped and named in `skipped`, never
counted as empty.

Usage:
    python3 tools/site_counts.py --out site/counts.json   # needs the tags (fetch-depth 0)
"""
from __future__ import annotations

import argparse
import collections
import json
import re
import subprocess
import sys
from datetime import date
from pathlib import Path

SINGLE_FILE = "docs/design/reflow2.json"
ITEM_DIR = "docs/design/reflow2"
MANIFEST = "INCLUDES"
# The kinds a reader knows by name; the rest are counted in `records` only.
NAMED = {
    "Requirement": "requirements",
    "Decision": "decisions",
    "Capability": "capabilities",
    "Verification": "checks",
    "ChangeEvent": "changes",
}
TAG = re.compile(r"^v(\d+)\.(\d+)\.(\d+)$")


def count_design(node_types: list[str], edge_types: list[str]) -> dict:
    """The counts the page shows, from one design's node and edge types."""
    kinds = collections.Counter(node_types)
    links = collections.Counter(edge_types)
    out = {
        "records": len(node_types),
        "links": len(edge_types) - links[MANIFEST],
        "manifest_links": links[MANIFEST],
        "kinds_of_record": len(kinds),
        "kinds_of_link": len([k for k in links if k != MANIFEST]),
    }
    for kind, word in NAMED.items():
        out[word] = kinds[kind]
    return out


def weekly(points: list[dict]) -> list[dict]:
    """The last release of each ISO week, and always the latest release.

    Every release would be ~100 points, most of them days apart; one a week
    shows the growth without reading a hundred designs at publish time.
    """
    if not points:
        return []
    by_week: dict[tuple[int, int], dict] = {}
    for p in points:
        y, w, _ = date.fromisoformat(p["date"]).isocalendar()
        by_week[(y, w)] = p  # points arrive in release order, so the last wins
    chosen = list(by_week.values())
    if chosen[-1] is not points[-1]:
        chosen.append(points[-1])
    return chosen


def _git(repo: Path, *args: str, data: bytes | None = None) -> bytes:
    return subprocess.run(
        ["git", "-C", str(repo), *args], input=data, check=True, capture_output=True
    ).stdout


def release_tags(repo: Path) -> list[dict]:
    """Every vX.Y.Z tag in version order, with the date of the commit it names."""
    tags = []
    for name in _git(repo, "tag", "-l", "v*").decode().split():
        m = TAG.match(name)
        if m:
            tags.append((tuple(int(g) for g in m.groups()), name))
    tags.sort()
    out = []
    for _, name in tags:
        day = _git(repo, "log", "-1", "--format=%cs", f"{name}^{{commit}}").decode().strip()
        out.append({"tag": name, "date": day})
    return out


def _paths(repo: Path, rev: str, under: str) -> list[tuple[str, str]]:
    """(blob id, path) for every file under `under` at `rev`."""
    out = []
    raw = _git(repo, "ls-tree", "-r", rev, "--", under).decode()
    for line in raw.splitlines():
        meta, path = line.split("\t", 1)
        out.append((meta.split()[2], path))
    return out


def _blobs(repo: Path, ids: list[str]) -> list[bytes]:
    """The contents of many blobs in one `git cat-file --batch`."""
    if not ids:
        return []
    raw = _git(repo, "cat-file", "--batch", data=("\n".join(ids) + "\n").encode())
    out, i = [], 0
    for _ in ids:
        nl = raw.index(b"\n", i)
        size = int(raw[i:nl].split()[2])
        out.append(raw[nl + 1 : nl + 1 + size])
        i = nl + 1 + size + 1
    return out


def design_at(repo: Path, rev: str) -> tuple[list[str], list[str]] | None:
    """The node types and edge types of the design committed at `rev`, or None."""
    single = _paths(repo, rev, SINGLE_FILE)
    if single:
        try:
            d = json.loads(_blobs(repo, [single[0][0]])[0])
        except (ValueError, IndexError):
            return None
        if not isinstance(d, dict) or not isinstance(d.get("nodes"), list):
            return None
        return [n.get("node_type", "") for n in d["nodes"]], [
            e.get("edge_type", "") for e in d.get("edges", [])
        ]
    nodes = _paths(repo, rev, f"{ITEM_DIR}/nodes")
    if not nodes:
        return None
    node_types = [p.split("/")[-2] for _, p in nodes]
    edges = _paths(repo, rev, f"{ITEM_DIR}/edges")
    edge_types = [json.loads(b).get("edge_type", "") for b in _blobs(repo, [b for b, _ in edges])]
    return node_types, edge_types


def counts(repo: Path) -> dict:
    tags = release_tags(repo)
    if not tags:
        raise SystemExit(
            "site_counts: no vX.Y.Z tags here. The site's numbers come from the design at each "
            "release, so the checkout needs its tags (actions/checkout with fetch-depth: 0)."
        )
    points, skipped = [], []
    for t in weekly(tags):
        found = design_at(repo, t["tag"])
        if found is None:
            skipped.append(t["tag"])
            continue
        points.append({**t, **count_design(*found)})
    if not points:
        raise SystemExit("site_counts: no release tag holds a readable design")
    return {
        "about": "reflow2's own design, counted from the copy committed at each release tag "
        "when the site was published. links leaves out each release's manifest (INCLUDES), "
        "which is counted apart as manifest_links.",
        "counted_at": _git(repo, "rev-parse", "HEAD").decode().strip(),
        "points": points,
        "latest": points[-1],
        "skipped": skipped,
    }


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--repo", default=".", type=Path)
    ap.add_argument("--out", required=True, type=Path)
    a = ap.parse_args(argv)
    result = counts(a.repo)
    a.out.write_text(json.dumps(result, indent=1) + "\n")
    last = result["latest"]
    print(
        f"site_counts: {len(result['points'])} release(s), latest {last['tag']}: "
        f"{last['records']} records, {last['links']} links "
        f"(+{last['manifest_links']} in release manifests); skipped {result['skipped'] or 'none'}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
