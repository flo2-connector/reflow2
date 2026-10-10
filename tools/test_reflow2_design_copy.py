#!/usr/bin/env python3
"""Tests for the copy of reflow2's hosted design, and its one-PR guard.

    python3 tools/test_reflow2_design_copy.py                              # the guard (hermetic)
    python3 tools/test_reflow2_design_copy.py --bin target/debug/reflow2-mcp   # and the copy, on the real binary

What must never happen, and is pinned here:

  - a PR's copy carries another branch's in-flight work. The guard refuses while
    another branch holds the design, and while writes nobody holds are in
    flight, unless the person says --adopt;
  - a hold claim reaches main. It is the lease, not design, and a copy that
    carried it would leave every later PR looking at a holder who merged long ago;
  - a merged holder blocks the next PR. Nothing in flight means the hold is stale;
  - the copy is taken against a main the branch does not contain;
  - the round trip quietly changes the design. The copy compares what it wrote
    with what flo2.io holds.

The real-binary half builds a small design with the `--call` door, commits it
as main in a throwaway repository, and copies a later state of it in through
`--from`, exactly as `copy` would from flo2.io.
"""

from __future__ import annotations

import atexit
import contextlib
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import design_io  # noqa: E402
import reflow2_design_copy as rdc  # noqa: E402


def node(t: str, i: str, **props) -> dict:
    return {"node_type": t, "node_id": i, "properties": props}


def edge(t: str, a: str, b: str, **props) -> dict:
    return {"edge_type": t, "from_id": a, "to_id": b, "properties": props}


def hold(who: str, branch: str) -> dict:
    return edge("CLAIMS", who, rdc.HOLD_SEED, depth=0, note=f"{rdc.HOLD_PREFIX}{branch}")


MAIN = {
    "graph_id": "reflow2",
    "nodes": [node("Project", "proj:reflow2", name="Reflow 2.0"),
              node("Requirement", "req:a", name="A", statement="First."),
              node("Requirement", "req:b", name="B", statement="Second.")],
    "edges": [edge("DECOMPOSES", "req:a", "req:b")],
}


def live_with(*, holds: tuple = (), extra_nodes: tuple = (), extra_edges: tuple = ()) -> dict:
    doc = json.loads(json.dumps(MAIN))
    doc["nodes"] += list(extra_nodes)
    doc["edges"] += list(holds) + list(extra_edges)
    return doc


def refused(fn) -> str:
    try:
        fn()
    except rdc.Refused as e:
        return str(e)
    raise AssertionError("expected a refusal, got none")


# ---- the guard (hermetic) ---------------------------------------------------


def test_a_hold_is_a_claim_on_the_project_and_names_its_branch():
    doc = live_with(holds=(hold("who:a", "feat/a"),
                           edge("CLAIMS", "who:b", "cap:x", note="design PR: feat/b"),
                           edge("CLAIMS", "who:c", rdc.HOLD_SEED, note="something else")))
    got = rdc.holds(doc)
    assert [(h["contributor"], h["branch"]) for h in got] == [("who:a", "feat/a"), ("who:c", None)], got


def test_a_copy_never_carries_a_hold_and_keeps_every_other_claim():
    region = edge("CLAIMS", "who:b", "req:a", depth=1, note="fixing A")
    doc = live_with(holds=(hold("who:a", "feat/a"),), extra_edges=(region,))
    bare = rdc.without_holds(doc)
    assert region in bare["edges"] and not rdc.holds(bare), bare["edges"]
    assert bare["content_hash"] == design_io.design_hash(bare)


def test_in_flight_is_every_item_main_lacks_and_a_hold_is_not_one():
    live = live_with(holds=(hold("who:a", "feat/a"),),
                     extra_nodes=(node("ChangeEvent", "chg:c", name="C"),),
                     extra_edges=(edge("ANSWERS", "chg:c", "req:a"),))
    live["nodes"] = [n for n in live["nodes"] if n["node_id"] != "req:b"]
    live["nodes"][1]["properties"]["statement"] = "First, reworded."
    flight = rdc.in_flight(rdc.without_holds(live), MAIN)
    assert flight["nodes"] == {"added": [("ChangeEvent", "chg:c")],
                               "removed": [("Requirement", "req:b")],
                               "changed": [("Requirement", "req:a")]}, flight["nodes"]
    assert flight["edges"]["added"] == [("ANSWERS", "chg:c", "req:a")], flight["edges"]
    assert rdc.flight_size(rdc.in_flight(rdc.without_holds(live_with(
        holds=(hold("who:a", "feat/a"),))), MAIN)) == 0


