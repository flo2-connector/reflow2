#!/usr/bin/env python3
"""The ONE Python reader of a saved reflow2 design — the single-file export or
the per-item layout.

Why one reader: on 2026-10-03, 39 files in this repository named the export's
path and every Python one parsed it with its own `json.load`. The item layout
(`dec:how-the-saved-design-is-laid-out-so-git-merges-it`, Anthony 2026-10-03)
makes that path a DIRECTORY, and a gate that kept its own `json.load` would read
it as "cannot read" — or worse, keep reading a stale single file beside it. So
every gate and tool loads the design through `load_design`, which takes either
form for the one-release overlap the decision grants, and returns the shape the
single file always had: `{"graph_id", "nodes", "edges", "content_hash", ...}`.

The layout, as `crates/reflow2-core/src/item_layout.rs` writes it (that file is
the authority; this one mirrors it, and `tools/test_item_layout_merges.py`
pins the two against each other on the real binary — names, hashes and the
odd ids that exercise the escaping):

    <dir>/design.json                 {"graph_id", "schema_version", "migrated_from"?}
    <dir>/nodes/<Type>/<escaped>.json one node + content_hash (+ prev_item_hash)
    <dir>/edges/<xx>/<hash20>.json    one edge + content_hash (+ prev_item_hash)
    <dir>/taken_at.json               git-ignored sidecar (never committed)

Standard library only: it ships in the consumer kit beside reflow2_check.py.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess

DESIGN_FILE = "design.json"
TAKEN_AT_FILE = "taken_at.json"
NODES_DIR = "nodes"
EDGES_DIR = "edges"
_MAX_NAME_BYTES = 150

# This repository's own saved design, in both forms. The directory wins when it
# exists: during the overlap a stale single file may still sit beside it.
REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ITEMS_REL = os.path.join("docs", "design", "reflow2")
FILE_REL = os.path.join("docs", "design", "reflow2.json")


def default_export(root: str = REPO) -> str:
    """This project's saved design: `docs/design/reflow2/` when the item layout
    is there, else `docs/design/reflow2.json`."""
    items = os.path.join(root, ITEMS_REL)
    if os.path.isdir(items):
        return items
    return os.path.join(root, FILE_REL)


def is_item_layout(path: str) -> bool:
    """A directory, or a path spelled with a trailing separator, is the item
    layout; anything else is the single file — the server's own rule."""
    return os.path.isdir(path) or path.endswith(("/", os.sep))


def canonical(obj) -> str:
    """The canonical compact JSON every reflow2 hash is taken over — sorted
    keys, compact separators, raw unicode — byte-identical to the Rust side."""
    return json.dumps(obj, sort_keys=True, ensure_ascii=False, separators=(",", ":"))


def sha256_tag(text: str) -> str:
    return "sha256:" + hashlib.sha256(text.encode("utf-8")).hexdigest()


def design_hash(doc: dict) -> str:
    """The whole-design content hash over `{edges, graph_id, nodes}`. For the
    item layout this is COMPUTED, never stored."""
    return sha256_tag(canonical({
        "edges": doc.get("edges", []),
        "graph_id": doc.get("graph_id"),
        "nodes": doc.get("nodes", []),
    }))


def escape_id(node_id: str) -> str:
    """An id as a file name: [a-z0-9._-] pass through, other ASCII becomes %XX,
    anything else passes through; a long name keeps its start and ends in
    `~` + 16 hex of the id's hash."""
    out = []
    for c in node_id:
        if ("a" <= c <= "z") or ("0" <= c <= "9") or c in "._-":
            out.append(c)
        elif ord(c) < 128:
            out.append("%%%02X" % ord(c))
        else:
            out.append(c)
    name = "".join(out)
    raw = name.encode("utf-8")
    if len(raw) <= _MAX_NAME_BYTES:
        return name
    cut = _MAX_NAME_BYTES - 17
    while cut > 0 and (raw[cut] & 0xC0) == 0x80:
        cut -= 1
    digest = hashlib.sha256(node_id.encode("utf-8")).hexdigest()
    return raw[:cut].decode("utf-8") + "~" + digest[:16]


def node_rel_path(node_type: str, node_id: str) -> str:
    return f"{NODES_DIR}/{node_type}/{escape_id(node_id)}.json"


def edge_rel_path(edge_type: str, from_id: str, to_id: str) -> str:
    key = hashlib.sha256(f"{edge_type}\0{from_id}\0{to_id}".encode("utf-8")).hexdigest()
    return f"{EDGES_DIR}/{key[:2]}/{key[:20]}.json"


def item_body(item: dict) -> dict:
    """An item file without its lineage fields — the node or edge as the single
    file holds it."""
    return {k: v for k, v in item.items() if k not in ("content_hash", "prev_item_hash")}


def item_rel_path(body: dict) -> str:
    if "node_id" in body:
        return node_rel_path(body["node_type"], body["node_id"])
    return edge_rel_path(body["edge_type"], body["from_id"], body["to_id"])


