#!/usr/bin/env python3
"""Hermetic tests for tools/dependency_currency.py: no network, a fixture tree and
a fake upstream.

What the check must never do is pass quietly. These pin the verdicts the
`dependencies` workflow acts on:

  - behind a new major or 0.x series with no hold FAILS, and so do a hold past its
    date, a hold missing its reason, and a lookup that could not see;
  - a newer release inside the declared range is REPORTED (a lock refresh owed),
    never failed;
  - a hold on something no longer behind is reported, so the holds file cannot rot
    into a list of excuses nobody reads;
  - the floor is stable minus one, and a floor two releases behind is owed.

Plus the two parsing traps met while writing it: Cargo's caret ranges for 0.x
(`0.25` admits 0.25.9 and not 0.26.0), and a release whose BUILD METADATA has a
hyphen (`toml` 1.1.6+spec-1.1.0), which is not a pre-release. The first live run
read that as one and reported `toml` 1.x as behind 0.9.8.
"""

from __future__ import annotations

import datetime as dt
import importlib.util
import json
import sys
import tempfile
from pathlib import Path

TOOL = Path(__file__).resolve().parent / "dependency_currency.py"
spec = importlib.util.spec_from_file_location("dependency_currency", TOOL)
dc = importlib.util.module_from_spec(spec)
sys.modules["dependency_currency"] = dc
spec.loader.exec_module(dc)

TODAY = dt.date(2026, 10, 2)


def tree(holds: str = "", rust_version: str = "1.98", extra_ws: str = "") -> Path:
    root = Path(tempfile.mkdtemp())
    (root / "Cargo.toml").write_text(f"""
[workspace]
members = ["crates/a"]
[workspace.package]
rust-version = "{rust_version}"
[workspace.dependencies]
current = "1"
inrange = "0.4"
behind = "0.24"
renamed = {{ version = "2", package = "real-name" }}
{extra_ws}
""")
    (root / "crates/a").mkdir(parents=True)
    (root / "crates/a/Cargo.toml").write_text("""
[package]
name = "a"
[dependencies]
current = { workspace = true }
inrange = { workspace = true }
behind = { workspace = true }
renamed = { workspace = true }
local = { path = "../b" }
[dev-dependencies]
devonly = "3"
""")
    lock = []
    for name, v in [("current", "1.2.0"), ("inrange", "0.4.1"), ("behind", "0.24.0"),
                    ("real-name", "2.0.0"), ("devonly", "3.0.0")]:
        lock.append(f'[[package]]\nname = "{name}"\nversion = "{v}"\n')
    (root / "Cargo.lock").write_text("version = 4\n\n" + "\n".join(lock))
    (root / ".github/workflows").mkdir(parents=True)
    (root / ".github/workflows/ci.yml").write_text("""
jobs:
  x:
    steps:
      - uses: actions/checkout@v7
      - uses: actions/upload-artifact@v4
      - uses: dtolnay/rust-toolchain@stable
""")
    if holds:
        (root / "dependency-holds.toml").write_text(holds)
    return root


PUBLISHED = {
    "current": ["1.0.0", "1.2.0"],
    "inrange": ["0.4.1", "0.4.3"],
    "behind": ["0.24.0", "0.25.0"],
    "real-name": ["2.0.0"],
    "devonly": ["3.0.0"],
}
ACTIONS = {"actions/checkout": "7.0.1", "actions/upload-artifact": "7.0.1"}


def upstream(stable: str = "1.99.0", fail: str | None = None) -> "dc.Upstream":
    def crate_versions(name: str):
        if name == fail:
            raise OSError("connection refused")
        return [dc.parse_version(v) for v in PUBLISHED[name]]

    return dc.Upstream(
        crate_versions=crate_versions,
        action_latest=lambda slug: dc.parse_version(ACTIONS[slug]),
        stable_rust=lambda: dc.parse_version(stable),
    )


def by_name(report: dict) -> dict:
    return {r["name"]: r for r in report["rows"]}


HOLD_BEHIND = """
[[hold]]
kind = "crate"
name = "behind"
reason = "waiting on an upstream fix"
look_again = {date}
[[hold]]
kind = "action"
name = "actions/upload-artifact"
reason = "the move is in flight"
look_again = 2026-12-01
"""


def test_a_new_series_with_no_hold_fails():
    r = dc.check(tree(), upstream(), TODAY)
    rows = by_name(r)
    assert rows["behind"]["state"] == "owed", rows["behind"]
    assert rows["actions/upload-artifact"]["state"] == "owed", rows["actions/upload-artifact"]
    assert r["failing"] == 2, r["failing"]


def test_a_newer_release_inside_the_range_is_reported_not_failed():
    rows = by_name(dc.check(tree(HOLD_BEHIND.format(date="2026-12-01")), upstream(), TODAY))
    assert rows["inrange"]["state"] == "lock_refresh", rows["inrange"]
    assert rows["current"]["state"] == "current", rows["current"]


