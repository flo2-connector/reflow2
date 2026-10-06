#!/usr/bin/env python3
"""reflow2 check — the consumer CI coherence gate (BL-66).

Answers one question on every commit, loudly: **does the committed design still
describe this build?** It reads the design from the committed export (never the
live `.reflow2/graph` — that directory is gitignored, machine-local, and
single-writer, so CI cannot and should not open it), recomputes every
registered artifact's hash from the working tree, reconciles, and runs the gap
detectors.

    tools/reflow2_check.py                          # design.json, cwd as root
    tools/reflow2_check.py --export docs/design/reflow2/        # the item layout
    tools/reflow2_check.py --export docs/design/reflow2.json    # the single file
    tools/reflow2_check.py --gap-threshold 0.9

The saved design may be the single-file export or the per-item layout (one
file per node and per edge, dec:how-the-saved-design-is-laid-out-so-git-merges-it).
For the layout, INTEGRITY is checked per item, LINEAGE per item against the
merge-base, and the design-vs-build check is GIT-AWARE: every registered file a
change touched must be covered by an acceptance in that change
(dec:item-13-checksums-move-to-change-edges-and-main-converts-in-one-pr).

The build FAILS (exit 1) when:
  - a registered artifact changed or vanished with no two-sided accept — an
    accepted drift updates the export, so a red here means the accept step was
    skipped, which is exactly the erosion this gate exists to catch; or
  - an **anchored** gap (one that names design nodes) at or above
    `--gap-threshold` (default 0.8) is open. Gaps the team has consciously
    accepted via `acknowledge_gap` are not reported by `detect_gaps`, so
    acknowledging — with a reason, on the record — is the sanctioned way to go
    green without fixing. Phase-level nudges ("what comes next") never fail
    the build; they are advice, not defects.

Everything else is printed but does not gate: `no_baseline` artifacts (no hash
registered — register one via the link-artifacts flow), sub-threshold gaps,
and unanchored nudges. Exit codes: 0 coherent · 1 gate failed · 2 could not
run (missing export/binary — never a silent pass).

Standard library only; needs the `reflow2-mcp` binary (`--bin`, `$REFLOW2_BIN`,
on PATH, or a local cargo build).
"""

from __future__ import annotations

import argparse
import glob
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import traceback

_REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# THE ONE READER OF A SAVED DESIGN, in either form — the single-file export or
# the per-item layout (dec:how-the-saved-design-is-laid-out-so-git-merges-it).
# It ships beside this file in the kit; a kit without it cannot read a layout,
# and saying so is exit 2, never a guess.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
try:
    import design_io  # noqa: E402
except ImportError:  # pragma: no cover — a broken kit
    print("reflow2_check: design_io.py must sit beside reflow2_check.py (it reads the saved "
          "design in either form); this kit is incomplete", file=sys.stderr)
    sys.exit(2)


def die(code: int, msg: str) -> None:
    print(f"reflow2_check: {msg}", file=sys.stderr)
    sys.exit(code)


def default_bin() -> str:
    env = os.environ.get("REFLOW2_BIN")
    if env:
        return env
    for candidate in (
        os.path.join(_REPO_ROOT, "target", "debug", "reflow2-mcp"),
        os.path.join(_REPO_ROOT, "target", "release", "reflow2-mcp"),
    ):
        if os.path.exists(candidate):
            return candidate
    found = shutil.which("reflow2-mcp")
    return found or "reflow2-mcp"


class Server:
    """A short-lived reflow2-mcp process spoken to over stdio JSON-RPC.

    The same tiny client as tools/reflow2_cli.py, embedded so this file is
    self-contained — it ships in the consumer kit alone.
    """

    def __init__(self, binary: str, graph_path: str, extra_args: tuple = ()) -> None:
        try:
            self.proc = subprocess.Popen(
                [binary, "--graph-path", graph_path, *extra_args],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1,
                env={**os.environ, "RUST_LOG": os.environ.get("RUST_LOG", "warn")},
            )
        except FileNotFoundError:
            die(2, f"binary not found: {binary} (set --bin or $REFLOW2_BIN)")
        self._id = 0
        self._rpc(
            "initialize",
            {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "reflow2_check", "version": "0"},
            },
        )
        self._rpc("notifications/initialized", {}, notify=True)

    def _rpc(self, method: str, params=None, notify: bool = False):
        msg = {"jsonrpc": "2.0", "method": method}
        if params is not None:
            msg["params"] = params
        if not notify:
            self._id += 1
            msg["id"] = self._id
        self.proc.stdin.write(json.dumps(msg) + "\n")
        self.proc.stdin.flush()
        if notify:
            return None
        line = self.proc.stdout.readline()
        if not line:
            err = (self.proc.stderr.read() or "").strip()
            die(2, f"server exited without responding.\n{err}")
        return json.loads(line)

    def scan_all(self, node_type: str) -> list:
        """Every node of `node_type` — paged, because one reply is not all of them.

        `scan_nodes` answers with as many nodes as fit and says what it withheld
        (`total` vs `returned`, plus `omitted`, `next_offset`, `capped_by`).
        `call` unwraps the `{count, items}` envelope to the items and throws
        those fields away, so a capped page arrives here looking exactly like a
        complete set — and this gate then asserted `exhaustive: true` over it.

        Measured on reflow2's own design 2026-08-04: `capped_by: "size"`,
        `total: 144`, `returned: 124`, `omitted: 20`. Twenty registered
        artifacts were never hashed, so a drifted file among them could not be
        reported — `art:tools-coherence` drifted in this very commit and the
        gate passed it in silence, while `reconcile_artifacts` named it the
        moment it was asked directly. A gate that measures 86% of the tree and
        reports as though it measured all of it is the false-green this whole
        file exists to prevent.

        The count is checked, not assumed: paging that silently comes up short
        would rebuild the same bug one layer down.
        """
        out, offset = [], 0
        while True:
            resp = self._rpc(
                "tools/call",
                {
                    "name": "scan_nodes",
                    "arguments": {"node_type": node_type, "offset": offset},
                },
            )
            if "error" in resp:
                die(2, f"scan_nodes: {resp['error'].get('message', resp['error'])}")
            env = resp["result"].get("structuredContent") or {}
            out.extend(env.get("items") or [])
            nxt, total = env.get("next_offset"), env.get("total")
            if nxt is None or nxt <= offset:
                break
            offset = nxt
        if total is not None and len(out) != total:
            die(
                2,
                f"scan_nodes({node_type}) paged to {len(out)} of {total} — the sweep "
                f"is short, and a short sweep reports OK over whatever it missed.",
            )
        return out

    def call(self, tool: str, args: dict):
        resp = self._rpc("tools/call", {"name": tool, "arguments": args})
        if "error" in resp:
            die(2, f"{tool}: {resp['error'].get('message', resp['error'])}")
        result = resp["result"]
        if result.get("isError"):
            blocks = result.get("content") or []
            text = blocks[0].get("text") if blocks else str(result)
            die(2, f"{tool}: {text}")
        if "structuredContent" in result:
            value = result["structuredContent"]
            # Unwrap ONLY the bare envelope. `loop_hint` (BL-91) is inert
            # prose and may ride along; anything else is a payload the tool
            # BUILT around its list — detect_gaps carries `budget` and
            # `by_source`, and returning the list alone would silently discard
            # the half that says the list is incomplete.
            if isinstance(value, dict) and {"count", "items"} <= value.keys() <= {
                "count", "items", "loop_hint"
            }:
                return value["items"]
            return value
        blocks = result.get("content") or []
        return json.loads(blocks[0]["text"]) if blocks else None

    def close(self) -> None:
        try:
            self.proc.stdin.close()
        except Exception:
            pass
        self.proc.terminate()
        self.proc.wait(timeout=10)


def hash_file(path: str) -> str:
    """The whole sha256 of the file — what an honest observer computes.

    This used to truncate the digest to the registered checksum's length,
    because designs register anything from 16 hex chars to the full 64 and
    `reconcile_artifacts` compared strings. That workaround is why the gate
    reported OK on 2026-08-01 in the same minute a direct sweep of the same
    clean tree called 51 artifacts drifted: **the compensation lived in the
    wrong layer**, so every consumer that was not this file hit the bug. The
    core now answers it for everyone (BL-160, `artifact::checksums_agree`), and
    the gate reports what it actually measured."""
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 16), b""):
            h.update(chunk)
    return f"sha256:{h.hexdigest()}"


_TOML_SECTION = re.compile(r"^\s*\[([^\]]+)\]\s*$")
_TOML_INLINE = re.compile(r"^\s*([A-Za-z0-9_.-]+)\s*=\s*\{(.+)\}\s*$")
_TOML_UNCLOSED = re.compile(r"^\s*([A-Za-z0-9_.-]+)\s*=\s*\{[^}]*$")
_TOML_KV = re.compile(r'([A-Za-z0-9_-]+)\s*=\s*"([^"]*)"')


