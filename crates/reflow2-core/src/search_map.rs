//! THE MAP OF A PROOF SEARCH, one page at any size.
//!
//! Three requirements, all Anthony's (2026-11-07), governed by
//! `dec:the-aa-attempts-lessons-are-built-into-reflow2-before-increment-5`
//! after a 16-round proof trial on the Aaronson–Ambainis conjecture, where
//! both arms spent their late rounds on holes whose closing could not close
//! the target and a waiting candidate sat unopened for eleven rounds:
//!
//! - `req:a-proof-hole-records-the-claim-it-serves-and-whether-closing-it-closes-that-claim`
//!   — a hole's decision point says which claim it is the hole for (`hole`),
//!   which claim it serves (`serves`) and what closing it does to that claim
//!   (`closing_it`: `closes`, `narrows` or `restates`). This map reads the
//!   links along the path to the target and says whether closing the hole
//!   could close the target. An unsaid link reads unsaid, never `closes`.
//! - `req:a-proof-round-starts-from-a-one-page-map-of-its-search` — one read
//!   that fits one reply at any size: the target and how it reads, the tree
//!   of holes under the claims they serve with each hole's routes (open with
//!   its refuter, failed in one line, waiting with its reason), each closed
//!   result in one line, and the rounds since the last progress that served
//!   the target. The arguments (a step's warrant and premises, a route's
//!   prose) are NOT in it: they are fetched on demand by id.
//! - `req:a-proof-search-checks-convergence-and-goes-back-to-the-highest-fork-with-an-untried-branch`
//!   — the rounds since the last progress that served the target are
//!   COMPUTED, and the forks that hold an untried (waiting) branch are listed,
//!   highest first, for the served skill work-a-proof-hole to go back to.
//! - `req:a-proof-search-charts-every-idea-at-each-fork-and-works-the-most-promising-path-by-stated-rank`
//!   — every untried branch anywhere on the chart is ranked: first by what
//!   closing it could do to the target (closes, then restates, then unsaid,
//!   then narrows), then by its stated rank, then by the higher fork. The
//!   convergence count says WHEN to switch; this order says WHICH branch is
//!   next. Each fork shows its top branches with their rank, the rest by id.
//!
//! # The encoding, proposed for the owner
//!
//! - `dec:a-proof-hole-records-what-it-serves-on-its-decision-point`: the
//!   three fields live on the hole's decision point, the Decision
//!   work-a-proof-hole already opens for each hole, as PROPERTIES, never
//!   edges — the reason `route_to` is one ([`crate::failed_route`]): no truth,
//!   no what-if and no structural detector crosses them.
//! - `dec:a-proof-routes-state-is-read-from-its-records-and-its-rank-is-stated-on-it`: a
//!   route is a registered alternative under the hole. It is FAILED when a
//!   failed route `req:route-<slug>` is recorded for `alt:route-<slug>` (the
//!   slug convention the skill writes) or names the hole's claim in
//!   `route_to`; WAITING while it says `waiting: true` (an untried branch,
//!   charted at its fork, with or without a reason); OPEN otherwise, with its
//!   `refuter` beside it. A branch may state its promise (`promise`, a short
//!   line, and `promise_rank`, a rough rank from 1), which the map orders by
//!   (req:a-proof-search-charts-every-idea-at-each-fork-and-works-the-most-promising-path-by-stated-rank).
//! - `dec:a-proof-round-is-a-search-round-finding-about-the-target`: a round
//!   is counted by the one record the skill writes when a round ends, a
//!   finding (`record_finding`) about the target with `fact_type`
//!   `search_round`, and is ORDERED by the creation stamp the server records
//!   on every node — never by a date, a session or a number the agent writes.
//!   Round k holds the records created after the (k−1)th marker and up to the
//!   kth; what comes after the last marker is the round in progress.
//!
//! # What counts as progress that served the target
//!
//! A step that HOLDS, or a failed route, on a hole whose closing could close
//! the target (every link from it to the target `closes` or `restates`): a
//! step in the reasoning under that hole's claim, or a failed route toward
//! that claim or a claim beneath it. Any closed step or failed route, as a
//! round gate counting activity would see it, is NOT this signal: work on a
//! special case that only narrows the target moves nothing here.
//!
//! # Honest limits
//!
//! - A step is credited to the round its record was CREATED in. A step that
//!   comes to hold later, when a premise beneath it is shown, is credited
//!   through that premise's own step, in the later round.
//! - A record without a creation stamp cannot be placed in a round; while
//!   any progress record is unplaced the count reads `null`, and names them.
//! - Whether a route's mathematics is right, and whether a hole's
//!   `closing_it` is true, are the writer's word: reflow2 reads the form.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use serde::Serialize;
use serde_json::{Value as Json, json};

use crate::failed_route::is_failed_route;
use crate::foundation::core::{DynoError, Value};
use crate::graph::DesignGraph;
use crate::inference::{Truth, statement_kind};
use crate::nodes::{edge, node};

/// The `fact_type` of the finding that marks the end of one round.
pub const ROUND_FACT: &str = "search_round";

/// What closing a hole may do to the claim it serves (`closing_it`).
pub const CLOSING_IT: [&str; 3] = ["closes", "narrows", "restates"];

/// The prefix a route registered under a hole carries, and the one its
/// failed route carries: `alt:route-<slug>` and `req:route-<slug>`.
pub const ROUTE_ALT_PREFIX: &str = "alt:route-";
pub const ROUTE_FAILED_PREFIX: &str = "req:route-";

/// How many characters a statement, a reason or a name keeps in the map.
pub const SHORT_CHARS: usize = 160;

/// How many waiting branches each fork shows; the rest are listed by id.
pub const WAITING_SHOWN: usize = 3;

/// How many of the most promising untried branches the map lists.
pub const BRANCHES_SHOWN: usize = 5;

/// How many records one reasoning walk under a claim reads at most.
const TREE_CAP: usize = 2000;

/// The default reply budget, as the other reads take it.
pub const MAP_DEFAULT_BUDGET: usize = 30_000;

