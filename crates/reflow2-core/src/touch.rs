//! WHERE DESIGNS ALREADY TOUCH — computed on exact evidence, before anyone
//! declares it (`req:designs-are-shown-where-they-already-touch-before-anyone-declares-it`,
//! governed by `dec:two-designs-are-linked-on-exact-evidence-and-a-similar-name-is-only-a-candidate`;
//! phase 7's fourth step, `epoch:planned-p7-a-hub-on-one-machine`).
//!
//! A composition namespaces each design, so no two share a node, and a seam
//! between two designs showed only where somebody had declared it. In a hub
//! of fifteen designs nobody declares every seam, so the seam bucket stayed
//! empty exactly where the cross-design bugs are (hxm_program, 2026-10-20).
//! This reads the records the designs ALREADY hold and says where they touch:
//!
//! - **declared**: a typed reference one design records to a node of another
//!   (`xref:<design>:<node>`, [`crate::crosslink`]) — somebody said so;
//! - **computed, on exact evidence only**:
//!   - the same FILE in both: an artifact `location` qualified by the
//!     design's repository (the `source` of a pin naming that design), or a
//!     URL, which qualifies itself; or an equal `checksum`. A bare relative
//!     path is never evidence — README.md, LICENSE and Cargo.toml are in
//!     every repository — and an equal checksum of an empty file, or of a
//!     file every repository carries (a LICENSE), is set aside and counted;
//!   - a PIN (`external_dependency`) in one design naming the other's
//!     graph_id, or its source repository;
//! - **candidates**: interfaces in two designs with closely matching names
//!   and the same stated medium. A similar name is never evidence (a fuzzy
//!   match at 80/100 was measured wrong across one owner's sibling projects,
//!   `fact:defect-whether-a-project-link-is-real-cannot-be-asked`): it is
//!   listed for a person to confirm and is never drawn as an edge.
//!
//! Everything here reads export documents and writes nothing. The composer
//! ([`crate::compose`]) draws each declared and computed link as a REAL edge
//! in its throwaway combined graph, labelled `declared` or `computed`, so the
//! ordinary checks see across designs through it.

use std::collections::{BTreeMap, BTreeSet};

use crate::crosslink::DESIGN_NODE_REFERENCE;
use crate::export::{ExportedNode, GraphExport};
use crate::foundation::core::Value;
use crate::fuzzy::token_sort_ratio;
use crate::nodes::node;

/// The `resource_type` of a pin on another design (`external_dependency`).
const DESIGN_DEPENDENCY: &str = "design-dependency";

/// How a pair of designs is written as one key: `a×b`, in the order given.
pub const PAIR_JOIN: &str = "×";

/// The edge a link is drawn as in a combined graph: a traceability edge, so
/// the structural checks thread designs joined by one.
pub const LINK_EDGE: &str = crate::nodes::edge::DEPENDS_ON;

/// The property on a drawn link edge that says which kind it is.
pub const LINK_PROPERTY: &str = "link";

/// The digests of EMPTY content (sha256, sha1, md5): equal in every design
/// that holds an empty file, so never evidence of the same file.
const EMPTY_DIGESTS: [&str; 3] = [
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "da39a3ee5e6b4b0d3255bfef95601890afd80709",
    "d41d8cd98f00b204e9800998ecf8427e",
];

/// File names nearly every repository carries, often byte for byte (a
/// stock LICENSE, an empty `__init__.py`): an equal checksum on one says the
/// CONTENT is common, not that two designs hold the same file. Matched on the
/// last path segment, case-insensitively; the first group by the part before
/// the first `.` (LICENSE, LICENSE.md, LICENSE-MIT).
const COMMON_STEMS: [&str; 14] = [
    "readme",
    "license",
    "licence",
    "copying",
    "notice",
    "authors",
    "contributors",
    "changelog",
    "changes",
    "history",
    "contributing",
    "code_of_conduct",
    "security",
    "codeowners",
];
const COMMON_NAMES: [&str; 26] = [
    ".gitignore",
    ".gitattributes",
    ".editorconfig",
    ".dockerignore",
    ".pre-commit-config.yaml",
    ".flake8",
    "makefile",
    "dockerfile",
    "manifest.in",
    "cargo.toml",
    "cargo.lock",
    "rust-toolchain.toml",
    "package.json",
    "package-lock.json",
    "yarn.lock",
    "pyproject.toml",
    "setup.py",
    "setup.cfg",
    "requirements.txt",
    "requirements-dev.txt",
    "tox.ini",
    "conftest.py",
    "__init__.py",
    "go.mod",
    "go.sum",
    ".python-version",
];

