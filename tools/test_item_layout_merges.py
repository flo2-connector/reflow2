#!/usr/bin/env python3
"""The item layout merges under git's ORDINARY merge — on the real binary, with
real git branches, and no merge driver.

`req:the-saved-design-merges-correctly-under-gits-ordinary-merge` (accepted
2026-09-22): design and code merge in the same act, on GitHub or anywhere, and
disjoint design changes never conflict. Anthony settled the form on 2026-10-03:
one file per node and per edge (`dec:how-the-saved-design-is-laid-out-so-git-merges-it`)
with per-item lineage (`dec:the-designs-lineage-is-kept-per-item`).

Git is the client here, so git runs the test — the lesson `test_merge_driver.py`
records: three home-grown layers once agreed with each other and were all wrong.
Every design write goes through `reflow2-mcp --call` with `--export-to` naming
the layout, exactly as a contributor's would, so each write exports.

The checks, by the planned verifications they make real:

- ver:two-branches-with-disjoint-design-changes-merge-with-plain-git — two PRs,
  each with a design record AND a CHANGELOG fragment, merge in either order
  (merge commit and squash) with no conflict and one identical tree; the merged
  design equals a single sequential replay (`--diff` says nothing differs); a
  control editing the same property of one node conflicts on exactly that file.
- ver:item-lineage-survives-squash-merge-and-a-mid-merge-export — a branch that
  exports three times, merges main mid-way and exports during the merge, then
  squash-merges: every changed item names its hash at the merge-base, every
  other item keeps main's bytes, the gate's per-item lineage check is clean, and
  the item chains link one hop per commit along main.
- the fold of a live store's record after a squash-merge: a long-lived branch's
  record merges into main with plain git, exactly once.

Skips cleanly when the binary is absent; CI's `full` job builds it first.
"""

from __future__ import annotations

import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import design_io  # noqa: E402
import reflow2_check  # noqa: E402
from reflow2_bin import default_bin  # noqa: E402

BIN = default_bin()
LAYOUT = "docs/design/demo"


def git(cwd, *args, check=True):
    r = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True, timeout=120)
    if check and r.returncode != 0:
        raise AssertionError(f"git {' '.join(args)} failed: {r.stdout}{r.stderr}")
    return r


class Store:
    """One design store, driven through `--call` the way a script drives it."""

    def __init__(self, path: pathlib.Path):
        self.path = path

    def call(self, tool: str, args: dict, export_to: str | None = None) -> dict:
        cmd = [BIN, "--graph-path", str(self.path), "--call", tool, "--args", "-"]
        cmd += ["--export-to", export_to] if export_to else ["--no-export"]
        r = subprocess.run(cmd, input=json.dumps(args), capture_output=True, text=True,
                           timeout=120)
        if r.returncode != 0:
            raise AssertionError(f"{tool} exited {r.returncode}: {r.stderr}{r.stdout}")
        return json.loads(r.stdout) if r.stdout.strip() else {}

    def import_from(self, source: str) -> None:
        r = subprocess.run([BIN, "--graph-path", str(self.path), "--import", source],
                           capture_output=True, text=True, timeout=120)
        if r.returncode != 0:
            raise AssertionError(f"import failed: {r.stderr}")


def node(store: Store, node_type: str, node_id: str, props: dict, export_to=None):
    store.call("create_node", {"node_type": node_type, "id": node_id, "props": props}, export_to)


def edge(store: Store, edge_type, from_type, from_id, to_type, to_id, export_to=None):
    store.call("create_edge", {"edge_type": edge_type, "from_type": from_type, "from_id": from_id,
                               "to_type": to_type, "to_id": to_id}, export_to)


def seed(store: Store, export_to: str) -> None:
    node(store, "Project", "proj:demo", {"name": "Demo"}, export_to)
    node(store, "Requirement", "req:base", {"name": "Base", "statement": "it works"}, export_to)
    for cap in ("cap:x", "cap:y"):
        node(store, "Capability", cap, {"name": cap, "description": f"{cap} as designed"},
             export_to)
        edge(store, "SATISFIES", "Capability", cap, "Requirement", "req:base", export_to)