const FETCH: &str = "THE ARGUMENTS ARE FETCHED ON DEMAND, BY ID, and are never in this map: \
     get_node on a step (`inf:…`) gives its warrant and premises, on a claim its statement \
     whole, on a failed route what was tried, where and why, on a route (`alt:…`) its refuter \
     or reason to wait whole. reasoning_report with `claim` a hole's claim reads the reasoning \
     under it; with `claim` a hole's claim and `map: true` it draws that hole's own subtree. \
     alternatives_for on a hole lists its routes.";

const NOTE: &str = "THE MAP OF THE SEARCH, computed from the record: the target, the holes \
     under the claims they serve (highest first), each hole's routes — open with the result \
     that would refute it, failed with where and why, waiting with its stated promise and rank \
     (the top of each fork; the rest by id) — each closed result in one line, the rounds since \
     the last progress that served the target, the forks that hold an untried branch, and the \
     untried branches anywhere on the chart, most promising first (what closing them could do \
     to the target, then their stated rank, then the higher fork). `can_close_target` reads the links from a hole to the target: \
     `yes` when every link closes or restates, `no` when one only narrows (a special case or a \
     class ruled out: shown, never hidden, and it can seed the general proof), `unsaid` when a \
     link says nothing. Unsaid is never read as closes. A step's argument stays free prose and \
     is not here.";

/// One claim in the map, in one line.
#[derive(Debug, Clone, Serialize)]
pub struct MapClaim {
    pub id: String,
    /// How it reads: its statement reading (`open`, `stated`, `shown`,
    /// `proven`, `conditional` ...), or its computed truth when it says no
    /// statement kind.
    pub reads: String,
    /// What it says, shortened to one line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub says: Option<String>,
    /// True when `says` was cut: get_node reads it whole.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub shortened: bool,
    /// For a closed result: true when no step yet joins it to the target or
    /// to a hole of this search — shown, and not yet used on a path.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub off_path: bool,
}

/// One route under a hole, in one line.
#[derive(Debug, Clone, Serialize)]
pub struct MapRoute {
    /// The route's record: the registered alternative, or for a failed route
    /// the failed route itself.
    pub id: String,
    /// Its name, shortened.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// For a failed route: the alternative it was registered as.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternative: Option<String>,
    /// For an open route: what would refute it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refuter: Option<String>,
    /// For an open route that says no refuter.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub refuter_unsaid: bool,
    /// For a failed route: where it fails and why, in one line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fails: Option<String>,
    /// For a waiting route: its stated rank, 1 the most promising.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<i64>,
    /// Its stated promise: why it is on the chart, in a line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub promise: Option<String>,
}

/// A hole's routes, by state.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MapRoutes {
    pub open: Vec<MapRoute>,
    pub failed: Vec<MapRoute>,
    pub waiting: Vec<MapRoute>,
}

/// One hole of the tree: its decision point, the claim it is the hole for,
/// the claim it serves, and whether closing it could close the target.
#[derive(Debug, Clone, Serialize)]
pub struct MapHole {
    /// The hole's decision point.
    pub id: String,
    /// Its name, shortened.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// How many links lie between it and the target: 0 for the target's own
    /// hole, 1 for a hole serving the target.
    pub depth: usize,
    /// The hole it is drawn under, when one is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// The claim it is the hole for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hole: Option<String>,
    /// How that claim reads now.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hole_reads: Option<String>,
    /// Where `hole` was read from: `hole` (said on the decision point), `its
    /// failed routes` (every failed route of its routes names it in
    /// `route_to`), or `unsaid`.
    pub hole_from: &'static str,
    /// The claim it serves.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serves: Option<String>,
    /// What closing it does to that claim: `closes`, `narrows`, `restates`,
    /// or `unsaid`.
    pub closing_it: &'static str,
    /// Whether closing it could close the target, along the path: `yes`,
    /// `no` or `unsaid`.
    pub can_close_target: &'static str,
    /// True when it could close the target only through a link that
    /// RESTATES: an equivalent form, no smaller a step. Ranked below a path
    /// that closes.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub by_restatement: bool,
    /// The path, in one line.
    pub why: String,
    /// Its routes. Waiting ones: the top of the fork, by rank.
    pub routes: MapRoutes,
    /// Its other waiting routes, by id (alternatives_for reads them whole).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub more_waiting: Vec<String>,
}

/// A hole the map could not place under the target.
#[derive(Debug, Clone, Serialize)]
pub struct UnplacedHole {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub why: String,
}

/// The newest progress that served the target.
#[derive(Debug, Clone, Serialize)]
pub struct ProgressRef {
    /// The round it was made in.
    pub round: usize,
    /// The record: a step (`inf:…`) or a failed route.
    pub record: String,
    /// `step` or `failed_route`.
    pub kind: &'static str,
    /// The hole it served.
    pub hole: String,
}

/// The round still in progress: what has been recorded since the last
/// round ended.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ThisRound {
    /// The progress that served the target, oldest first.
    pub progress: Vec<String>,
}

/// The rounds of the search, counted.
#[derive(Debug, Clone, Serialize)]
pub struct RoundsRead {
    /// How many rounds were recorded as ended (`search_round` findings about
    /// the target).
    pub recorded: usize,
    /// The ended rounds since the last one that made progress serving the
    /// target, newest first. `null` when it cannot be counted, and `reads`
    /// says why; never zero for "nobody said".
    pub since_progress: Option<usize>,
    /// What the count means, in a sentence.
    pub reads: String,
    /// The newest progress in an ended round.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_progress: Option<ProgressRef>,
    /// The round in progress.
    pub this_round: ThisRound,
    /// Records that could not be placed in a round (no creation stamp).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unordered: Vec<String>,
}

/// An untried branch: a waiting route, where it sits, and its promise.
#[derive(Debug, Clone, Serialize)]
pub struct Branch {
    pub id: String,
    /// The fork it sits at.
    pub hole: String,
    pub depth: usize,
    /// What closing its fork could do to the target.
    pub can_close_target: &'static str,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub by_restatement: bool,
    /// Its stated rank, 1 the most promising; absent when unranked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub promise: Option<String>,
}

