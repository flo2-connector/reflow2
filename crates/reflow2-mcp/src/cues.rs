//! Cues: the sentences reflow2 offers an agent that name its next call, each
//! with an id, and the count of whether the agent acted on it.
//!
//! `req:each-cue-is-counted-by-whether-the-agent-acted-on-it` (Anthony,
//! 2026-10-29, option (d) of
//! `dec:idea-cues-are-a-message-on-the-interface-between-reflow2-and-the-agent`),
//! built as `cap:each-cue-carries-an-id-and-is-counted-by-whether-it-was-followed`.
//! The baseline it measures against: in the recall pilot, 245 cues named a
//! call and the agent made that call within five calls 9 times (4%)
//! (`fact:in-the-recall-pilot-agents-followed-4-percent-of-reflow2s-cues-within-five-calls-2026-10-29`).
//! Nothing in reflow2 could say that before; it was counted by hand from a
//! wrapper's log.
//!
//! # What a cue is
//!
//! A string under one of a reply's cue fields ([`CUE_FIELDS`], at any depth,
//! never inside a node's `properties`, which are the design's content) that
//! names a served tool, or a served skill to fetch, other than the tool just
//! called. The same definition the baseline used, so the numbers compare.
//! And the agent-side cue the connect-time instructions carry
//! ([`INSTRUCTIONS_OFFER`]), offered once per session at the handshake.
//!
//! # The id, derived rather than assigned
//!
//! `<field>/<target>`: `loop_hint/set_decision_status`, `next/loop_status`,
//! `hint/get_skill:detect-and-ask`, `instructions/graph_report`.
//! Derived at the one place every reply passes, from words that are reflow2's
//! own vocabulary, rather than hand-assigned at the ~40 places a hint is
//! written: an id each emit site had to remember would be the one a new hint
//! forgot. Stable for as long as the field and the call it names are.
//!
//! The ids ride the reply in `_meta` under [`META_KEY`] — protocol metadata a
//! model is not shown, so naming a cue adds nothing to read (cue fatigue is
//! the pushback recorded against cues) — and are written on the call's ledger
//! line ([`crate::usage::UsageLine::cues`]).
//!
//! # Verbs only
//!
//! `req:telemetry-carries-usage-never-design-content`: an id holds a field
//! name and a tool or skill name — reflow2's words — and never the sentence,
//! a node id or an argument. A word is a target only if it is in the served
//! vocabulary passed in, so nothing of the design's own wording can become one.
//!
//! # The `cues` block: one named message (increment 6)
//!
//! `req:a-cue-is-one-named-kind-of-message-in-reflow2s-interface` (Anthony,
//! 2026-10-29, option (a) of the cues idea). Every structured reply that
//! offers a cue carries ONE block, `cues`, at the top level of its
//! `structuredContent` ([`attach_block`]): `{"items": [...], "left_out": n}`,
//! each item `{cue, type, call, args?, size_chars?}`, in rank order. Four
//! types ([`Kind`]): `next_call` (hint, next_step, nudge, carry_it), `owed`
//! (loop_hint and loop_status's `next`: what the loop is owed), `lesson` (a
//! lesson this design holds for the step, withheld from the reply), `related`
//! (a near-match a write found). The fields the cues come from stay exactly
//! as they were: the block is computed from them, beside them, and replaces
//! none (MCP changes are additive; a removal needs a deprecating release).
//!
//! THE COUNTS CHOOSE WHAT GOES IN ([`MEASURED`], [`standing`]): a cue the
//! counts show followed ranks first, by how often it was followed within five
//! calls; one never followed across [`MIN_EVIDENCE`] offers or more is left
//! out of the block (it stays in its field). And THE BLOCK IS HELD TO A SMALL
//! BUDGET ([`BLOCK_MAX_CUES`], [`BLOCK_MAX_CHARS`]) and never pushes a reply
//! past the reply's own budget, because more cues must never mean each one is
//! read less (the CUE FATIGUE pushback recorded against cues).
//!
//! A CUE THAT POINTS AT A READ ASKS FOR IT WITH A BUDGET (`args.budget_chars`
//! [`CUE_READ_BUDGET`], or `args.limit` for a search) or says its size
//! (`size_chars`, a skill's body), and a read that can do neither is left out:
//! measured 2026-10-30, a cued reader with a 12 KB reading budget spent 6.6 KB
//! on the first read the cue named and never reached the design's own
//! current-state record
//! (`fact:the-size-sweep-full-reads-held-at-every-size-and-reflow2-at-12kb-led-then-collapsed-2026-10-30`).

use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// The reply fields whose strings are cues — the four the baseline counted,
/// `next_step`, the latent surface's spelling of the same thing, and
/// `carry_it`, `mint_seat`'s pointer at `claim_region`: the most-followed
/// pointer reflow2 has (38 of 38 runs on flo2.io), uncounted until increment 6
/// decided what the count counts
/// (`dec:idea-what-the-cue-count-counts-reference-hints-out-a-carried-pointer-in`,
/// option C).
pub const CUE_FIELDS: [&str; 6] = [
    "loop_hint",
    "hint",
    "next",
    "nudge",
    "next_step",
    "carry_it",
];

/// A cue field that is REFERENCE TEXT on one tool's reply, never a cue:
/// `describe_schema`'s per-type and per-edge-type `hint` says which tool writes
/// a thing, and nobody is asked to call it. Counted as cues through 0.95.0, it
/// was 105 of the 683 offers in increment 3's sample, followed once (the same
/// decision, option C).
pub const REFERENCE_FIELDS: [(&str, &str); 1] = [("describe_schema", "hint")];

/// Where a reply carries its cue ids: `_meta["reflow2/cues"]`.
pub const META_KEY: &str = "reflow2/cues";

/// How many of a session's next calls count as acting on a cue "promptly":
/// five, the baseline's window.
pub const FOLLOW_WITHIN: usize = 5;

/// At most this many cue ids are kept from one reply.
pub const MAX_PER_REPLY: usize = 24;

/// The first read the connect-time instructions name, and the second
/// (`cap:the-first-read-cue-reaches-the-agent-before-its-first-call`), as the
/// cue ids a handshake offers. Kept beside [`FIRST_READ_CUE`] so the sentence
/// and what is counted for it cannot drift apart unseen: a test holds the
/// sentence to these names.
///
/// THE REPORT'S ID CHANGED ON 2026-10-30, from `instructions/graph_report_markdown`
/// to `instructions/graph_report`: `graph_report_markdown` became a counted
/// refusing stub, folded into `graph_report` with `format: "markdown"`, and a
/// cue id names the call (`<field>/<named call>`), never its arguments. The
/// count reads the two ids as one cue ([`canonical`]).
///
/// AND THE ORDER CHANGED ON 2026-10-31 (increment 6): the search first, the
/// report second, because the counts say so. On flo2.io (0.95.0, 2026-10-29
/// to 10-31) the handshake offered the report 60 times and it was followed 0
/// times in 42 attributable offers, the search 3 in 42; in the recall re-run
/// the 12 KB cued reader skipped the report, followed the search to the
/// design's own state record and scored 16/25; in the size sweep the 12 KB
/// cued reader that read the report first spent 6,575 of its 12,000 bytes on
/// it, never opened the state record and scored 5.5
/// (`fact:the-size-sweep-full-reads-held-at-every-size-and-reflow2-at-12kb-led-then-collapsed-2026-10-30`).
pub const INSTRUCTIONS_OFFER: [&str; 2] =
    ["instructions/search_design", "instructions/graph_report"];

/// The agent-side cue itself: the sentence the connect-time instructions
/// carry, and the kit's session-start hooks repeat (`tools/loop_nudge.py`).
/// It names what reflow2 already serves; reflow2 serves no compact read of a
/// design's CURRENT VALUES, which is recorded as a finding, not built here.
///
/// IT SAYS WHAT EACH READ COSTS, so a reader with a small reading budget can
/// choose: the search is held by its `limit` (about 2.4 KB for ten hits on the
/// sweep's 240-document design), the Markdown report is not budgeted and
/// measured 6,402 to 6,575 bytes there. And when several state records come
/// back, it says which is in force: on that design the search ranked the
/// OLDEST of eleven per-batch summaries first
/// (`fact:root-cause-the-search-for-current-state-ranks-the-oldest-of-several-state-records-first-2026-10-31`).
///
/// HELD SHORT (merged with main 2026-11-21): the whole handshake has to fit a
/// client's 4,096-character cap (`tests/the_handshake_fits_the_clients_cap.rs`,
/// shortened on main 2026-11-01), so the sentence keeps the order, the two
/// costs and which record is in force, and drops the rest; the hooks' copy
/// (`tools/loop_nudge.py`) keeps the longer form.
pub const FIRST_READ_CUE: &str = "TO LEARN WHERE THIS DESIGN STANDS, `search_design` \
     \"current state\" (or \"status as of\") first: about 2 KB for ten hits, and the newest is the \
     one in force. Then, if your reading allows, `graph_report` with `format` `markdown`: about \
     6 KB, unbudgeted.";