def _dependency_tables(path: str) -> tuple[list[dict], list[str]]:
    """Every `*dependencies` table in a Cargo manifest, and what could not be read.

    Prefers `tomllib` (Python 3.11+), which is exact. Falls back to a LINE READER
    when it is absent — and that fallback exists because of a measured failure,
    not a hypothetical one: this gate shipped reading only `tomllib`, and its
    first CI run reported "could not read any build file to check them" on
    reflow2's own repo. The runner is ubuntu-22.04, which carries Python 3.10.
    Most CI in the world is older than 3.11, so the dependency check was inert
    almost everywhere it mattered while being perfectly correct locally.

    The fallback is deliberately narrow — single-line inline tables, which is how
    Cargo pins are almost always written — and it REPORTS WHAT IT CANNOT READ
    rather than skipping quietly. A parser that silently drops the shapes it does
    not know rebuilds, one level down, exactly the silence this gate was extended
    to remove. The kit stays install-free: no `tomli`, no pip, because reflow2
    installs into other people's projects and a runtime dependency is a cost they
    did not agree to.
    """
    try:
        import tomllib
    except ModuleNotFoundError:
        tomllib = None

    if tomllib is not None:
        try:
            with open(path, "rb") as fh:
                data = tomllib.load(fh)
        except (OSError, ValueError):
            return [], [f"{path}: unreadable as TOML"]
        return [
            data.get("dependencies", {}) or {},
            data.get("dev-dependencies", {}) or {},
            (data.get("workspace", {}) or {}).get("dependencies", {}) or {},
        ], []

    tables: dict[str, dict] = {}
    unparsed: list[str] = []
    section = ""
    try:
        with open(path, encoding="utf-8") as fh:
            lines = fh.readlines()
    except OSError:
        return [], [f"{path}: unreadable"]

    for line in lines:
        if (m := _TOML_SECTION.match(line)) is not None:
            section = m.group(1).strip()
            # `[dependencies.serde]` is a sub-table per dependency. Legal TOML and
            # not handled here; say so rather than pretend the section was empty.
            if re.search(r"(^|\.)dependencies\.", section):
                unparsed.append(f"{path}: [{section}] sub-table form not read")
            continue
        if not section.endswith("dependencies"):
            continue
        if (m := _TOML_INLINE.match(line)) is not None:
            tables.setdefault(section, {})[m.group(1)] = dict(
                _TOML_KV.findall(m.group(2))
            )
        elif _TOML_UNCLOSED.match(line) is not None:
            unparsed.append(
                f"{path}: '{_TOML_UNCLOSED.match(line).group(1)}' spans lines, not read"
            )
    return list(tables.values()), unparsed


def observe_dependencies(root: str, declared: list) -> tuple[list, list[str]]:
    """What the BUILD actually pins, read fresh from build files.

    Returns `(observations, sources_read, unparsed)`. An empty `sources_read`
    means this gate could not look — which is reported, never treated as "depends
    on nothing" (`reconcile_dependencies` insists on that distinction and the
    gate must honour it). `unparsed` names manifest shapes that WERE seen and
    could not be read, so a partial read is never mistaken for a complete one.

    MATCHED BY SOURCE, NOT BY NAME, and that is the whole difficulty. A design
    declares ONE dependency — `dynograph-foundation` — while the build names
    five crates from it (`dynograph-core`, `-storage`, `-graph`, …). Matching on
    name would report the declaration as unobserved and all five crates as
    undeclared, which is four false findings and one backwards one. The stable
    identifier both sides share is the SOURCE (a git URL, a registry), so
    observations are grouped by source and reported under the declared name.

    Only Cargo is read today. That is a real limit, not a hidden one: a project
    whose pins live in package.json, pyproject.toml, go.mod or versions.env gets
    `sources_read == []` and is told so plainly.
    """
    by_source: dict[str, dict] = {}
    sources_read: list[str] = []
    unparsed: list[str] = []

    manifests = [os.path.join(root, "Cargo.toml")]
    manifests += sorted(glob.glob(os.path.join(root, "crates", "*", "Cargo.toml")))
    for manifest in manifests:
        if not os.path.exists(manifest):
            continue
        tables, could_not_read = _dependency_tables(manifest)
        unparsed.extend(could_not_read)
        if not tables and could_not_read:
            continue
        sources_read.append(os.path.relpath(manifest, root))
        for table in tables:
            for crate, spec in (table or {}).items():
                if not isinstance(spec, dict):
                    continue  # a bare "1.2.3" is a crates.io pin with no shared source
                source = spec.get("git") or spec.get("path") or spec.get("registry")
                if not source:
                    continue
                # A git dep's VERSION is whatever pins it. `tag` first: it is the
                # only one a human wrote on purpose and the only one a provider
                # can be held to.
                version = spec.get("tag") or spec.get("rev") or spec.get("branch") or spec.get("version")
                entry = by_source.setdefault(
                    source, {"version": version, "components": set(), "features": set()}
                )
                entry["components"].add(crate)
                for feat in spec.get("features", []) or []:
                    entry["features"].add(feat)
                if entry["version"] is None:
                    entry["version"] = version

    # Report each observed source under the DECLARED name when one claims it, so
    # the reconcile compares like with like. An observation matching nothing
    # declared keeps its own source as its name — that is the "reliance nobody
    # agreed to", and it should surface rather than be quietly dropped.
    declared_by_source = {
        (d.get("source") or "").strip(): d.get("name") for d in declared if d.get("source")
    }
    observations = []
    for source, entry in sorted(by_source.items()):
        if entry["version"] is None:
            continue  # a path/registry dep with no pin says nothing about version
        observations.append(
            {
                "name": declared_by_source.get(source, source),
                "version": entry["version"],
                "components": sorted(entry["components"]),
                "features": sorted(entry["features"]),
                "observed_in": ", ".join(sources_read),
            }
        )
    return observations, sources_read, unparsed


def _git(args: list[str], cwd: str) -> str | None:
    """Run a git command, or None if git is unavailable or the command failed.
    Never raises: the lineage check is a bonus, and a project without git must
    still be able to run the gate."""
    try:
        out = subprocess.run(
            ["git", *args], capture_output=True, text=True, timeout=60, cwd=cwd
        )
    except (OSError, subprocess.SubprocessError):
        return None
    return out.stdout if out.returncode == 0 else None


def _changed_paths(root: str) -> tuple[set[str], str] | None:
    """Repo-relative paths THIS CHANGE touched, with a word for what was compared.
    `None` when git cannot answer at all.

    WHY THIS EXISTS. The unmodelled-source note below is correct and was being
    skimmed: it reports every unmodelled file in the tree (107 here), which reads
    as an institutional backlog rather than as anything the reader did. Measured
    from the other side on 2026-08-23 — a dev_storyflow agent built a four-lane
    feature, registered ZERO artifacts, and named the aggregate framing as the
    reason: "391 is not a number anybody can act on ... three gaps on the node I
    just edited would have been a task." The files a reader actually touched are
    the subset they can act on now, and they are usually two or three.

    IT DOES NOT CHANGE THE SEVERITY, and that is deliberate. The note stays a
    note — `dec:idea-allocation-waits-for-the-last-responsible-moment` defers
    allocation, and failing the build here would reverse that ruling while
    appearing to implement it (Anthony, 2026-08-21). Narrowing WHAT IS SHOWN and
    demanding action are different acts; this does the first.

    Two sources, unioned, because either alone lies. `status --porcelain` sees a
    file written but not yet committed; the merge-base diff sees one committed
    earlier on this branch. A session that commits as it goes would be invisible
    to the first, and one that has not committed yet invisible to the second.
    """
    top_out = _git(["rev-parse", "--show-toplevel"], root)
    if not top_out:
        return None
    top = top_out.strip()
    changed: set[str] = set()
    parts: list[str] = []

    status = _git(["status", "--porcelain"], top)
    if status is not None:
        for line in status.splitlines():
            if len(line) < 4:
                continue
            path = line[3:]
            # A rename reads `old -> new`; the NEW name is the one on disk.
            if " -> " in path:
                path = path.split(" -> ", 1)[1]
            changed.add(os.path.normpath(path.strip().strip('"')))
        parts.append("uncommitted work")

    # Everything committed on this branch since it left the trunk. The base is
    # tried in order rather than assumed: a clone with no `origin`, or a trunk
    # called something else, must degrade to "no branch half" and SAY so, not
    # silently report a smaller set.
    for base in ("origin/HEAD", "origin/main", "origin/master", "main", "master"):
        merge_base = _git(["merge-base", "HEAD", base], top)
        if not merge_base:
            continue
        diff = _git(["diff", "--name-only", merge_base.strip(), "HEAD"], top)
        if diff is None:
            continue
        for line in diff.splitlines():
            if line.strip():
                changed.add(os.path.normpath(line.strip()))
        parts.append(f"commits since {base}")
        break

    if not parts:
        return None
    return changed, " and ".join(parts)