/// One design the touch is computed over.
#[derive(Debug, Clone, Copy)]
pub struct TouchDesign<'a> {
    /// The name its links are reported under — unique in the set (a graph_id,
    /// as a rule; a composition's namespace where one was given).
    pub key: &'a str,
    /// The design's own graph_id, which a pin names.
    pub graph_id: &'a str,
    pub export: &'a GraphExport,
}

/// One place two designs touch: declared by a reference one holds, or
/// computed from exact evidence. Ids are each design's own.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TouchLink {
    /// `declared` (a reference somebody recorded) or `computed` (evidence the
    /// records hold). Never a name match: those are [`NameCandidate`]s.
    pub link: &'static str,
    /// `cross_design_reference`, `same_location`, `same_checksum`,
    /// `pin_names_graph_id` or `pin_names_repository`.
    pub evidence: &'static str,
    pub from_design: String,
    pub from_id: String,
    #[serde(skip)]
    pub from_type: String,
    pub to_design: String,
    /// The node it reaches in the other design. For a pin, which names a
    /// whole design, the node that design's structure is most connected
    /// through (its `anchor`).
    pub to_id: String,
    #[serde(skip)]
    pub to_type: String,
    /// What the evidence is, in words.
    pub detail: String,
    /// In a composition, whether it was drawn as an edge in the combined
    /// graph: `false` when the node it reaches is not in the other design's
    /// record (a stale reference), or that design has no node to anchor a pin
    /// on — reported either way. Absent where nothing is drawn (a pairing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drawn: Option<bool>,
}

/// Two interfaces in two designs whose names closely match and whose stated
/// medium is the same — for a person to confirm. Never an edge, never read by
/// a check.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NameCandidate {
    pub a_design: String,
    pub a_id: String,
    pub b_design: String,
    pub b_id: String,
    pub a_name: String,
    pub b_name: String,
    pub medium: String,
    /// 0–100 name similarity — never an assertion that they are one contract.
    pub name_score: u32,
}

/// Matches that were looked at and are NOT evidence, counted so a reader can
/// see what was set aside and why.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SetAside {
    /// The same relative path in two designs that no shared repository
    /// qualifies (no pin says where either lives, or they live in different
    /// repositories): README.md is in every repository.
    pub bare_path_matches: usize,
    /// A few of those paths, as examples.
    pub bare_path_examples: Vec<String>,
    /// An equal checksum on an empty file or on a file every repository
    /// carries (LICENSE, __init__.py ...): common content, not one file.
    pub common_content_checksums: usize,
}

/// Where one pair of designs stands.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PairTouch {
    pub a: String,
    pub b: String,
    /// `linked` (at least one declared or computed link), `candidates_only`
    /// (no link, but names to confirm) or `never_touch` (checked, nothing).
    pub status: &'static str,
    pub declared: usize,
    pub computed: usize,
    pub candidates: usize,
}

/// Where a set of designs already touch.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Touching {
    pub links: Vec<TouchLink>,
    pub candidates: Vec<NameCandidate>,
    pub set_aside: SetAside,
    /// Each design's repository, as the pins naming it state it (normalised),
    /// where any does. A design no pin names has none known here.
    pub repositories: BTreeMap<String, Vec<String>>,
}

