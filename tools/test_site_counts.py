#!/usr/bin/env python3
"""Tests for the site's counts of reflow2's own design (tools/site_counts.py).

The numbers go on a public page as evidence, so what this pins is that they are
never inflated and never quietly empty:

  - a release's manifest (INCLUDES) is counted apart and never in `links`;
  - both layouts the design has been committed in are read, the single file up to
    0.78.0 and one file per record from 0.79.0, and give the same counts for the
    same design;
  - a tag holding no readable design is named in `skipped`, never counted as an
    empty design;
  - one point is kept per week, and the latest release is always kept.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import site_counts  # noqa: E402

NODES = [
    {"node_id": "req:a", "node_type": "Requirement"},
    {"node_id": "dec:a", "node_type": "Decision"},
    {"node_id": "dec:b", "node_type": "Decision"},
    {"node_id": "art:a", "node_type": "Artifact"},
    {"node_id": "rel:a", "node_type": "Release"},
]
EDGES = [
    {"edge_type": "DEPENDS_ON", "from_id": "dec:a", "to_id": "req:a"},
    {"edge_type": "EVOLVES_INTO", "from_id": "dec:b", "to_id": "req:a"},
    {"edge_type": "INCLUDES", "from_id": "rel:a", "to_id": "art:a"},
]


def git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)


def commit_tag(repo: Path, tag: str, day: str) -> None:
    git(repo, "add", "-A")
    env = f"{day}T12:00:00+00:00"
    subprocess.run(
        ["git", "-C", str(repo), "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q",
         "--allow-empty", "-m", tag],
        check=True, capture_output=True,
        env={"GIT_COMMITTER_DATE": env, "GIT_AUTHOR_DATE": env, "PATH": "/usr/bin:/bin"},
    )
    git(repo, "tag", tag)


def write_single(repo: Path) -> None:
    p = repo / site_counts.SINGLE_FILE
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps({"nodes": NODES, "edges": EDGES}))


def write_items(repo: Path) -> None:
    (repo / site_counts.SINGLE_FILE).unlink()
    base = repo / site_counts.ITEM_DIR
    for n in NODES:
        d = base / "nodes" / n["node_type"]
        d.mkdir(parents=True, exist_ok=True)
        (d / (n["node_id"].replace(":", "%3A") + ".json")).write_text(json.dumps(n))
    for i, e in enumerate(EDGES):
        d = base / "edges" / f"{i:02x}"
        d.mkdir(parents=True, exist_ok=True)
        (d / f"{i}.json").write_text(json.dumps(e))
    (base / "design.json").write_text("{}")


def test_a_manifest_is_counted_apart() -> None:
    c = site_counts.count_design([n["node_type"] for n in NODES], [e["edge_type"] for e in EDGES])
    assert c["links"] == 2, c
    assert c["manifest_links"] == 1, c
    assert c["kinds_of_link"] == 2, c  # INCLUDES is not a kind of link the page counts
    assert c["records"] == 5 and c["decisions"] == 2 and c["requirements"] == 1, c


def test_one_point_a_week_and_the_latest_kept() -> None:
    pts = [
        {"tag": "v0.1.0", "date": "2026-07-13"},  # Monday, week 29
        {"tag": "v0.2.0", "date": "2026-07-15"},  # week 29: replaces v0.1.0
        {"tag": "v0.3.0", "date": "2026-07-21"},  # week 30
        {"tag": "v0.4.0", "date": "2026-07-22"},  # week 30, the latest
    ]
    assert [p["tag"] for p in site_counts.weekly(pts)] == ["v0.2.0", "v0.4.0"]
    assert site_counts.weekly([]) == []


def test_both_layouts_read_the_same_and_an_empty_tag_is_skipped() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        git(repo, "init", "-q")
        (repo / "README").write_text("no design yet")
        commit_tag(repo, "v0.1.0", "2026-07-06")  # holds no design
        write_single(repo)
        commit_tag(repo, "v0.2.0", "2026-07-13")
        write_items(repo)
        commit_tag(repo, "v0.10.0", "2026-07-20")  # sorts after v0.2.0 by version, not by text
        result = site_counts.counts(repo)
        assert result["skipped"] == ["v0.1.0"], result
        a, b = result["points"]
        assert (a["tag"], b["tag"]) == ("v0.2.0", "v0.10.0"), result
        strip = lambda p: {k: v for k, v in p.items() if k not in ("tag", "date")}  # noqa: E731
        assert strip(a) == strip(b), (a, b)
        assert result["latest"]["tag"] == "v0.10.0"


def main() -> int:
    failed = 0
    for name, fn in sorted(globals().items()):
        if name.startswith("test_") and callable(fn):
            try:
                fn()
                print(f"ok    {name}")
            except Exception as e:  # noqa: BLE001
                failed += 1
                print(f"FAIL  {name}: {e!r}")
    print(f"{failed} failed" if failed else "all passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