def _repo_relative(path: str) -> tuple[str, str] | None:
    """`(repo_root, path_within_repo)`, or None when the file is not in a git
    working tree. `git show REV:path` only understands repo-relative paths, so
    an absolute --export would otherwise skip the check without saying so."""
    directory = os.path.dirname(os.path.abspath(path)) or "."
    top = _git(["rev-parse", "--show-toplevel"], directory)
    if not top:
        return None
    root = top.strip()
    try:
        rel = os.path.relpath(os.path.abspath(path), root)
    except ValueError:
        return None
    if rel.startswith(".."):
        return None
    return root, rel.replace(os.sep, "/")


def _export_at(rev: str, root: str, rel: str) -> dict | None:
    """The export document as of a git revision, or None when there isn't one
    (untracked, or the revision predates the file)."""
    out = _git(["show", f"{rev}:{rel}"], root)
    if not out or not out.strip():
        return None
    try:
        return json.loads(out)
    except ValueError:
        return None


def _lineage_anchor(root: str) -> str | None:
    """The commit an export's lineage chains FROM: the merge-base with the
    default branch, when this checkout has one and HEAD is not already on it.

    THE SAME ANCHOR THE EXPORT TOOL USES (crates/reflow2-mcp/src/git.rs, since
    2026-09-12). Until then both sides chained from the last file at the path,
    and `dec:export-once-per-pr` — one exporting commit per branch, last — was
    the discipline that kept that honest. The tool moved to the merge-base so
    the rule holds by construction, and THIS CHECK DID NOT MOVE WITH IT: it
    went on expecting HEAD~1, so the first branch with two export commits was
    refused by the gate for doing exactly what the tool now guarantees. A
    mechanism wired into the one place that motivated it, siblings left alone.

    Returns None when HEAD IS the merge-base (a commit on the trunk itself, or
    no default branch resolvable): there the predecessor is HEAD's parent, as
    before, and a squash-merge lands one hop from the previous trunk commit.
    """
    for base in ("origin/HEAD", "origin/main", "origin/master", "main", "master"):
        merge_base = _git(["merge-base", "HEAD", base], root)
        if not merge_base:
            continue
        merge_base = merge_base.strip()
        head = _git(["rev-parse", "HEAD"], root)
        if head and head.strip() == merge_base:
            return None
        return merge_base
    return None


def _export_pair(path: str, doc: dict) -> tuple[dict, dict] | None:
    """This export and the one it replaced, or None when unanswerable.

    Two contexts, one rule. Before a commit the working file is new and its
    predecessor is the committed anchor's version; in CI the working file IS
    HEAD's version, so the pair is HEAD against the anchor. THE ANCHOR is the
    merge-base with the default branch when the branch has one (see
    `_lineage_anchor`), otherwise HEAD or HEAD~1 as it always was. Either way we
    return a document and the one it replaced.

    Shared by every check that compares an export with its predecessor, rather
    than being reimplemented per check. Two copies of a predicate drift, and
    when they do they give contradictory answers about the same file — the
    defect [BL-177] records in `reflow2_init.py`, where the dry run and the real
    run disagreed because each tested its own version of "would this change?".
    """
    located = _repo_relative(path)
    if located is None:
        return None  # not in a git working tree — nothing to compare against
    root, rel = located
    head = _export_at("HEAD", root, rel)
    if head is None:
        return None  # untracked, no commits yet, or the commit introducing it
    anchor = _lineage_anchor(root)
    if head.get("content_hash") != doc.get("content_hash"):
        # A new export, not yet committed: it chains from the anchor if the
        # branch has one, else from HEAD's version.
        previous = _export_at(anchor, root, rel) if anchor else head
        if previous is None:
            return None  # the anchor commit does not carry this export yet
        current = doc
    else:
        previous = _export_at(anchor or "HEAD~1", root, rel)
        if previous is None:
            return None  # HEAD is the first commit carrying this export
        current = head
    if previous.get("content_hash") == current.get("content_hash"):
        return None  # content unchanged — nothing replaced anything
    return current, previous


def check_export_identity(path: str, doc: dict) -> str | None:
    """Refuse a design that changed its NAME without anyone saying so (BL-169).

    `graph_id` is the design's durable identity: minted once, never negotiated,
    and it namespaces every stored key — so a graph reopened under a different
    name finds nothing and presents as an empty design. It is also inside the
    export's `content_hash`, which means a rename is indistinguishable from
    ordinary content change to every other check here.

    That is not hypothetical. On 2026-08-02 an export replayed through a temp
    graph came back as `05a6fbe860bf7a23` where the design had been `reflow2`
    since its first commit, and it was committed and pushed. **The lineage check
    passed** (the chain was intact across the rename), the integrity check
    passed (the hash matched its own content), and **both CI jobs were green.**
    The only signal anywhere was a `provenance_note` string in `compare_designs`
    that nothing gates on. A design's identity moving is either deliberate or a
    bug, and it must not be able to happen quietly.

    Returns a failure message, or None when sound or unanswerable. A first
    export has no predecessor to disagree with, and an unidentified document
    (`graph_id: ""`, legitimate for a hand-authored one — BL-138) is not a
    rename: absence of a name is not a different name.
    """
    pair = _export_pair(path, doc)
    if pair is None:
        return None
    current, previous = pair
    was, now = previous.get("graph_id"), current.get("graph_id")
    if not was or not now or was == now:
        return None
    return (
        f"IDENTITY  '{path}' changed the design's name from '{was}' to '{now}'. "
        f"`graph_id` is minted once and never negotiated — it namespaces every "
        f"stored key, so a store reopened under a different name finds nothing "
        f"and reads as an EMPTY design — and it sits inside the content hash, "
        f"which is why every other check here passes across a rename. The usual "
        f"cause is a replay: an export imported into a TEMP graph through the "
        f"`import_graph` tool and re-exported from there takes the temp store's "
        f"name. Seed a replay with the CLI (`reflow2-mcp --graph-path <tmp> "
        f"--import <doc>`), which adopts the document's identity into an empty "
        f"store. If the rename is deliberate, commit it on its own so it is "
        f"reviewable as what it is."
    )


def check_round_trip(doc: dict, server, tmp: str) -> str | None:
    """Does this export survive being READ BACK? Export → import → re-export.

    THE ONLY PROBE THAT SEES THIS CLASS, and it needs no knowledge of any
    particular schema rule. Everything else here verifies the export against
    ITSELF: `content_hash` re-hashes the same bytes, the chain links one export
    to the last, `sync_status` compares a recorded hash to the file. All of them
    stay green while the document is unreadable by the importer, because none of
    them ever runs the importer.

    dev_storyflow's committed export could not be imported by the binary that
    wrote it, and had not been restorable for at least four days across thirty
    export commits, while every one of those signals read clean (2026-08-15).
    THE STAKE IS NOT CI: `.reflow2/graph` is gitignored, machine-local and
    single-writer, so the committed export is the only copy of a design that
    survives losing that directory. An export nobody has ever imported is a
    backup nobody has ever restored.

    The import leg already ran above — an unimportable document dies at exit 2
    before this. What this adds is the RETURN LEG: a document can import cleanly
    and still come back DIFFERENT, and a lossy round trip is silent in a way an
    invalid one is not.

    Compared structurally rather than by hash, deliberately: a hash says only
    THAT they differ, and the useful answer is WHICH nodes and edges did not
    survive. Lineage fields are excluded because they are expected to differ —
    a re-export chains from what it replaced, which is a different question and
    `check_export_chain` already owns it.
    """
    out = os.path.join(tmp, "round-trip.json")
    try:
        server.call("export_graph", {"path": out, "overwrite": True})
        with open(out, encoding="utf-8") as f:
            back = json.load(f)
    except Exception as e:  # noqa: BLE001 — a failed re-export IS the finding
        return f"ROUND TRIP  the design could not be re-exported after import: {e}"

    def nodes_of(d):
        return {(n.get("node_type"), n.get("node_id")): n.get("properties", {})
                for n in d.get("nodes", [])}

    def edges_of(d):
        return {(e.get("edge_type"), e.get("from_id"), e.get("to_id")): e.get("properties", {})
                for e in d.get("edges", [])}

    before_n, after_n = nodes_of(doc), nodes_of(back)
    before_e, after_e = edges_of(doc), edges_of(back)

    lost_n = sorted(k for k in before_n if k not in after_n)
    lost_e = sorted(k for k in before_e if k not in after_e)
    gained_n = sorted(k for k in after_n if k not in before_n)
    gained_e = sorted(k for k in after_e if k not in before_e)
    changed = sorted(k for k in before_n if k in after_n and before_n[k] != after_n[k])

    if not (lost_n or lost_e or gained_n or gained_e or changed):
        return None

    def sample(items, n=5):
        shown = ", ".join(":".join(str(p) for p in k) for k in items[:n])
        return shown + (f" (+{len(items) - n} more)" if len(items) > n else "")

    parts = []
    if lost_n:
        parts.append(f"{len(lost_n)} node(s) did not survive: {sample(lost_n)}")
    if lost_e:
        parts.append(f"{len(lost_e)} edge(s) did not survive: {sample(lost_e)}")
    if changed:
        parts.append(f"{len(changed)} node(s) came back with different properties: {sample(changed)}")
    if gained_n:
        parts.append(f"{len(gained_n)} node(s) appeared that were not in the export: {sample(gained_n)}")
    if gained_e:
        parts.append(f"{len(gained_e)} edge(s) appeared that were not in the export: {sample(gained_e)}")

    return (
        "ROUND TRIP  this export does not survive being read back — "
        + "; ".join(parts)
        + ". The committed export is the only copy of this design that survives "
        "losing the gitignored graph directory, so a lossy round trip is a backup "
        "that would not restore. Every other check here compares the export to "
        "ITSELF and cannot see this."
    )


