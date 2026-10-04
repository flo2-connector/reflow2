#!/usr/bin/env python3
"""Convert reflow2's committed design from the single file to the item layout —
the one-PR conversion of main (decision 6 of
dec:item-13-checksums-move-to-change-edges-and-main-converts-in-one-pr).

    python3 tools/convert_design_to_items.py [--source docs/design/reflow2.json]
                                             [--to docs/design/reflow2/] [--bin <reflow2-mcp>]
                                             [--change-id chg:...] [--date YYYY-MM-DD]

What it does, in order, each step checked:

1. Imports the single file into a FRESH store (the CLI adopts the design's own
   identity), and exports it as the item layout into a scratch directory: the
   reassembled whole-design hash must EQUAL the single file's content hash, so
   the layout alone loses nothing (measured on main 2026-10-03: sha256:3e1f2f…).
2. Records ONE baseline ChangeEvent that puts every registered artifact's
   current checksum on a CHANGED edge (`checksum_after`, its basis, and an
   `accepted_seq` above any acceptance already there), so from the first commit
   in the new layout every accepted checksum lives on a change, not on a node
   two PRs would both rewrite.
3. Exports the layout to `--to` (beside the single file, so `design.json`
   records `migrated_from`), and removes the single file.

Run it from a clean checkout of main, on a branch, when no other
design-touching PR is open; then commit with the path changes the PR carries
(ci.yml's and AGENTS.md's gate lines). Re-run it from scratch if main moved.
Standard library only.
"""

from __future__ import annotations

import argparse
import datetime
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import design_io  # noqa: E402
from reflow2_bin import default_bin  # noqa: E402


def die(msg: str) -> None:
    print(f"convert_design_to_items: {msg}", file=sys.stderr)
    sys.exit(2)