impl Touching {
    /// Every pair of the designs (in the order given), with its status.
    pub fn pairs(&self, keys: &[&str]) -> Vec<PairTouch> {
        let mut out = Vec::new();
        for (i, a) in keys.iter().enumerate() {
            for b in &keys[i + 1..] {
                let same = |x: &str, y: &str| (x == *a && y == *b) || (x == *b && y == *a);
                let declared = self
                    .links
                    .iter()
                    .filter(|l| l.link == "declared" && same(&l.from_design, &l.to_design))
                    .count();
                let computed = self
                    .links
                    .iter()
                    .filter(|l| l.link == "computed" && same(&l.from_design, &l.to_design))
                    .count();
                let candidates = self
                    .candidates
                    .iter()
                    .filter(|c| same(&c.a_design, &c.b_design))
                    .count();
                let status = if declared + computed > 0 {
                    "linked"
                } else if candidates > 0 {
                    "candidates_only"
                } else {
                    "never_touch"
                };
                out.push(PairTouch {
                    a: (*a).to_string(),
                    b: (*b).to_string(),
                    status,
                    declared,
                    computed,
                    candidates,
                });
            }
        }
        out
    }
}

fn text<'a>(n: &'a ExportedNode, k: &str) -> Option<&'a str> {
    n.properties
        .get(k)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// A pin's source as a repository, `host/owner/name`, lower case, or `None`
/// when it does not name one (a registry such as `crates.io`, a path).
pub fn normalise_repository(source: &str) -> Option<String> {
    let mut s = source.trim().to_lowercase();
    if let Some(rest) = s.strip_prefix("git+") {
        s = rest.to_string();
    }
    // scp form: git@host:owner/name
    if !s.contains("://")
        && let Some((user_host, path)) = s.split_once(':')
        && user_host.contains('@')
    {
        let host = user_host.rsplit('@').next().unwrap_or("");
        s = format!("{host}/{path}");
    }
    if let Some((_, rest)) = s.split_once("://") {
        s = rest.to_string();
    }
    // user@host/...
    if let Some((before, after)) = s.split_once('/')
        && let Some((_, host)) = before.rsplit_once('@')
    {
        s = format!("{host}/{after}");
    }
    let s = s.trim_end_matches('/');
    let s = s.strip_suffix(".git").unwrap_or(s).trim_end_matches('/');
    let (host, path) = s.split_once('/')?;
    let is_host = host.contains('.')
        && host.split('.').all(|part| {
            !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        });
    if !is_host || path.trim_matches('/').is_empty() {
        return None;
    }
    Some(s.to_string())
}

/// An artifact location as one of three things.
enum Location {
    /// A URL: it says which repository or site it is in by itself.
    Url(String),
    /// A path relative to its design's repository.
    Relative(String),
    /// A path on somebody's machine, or nothing usable.
    Unusable,
}

fn location(raw: &str) -> Location {
    let raw = raw.trim();
    if raw.is_empty() {
        return Location::Unusable;
    }
    if raw.contains("://") {
        let lower = raw.to_lowercase();
        let rest = lower.split_once("://").map(|(_, r)| r).unwrap_or(&lower);
        return Location::Url(rest.trim_end_matches('/').to_string());
    }
    let path = raw.replace('\\', "/");
    let is_absolute =
        path.starts_with('/') || path.starts_with('~') || path.as_bytes().get(1) == Some(&b':');
    if is_absolute {
        return Location::Unusable;
    }
    let mut p = path.as_str();
    while let Some(rest) = p.strip_prefix("./") {
        p = rest;
    }
    let p = p.trim_end_matches('/');
    if p.is_empty() || p.starts_with("../") {
        return Location::Unusable;
    }
    Location::Relative(p.to_string())
}

fn digest(raw: &str) -> Option<String> {
    let lower = raw.trim().to_lowercase();
    let d = match lower.split_once(':') {
        Some((algo, d)) if algo.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') => d,
        _ => lower.as_str(),
    };
    let d = d.trim();
    (!d.is_empty()).then(|| d.to_string())
}