/// THE CUES OFFERED ONCE PER BATCH OF WRITES, not on every write
/// (`epoch:planned-the-tool-surface-refined-from-the-counts`, increment 7).
///
/// Each names the step for when a batch of writes LANDS — run the gap or
/// defect check, read what the loop owes, link a settled decision — and
/// through 0.95.0 each was offered on every write of the batch. Measured on
/// 0.94.0 (increment 3's writers): `loop_hint/detect_gaps` offered 97 times
/// and followed 0, `loop_hint/get_skill:detect-and-ask` 75 and 0,
/// `loop_hint/governed_by` 31 and 0, `loop_hint/detect_defects` and
/// `loop_hint/get_skill:check-health` 17 and 0 each, `loop_hint/loop_status`
/// 162 offered, 2 followed within five calls and all 162 only at the end of
/// the round. The tools are called where they are needed (flo2.io's log,
/// 2026-09-24 to 10-30: detect_gaps 211, loop_status 397, governed_by 226) —
/// the cue was the problem, offered at the wrong moment, so its TIMING
/// changes and its id and words do not.
///
/// `loop_hint/set_decision_status` is deliberately NOT here: it is the
/// owner-only cue whose audience rule 0.94.0 and 0.95.0 fixed, and the one
/// write cue the counts show followed (23 of 167 within five calls).
pub const BATCH_CUES: [&str; 6] = [
    "loop_hint/detect_gaps",
    "loop_hint/get_skill:detect-and-ask",
    "loop_hint/detect_defects",
    "loop_hint/get_skill:check-health",
    "loop_hint/loop_status",
    "loop_hint/governed_by",
];

/// A batch cue offered within this many of the session's latest calls…
pub const BATCH_CALLS: usize = 25;

/// …and at most this many seconds ago, with nothing since acting on it, is
/// not offered again: the batch it was offered for is still landing.
pub const BATCH_SECS: u64 = 900;

/// What a write's `loop_hint` says when every cue it carries was offered
/// earlier in the same batch. It names no call, so it is not a cue and is not
/// counted as one; it says the step was withheld rather than leaving the field
/// silently absent.
pub const WITHHELD_HINT: &str = "loop: the step for when this batch lands was named earlier in \
     the batch and is not repeated on every write";

/// How many bytes from the end of the usage ledger are read to decide what is
/// withheld. A session whose last [`BATCH_CALLS`] calls lie further back than
/// this (a ledger shared by many busy sessions) is simply offered every cue,
/// as before.
pub const BATCH_TAIL_BYTES: u64 = 256 * 1024;

tokio::task_local! {
    /// The batch cues withheld on the tool call now being served, set by
    /// `call_tool` around the handler from the usage ledger. Absent outside a
    /// served call and on a design with no ledger (in memory), where every cue
    /// is offered exactly as before.
    static WITHHELD: std::sync::Arc<BTreeSet<String>>;
}

/// Run `call` with `withheld` as the batch cues this call does not offer.
pub async fn with_withheld<T>(
    withheld: BTreeSet<String>,
    call: impl std::future::Future<Output = T>,
) -> T {
    WITHHELD.scope(std::sync::Arc::new(withheld), call).await
}

/// Is this cue offered on the call now being served? Every cue is, except a
/// [`BATCH_CUES`] id the ledger shows was offered earlier in this batch.
pub fn due(cue_id: &str) -> bool {
    WITHHELD.try_with(|w| !w.contains(cue_id)).unwrap_or(true)
}

/// `clause` when any of `ids` is due on this call, else nothing: the
/// batch-end clause of a hint whose first half is about the node just
/// written and is said every time.
pub fn batch_clause(ids: &[&str], clause: &'static str) -> &'static str {
    if ids.iter().any(|id| due(id)) {
        clause
    } else {
        ""
    }
}

/// `hint` when any of `ids` is due on this call, else [`WITHHELD_HINT`]: a
/// hint that is nothing but a batch-end step.
pub fn batch_hint(ids: &[&str], hint: &'static str) -> &'static str {
    if ids.iter().any(|id| due(id)) {
        hint
    } else {
        WITHHELD_HINT
    }
}

/// The batch cues a session is NOT offered on its next call: each one this
/// session was offered within its last [`BATCH_CALLS`] calls and
/// [`BATCH_SECS`] seconds, with no call since acting on it. `lines` is the
/// ledger's tail in ledger order; `session` is [`session_key`] of the call.
/// Acting on a cue ends its batch, so the next write offers it again.
pub fn withheld(lines: &[crate::usage::UsageLine], session: &str, now: u64) -> BTreeSet<String> {
    let seq: Vec<&crate::usage::UsageLine> = lines
        .iter()
        .filter(|l| l.kind == "call" && session_of(l) == session)
        .collect();
    let recent = &seq[seq.len().saturating_sub(BATCH_CALLS)..];
    BATCH_CUES
        .iter()
        .filter(|id| {
            let target = target_of(id);
            recent
                .iter()
                .rposition(|l| l.cues.iter().any(|c| c == *id))
                .is_some_and(|i| {
                    now.saturating_sub(recent[i].at) <= BATCH_SECS
                        && !recent[i + 1..].iter().any(|x| acts_on(x, target))
                })
        })
        .map(|id| id.to_string())
        .collect()
}

/// How a READ can be held to a small reader's budget: what a cue to it must
/// pass so it never leads that reader into a large first read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadBound {
    /// It declares a top-level `budget_chars`: the cue asks with
    /// [`CUE_READ_BUDGET`].
    Budget,
    /// It declares a `limit` and no budget (a search): the cue asks with
    /// [`CUE_SEARCH_LIMIT`].
    Limit,
    /// Neither: nothing the cue can pass holds it, so it is left out of the
    /// block unless its size is known ([`Vocabulary::skill_chars`]).
    Unbounded,
}

/// Reads whose `budget_chars` trims PROSE ONLY and never shortens a list,
/// because for each the whole list is the answer
/// (`crate::reply_budget::bound_reply`): their reply has a floor the budget
/// cannot reach, so asking one for 4,000 characters does not hold it there.
/// Measured on main c376e4a10: `graph_report` JSON answered 109,610
/// characters on the generated design of reflow2's size with its default
/// budget, and 6,197 to 6,355 with `budget_chars` 3,000 on the size sweep's
/// designs; `describe_schema` 57,917 at a budget of 30,000. A cue to one of
/// these is a cue to a read nothing bounds ([`ReadBound::Unbounded`]).
pub const BUDGET_KEEPS_EVERY_ROW: [&str; 4] = [
    "graph_report",
    "describe_schema",
    "list_skills",
    "manual_work_ledger",
];

/// How a served read is bounded, from what its schema declares.
pub fn read_bound(tool: &str, declares_budget: bool, declares_limit: bool) -> ReadBound {
    if declares_budget && !BUDGET_KEEPS_EVERY_ROW.contains(&tool) {
        ReadBound::Budget
    } else if declares_limit {
        ReadBound::Limit
    } else {
        ReadBound::Unbounded
    }
}

/// The words a cue may name: the served tools and the served skills — and,
/// for the block, how each served read is bounded and how long each skill is.
#[derive(Debug, Clone, Default)]
pub struct Vocabulary {
    pub tools: BTreeSet<String>,
    pub skills: BTreeSet<String>,
    /// Every served READ (`read_only_hint: true`) and how it is bounded. A
    /// tool absent here is a write, whose reply is a receipt.
    pub reads: BTreeMap<String, ReadBound>,
    /// Each served skill's body, in characters: what `get_skill` sends whole,
    /// whatever its `budget_chars` (that bounds only the lessons beside it).
    pub skill_chars: BTreeMap<String, usize>,
}

/// One cue a reply offers: its id, the field it came from, the call it names,
/// and the record that call should read, when the cue points at one (a
/// withheld lesson, a near-match). The record rides the reply's block, never
/// the ledger (verbs only).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Offer {
    pub id: String,
    pub field: String,
    pub target: String,
    pub record: Option<String>,
}

