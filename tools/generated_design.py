#!/usr/bin/env python3
"""A generated design of realistic size, made from the schema: the stand-in for
reflow2's own design wherever a gate needs a LARGE one.

WHY IT EXISTS. Until 2026-10-30 a copy of reflow2's own design was committed at
docs/design/reflow2/, and two surface gates seeded a server from it because they
need a design big enough to make replies large: `replies_are_bounded.py` (a
reply that outgrows the budget must offer a bound, and the gate FAILS if
nothing overflowed) and `a_reply_is_sent_once.py` (every reply carries one
payload, and the gate FAILS if it measured nothing). The design is hosted on
flo2.io and private, so the copy was retired. These gates are about the SERVED
SURFACE, not about reflow2's design, so they now seed from this.

WHAT IT KEEPS FROM THE REAL ONE, and what it does not. The SIZE and the MIX:
how many nodes of each type, and the 61 (edge type, from type, to type)
triples that carried 67,725 of reflow2's 70,189 edges on 2026-10-30, scaled by
`--scale`. On top of that mix it is made from the schema, so it holds EVERY
node type the schema declares, every declared property somewhere on its type,
values of every shape the schema allows (string, enum, bool, int, float,
list), and every edge type on endpoints the schema allows. What it does not
keep is meaning: its text is generated, so a gate that reads WHAT the design
says (whose name a served text must not carry, what a capability claims) has
no business here and moved to the nightly check of the hosted design.

Deterministic: one seed, one document, byte for byte.

Usage:
    python3 tools/generated_design.py --out /tmp/design.json [--scale 1.0] [--seed 7]

and from a gate: `generated_design.write(path)`. Standard library + PyYAML.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import random
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
SCHEMA_DIR = REPO / "schema"

# How many nodes of each type reflow2's own design held on 2026-10-30. A type
# the schema declares and this table does not name gets FLOOR nodes.
NODE_MIX = {
    "ChangeEvent": 1761, "TemporalFact": 1111, "Decision": 1066, "Snapshot": 928,
    "Artifact": 811, "Fragment": 542, "Verification": 534, "Capability": 491,
    "Requirement": 484, "DesignEpoch": 338, "Release": 110, "Component": 110,
    "Question": 65, "DriftEvent": 40, "DesignRule": 36, "Interface": 23,
    "Resource": 20, "Contributor": 9, "Constraint": 9, "Actor": 3,
    "Environment": 3, "Inference": 3, "Project": 1, "Flow": 1,
}
FLOOR = 3

# The (edge type, from, to) triples that carried 67,725 of its 70,189 edges.
EDGE_MIX = [
    ("INCLUDES", "Release", "Artifact", 37457), ("INCLUDES", "Release", "Component", 9034),
    ("CHANGED", "ChangeEvent", "Artifact", 5319), ("AT_EPOCH", "ChangeEvent", "DesignEpoch", 1574),
    ("AUTHORED_BY", "Decision", "Contributor", 992), ("REALIZES", "Artifact", "Capability", 988),
    ("AT_EPOCH", "Snapshot", "DesignEpoch", 916), ("GOVERNED_BY", "Capability", "Decision", 812),
    ("VERIFIES", "Verification", "Capability", 634), ("CHANGED", "ChangeEvent", "Capability", 629),
    ("SATISFIES", "Capability", "Requirement", 539), ("YIELDED", "Fragment", "Artifact", 528),
    ("AUTHORED_BY", "Artifact", "Contributor", 454), ("HAS_TEMPORAL_FACT", "Capability", "TemporalFact", 437),
    ("AUTHORED_BY", "ChangeEvent", "Contributor", 428), ("ALLOCATED_TO", "Capability", "Component", 415),
    ("AUTHORED_BY", "Requirement", "Contributor", 396), ("IMPLEMENTS", "Artifact", "Verification", 339),
    ("CHANGED", "ChangeEvent", "Verification", 327), ("DEPLOYED_TO", "Release", "Environment", 267),
    ("GOVERNED_BY", "ChangeEvent", "Decision", 264), ("HAS_SNAPSHOT", "Decision", "Snapshot", 251),
    ("INVALIDATES", "ChangeEvent", "TemporalFact", 235), ("CHANGED", "ChangeEvent", "Decision", 214),
    ("CHANGED", "ChangeEvent", "Requirement", 210), ("CHANGED", "ChangeEvent", "Component", 185),
    ("GOVERNED_BY", "Requirement", "Decision", 184), ("HAS_SNAPSHOT", "TemporalFact", "Snapshot", 178),
    ("DEPENDS_ON", "Decision", "Decision", 174), ("HAS_SNAPSHOT", "Capability", "Snapshot", 172),
    ("ANTICIPATES", "Decision", "Decision", 155), ("AUTHORED_BY", "Capability", "Contributor", 153),
    ("EVOLVES_INTO", "Decision", "Requirement", 153), ("CAUSES", "Artifact", "TemporalFact", 151),
    ("HAS_TEMPORAL_FACT", "Component", "TemporalFact", 150), ("PRECEDES", "DesignEpoch", "DesignEpoch", 148),
    ("HAS_TEMPORAL_FACT", "Decision", "TemporalFact", 146), ("AUTHORED_BY", "TemporalFact", "Contributor", 141),
    ("HAS_TEMPORAL_FACT", "Project", "TemporalFact", 125), ("REALIZES", "Artifact", "Component", 125),
    ("AUTHORED_BY", "Verification", "Contributor", 111), ("AT_EPOCH", "Release", "DesignEpoch", 110),
    ("SCHEDULED_FOR", "Requirement", "DesignEpoch", 103), ("GOVERNED_BY", "Component", "Decision", 98),
    ("HAS_TEMPORAL_FACT", "Requirement", "TemporalFact", 98), ("CONTAINS", "Component", "Component", 97),
    ("HAS_SNAPSHOT", "Requirement", "Snapshot", 91), ("MITIGATES", "Decision", "TemporalFact", 89),
    ("DEPENDS_ON", "Requirement", "Requirement", 86), ("ASKS_ABOUT", "Question", "Capability", 83),
    ("ABOUT_ENTITY", "TemporalFact", "Component", 77), ("HAS_TEMPORAL_FACT", "Artifact", "TemporalFact", 76),
    ("VALID_FROM", "TemporalFact", "DesignEpoch", 76), ("DEPENDS_ON", "Component", "Component", 72),
    ("CONTAINS", "Project", "Requirement", 68), ("HAS_SNAPSHOT", "Artifact", "Snapshot", 68),
    ("CHANGED", "ChangeEvent", "TemporalFact", 67), ("DEPENDS_ON", "Decision", "Requirement", 67),
    ("EVOLVES_INTO", "Decision", "Decision", 64), ("EVOLVES_INTO", "Decision", "Capability", 64),
    ("HAS_SNAPSHOT", "Release", "Snapshot", 61),
]
# Every edge type the schema declares gets at least this many, on endpoints it allows.
EDGE_FLOOR = 4

# HOW LOCAL THE LINKS ARE. A real design is clustered: a change touches the
# files of one feature, a decision governs its own neighbourhood. Uniformly
# random links instead make one dense tangle, and on it some analyses are far
# slower than on any real design (measured 2026-10-30, debug build, 8,500 nodes:
# design_regions 417 s and derived_report 241 s), which measures the generator,
# not reflow2. So most links land near their source in id order (within
# WINDOW), and the rest anywhere; a release's manifest (INCLUDES) is never local.
LOCAL_SHARE = 0.85
WINDOW = 12

# HOW FULL A NODE IS. reflow2's own design averaged about five properties a node
# and one an edge; a schema type declares up to thirty. The first node of each
# type carries every property it declares (so each is somewhere), and the rest
# carry what they must and this share of what they may.
OPTIONAL_SHARE = 0.15
EDGE_OPTIONAL_SHARE = 0.1

# The typed-id convention (crates/reflow2-core/src/nodes.rs, ID_PREFIXES).
PREFIX = {
    "Project": "proj", "Requirement": "req", "Constraint": "con", "DesignRule": "rule",
    "Question": "question", "Contributor": "who", "Capability": "cap", "Flow": "flow",
    "Actor": "actor", "Component": "cmp", "Interface": "ifc", "Decision": "dec",
    "Artifact": "art", "Fragment": "frag", "Verification": "ver", "DriftEvent": "drift",
    "Release": "rel", "Environment": "env", "EnvironmentRule": "envrule", "Resource": "res",
    "DesignEpoch": "epoch", "TemporalFact": "fact", "Snapshot": "snap", "ChangeEvent": "chg",
    "ReadinessAssessment": "rdy", "Inference": "inf",
}

# REASONING STAYS GROUNDED. An Inference concludes exactly one node, and a
# premise may not lead back to itself. The generator keeps every premise apart
# from every conclusion, so no chain it draws can close a circle.
REASONING = ("CONCLUDES", "PREMISE_OF")

# PROPERTIES THAT HOLD JSON TEXT hold JSON. A Snapshot's `edges` read as
# "edges 7" stopped derived_report whole ("parse snapshot edges: expected value");
# reflow2 writes these itself, so a real design's always parse.
JSON_TEXT = {
    ("Snapshot", "state"): "{}",
    ("Snapshot", "edges"): "[]",
    ("TemporalFact", "value"): '{"status": "realized"}',
    ("Capability", "inputs"): '["request"]',
    ("Capability", "outputs"): '["reply"]',
    ("Decision", "alternatives"): '["the option not taken"]',
}
# Written only by an operation from a kept record, never by hand: left out.
NOT_GENERATED = {("Verification", "hypothesis_test"), ("Verification", "criteria_at_data")}

# The CHANGED edge's acceptance of a file: set only on a change INTO an
# Artifact, where an import derives the artifact's checksum from it.
ACCEPTANCE = ("checksum_after", "accepted_seq", "checksum_basis")

WORDS = (
    "design record change review owner reason file check build release store "
    "query export import edge node capability requirement decision evidence "
    "interface component budget margin epoch lineage merge layout schema "
    "surface reply bound signpost payload gate trial finding cause fix"
).split()


def load_schema(schema_dir: pathlib.Path = SCHEMA_DIR) -> tuple[dict, dict]:
    """({node type: def}, {edge type: def}) merged over schema/*.yaml."""
    try:
        import yaml
    except ImportError:  # pragma: no cover - CI installs it
        raise SystemExit("generated_design.py needs PyYAML: python3 -m pip install pyyaml")
    nodes: dict = {}
    edges: dict = {}
    for f in sorted(schema_dir.glob("*.yaml")):
        doc = (yaml.safe_load(f.read_text(encoding="utf-8")) or {}).get("schema") or {}
        nodes.update(doc.get("node_types") or {})
        edges.update(doc.get("edge_types") or {})
    return nodes, edges


def _endpoints(v, all_types: list[str]) -> list[str]:
    if v in (None, "*") or v == ["*"]:
        return list(all_types)
    return [v] if isinstance(v, str) else list(v)


class _Gen:
    def __init__(self, seed: int):
        self.r = random.Random(seed)
        self.n = 0

    def text(self, lo: int = 6, hi: int = 60) -> str:
        self.n += 1
        words = [self.r.choice(WORDS) for _ in range(self.r.randint(lo, hi))]
        return f"{' '.join(words).capitalize()} ({self.n})."

    def value(self, prop: str, spec: dict, i: int):
        kind = (spec or {}).get("type", "string")
        if kind == "enum":
            values = spec.get("values") or ["x"]
            return values[i % len(values)]
        if kind == "bool":
            return i % 2 == 0
        if kind == "int":
            # SMALL, as a real design's counts and depths are. An int with no
            # declared range read as anything up to 1,000 once made a claim's
            # `depth` 500, and a region walked to depth 500 is the whole design.
            lo, hi = spec.get("range") or (0, 4)
            return int(lo) + (i % max(1, min(int(hi) - int(lo), 4) + 1))
        if kind == "float":
            lo, hi = (spec.get("range") or (0.0, 100.0))
            return round(float(lo) + (float(hi) - float(lo)) * ((i % 97) / 96), 3)
        if kind == "list:string":
            return [f"{prop}-{i}-{k}" for k in range(1 + i % 3)]
        if kind == "datetime":
            return f"2026-{1 + i % 12:02d}-{1 + i % 28:02d}T00:00:00Z"
        # A string. Prose for the text a reader reads, short tokens for the rest.
        if spec.get("fulltext"):
            return self.text()
        return f"{prop} {i}"


def generate(scale: float = 1.0, seed: int = 7, graph_id: str = "generated") -> dict:
    """The design document: {graph_id, nodes, edges}, ready for import_graph."""
    node_types, edge_types = load_schema()
    all_types = sorted(node_types)
    g = _Gen(seed)
    nodes: list[dict] = []
    by_type: dict[str, list[str]] = {}
    for t in all_types:
        props = (node_types[t] or {}).get("properties") or {}
        count = max(FLOOR if t not in NODE_MIX else 1, round(NODE_MIX.get(t, FLOOR) * scale))
        if t == "Project":
            count = 1
        prefix = PREFIX.get(t, t.lower())
        ids = by_type.setdefault(t, [])
        for i in range(count):
            node_id = f"{prefix}:{t.lower()}-{i:05d}"
            body: dict = {}
            for p, spec in sorted(props.items()):
                spec = spec or {}
                if (t, p) in NOT_GENERATED:
                    continue
                # The first node of a type carries EVERY property, so each
                # declared property is somewhere (see OPTIONAL_SHARE).
                if spec.get("required") or i == 0 or g.r.random() < OPTIONAL_SHARE:
                    body[p] = JSON_TEXT.get((t, p)) or g.value(p, spec, i)
            if t == "Contributor" and i % 3 == 0:
                body["kind"] = "person"  # a person is what the owner-scoped reads ask after
            nodes.append({"node_id": node_id, "node_type": t, "properties": body})
            ids.append(node_id)

    # A PROPERTY THAT NAMES A NODE NAMES A REAL ONE. `node_ref` properties and
    # the ones an edge mirrors (`twin_of`) held text such as "subject_id 7",
    # which named nothing: the import then repaired thousands of mirrored edges
    # and reported every dangling name, and the readers that resolve references
    # did work no real design asks of them. Each now names an existing node of a
    # type its edge allows, its type property agrees, and a mirrored property's
    # edge is drawn from it, so the document is already in the form an export has.
    twin_edges: set[str] = set()
    for t in all_types:
        for p, spec in ((node_types[t] or {}).get("properties") or {}).items():
            twin = (spec or {}).get("twin_of") or {}
            if twin.get("edge"):
                twin_edges.add(twin["edge"])
    twin_drawn: list[tuple[str, str, str, str, str]] = []
    for n in nodes:
        t = n["node_type"]
        for p, spec in ((node_types[t] or {}).get("properties") or {}).items():
            spec = spec or {}
            twin = spec.get("twin_of") or {}
            if not (spec.get("node_ref") or twin):
                continue
            if p not in n["properties"]:
                continue
            edge_def = edge_types.get(twin.get("edge", "")) or {}
            side = "from" if twin.get("names") == "source" else "to"
            allowed = [x for x in _endpoints(edge_def.get(side), all_types) if by_type.get(x)] or [
                x for x in all_types if by_type.get(x)
            ]
            for _ in range(20):  # never itself: draw again
                other_t = allowed[g.r.randrange(len(allowed))]
                other = g.r.choice(by_type[other_t])
                if other != n["node_id"]:
                    break
            else:
                n["properties"].pop(p)
                continue
            n["properties"][p] = other
            if twin.get("type_property"):
                n["properties"][twin["type_property"]] = other_t
            if twin.get("edge"):
                if twin.get("names") == "source":
                    twin_drawn.append((twin["edge"], other, n["node_id"], other_t, t))
                else:
                    twin_drawn.append((twin["edge"], n["node_id"], other, t, other_t))

    edges: list[dict] = []
    seen: set[tuple[str, str, str]] = set()
    concluded_by: set[str] = set()   # Inferences that already conclude something
    conclusions: set[str] = set()    # nodes some Inference concludes
    premises: set[str] = set()       # nodes that are the premise of some Inference
    seq = 0

    def add(et: str, frm: str, to: str, ft: str, tt: str) -> None:
        nonlocal seq
        key = (et, frm, to)
        if frm == to or key in seen:
            return
        if et == "CONCLUDES":
            if frm in concluded_by or to in premises:
                return
            concluded_by.add(frm)
            conclusions.add(to)
        elif et == "PREMISE_OF":
            if frm in conclusions:
                return
            premises.add(frm)
        seen.add(key)
        spec = (edge_types.get(et) or {}).get("properties") or {}
        props: dict = {}
        for p, ps in sorted(spec.items()):
            ps = ps or {}
            if p in ACCEPTANCE and et == "CHANGED":
                continue
            if ps.get("required") or len(seen) % 7 == 0 or g.r.random() < EDGE_OPTIONAL_SHARE:
                if ps.get("node_ref"):
                    props[p] = g.r.choice(nodes)["node_id"]
                else:
                    props[p] = g.value(p, ps, len(seen))
        if et == "CHANGED" and tt == "Artifact" and g.r.random() < 0.5:
            seq += 1
            props["checksum_after"] = f"sha256:{seq:064x}"
            props["accepted_seq"] = seq
            basis = ((spec.get("checksum_basis") or {}).get("values")) or []
            if basis:
                props["checksum_basis"] = basis[seq % len(basis)]
        edges.append({"edge_type": et, "from_id": frm, "to_id": to, "properties": props})

    def near(et: str, froms: list[str], tos: list[str]) -> tuple[str, str]:
        i = g.r.randrange(len(froms))
        # A release's manifest lists most of what it ships: never local.
        if et == "INCLUDES" or g.r.random() >= LOCAL_SHARE:
            return froms[i], g.r.choice(tos)
        centre = i * len(tos) // max(1, len(froms))
        j = min(len(tos) - 1, max(0, centre + g.r.randint(-WINDOW, WINDOW)))
        return froms[i], tos[j]

    # The mirrored edges, drawn from the properties they mirror (see above).
    for et, frm, to, ft, tt in twin_drawn:
        add(et, frm, to, ft, tt)

    for et, ft, tt, n in EDGE_MIX:
        if et in twin_edges:
            continue  # drawn from the property it mirrors, above
        if et not in edge_types or ft not in by_type or tt not in by_type:
            continue
        want = max(1, round(n * scale))
        tries = 0
        made = 0
        while made < want and tries < want * 4:
            tries += 1
            before = len(edges)
            frm, to = near(et, by_type[ft], by_type[tt])
            add(et, frm, to, ft, tt)
            made += len(edges) - before
    for et in sorted(edge_types):
        if et in twin_edges:
            continue
        spec = edge_types[et] or {}
        froms = [t for t in _endpoints(spec.get("from"), all_types) if t in by_type]
        tos = [t for t in _endpoints(spec.get("to"), all_types) if t in by_type]
        if not froms or not tos:
            continue
        for k in range(EDGE_FLOOR):
            ft, tt = froms[k % len(froms)], tos[(k * 7) % len(tos)]
            add(et, g.r.choice(by_type[ft]), g.r.choice(by_type[tt]), ft, tt)
    return {"graph_id": graph_id, "nodes": nodes, "edges": edges}


def write(path: pathlib.Path | str, scale: float = 1.0, seed: int = 7) -> pathlib.Path:
    path = pathlib.Path(path)
    path.write_text(json.dumps(generate(scale, seed), ensure_ascii=False), encoding="utf-8")
    return path


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    ap.add_argument("--out", required=True)
    ap.add_argument("--scale", type=float, default=1.0)
    ap.add_argument("--seed", type=int, default=7)
    a = ap.parse_args()
    out = write(a.out, a.scale, a.seed)
    doc = json.loads(out.read_text(encoding="utf-8"))
    print(f"wrote {out}: {len(doc['nodes'])} nodes, {len(doc['edges'])} edges")
    return 0


if __name__ == "__main__":
    sys.exit(main())
