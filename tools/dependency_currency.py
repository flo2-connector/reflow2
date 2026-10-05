#!/usr/bin/env python3
"""Is every dependency at or near its latest release — and is every one that is
not held back on purpose, in writing, with a date to look again?

    tools/dependency_currency.py [--json report.json] [--summary summary.md]

THE REQUIREMENT this checks is
`req:reflow2-keeps-its-dependencies-at-or-near-their-latest-releases`, accepted
by Anthony on 2026-10-02. He answered how it runs: "both before release cuts and
monthly, CI job, include actions too". So it reads three things:

  - every direct dependency in every Cargo.toml (the workspace table and each
    crate's), against the newest release on crates.io;
  - every GitHub Action the workflows use, against its newest release;
  - the declared `rust-version`, against the current stable Rust. The floor is
    stable minus one (Anthony, 2026-10-02), and the `floor` job in
    dependencies.yml builds at it, so the number is checked rather than claimed.

WHAT FAILS, AND WHAT ONLY REPORTS. The requirement's own words draw the line:
  - a patch or minor release inside the declared range is "taken promptly with
    a lock refresh" — REPORTED (lock refresh owed), never failed, because a gate
    that went red on every upstream patch would be ignored within a month;
  - a new major or new 0.x series is "adopted by its own PR", and a dependency
    held back "carries a written reason and a date to look again" — so one that
    is behind a series and has NO hold in `dependency-holds.toml` FAILS, and so
    does a hold whose look-again date has passed. An exception is recorded,
    never silent.

A lookup that fails (network, an action with no releases) is reported as
`unknown` and FAILS the run: a currency check that cannot see is not a pass
(`req:no-silent-fallback`).
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import sys
import urllib.request

# `tomllib` is Python 3.11+, and CI's ubuntu-22.04 (pinned to match release.yml)
# carries 3.10, so the first CI run of this checker failed on its import. `tomli`
# is the package `tomllib` was copied from, same API; ci.yml and dependencies.yml
# install it. No line-reader fallback like reflow2_check.py's: that one reads
# single-line pins, and this reads Cargo.lock and Rust's stable-channel manifest
# too, which only a real parser reads correctly. Without either it stops and says
# what to install, rather than reporting on a partial read.
try:
    import tomllib
except ModuleNotFoundError:
    try:
        import tomli as tomllib
    except ModuleNotFoundError:
        raise SystemExit(
            "dependency_currency.py reads TOML: it needs Python 3.11+ (tomllib) or the "
            "tomli package on older Pythons — python3 -m pip install tomli"
        )
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable

REPO = Path(__file__).resolve().parent.parent
HOLDS = "dependency-holds.toml"
USER_AGENT = "reflow2-dependency-currency (github.com/flo2-connector/reflow2)"

Version = tuple[int, int, int]


# ---- versions -----------------------------------------------------------------


def parse_version(s: str) -> Version | None:
    """`1.2.3` (or `1.2`, `1`) as a tuple; None for a pre-release or junk."""
    m = re.fullmatch(r"v?(\d+)(?:\.(\d+))?(?:\.(\d+))?", s.strip())
    if not m:
        return None
    return (int(m.group(1)), int(m.group(2) or 0), int(m.group(3) or 0))


def caret_upper(req: Version, parts: int) -> Version:
    """The exclusive upper bound of a Cargo caret requirement.

    `1.2` → <2.0.0, `0.25` → <0.26.0, `0.0.3` → <0.0.4. `parts` is how many
    components the requirement spelled, because `0` alone means <1.0.0 while
    `0.0` means <0.1.0.
    """
    major, minor, patch = req
    if major > 0 or parts == 1:
        return (major + 1, 0, 0)
    if minor > 0 or parts == 2:
        return (0, minor + 1, 0)
    return (0, 0, patch + 1)


@dataclass
class Requirement:
    raw: str
    lower: Version | None = None
    upper: Version | None = None
    exact: bool = False

    @classmethod
    def parse(cls, raw: str) -> "Requirement":
        text = raw.strip()
        exact = text.startswith("=")
        bare = text.lstrip("^=").strip()
        if any(c in bare for c in "<>~*, "):
            return cls(raw)  # a range Cargo understands and this does not; reported, not guessed
        lower = parse_version(bare)
        if lower is None:
            return cls(raw)
        parts = len(bare.split("."))
        upper = (lower[0], lower[1], lower[2] + 1) if exact else caret_upper(lower, parts)
        return cls(raw, lower, upper, exact)

    def admits(self, v: Version) -> bool:
        return self.lower is not None and self.lower <= v < self.upper  # type: ignore[operator]


def fmt(v: Version | None) -> str:
    return "?" if v is None else ".".join(map(str, v))


# ---- what the tree declares ---------------------------------------------------


@dataclass
class Crate:
    name: str
    requirement: Requirement
    declared_in: list[str] = field(default_factory=list)


def _dep_tables(doc: dict) -> list[dict]:
    tables = [doc.get(k, {}) for k in ("dependencies", "dev-dependencies", "build-dependencies")]
    for target in doc.get("target", {}).values():
        tables += [target.get(k, {}) for k in ("dependencies", "dev-dependencies", "build-dependencies")]
    return tables


def declared_crates(repo: Path) -> dict[str, Crate]:
    """Every registry dependency named directly, keyed by crate name."""
    root = tomllib.loads((repo / "Cargo.toml").read_text())
    ws = root.get("workspace", {}).get("dependencies", {})
    out: dict[str, Crate] = {}

    def add(name: str, spec, where: str) -> None:
        if isinstance(spec, str):
            version = spec
        elif isinstance(spec, dict):
            if spec.get("workspace"):
                spec = ws.get(name, {})
                version = spec if isinstance(spec, str) else spec.get("version")
                where = "Cargo.toml [workspace.dependencies]"
            elif "path" in spec or "git" in spec:
                return
            else:
                version = spec.get("version")
        else:
            return
        if not version:
            return
        package = spec.get("package", name) if isinstance(spec, dict) else name
        crate = out.setdefault(package, Crate(package, Requirement.parse(version)))
        if where not in crate.declared_in:
            crate.declared_in.append(where)

    for name, spec in ws.items():
        add(name, spec, "Cargo.toml [workspace.dependencies]")
    for manifest in sorted(repo.glob("crates/*/Cargo.toml")):
        rel = str(manifest.relative_to(repo))
        for table in _dep_tables(tomllib.loads(manifest.read_text())):
            for name, spec in table.items():
                add(name, spec, rel)
    return out


def locked_versions(repo: Path) -> dict[str, list[Version]]:
    lock = tomllib.loads((repo / "Cargo.lock").read_text())
    out: dict[str, list[Version]] = {}
    for pkg in lock.get("package", []):
        v = parse_version(pkg["version"].split("+")[0])
        if v is not None:
            out.setdefault(pkg["name"], []).append(v)
    return out


ACTION = re.compile(r"^\s*-?\s*uses:\s*([\w.-]+/[\w.-]+)(?:/[\w./-]+)?@([\w.-]+)", re.M)


def declared_actions(repo: Path) -> dict[str, dict]:
    """Every `uses: owner/repo@ref` in the workflows, with each ref it is used at."""
    out: dict[str, dict] = {}
    for wf in sorted((repo / ".github" / "workflows").glob("*.y*ml")):
        for m in ACTION.finditer(wf.read_text()):
            entry = out.setdefault(m.group(1), {"refs": set(), "files": set()})
            entry["refs"].add(m.group(2))
            entry["files"].add(str(wf.relative_to(repo)))
    return out


def declared_rust_version(repo: Path) -> str | None:
    root = tomllib.loads((repo / "Cargo.toml").read_text())
    return root.get("workspace", {}).get("package", {}).get("rust-version")


def load_holds(repo: Path) -> list[dict]:
    path = repo / HOLDS
    if not path.exists():
        return []
    return tomllib.loads(path.read_text()).get("hold", [])


# ---- what upstream has --------------------------------------------------------


def _get(url: str, token: str | None = None) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    if token:
        req.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(req, timeout=30) as resp:
        return resp.read()


def index_path(name: str) -> str:
    n = name.lower()
    if len(n) <= 2:
        return f"{len(n)}/{n}"
    if len(n) == 3:
        return f"3/{n[0]}/{n}"
    return f"{n[:2]}/{n[2:4]}/{n}"


def crates_io_versions(name: str) -> list[Version]:
    """Every published, unyanked, non-pre-release version — from the sparse index."""
    body = _get(f"https://index.crates.io/{index_path(name)}").decode()
    out = []
    for line in body.splitlines():
        if not line.strip():
            continue
        entry = json.loads(line)
        if entry.get("yanked"):
            continue
        # A pre-release has a `-` BEFORE any build metadata. `toml` publishes
        # `1.1.6+spec-1.1.0`: a release, with a hyphen only in its metadata.
        base = entry["vers"].split("+")[0]
        v = parse_version(base) if "-" not in base else None
        if v is not None:
            out.append(v)
    return out


def action_latest(repo_slug: str) -> Version | None:
    token = os.environ.get("GITHUB_TOKEN")
    try:
        rel = json.loads(_get(f"https://api.github.com/repos/{repo_slug}/releases/latest", token))
        v = parse_version(rel.get("tag_name", ""))
        if v is not None:
            return v
    except Exception:
        pass
    tags = json.loads(_get(f"https://api.github.com/repos/{repo_slug}/tags?per_page=100", token))
    versions = [v for v in (parse_version(t["name"]) for t in tags) if v is not None]
    return max(versions) if versions else None


def stable_rust() -> Version | None:
    doc = tomllib.loads(_get("https://static.rust-lang.org/dist/channel-rust-stable.toml").decode())
    return parse_version(doc["pkg"]["rust"]["version"].split()[0])


@dataclass
class Upstream:
    crate_versions: Callable[[str], list[Version]] = crates_io_versions
    action_latest: Callable[[str], Version | None] = action_latest
    stable_rust: Callable[[], Version | None] = stable_rust


# ---- the verdict --------------------------------------------------------------


def find_hold(holds: list[dict], kind: str, name: str) -> dict | None:
    return next((h for h in holds if h.get("kind") == kind and h.get("name") == name), None)


def judge_hold(hold: dict | None, today: dt.date) -> tuple[str, str]:
    """(state, note) for something that is behind: owed, held, or hold_expired."""
    if hold is None:
        return "owed", "no hold recorded in dependency-holds.toml"
    for need in ("reason", "look_again"):
        if not hold.get(need):
            return "owed", f"the hold has no `{need}`, and a hold without one is silence"
    look = hold["look_again"]
    look = look if isinstance(look, dt.date) else dt.date.fromisoformat(str(look))
    if look < today:
        return "hold_expired", f"held until {look}: {hold['reason']}"
    return "held", f"until {look}: {hold['reason']}"


def check(repo: Path, upstream: Upstream, today: dt.date) -> dict:
    holds = load_holds(repo)
    rows: list[dict] = []

    locked = locked_versions(repo)
    for name, crate in sorted(declared_crates(repo).items()):
        row = {"kind": "crate", "name": name, "declared": crate.requirement.raw,
               "declared_in": crate.declared_in}
        req = crate.requirement
        mine = sorted(v for v in locked.get(name, []) if req.lower is None or req.admits(v))
        row["locked"] = fmt(mine[-1]) if mine else None
        try:
            published = sorted(upstream.crate_versions(name))
        except Exception as e:  # noqa: BLE001 — reported, and it fails the run
            rows.append({**row, "state": "unknown", "note": f"crates.io lookup failed: {e}"})
            continue
        if not published:
            rows.append({**row, "state": "unknown", "note": "crates.io lists no release"})
            continue
        latest = published[-1]
        row["latest"] = fmt(latest)
        if req.lower is None:
            rows.append({**row, "state": "unknown",
                         "note": "a requirement this check does not parse — read it by hand"})
            continue
        in_range = [v for v in published if req.admits(v)]
        if req.admits(latest):
            if mine and mine[-1] < latest:
                rows.append({**row, "state": "lock_refresh", "note": "cargo update takes it"})
            else:
                rows.append({**row, "state": "current"})
            continue
        state, note = judge_hold(find_hold(holds, "crate", name), today)
        series = "a new major" if latest[0] > req.lower[0] else "a new series"
        rows.append({**row, "state": state,
                     "note": f"{series} ({fmt(latest)}) is outside `{req.raw}`"
                             + (f"; newest in range {fmt(in_range[-1])}" if in_range else "")
                             + f" — {note}"})

    for slug, use in sorted(declared_actions(repo).items()):
        refs = sorted(use["refs"])
        row = {"kind": "action", "name": slug, "declared": ", ".join(refs),
               "declared_in": sorted(use["files"])}
        majors = [parse_version(r) for r in refs]
        if any(m is None for m in majors):
            rows.append({**row, "state": "branch_ref",
                         "note": "pinned to a branch, which moves by itself"})
            continue
        try:
            latest = upstream.action_latest(slug)
        except Exception as e:  # noqa: BLE001
            rows.append({**row, "state": "unknown", "note": f"GitHub lookup failed: {e}"})
            continue
        if latest is None:
            rows.append({**row, "state": "unknown", "note": "no release or version tag found"})
            continue
        row["latest"] = fmt(latest)
        behind = [m for m in majors if m is not None and m[0] < latest[0]]
        if not behind:
            rows.append({**row, "state": "current"})
            continue
        state, note = judge_hold(find_hold(holds, "action", slug), today)
        rows.append({**row, "state": state, "note": f"v{latest[0]} is out — {note}"})

    declared = declared_rust_version(repo)
    row = {"kind": "toolchain", "name": "rust-version", "declared": declared,
           "declared_in": ["Cargo.toml [workspace.package]"]}
    floor = parse_version(declared) if declared else None
    try:
        stable = upstream.stable_rust()
    except Exception as e:  # noqa: BLE001
        stable = None
        row["note"] = f"stable lookup failed: {e}"
    if floor is None or stable is None:
        rows.append({**row, "state": "unknown", "note": row.get("note", "no rust-version declared")})
    else:
        row["latest"] = fmt(stable)
        policy = (stable[0], max(stable[1] - 1, 0), 0)
        if floor[:2] >= policy[:2]:
            rows.append({**row, "state": "current",
                         "note": f"the floor is stable minus one ({policy[0]}.{policy[1]}) or newer"})
        else:
            state, note = judge_hold(find_hold(holds, "toolchain", "rust-version"), today)
            rows.append({**row, "state": state,
                         "note": f"stable is {fmt(stable)}, so the floor should be "
                                 f"{policy[0]}.{policy[1]} — {note}"})

    # A hold on something that is no longer behind is reported, never failed:
    # it is a record that outlived its reason, which is how a hold file rots.
    for hold in holds:
        target = next((r for r in rows if r["kind"] == hold.get("kind") and r["name"] == hold.get("name")), None)
        if target is None:
            rows.append({"kind": hold.get("kind", "?"), "name": hold.get("name", "?"),
                         "state": "hold_unneeded", "note": "held, but nothing by that name is declared"})
        elif target["state"] in ("current", "lock_refresh", "branch_ref"):
            target["note"] = (target.get("note", "") + " — and a hold for it is still in "
                              f"{HOLDS}: remove it").lstrip(" —")
            target["state"] = "hold_unneeded"

    failing = [r for r in rows if r["state"] in ("owed", "hold_expired", "unknown")]
    return {"date": today.isoformat(), "rows": rows, "failing": len(failing),
            "counts": {s: sum(r["state"] == s for r in rows) for s in sorted({r["state"] for r in rows})}}


def markdown(report: dict) -> str:
    order = {"owed": 0, "hold_expired": 1, "unknown": 2, "hold_unneeded": 3, "held": 4,
             "lock_refresh": 5, "branch_ref": 6, "current": 7}
    lines = [f"## Dependency currency, {report['date']}", "",
             "`req:reflow2-keeps-its-dependencies-at-or-near-their-latest-releases`. "
             + ", ".join(f"{n} {s}" for s, n in report["counts"].items()), "",
             "| state | kind | name | declared | locked | latest | note |",
             "|---|---|---|---|---|---|---|"]
    for r in sorted(report["rows"], key=lambda r: (order.get(r["state"], 9), r["kind"], r["name"])):
        lines.append(f"| {r['state']} | {r['kind']} | {r['name']} | {r.get('declared') or ''} | "
                     f"{r.get('locked') or ''} | {r.get('latest') or ''} | {r.get('note', '')} |")
    return "\n".join(lines) + "\n"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--json", help="write the dated report here as JSON")
    ap.add_argument("--summary", help="append the report here as Markdown ($GITHUB_STEP_SUMMARY)")
    ap.add_argument("--today", help="the date to judge holds against (default: today, UTC)")
    args = ap.parse_args()
    today = dt.date.fromisoformat(args.today) if args.today else dt.datetime.now(dt.UTC).date()
    report = check(REPO, Upstream(), today)
    md = markdown(report)
    print(md)
    if args.json:
        Path(args.json).write_text(json.dumps(report, indent=2) + "\n")
    if args.summary:
        with open(args.summary, "a") as f:
            f.write(md)
    if report["failing"]:
        print(f"dependency currency: FAILED — {report['failing']} owed, expired or unreadable. "
              f"Bump each in its own PR, or record a hold with a reason and a look-again date "
              f"in {HOLDS}.", file=sys.stderr)
        return 1
    print("dependency currency: OK — everything is current, refreshable in range, or held on the record.",
          file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