impl Vocabulary {
    pub fn new(
        tools: impl IntoIterator<Item = String>,
        skills: impl IntoIterator<Item = String>,
    ) -> Self {
        Vocabulary {
            tools: tools.into_iter().collect(),
            skills: skills.into_iter().collect(),
            ..Vocabulary::default()
        }
    }

    /// The same vocabulary, knowing how each served read is bounded and how
    /// long each skill is — what the block needs and the ledger does not.
    #[must_use]
    pub fn with_bounds(
        mut self,
        reads: impl IntoIterator<Item = (String, ReadBound)>,
        skill_chars: impl IntoIterator<Item = (String, usize)>,
    ) -> Self {
        self.reads = reads.into_iter().collect();
        self.skill_chars = skill_chars.into_iter().collect();
        self
    }

    /// The calls one cue sentence names, as targets: a tool name, or
    /// `get_skill:<skill>`. In order of first mention, without repeats.
    ///
    /// A tool name with an underscore is matched as a whole word; a one-word
    /// tool name (`genesis`, `answers`, `contains`) only when back-quoted, so
    /// an English word is not miscounted as a call. A skill is matched after
    /// `get_skill`, before the word `skill`, or anywhere when its name is
    /// hyphenated (`detect-and-ask`) — a one-word skill (`topic`, `why`) is
    /// otherwise an ordinary word.
    pub fn named_calls(&self, text: &str) -> Vec<String> {
        let tokens = tokens(text);
        let mut out: Vec<String> = Vec::new();
        let mut push = |t: String| {
            if !out.contains(&t) {
                out.push(t);
            }
        };
        for (i, (word, quoted)) in tokens.iter().enumerate() {
            let prev = i.checked_sub(1).map(|j| tokens[j].0.as_str());
            let next = tokens.get(i + 1).map(|t| t.0.as_str());
            if self.skills.contains(word)
                && (word.contains('-')
                    || prev == Some("get_skill")
                    || next.is_some_and(|n| n.eq_ignore_ascii_case("skill")))
            {
                push(format!("get_skill:{word}"));
                continue;
            }
            if self.tools.contains(word) && (word.contains('_') || *quoted) {
                // `get_skill <name>` is a cue to the skill, said once.
                if word == "get_skill" && next.is_some_and(|n| self.skills.contains(n)) {
                    continue;
                }
                push(word.clone());
            }
        }
        out
    }

    /// The cue ids one reply offers: every call named under a cue field,
    /// except the tool that was just called (`called`), and the record cues
    /// ([`Vocabulary::offers`]). Sorted, deduplicated, at most
    /// [`MAX_PER_REPLY`].
    pub fn offered(&self, reply: &Value, called: &str) -> Vec<String> {
        let ids: BTreeSet<String> = self
            .offers(reply, called)
            .into_iter()
            .map(|o| o.id)
            .collect();
        ids.into_iter().take(MAX_PER_REPLY).collect()
    }

    /// Every cue one reply offers, once per id, sorted by id:
    ///
    /// - a call named under a cue field ([`CUE_FIELDS`], at any depth, never
    ///   inside a node's `properties` and never in a [`REFERENCE_FIELDS`]
    ///   field), other than the tool just called — `<field>/<call>`;
    /// - a LESSON this design holds for the step that the reply withheld to
    ///   fit (`lessons.withheld_ids`, as `get_skill` sends them):
    ///   `lessons/get_node`, on the newest withheld;
    /// - a RELATED record a write found (`search_first.near_matches`):
    ///   `near_matches/get_node`, on the nearest.
    pub fn offers(&self, reply: &Value, called: &str) -> Vec<Offer> {
        let mut found: BTreeMap<String, Offer> = BTreeMap::new();
        self.walk(reply, &mut found, called);
        found.into_values().collect()
    }

    fn walk(&self, v: &Value, found: &mut BTreeMap<String, Offer>, called: &str) {
        match v {
            Value::Object(o) => {
                for (k, child) in o {
                    // A node's properties are the design's content, never a cue.
                    if k == "properties" {
                        continue;
                    }
                    let reference = REFERENCE_FIELDS
                        .iter()
                        .any(|(tool, field)| *tool == called && field == k);
                    if CUE_FIELDS.contains(&k.as_str()) && !reference {
                        let mut texts = Vec::new();
                        strings(child, &mut texts);
                        for t in texts {
                            for target in self.named_calls(t) {
                                if target != called {
                                    let id = format!("{k}/{target}");
                                    found.entry(id.clone()).or_insert(Offer {
                                        id,
                                        field: k.clone(),
                                        target,
                                        record: None,
                                    });
                                }
                            }
                        }
                    } else if reference {
                        continue;
                    } else if let Some(record) = record_cue(k, child) {
                        if called != "get_node" && self.tools.contains("get_node") {
                            let id = format!("{k}/get_node");
                            found.entry(id.clone()).or_insert(Offer {
                                id,
                                field: k.clone(),
                                target: "get_node".into(),
                                record: Some(record),
                            });
                        }
                    } else {
                        self.walk(child, found, called);
                    }
                }
            }
            Value::Array(items) => {
                for x in items {
                    self.walk(x, found, called);
                }
            }
            _ => {}
        }
    }
}

/// The record a `lessons` or `near_matches` field points at, when it points
/// at one: the first id `lessons.withheld_ids` names (the lessons are ranked
/// newest first), or the first near-match's `node_id`. Nothing for any other
/// field, or for a lessons block that withheld nothing — its lessons are in
/// the reply already.
fn record_cue(field: &str, v: &Value) -> Option<String> {
    match field {
        "lessons" => v
            .get("withheld_ids")?
            .as_array()?
            .first()?
            .as_str()
            .map(str::to_string),
        "near_matches" => v
            .as_array()?
            .first()?
            .get("node_id")?
            .as_str()
            .map(str::to_string),
        _ => None,
    }
}

/// Every string under `v`, depth first.
fn strings<'a>(v: &'a Value, out: &mut Vec<&'a str>) {
    match v {
        Value::String(s) => out.push(s),
        Value::Array(items) => items.iter().for_each(|x| strings(x, out)),
        Value::Object(o) => o.values().for_each(|x| strings(x, out)),
        _ => {}
    }
}

/// Words made of letters, digits, `_` and `-`, each with whether it was
/// back-quoted. Leading and trailing `-` are dropped (a dash used as
/// punctuation is not part of a name).
fn tokens(text: &str) -> Vec<(String, bool)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '-' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '-')
            {
                i += 1;
            }
            let raw: String = chars[start..i].iter().collect();
            let word = raw.trim_matches('-').to_string();
            let quoted = start > 0 && chars[start - 1] == '`' && chars.get(i) == Some(&'`');
            if !word.is_empty() {
                out.push((word, quoted));
            }
        } else {
            i += 1;
        }
    }
    out
}

/// The named call a cue id points at: `loop_hint/set_decision_status` →
/// `set_decision_status`; `hint/get_skill:root-cause` → `get_skill:root-cause`.
pub fn target_of(cue_id: &str) -> &str {
    cue_id.split_once('/').map(|(_, t)| t).unwrap_or(cue_id)
}

/// The call a tool's job moved to, when it moved: a counted refusing stub's
/// survivor (`crate::deprecation::survivor`), else the name itself.
/// `graph_report_markdown` → `graph_report`, `propagate_change` →
/// `propagate_from`.
pub fn canonical_call(name: &str) -> &str {
    crate::deprecation::survivor(crate::deprecation::REGISTRY, name).unwrap_or(name)
}

/// A cue id AS IT IS COUNTED: one whose named call moved to another tool is
/// the survivor's id, so a count read across the move reads one cue, not two.
/// `instructions/graph_report_markdown` (the handshake's id through
/// 2026-10-30) is `instructions/graph_report`; an id whose call did not move
/// is itself. A skill target never moves.
pub fn canonical(cue_id: &str) -> String {
    match cue_id.split_once('/') {
        Some((field, target)) if !target.starts_with("get_skill:") => {
            format!("{field}/{}", canonical_call(target))
        }
        _ => cue_id.to_string(),
    }
}

/// Whether a ledger call line made the call a cue target names. A call to a
/// tool whose job moved acts on its survivor: before 2026-10-30 a call to
/// `graph_report_markdown` WAS the call `instructions/graph_report` names.
fn acts_on(line: &crate::usage::UsageLine, target: &str) -> bool {
    let called = line.tool.as_deref().map(canonical_call);
    match target.strip_prefix("get_skill:") {
        Some(skill) => called == Some("get_skill") && line.skill.as_deref() == Some(skill),
        None => called == Some(canonical_call(target)),
    }
}