def check_export_chain(path: str, doc: dict) -> str | None:
    """Verify this export links to its predecessor (`dec:export-hash-chain`).

    The chain gives the design a history independent of git: each export records
    the `content_hash` of the one it replaced. Inside a git repository
    `export_graph --path` builds that link from the file AS COMMITTED AT THE
    MERGE-BASE with the default branch (since 2026-09-12); outside one, from
    whatever file is already at the target path, so exporting to a scratch path
    and copying the result into place severs it — silently, which is how six
    consecutive commits lost the link in July 2026 with the gate green, the loop
    clean and zero gaps every time (BL-107).

    Two contexts, one rule. Before a commit the working file is new and its
    predecessor is the anchor's version; in CI the working file IS HEAD's
    version, so the pair to check is HEAD against the anchor. Either way we
    compare a document with the one it replaced — see `_export_pair`.

    Returns a failure message, or None when sound OR when there is nothing to
    check against — an unanswerable question is skipped, never guessed. The
    chain deliberately does not advance while content is unchanged, and a first
    export has no predecessor; neither is a break.
    """
    pair = _export_pair(path, doc)
    if pair is None:
        return None
    current, previous = pair
    expected = previous.get("content_hash")
    actual = current.get("prev_content_hash")
    if not expected or actual == expected:
        return None
    was = "nothing" if actual is None else actual
    return (
        f"LINEAGE  '{path}' does not link to the export it replaced: it records "
        f"{was} where {expected} is expected. The design's history is independent "
        f"of git and this severs it.\n"
        f"      THE RULE THIS ENFORCES is `dec:export-once-per-pr`: a pull request "
        f"lands exactly ONE hop of the chain. Since 2026-09-12 that holds by "
        f"construction — inside a git repository `export_graph` chains from "
        f"{path} as COMMITTED AT THE MERGE-BASE with the default branch, so any "
        f"number of exports on a branch each chain from the same ancestor and "
        f"this check expects that ancestor's hash. Multiple exporting commits on "
        f"one branch are fine; the chain being anchored anywhere else is not.\n"
        f"      TWO CAUSES. (1) The export was written by a reflow2 older than "
        f"2026-09-12, or outside this repository and copied in: it chained from "
        f"whatever file was at the path — an intermediate that was never on the "
        f"default branch. Restore the committed file (`git checkout {path}`) and "
        f"export straight onto it with a current reflow2. (2) The default branch "
        f"could not be resolved from this checkout (no `origin`, or a trunk with "
        f"another name), so the tool fell back to on-disk chaining and this check "
        f"fell back to HEAD's parent, and the two picked different ancestors. "
        f"Read `chained_from` in the export receipt: `origin/main@<sha>` means "
        f"the anchor was used, `disk` means it was not, and `chain_note` says why."
    )


def check_taken_on_this_branch(path: str, doc: dict) -> str | None:
    """An export ABOUT TO BE COMMITTED was taken on the branch it is being
    committed to.

    Since 2026-09-16 every export written inside a git repository carries
    `taken_at` — the branch and commit the working tree was at, and whether it
    was dirty — because the graph does not branch with git: one store serves
    every branch checked out in a directory, so a plain export on branch B
    carries branch A's writes and merges cleanly. That was caught once as four
    phantom drifts (`dec:idea-should-a-node-carry-its-git-coordinate`, option
    ②), and the export is the one thing that can say which tree it came from.

    THE ONLY CASE CHECKED is the one that is a defect: the working-tree export
    is MODIFIED (about to be committed) and its `taken_at.branch` names a
    different branch from the one checked out. A committed export is not
    re-judged — squash merges rewrite branch history, so the name it carries
    stops meaning anything once it lands — and a detached HEAD, an export with
    no coordinate, or a tree outside git cannot be judged and is skipped
    rather than guessed.
    """
    taken = doc.get("taken_at") or {}
    taken_branch = taken.get("branch")
    if not taken_branch:
        return None
    rel = _repo_relative(path)
    if rel is None:
        return None
    root, inside = rel
    here = (_git(["rev-parse", "--abbrev-ref", "HEAD"], root) or "").strip()
    if not here or here == "HEAD" or here == taken_branch:
        return None
    status = _git(["status", "--porcelain", "--", inside], root)
    if not status or not status.strip():
        return None
    return (
        f"PHANTOM  '{path}' was taken on branch '{taken_branch}' and is about to be "
        f"committed on '{here}'. The graph does not branch with git — one store serves "
        f"every branch checked out here — so an export taken elsewhere carries that "
        f"branch's writes into this one, and it merges cleanly. Restore the committed "
        f"file (`git checkout {path}`) and export again FROM THIS BRANCH so the record "
        f"says where it came from; if the other branch's design writes really belong "
        f"here, say so in the commit and re-export anyway."
    )


# ---- the change this run is judging (per-item lineage and coverage) --------


class ChangeRange:
    """What this run judges as "the change": everything between `base` and the
    working tree.

    Three shapes, decided from git and nothing else:

    - **pr** — CI on a pull request, where the checkout is the merge commit
      (two parents). `base` is main's tip (HEAD^1), so the range is exactly
      what the PR brings to main, and `pr_head` (HEAD^2) is where its
      acceptances must match its files: "at the PR head", decision 3.
    - **branch** — a working tree on a branch (or uncommitted work on the
      trunk): `base` is the merge-base with the default branch (or HEAD), and
      acceptances must match the files as they are on disk.
    - **trunk** — a commit on the default branch itself (CI's push to main):
      `base` is HEAD^1 and only COVERAGE is asked — a file two PRs both edited
      is a merge of two accepted changes, which neither acceptance can match.
    """

    def __init__(self, root: str, base: str, mode: str, pr_head: str | None, label: str):
        self.root, self.base, self.mode, self.pr_head, self.label = root, base, mode, pr_head, label

    @property
    def requires_match(self) -> bool:
        return self.mode in ("pr", "branch")


def change_range(start: str) -> ChangeRange | None:
    """The range this run judges, or None when git cannot say (no repository,
    no commits) — and then the git-aware checks are skipped, SAID, and the
    record-only checks stand."""
    top = _git(["rev-parse", "--show-toplevel"], start)
    if not top:
        return None
    root = top.strip()
    head = (_git(["rev-parse", "--verify", "--quiet", "HEAD"], root) or "").strip()
    if not head:
        return None
    parents = (_git(["rev-parse", "HEAD^@"], root) or "").split()
    # A pull request's CI checks out the merge commit GitHub made, and says so:
    # GITHUB_SHA is that commit. Requiring it to BE HEAD keeps any other merge
    # commit (a scratch repository a test builds, a local merge) out of PR mode.
    if (os.environ.get("GITHUB_EVENT_NAME") == "pull_request"
            and os.environ.get("GITHUB_SHA") == head and len(parents) == 2):
        return ChangeRange(root, parents[0], "pr", parents[1],
                           f"this pull request ({parents[1][:7]} merged onto {parents[0][:7]})")
    merge_base = None
    for candidate in ("origin/HEAD", "origin/main", "origin/master", "main", "master"):
        mb = _git(["merge-base", "HEAD", candidate], root)
        if mb and mb.strip():
            merge_base = mb.strip()
            break
    if merge_base and merge_base != head:
        return ChangeRange(root, merge_base, "branch", None,
                           f"this branch since {merge_base[:7]}, as on disk")
    dirty = (_git(["status", "--porcelain", "--untracked-files=no"], root) or "").strip()
    if dirty:
        return ChangeRange(root, head, "branch", None, f"uncommitted work on {head[:7]}")
    if parents:
        return ChangeRange(root, parents[0], "trunk", None,
                           f"commit {head[:7]} on the default branch")
    return None


