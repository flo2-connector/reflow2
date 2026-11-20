#!/usr/bin/env python3
"""Assert that version claims in reflow2's docs match what the build actually uses.

The drift class: a bump lands in `Cargo.toml` and the prose still advertises the
previous one. Cargo pins dependencies and enforces it; nothing enforces a
sentence. So a reader orienting from `AGENTS.md` gets a confident, wrong answer —
and confidently wrong is worse than absent, because nobody goes to check.

Adapted from dynograph-foundation's `scripts/check-doc-versions.sh`, WITH ONE
DELIBERATE DIFFERENCE. That script greps with `|| true`, so a pattern matching
nothing produces no output and the check passes; its own header documents this as
known brittleness — "reworking any of those phrases in the docs disables the
corresponding check silently". reflow2 holds `req:no-silent-fallback` at critical
priority, so here **a pattern that matches nothing is a FAILURE**. Rewording a
doc line breaks the build loudly and you fix both sides together, which is the
whole point: a check that can be disabled by editing prose is a check that will
be, eventually, by someone who never knew it existed.

Targeted patterns only — never a generic "any X.Y.Z" scan. Historical references
("the nine releases up to v0.10.1", "as of the v0.5.0 as-released work") are
CORRECT prose about the past and must not be rewritten by a version bump. That is
why each claim is anchored on a literal phrase rather than matched by shape.

Usage:  python3 tools/check_doc_versions.py
Exit 0 = every targeted claim agrees with the build.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# --------------------------------------------------------------------------
# What the build actually says. These are the facts every doc claim is checked
# against; read from the manifests rather than passed in, so the checker cannot
# be run against a version somebody asserted on the command line.
# --------------------------------------------------------------------------


def reflow2_version() -> str:
    """reflow2's own workspace version."""
    text = (REPO / "Cargo.toml").read_text()
    m = re.search(r'^\[workspace\.package\](?:.|\n)*?^version = "([^"]+)"', text, re.M)
    if not m:
        sys.exit("FAIL: could not read the workspace version from Cargo.toml")
    return m.group(1)


def foundation_tag() -> str:
    """The dynograph-foundation tag the absorbed code was taken FROM.

    Until 2026-08-24 this read a git-tag pin out of the workspace `Cargo.toml`.
    `dec:absorb-the-foundation-subset-and-end-the-dependency` removed the last
    pin, so that question has no answer any more — but the FACT it protected
    survives the move, because the decision requires every absorbed module to
    carry a provenance header naming the tag and files it took. That header is
    now the source of truth, and it is still read from the tree rather than from
    prose, which is the property that made this check worth having.

    Every absorbed module must name ONE tag, for the same reason five pins had
    to agree: modules taken from different vintages of the same repository is a
    real defect, so disagreement is reported rather than resolved by taking the
    first. **Finding no header at all is also a failure** — that is the objection
    the decision recorded against absorbing anything (vendoring turns a visible
    dependency into an invisible one), and a silently-passing check here is
    exactly how the header would rot away unnoticed.
    """
    headers = sorted((REPO / "crates/reflow2-core/src").rglob("*.rs"))
    tags: dict[str, list[str]] = {}
    for path in headers:
        for tag in re.findall(r"^//! tag\s+(v[0-9]+\.[0-9]+\.[0-9]+)\s*$", path.read_text(), re.M):
            tags.setdefault(tag, []).append(str(path.relative_to(REPO)))
    if not tags:
        sys.exit(
            "FAIL: no absorbed-code provenance header names a dynograph-foundation "
            "tag. dec:absorb-the-foundation-subset-and-end-the-dependency REQUIRES "
            "one in every absorbed module — see crates/reflow2-core/src/foundation/mod.rs"
        )
    if len(tags) > 1:
        detail = "; ".join(f"{t}: {', '.join(f)}" for t, f in sorted(tags.items()))
        sys.exit(
            "FAIL: absorbed modules name DIFFERENT foundation tags "
            f"({detail}) — code taken from mixed vintages of one repository"
        )
    return next(iter(tags))