def call(binary: str, graph: str, tool: str, args: dict) -> dict:
    r = subprocess.run([binary, "--graph-path", graph, "--tree-root", design_io.REPO, "--call",
                        tool, "--args", "-", "--no-export"], input=json.dumps(args),
                       capture_output=True, text=True)
    if r.returncode != 0:
        die(f"{tool} exited {r.returncode}: {r.stderr.strip()[:2000]}")
    return json.loads(r.stdout) if r.stdout.strip() else {}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--source", default=os.path.join(design_io.REPO, design_io.FILE_REL))
    ap.add_argument("--to", default=os.path.join(design_io.REPO, design_io.ITEMS_REL) + "/")
    ap.add_argument("--bin", default=default_bin())
    ap.add_argument("--date", default=datetime.date.today().isoformat())
    ap.add_argument("--change-id", default=None)
    ap.add_argument("--keep-source", action="store_true",
                    help="leave the single file in place (default: remove it)")
    ap.add_argument("--accept-changed-since", metavar="REV", default=None,
                    help="also accept, against the baseline change, every registered file that "
                         "differs between REV and the working tree (the conversion PR's own edits)")
    opts = ap.parse_args()
    if not opts.to.endswith("/"):
        opts.to += "/"
    if not os.path.isfile(opts.source):
        die(f"no single-file export at {opts.source}")
    if os.path.exists(opts.to.rstrip("/")):
        die(f"{opts.to} already exists — the conversion writes a new layout, it does not merge")
    change_id = opts.change_id or f"chg:the-saved-design-moves-to-the-item-layout-{opts.date}"

    single = design_io.load_design(opts.source)
    stated = single.get("content_hash")
    recomputed = design_io.design_hash(single)
    print(f"single file: {len(single['nodes'])} nodes, {len(single['edges'])} edges, "
          f"content_hash {stated}, recomputed {recomputed}")
    if stated and stated != recomputed:
        die("the single file does not match its own content_hash — fix that before converting")

    with tempfile.TemporaryDirectory(prefix="convert-items-") as tmp:
        graph = os.path.join(tmp, "graph")
        r = subprocess.run([opts.bin, "--graph-path", graph, "--import", opts.source],
                           capture_output=True, text=True)
        if r.returncode != 0:
            die(f"import failed: {r.stderr.strip()[:2000]}")

        # 1. The layout alone is lossless: same whole-design hash.
        probe = os.path.join(tmp, "probe") + "/"
        call(opts.bin, graph, "export_graph", {"path": probe, "overwrite": True})
        layout = design_io.load_design(probe, verify=True)
        print(f"layout only: {len(layout['nodes'])} nodes, {len(layout['edges'])} edges, "
              f"reassembled hash {layout['content_hash']}")
        if layout["content_hash"] != recomputed:
            die(f"the reassembled hash {layout['content_hash']} differs from the single file's "
                f"{recomputed} — the layout would change the design; nothing was converted")
        if layout["tampered"] or layout["misplaced"]:
            die(f"the layout written is not intact: {layout['tampered'][:3]} {layout['misplaced'][:3]}")

        # 2. One baseline ChangeEvent carries every current checksum.
        current = design_io.current_acceptances(single["edges"])
        seq_max: dict = {}
        for e in single["edges"]:
            p = e.get("properties") or {}
            if e.get("edge_type") == "CHANGED" and p.get("checksum_after"):
                seq_max[e["to_id"]] = max(seq_max.get(e["to_id"], 0), p.get("accepted_seq") or 0)
        artifacts = []
        for n in single["nodes"]:
            if n["node_type"] != "Artifact":
                continue
            props = n.get("properties") or {}
            checksum = props.get("checksum") or (current.get(n["node_id"]) or (None,))[0]
            if not checksum:
                continue
            basis = props.get("checksum_basis") or (current.get(n["node_id"]) or (None, None))[1]
            artifacts.append((n["node_id"], checksum, basis))
        call(opts.bin, graph, "add_change_event", {
            "id": change_id,
            "name": "reflow2's committed design moves to the item layout, and its accepted checksums onto one baseline change",
            "summary": (
                f"The committed design moves from the single file docs/design/reflow2.json to "
                f"the item layout docs/design/reflow2/ (one file per node and per edge), and this "
                f"baseline puts the {len(artifacts)} registered artifacts' current checksums on "
                f"its CHANGED edges (checksum_after), so from here every accepted checksum lives "
                f"on the change that accepted it. Nothing about the system moved: the checksums "
                f"are the ones the record already held. Settled by Anthony 2026-10-03: "
                f"dec:how-the-saved-design-is-laid-out-so-git-merges-it, "
                f"dec:the-designs-lineage-is-kept-per-item, "
                f"dec:item-13-checksums-move-to-change-edges-and-main-converts-in-one-pr "
                f"(decisions 3 and 6). Reassembled, the layout's whole-design hash equals the "
                f"single file's ({recomputed}) before this baseline."),
            "change_type": "refactor",
            "subject": "record",
            "detected_at": opts.date,
        })
        edges = []
        for art, checksum, basis in artifacts:
            props = {"action": "modified", "accepted_baseline": True, "checksum_after": checksum,
                     "accepted_seq": seq_max.get(art, 0) + 1}
            if basis:
                props["checksum_basis"] = basis
            edges.append({"edge_type": "CHANGED", "from_type": "ChangeEvent", "from_id": change_id,
                          "to_type": "Artifact", "to_id": art, "properties": props})
        for i in range(0, len(edges), 200):
            call(opts.bin, graph, "create_edges", {"edges": edges[i:i + 200]})
        print(f"baseline: {change_id} carries {len(edges)} artifact checksums")

        # The conversion PR's own edits (AGENTS.md's gate lines, say) are
        # accepted against the same change, measured from the tree.
        if opts.accept_changed_since:
            diff = subprocess.run(["git", "diff", "--name-only", opts.accept_changed_since],
                                  cwd=design_io.REPO, capture_output=True, text=True, check=True)
            touched = set(diff.stdout.split())
            accepts = [{"artifact_id": art, "disposition": "design_updated",
                        "design_change_event_id": change_id,
                        "note": "edited by the conversion to the item layout"}
                       for art, loc in (
                           (n["node_id"], ((n.get("properties") or {}).get("location") or "").split("#")[0])
                           for n in single["nodes"] if n["node_type"] == "Artifact")
                       if loc in touched and os.path.isfile(os.path.join(design_io.REPO, loc))]
            if accepts:
                call(opts.bin, graph, "set_artifact_checksums", {"accepts": accepts})
            print(f"accepted {len(accepts)} registered file(s) the conversion edited: "
                  f"{', '.join(a['artifact_id'] for a in accepts) or 'none'}")

        # 3. The layout, beside the single file it replaces.
        receipt = call(opts.bin, graph, "export_graph", {"path": opts.to, "overwrite": True})
        print(f"wrote {opts.to}: {receipt.get('items')} content_hash {receipt.get('content_hash')}")
    with open(os.path.join(opts.to, design_io.DESIGN_FILE), encoding="utf-8") as fh:
        stamp = json.load(fh)
    if stamp.get("migrated_from") != recomputed:
        die(f"design.json records migrated_from {stamp.get('migrated_from')}, expected {recomputed}")
    after = design_io.load_design(opts.to, verify=True)
    nodes_with_checksum = sum(1 for n in after["nodes"] if n["node_type"] == "Artifact"
                              and (n.get("properties") or {}).get("checksum"))
    print(f"converted: {len(after['nodes'])} nodes, {len(after['edges'])} edges, "
          f"{nodes_with_checksum} Artifact node(s) still state a checksum the edges do not derive")
    if not opts.keep_source:
        os.remove(opts.source)
        print(f"removed {opts.source}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