class Item:
    """One item file, parsed and checked against itself."""

    __slots__ = ("rel", "body", "stated", "_computed", "prev")

    def __init__(self, rel: str, data: dict):
        self.rel = rel
        self.body = item_body(data)
        self.stated = data.get("content_hash")
        self.prev = data.get("prev_item_hash")
        self._computed = None

    @property
    def computed(self) -> str:
        """The hash of the body as it actually is (computed on first ask)."""
        if self._computed is None:
            self._computed = sha256_tag(canonical(self.body))
        return self._computed

    @property
    def intact(self) -> bool:
        return self.stated == self.computed

    @property
    def is_node(self) -> bool:
        return "node_id" in self.body

    @property
    def key(self) -> tuple:
        b = self.body
        if self.is_node:
            return ("N", b.get("node_type"), b.get("node_id"))
        return ("E", b.get("edge_type"), b.get("from_id"), b.get("to_id"))


def parse_item(rel: str, raw: bytes | str) -> Item:
    data = json.loads(raw)
    if not isinstance(data, dict):
        raise ValueError(f"{rel} is not a reflow2 item file")
    return Item(rel, data)


class LayoutError(ValueError):
    """The item layout cannot be read as one design."""


def read_items(path: str) -> tuple[dict | None, dict]:
    """`(stamp, {rel: Item})` for the layout at `path`. Raises LayoutError on a
    file that is not an item."""
    stamp = None
    stamp_path = os.path.join(path, DESIGN_FILE)
    if os.path.exists(stamp_path):
        with open(stamp_path, encoding="utf-8") as fh:
            stamp = json.load(fh)
    items: dict = {}
    for top in (NODES_DIR, EDGES_DIR):
        base = os.path.join(path, top)
        if not os.path.isdir(base):
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = sorted(d for d in dirnames if not d.startswith("."))
            for fn in sorted(filenames):
                if fn.startswith("."):
                    continue
                full = os.path.join(dirpath, fn)
                rel = os.path.relpath(full, path).replace(os.sep, "/")
                if not fn.endswith(".json"):
                    raise LayoutError(f"{path} holds {rel}, which is not an item file")
                with open(full, "rb") as fh:
                    try:
                        items[rel] = parse_item(rel, fh.read())
                    except ValueError as e:
                        raise LayoutError(f"{rel} is not a reflow2 item file: {e}") from e
    return stamp, items


def assemble(stamp: dict, items: dict, verify: bool = True) -> dict:
    """One document from item files, sorted as the exporter sorts, with the
    whole-design hash computed. Raises LayoutError when two files hold one
    item — choosing one would silently drop the other. `verify=False` skips
    the per-item integrity and placement checks (a reader that is not a gate),
    and then `tampered`/`misplaced` are None rather than an empty list — not
    checked is not the same as clean."""
    nodes, edges, seen = {}, {}, {}
    tampered, misplaced, duplicates = ([], [], []) if verify else (None, None, [])
    for rel in sorted(items):
        it = items[rel]
        if verify:
            if not it.intact:
                tampered.append(rel)
            expected = item_rel_path(it.body)
            if expected != rel:
                misplaced.append(f"{rel} (belongs at {expected})")
        if it.key in seen:
            duplicates.append(f"{it.key} in {seen[it.key]} and {rel}")
            continue
        seen[it.key] = rel
        if it.is_node:
            nodes[(it.body["node_type"], it.body["node_id"])] = it.body
        else:
            edges[(it.body["edge_type"], it.body["from_id"], it.body["to_id"])] = it.body
    if duplicates:
        raise LayoutError(
            f"{len(duplicates)} item(s) are held by two files: " + "; ".join(duplicates[:5]))
    doc = {
        "graph_id": (stamp or {}).get("graph_id", ""),
        "nodes": [nodes[k] for k in sorted(nodes)],
        "edges": [edges[k] for k in sorted(edges)],
    }
    doc["content_hash"] = design_hash(doc)
    doc["layout"] = "items"
    doc["design"] = stamp or {}
    doc["tampered"] = tampered
    doc["misplaced"] = misplaced
    return doc


def load_design(path: str, verify: bool = False) -> dict:
    """The saved design at `path`, in either form, as one document.

    The single file comes back exactly as stored, plus `"layout": "file"`. The
    item layout comes back assembled — `graph_id`, `nodes`, `edges`, the
    computed `content_hash` — plus `"layout": "items"`, its `design` stamp, and
    the item files that are `tampered` (content does not match its own hash) or
    `misplaced` (None unless `verify`). Raises OSError/ValueError (LayoutError)
    when unreadable.
    """
    if is_item_layout(path):
        stamp, items = read_items(path)
        if stamp is None:
            raise LayoutError(f"{path} is not a reflow2 design: there is no {DESIGN_FILE} in it")
        return assemble(stamp, items, verify=verify)
    with open(path, encoding="utf-8") as fh:
        doc = json.load(fh)
    if isinstance(doc, dict):
        doc.setdefault("layout", "file")
    return doc