/// One cue's counts in a window.
#[derive(Debug, Default, Clone, Serialize, PartialEq, Eq)]
pub struct CueCount {
    pub offered: usize,
    /// Acted on within the session's next [`FOLLOW_WITHIN`] calls.
    pub followed_within_5: usize,
    /// Acted on at any later call of the session in the window (includes
    /// the prompt ones).
    pub followed_at_all: usize,
    /// Offered on a session that made no other call in the window — a
    /// per-request seat, which nothing can follow. Not counted as ignored.
    pub unattributable: usize,
    /// Which tools (or `instructions`, the handshake) offered it, and how often.
    pub offered_by: BTreeMap<String, usize>,
}

/// The `cues` block of `usage_report`'s tally.
#[derive(Debug, Default, Serialize)]
pub struct CueTally {
    pub follow_within_calls: usize,
    pub offered: usize,
    pub followed_within_5: usize,
    pub followed_at_all: usize,
    pub unattributable: usize,
    /// cue id → its counts, every cue offered in the window, each under the
    /// id it is counted by ([`canonical`]).
    pub by_cue: BTreeMap<String, CueCount>,
    /// The ids in this window whose call MOVED, and the id each was counted
    /// under: `instructions/graph_report_markdown` →
    /// `instructions/graph_report`. Absent when none did.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub moved: BTreeMap<String, String>,
    pub note: &'static str,
}

const NOTE: &str = "A cue is a sentence in a reply's loop_hint, hint, next, nudge, next_step or \
     carry_it naming a served tool or skill other than the one just called (describe_schema's \
     per-type `hint` is reference text and is not one), a lesson the reply withheld \
     (`lessons/get_node`) or a near-match a write found (`near_matches/get_node`), or the \
     first-read cue the connect-time instructions carry (`instructions/…`, offered once per \
     session at the handshake). The reply's `cues` block shows the ones that earn a place; every \
     one is counted. Its id is `<field>/<named call>`; an id whose call moved to another tool is \
     counted under the survivor's (`moved`). FOLLOWED means the same session made that call \
     later in the window: within its next 5 calls, or at all. A session is the seat that made the \
     calls (behind a trusted gateway, the seat the gateway named for that caller); every `reflow2-mcp --call` door call has a seat of its own, so the door's calls are \
     followed as one sequence in ledger order. UNATTRIBUTABLE: offered on a session that made no \
     other call in the window (a per-request seat), so nothing could follow it; not counted as \
     ignored. Not counted: the kit's session-start hook repeats the first-read cue outside the \
     server, where no seat is known. Verbs only: no sentence, node id or argument is kept.";

/// The session a ledger line belongs to: its seat, except that every door
/// call (`reflow2-mcp --call`) runs in a process of its own and so has a seat
/// of its own — the door's calls are one sequence, in ledger order.
fn session_of(line: &crate::usage::UsageLine) -> String {
    session_key(line.client.as_deref(), line.seat.as_deref())
}

/// The session a call belongs to, from the client it came through and the
/// seat it is recorded under — the key [`tally`] follows cues by, and
/// [`withheld`] withholds them by.
pub fn session_key(client: Option<&str>, seat: Option<&str>) -> String {
    if client == Some(crate::service::CALL_DOOR_CLIENT) {
        return "door".to_string();
    }
    seat.unwrap_or("unknown").to_string()
}

/// Count every cue offered in `lines` (a window of `call` and `cue` lines, in
/// ledger order) by whether the same session acted on it afterwards.
pub fn tally(lines: &[&crate::usage::UsageLine]) -> CueTally {
    let mut sessions: BTreeMap<String, Vec<&crate::usage::UsageLine>> = BTreeMap::new();
    for l in lines {
        sessions.entry(session_of(l)).or_default().push(l);
    }
    let mut t = CueTally {
        follow_within_calls: FOLLOW_WITHIN,
        note: NOTE,
        ..CueTally::default()
    };
    for seq in sessions.values() {
        let calls_in_session = seq.iter().filter(|l| l.kind == "call").count();
        for (i, l) in seq.iter().enumerate() {
            if l.cues.is_empty() {
                continue;
            }
            let later: Vec<&&crate::usage::UsageLine> =
                seq[i + 1..].iter().filter(|x| x.kind == "call").collect();
            let source = if l.kind == "cue" {
                "instructions".to_string()
            } else {
                l.tool.clone().unwrap_or_else(|| "?".into())
            };
            // The handshake's offer is not a call, so a session whose only
            // line is the offer, or whose only call made the offer, made no
            // OTHER call that could follow it.
            let own = usize::from(l.kind == "call");
            let lone = calls_in_session <= own;
            for raw in &l.cues {
                let id = canonical(raw);
                if id != *raw {
                    t.moved.insert(raw.clone(), id.clone());
                }
                let target = target_of(&id);
                let c = t.by_cue.entry(id.clone()).or_default();
                c.offered += 1;
                *c.offered_by.entry(source.clone()).or_default() += 1;
                t.offered += 1;
                if lone {
                    c.unattributable += 1;
                    t.unattributable += 1;
                    continue;
                }
                if later.iter().take(FOLLOW_WITHIN).any(|x| acts_on(x, target)) {
                    c.followed_within_5 += 1;
                    t.followed_within_5 += 1;
                }
                if later.iter().any(|x| acts_on(x, target)) {
                    c.followed_at_all += 1;
                    t.followed_at_all += 1;
                }
            }
        }
    }
    t
}

/// Put a reply's cue ids in its `_meta` under [`META_KEY`]. Nothing when
/// there are none, so a reply that offers no cue is byte for byte what it was.
pub fn attach(result: &mut rmcp::model::CallToolResult, ids: &[String]) {
    if ids.is_empty() {
        return;
    }
    let mut meta = result.meta.take().unwrap_or_default();
    meta.0.insert(
        META_KEY.to_string(),
        Value::Array(ids.iter().cloned().map(Value::String).collect()),
    );
    result.meta = Some(meta);
}

// ---- the `cues` block: one named message ----------------------------------

/// Where the block rides: this key, at the top level of `structuredContent`.
pub const BLOCK_KEY: &str = "cues";

/// At most this many cues in one reply's block.
pub const BLOCK_MAX_CUES: usize = 3;

/// The block's own budget, in characters of JSON (three cues of the longest
/// shape fit with room to spare). Held small on purpose: a reply already
/// carries its fields' sentences, and the block exists to say which ONE to
/// act on first, not to say them again.
pub const BLOCK_MAX_CHARS: usize = 600;

/// The budget a cue asks a read for: a third of the 12,000-byte reading
/// budget the recall and size-sweep runs measured at, so following one cue
/// never spends most of a small reader's reading.
pub const CUE_READ_BUDGET: usize = 4_000;

/// The `limit` a cue asks a search for (a search takes no budget; ten hits
/// measured about 2.4 KB on the size sweep's largest design).
pub const CUE_SEARCH_LIMIT: usize = 5;

/// A cue counts as NEVER FOLLOWED only on at least this many attributable
/// offers, none of them followed at all. Fewer cannot tell "rarely" from
/// "never": a cue followed one time in ten is missed by twenty offers about
/// one time in eight.
pub const MIN_EVIDENCE: usize = 20;

/// The four types a cue has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// The call to make next: `hint`, `next_step`, `nudge`, `carry_it`.
    NextCall,
    /// A lesson this design holds for the step, withheld from the reply.
    Lesson,
    /// A record the write resembles (a near-match).
    Related,
    /// What the coherence loop is owed: `loop_hint`, and loop_status's `next`.
    Owed,
}

/// A cue's type, from the field it came from.
pub fn kind_of(field: &str) -> Kind {
    match field {
        "loop_hint" | "next" => Kind::Owed,
        "lessons" => Kind::Lesson,
        "near_matches" => Kind::Related,
        _ => Kind::NextCall,
    }
}

/// What the counts say about one cue id: attributable offers, followed within
/// [`FOLLOW_WITHIN`] calls, followed at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Measured {
    pub id: &'static str,
    pub offered: usize,
    pub followed_within_5: usize,
    pub followed_at_all: usize,
}