def _changed_since(rng: ChangeRange, under: str | None = None) -> set[str]:
    """Repo-relative paths that differ between `rng.base` and the working tree
    (tracked changes plus untracked files), optionally only `under` a path."""
    # -z: without it git QUOTES any path holding a byte past ASCII ("\303\251"),
    # and an escaped item id passes such characters through, so that item —
    # or a registered file with such a name — would silently drop out of
    # every check that reads this set.
    tail = ["--", under] if under else []
    out: set[str] = set()
    diff = _git(["diff", "--name-only", "--no-renames", "-z", rng.base, *tail], rng.root) or ""
    out.update(p for p in diff.split("\0") if p)
    untracked = _git(["ls-files", "--others", "--exclude-standard", "-z", *tail], rng.root) or ""
    out.update(p for p in untracked.split("\0") if p)
    return out


def check_item_lineage(path: str, rng: ChangeRange | None) -> list[str]:
    """PER-ITEM LINEAGE (dec:the-designs-lineage-is-kept-per-item): every item
    file the change touched names, in `prev_item_hash`, that item's hash at the
    base — or nothing, when the item is new. An unchanged item is not judged.
    That is what makes a squash-merge land each changed item one hop, and what
    lets a branch export as often as it likes, before or during a merge."""
    if rng is None:
        return []
    located = _repo_relative(path)
    if located is None:
        return []
    _, rel_dir = located
    rel_dir = rel_dir.rstrip("/")
    prefix = rel_dir + "/"
    touched = sorted(
        p[len(prefix):] for p in _changed_since(rng, rel_dir)
        if p.startswith(prefix) and p.endswith(".json")
        and p[len(prefix):].split("/", 1)[0] in (design_io.NODES_DIR, design_io.EDGES_DIR)
    )
    if not touched:
        return []
    at_base = design_io.items_at(rng.root, rng.base, rel_dir, touched)
    broken = []
    for rel in touched:
        full = os.path.join(path, rel)
        if not os.path.exists(full):
            continue  # deleted: nothing left to chain
        with open(full, "rb") as fh:
            try:
                now = design_io.parse_item(rel, fh.read())
            except ValueError:
                continue  # unreadable items fail the load itself
        before = at_base.get(rel)
        expected = before.stated if before is not None else None
        if before is not None and before.stated == now.stated:
            continue  # same content as the base: not part of this change
        if now.prev != expected:
            broken.append(f"{rel}: records {now.prev or 'nothing'}, expected "
                          f"{expected or 'nothing (new in this change)'}")
    if not broken:
        return []
    shown = "; ".join(broken[:5]) + (f" (+{len(broken) - 5} more)" if len(broken) > 5 else "")
    return [
        f"LINEAGE  {len(broken)} item(s) in '{path}' do not name their hash at {rng.base[:7]} "
        f"({rng.label}): {shown}. Each changed item's prev_item_hash must be its content_hash "
        f"at the merge-base with the default branch (dec:the-designs-lineage-is-kept-per-item). "
        f"The usual cause is an item copied in from another tree or edited by hand; re-export "
        f"from the graph with a current reflow2, which anchors every changed item there."
    ]


def _repo_rel_location(root: str, project_root: str, location: str) -> str | None:
    """A registered location as a repo-relative path, or None when it is not a
    plain file inside the repository (a URI, an escape, a directory)."""
    loc = location.split("#", 1)[0]
    if not loc or "://" in loc:
        return None
    full = os.path.normpath(os.path.join(project_root, loc))
    try:
        rel = os.path.relpath(full, root)
    except ValueError:
        return None
    if rel.startswith(".."):
        return None
    return rel.replace(os.sep, "/")