def test_nothing_in_flight_means_a_merged_holder_blocks_nobody():
    flight = rdc.in_flight(MAIN, MAIN)
    said = rdc.judge("hold", "feat/me", [{"contributor": "who:a", "branch": "feat/old",
                                          "claimed_at": None, "note": "design PR: feat/old"}],
                     flight, adopt=False)
    text = " ".join(said)
    assert "stale" in text and "design PR: feat/me" in text, said
    rdc.judge("copy", "feat/me", [], flight, adopt=False)


def test_another_branchs_hold_refuses_both_verbs_until_adopted():
    live = live_with(extra_nodes=(node("Requirement", "req:c", name="C", statement="Third."),))
    flight = rdc.in_flight(live, MAIN)
    holders = rdc.holds(live_with(holds=(hold("who:a", "feat/other"),)))
    for verb in ("hold", "copy"):
        why = refused(lambda: rdc.judge(verb, "feat/me", holders, flight, adopt=False))
        assert "feat/other" in why and "wait for that PR to merge" in why, why
    assert "ADOPTING" in rdc.judge("copy", "feat/me", holders, flight, adopt=True)[0]


def test_writes_nobody_holds_refuse_until_adopted():
    live = live_with(extra_nodes=(node("Requirement", "req:c", name="C", statement="Third."),))
    flight = rdc.in_flight(live, MAIN)
    for verb in ("hold", "copy"):
        why = refused(lambda: rdc.judge(verb, "feat/me", [], flight, adopt=False))
        assert "NOBODY" in why and "--adopt" in why, why
    assert "ADOPTING" in rdc.judge("hold", "feat/me", [], flight, adopt=True)[0]


def test_the_holder_may_resume_and_copy():
    live = live_with(holds=(hold("who:a", "feat/me"),),
                     extra_nodes=(node("Requirement", "req:c", name="C", statement="Third."),))
    flight = rdc.in_flight(rdc.without_holds(live), MAIN)
    for verb in ("hold", "copy"):
        said = rdc.judge(verb, "feat/me", rdc.holds(live), flight, adopt=False)
        assert "holds the design" in said[0], said


def test_the_work_in_flight_is_named_change_events_first():
    live = live_with(extra_nodes=(node("Requirement", "req:c", name="Third"),
                                  node("ChangeEvent", "chg:z", name="The change")))
    lines = rdc.describe(rdc.in_flight(live, MAIN), live, MAIN)
    assert lines[0] == "  nodes added: 2" and "chg:z" in lines[1] and "req:c" in lines[2], lines


# ---- the copy, on the real binary -------------------------------------------


def sh(cwd: str, *cmd: str) -> str:
    r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    assert r.returncode == 0, f"{' '.join(cmd)}: {r.stderr[-800:]}"
    return r.stdout


class Fixture:
    """A throwaway repository whose main commits a small design, and a store
    that has moved on from it, as flo2.io would have."""

    def __init__(self, binary: str):
        self.bin = binary
        self.tmp = tempfile.mkdtemp(prefix="reflow2-design-copy-test-")
        atexit.register(shutil.rmtree, self.tmp, True)
        self.repo = os.path.join(self.tmp, "repo")
        self.store = os.path.join(self.tmp, "store", "g")
        os.makedirs(self.repo)
        sh(self.repo, "git", "init", "-q", "-b", "main")
        sh(self.repo, "git", "config", "user.email", "t@t")
        sh(self.repo, "git", "config", "user.name", "t")
        sh(self.repo, "git", "config", "commit.gpgsign", "false")
        self.call("add_project", id="proj:reflow2", name="Fixture")
        self.call("add_requirement", id="req:one", name="One", statement="The first.")
        self.call("add_contributor", id="who:t", name="T")
        sh(self.repo, self.bin, "--graph-path", self.store, "--call", "export_graph", "--args",
           json.dumps({"path": os.path.join(self.repo, design_io.ITEMS_REL) + "/", "overwrite": True}))
        sh(self.repo, "git", "add", "-A")
        sh(self.repo, "git", "commit", "-q", "-m", "main")

    def call(self, tool: str, **args):
        sh(self.tmp, self.bin, "--graph-path", self.store, "--no-export", "--call", tool,
           "--args", json.dumps(args))

    def live(self) -> str:
        path = os.path.join(self.tmp, f"live-{len(os.listdir(self.tmp))}.json")
        with open(path, "w") as fh:
            fh.write(sh(self.tmp, self.bin, "--graph-path", self.store, "--export"))
        return path

    def copy(self, branch: str, source: str, *extra: str) -> tuple[int, str]:
        sh(self.repo, "git", "switch", "-q", "-C", branch)
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            rc = rdc.main(["copy", "--from", source, "--root", self.repo, "--base", "main",
                           "--bin", self.bin, *extra])
        return rc, out.getvalue() + err.getvalue()

    def layout(self) -> dict:
        return design_io.load_design(os.path.join(self.repo, design_io.ITEMS_REL))