/// THE COUNTS THE BLOCK IS CHOSEN BY, as measured, under the ids they are
/// counted by now ([`canonical`]), attributable offers only (offered minus
/// `unattributable`), in rank order.
///
/// RE-RANKED 2026-11-21 FROM FRESH COUNTS: `usage_report` (`since`
/// 2026-10-31, `peek`, read through the hub 2026-11-21, server 0.102.0) on
/// flo2.io's ledgers for reflow2's own design and for flo2's
/// (`1a97fc1f1de4d5d2`), the two summed, 2026-10-31 to 2026-11-21. The 11-20
/// re-count of the tools
/// (`fact:the-tool-counts-re-run-on-0-101-0-three-weeks-later-2026-11-20`)
/// did not count cues, so this is the first count of them since the 10-30
/// table this one replaces. What moved: `next_step/set_artifact_checksums`
/// (67 of 118 within five calls) and loop_status's `next/add_change_event` are
/// the most-followed counted cues; `loop_hint/detect_gaps`,
/// `loop_hint/get_skill:detect-and-ask` and `loop_hint/governed_by`, never
/// followed in the 10-30 window, were each followed later in the session
/// (governed_by twice within five calls, the other two never), so they are
/// ranked rather than left out; and six of loop_status's `next` cues and both halves of the
/// handshake's first-read cue were never followed in 46 or more attributable
/// offers each.
///
/// `carry_it/claim_region` is NOT in the fresh count: the server that wrote
/// these ledgers (0.102.0) does not count `carry_it` yet (this increment makes
/// it a cue). Its row is still the 10-30 one, from flo2.io's call log,
/// 2026-09-24 to 10-30, where every one of 38 `mint_seat` runs was followed by
/// `claim_region`
/// (`fact:the-served-tools-sorted-by-their-counts-109-used-5-cued-unfollowed-40-never-reached-6-steps-39-close-2026-10-30`).
///
/// `instructions/graph_report` sums `instructions/graph_report_markdown`
/// (its id through 2026-10-30), which the 0.102.0 counter still lists apart.
/// An id not listed is UNCOUNTED, and so is one with fewer than
/// [`MIN_EVIDENCE`] offers and no follow. Re-read at the next count (the
/// December re-count, about 2026-12-23): a table, so the change is a reviewed
/// diff.
pub const MEASURED: &[Measured] = &[
    Measured {
        id: "carry_it/claim_region",
        offered: 38,
        followed_within_5: 38,
        followed_at_all: 38,
    },
    Measured {
        id: "next_step/set_artifact_checksums",
        offered: 118,
        followed_within_5: 67,
        followed_at_all: 95,
    },
    Measured {
        id: "next/add_change_event",
        offered: 67,
        followed_within_5: 9,
        followed_at_all: 64,
    },
    Measured {
        id: "loop_hint/governed_by",
        offered: 25,
        followed_within_5: 2,
        followed_at_all: 24,
    },
    Measured {
        id: "loop_hint/loop_status",
        offered: 228,
        followed_within_5: 17,
        followed_at_all: 221,
    },
    Measured {
        id: "next/set_decision_status",
        offered: 45,
        followed_within_5: 3,
        followed_at_all: 43,
    },
    Measured {
        id: "loop_hint/set_decision_status",
        offered: 320,
        followed_within_5: 16,
        followed_at_all: 312,
    },
    Measured {
        id: "loop_hint/detect_gaps",
        offered: 122,
        followed_within_5: 0,
        followed_at_all: 68,
    },
    Measured {
        id: "loop_hint/get_skill:detect-and-ask",
        offered: 120,
        followed_within_5: 0,
        followed_at_all: 6,
    },
    Measured {
        id: "loop_hint/reconcile_artifacts",
        offered: 73,
        followed_within_5: 0,
        followed_at_all: 21,
    },
    Measured {
        id: "next/get_skill:detect-and-ask",
        offered: 125,
        followed_within_5: 0,
        followed_at_all: 8,
    },
    Measured {
        id: "next/reconcile_artifacts",
        offered: 96,
        followed_within_5: 0,
        followed_at_all: 3,
    },
    Measured {
        id: "next/external_dependency",
        offered: 46,
        followed_within_5: 0,
        followed_at_all: 21,
    },
    Measured {
        id: "next/detect_gaps",
        offered: 12,
        followed_within_5: 0,
        followed_at_all: 4,
    },
    Measured {
        id: "instructions/search_design",
        offered: 158,
        followed_within_5: 0,
        followed_at_all: 0,
    },
    Measured {
        id: "instructions/graph_report",
        offered: 158,
        followed_within_5: 0,
        followed_at_all: 0,
    },
    Measured {
        id: "next/repair_report",
        offered: 67,
        followed_within_5: 0,
        followed_at_all: 0,
    },
    Measured {
        id: "next/detect_defects",
        offered: 64,
        followed_within_5: 0,
        followed_at_all: 0,
    },
    Measured {
        id: "next/get_skill:check-health",
        offered: 51,
        followed_within_5: 0,
        followed_at_all: 0,
    },
    Measured {
        id: "next/answers",
        offered: 46,
        followed_within_5: 0,
        followed_at_all: 0,
    },
    Measured {
        id: "next/open_questions",
        offered: 46,
        followed_within_5: 0,
        followed_at_all: 0,
    },
    Measured {
        id: "next/reasoning_report",
        offered: 19,
        followed_within_5: 0,
        followed_at_all: 0,
    },
    Measured {
        id: "next/confirmation_ledger",
        offered: 9,
        followed_within_5: 0,
        followed_at_all: 0,
    },
];

/// Where the counts put a cue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// Followed at least once: ranked first, by how often within five calls
    /// (per thousand offers).
    Followed { per_mille: usize },
    /// Not counted yet, or too few offers to say: ranked after every followed
    /// cue, by type.
    Uncounted,
    /// Offered at least [`MIN_EVIDENCE`] times and never followed: left out
    /// of the block. It stays in the field it came from.
    NeverFollowed,
}

/// What [`MEASURED`] says about a cue id (read under its [`canonical`] id).
pub fn standing(cue_id: &str) -> Standing {
    let id = canonical(cue_id);
    match MEASURED.iter().find(|m| m.id == id) {
        Some(m) if m.followed_at_all > 0 => Standing::Followed {
            per_mille: m.followed_within_5 * 1000 / m.offered.max(1),
        },
        Some(m) if m.offered >= MIN_EVIDENCE => Standing::NeverFollowed,
        _ => Standing::Uncounted,
    }
}

/// One block item for an offer, or `None` when a cue to it could lead a small
/// reader into a large read: a read that takes neither a budget nor a limit,
/// or a skill whose size is not known.
fn block_item(o: &Offer, v: &Vocabulary) -> Option<Value> {
    let (call, mut args) = match o.target.strip_prefix("get_skill:") {
        Some(skill) => ("get_skill".to_string(), json!({ "name": skill })),
        None => (o.target.clone(), json!({})),
    };
    if let Some(record) = &o.record {
        args["id"] = json!(record);
    }
    let mut item = json!({ "cue": o.id, "type": kind_of(&o.field), "call": call });
    match v.reads.get(&call) {
        // A write: its reply is a receipt, whatever the design's size.
        None => {}
        Some(ReadBound::Budget) => args["budget_chars"] = json!(CUE_READ_BUDGET),
        Some(ReadBound::Limit) => args["limit"] = json!(CUE_SEARCH_LIMIT),
        Some(ReadBound::Unbounded) => return None,
    }
    // A skill's body comes whole, whatever the budget: say how long it is.
    if let Some(skill) = o.target.strip_prefix("get_skill:") {
        item["size_chars"] = json!(v.skill_chars.get(skill)?);
    }
    if args.as_object().is_some_and(|a| !a.is_empty()) {
        item["args"] = args;
    }
    Some(item)
}

/// THE BLOCK for one reply's offers, ranked and held to `room` characters
/// (and never more than [`BLOCK_MAX_CHARS`] or [`BLOCK_MAX_CUES`]), or `None`
/// when no cue earns a place. Ranked: followed cues first, most often followed
/// within five calls first; then uncounted ones by type ([`Kind`]'s order);
/// ties by id. Left out, and counted in `left_out`: never-followed cues, cues
/// to a read nothing bounds, a second cue to a call already in the block, and
/// whatever the budget did not hold.
pub fn block(offers: &[Offer], v: &Vocabulary, room: usize) -> Option<Value> {
    let mut left_out = 0usize;
    let mut ranked: Vec<((u8, usize, Kind, String), Value)> = Vec::new();
    for o in offers {
        let key = match standing(&o.id) {
            Standing::NeverFollowed => {
                left_out += 1;
                continue;
            }
            Standing::Followed { per_mille } => (0u8, 1000usize.saturating_sub(per_mille)),
            Standing::Uncounted => (1u8, 0usize),
        };
        let Some(item) = block_item(o, v) else {
            left_out += 1;
            continue;
        };
        ranked.push(((key.0, key.1, kind_of(&o.field), o.id.clone()), item));
    }
    ranked.sort_by(|a, b| a.0.cmp(&b.0));
    let limit = room.min(BLOCK_MAX_CHARS);
    let mut items: Vec<Value> = Vec::new();
    let mut calls: Vec<(Value, Value)> = Vec::new();
    let assemble =
        |items: &[Value], left_out: usize| json!({ "items": items, "left_out": left_out });
    let total = ranked.len();
    for (_, item) in ranked {
        let same = (item["call"].clone(), item["args"].clone());
        if calls.contains(&same) || items.len() == BLOCK_MAX_CUES {
            continue;
        }
        items.push(item);
        if crate::reply_budget::chars_of(&assemble(&items, left_out + total - items.len())) > limit
        {
            items.pop();
            continue;
        }
        calls.push(same);
    }
    if items.is_empty() {
        return None;
    }
    Some(assemble(&items, left_out + total - items.len()))
}