fn is_common_file(location: Option<&str>) -> bool {
    let Some(loc) = location else {
        return false;
    };
    let name = loc
        .trim_end_matches('/')
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .to_lowercase();
    let stem = name.split('.').next().unwrap_or("");
    COMMON_NAMES.contains(&name.as_str())
        || COMMON_STEMS.contains(&stem)
        || COMMON_STEMS
            .iter()
            .any(|s| stem.starts_with(&format!("{s}-")))
}

/// The node a design's structure is most connected through: the node of the
/// design network with the most traceability edges, ties to the smallest id.
/// A pin names a whole design, and this is the node that stands for it.
fn anchor(doc: &GraphExport) -> Option<(String, String)> {
    let member = |n: &ExportedNode| {
        !crate::structure::NON_DESIGN_TYPES.contains(&n.node_type.as_str())
            && !crate::structure::is_review_record(&n.node_id)
    };
    let types: BTreeMap<&str, &str> = doc
        .nodes
        .iter()
        .filter(|n| member(n))
        .map(|n| (n.node_id.as_str(), n.node_type.as_str()))
        .collect();
    let mut degree: BTreeMap<&str, usize> = types.keys().map(|k| (*k, 0)).collect();
    for e in &doc.edges {
        if !crate::nodes::is_traceability_edge(&e.edge_type)
            || !types.contains_key(e.from_id.as_str())
            || !types.contains_key(e.to_id.as_str())
        {
            continue;
        }
        *degree.entry(e.from_id.as_str()).or_default() += 1;
        *degree.entry(e.to_id.as_str()).or_default() += 1;
    }
    // Max degree, ties to the smallest id (BTreeMap iterates ids in order,
    // and only a strictly larger degree replaces the one held).
    let mut best: Option<(&str, usize)> = None;
    for (id, d) in degree {
        if best.is_none_or(|(_, b)| d > b) {
            best = Some((id, d));
        }
    }
    best.map(|(id, _)| (id.to_string(), types[id].to_string()))
}