# --------------------------------------------------------------------------
# The claims. Each is (file, label, regex, expected). The regex must capture the
# version in group 1, and must be anchored on a literal phrase from the prose so
# that historical mentions nearby are not swept up.
# --------------------------------------------------------------------------


def schema_counts() -> tuple[str, str]:
    """How many node and edge types the schema ACTUALLY declares.

    Added 2026-09-07, when retiring `QualityGate` moved the node count 29 -> 28
    and a sweep found the edge count in the same four sentences had ALREADY been
    wrong for weeks — three said 60 and one said 64 against a real 65. Nothing
    checked them, so they drifted silently in exactly the way this file's header
    describes for versions. Same class, same remedy.

    Read from `schema/*.yaml` rather than from the compiled binary so the check
    runs without a build, like every other claim here.
    """
    import glob

    import yaml

    nodes: set[str] = set()
    edges: set[str] = set()
    for path in glob.glob(str(REPO / "schema" / "*.yaml")):
        with open(path, encoding="utf-8") as fh:
            domain = yaml.safe_load(fh)["schema"]
        nodes |= set((domain.get("node_types") or {}).keys())
        edges |= set((domain.get("edge_types") or {}).keys())
    if not nodes or not edges:
        sys.exit("FAIL: read no node or edge types from schema/*.yaml")
    return str(len(nodes)), str(len(edges))


def claims(reflow2: str, foundation: str) -> list[tuple[str, str, str, str]]:
    node_types, edge_types = schema_counts()
    return [
        (
            "AGENTS.md",
            "foundation provenance prose",
            r"absorbed from it at `(v[0-9]+\.[0-9]+\.[0-9]+)`",
            foundation,
        ),
        (
            "AGENTS.md",
            "current-state shipping version",
            r"\*\*Shipping at v([0-9]+\.[0-9]+\.[0-9]+)\.\*\*",
            reflow2,
        ),
        (
            "AGENTS.md",
            "loop-status shipping version",
            r"is built and shipping as of\s*\n?v([0-9]+\.[0-9]+\.[0-9]+)",
            reflow2,
        ),
        # The schema counts. Each pattern deliberately matches EVERY occurrence
        # in its file, so a second sentence carrying the same number cannot rot
        # while the first stays true — which is how the edge count reached 65
        # while four sentences still said 60 and 64.
        (
            "AGENTS.md",
            "schema node-type count",
            r"\((\d+) node types, \d+ edge types",
            node_types,
        ),
        (
            "AGENTS.md",
            "schema edge-type count",
            r"\(\d+ node types, (\d+) edge types",
            edge_types,
        ),
        (
            "README.md",
            "schema node-type count",
            r"\((\d+) node(?: types?)? (?:and|/) \d+ edge types",
            node_types,
        ),
        (
            "README.md",
            "schema edge-type count",
            r"\(\d+ node(?: types?)? (?:and|/) (\d+) edge types",
            edge_types,
        ),
        (
            "docs/overview.md",
            "schema node-type count",
            r"\((\d+) node types, \d+ edge types\)",
            node_types,
        ),
        (
            "docs/overview.md",
            "schema edge-type count",
            r"\(\d+ node types, (\d+) edge types\)",
            edge_types,
        ),
        (
            "docs/interaction-surfaces.md",
            "schema node-type count",
            r"\((\d+) node types, \d+ edges\)",
            node_types,
        ),
        (
            "docs/interaction-surfaces.md",
            "schema edge-type count",
            r"\(\d+ node types, (\d+) edges\)",
            edge_types,
        ),        # THE SITE (flo2-connector.github.io/reflow2) names the release each page
        # was checked against, so a cut that leaves the site describing an older
        # release fails here instead of being noticed by a reader
        # (req:every-release-brings-reflow2s-site-up-to-date-with-what-it-ships).
        # Moving the line is the cut's promise that the page was read again.
        (
            "site/index.html",
            "site page checked against",
            r"Checked against reflow2 v([0-9]+\.[0-9]+\.[0-9]+)\.",
            reflow2,
        ),
        (
            "site/setup.html",
            "site page checked against",
            r"Checked against reflow2 v([0-9]+\.[0-9]+\.[0-9]+)\.",
            reflow2,
        ),
        (
            "site/hub.html",
            "site page checked against",
            r"Checked against reflow2 v([0-9]+\.[0-9]+\.[0-9]+)\.",
            reflow2,
        ),
        (
            "site/walkthrough.html",
            "site page checked against",
            r"Checked against reflow2 v([0-9]+\.[0-9]+\.[0-9]+)\.",
            reflow2,
        ),
        (
            "site/why.html",
            "site page checked against",
            r"Checked against reflow2 v([0-9]+\.[0-9]+\.[0-9]+)\.",
            reflow2,
        ),
    ]