def _sha256_bytes(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def range_acceptances(path: str, doc: dict, rng: ChangeRange) -> dict:
    """`{artifact_id: [(seq, change_id, checksum)]}` — the acceptances THIS
    change made: CHANGED edges carrying `checksum_after` that are new, or carry
    a different checksum, relative to the design at `rng.base`."""
    located = _repo_relative(path)
    if located is None:
        return {}
    _, rel = located
    rel = rel.rstrip("/")
    found: dict = {}

    def note(e, checksum):
        props = e.get("properties") or {}
        found.setdefault(e["to_id"], []).append(
            (props.get("accepted_seq") or 0, e["from_id"], checksum))

    if doc.get("layout") == "items":
        prefix = rel + "/" + design_io.EDGES_DIR + "/"
        touched = sorted(p[len(rel) + 1:] for p in _changed_since(rng, rel)
                         if p.startswith(prefix) and p.endswith(".json"))
        at_base = design_io.items_at(rng.root, rng.base, rel, touched)
        for r in touched:
            full = os.path.join(path, r)
            if not os.path.exists(full):
                continue
            with open(full, "rb") as fh:
                it = design_io.parse_item(r, fh.read())
            body = it.body
            if body.get("edge_type") != "CHANGED":
                continue
            checksum = (body.get("properties") or {}).get("checksum_after")
            if not checksum:
                continue
            before = at_base.get(r)
            if before is not None and (before.body.get("properties") or {}).get("checksum_after") == checksum:
                continue
            note(body, checksum)
        return found

    base_doc = design_io.load_design_at(rng.root, rng.base, rel) or {"edges": []}
    before = design_io.acceptances(base_doc.get("edges", []))
    for e in doc.get("edges", []):
        if e.get("edge_type") != "CHANGED":
            continue
        checksum = (e.get("properties") or {}).get("checksum_after")
        if checksum and before.get((e["from_id"], e["to_id"])) != checksum:
            note(e, checksum)
    return found


def check_checksum_coverage(path: str, doc: dict, rng: ChangeRange,
                            project_root: str) -> tuple[list[str], set[str]]:
    """THE GIT-AWARE DESIGN-VS-BUILD CHECK (decision 3 of
    dec:item-13-checksums-move-to-change-edges-and-main-converts-in-one-pr).

    Every registered file this change touched must be covered by an acceptance
    in this change; on a PR or a branch, the acceptance must also MATCH the file
    at the PR head (or on disk). On the trunk only coverage is asked: a file two
    PRs both edited is the merge of two accepted changes, and neither one's
    checksum can be the merged file's.

    Returns the failures and the artifact ids it judged, so the server's
    record-only drift verdict on those is not reported twice."""
    failures: list[str] = []
    judged: set[str] = set()
    changed = _changed_since(rng)
    accepted = range_acceptances(path, doc, rng)
    by_artifact: dict[str, str] = {}
    for n in doc.get("nodes", []):
        if n.get("node_type") != "Artifact":
            continue
        props = n.get("properties") or {}
        if props.get("volatility") in ("append_only", "living"):
            continue  # an expected change, reported by the server as such
        loc = props.get("location")
        if not loc:
            continue
        rel = _repo_rel_location(rng.root, project_root, loc)
        if rel:
            by_artifact[n["node_id"]] = rel

    def file_at_target(rel: str) -> bytes | None:
        if rng.pr_head:
            out = subprocess.run(["git", "show", f"{rng.pr_head}:{rel}"], cwd=rng.root,
                                 capture_output=True, timeout=60)
            return out.stdout if out.returncode == 0 else None
        full = os.path.join(rng.root, rel)
        if not os.path.isfile(full):
            return None
        with open(full, "rb") as fh:
            return fh.read()

    for art, rel in sorted(by_artifact.items()):
        mine = sorted(accepted.get(art, []), key=lambda t: (-t[0], t[1]))
        touched = rel in changed
        if not touched and not mine:
            continue
        if touched and not os.path.isfile(os.path.join(rng.root, rel)):
            continue  # deleted: the server's missing_artifact owns it
        judged.add(art)
        if touched and not mine:
            failures.append(
                f"DRIFT  {art}: checksum_change — {rel} changed in {rng.label} and no change in "
                f"it accepted the new checksum. Accept it two-sided (set_artifact_checksums, a "
                f"disposition per file), then export."
            )
            continue
        if not rng.requires_match:
            continue
        content = file_at_target(rel)
        if content is None:
            continue
        actual = _sha256_bytes(content)
        _, change, checksum = mine[0]
        if not _checksums_agree(checksum, actual):
            where = f"the PR head {rng.pr_head[:7]}" if rng.pr_head else "disk"
            failures.append(
                f"DRIFT  {art}: checksum_change — {change} accepted {checksum} for {rel}, and the "
                f"file at {where} is {actual}. Re-accept it after the last edit "
                f"(set_artifact_checksums), then export."
            )
    return failures, judged


def _checksums_agree(a: str, b: str) -> bool:
    """The core's `checksums_agree`: one digest, whatever length it was written at."""
    if a == b:
        return True
    if not (a.startswith("sha256:") and b.startswith("sha256:")):
        return False
    ah, bh = a[7:], b[7:]
    return bool(ah) and bool(bh) and (ah.startswith(bh) or bh.startswith(ah))


def check_layout_identity(path: str, rng: ChangeRange | None) -> str | None:
    """BL-169 for the item layout: `graph_id` in design.json did not move."""
    if rng is None:
        return None
    located = _repo_relative(path)
    if located is None:
        return None
    _, rel = located
    before = _git(["show", f"{rng.base}:{rel.rstrip('/')}/{design_io.DESIGN_FILE}"], rng.root)
    try:
        with open(os.path.join(path, design_io.DESIGN_FILE), encoding="utf-8") as fh:
            now = json.load(fh).get("graph_id")
        was = json.loads(before).get("graph_id") if before else None
    except (OSError, ValueError):
        return None
    if not was or not now or was == now:
        return None
    return (
        f"IDENTITY  '{path}' changed the design's name from '{was}' to '{now}'. `graph_id` is "
        f"minted once and never negotiated — it namespaces every stored key. The usual cause is "
        f"a replay through a TEMP graph; seed one with `reflow2-mcp --graph-path <tmp> --import "
        f"<design>`, which adopts the design's identity into an empty store."
    )


def check_layout_taken_on_this_branch(path: str) -> str | None:
    """The PHANTOM check for the item layout: its `taken_at` lives in the
    git-ignored sidecar, and "about to be committed" means any item file in the
    layout is modified or new."""
    try:
        with open(os.path.join(path, design_io.TAKEN_AT_FILE), encoding="utf-8") as fh:
            taken = json.load(fh) or {}
    except (OSError, ValueError):
        return None
    taken_branch = taken.get("branch")
    if not taken_branch:
        return None
    rel = _repo_relative(os.path.join(path, design_io.DESIGN_FILE))
    if rel is None:
        return None
    root, _ = rel
    here = (_git(["rev-parse", "--abbrev-ref", "HEAD"], root) or "").strip()
    if not here or here == "HEAD" or here == taken_branch:
        return None
    status = _git(["status", "--porcelain", "--", os.path.abspath(path)], root)
    if not status or not status.strip():
        return None
    return (
        f"PHANTOM  '{path}' was taken on branch '{taken_branch}' and is about to be committed on "
        f"'{here}'. The graph does not branch with git — one store serves every branch checked "
        f"out here — so a design taken elsewhere carries that branch's writes into this one. "
        f"Restore the committed layout (`git checkout -- {path}`) and export again FROM THIS "
        f"BRANCH; if the other branch's writes really belong here, say so and re-export anyway."
    )


def synced_export_path(root: str) -> str | None:
    """The export the graph under `root` last recorded itself in step with —
    `.reflow2/graph.sync.json`'s `last_synced` keys — the first that exists,
    else the first named (so a vanished export is reported by the name the
    graph gave it, not as a missing `design.json`). None only when there is
    no sidecar or it names nothing."""
    sidecar = os.path.join(root, ".reflow2", "graph.sync.json")
    try:
        with open(sidecar, encoding="utf-8") as fh:
            synced = json.load(fh).get("last_synced") or {}
    except (OSError, ValueError):
        return None
    named = [p if os.path.isabs(p) else os.path.join(root, p) for p in synced]
    for candidate in named:
        if os.path.exists(candidate):
            return candidate
    return named[0] if named else None


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument(
        "--export",
        default=None,
        help=(
            "committed design export (JSON). Default: the export the graph under --root says "
            "it is in step with (.reflow2/graph.sync.json), else design.json"
        ),
    )
    ap.add_argument("--root", default=".", help="project root artifact locations are relative to")
    ap.add_argument("--bin", default=default_bin(), help="reflow2-mcp binary")
    ap.add_argument(
        "--gap-threshold",
        type=float,
        default=0.8,
        help="anchored gaps at/above this severity fail the build (default 0.8)",
    )
    ap.add_argument(
        "--gap-reply-budget",
        type=int,
        default=5_000_000,
        help=(
            "characters of detect_gaps reply to ask for (default 5,000,000). "
            "Deliberately enormous: the tool's own default is sized for an agent's "
            "context window and this is a script reading a pipe, so the gate wants "
            "every gap in full and would rather be slow than judge a partial list"
        ),
    )
    opts = ap.parse_args()

    # THE DEFAULT IS WHAT THE GRAPH ITSELF RECORDS. The three designs on the
    # maintainer's own box keep their export at three different paths and none
    # of them was `design.json` (flo2, 2026-09-18); the store's sync sidecar
    # already names the export it is in step with, so that is the honest
    # default and a bare `reflow2 check` finds it.
    # FIRST the record the project COMMITS: `.reflow2.toml` `[export] path`, which a
    # CI checkout has even though it has no store and so no sidecar (field log
    # 2026-10-06, fact:root-cause-the-export-path-has-no-owner-so-writes-the-hook-and-init-disagree-2026-10-06).
    export_from_sidecar = None
    named_by = None
    if opts.export is None:
        if recorded := design_io.recorded_export(opts.root):
            opts.export = os.path.join(opts.root, recorded)
            named_by = os.path.join(opts.root, design_io.RECORD_FILE) + " [export] path"
        else:
            export_from_sidecar = synced_export_path(opts.root)
            opts.export = export_from_sidecar or "design.json"
            if export_from_sidecar:
                named_by = os.path.join(opts.root, ".reflow2", "graph.sync.json")

    if not os.path.exists(opts.export):
        where = f"'{opts.export}' (named by {named_by})" if named_by else f"'{opts.export}'"
        die(
            2,
            f"no design export at {where}. Commit one "
            f"(export_graph to a repo path, or reflow2-mcp --export) and point --export at it — "
            f"the gate reads the committed design, never the live .reflow2/ store.",
        )

    failures: list[str] = []
    notes: list[str] = []

    # Integrity first (dec:export-hash-chain): the export carries a hash of its
    # own content, so a committed record that was hand-edited or corrupted is
    # detectable before anything downstream trusts it. The canonical form must
    # byte-match the Rust side's: compact separators, sorted keys, raw unicode
    # (tools/smoke_mcp.py pins the two implementations against each other).
    # For the item layout the same question is asked PER ITEM: each file
    # carries its own content_hash, and the whole-design hash is computed.
    try:
        doc = design_io.load_design(opts.export, verify=True)
    except (OSError, ValueError) as e:
        die(2, f"could not read '{opts.export}' as a reflow2 design: {e}")
    items_layout = doc.get("layout") == "items"
    # The git-aware checks belong to the item layout: the single file keeps
    # its whole-file chain and record-only drift check for the overlap
    # release, exactly as before, so a project that has not converted sees no
    # change in what fails its build.
    rng = change_range(os.path.abspath(opts.export)) if items_layout else None
    embedded = None if items_layout else doc.get("content_hash")
    if items_layout:
        if doc.get("tampered"):
            shown = ", ".join(doc["tampered"][:5])
            more = f" (+{len(doc['tampered']) - 5} more)" if len(doc["tampered"]) > 5 else ""
            failures.append(
                f"INTEGRITY  {len(doc['tampered'])} item file(s) in '{opts.export}' do not match "
                f"their own content_hash — edited outside reflow2 or corrupted: {shown}{more}. "
                f"Re-export from the graph, or review what changed them."
            )
        if doc.get("misplaced"):
            failures.append(
                f"INTEGRITY  {len(doc['misplaced'])} item file(s) in '{opts.export}' sit at a path "
                f"their content does not belong at: {', '.join(doc['misplaced'][:5])}. A merge "
                f"that kept a renamed copy looks like this; re-export from the graph."
            )
        notes.append(
            f"design: item layout, {len(doc.get('nodes', []))} nodes and "
            f"{len(doc.get('edges', []))} edges, content hash {doc.get('content_hash')} "
            f"(computed on read)"
        )
    elif embedded:
        canonical = json.dumps(
            {"edges": doc.get("edges", []), "graph_id": doc.get("graph_id"),
             "nodes": doc.get("nodes", [])},
            sort_keys=True, ensure_ascii=False, separators=(",", ":"),
        )
        actual = "sha256:" + hashlib.sha256(canonical.encode("utf-8")).hexdigest()
        if actual != embedded:
            failures.append(
                f"INTEGRITY  '{opts.export}' does not match its own content_hash — the "
                f"committed design record was edited outside reflow2 or corrupted. "
                f"Re-export from the graph, or review what changed it."
            )
    else:
        notes.append("integrity: export predates content hashing (no content_hash)")

    # LINEAGE (BL-107) — a severed chain used to be completely silent. The
    # single file keeps its whole-file chain for the overlap release; the item
    # layout's lineage is per item (dec:the-designs-lineage-is-kept-per-item).
    if items_layout:
        failures.extend(check_item_lineage(opts.export, rng))
        if rng is None:
            notes.append("lineage: not judged — git could not say what this change is (no "
                         "repository or no commits), so per-item lineage was not checked")
        phantom = check_layout_taken_on_this_branch(opts.export)
        renamed = check_layout_identity(opts.export, rng)
    else:
        broken_chain = check_export_chain(opts.export, doc)
        if broken_chain:
            failures.append(broken_chain)
        phantom = check_taken_on_this_branch(opts.export, doc)
        # IDENTITY (BL-169) — a rename passes every check above, because
        # graph_id is inside the content hash and the chain links across it.
        renamed = check_export_identity(opts.export, doc)
    if phantom:
        failures.append(phantom)
    if renamed:
        failures.append(renamed)

    with tempfile.TemporaryDirectory(prefix="reflow2-check-") as tmp:
        graph = os.path.join(tmp, "graph")
        imported = subprocess.run(
            [opts.bin, "--graph-path", graph, "--import", opts.export],
            capture_output=True,
            text=True,
        )
        if imported.returncode != 0:
            die(2, f"could not import '{opts.export}':\n{imported.stderr.strip()}")

        # `--tree-root`: the imported copy lives in a temp dir, and registered
        # locations are relative to the PROJECT — so the server is told where
        # the tree is and does the measuring itself (see the reconcile below).
        server = Server(opts.bin, graph, extra_args=("--tree-root", os.path.abspath(opts.root)))
        try:
            # ROUND TRIP FIRST, because it is the only check here that asks
            # whether the document can be READ BACK, and everything below this
            # line is already reasoning about the imported copy rather than the
            # committed one. If the two disagree, every finding after this is
            # about a design that is not quite the one in the repository.
            lossy = check_round_trip(doc, server, tmp)
            # A relation the export holds twice and out of step is brought
            # into step as it imports, and the import SAYS so. The round trip
            # then differs, but the cause is not a backup that fails to
            # restore: it is a committed export written before its copies were
            # kept in step. So the import's own line is named first
            # (req:a-relation-stored-in-more-than-one-place-has-one-authoritative-copy-and-no-copy-drifts-unnoticed).
            twins = next((line.strip() for line in imported.stderr.splitlines()
                          if "stored twice" in line), None)
            if twins:
                failures.append(
                    "STORED TWINS  the committed export holds relations stored twice "
                    f"and out of step, and importing it {twins.removeprefix('reflow2: ')} "
                    "Re-export from a store this binary has opened (the repair runs on "
                    "open and is reported in loop_status as `repaired_on_open`), so the "
                    "committed copy agrees with its authority.")
            # Reported as well, never instead: a round trip can also lose
            # something the twin repair has nothing to do with.
            if lossy:
                failures.append(lossy)

            # Paged, not one reply: `exhaustive: true` below is a CLAIM, and it
            # was false by 20 artifacts until this used scan_all.
            artifacts = server.scan_all("Artifact")
            # ⭐ THE SERVER MEASURES, NOT THIS SCRIPT. The server was started
            # with `--tree-root` at the project, so the reconcile below (no
            # `observed`) hashes every registered file with the same code
            # `loop_status` runs in a session. This loop used to build the
            # observations itself, and its idea of what a location MEANS drifted
            # from the server's on four shapes — a URI (not judged here, MISSING
            # to the server, so flo2's session kept reporting twelve missing
            # files after its CI went green), a path outside the project and a
            # `..` escape (hashed here, refused by the server as outside the
            # root), and `file#fragment` (missing here, measured there). One
            # instrument said "note", the other "red build", about the same
            # artifact in the same state. tools/test_one_meaning_for_an_artifact_location.py
            # holds the two to one answer per shape.
            #
            # The reply budget is deliberately enormous for the same reason the
            # gap budget is: a script reading a pipe wants every finding, and a
            # trimmed answer is refused below rather than judged.
            drift = server.call(
                "reconcile_artifacts",
                {"exhaustive": True, "budget_chars": opts.gap_reply_budget},
            )
            if "budget" in drift:
                die(
                    2,
                    "the server trimmed its reconcile reply to fit a budget, so the gate "
                    "would be judging a partial list — raise --gap-reply-budget",
                )
            measurement = drift.get("measurement", {})
            if measurement.get("basis") != "measured":
                die(2, f"the server did not measure the tree: {json.dumps(measurement)[:400]}")

            # NO SILENT CAPS: an artifact nobody could judge must say so, or "0
            # drift findings" reads as "everything was checked". The sentence is
            # the SERVER'S (`reason`), so there is no second table of what a URI,
            # a directory or an escape means kept here to drift from it.
            for item in sorted(
                measurement.get("unmeasurable", []), key=lambda u: u.get("artifact_id", "")
            ):
                notes.append(
                    f"not judged: {item.get('artifact_id')} is located at "
                    f"{item.get('location')} — {item.get('reason') or item.get('not_measured')}"
                )
            if measurement.get("without_location"):
                notes.append(
                    f"not judged: {measurement['without_location']} artifact(s) carry no "
                    f"location, so there is nothing to measure"
                )

            # ⭐ WHAT THE DESIGN HAS NEVER HEARD OF. Every check above this line
            # reasons over artifacts the design ALREADY KNOWS — the loop is
            # `for art in artifacts`, and `reconcile_artifacts` does no file I/O
            # at all, so the caller decides what is looked at. That makes
            # `undocumented_addition` a drift kind THIS GATE STRUCTURALLY CANNOT
            # PRODUCE: add a file, commit, push, and every check stays green.
            # Measured 2026-08-21, and the same design-outward-to-files blind
            # spot had just been found and fixed in tools/wall_check.py.
            #
            # The roots are DERIVED from the paths the design already gave, so
            # this needs no configuration — the same property wall_check keeps.
            #
            # ⚠️ IT IS A NOTE, NEVER A FAILURE, AND THAT IS THE WHOLE POINT.
            # `dec:idea-allocation-waits-for-the-last-responsible-moment`
            # (accepted) defers allocation to the last responsible moment, and a
            # RED BUILD here would force every new file to be placed at the
            # moment it is written — reversing that ruling while appearing to
            # implement it. NOTICING a file is unmodelled and DEMANDING it be
            # allocated are different acts. This does the first. Anthony's call,
            # 2026-08-21: a note first, and whether it ever becomes a failure is
            # a governance question answered by whether this gets acted on or
            # skimmed.
            claimed_paths = set()
            for art in artifacts:
                loc = (art.get("properties", {}) or {}).get("location")
                if loc:
                    claimed_paths.add(os.path.normpath(os.path.join(opts.root, loc)))
            source_roots = {
                os.path.dirname(p) for p in claimed_paths if os.path.isdir(os.path.dirname(p))
            }
            # Only the outermost of any nested pair, so a directory is not walked twice.
            tops = sorted(
                r
                for r in source_roots
                if not any(r != o and r.startswith(o + os.sep) for o in source_roots)
            )
            unheard = []
            for top in tops:
                for dirpath, _, filenames in os.walk(top):
                    for fn in filenames:
                        # Source only. A `mod.rs` or `lib.rs` is a namespace
                        # declaration rather than a unit of design — the same
                        # reason an assembly correctly points at no file.
                        if not fn.endswith((".rs", ".py")) or fn in ("mod.rs", "lib.rs"):
                            continue
                        full = os.path.normpath(os.path.join(dirpath, fn))
                        if full not in claimed_paths:
                            unheard.append(os.path.relpath(full, opts.root))
            unheard.sort()

            # ⭐ LEAD WITH WHAT THIS CHANGE TOUCHED. Same finding, narrowed to the
            # subset the reader can act on right now. The aggregate still prints
            # below it, so nothing is hidden — but it stops being the first thing
            # read, which is what turned it into furniture.
            touched = _changed_paths(opts.root)
            if touched is None:
                # A ZERO HERE MUST NOT READ AS A CLEAN RESULT. Without git there
                # is no "this change" to scope to, and staying silent would be
                # indistinguishable from having looked and found nothing.
                if unheard:
                    notes.append(
                        "unmodelled source, THIS CHANGE: not computed — git could not say which "
                        "files this working copy touched, so only the whole-tree count below is "
                        "available. That is a missing measurement, not a clean result."
                    )
            else:
                changed_paths, basis = touched
                mine = [u for u in unheard if u in changed_paths]
                if mine:
                    shown = ", ".join(mine[:8])
                    more = f", +{len(mine) - 8} more" if len(mine) > 8 else ""
                    notes.append(
                        f"unmodelled source, THIS CHANGE: {len(mine)} of the file(s) you "
                        f"touched ({basis}) have no Artifact pointing at them — {shown}{more}. "
                        f"link-artifacts registers one against the capability it realizes, with a "
                        f"checksum so a later edit is detectable. Registering is an OFFER, not a "
                        f"demand: allocation stays deferred to the last responsible moment "
                        f"(dec:idea-allocation-waits-for-the-last-responsible-moment), and this "
                        f"is a note either way."
                    )

            if unheard:
                # GROUPED BY DIRECTORY, not listed. Ninety-seven filenames on one
                # line is the signal a reader learns to skim, and a count that
                # mixes kinds is a count nobody acts on — the same lesson that
                # split assemblies out of wall_check's coverage gap twice over.
                # The grouping is mechanical, so it states where they are and
                # leaves which-of-these-matter to the person who knows.
                by_dir = {}
                for rel in unheard:
                    by_dir.setdefault(os.path.dirname(rel) or ".", []).append(rel)
                where = ", ".join(
                    f"{d} {len(v)}"
                    for d, v in sorted(by_dir.items(), key=lambda kv: -len(kv[1]))[:5]
                )
                extra = f", +{len(by_dir) - 5} more dir(s)" if len(by_dir) > 5 else ""
                notes.append(
                    f"unmodelled source: {len(unheard)} file(s) no Artifact points at, in "
                    f"{len(by_dir)} director(y/ies) — {where}{extra}. NOT a failure and NOT a "
                    f"demand to allocate them: the design simply has not been told they exist. "
                    f"link-artifacts registers one; allocation stays deferred to the last "
                    f"responsible moment (dec:idea-allocation-waits-for-the-last-responsible-moment)."
                )

            # THE DESIGN-VS-BUILD CHECK IS GIT-AWARE when git can say what this
            # change is (decision 3 of
            # dec:item-13-checksums-move-to-change-edges-and-main-converts-in-one-pr):
            # every registered file the change touched is covered by an
            # acceptance in it, matching the file on a PR or a branch. The
            # server's record-only verdict then judges only what that cannot —
            # a vanished file — and a checksum mismatch on a file this change
            # did NOT touch is the merge of two accepted changes, a note.
            covered: set[str] = set()
            if items_layout and rng is None:
                notes.append(
                    "design vs build: git could not say what this change is, so drift is judged "
                    "against the record alone (every checksum mismatch fails)")
            if rng is not None:
                coverage, covered = check_checksum_coverage(
                    opts.export, doc, rng, os.path.abspath(opts.root))
                failures.extend(coverage)
                notes.append(f"design vs build: judged per change — {rng.label}")
            for finding in drift.get("findings", []):
                kind = finding.get("kind")
                art = finding.get("artifact_id")
                what = f"{art}: {kind}"
                if kind == "missing_artifact" or (kind == "checksum_change" and rng is None):
                    failures.append(
                        f"DRIFT  {what} — the build no longer matches the committed design. "
                        f"Reconcile and accept two-sided (set_artifact_checksum), then re-export."
                    )
                elif kind == "checksum_change" and art in covered:
                    continue  # judged above, against this change
                elif kind == "checksum_change":
                    notes.append(
                        f"drift: {what} — the file is not part of {rng.label}, so this is not "
                        f"this change's to accept; its latest acceptance differs from it, which "
                        f"is what a merge of two accepted edits to one file leaves. Accept the "
                        f"merged content (set_artifact_checksums) when convenient."
                    )
                else:
                    notes.append(f"drift: {what}")

            # DEPENDENCIES — the design says which version of another design it
            # depends on (req:design-dependencies-declared, accepted). That has
            # been checkable since the capability shipped and NOTHING A CONSUMER
            # RUNS EVER CHECKED IT: this gate reconciled artifacts and stopped.
            # A declaration nobody verifies is a promise, not a check.
            #
            # ⭐ WHY THE EMPTY CASE IS HANDLED SEPARATELY RATHER THAN JUST
            # RECONCILING: with no observations every declared dependency comes
            # back `unobserved`, so a project whose pins this gate cannot read
            # would fail for having declared anything at all — punishing the
            # correct behaviour. Silence about what was not checked is the bug;
            # inventing findings about it is a worse one.
            # The tool answers `{manifest, report}` — the report is the nested
            # half. Reading `declared` off the top level silently yields [] and
            # the gate then reports "0 declared … agree", which is a FALSE GREEN
            # in the code written to prevent false greens. Caught by checking the
            # number against a design known to declare one; it would never have
            # been caught by the gate passing.
            deps = (server.call("reconcile_dependencies", {"observed": []}) or {}).get("report", {})
            declared_deps = deps.get("declared", []) or []
            observed_deps, sources_read, unparsed = observe_dependencies(
                opts.root, declared_deps
            )
            for shape in unparsed:
                notes.append(f"dependencies: {shape}")

            if declared_deps and not sources_read:
                notes.append(
                    f"dependencies: {len(declared_deps)} declared, and this gate could not read "
                    f"any build file to check them — NOTHING VERIFIED THE PINS. Only Cargo is "
                    f"read today; a project pinning elsewhere is unchecked, not clean."
                )
            elif declared_deps or observed_deps:
                report = (server.call(
                    "reconcile_dependencies", {"observed": observed_deps}
                ) or {}).get("report", {})
                for finding in report.get("findings", []):
                    kind = finding.get("kind")
                    detail = finding.get("detail") or kind
                    if kind in ("undeclared", "version_mismatch", "unobserved"):
                        failures.append(
                            f"DEPEND {finding.get('dependency')}: {kind} — {detail} "
                            f"Fix the pin, or re-declare it (declare_dependency), then re-export."
                        )
                    else:
                        notes.append(f"dependency: {finding.get('dependency')}: {kind} — {detail}")
                if not report.get("findings"):
                    notes.append(
                        f"dependencies: {len(declared_deps)} declared, "
                        f"{len(observed_deps)} observed in {', '.join(sources_read)} — agree"
                    )
            # Nothing declared AND nothing observed is genuinely quiet: the design
            # has said it depends on no other design, which is a statement, not a
            # silence — and reconcile's own note already says so if asked.

            reply = server.call("detect_gaps", {"budget_chars": opts.gap_reply_budget}) or []
            # A rich envelope now, a bare list against a server older than the
            # budget (0.38.0 and before). Both are read rather than one assumed.
            gaps = reply.get("items", []) if isinstance(reply, dict) else reply
            budget = reply.get("budget") if isinstance(reply, dict) else None
            if budget and budget.get("listed", 0) < budget.get("of", 0):
                # The one tier where gaps are absent from the list altogether.
                # A gate that judged only what it was shown, and said nothing
                # about the rest, would be the erosion this script exists to
                # catch, performed by the script.
                failures.append(
                    f"GAPS   detect_gaps listed {budget['listed']} of {budget['of']} open "
                    f"gap(s) — the rest were not shown, so this gate cannot judge them. "
                    f"Re-run with a larger --gap-reply-budget."
                )
            for gap in gaps:
                # `affected_total`, NOT `affected_ids`: a budgeted reply
                # withholds the id lists when the full answer would not fit, and
                # reading emptiness as "unanchored" would demote every real gap
                # to a phase nudge and turn this gate GREEN with the design full
                # of them. `affected_total` is present in every tier. The
                # fallback keeps this working against a server older than the
                # budget (0.38.0 and before), where the field does not exist.
                anchored = bool(gap.get("affected_total", len(gap.get("affected_ids") or [])))
                severity = float(gap.get("severity", 0.0))
                line = f"{gap.get('id')} [{severity:.2f}] {gap.get('title')}"
                if anchored and severity >= opts.gap_threshold:
                    failures.append(
                        f"GAP    {line} — fix it, or accept it on the record (acknowledge_gap)."
                    )
                else:
                    notes.append(f"gap: {line}" + ("" if anchored else " (phase nudge)"))
        finally:
            server.close()

    for note in notes:
        print(f"  note  {note}")
    for failure in failures:
        print(f"  FAIL  {failure}")
    if failures:
        print(f"\nreflow2 check: FAILED — {len(failures)} finding(s), {len(notes)} note(s).")
        return 1
    print(f"\nreflow2 check: OK — design and build agree ({len(notes)} note(s)).")
    return 0


def run_guarded() -> int:
    """`main()`, with the exit-code contract actually enforced.

    THE CONTRACT IS ENFORCED HERE OR NOWHERE. This module's docstring promises
    "0 coherent · 1 gate failed · 2 could not run", and a dedicated code exists
    for could-not-run precisely so a CI job can tell a drifted design from a
    broken tool. Until this guard existed, ANY unhandled exception in `main()`
    landed in Python's own exit 1 — the code reserved for "gate failed" — so the
    one confusion the three-code design was built to prevent was reachable from
    every line of it. Not a silent pass; a MISFILED one, and invisible in a badge
    or a required-check summary where the traceback is the only tell.

    ⭐ A NAMED FUNCTION RATHER THAN A BARE `try` UNDER `__main__`, because a
    guard nobody can call is a claim nobody can check. The first version of this
    fix lived under `__main__` and its test passed with the guard REMOVED — it
    forced a fault `die()` already handled, so it proved nothing. Every fault
    this tool knows about is already routed to 2 correctly; the guard is for the
    ones it does not know about, and the only honest way to exercise that is to
    hand it one.

    `die()` raises SystemExit, which must pass through untouched so an explicit
    exit code wins — and it does, because SystemExit derives from BaseException
    rather than Exception.
    """
    try:
        return main()
    except Exception:  # noqa: BLE001 — deliberate: any unexpected fault is a 2.
        traceback.print_exc()
        print(
            "reflow2_check: the check could not run — this is exit 2, not a gate failure",
            file=sys.stderr,
        )
        return 2


if __name__ == "__main__":
    sys.exit(run_guarded())