/// PUT THE BLOCK ON A REPLY: at the top level of its `structuredContent`,
/// under [`BLOCK_KEY`], when it has an object there and offers a cue that
/// earns a place — and only within `budget`, the reply's own budget (the
/// caller's `budget_chars`, else the 30,000 default): the block takes what
/// is left of it and never pushes the reply past it. A reply over its budget
/// already gets none. A document-shaped reply (no structured half) and a
/// refusal get none; nothing is added to a reply that offers no cue, so it is
/// byte for byte what it was.
pub fn attach_block(
    result: &mut rmcp::model::CallToolResult,
    offers: &[Offer],
    v: &Vocabulary,
    budget: usize,
) {
    if offers.is_empty() || result.is_error == Some(true) {
        return;
    }
    let Some(Value::Object(obj)) = result.structured_content.as_mut() else {
        return;
    };
    if obj.contains_key(BLOCK_KEY) {
        return;
    }
    let used = serde_json::to_string(obj)
        .map(|s| s.len())
        .unwrap_or(usize::MAX);
    // `,"cues":` beside what is already there.
    let room = budget.saturating_sub(used.saturating_add(BLOCK_KEY.len() + 4));
    if let Some(b) = block(offers, v, room) {
        obj.insert(BLOCK_KEY.to_string(), b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::{Outcome, UsageLine};
    use serde_json::json;

    fn vocab() -> Vocabulary {
        Vocabulary::new(
            [
                "set_decision_status",
                "loop_status",
                "detect_gaps",
                "get_skill",
                "genesis",
                "add_decision",
                "ask_question",
            ]
            .map(String::from),
            ["detect-and-ask", "root-cause", "topic"].map(String::from),
        )
    }

    /// The baseline's two commonest cues are read as it read them, and an
    /// English word that is also a one-word name is not.
    #[test]
    fn the_calls_a_sentence_names_are_read_from_the_served_vocabulary() {
        let v = vocab();
        assert_eq!(
            v.named_calls(
                "loop: one field moved — if the change is a settlement, set_decision_status \
                 carries the owner's word; loop_status says what else is owed"
            ),
            vec!["set_decision_status", "loop_status"]
        );
        assert_eq!(
            v.named_calls("run detect_gaps, then get_skill detect-and-ask; the root-cause skill"),
            vec![
                "detect_gaps",
                "get_skill:detect-and-ask",
                "get_skill:root-cause"
            ]
        );
        assert!(
            v.named_calls("the genesis of the topic").is_empty(),
            "one-word names are words unless quoted or called a skill"
        );
        assert_eq!(
            v.named_calls("start with `genesis`, or the topic skill"),
            vec!["genesis", "get_skill:topic"]
        );
    }

    /// Cue fields at any depth are read; node properties never are; the tool
    /// just called is not a cue to itself.
    #[test]
    fn a_reply_offers_the_calls_its_cue_fields_name_and_nothing_else() {
        let v = vocab();
        let reply = json!({
            "loop_hint": "loop: a Decision lands `proposed` — only the owner's word moves it (set_decision_status)",
            "properties": {"next": "set_decision_status is design content here"},
            "status": {"next": ["call loop_status", "add_decision again"]},
            "statement": "detect_gaps is not under a cue field",
        });
        assert_eq!(
            v.offered(&reply, "add_decision"),
            vec!["loop_hint/set_decision_status", "next/loop_status"]
        );
    }

    fn line(kind: &str, tool: &str, seat: &str, cues: &[&str]) -> UsageLine {
        UsageLine {
            at: 1,
            kind: kind.into(),
            tool: (kind == "call").then(|| tool.to_string()),
            outcome: (kind == "call").then_some(Outcome::Ok),
            refusal: None,
            ms: None,
            client: Some("probe".into()),
            client_version: None,
            seat: Some(seat.into()),
            skill: None,
            cues: cues.iter().map(|c| c.to_string()).collect(),
        }
    }

    /// Followed within five, followed later, never followed, and offered on a
    /// session that made no other call — the four readings, per cue.
    #[test]
    fn a_cue_is_counted_by_whether_its_session_acted_on_it() {
        let mut lines = vec![
            line("cue", "", "s1", &["instructions/graph_report"]),
            line("call", "graph_report", "s1", &[]),
            line(
                "call",
                "add_decision",
                "s1",
                &["loop_hint/set_decision_status", "next/loop_status"],
            ),
        ];
        for _ in 0..5 {
            lines.push(line("call", "get_node", "s1", &[]));
        }
        lines.push(line("call", "loop_status", "s1", &[]));
        // A per-request seat: one call, nothing can follow its cue.
        lines.push(line(
            "call",
            "add_decision",
            "s2",
            &["loop_hint/set_decision_status"],
        ));
        let refs: Vec<&UsageLine> = lines.iter().collect();
        let t = tally(&refs);

        let first = &t.by_cue["instructions/graph_report"];
        assert_eq!(
            (
                first.offered,
                first.followed_within_5,
                first.followed_at_all
            ),
            (1, 1, 1)
        );
        assert_eq!(first.offered_by["instructions"], 1);

        let settle = &t.by_cue["loop_hint/set_decision_status"];
        assert_eq!(settle.offered, 2);
        assert_eq!((settle.followed_within_5, settle.followed_at_all), (0, 0));
        assert_eq!(
            settle.unattributable, 1,
            "the lone seat's offer is not counted as ignored"
        );

        let status = &t.by_cue["next/loop_status"];
        assert_eq!(
            (status.followed_within_5, status.followed_at_all),
            (0, 1),
            "made on the sixth call after: later, not within five"
        );
        assert_eq!(t.offered, 4);
        assert_eq!(t.unattributable, 1);
    }

    /// The door's calls each carry a seat of their own, so they are followed
    /// as one sequence; a skill cue is followed by fetching that skill.
    #[test]
    fn door_calls_are_one_sequence_and_a_skill_cue_is_followed_by_its_fetch() {
        let mut a = line(
            "call",
            "add_requirement",
            "p1",
            &["loop_hint/get_skill:detect-and-ask"],
        );
        a.client = Some(crate::service::CALL_DOOR_CLIENT.into());
        let mut b = line("call", "get_skill", "p2", &[]);
        b.client = Some(crate::service::CALL_DOOR_CLIENT.into());
        b.skill = Some("detect-and-ask".into());
        let refs = vec![&a, &b];
        let t = tally(&refs);
        let c = &t.by_cue["loop_hint/get_skill:detect-and-ask"];
        assert_eq!((c.followed_within_5, c.unattributable), (1, 0));
    }

    fn at(mut l: UsageLine, t: u64) -> UsageLine {
        l.at = t;
        l
    }

    /// A batch cue is offered on the first write of a batch and withheld on
    /// the writes after it; the same session acting on it ends the batch, so
    /// the next write offers it again.
    #[test]
    fn a_batch_cue_is_withheld_after_its_offer_until_the_session_acts_on_it() {
        let gaps = "loop_hint/detect_gaps";
        let mut lines = vec![at(line("call", "add_requirement", "s1", &[gaps]), 100)];
        // The first write offered it: the second is not offered it again.
        assert!(withheld(&lines, "s1", 110).contains(gaps));
        // Another session was never offered it, so it is due there.
        assert!(withheld(&lines, "s2", 110).is_empty());
        // Writes that withheld it record no offer; it stays withheld.
        for i in 0..5 {
            lines.push(at(line("call", "add_requirement", "s1", &[]), 111 + i));
        }
        assert!(withheld(&lines, "s1", 120).contains(gaps));
        // Acting on it ends the batch: due again.
        lines.push(at(line("call", "detect_gaps", "s1", &[]), 121));
        assert!(!withheld(&lines, "s1", 122).contains(gaps));
    }

    /// The batch ends on its own after BATCH_CALLS of the session's calls or
    /// BATCH_SECS, so a long session is reminded again, not told once forever.
    #[test]
    fn a_batch_ends_after_its_calls_or_its_seconds() {
        let status = "loop_hint/loop_status";
        let mut lines = vec![at(line("call", "replace_text", "s1", &[status]), 100)];
        assert!(withheld(&lines, "s1", 100 + BATCH_SECS).contains(status));
        assert!(
            withheld(&lines, "s1", 101 + BATCH_SECS).is_empty(),
            "older than BATCH_SECS"
        );
        for i in 0..BATCH_CALLS as u64 {
            lines.push(at(line("call", "replace_text", "s1", &[]), 101 + i));
        }
        assert!(
            withheld(&lines, "s1", 200).is_empty(),
            "the offer is no longer among the session's last BATCH_CALLS calls"
        );
    }

    /// Only the batch cues are ever withheld — the owner-only settling cue
    /// above all — and the door's calls are one session, as they are counted.
    #[test]
    fn only_batch_cues_are_withheld_and_the_door_is_one_session() {
        let mut a = line(
            "call",
            "replace_text",
            "p1",
            &["loop_hint/set_decision_status", "loop_hint/loop_status"],
        );
        a.client = Some(crate::service::CALL_DOOR_CLIENT.into());
        a.at = 100;
        let w = withheld(
            &[a],
            &session_key(Some(crate::service::CALL_DOOR_CLIENT), Some("p2")),
            101,
        );
        assert_eq!(w, ["loop_hint/loop_status".to_string()].into());
        assert!(!BATCH_CUES.contains(&"loop_hint/set_decision_status"));
    }

    /// Outside a served call nothing is withheld; inside one, exactly the set
    /// `call_tool` put there.
    #[tokio::test]
    async fn due_reads_the_set_the_call_was_served_with() {
        assert!(due("loop_hint/detect_gaps"));
        let set: BTreeSet<String> = ["loop_hint/detect_gaps".to_string()].into();
        with_withheld(set, async {
            assert!(!due("loop_hint/detect_gaps"));
            assert!(due("loop_hint/loop_status"));
            assert_eq!(
                batch_hint(&["loop_hint/detect_gaps"], "run detect_gaps"),
                WITHHELD_HINT
            );
            assert_eq!(
                batch_clause(&["loop_hint/detect_gaps"], ", then detect_gaps"),
                ""
            );
            assert_eq!(
                batch_clause(
                    &["loop_hint/detect_gaps", "loop_hint/loop_status"],
                    "; both"
                ),
                "; both"
            );
        })
        .await;
        assert!(
            vocab().named_calls(WITHHELD_HINT).is_empty(),
            "the withheld hint names no call, so it is not a cue"
        );
    }

    /// The instructions sentence names exactly the calls counted for it.
    #[test]
    fn the_first_read_sentence_names_what_the_handshake_counts() {
        let v = Vocabulary::new(
            ["graph_report", "search_design"].map(String::from),
            Vec::<String>::new(),
        );
        let named: Vec<String> = v
            .named_calls(FIRST_READ_CUE)
            .into_iter()
            .map(|t| format!("instructions/{t}"))
            .collect();
        assert_eq!(named, INSTRUCTIONS_OFFER.map(String::from).to_vec());
    }

    /// The ids ride `_meta`, and a reply with none is untouched.
    #[test]
    fn the_ids_ride_meta_and_only_when_there_are_some() {
        let mut r = rmcp::model::CallToolResult::success(vec![]);
        attach(&mut r, &[]);
        assert!(r.meta.is_none());
        attach(&mut r, &["next/loop_status".to_string()]);
        assert_eq!(
            r.meta.as_ref().unwrap().0[META_KEY],
            json!(["next/loop_status"])
        );
    }

    // ---- increment 6: what the count counts, and the one `cues` block ----

    /// A vocabulary that knows how its reads are bounded, as the served one
    /// does: writes are absent from `reads`.
    fn bounded() -> Vocabulary {
        Vocabulary::new(
            [
                "loop_status",
                "detect_gaps",
                "get_node",
                "search_design",
                "seam_report",
                "claim_region",
                "set_decision_status",
                "get_skill",
                "add_decision",
                "governed_by",
                "repair_report",
                "open_questions",
            ]
            .map(String::from),
            ["detect-and-ask", "root-cause"].map(String::from),
        )
        .with_bounds(
            [
                ("loop_status", ReadBound::Budget),
                ("detect_gaps", ReadBound::Budget),
                ("get_node", ReadBound::Budget),
                ("get_skill", ReadBound::Budget),
                ("search_design", ReadBound::Limit),
                ("seam_report", ReadBound::Unbounded),
            ]
            .map(|(t, b)| (t.to_string(), b)),
            [
                ("root-cause".to_string(), 18_000),
                ("detect-and-ask".to_string(), 9_000),
            ],
        )
    }

    fn cues_of(block: &Value) -> Vec<&str> {
        block["items"]
            .as_array()
            .expect("items")
            .iter()
            .map(|i| i["cue"].as_str().expect("cue"))
            .collect()
    }

    /// Decision C of `dec:idea-what-the-cue-count-counts-…`: mint_seat's
    /// `carry_it` is a cue, describe_schema's per-type `hint` is reference
    /// text — on describe_schema only.
    #[test]
    fn carry_it_is_a_cue_and_describe_schemas_hint_is_reference_text() {
        let v = bounded();
        let seat = json!({
            "seat": "h:1:a",
            "carry_it": "Pass this as `seat` on claim_region for the rest of this session."
        });
        assert_eq!(v.offered(&seat, "mint_seat"), vec!["carry_it/claim_region"]);
        let schema = json!({"edge_types": [
            {"edge_type": "SATISFIES", "hint": "claim_region and governed_by write nothing here"}
        ]});
        assert!(
            v.offered(&schema, "describe_schema").is_empty(),
            "reference text on describe_schema is not a cue"
        );
        assert_eq!(
            v.offered(&schema, "some_other_read"),
            vec!["hint/claim_region", "hint/governed_by"],
            "a `hint` anywhere else still is"
        );
    }

    /// A lesson the reply withheld, and a near-match a write found, are cues
    /// to read that record; a lessons block that withheld nothing is not.
    #[test]
    fn a_withheld_lesson_and_a_near_match_are_cues_to_their_record() {
        let v = bounded();
        let reply = json!({
            "lessons": {"items": [], "withheld_ids": ["fact:newest", "fact:older"]},
            "search_first": {"near_matches": [
                {"node_id": "req:nearest", "node_type": "Requirement"},
                {"node_id": "req:second", "node_type": "Requirement"}
            ]},
        });
        let offers = v.offers(&reply, "get_skill");
        assert_eq!(
            offers
                .iter()
                .map(|o| (o.id.as_str(), o.record.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("lessons/get_node", Some("fact:newest")),
                ("near_matches/get_node", Some("req:nearest")),
            ]
        );
        let shown = json!({"lessons": {"items": [{"id": "fact:a"}], "note": "1 lesson"}});
        assert!(v.offers(&shown, "get_skill").is_empty());
        assert!(
            v.offers(&reply, "get_node").is_empty(),
            "not a cue to the tool just called"
        );
    }

    // The moved-id fold (`instructions/graph_report_markdown` counted as
    // `instructions/graph_report`) is tested in
    // tests/a_cue_whose_call_moved_is_counted_as_one_cue.rs, not here: a test
    // of it has to name the counted refusing stub, and every string literal in
    // src/ is read as served text by
    // tests/no_served_text_names_a_tool_that_is_not_served.rs.

    /// The table is read under the ids it is counted by, and says what the
    /// 2026-11-21 counts say: carry_it, the checksum step and the settling cue
    /// followed, detect_gaps followed late; loop_status's repair, defect,
    /// health, answers and open-questions cues and both halves of the
    /// handshake's first read never followed; confirmation_ledger too few to
    /// say.
    #[test]
    fn the_counts_table_is_read_under_the_ids_it_counts_by() {
        let mut seen = BTreeSet::new();
        for m in MEASURED {
            assert_eq!(canonical(m.id), m.id, "{} is not its own counted id", m.id);
            assert!(seen.insert(m.id), "{} is listed twice", m.id);
            assert!(m.followed_within_5 <= m.followed_at_all && m.followed_at_all <= m.offered);
        }
        assert!(matches!(
            standing("carry_it/claim_region"),
            Standing::Followed { per_mille: 1000 }
        ));
        assert!(matches!(
            standing("loop_hint/set_decision_status"),
            Standing::Followed { .. }
        ));
        assert!(matches!(
            standing("next_step/set_artifact_checksums"),
            Standing::Followed { per_mille: 567 }
        ));
        assert_eq!(
            standing("loop_hint/detect_gaps"),
            Standing::Followed { per_mille: 0 }
        );
        for never in [
            "next/repair_report",
            "next/detect_defects",
            "next/get_skill:check-health",
            "next/answers",
            "next/open_questions",
            "instructions/search_design",
            "instructions/graph_report",
        ] {
            assert_eq!(standing(never), Standing::NeverFollowed, "{never}");
        }
        assert_eq!(standing("next/confirmation_ledger"), Standing::Uncounted);
        assert_eq!(standing("loop_hint/detect_defects"), Standing::Uncounted);
        assert_eq!(standing("near_matches/get_node"), Standing::Uncounted);
    }

    /// RANKING: followed cues first, most often followed within five calls
    /// first; never-followed cues left out and counted; reads asked for with a
    /// small budget; types from the field.
    #[test]
    fn the_block_ranks_by_the_counts_and_leaves_out_what_was_never_followed() {
        let v = bounded();
        // Ranked by the 2026-11-21 table: carry_it (38 of 38), loop_status
        // (17 of 228), the settling cue (16 of 320), then detect_gaps and
        // detect-and-ask (followed late, never within five); repair_report and
        // open_questions, never followed, are left out whatever the room.
        let reply = json!({
            "loop_hint": "loop: run detect_gaps, then get_skill detect-and-ask; \
                          set_decision_status carries the owner's word; loop_status says \
                          what else is owed",
            "next": ["call repair_report", "then open_questions"],
            "carry_it": "Pass this as `seat` on claim_region",
        });
        let b = block(&v.offers(&reply, "add_decision"), &v, 30_000).expect("a block");
        assert_eq!(
            cues_of(&b),
            vec![
                "carry_it/claim_region",
                "loop_hint/loop_status",
                "loop_hint/set_decision_status"
            ]
        );
        assert_eq!(
            b["left_out"], 4,
            "repair_report and open_questions never followed, detect_gaps and \
             detect-and-ask past three: {b}"
        );
        let items = b["items"].as_array().unwrap();
        assert_eq!(items[0]["type"], "next_call");
        assert_eq!(items[1]["type"], "owed");
        assert!(items[0].get("args").is_none(), "a write needs no bound");
        assert_eq!(items[1]["args"], json!({"budget_chars": CUE_READ_BUDGET}));
        assert!(items[2].get("args").is_none(), "a write needs no bound");

        // With room for every followed cue, the never-followed ones are still
        // left out: only they are, when the rest fits.
        let few = json!({"next": ["call repair_report", "then open_questions"],
                         "loop_hint": "loop_status says what is owed"});
        let b = block(&v.offers(&few, "add_decision"), &v, 30_000).expect("a block");
        assert_eq!(cues_of(&b), vec!["loop_hint/loop_status"]);
        assert_eq!(b["left_out"], 2, "{b}");
    }

    /// Uncounted cues come after every followed one, by type: the next call,
    /// then a lesson, then a related record. A search is asked for with a
    /// limit; a record read with its id and a budget.
    #[test]
    fn uncounted_cues_rank_after_followed_ones_by_type() {
        let v = bounded();
        let reply = json!({
            "near_matches": [{"node_id": "req:near"}],
            "lessons": {"withheld_ids": ["fact:lesson"]},
            "next_step": "find it with search_design",
            "loop_hint": "loop_status says what is owed",
        });
        let b = block(&v.offers(&reply, "add_requirement"), &v, 30_000).expect("a block");
        assert_eq!(
            cues_of(&b),
            vec![
                "loop_hint/loop_status",
                "next_step/search_design",
                "lessons/get_node"
            ]
        );
        assert_eq!(b["left_out"], 1, "the related cue did not fit three: {b}");
        let items = b["items"].as_array().unwrap();
        assert_eq!(items[1]["args"], json!({"limit": CUE_SEARCH_LIMIT}));
        assert_eq!(items[2]["type"], "lesson");
        assert_eq!(
            items[2]["args"],
            json!({"id": "fact:lesson", "budget_chars": CUE_READ_BUDGET})
        );
    }

    /// A read nothing bounds is never put in the block; a skill, whose body
    /// comes whole, says how long it is; two cues to one call are one item.
    #[test]
    fn a_read_nothing_bounds_is_left_out_and_a_skill_says_its_size() {
        let v = bounded();
        let reply = json!({
            "hint": "see seam_report, or fetch the root-cause skill",
            "next": ["call loop_status"],
            "loop_hint": "loop_status says what is owed",
        });
        let b = block(&v.offers(&reply, "some_read"), &v, 30_000).expect("a block");
        assert_eq!(
            cues_of(&b),
            vec!["loop_hint/loop_status", "hint/get_skill:root-cause"],
            "{b}"
        );
        assert_eq!(
            b["left_out"], 2,
            "seam_report, and the second loop_status: {b}"
        );
        let skill = &b["items"][1];
        assert_eq!(skill["call"], "get_skill");
        assert_eq!(skill["size_chars"], 18_000);
        assert_eq!(skill["args"]["name"], "root-cause");
    }

    /// BUDGET: at most three cues and BLOCK_MAX_CHARS; a block that does not
    /// fit the room is no block; attached, it never pushes a reply past the
    /// reply's own budget, and a reply that offers nothing is untouched.
    #[test]
    fn the_block_is_held_to_its_budget_and_never_pushes_a_reply_over() {
        let v = bounded();
        let many = json!({
            "next_step": "search_design, then get_node, then loop_status, then detect_gaps, \
                          then get_skill root-cause",
        });
        let offers = v.offers(&many, "x");
        assert_eq!(offers.len(), 5);
        let b = block(&offers, &v, 30_000).expect("a block");
        assert_eq!(b["items"].as_array().unwrap().len(), BLOCK_MAX_CUES);
        assert_eq!(b["left_out"], 2);
        assert!(crate::reply_budget::chars_of(&b) <= BLOCK_MAX_CHARS, "{b}");
        assert!(block(&offers, &v, 40).is_none(), "no room is no block");

        // Attached at the top level, inside the reply's budget.
        let small = json!({"ok": true, "loop_hint": "loop_status says what is owed"});
        let mut r = rmcp::model::CallToolResult::structured(small.clone());
        attach_block(&mut r, &v.offers(&small, "add_requirement"), &v, 30_000);
        let got = r.structured_content.as_ref().unwrap();
        assert_eq!(cues_of(&got[BLOCK_KEY]), vec!["loop_hint/loop_status"]);
        assert_eq!(
            got["loop_hint"], small["loop_hint"],
            "the field is untouched"
        );

        // A reply already near its budget gets only what fits, and never more.
        let near = json!({"loop_hint": "loop_status says what is owed", "pad": "x".repeat(3_900)});
        let budget = crate::reply_budget::chars_of(&near) + 40;
        let mut r = rmcp::model::CallToolResult::structured(near.clone());
        attach_block(&mut r, &v.offers(&near, "get_node"), &v, budget);
        let got = r.structured_content.as_ref().unwrap();
        assert!(
            got.get(BLOCK_KEY).is_none(),
            "nothing fits in 40 characters"
        );
        assert!(crate::reply_budget::chars_of(got) <= budget);
        let roomy = crate::reply_budget::chars_of(&near) + 200;
        let mut r = rmcp::model::CallToolResult::structured(near.clone());
        attach_block(&mut r, &v.offers(&near, "get_node"), &v, roomy);
        let got = r.structured_content.as_ref().unwrap();
        assert!(got.get(BLOCK_KEY).is_some());
        assert!(
            crate::reply_budget::chars_of(got) <= roomy,
            "{}",
            crate::reply_budget::chars_of(got)
        );

        // Nothing offered, a refusal, a document: untouched.
        let plain = json!({"node": {"id": "req:a"}});
        let mut r = rmcp::model::CallToolResult::structured(plain.clone());
        attach_block(&mut r, &v.offers(&plain, "get_node"), &v, 30_000);
        assert_eq!(r.structured_content.as_ref().unwrap(), &plain);
        let mut refusal = rmcp::model::CallToolResult::structured_error(small.clone());
        attach_block(&mut refusal, &v.offers(&small, "x"), &v, 30_000);
        assert!(
            refusal
                .structured_content
                .as_ref()
                .unwrap()
                .get(BLOCK_KEY)
                .is_none()
        );
        let mut doc = rmcp::model::CallToolResult::success(vec![]);
        attach_block(&mut doc, &v.offers(&small, "x"), &v, 30_000);
        assert!(doc.structured_content.is_none());
    }
}