# --------------------------------------------------------------------------
# COUNTS, FOUND RATHER THAN LISTED (req:a-count-a-current-doc-states-matches-the-build,
# dec:idea-every-number-a-doc-states-is-checked-against-the-build-or-not-stated).
#
# The claims above are a hand-kept table, and that was the hole: on 2026-11-20
# an outside review of v0.101.0 found README saying 165 tools (200 served) and
# 25 served skills (36), and lib.rs saying 60 edge types (71), while this gate
# said "OK — 16 targeted claim(s)". Nobody had added those sentences to the
# table, so nothing read them. Here the gate SCANS every current-state doc for
# a counted claim and checks each one, so a sentence written tomorrow is checked
# the day it is written.
#
# History is not current state. An upgrade note saying what the schema had at
# v0.65, or the original Reflow's tool count, is correct prose about the past,
# so it is named in HISTORY with its reason. A history entry that no longer
# matches anything FAILS, for the reason an unmatched claim does above.
# --------------------------------------------------------------------------

# The documents that describe reflow2 AS IT IS. CHANGELOG, changelog fragments,
# dated records and the design's own facts are history and are not scanned.
CURRENT_DOCS = (
    "README.md",
    "AGENTS.md",
    "SKILLS.md",
    "getting-started/AGENTS.md",
    "docs/overview.md",
    "docs/interaction-surfaces.md",
    "crates/reflow2-core/src/lib.rs",
    "crates/reflow2-mcp/src/lib.rs",
)

# (file, the literal text of the past-tense claim, why it is history).
HISTORY = (
    (
        "docs/overview.md",
        "(66 edge types)",
        "the v0.64.0 upgrade note: what the schema stamp moved to then",
    ),
    (
        "docs/overview.md",
        "all 25 workflows and all 51 tools",
        "the audit of the ORIGINAL Reflow, not reflow2",
    ),
)


def served_tool_count() -> int:
    """The served tool list: one committed golden per tool (tools/toolsnap.py)."""
    n = len(list((REPO / "tools" / "toolsnaps").glob("*.json")))
    if not n:
        sys.exit("FAIL: tools/toolsnaps/ holds no tool goldens to count")
    return n


def served_skill_count() -> int:
    """The skills the binary embeds and serves (crates/reflow2-mcp/build.rs)."""
    n = len([p for p in (REPO / "getting-started" / "skills").glob("*/SKILL.md")])
    if not n:
        sys.exit("FAIL: getting-started/skills/ holds no SKILL.md to count")
    return n


def core_module_count() -> int:
    """The modules reflow2-core's lib.rs declares (`mod x;` / `pub mod x;`)."""
    text = (REPO / "crates" / "reflow2-core" / "src" / "lib.rs").read_text()
    n = len(re.findall(r"^\s*(?:pub(?:\([a-z]+\))?\s+)?mod\s+[a-z_0-9]+\s*;", text, re.M))
    if not n:
        sys.exit("FAIL: read no module declarations from crates/reflow2-core/src/lib.rs")
    return n