# ---- git: the design as committed ------------------------------------------


def _git(args: list, cwd: str, binary: bool = False):
    try:
        out = subprocess.run(["git", *args], cwd=cwd, capture_output=True, timeout=120)
    except (OSError, subprocess.SubprocessError):
        return None
    if out.returncode != 0:
        return None
    return out.stdout if binary else out.stdout.decode("utf-8", "replace")


def items_at(root: str, rev: str, rel_dir: str, rels: list) -> dict:
    """`{rel: Item}` for those of `rels` (paths inside the layout at `rel_dir`)
    that exist at `rev` — one `git cat-file --batch` for all of them."""
    if not rels:
        return {}
    prefix = f"{rel_dir.rstrip('/')}/" if rel_dir else ""
    feed = "".join(f"{rev}:{prefix}{r}\n" for r in rels).encode("utf-8")
    try:
        out = subprocess.run(["git", "cat-file", "--batch"], cwd=root, input=feed,
                             capture_output=True, timeout=600)
    except (OSError, subprocess.SubprocessError):
        return {}
    if out.returncode != 0:
        return {}
    buf, at, found = out.stdout, 0, {}
    for rel in rels:
        nl = buf.find(b"\n", at)
        if nl < 0:
            break
        header = buf[at:nl].decode("utf-8", "replace")
        at = nl + 1
        if header.endswith(" missing") or header.endswith(" ambiguous"):
            continue
        size = int(header.rsplit(" ", 1)[1])
        blob = buf[at:at + size]
        at += size + 1
        try:
            found[rel] = parse_item(rel, blob)
        except ValueError:
            pass
    return found


def load_design_at(root: str, rev: str, rel_path: str) -> dict | None:
    """The saved design as committed at `rev`, or None when that revision does
    not carry one at `rel_path` (a repo-relative path, either form)."""
    rel_path = rel_path.rstrip("/")
    listing = _git(["ls-tree", "-r", "--name-only", rev, "--", rel_path + "/"], root)
    if listing:
        names = [n for n in listing.splitlines() if n]
        stamp_blob = _git(["show", f"{rev}:{rel_path}/{DESIGN_FILE}"], root)
        if stamp_blob is None:
            return None
        rels = [n[len(rel_path) + 1:] for n in names
                if n.endswith(".json") and n[len(rel_path) + 1:].split("/", 1)[0] in (NODES_DIR, EDGES_DIR)]
        items = items_at(root, rev, rel_path, rels)
        return assemble(json.loads(stamp_blob), items)
    blob = _git(["show", f"{rev}:{rel_path}"], root)
    if not blob or not blob.strip():
        return None
    try:
        doc = json.loads(blob)
    except ValueError:
        return None
    if isinstance(doc, dict):
        doc.setdefault("layout", "file")
    return doc


# ---- accepted checksums (dec:item-13-checksums-move-to-change-edges-…) -----


def current_acceptances(edges: list) -> dict:
    """`{artifact_id: (checksum, basis, seq, change_id)}` — the CURRENT
    acceptance of each artifact among CHANGED edges carrying `checksum_after`:
    the highest `accepted_seq`, ties to the smaller change id. The rule
    `export::current_acceptances` applies in Rust."""
    best: dict = {}
    for e in edges:
        if e.get("edge_type") != "CHANGED":
            continue
        props = e.get("properties") or {}
        checksum = props.get("checksum_after")
        if not checksum:
            continue
        seq = props.get("accepted_seq") or 0
        cur = best.get(e["to_id"])
        if cur is None or seq > cur[2] or (seq == cur[2] and e["from_id"] < cur[3]):
            best[e["to_id"]] = (checksum, props.get("checksum_basis"), seq, e["from_id"])
    return best


def acceptances(edges: list) -> dict:
    """`{(change_id, artifact_id): checksum_after}` for every CHANGED edge that
    accepted a checksum."""
    out = {}
    for e in edges:
        if e.get("edge_type") == "CHANGED":
            c = (e.get("properties") or {}).get("checksum_after")
            if c:
                out[(e["from_id"], e["to_id"])] = c
    return out




if __name__ == "__main__":  # pragma: no cover — a tiny CLI for humans
    import sys

    target = sys.argv[1] if len(sys.argv) > 1 else default_export()
    d = load_design(target, verify=True)
    print(json.dumps({
        "path": target,
        "layout": d.get("layout"),
        "graph_id": d.get("graph_id"),
        "nodes": len(d.get("nodes", [])),
        "edges": len(d.get("edges", [])),
        "content_hash": d.get("content_hash") if d.get("layout") == "items" else design_hash(d),
        "tampered": len(d.get("tampered") or []),
        "misplaced": len(d.get("misplaced") or []),
    }, indent=2))