# Each PR's design writes, as (fn, args) so a sequential replay applies the same.
def writes_a():
    return [
        (node, ("Requirement", "req:from-a", {"name": "From A", "statement": "A needs it"})),
        (node, ("Capability", "cap:x", {"name": "cap:x", "description": "cap:x as A changed it"})),
        (edge, ("SATISFIES", "Capability", "cap:x", "Requirement", "req:from-a")),
    ]


def writes_b():
    return [
        (node, ("Decision", "dec:from-b", {"name": "From B", "decision": "B chose this",
                                            "status": "proposed"})),
        (node, ("Capability", "cap:y", {"name": "cap:y", "description": "cap:y as B changed it"})),
    ]


def apply(store: Store, writes, export_to=None):
    for fn, args in writes:
        fn(store, *args, export_to=export_to)


@unittest.skipUnless(os.path.exists(BIN), "reflow2-mcp binary not found (cargo build -p reflow2-mcp)")
class ItemLayoutMerges(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory(prefix="item-layout-merges-")
        self.tmp = pathlib.Path(self._tmp.name)
        self.repo = self.tmp / "repo"
        self.repo.mkdir()
        for args in (("init", "-q", "-b", "main"), ("config", "user.email", "t@t"),
                     ("config", "user.name", "t"), ("config", "commit.gpgsign", "false")):
            git(self.repo, *args)
        # NO merge driver: git's ordinary merge is the whole point.
        self.assertEqual(git(self.repo, "config", "--get", "merge.reflow2.driver",
                             check=False).stdout.strip(), "")
        self.layout = str(self.repo / LAYOUT) + "/"
        main = Store(self.tmp / "main-store")
        seed(main, self.layout)
        (self.repo / "changelog.d").mkdir()
        (self.repo / "changelog.d" / "README.md").write_text("fragments\n")
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-qm", "seed")
        self.main_copy = self.tmp / "main-layout"
        shutil.copytree(self.repo / LAYOUT, self.main_copy)

    def tearDown(self):
        self._tmp.cleanup()

    def pr(self, branch: str, writes, fragment: str | None, exports: int = 1, base="main"):
        """A pull request: a branch whose record is built in its own store
        seeded from main's layout, every write exporting, plus a fragment."""
        git(self.repo, "checkout", "-q", "-b", branch, base)
        store = Store(self.tmp / f"{branch}-store")
        store.import_from(str(self.repo / LAYOUT))
        for _ in range(exports):
            apply(store, writes, export_to=self.layout)
        if fragment:
            (self.repo / "changelog.d" / f"{branch}.md").write_text(
                f"### Fixed\n\n- **{fragment}.**\n")
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-qm", branch)
        git(self.repo, "checkout", "-q", "main")
        return store

    def unmerged(self):
        out = git(self.repo, "diff", "--name-only", "--diff-filter=U").stdout
        return [line for line in out.splitlines() if line]

    def land(self, order, squash: bool):
        git(self.repo, "checkout", "-q", "-B", "land", "main")
        conflicts = []
        for b in order:
            if squash:
                r = git(self.repo, "merge", "--squash", "-q", b, check=False)
                conflicts += self.unmerged()
                if r.returncode == 0:
                    git(self.repo, "commit", "-qm", f"squash {b}")
            else:
                r = git(self.repo, "merge", "--no-edit", "-q", b, check=False)
                conflicts += self.unmerged()
            if r.returncode != 0:
                conflicts.append(f"exit {r.returncode}: {r.stdout}{r.stderr}")
                git(self.repo, "merge", "--abort", check=False)
                git(self.repo, "reset", "-q", "--hard", check=False)
        tree = git(self.repo, "rev-parse", "HEAD^{tree}").stdout.strip()
        return conflicts, tree

    def diff(self, a: str, b: str) -> dict:
        r = subprocess.run([BIN, "--diff", a, b], capture_output=True, text=True, timeout=120)
        self.assertEqual(r.returncode, 0, r.stderr)
        return json.loads(r.stdout)

    def assert_same_design(self, a: str, b: str):
        d = self.diff(a, b)
        self.assertIs(d.get("summary", {}).get("identical"), True,
                      f"compare_designs reads a difference: {json.dumps(d.get('summary'))}")
        self.assertEqual(design_io.load_design(a)["content_hash"],
                         design_io.load_design(b)["content_hash"])

    # ---- ver:two-branches-with-disjoint-design-changes-merge-with-plain-git ----

    def test_two_prs_with_records_and_fragments_merge_in_either_order_with_no_conflict(self):
        self.pr("pr-a", writes_a(), "A's entry", exports=2)
        self.pr("pr-b", writes_b(), "B's entry", exports=3)
        trees = set()
        for squash in (False, True):
            for order in (("pr-a", "pr-b"), ("pr-b", "pr-a")):
                conflicts, tree = self.land(order, squash)
                self.assertEqual(conflicts, [], f"{'squash' if squash else 'merge'} {order}")
                trees.add(tree)
                fragments = sorted(p.name for p in (self.repo / "changelog.d").iterdir())
                self.assertEqual(fragments, ["README.md", "pr-a.md", "pr-b.md"])
                git(self.repo, "checkout", "-q", "main")
        self.assertEqual(len(trees), 1, "every order lands the identical tree")

        # The merged design equals a single sequential replay of both records.
        self.land(("pr-a", "pr-b"), squash=True)
        replay = Store(self.tmp / "replay-store")
        replay.import_from(str(self.main_copy))
        apply(replay, writes_a())
        apply(replay, writes_b())
        replay_dir = str(self.tmp / "replay") + "/"
        replay.call("export_graph", {"path": replay_dir, "overwrite": True})
        self.assert_same_design(str(self.repo / LAYOUT), replay_dir)
        merged = Store(self.tmp / "merged-store")
        merged.import_from(str(self.repo / LAYOUT))  # and the merge imports cleanly
        git(self.repo, "checkout", "-q", "main")

    def test_the_same_property_changed_on_both_sides_conflicts_on_that_one_item(self):
        one = [(node, ("Capability", "cap:x", {"name": "cap:x", "description": "one"}))]
        two = [(node, ("Capability", "cap:x", {"name": "cap:x", "description": "two"}))]
        self.pr("left", one, None)
        self.pr("right", two, None)
        git(self.repo, "checkout", "-q", "-B", "land", "main")
        git(self.repo, "merge", "--no-edit", "-q", "left")
        r = git(self.repo, "merge", "--no-edit", "-q", "right", check=False)
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual(self.unmerged(), [f"{LAYOUT}/{design_io.node_rel_path('Capability', 'cap:x')}"],
                         "a real conflict shows up as exactly the one item file")
        git(self.repo, "merge", "--abort")
        git(self.repo, "checkout", "-q", "main")

    def test_the_single_file_export_conflicts_where_the_layout_does_not(self):
        """The control the planned check asks for: it must fail on the single
        file before it is trusted on the layout."""
        git(self.repo, "checkout", "-q", "-b", "single", "main")
        store = Store(self.tmp / "single-store")
        store.import_from(str(self.repo / LAYOUT))
        single = str(self.repo / "docs" / "design" / "demo.json")
        store.call("export_graph", {"path": single, "overwrite": True})
        shutil.rmtree(self.repo / LAYOUT)
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-qm", "single file")
        for b, writes in (("s-a", writes_a()), ("s-b", writes_b())):
            git(self.repo, "checkout", "-q", "-b", b, "single")
            s = Store(self.tmp / f"{b}-store")
            s.import_from(single)
            apply(s, writes, export_to=single)
            git(self.repo, "add", "-A")
            git(self.repo, "commit", "-qm", b)
        git(self.repo, "checkout", "-q", "-B", "land", "single")
        git(self.repo, "merge", "--no-edit", "-q", "s-a")
        r = git(self.repo, "merge", "--no-edit", "-q", "s-b", check=False)
        self.assertNotEqual(r.returncode, 0, "the single file conflicts on disjoint changes")
        self.assertEqual(self.unmerged(), ["docs/design/demo.json"])
        git(self.repo, "merge", "--abort")
        git(self.repo, "checkout", "-q", "main")

    # ---- ver:item-lineage-survives-squash-merge-and-a-mid-merge-export -------

    def test_item_lineage_stays_one_hop_through_a_mid_merge_export_and_a_squash(self):
        main_before = git(self.repo, "rev-parse", "main").stdout.strip()
        # The branch: three exports, each committed.
        git(self.repo, "checkout", "-q", "-b", "long", "main")
        long_store = Store(self.tmp / "long-store")
        long_store.import_from(str(self.repo / LAYOUT))
        for i in range(3):
            node(long_store, "Capability", "cap:x",
                 {"name": "cap:x", "description": f"cap:x, export {i}"}, self.layout)
            node(long_store, "Requirement", f"req:long-{i}",
                 {"name": f"Long {i}", "statement": f"round {i}"}, self.layout)
            git(self.repo, "add", "-A")
            git(self.repo, "commit", "-qm", f"long export {i}")
        git(self.repo, "checkout", "-q", "main")
        # Main moves under it: another PR squash-merges.
        self.pr("moved", writes_b(), "main moved")
        git(self.repo, "merge", "--squash", "-q", "moved")
        git(self.repo, "commit", "-qm", "squash moved")
        main_mid = git(self.repo, "rev-parse", "main").stdout.strip()
        # Merge main into the branch, and export DURING the merge, before it is
        # committed: the store takes main's changes in, then writes once more.
        git(self.repo, "checkout", "-q", "long")
        r = git(self.repo, "merge", "--no-commit", "--no-ff", "main", check=False)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        long_store.import_from(str(self.repo / LAYOUT))
        node(long_store, "Requirement", "req:during-merge",
             {"name": "During", "statement": "written mid-merge"}, self.layout)
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-qm", "merge main into long")
        # The gate on the branch: per-item lineage clean.
        rng = reflow2_check.change_range(str(self.repo / LAYOUT))
        self.assertIsNotNone(rng)
        self.assertEqual(reflow2_check.check_item_lineage(str(self.repo / LAYOUT), rng), [])
        # Squash-merge it.
        git(self.repo, "checkout", "-q", "main")
        git(self.repo, "merge", "--squash", "-q", "long")
        git(self.repo, "commit", "-qm", "squash long")

        changed = [p for p in git(self.repo, "diff", "--name-only", main_mid, "HEAD", "--",
                                  LAYOUT).stdout.splitlines() if p.endswith(".json")]
        rels = [p[len(LAYOUT) + 1:] for p in changed
                if p[len(LAYOUT) + 1:].split("/")[0] in ("nodes", "edges")]
        self.assertTrue(rels, "the branch changed items")
        before = design_io.items_at(str(self.repo), main_mid, LAYOUT, rels)
        for rel in rels:
            with open(self.repo / LAYOUT / rel, "rb") as fh:
                now = design_io.parse_item(rel, fh.read())
            expected = before[rel].stated if rel in before else None
            self.assertEqual(now.prev, expected,
                             f"{rel}: one hop from its version at the merge-base")
        # Every item the branch did not change keeps main's bytes.
        all_now = {p for p in git(self.repo, "ls-files", LAYOUT).stdout.splitlines()}
        for path in sorted(all_now - set(changed)):
            self.assertEqual(
                git(self.repo, "show", f"{main_mid}:{path}").stdout,
                (self.repo / path).read_text(),
                f"{path} was untouched and is main's file byte for byte")
        # The gate, on the squash commit (the trunk): clean.
        rng = reflow2_check.change_range(str(self.repo / LAYOUT))
        self.assertEqual(rng.mode, "trunk")
        self.assertEqual(reflow2_check.check_item_lineage(str(self.repo / LAYOUT), rng), [])
        # The chains link one hop per commit along main, from the seed on.
        commits = git(self.repo, "rev-list", "--first-parent", "--reverse",
                      f"{main_before}..HEAD").stdout.split()
        parent = main_before
        for c in commits:
            paths = [p[len(LAYOUT) + 1:] for p in git(
                self.repo, "diff", "--name-only", parent, c, "--", LAYOUT).stdout.splitlines()
                if p.endswith(".json") and p[len(LAYOUT) + 1:].split("/")[0] in ("nodes", "edges")]
            then = design_io.items_at(str(self.repo), parent, LAYOUT, paths)
            now = design_io.items_at(str(self.repo), c, LAYOUT, paths)
            for rel, item in now.items():
                self.assertEqual(item.prev, then[rel].stated if rel in then else None,
                                 f"{c[:7]} {rel}: the chain skips a hop")
            parent = c

    def test_a_hand_edited_prev_is_named_by_the_lineage_check(self):
        git(self.repo, "checkout", "-q", "-b", "forged", "main")
        store = Store(self.tmp / "forged-store")
        store.import_from(str(self.repo / LAYOUT))
        node(store, "Capability", "cap:x", {"name": "cap:x", "description": "edited"}, self.layout)
        rel = design_io.node_rel_path("Capability", "cap:x")
        path = self.repo / LAYOUT / rel
        data = json.loads(path.read_text())
        data["prev_item_hash"] = "sha256:" + "0" * 64
        path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
        rng = reflow2_check.change_range(str(self.repo / LAYOUT))
        found = reflow2_check.check_item_lineage(str(self.repo / LAYOUT), rng)
        self.assertEqual(len(found), 1)
        self.assertIn(rel, found[0])
        git(self.repo, "checkout", "-q", "-f", "main")

    # ---- the fold of a live store's record after a squash-merge --------------

    def test_a_live_stores_record_folds_into_main_after_a_squash_merge_exactly_once(self):
        # The live store works on a long-lived branch (the main checkout's).
        git(self.repo, "checkout", "-q", "-b", "live", "main")
        live = Store(self.tmp / "live-store")
        live.import_from(str(self.repo / LAYOUT))
        live_writes = [
            (node, ("Decision", "dec:live", {"name": "Live", "decision": "settled live",
                                              "status": "proposed"})),
            (node, ("Capability", "cap:x", {"name": "cap:x", "description": "cap:x, live"})),
        ]
        apply(live, live_writes, export_to=self.layout)
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-qm", "live store's record")
        git(self.repo, "checkout", "-q", "main")
        # A PR squash-merges into main meanwhile.
        self.pr("pr-p", writes_b(), "P")
        git(self.repo, "merge", "--squash", "-q", "pr-p")
        git(self.repo, "commit", "-qm", "squash pr-p")
        # The fold: a branch from the live record merges main with plain git.
        git(self.repo, "checkout", "-q", "-b", "fold", "live")
        r = git(self.repo, "merge", "--no-edit", "-q", "main", check=False)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertEqual(self.unmerged(), [])
        git(self.repo, "checkout", "-q", "main")
        git(self.repo, "merge", "--squash", "-q", "fold")
        git(self.repo, "commit", "-qm", "squash fold")
        replay = Store(self.tmp / "fold-replay")
        replay.import_from(str(self.main_copy))
        apply(replay, live_writes)
        apply(replay, writes_b())
        replay_dir = str(self.tmp / "fold-replay-out") + "/"
        replay.call("export_graph", {"path": replay_dir, "overwrite": True})
        self.assert_same_design(str(self.repo / LAYOUT), replay_dir)
        # Exactly once: folding the same record again changes nothing.
        head = git(self.repo, "rev-parse", "HEAD^{tree}").stdout.strip()
        git(self.repo, "merge", "--squash", "-q", "fold", check=False)
        self.assertEqual(git(self.repo, "diff", "--cached", "--name-only").stdout.strip(), "")
        self.assertEqual(git(self.repo, "rev-parse", "HEAD^{tree}").stdout.strip(), head)


if __name__ == "__main__":
    unittest.main(verbosity=2)
