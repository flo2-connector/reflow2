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
//! `hint/get_skill:detect-and-ask`, `instructions/graph_report_markdown`.
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

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// The reply fields whose strings are cues — the four the baseline counted,
/// plus `next_step`, the latent surface's spelling of the same thing.
pub const CUE_FIELDS: [&str; 5] = ["loop_hint", "hint", "next", "nudge", "next_step"];

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
pub const INSTRUCTIONS_OFFER: [&str; 2] = [
    "instructions/graph_report_markdown",
    "instructions/search_design",
];

/// The agent-side cue itself: the sentence the connect-time instructions
/// carry, and the kit's session-start hooks repeat (`tools/loop_nudge.py`).
/// It names what reflow2 already serves; reflow2 serves no compact read of a
/// design's CURRENT VALUES, which is recorded as a finding, not built here.
pub const FIRST_READ_CUE: &str = "TO LEARN WHERE THIS DESIGN STANDS, read `graph_report_markdown` \
     first (a few KB: what the design holds, what is delivered, the top open gaps) before \
     anything longer; if the design keeps its own record of where things stand, `search_design` \
     finds it (\"current state\", \"status as of\").";

/// The words a cue may name: the served tools and the served skills.
#[derive(Debug, Clone, Default)]
pub struct Vocabulary {
    pub tools: BTreeSet<String>,
    pub skills: BTreeSet<String>,
}

impl Vocabulary {
    pub fn new(
        tools: impl IntoIterator<Item = String>,
        skills: impl IntoIterator<Item = String>,
    ) -> Self {
        Vocabulary {
            tools: tools.into_iter().collect(),
            skills: skills.into_iter().collect(),
        }
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
    /// except the tool that was just called (`called`). Sorted, deduplicated,
    /// at most [`MAX_PER_REPLY`].
    pub fn offered(&self, reply: &Value, called: &str) -> Vec<String> {
        let mut found: BTreeSet<String> = BTreeSet::new();
        self.walk(reply, &mut found, called);
        found.into_iter().take(MAX_PER_REPLY).collect()
    }

    fn walk(&self, v: &Value, found: &mut BTreeSet<String>, called: &str) {
        match v {
            Value::Object(o) => {
                for (k, child) in o {
                    // A node's properties are the design's content, never a cue.
                    if k == "properties" {
                        continue;
                    }
                    if CUE_FIELDS.contains(&k.as_str()) {
                        let mut texts = Vec::new();
                        strings(child, &mut texts);
                        for t in texts {
                            for target in self.named_calls(t) {
                                if target != called {
                                    found.insert(format!("{k}/{target}"));
                                }
                            }
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

/// Whether a ledger call line made the call a cue target names.
fn acts_on(line: &crate::usage::UsageLine, target: &str) -> bool {
    match target.strip_prefix("get_skill:") {
        Some(skill) => {
            line.tool.as_deref() == Some("get_skill") && line.skill.as_deref() == Some(skill)
        }
        None => line.tool.as_deref() == Some(target),
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
    /// cue id → its counts, every cue offered in the window.
    pub by_cue: BTreeMap<String, CueCount>,
    pub note: &'static str,
}

const NOTE: &str = "A cue is a sentence in a reply's loop_hint, hint, next, nudge or next_step \
     naming a served tool or skill other than the one just called, or the first-read cue the \
     connect-time instructions carry (`instructions/…`, offered once per session at the \
     handshake). Its id is `<field>/<named call>`. FOLLOWED means the same session made that call \
     later in the window: within its next 5 calls, or at all. A session is the seat that made the \
     calls; every `reflow2-mcp --call` door call has a seat of its own, so the door's calls are \
     followed as one sequence in ledger order. UNATTRIBUTABLE: offered on a session that made no \
     other call in the window (a per-request seat), so nothing could follow it; not counted as \
     ignored. Not counted: the kit's session-start hook repeats the first-read cue outside the \
     server, where no seat is known. Verbs only: no sentence, node id or argument is kept.";

/// The session a ledger line belongs to: its seat, except that every door
/// call (`reflow2-mcp --call`) runs in a process of its own and so has a seat
/// of its own — the door's calls are one sequence, in ledger order.
fn session_of(line: &crate::usage::UsageLine) -> String {
    if line.client.as_deref() == Some(crate::service::CALL_DOOR_CLIENT) {
        return "door".to_string();
    }
    line.seat.clone().unwrap_or_else(|| "unknown".to_string())
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
            for id in &l.cues {
                let target = target_of(id);
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
            line("cue", "", "s1", &["instructions/graph_report_markdown"]),
            line("call", "graph_report_markdown", "s1", &[]),
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

        let first = &t.by_cue["instructions/graph_report_markdown"];
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

    /// The instructions sentence names exactly the calls counted for it.
    #[test]
    fn the_first_read_sentence_names_what_the_handshake_counts() {
        let v = Vocabulary::new(
            ["graph_report_markdown", "search_design"].map(String::from),
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
}