def test_a_dated_hold_with_a_reason_counts():
    r = dc.check(tree(HOLD_BEHIND.format(date="2026-12-01")), upstream(), TODAY)
    rows = by_name(r)
    assert rows["behind"]["state"] == "held", rows["behind"]
    assert "waiting on an upstream fix" in rows["behind"]["note"]
    assert r["failing"] == 0, [x for x in r["rows"] if x["state"] in ("owed", "hold_expired", "unknown")]


def test_a_hold_past_its_date_fails():
    r = dc.check(tree(HOLD_BEHIND.format(date="2026-10-01")), upstream(), TODAY)
    assert by_name(r)["behind"]["state"] == "hold_expired"
    assert r["failing"] == 1


def test_a_hold_without_a_reason_is_silence_and_fails():
    holds = '[[hold]]\nkind = "crate"\nname = "behind"\nlook_again = 2026-12-01\n'
    rows = by_name(dc.check(tree(holds), upstream(), TODAY))
    assert rows["behind"]["state"] == "owed", rows["behind"]
    assert "reason" in rows["behind"]["note"]


def test_a_hold_on_something_current_is_reported_for_removal():
    holds = '[[hold]]\nkind = "crate"\nname = "current"\nreason = "x"\nlook_again = 2026-12-01\n'
    r = dc.check(tree(holds), upstream(), TODAY)
    row = by_name(r)["current"]
    assert row["state"] == "hold_unneeded" and "remove it" in row["note"], row


def test_a_lookup_that_cannot_see_fails():
    r = dc.check(tree(HOLD_BEHIND.format(date="2026-12-01")), upstream(fail="inrange"), TODAY)
    assert by_name(r)["inrange"]["state"] == "unknown"
    assert r["failing"] == 1


def test_workspace_inheritance_renames_paths_and_branch_refs():
    rows = by_name(dc.check(tree(), upstream(), TODAY))
    assert "real-name" in rows and "renamed" not in rows, "a renamed dependency is looked up by its package"
    assert "local" not in rows, "a path dependency is not a registry release"
    assert rows["devonly"]["state"] == "current", "dev-dependencies are read too"
    assert rows["dtolnay/rust-toolchain"]["state"] == "branch_ref"


def test_the_floor_is_stable_minus_one():
    assert by_name(dc.check(tree(rust_version="1.98"), upstream("1.99.0"), TODAY))["rust-version"]["state"] == "current"
    assert by_name(dc.check(tree(rust_version="1.99"), upstream("1.99.0"), TODAY))["rust-version"]["state"] == "current"
    late = by_name(dc.check(tree(rust_version="1.97"), upstream("1.99.0"), TODAY))["rust-version"]
    assert late["state"] == "owed" and "1.98" in late["note"], late


def test_caret_ranges_follow_cargo():
    r = dc.Requirement.parse
    assert r("0.25").admits((0, 25, 9)) and not r("0.25").admits((0, 26, 0))
    assert r("1").admits((1, 99, 0)) and not r("1").admits((2, 0, 0))
    assert r("0").admits((0, 9, 0)) and not r("0").admits((1, 0, 0))
    assert r("3.5").admits((3, 9, 1)) and not r("3.5").admits((3, 4, 9))
    assert r("=1.2.3").admits((1, 2, 3)) and not r("=1.2.3").admits((1, 2, 4))
    assert r(">=1, <3").lower is None, "a range this does not parse is reported, not guessed"


def test_build_metadata_with_a_hyphen_is_not_a_prerelease():
    lines = [{"vers": "0.9.8"}, {"vers": "1.1.6+spec-1.1.0"}, {"vers": "1.2.0-rc.1"},
             {"vers": "1.1.7", "yanked": True}]
    real_get = dc._get
    dc._get = lambda url, token=None: "\n".join(json.dumps(x) for x in lines).encode()
    try:
        got = sorted(dc.crates_io_versions("toml"))
    finally:
        dc._get = real_get
    assert got == [(0, 9, 8), (1, 1, 6)], got


def test_the_index_path_matches_the_sparse_index():
    assert dc.index_path("a") == "1/a"
    assert dc.index_path("ab") == "2/ab"
    assert dc.index_path("abc") == "3/a/abc"
    assert dc.index_path("Serde") == "se/rd/serde"


def main() -> int:
    tests = [
        test_a_new_series_with_no_hold_fails,
        test_a_newer_release_inside_the_range_is_reported_not_failed,
        test_a_dated_hold_with_a_reason_counts,
        test_a_hold_past_its_date_fails,
        test_a_hold_without_a_reason_is_silence_and_fails,
        test_a_hold_on_something_current_is_reported_for_removal,
        test_a_lookup_that_cannot_see_fails,
        test_workspace_inheritance_renames_paths_and_branch_refs,
        test_the_floor_is_stable_minus_one,
        test_caret_ranges_follow_cargo,
        test_build_metadata_with_a_hyphen_is_not_a_prerelease,
        test_the_index_path_matches_the_sparse_index,
    ]
    failed = 0
    for t in tests:
        try:
            t()
            print(f"PASS  {t.__name__}")
        except (AssertionError, KeyError, TypeError, ValueError, FileNotFoundError) as e:
            print(f"FAIL  {t.__name__}: {type(e).__name__}: {e}")
            failed += 1
    print(f"\n{len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