def _flattened(text: str) -> str:
    """One line per document, so a count wrapped across lines (a `//!` header,
    a hard-wrapped paragraph) reads as the sentence it is."""
    text = re.sub(r"[ \t]*\n[ \t]*(?://[!/]?[ \t]*)?", " ", text)
    return re.sub(r"[ \t]+", " ", text)


def count_kinds() -> list[tuple[str, str, int]]:
    """(label, pattern capturing the number in group 1, what the build says)."""
    nodes, edges = (int(x) for x in schema_counts())
    return [
        ("tools", r"\b~?(\d{2,4}) (?:MCP )?tools\b", served_tool_count()),
        ("served skills", r"\b(\d{2,3}) (?:served )?skills\b", served_skill_count()),
        ("node types", r"\b(\d{2,3}) node(?: types?)?(?=[ ,/)]|$)", nodes),
        ("edge types", r"\b(\d{2,3}) (?:edge types?\b|edges\))", edges),
        ("modules", r"\b(\d{2,3}) modules\b", core_module_count()),
    ]


def scan_counts() -> tuple[int, list[str]]:
    """Every counted claim in the current-state docs, checked. Returns how many
    were read and what disagreed."""
    failures: list[str] = []
    checked = 0
    kinds = count_kinds()
    for rel in CURRENT_DOCS:
        path = REPO / rel
        if not path.exists():
            failures.append(f"MISSING FILE: {rel} (a current-state doc the count scan reads)")
            continue
        flat = _flattened(path.read_text())
        exempt: list[tuple[int, int]] = []
        for hrel, literal, why in HISTORY:
            if hrel != rel:
                continue
            spans = [(m.start(), m.end()) for m in re.finditer(re.escape(literal), flat)]
            if not spans:
                failures.append(
                    f"HISTORY ENTRY MATCHES NOTHING: {rel} — {literal!r} ({why}). "
                    f"The past-tense sentence was reworded or removed: update or "
                    f"remove its entry in HISTORY in tools/check_doc_versions.py."
                )
            exempt.extend(spans)
        for label, pattern, actual in kinds:
            for m in re.finditer(pattern, flat, re.I):
                if any(a <= m.start() < b for a, b in exempt):
                    continue
                checked += 1
                if int(m.group(1)) != actual:
                    context = flat[max(0, m.start() - 50) : m.end() + 10].strip()
                    failures.append(
                        f"COUNT: {rel} ({label}) — says {m.group(1)}, build says {actual}: "
                        f"…{context}…"
                    )
    return checked, failures


def main() -> int:
    reflow2 = reflow2_version()
    foundation = foundation_tag()
    print(f"reflow2 {reflow2} · absorbed from dynograph-foundation {foundation}\n")

    counted, failures = scan_counts()
    for filename, label, pattern, expected in claims(reflow2, foundation):
        path = REPO / filename
        if not path.exists():
            failures.append(f"MISSING FILE: {filename} (claim '{label}' cannot be checked)")
            continue
        text = path.read_text()
        found = re.findall(pattern, text)

        # The difference from the upstream script: no matches is a FAILURE.
        # A silently-skipped check is indistinguishable from a passing one, and
        # that is exactly how a doc line rots after someone rewords it.
        if not found:
            failures.append(
                f"UNMATCHED: {filename} ({label}) — the pattern found nothing. "
                f"Either the prose was reworded (update the pattern in "
                f"tools/check_doc_versions.py alongside it) or the claim was "
                f"deleted (remove it here). A check that matches nothing is not "
                f"a check that passes."
            )
            continue

        for got in found:
            if got.lstrip("v") != expected.lstrip("v"):
                failures.append(
                    f"DRIFT: {filename} ({label}) — says {got}, build says {expected}"
                )

    if failures:
        print("\n".join(failures))
        print(f"\ncheck_doc_versions: FAILED — {len(failures)} finding(s).")
        return 1

    n = len(claims(reflow2, foundation))
    print(
        f"check_doc_versions: OK — {n} targeted claim(s) and {counted} count(s) "
        f"found in {len(CURRENT_DOCS)} current-state doc(s) agree with the build."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