/// A fork that holds an untried branch.
#[derive(Debug, Clone, Serialize)]
pub struct Fork {
    /// The hole's decision point.
    pub hole: String,
    pub depth: usize,
    pub can_close_target: &'static str,
    /// Its waiting routes, the top of the fork by rank.
    pub waiting: Vec<Branch>,
    /// How many waiting routes it holds in all.
    pub waiting_total: usize,
    /// The open routes at or beneath this fork: the routes it is above.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub above_open_routes: Vec<String>,
}

/// What the map holds, counted (never budgeted away).
#[derive(Debug, Clone, Default, Serialize)]
pub struct MapCounts {
    pub holes: usize,
    pub open: usize,
    pub failed: usize,
    pub waiting: usize,
    pub closed: usize,
    pub not_placed: usize,
}

/// THE MAP OF ONE SEARCH.
#[derive(Debug, Clone, Serialize)]
pub struct SearchMap {
    pub view: &'static str,
    pub target: MapClaim,
    /// The target's own hole, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_hole: Option<String>,
    /// The holes, in tree order: each after the hole it serves, highest
    /// first.
    pub holes: Vec<MapHole>,
    /// Holes the map could not place under the target, and why.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub not_placed: Vec<UnplacedHole>,
    /// Each closed result, one line.
    pub closed: Vec<MapClaim>,
    pub rounds: RoundsRead,
    /// The forks that hold an untried branch, highest first.
    pub forks_with_an_untried_branch: Vec<Fork>,
    /// The untried branches anywhere on the chart, most promising first:
    /// what closing them could do to the target, then their stated rank,
    /// then the higher fork. The first is the branch to work next when the
    /// path being worked stops converging.
    pub untried_branches_by_promise: Vec<Branch>,
    pub counts: MapCounts,
    pub fetch: &'static str,
    pub note: &'static str,
}

/// What `search_map` answers: the map, or — with no target named and more
/// than one search in the design — the targets to choose from.
#[derive(Debug, Clone)]
pub enum SearchMapRead {
    Map(Box<SearchMap>),
    ChooseTarget(Vec<MapClaim>),
}

fn sprop<'a>(props: &'a HashMap<String, Value>, key: &str) -> Option<&'a str> {
    props
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// One line, at most `max` characters: whitespace collapsed, cut at a char
/// boundary, `…` when cut.
pub fn one_line(text: &str, max: usize) -> (String, bool) {
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        return (flat, false);
    }
    let cut: String = flat.chars().take(max.saturating_sub(1)).collect();
    (format!("{}…", cut.trim_end()), true)
}

fn short(text: Option<&str>) -> Option<String> {
    text.map(|t| one_line(t, SHORT_CHARS).0)
}

/// A candidate hole, as read from its decision point.
#[derive(Debug, Clone)]
struct Candidate {
    id: String,
    name: Option<String>,
    hole: Option<String>,
    hole_from: &'static str,
    serves: Option<String>,
    closing_it: Option<&'static str>,
    alternatives: Vec<(String, HashMap<String, Value>)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Can {
    Yes,
    Unsaid,
    No,
}

impl Can {
    fn word(self) -> &'static str {
        match self {
            Can::Yes => "yes",
            Can::No => "no",
            Can::Unsaid => "unsaid",
        }
    }
}

impl DesignGraph {
    /// How a claim reads, in one word: its statement reading, or its
    /// computed truth when it says no statement kind.
    fn map_reads(&self, id: &str) -> Result<String, DynoError> {
        if let Some(r) = self.statement_reading(id)? {
            return Ok(r.reads.to_string());
        }
        let (truth, _) = self.premise_truth(id)?;
        Ok(match truth {
            Truth::Holds => "holds",
            Truth::Fails => "fails",
            Truth::Unknown => "unknown",
        }
        .to_string())
    }

    fn map_claim(&self, id: &str) -> Result<MapClaim, DynoError> {
        let props = self.node_props(id)?.unwrap_or_default();
        let text = sprop(&props, "statement")
            .or_else(|| sprop(&props, "decision"))
            .or_else(|| sprop(&props, "description"))
            .or_else(|| sprop(&props, "name"));
        let (says, shortened) = match text {
            Some(t) => {
                let (s, cut) = one_line(t, SHORT_CHARS);
                (Some(s), cut)
            }
            None => (None, false),
        };
        Ok(MapClaim {
            id: id.to_string(),
            reads: self.map_reads(id)?,
            says,
            shortened,
            off_path: false,
        })
    }

    /// The reasoning beneath a claim: the claims and the steps under it, at
    /// any depth, each step's premises followed down.
    fn reasoning_beneath(
        &self,
        claim: &str,
    ) -> Result<(BTreeSet<String>, BTreeSet<String>), DynoError> {
        let mut claims = BTreeSet::new();
        let mut steps = BTreeSet::new();
        let mut queue = VecDeque::from([claim.to_string()]);
        while let Some(c) = queue.pop_front() {
            if !claims.insert(c.clone()) || claims.len() + steps.len() > TREE_CAP {
                continue;
            }
            for inf in self.inferences_concluding(&c)? {
                if steps.insert(inf.clone()) {
                    for p in self.premises_of(&inf)? {
                        queue.push_back(p);
                    }
                }
            }
        }
        Ok((claims, steps))
    }