/// Where these designs already touch: declared references, links computed on
/// exact evidence, and name candidates. Reads the documents; writes nothing.
pub fn where_designs_touch(designs: &[TouchDesign<'_>]) -> Touching {
    let mut out = Touching::default();
    let by_graph_id: BTreeMap<&str, usize> = designs
        .iter()
        .enumerate()
        .map(|(i, d)| (d.graph_id, i))
        .collect();
    let node_type = |i: usize, id: &str| -> Option<String> {
        designs[i]
            .export
            .nodes
            .iter()
            .find(|n| n.node_id == id)
            .map(|n| n.node_type.clone())
    };

    // Each design's pins, and each design's repository as the pins naming it
    // state it.
    struct PinRec {
        id: String,
        graph_id: Option<String>,
        source: String,
        repository: Option<String>,
    }
    let pins: Vec<Vec<PinRec>> = designs
        .iter()
        .map(|d| {
            d.export
                .nodes
                .iter()
                .filter(|n| {
                    n.node_type == node::RESOURCE
                        && text(n, "resource_type") == Some(DESIGN_DEPENDENCY)
                })
                .map(|n| {
                    let source = text(n, "provider").unwrap_or("").to_string();
                    PinRec {
                        id: n.node_id.clone(),
                        graph_id: text(n, "dependency_graph_id").map(str::to_string),
                        repository: normalise_repository(&source),
                        source,
                    }
                })
                .collect()
        })
        .collect();
    let mut repos: Vec<BTreeSet<String>> = vec![BTreeSet::new(); designs.len()];
    for list in &pins {
        for p in list {
            if let (Some(g), Some(r)) = (&p.graph_id, &p.repository)
                && let Some(&j) = by_graph_id.get(g.as_str())
            {
                repos[j].insert(r.clone());
            }
        }
    }
    for (i, d) in designs.iter().enumerate() {
        if !repos[i].is_empty() {
            out.repositories
                .insert(d.key.to_string(), repos[i].iter().cloned().collect());
        }
    }
    let anchors: Vec<Option<(String, String)>> = designs.iter().map(|d| anchor(d.export)).collect();

    // 1. DECLARED: a reference one design records to a node of another.
    for (i, d) in designs.iter().enumerate() {
        for n in &d.export.nodes {
            if n.node_type != node::RESOURCE
                || text(n, "resource_type") != Some(DESIGN_NODE_REFERENCE)
            {
                continue;
            }
            let (Some(far), Some(far_node)) =
                (text(n, "design_graph_id"), text(n, "design_node_id"))
            else {
                continue;
            };
            let Some(&j) = by_graph_id.get(far) else {
                continue;
            };
            if j == i {
                continue;
            }
            let to_type = node_type(j, far_node);
            out.links.push(TouchLink {
                link: "declared",
                evidence: "cross_design_reference",
                from_design: d.key.to_string(),
                from_id: n.node_id.clone(),
                from_type: n.node_type.clone(),
                to_design: designs[j].key.to_string(),
                to_id: far_node.to_string(),
                to_type: to_type.clone().unwrap_or_default(),
                detail: match to_type {
                    Some(_) => format!(
                        "{} records a reference to {far_node} in {}",
                        d.key, designs[j].key
                    ),
                    None => format!(
                        "{} records a reference to {far_node} in {}, which that design's \
                         record no longer holds (stale)",
                        d.key, designs[j].key
                    ),
                },
                drawn: None,
            });
        }
    }

    // 2. PINS naming another design by graph_id, or by its repository.
    for (i, d) in designs.iter().enumerate() {
        for p in &pins[i] {
            let mut targets: Vec<(usize, &'static str, String)> = Vec::new();
            if let Some(g) = &p.graph_id
                && let Some(&j) = by_graph_id.get(g.as_str())
                && j != i
            {
                targets.push((
                    j,
                    "pin_names_graph_id",
                    format!("{}'s pin {} names design {g}", d.key, p.id),
                ));
            }
            if let Some(r) = &p.repository {
                for (j, set) in repos.iter().enumerate() {
                    if j == i || !set.contains(r) || targets.iter().any(|(t, _, _)| *t == j) {
                        continue;
                    }
                    targets.push((
                        j,
                        "pin_names_repository",
                        format!(
                            "{}'s pin {} comes from {}, the repository {} lives in",
                            d.key, p.id, p.source, designs[j].key
                        ),
                    ));
                }
            }
            for (j, evidence, detail) in targets {
                let (to_id, to_type) = anchors[j].clone().unwrap_or_default();
                out.links.push(TouchLink {
                    link: "computed",
                    evidence,
                    from_design: d.key.to_string(),
                    from_id: p.id.clone(),
                    from_type: node::RESOURCE.to_string(),
                    to_design: designs[j].key.to_string(),
                    drawn: None,
                    detail: if to_id.is_empty() {
                        format!(
                            "{detail}; that design holds no node to anchor it on, so it is \
                             reported and not drawn"
                        )
                    } else {
                        detail
                    },
                    to_id,
                    to_type,
                });
            }
        }
    }

    // 3. THE SAME FILE: a location one repository qualifies (or a URL), or
    //    an equal checksum on content that is not common to every repository.
    struct Art<'a> {
        design: usize,
        id: &'a str,
        location: Option<&'a str>,
    }
    let mut by_place: BTreeMap<String, Vec<Art<'_>>> = BTreeMap::new();
    let mut by_bare: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    let mut by_digest: BTreeMap<String, Vec<Art<'_>>> = BTreeMap::new();
    for (i, d) in designs.iter().enumerate() {
        for n in &d.export.nodes {
            if n.node_type != node::ARTIFACT {
                continue;
            }
            let loc = text(n, "location");
            match loc.map(location) {
                Some(Location::Url(u)) => {
                    by_place.entry(format!("url {u}")).or_default().push(Art {
                        design: i,
                        id: &n.node_id,
                        location: loc,
                    })
                }
                Some(Location::Relative(p)) => {
                    by_bare.entry(p.clone()).or_default().insert(i);
                    for r in &repos[i] {
                        by_place.entry(format!("{r} {p}")).or_default().push(Art {
                            design: i,
                            id: &n.node_id,
                            location: loc,
                        });
                    }
                }
                _ => {}
            }
            if let Some(h) = text(n, "checksum").and_then(digest) {
                by_digest.entry(h).or_default().push(Art {
                    design: i,
                    id: &n.node_id,
                    location: loc,
                });
            }
        }
    }
    let mut seen: BTreeSet<(usize, String, usize, String)> = BTreeSet::new();
    let mut linked_paths: BTreeSet<(usize, usize, String)> = BTreeSet::new();
    let mut push_file =
        |out: &mut Touching, a: &Art<'_>, b: &Art<'_>, evidence: &'static str, detail: String| {
            let key = (a.design, a.id.to_string(), b.design, b.id.to_string());
            if !seen.insert(key) {
                return;
            }
            out.links.push(TouchLink {
                link: "computed",
                evidence,
                from_design: designs[a.design].key.to_string(),
                from_id: a.id.to_string(),
                from_type: node::ARTIFACT.to_string(),
                to_design: designs[b.design].key.to_string(),
                to_id: b.id.to_string(),
                to_type: node::ARTIFACT.to_string(),
                detail,
                drawn: None,
            });
        };
    for (place, arts) in &by_place {
        for (x, a) in arts.iter().enumerate() {
            for b in &arts[x + 1..] {
                if a.design == b.design {
                    continue;
                }
                let (a, b) = if a.design < b.design { (a, b) } else { (b, a) };
                let shown = place.split_once(' ').map(|(_, p)| p).unwrap_or(place);
                let repo = place.split_once(' ').map(|(r, _)| r).unwrap_or("");
                if let Some(Location::Relative(p)) = a.location.map(location) {
                    linked_paths.insert((a.design, b.design, p));
                }
                push_file(
                    &mut out,
                    a,
                    b,
                    "same_location",
                    if repo == "url" {
                        format!("both record the file at {shown}")
                    } else {
                        format!("both record {shown} in the repository {repo}")
                    },
                );
            }
        }
    }
    for (path, ds) in &by_bare {
        let ds: Vec<usize> = ds.iter().copied().collect();
        for (x, a) in ds.iter().enumerate() {
            for b in &ds[x + 1..] {
                if linked_paths.contains(&(*a, *b, path.clone())) {
                    continue;
                }
                out.set_aside.bare_path_matches += 1;
                if out.set_aside.bare_path_examples.len() < 5
                    && !out.set_aside.bare_path_examples.contains(path)
                {
                    out.set_aside.bare_path_examples.push(path.clone());
                }
            }
        }
    }
    for (h, arts) in &by_digest {
        for (x, a) in arts.iter().enumerate() {
            for b in &arts[x + 1..] {
                if a.design == b.design {
                    continue;
                }
                if EMPTY_DIGESTS.contains(&h.as_str())
                    || is_common_file(a.location)
                    || is_common_file(b.location)
                {
                    out.set_aside.common_content_checksums += 1;
                    continue;
                }
                let (a, b) = if a.design < b.design { (a, b) } else { (b, a) };
                let short: String = h.chars().take(16).collect();
                push_file(
                    &mut out,
                    a,
                    b,
                    "same_checksum",
                    format!("both record content with checksum {short}…"),
                );
            }
        }
    }

    // 4. CANDIDATES: interfaces whose names closely match, with the same
    //    stated medium. Listed, never drawn.
    let interfaces: Vec<Vec<(&str, String, String)>> = designs
        .iter()
        .map(|d| {
            d.export
                .nodes
                .iter()
                .filter(|n| n.node_type == node::INTERFACE)
                .filter_map(|n| {
                    let medium = text(n, "medium").filter(|m| *m != "unspecified")?;
                    let name = text(n, "name").unwrap_or(&n.node_id).to_string();
                    Some((n.node_id.as_str(), name, medium.to_lowercase()))
                })
                .collect()
        })
        .collect();
    for i in 0..designs.len() {
        for j in i + 1..designs.len() {
            for (a_id, a_name, a_medium) in &interfaces[i] {
                for (b_id, b_name, b_medium) in &interfaces[j] {
                    if a_medium != b_medium {
                        continue;
                    }
                    let score = token_sort_ratio(&a_name.to_lowercase(), &b_name.to_lowercase());
                    if score < crate::seam::PAIR_CERTAIN_THRESHOLD {
                        continue;
                    }
                    out.candidates.push(NameCandidate {
                        a_design: designs[i].key.to_string(),
                        a_id: (*a_id).to_string(),
                        b_design: designs[j].key.to_string(),
                        b_id: (*b_id).to_string(),
                        a_name: a_name.clone(),
                        b_name: b_name.clone(),
                        medium: a_medium.clone(),
                        name_score: score,
                    });
                }
            }
        }
    }
    out
}