def test_the_holders_copy_is_the_live_design_without_its_hold(binary: str):
    fx = Fixture(binary)
    fx.call("add_requirement", id="req:two", name="Two", statement="The second.")
    fx.call("claim_region", contributor_id="who:t", seed_id=rdc.HOLD_SEED, depth=0,
            note=f"{rdc.HOLD_PREFIX}feat/x")
    live = design_io.load_design(fx.live())
    rc, said = fx.copy("feat/x", fx.live())
    assert rc == 0, said
    got = fx.layout()
    ids = {n["node_id"] for n in got["nodes"]}
    assert "req:two" in ids, ids
    assert not rdc.holds(got), "the hold claim reached the copy"
    assert got["content_hash"] == rdc.without_holds(live)["content_hash"], said
    assert "Requirement   req:two" in said, said


def test_a_copy_refuses_another_branchs_hold_and_writes_nothing(binary: str):
    fx = Fixture(binary)
    fx.call("add_requirement", id="req:two", name="Two", statement="The second.")
    fx.call("claim_region", contributor_id="who:t", seed_id=rdc.HOLD_SEED, depth=0,
            note=f"{rdc.HOLD_PREFIX}feat/x")
    rc, said = fx.copy("feat/y", fx.live())
    assert rc == 1 and "held by feat/x" in said, said
    assert sh(fx.repo, "git", "status", "--porcelain") == "", "a refused copy wrote files"


def test_a_copy_refuses_until_the_branch_contains_main(binary: str):
    fx = Fixture(binary)
    fx.call("claim_region", contributor_id="who:t", seed_id=rdc.HOLD_SEED, depth=0,
            note=f"{rdc.HOLD_PREFIX}feat/x")
    source = fx.live()
    sh(fx.repo, "git", "switch", "-q", "-c", "feat/x")
    sh(fx.repo, "git", "switch", "-q", "main")
    sh(fx.repo, "git", "commit", "-q", "--allow-empty", "-m", "main moves on")
    sh(fx.repo, "git", "switch", "-q", "feat/x")
    out = io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(out):
        rc = rdc.main(["copy", "--from", source, "--root", fx.repo, "--base", "main",
                       "--bin", binary])
    assert rc == 1 and "git merge main" in out.getvalue(), out.getvalue()


def main() -> int:
    binary = None
    if "--bin" in sys.argv:
        binary = os.path.abspath(sys.argv[sys.argv.index("--bin") + 1])
        if not os.access(binary, os.X_OK):
            print(f"no binary at {binary}")
            return 1
    tests = [
        test_a_hold_is_a_claim_on_the_project_and_names_its_branch,
        test_a_copy_never_carries_a_hold_and_keeps_every_other_claim,
        test_in_flight_is_every_item_main_lacks_and_a_hold_is_not_one,
        test_nothing_in_flight_means_a_merged_holder_blocks_nobody,
        test_another_branchs_hold_refuses_both_verbs_until_adopted,
        test_writes_nobody_holds_refuse_until_adopted,
        test_the_holder_may_resume_and_copy,
        test_the_work_in_flight_is_named_change_events_first,
    ]
    real = [
        test_the_holders_copy_is_the_live_design_without_its_hold,
        test_a_copy_refuses_another_branchs_hold_and_writes_nothing,
        test_a_copy_refuses_until_the_branch_contains_main,
    ]
    failed = 0
    for t in tests + (real if binary else []):
        try:
            t(binary) if t in real else t()
            print(f"PASS  {t.__name__}")
        except AssertionError as e:
            print(f"FAIL  {t.__name__}: {e}")
            failed += 1
    ran = len(tests) + (len(real) if binary else 0)
    print(f"\n{ran - failed}/{ran} passed"
          + ("" if binary else f"; the {len(real)} real-binary tests did NOT run (pass --bin)"))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