    /// Every decision point that reads as a proof hole: one saying `hole`,
    /// `serves` or `closing_it`, or one with a route registered under it as
    /// `alt:route-<slug>`.
    fn hole_candidates(
        &self,
        failed_by_slug: &BTreeMap<String, String>,
    ) -> Result<Vec<Candidate>, DynoError> {
        let mut out = Vec::new();
        for d in self.scan_nodes(node::DECISION)? {
            let p = &d.properties;
            let mut alternatives = Vec::new();
            for e in self.incoming(&d.node_id, Some(edge::GOVERNED_BY))? {
                if let Some(a) = self.get_node(node::ARTIFACT, &e.from_id)? {
                    alternatives.push((a.node_id, a.properties));
                }
            }
            alternatives.sort_by(|a, b| a.0.cmp(&b.0));
            alternatives.dedup_by(|a, b| a.0 == b.0);
            let said = ["hole", "serves", "closing_it"]
                .iter()
                .any(|k| sprop(p, k).is_some());
            let routed = alternatives
                .iter()
                .any(|(id, _)| id.starts_with(ROUTE_ALT_PREFIX));
            if !said && !routed {
                continue;
            }
            let (hole, hole_from) = match sprop(p, "hole") {
                Some(h) => (Some(h.to_string()), "hole"),
                None => {
                    // Read from its routes' failed routes, when all agree.
                    let mut named = BTreeSet::new();
                    for (alt, _) in &alternatives {
                        if let Some(slug) = alt.strip_prefix(ROUTE_ALT_PREFIX)
                            && let Some(f) = failed_by_slug.get(slug)
                            && let Some(c) = self.failed_route_claim(f)?
                        {
                            named.insert(c);
                        }
                    }
                    if named.len() == 1 {
                        (named.into_iter().next(), "its failed routes")
                    } else {
                        (None, "unsaid")
                    }
                }
            };
            let closing_it =
                sprop(p, "closing_it").and_then(|c| CLOSING_IT.iter().copied().find(|k| *k == c));
            out.push(Candidate {
                id: d.node_id.clone(),
                name: short(sprop(p, "name")),
                hole,
                hole_from,
                serves: sprop(p, "serves").map(str::to_string),
                closing_it,
                alternatives,
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// THE MAP OF THE SEARCH UNDER `target` (req:a-proof-round-starts-from-
    /// a-one-page-map-of-its-search). With no target named: the one search
    /// the design holds, or, with several, the targets to choose from.
    pub fn search_map(&self, target: Option<&str>) -> Result<SearchMapRead, DynoError> {
        let _shared = self.reasoning_scope();
        // Failed routes, by the slug of their id and by the claim they are
        // read under.
        let mut failed_by_slug: BTreeMap<String, String> = BTreeMap::new();
        let mut failed_by_claim: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for f in self.failed_route_ids()? {
            if let Some(slug) = f.strip_prefix(ROUTE_FAILED_PREFIX) {
                failed_by_slug.insert(slug.to_string(), f.clone());
            }
            if let Some(c) = self.failed_route_claim(&f)? {
                failed_by_claim.entry(c).or_default().push(f.clone());
            }
        }
        let candidates = self.hole_candidates(&failed_by_slug)?;
        let by_id: BTreeMap<&str, &Candidate> =
            candidates.iter().map(|c| (c.id.as_str(), c)).collect();
        let mut by_hole: BTreeMap<&str, Vec<&Candidate>> = BTreeMap::new();
        for c in &candidates {
            if let Some(h) = &c.hole {
                by_hole.entry(h.as_str()).or_default().push(c);
            }
        }

        let target = match target {
            Some(t) => t.to_string(),
            None => {
                // The roots: the claim of a hole that serves nothing.
                let roots: BTreeSet<String> = candidates
                    .iter()
                    .filter(|c| c.serves.is_none())
                    .filter_map(|c| c.hole.clone())
                    .collect();
                if roots.len() == 1 {
                    roots.into_iter().next().unwrap_or_default()
                } else {
                    let mut out = Vec::new();
                    for r in roots {
                        out.push(self.map_claim(&r)?);
                    }
                    return Ok(SearchMapRead::ChooseTarget(out));
                }
            }
        };
        if self.node_types_holding(&target)?.is_empty() {
            return Err(DynoError::NodeNotFound {
                node_type: "claim".into(),
                node_id: target,
            });
        }

        // The target's own hole: one that is the hole for the target and
        // serves nothing.
        let target_hole: Option<&Candidate> = by_hole
            .get(target.as_str())
            .and_then(|v| v.iter().find(|c| c.serves.is_none()).copied());

        // The claims in the reasoning under the target: a hole serving one of
        // them that has no hole of its own is still this search's.
        let (under_target, _) = self.reasoning_beneath(&target)?;

        // Each candidate's parent, and why one is not placed.
        enum Up<'a> {
            /// The target's own hole.
            Root,
            /// Drawn under another hole.
            Under(&'a Candidate),
            /// Serves the target, which has no hole of its own.
            TopLevel,
            /// Serves a claim in the reasoning under the target that has no
            /// hole of its own: placed, with that link unsaid.
            Via(String),
            /// The hole of a search under another claim: not this map's.
            OtherSearch,
            /// Cannot be placed, and why.
            Stray(String),
        }
        let parent_of = |c: &Candidate| -> Up<'_> {
            match &c.serves {
                None => match c.hole.as_deref() {
                    Some(h) if h == target => match target_hole {
                        Some(t) if t.id == c.id => Up::Root,
                        Some(t) => Up::Under(t),
                        None => Up::Root,
                    },
                    Some(_) => Up::OtherSearch,
                    None => Up::Stray(
                        "it names neither the claim it is the hole for (`hole`) nor the claim it \
                         serves (`serves`), and no failed route of its routes names one"
                            .to_string(),
                    ),
                },
                Some(x) => {
                    if let Some(p) = by_id.get(x.as_str()) {
                        Up::Under(p)
                    } else if x == &target {
                        match target_hole {
                            Some(t) => Up::Under(t),
                            None => Up::TopLevel,
                        }
                    } else if let Some(p) = by_hole
                        .get(x.as_str())
                        .and_then(|v| v.iter().find(|p| p.id != c.id).copied())
                    {
                        Up::Under(p)
                    } else if under_target.contains(x) {
                        Up::Via(x.clone())
                    } else {
                        Up::OtherSearch
                    }
                }
            }
        };

        // Walk each candidate up to the target: its links, or why it stops.
        struct Placed<'a> {
            c: &'a Candidate,
            parent: Option<String>,
            depth: usize,
            can: Can,
            restates: bool,
            why: String,
        }
        let mut placed: Vec<Placed<'_>> = Vec::new();
        let mut not_placed: Vec<UnplacedHole> = Vec::new();
        for c in &candidates {
            let mut links: Vec<(Option<&'static str>, String)> = Vec::new();
            let mut seen = BTreeSet::new();
            let mut at = c;
            let mut first_parent: Option<String> = None;
            // None: placed. Some(None): another search's. Some(Some(why)): stray.
            let stopped: Option<Option<String>> = loop {
                if !seen.insert(at.id.clone()) {
                    break Some(Some("its `serves` chain returns to itself".to_string()));
                }
                match parent_of(at) {
                    Up::Root => break None,
                    Up::TopLevel => {
                        links.push((at.closing_it, target.clone()));
                        break None;
                    }
                    Up::Via(x) => {
                        links.push((at.closing_it, x));
                        links.push((None, target.clone()));
                        break None;
                    }
                    Up::Under(p) => {
                        if let Some(sv) = &at.serves {
                            links.push((at.closing_it, sv.clone()));
                        }
                        if first_parent.is_none() {
                            first_parent = Some(p.id.clone());
                        }
                        at = p;
                    }
                    Up::OtherSearch => break Some(None),
                    Up::Stray(why) => break Some(Some(why)),
                }
            };
            match stopped {
                Some(None) => {}
                Some(Some(why)) => not_placed.push(UnplacedHole {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    why,
                }),
                None => {
                    let can = links
                        .iter()
                        .map(|(k, _)| match k {
                            Some("narrows") => Can::No,
                            Some(_) => Can::Yes,
                            None => Can::Unsaid,
                        })
                        .max()
                        .unwrap_or(Can::Yes);
                    let why = if links.is_empty() {
                        "it is the target's own hole: closing it closes the target".to_string()
                    } else {
                        let said: Vec<String> = links
                            .iter()
                            .map(|(k, x)| match k {
                                Some(k) => format!("{k} {x}"),
                                None => format!("serves {x} (what closing does there is unsaid)"),
                            })
                            .collect();
                        let verdict = match can {
                            Can::Yes => "so closing it could close the target",
                            Can::No => "so closing it cannot close the target",
                            Can::Unsaid => "so whether closing it could close the target is unsaid",
                        };
                        format!("closing it {}: {verdict}", said.join(", which "))
                    };
                    let restates =
                        can == Can::Yes && links.iter().any(|(k, _)| *k == Some("restates"));
                    placed.push(Placed {
                        c,
                        parent: first_parent,
                        depth: links.len(),
                        can,
                        restates,
                        why,
                    });
                }
            }
        }

        // Tree order: each hole after the one it serves; among siblings, the
        // ones that could close the target first.
        let mut children: BTreeMap<Option<String>, Vec<usize>> = BTreeMap::new();
        for (i, p) in placed.iter().enumerate() {
            children.entry(p.parent.clone()).or_default().push(i);
        }
        for v in children.values_mut() {
            v.sort_by(|a, b| {
                let key = |p: &Placed<'_>| (class_of(p.can, p.restates), p.c.id.clone());
                key(&placed[*a]).cmp(&key(&placed[*b]))
            });
        }
        let mut order: Vec<usize> = Vec::new();
        let mut stack: Vec<usize> = children
            .get(&None)
            .map(|v| v.iter().rev().copied().collect())
            .unwrap_or_default();
        let mut visited = BTreeSet::new();
        while let Some(i) = stack.pop() {
            if !visited.insert(i) {
                continue;
            }
            order.push(i);
            if let Some(kids) = children.get(&Some(placed[i].c.id.clone())) {
                for k in kids.iter().rev() {
                    stack.push(*k);
                }
            }
        }

        // Each hole's routes.
        let mut claimed_failed: BTreeSet<String> = BTreeSet::new();
        let mut holes: Vec<MapHole> = Vec::new();
        let mut counts = MapCounts::default();
        let mut failed_of_hole: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut branches: Vec<Branch> = Vec::new();
        let mut waiting_totals: Vec<usize> = Vec::new();
        for &i in &order {
            let p = &placed[i];
            let c = p.c;
            let mut routes = MapRoutes::default();
            for (alt, props) in &c.alternatives {
                let slug = alt.strip_prefix(ROUTE_ALT_PREFIX);
                let failed = slug
                    .and_then(|s| failed_by_slug.get(s))
                    .filter(|f| !claimed_failed.contains(*f))
                    .cloned();
                let name = short(sprop(props, "name"));
                if let Some(f) = failed {
                    claimed_failed.insert(f.clone());
                    failed_of_hole
                        .entry(c.id.clone())
                        .or_default()
                        .push(f.clone());
                    routes.failed.push(MapRoute {
                        id: f.clone(),
                        name,
                        alternative: Some(alt.clone()),
                        refuter: None,
                        refuter_unsaid: false,
                        fails: Some(self.fails_line(&f)?),
                        rank: None,
                        promise: None,
                    });
                } else if props.get("waiting").and_then(Value::as_bool) == Some(true) {
                    routes.waiting.push(MapRoute {
                        id: alt.clone(),
                        name,
                        alternative: None,
                        refuter: None,
                        refuter_unsaid: false,
                        fails: None,
                        rank: props
                            .get("promise_rank")
                            .and_then(Value::as_i64)
                            .filter(|r| *r >= 1),
                        promise: short(sprop(props, "promise")),
                    });
                } else {
                    let refuter = short(sprop(props, "refuter"));
                    routes.open.push(MapRoute {
                        id: alt.clone(),
                        name,
                        alternative: None,
                        refuter_unsaid: refuter.is_none(),
                        refuter,
                        fails: None,
                        rank: None,
                        promise: short(sprop(props, "promise")),
                    });
                }
            }
            // Failed routes toward the hole's claim that no route of it names.
            if let Some(h) = &c.hole {
                for f in failed_by_claim.get(h).cloned().unwrap_or_default() {
                    if claimed_failed.contains(&f) {
                        continue;
                    }
                    claimed_failed.insert(f.clone());
                    failed_of_hole
                        .entry(c.id.clone())
                        .or_default()
                        .push(f.clone());
                    let props = self.node_props(&f)?.unwrap_or_default();
                    routes.failed.push(MapRoute {
                        id: f.clone(),
                        name: short(sprop(&props, "name")),
                        alternative: None,
                        refuter: None,
                        refuter_unsaid: false,
                        fails: Some(self.fails_line(&f)?),
                        rank: None,
                        promise: None,
                    });
                }
            }
            routes.waiting.sort_by_key(|a| by_rank(a.rank, &a.id));
            counts.open += routes.open.len();
            counts.failed += routes.failed.len();
            counts.waiting += routes.waiting.len();
            for w in &routes.waiting {
                branches.push(Branch {
                    id: w.id.clone(),
                    hole: c.id.clone(),
                    depth: p.depth,
                    can_close_target: p.can.word(),
                    by_restatement: p.restates,
                    rank: w.rank,
                    name: w.name.clone(),
                    promise: w.promise.clone(),
                });
            }
            let more_waiting: Vec<String> = routes
                .waiting
                .iter()
                .skip(WAITING_SHOWN)
                .map(|w| w.id.clone())
                .collect();
            let waiting_total = routes.waiting.len();
            routes.waiting.truncate(WAITING_SHOWN);
            let hole_reads = match &c.hole {
                Some(h) => Some(self.map_reads(h)?),
                None => None,
            };
            holes.push(MapHole {
                id: c.id.clone(),
                name: c.name.clone(),
                depth: p.depth,
                parent: p.parent.clone(),
                hole: c.hole.clone(),
                hole_reads,
                hole_from: c.hole_from,
                serves: c.serves.clone(),
                closing_it: c.closing_it.unwrap_or("unsaid"),
                can_close_target: p.can.word(),
                by_restatement: p.restates,
                why: p.why.clone(),
                routes,
                more_waiting,
            });
            waiting_totals.push(waiting_total);
        }
        counts.holes = holes.len();
        counts.not_placed = not_placed.len();

        // The reasoning under the target and under each placed hole's claim.
        let mut beneath: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
        beneath.insert(target.clone(), self.reasoning_beneath(&target)?);
        for h in &holes {
            if let Some(claim) = &h.hole
                && !beneath.contains_key(claim)
            {
                beneath.insert(claim.clone(), self.reasoning_beneath(claim)?);
            }
        }

        // Closed results: every result a holding step concludes — those in
        // the reasoning under the target or a hole, or that a failed route
        // rests on, first; then those no step joins to this search yet,
        // unless they lie under another search's target.
        let mut joined: BTreeSet<String> = BTreeSet::new();
        for (claims, _) in beneath.values() {
            joined.extend(claims.iter().cloned());
        }
        for f in &claimed_failed {
            let props = self.node_props(f)?.unwrap_or_default();
            if let Some(s) = sprop(&props, "fails_at_step") {
                joined.insert(s.to_string());
            }
            for r in crate::inference::list_prop(&props, "route_rests_on") {
                joined.insert(r);
            }
        }
        let mut elsewhere: BTreeSet<String> = BTreeSet::new();
        for c in &candidates {
            if c.serves.is_none()
                && let Some(h) = &c.hole
                && h != &target
            {
                elsewhere.extend(self.reasoning_beneath(h)?.0);
            }
        }
        let mut closed: Vec<MapClaim> = Vec::new();
        let mut off: Vec<MapClaim> = Vec::new();
        for ty in [node::REQUIREMENT, node::CONSTRAINT] {
            for n in self.scan_nodes(ty)? {
                let props = &n.properties;
                if is_failed_route(props) || !is_result_kind(props) {
                    continue;
                }
                let on = joined.contains(&n.node_id);
                if !on && elsewhere.contains(&n.node_id) {
                    continue;
                }
                let Some(r) = self.statement_reading(&n.node_id)? else {
                    continue;
                };
                if !matches!(r.reads, "proven" | "shown" | "conditional" | "supported")
                    || self.inferences_concluding(&n.node_id)?.is_empty()
                {
                    continue;
                }
                let mut line = self.map_claim(&n.node_id)?;
                if on {
                    closed.push(line);
                } else {
                    line.off_path = true;
                    off.push(line);
                }
            }
        }
        closed.sort_by(|a, b| a.id.cmp(&b.id));
        off.sort_by(|a, b| a.id.cmp(&b.id));
        closed.extend(off);
        counts.closed = closed.len();

        // Progress that served the target: on holes whose closing could
        // close it.
        let mut progress: Vec<(String, &'static str, String)> = Vec::new();
        let mut seen_progress = BTreeSet::new();
        for h in holes.iter().filter(|h| h.can_close_target == "yes") {
            let (claims, steps) = h
                .hole
                .as_ref()
                .and_then(|c| beneath.get(c).cloned())
                .unwrap_or_default();
            for s in steps {
                let holds = self
                    .inference_reading(&s)?
                    .is_some_and(|r| r.verdict == Truth::Holds);
                if holds && seen_progress.insert(s.clone()) {
                    progress.push((s, "step", h.id.clone()));
                }
            }
            for f in failed_of_hole.get(&h.id).cloned().unwrap_or_default() {
                if seen_progress.insert(f.clone()) {
                    progress.push((f, "failed_route", h.id.clone()));
                }
            }
            for c in claims {
                for f in failed_by_claim.get(&c).cloned().unwrap_or_default() {
                    if seen_progress.insert(f.clone()) {
                        progress.push((f, "failed_route", h.id.clone()));
                    }
                }
            }
        }
        let rounds = self.rounds_read(&target, target_hole.map(|t| t.id.as_str()), &progress)?;

        // The forks that hold an untried branch, highest first.
        let mut forks = Vec::new();
        for (pos, h) in holes.iter().enumerate() {
            if h.routes.waiting.is_empty() {
                continue;
            }
            let restates = h.by_restatement;
            // The open routes at or beneath it: its subtree in tree order.
            let mut above = Vec::new();
            for sub in holes[pos..].iter().enumerate() {
                let (k, s) = sub;
                if k > 0 && s.depth <= h.depth {
                    break;
                }
                above.extend(s.routes.open.iter().map(|r| r.id.clone()));
            }
            forks.push(Fork {
                hole: h.id.clone(),
                depth: h.depth,
                can_close_target: h.can_close_target,
                waiting: h
                    .routes
                    .waiting
                    .iter()
                    .map(|w| Branch {
                        id: w.id.clone(),
                        hole: h.id.clone(),
                        depth: h.depth,
                        can_close_target: h.can_close_target,
                        by_restatement: restates,
                        rank: w.rank,
                        name: w.name.clone(),
                        promise: w.promise.clone(),
                    })
                    .collect(),
                waiting_total: waiting_totals[pos],
                above_open_routes: above,
            });
        }
        forks.sort_by(|a, b| (a.depth, &a.hole).cmp(&(b.depth, &b.hole)));

        // Every untried branch on the chart, most promising first.
        branches.sort_by_key(branch_key);
        branches.truncate(BRANCHES_SHOWN);

        Ok(SearchMapRead::Map(Box::new(SearchMap {
            view: "search_map",
            target: self.map_claim(&target)?,
            target_hole: target_hole.map(|t| t.id.clone()),
            holes,
            not_placed,
            closed,
            rounds,
            forks_with_an_untried_branch: forks,
            untried_branches_by_promise: branches,
            counts,
            fetch: FETCH,
            note: NOTE,
        })))
    }

    /// A failed route's where and why, in one line.
    fn fails_line(&self, id: &str) -> Result<String, DynoError> {
        let props = self.node_props(id)?.unwrap_or_default();
        let at = sprop(&props, "fails_at")
            .map(str::to_string)
            .or_else(|| sprop(&props, "fails_at_step").map(str::to_string))
            .unwrap_or_else(|| "a step it does not name".to_string());
        let because = sprop(&props, "fails_because").unwrap_or("a reason it does not say");
        Ok(one_line(&format!("at {at}: {because}"), SHORT_CHARS * 2).0)
    }

    /// THE ROUNDS, COUNTED (req:a-proof-search-checks-convergence-and-goes-
    /// back-to-the-highest-fork-with-an-untried-branch, part 1).
    fn rounds_read(
        &self,
        target: &str,
        target_hole: Option<&str>,
        progress: &[(String, &'static str, String)],
    ) -> Result<RoundsRead, DynoError> {
        let mut markers: Vec<(String, Option<String>)> = Vec::new();
        for f in self.scan_nodes(node::TEMPORAL_FACT)? {
            let p = &f.properties;
            if sprop(p, "fact_type") != Some(ROUND_FACT) {
                continue;
            }
            let about = sprop(p, "subject_id");
            if about != Some(target) && (about.is_none() || about != target_hole) {
                continue;
            }
            let created = self
                .node_stamp(node::TEMPORAL_FACT, &f.node_id)?
                .and_then(|s| s.created);
            markers.push((f.node_id.clone(), created));
        }
        let mut unordered: Vec<String> = markers
            .iter()
            .filter(|(_, c)| c.is_none())
            .map(|(id, _)| id.clone())
            .collect();
        let mut ends: Vec<String> = markers.into_iter().filter_map(|(_, c)| c).collect();
        ends.sort();
        let recorded = ends.len() + unordered.len();

        // Each progress record's round: 1 + the markers created before it.
        let mut placed: Vec<(usize, String, String, &'static str, String)> = Vec::new();
        for (id, kind, hole) in progress {
            let ty = if *kind == "step" {
                node::INFERENCE
            } else {
                node::REQUIREMENT
            };
            match self.node_stamp(ty, id)?.and_then(|s| s.created) {
                Some(at) => {
                    let round = 1 + ends.iter().filter(|e| e.as_str() < at.as_str()).count();
                    placed.push((round, at, id.clone(), kind, hole.clone()));
                }
                None => unordered.push(id.clone()),
            }
        }
        placed.sort_by(|a, b| (a.0, &a.1, &a.2).cmp(&(b.0, &b.1, &b.2)));
        let this_round = ThisRound {
            progress: placed
                .iter()
                .filter(|p| p.0 > ends.len())
                .map(|p| p.2.clone())
                .collect(),
        };
        let last = placed.iter().rev().find(|p| p.0 <= ends.len());
        let last_progress = last.map(|p| ProgressRef {
            round: p.0,
            record: p.2.clone(),
            kind: p.3,
            hole: p.4.clone(),
        });
        let (since_progress, reads) = if recorded == 0 {
            (
                None,
                format!(
                    "No round of this search is recorded, so the rounds since progress cannot be \
                     counted. A round is recorded when it ends: record_finding with `fact_type` \
                     `{ROUND_FACT}` about {target}."
                ),
            )
        } else if !unordered.is_empty() {
            (
                None,
                format!(
                    "{} record(s) carry no creation stamp ({}), so they cannot be placed in a \
                     round and the count is not given.",
                    unordered.len(),
                    unordered.join(", ")
                ),
            )
        } else {
            match &last_progress {
                Some(l) => {
                    let since = ends.len() - l.round;
                    (
                        Some(since),
                        format!(
                            "{since} of the {} ended round(s) since the last progress that served \
                             the target ({}, round {}, on {}).",
                            ends.len(),
                            l.record,
                            l.round,
                            l.hole
                        ),
                    )
                }
                None => (
                    Some(ends.len()),
                    format!(
                        "None of the {} ended round(s) made progress that served the target: no \
                         step closed and no route failed on a hole whose closing could close it.",
                        ends.len()
                    ),
                ),
            }
        };
        Ok(RoundsRead {
            recorded,
            since_progress,
            reads,
            last_progress,
            this_round,
            unordered,
        })
    }
}

/// Fit the map to `budget` characters, withholding detail in a fixed order
/// and naming each cut in `page.withheld`. The target, the counts, the rounds
/// and the first forks are never withheld; at the last stage the holes are
/// listed from the top of the tree down as far as they fit, and the rest are
/// counted and reached by drawing a hole's own subtree.
pub fn fit_map(map: &SearchMap, budget: usize) -> Json {
    let mut v = serde_json::to_value(map).unwrap_or(Json::Null);
    let mut withheld: Vec<String> = Vec::new();
    let size = |v: &Json| v.to_string().len();
    // Room for the page block itself.
    let room = budget.saturating_sub(2_500);
    type Stage = (&'static str, fn(&mut Json));
    let stages: [Stage; 7] = [
        (
            "each closed result's statement (get_node on its id reads it whole)",
            |v| {
                if let Some(a) = v["closed"].as_array_mut() {
                    for c in a {
                        if let Some(o) = c.as_object_mut() {
                            o.remove("says");
                            o.remove("shortened");
                        }
                    }
                }
            },
        ),
        (
            "each failed route's line and name (get_node on the failed route reads where and why)",
            |v| for_routes(v, "failed", &["fails", "name", "alternative"]),
        ),
        (
            "each open and waiting route's name, refuter and promise under its hole (get_node \
             on the route; the most promising branches below keep theirs)",
            |v| {
                for_routes(v, "open", &["name", "refuter", "refuter_unsaid", "promise"]);
                for_routes(v, "waiting", &["name", "promise"]);
                if let Some(a) = v["holes"].as_array_mut() {
                    for h in a {
                        if let Some(o) = h.as_object_mut() {
                            o.remove("more_waiting");
                        }
                    }
                }
            },
        ),
        (
            "each hole's name and path below the first level (get_node on the hole)",
            |v| {
                if let Some(a) = v["holes"].as_array_mut() {
                    for h in a {
                        if h["depth"].as_u64().unwrap_or(0) >= 2
                            && let Some(o) = h.as_object_mut()
                        {
                            o.remove("name");
                            o.remove("why");
                        }
                    }
                }
            },
        ),
        (
            "the closed results' list (counted in `counts.closed`; reasoning_report with `claim` \
             a hole's claim reads them)",
            |v| {
                if let Some(o) = v.as_object_mut() {
                    o.insert("closed".into(), json!([]));
                }
            },
        ),
        (
            "every hole's name and path, and the routes' ids under each hole (counted; \
             alternatives_for on a hole lists them)",
            |v| {
                if let Some(a) = v["holes"].as_array_mut() {
                    for h in a {
                        if let Some(o) = h.as_object_mut() {
                            o.remove("name");
                            o.remove("why");
                            let r = &o["routes"];
                            let n = |k: &str| r[k].as_array().map(Vec::len).unwrap_or(0);
                            let counted = json!({"open": n("open"), "failed": n("failed"),
                                                 "waiting": n("waiting")});
                            o.insert("routes".into(), counted);
                        }
                    }
                }
            },
        ),
        (
            "each fork's waiting branches after its first, their names and promises, and the \
             routes each fork is above (`waiting_total` counts them)",
            |v| {
                if let Some(a) = v["forks_with_an_untried_branch"].as_array_mut() {
                    for f in a {
                        if let Some(o) = f.as_object_mut() {
                            if let Some(w) = o.get_mut("waiting").and_then(Json::as_array_mut) {
                                w.truncate(1);
                                for b in w {
                                    if let Some(bo) = b.as_object_mut() {
                                        bo.remove("name");
                                        bo.remove("promise");
                                    }
                                }
                            }
                            o.remove("above_open_routes");
                        }
                    }
                }
            },
        ),
    ];
    for (what, cut) in stages {
        if size(&v) <= room {
            break;
        }
        cut(&mut v);
        withheld.push(what.to_string());
    }
    // Last: list as many holes, then forks, as fit, top of the tree first.
    for (key, what) in [
        (
            "holes",
            "the holes deeper in the tree than fit here: reasoning_report with `claim` a listed \
             hole's claim and `map: true` draws its subtree",
        ),
        (
            "forks_with_an_untried_branch",
            "the lower forks with an untried branch (the highest are listed)",
        ),
    ] {
        if size(&v) <= room {
            break;
        }
        let total = v[key].as_array().map(Vec::len).unwrap_or(0);
        let over = size(&v).saturating_sub(room);
        let mut keep = total;
        let mut freed = 0usize;
        if let Some(a) = v[key].as_array() {
            while keep > 1 && freed < over {
                keep -= 1;
                freed += a[keep].to_string().len() + 1;
            }
        }
        if keep < total {
            if let Some(a) = v[key].as_array_mut() {
                a.truncate(keep);
            }
            withheld.push(format!("{} of {total} {}", total - keep, what));
        }
    }
    let chars = size(&v);
    if let Some(o) = v.as_object_mut() {
        o.insert(
            "page".into(),
            json!({"budget_chars": budget, "chars": chars, "withheld": withheld,
                   "note": "THE MAP FITS ONE REPLY AT ANY SIZE: past the budget it withholds \
                            detail in a fixed order, each cut named here, and never the target, \
                            the counts, the rounds or the top of the tree."}),
        );
    }
    v
}

fn for_routes(v: &mut Json, state: &str, keys: &[&str]) {
    if let Some(a) = v["holes"].as_array_mut() {
        for h in a {
            if let Some(list) = h["routes"][state].as_array_mut() {
                for r in list {
                    if let Some(o) = r.as_object_mut() {
                        for k in keys {
                            o.remove(*k);
                        }
                    }
                }
            }
        }
    }
}

/// What closing a fork could do to the target, as a rank: a path that
/// closes (0), one through a restatement (1), an unsaid one (2), one that
/// only narrows (3). Unsaid ranks below restates so that saying nothing
/// never outranks saying what a hole does.
fn class_of(can: Can, restates: bool) -> u8 {
    match (can, restates) {
        (Can::Yes, false) => 0,
        (Can::Yes, true) => 1,
        (Can::Unsaid, _) => 2,
        (Can::No, _) => 3,
    }
}

/// A waiting branch's place within its fork: ranked first, by rank, then
/// unranked; ties by id.
fn by_rank(rank: Option<i64>, id: &str) -> (bool, i64, String) {
    (rank.is_none(), rank.unwrap_or(0), id.to_string())
}

/// A branch's place on the whole chart: what closing its fork could do to
/// the target, then its stated rank, then the higher fork.
fn branch_key(b: &Branch) -> (u8, bool, i64, usize, String, String) {
    let can = match b.can_close_target {
        "yes" => Can::Yes,
        "no" => Can::No,
        _ => Can::Unsaid,
    };
    let (unranked, rank, _) = by_rank(b.rank, &b.id);
    (
        class_of(can, b.by_restatement),
        unranked,
        rank,
        b.depth,
        b.hole.clone(),
        b.id.clone(),
    )
}

/// Whether a statement is a result the map can list as closed: a theorem,
/// lemma, corollary, proposition or conjecture.
pub fn is_result_kind(props: &HashMap<String, Value>) -> bool {
    matches!(
        statement_kind(props),
        Some("theorem" | "lemma" | "corollary" | "proposition" | "conjecture")
    )
}