/// What the reply says about links, the same sentence wherever a composition
/// or a pairing reports them.
pub const LINKS_NOTE: &str = "Where the designs touch is read from what their records already \
     hold. DECLARED: a reference one design records to a node of another. COMPUTED, on exact \
     evidence only: the same file (an artifact location in the same repository, the repository \
     being the source a pin naming that design states, or the same URL; or an equal checksum) \
     and a pin in one design naming the other's graph_id or source repository. A bare relative \
     path is never evidence (README.md is in every repository), and an equal checksum of an \
     empty or stock file is set aside; both are counted in `set_aside`. A similar NAME is never \
     a link: interfaces whose names closely match with the same medium are listed as \
     `candidates` for a person to confirm, never drawn and never read by a check. In \
     `link_status` every pair is `linked`, `candidates_only`, `never_touch` (checked: no \
     evidence either way) or `not_checked` (a design of the pair was not read) — never \
     `agree`.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_repository_reads_alike_however_a_pin_spells_it() {
        for s in [
            "https://github.com/NSLS2/ansible",
            "https://github.com/NSLS2/ansible.git",
            "https://github.com/NSLS2/ansible/",
            "git@github.com:NSLS2/ansible.git",
            "git+https://github.com/NSLS2/ansible",
            "ssh://git@github.com/NSLS2/ansible.git",
            "http://GitHub.com/nsls2/ansible",
        ] {
            assert_eq!(
                normalise_repository(s).as_deref(),
                Some("github.com/nsls2/ansible"),
                "{s}"
            );
        }
        for s in ["crates.io", "", "../ansible", "pypi", "https://github.com"] {
            assert_eq!(normalise_repository(s), None, "{s} names no repository");
        }
    }

    #[test]
    fn a_file_every_repository_carries_is_common_and_a_named_one_is_not() {
        for loc in [
            "README.md",
            "LICENSE",
            "./LICENSE-MIT",
            "docs/README.rst",
            "Cargo.toml",
            "pkg/__init__.py",
            ".gitignore",
            "COPYING",
        ] {
            assert!(is_common_file(Some(loc)), "{loc}");
        }
        for loc in [
            "roles/pdfstream/tasks/main.yml",
            "src/stream.py",
            "licensing.md",
        ] {
            assert!(!is_common_file(Some(loc)), "{loc}");
        }
    }

    #[test]
    fn a_bare_path_an_absolute_path_and_a_url_are_told_apart() {
        assert!(matches!(location("./a/b.py"), Location::Relative(p) if p == "a/b.py"));
        assert!(matches!(location("/home/x/a.py"), Location::Unusable));
        assert!(matches!(location("C:\\x\\a.py"), Location::Unusable));
        assert!(matches!(location("../a.py"), Location::Unusable));
        assert!(
            matches!(location("https://Example.org/a/"), Location::Url(u) if u == "example.org/a")
        );
    }
}
