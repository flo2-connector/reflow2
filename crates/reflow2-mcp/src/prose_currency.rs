//! Say when a status has advanced past the prose that describes it.
//!
//! # The finding this exists to fix
//!
//! A node's `status` and its own `description` are two claims in ONE node, and
//! nothing kept them honest with each other. Reported by dev_storyflow
//! 2026-09-02 and recorded as
//! `fact:defect-a-status-can-advance-past-its-own-prose-and-nothing-says-so`:
//! they wrote a capability whose description said, in capitals, *"THE DROPLET
//! STILL RUNS THE OLD SCRIPT"*, installed the fix TWENTY MINUTES LATER, called
//! `set_capability_status(realized)` — and the description still said the
//! droplet ran the old script. The status said delivered, the prose said not
//! started, nothing flagged it, and they caught it only by re-reading their own
//! writing.
//!
//! ⭐ THE ROT HAPPENED INSIDE THE GRAPH, INSIDE ONE NODE, INSIDE TWENTY
//! MINUTES, to someone who had read `epoch:2026_08_17_state_documents_rot_
//! silently` — an entire epoch about state-describing text going stale — THAT
//! SAME DAY. Their conclusion is the one this module is built on: vigilance is
//! not the countermeasure, something mechanical has to be.
//!
//! # Why a sentence in a reply, and not a detector
//!
//! The reporter's own proposal, and it is right: a `set_*_status` call ALREADY
//! KNOWS the description was not touched, because changing the status is the
//! whole of what it does. So this needs no gap, no nudge, no sweep and no
//! second call — one block in the reply the caller is already reading, at the
//! exact moment the divergence is created.
//!
//! A detector would find the same thing later, on a graph where the prose has
//! already been quoted back to somebody. This finds it while the author is
//! still in the room.
//!
//! # What it deliberately does NOT do
//!
//! - **It never judges the prose.** It cannot read English, and a description
//!   that is still perfectly true after a status change is the common case. It
//!   states the FACT — this prose was written while the status was X, and the
//!   status is now Y — and asks. `dec:report-dont-judge`.
//! - **It is silent when the status did not move.** Re-setting a status to what
//!   it already was creates no divergence, and a block that appears on every
//!   call is the noise `with_capture_notes`' siblings are explicitly built not
//!   to become.
//! - **It is silent when there is no prose.** Nothing to have gone stale.

use serde::Serialize;
use serde_json::{Map as JsonMap, Value as JsonValue};

/// How much of the prose to quote back. Enough to recognise what it claims,
/// short enough that it does not bury the reply it is attached to.
const EXCERPT_CHARS: usize = 240;

/// The prose fields a status can outrun, in the order they are looked for.
///
/// `description` covers Capability and DesignEpoch; `statement` covers the
/// types that carry their prose under that name. The FIRST non-empty one wins —
/// a node carrying both is describing itself twice and either is enough to make
/// the point.
const PROSE_FIELDS: [&str; 2] = ["description", "statement"];

/// Attach the currency note to a `set_*_status` reply, if there is one to make.
///
/// `prior_status` is what the node said BEFORE this call — read it before the
/// write, because after the write there is nothing left to compare against.
pub fn with_prose_currency<T: Serialize>(
    value: T,
    prior_status: Option<&str>,
) -> Result<JsonValue, serde_json::Error> {
    let mut v = serde_json::to_value(value)?;
    let Some(note) = currency_note(&v, prior_status) else {
        return Ok(v);
    };
    if let Some(obj) = v.as_object_mut() {
        obj.insert("prose_currency".into(), JsonValue::Object(note));
    }
    Ok(v)
}

/// The note itself, or `None` when nothing diverged. Split out so it is
/// testable without building a whole reply.
pub fn currency_note(
    v: &JsonValue,
    prior_status: Option<&str>,
) -> Option<JsonMap<String, JsonValue>> {
    let props = v.get("properties")?.as_object()?;
    let now = props.get("status")?.as_str()?;

    // Silent unless the status actually MOVED. Re-setting a status to what it
    // already was creates no divergence to report.
    let prior = prior_status?;
    if prior == now {
        return None;
    }

    let (field, prose) = PROSE_FIELDS.iter().find_map(|f| {
        let s = props.get(*f)?.as_str()?;
        (!s.trim().is_empty()).then_some((*f, s))
    })?;

    let mut note = JsonMap::new();
    note.insert("field".into(), JsonValue::String(field.to_string()));
    note.insert(
        "written_under_status".into(),
        JsonValue::String(prior.to_string()),
    );
    note.insert("status_now".into(), JsonValue::String(now.to_string()));
    note.insert("excerpt".into(), JsonValue::String(excerpt(prose)));
    note.insert(
        "note".into(),
        JsonValue::String(format!(
            "`status` moved {prior} -> {now}, and this call did not touch `{field}` — so that \
             prose was written while the status was {prior}. It is quoted above so you can judge \
             it here rather than in another call. DOES IT STILL READ TRUE? Nothing checks this: \
             a status and a description are two claims in one node, and only a person can say \
             whether they still agree."
        )),
    );
    Some(note)
}

/// First [`EXCERPT_CHARS`] characters, cut on a CHAR boundary and marked when
/// cut. Slicing bytes would panic on the first non-ASCII description, and this
/// project's prose is full of them.
fn excerpt(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() <= EXCERPT_CHARS {
        return trimmed.to_string();
    }
    let head: String = trimmed.chars().take(EXCERPT_CHARS).collect();
    format!("{head}… (cut at {EXCERPT_CHARS} chars)")
}

// ─────────────────────────────────────────────────────────────────────────────
// The sibling: prose that still says the question is open, under a Decision
// that has settled it.
// ─────────────────────────────────────────────────────────────────────────────
//
// # The finding this exists to fix
//
// Recorded 2026-09-09 as `fact:a-settled-question-was-re-asked-because-the-
// requirements-prose-outlived-its-decision`. A requirement's statement ended
// "NOT YET DECIDED: … that is his call to make explicitly rather than mine to
// infer." The call had been made three weeks earlier, in a Decision that was
// ACCEPTED and already `GOVERNED_BY`-linked to that very requirement. A session
// read the paragraph, believed the fork open, and put a settled question to the
// owner a second time.
//
// ⭐ THE CAUSE IS NOT AN INADEQUATE SEARCH. The session did search, and
// `search_design` returned the requirement, the rule, the capability, the
// artifact and the verification — and not the governing Decision, whose name
// shares few tokens with any query about the requirement. BM25 over what a node
// SAYS does not reliably surface the node that GOVERNS it. The edge was there
// the whole time and nothing read it.
//
// # Why a reply and not a detector, measured rather than assumed
//
// Over reflow2's own design, 43 of 640 governed nodes carry one of these
// markers somewhere in their prose, and a good share are legitimate — a node
// that QUOTES an old question, or discusses one elsewhere in a long statement.
// (Measured with exactly the nine below, after the list was tightened; a looser
// draft list gave 48, and the wider number was quoted in three places before
// being re-measured against what actually shipped. Three of the nine —
// `to be decided`, `not been decided`, `awaiting a decision` — match NOTHING in
// this corpus and are kept as phrases other projects write, not as dead weight
// this one has evidence for.)
// As a sweep that is 48 findings of mixed quality, which is the noise that gets
// switched off in a week. Fired at the moment somebody ACCEPTS the decision or
// DRAWS the edge, it is one node in front of the person who just moved it, and
// the same reasoning the status-side sibling above was built on.
//
// # What it deliberately does NOT do
//
// - **It never says the prose is wrong.** It cannot read English. A statement
//   that quotes an old question, or leaves a genuinely different question open,
//   is a correct node and a false positive here. It states which phrase matched
//   and quotes the text; a person judges. `dec:report-dont-judge`.
// - **It is silent unless the decision actually SETTLED.** `rejected` and
//   `superseded` retire rather than settle, and `accepted` -> `accepted` moves
//   nothing.
// - **It is silent when the governed prose carries no marker**, which is the
//   overwhelmingly common case and the one that decides whether this stays on.

/// Phrases that ASSERT a question is still open.
///
/// Deliberately phrases ANY project would write, not this one's idioms —
/// reflow2 is built for other people's designs (`rule:reflow2-is-built-for-
/// other-projects-not-for-itself`), so "that is his call to make" is out
/// however exactly it matched the incident. Tuned against the live design:
/// `unresolved` and `tbd` were dropped for firing on prose about everything
/// else, and each addition costs a false positive somewhere.
const OPEN_QUESTION_MARKERS: [&str; 9] = [
    "not yet decided",
    "to be decided",
    "not been decided",
    "undecided",
    "still open",
    "remains open",
    "open question",
    "not settled",
    "awaiting a decision",
];

/// Prose fields a governed node may carry, first non-empty wins.
///
/// Wider than [`PROSE_FIELDS`] because anything can be governed: `statement`
/// covers Requirement and DesignRule, `description` Capability and Component,
/// `decision` a Decision governed by another Decision.
const GOVERNED_PROSE_FIELDS: [&str; 4] = ["statement", "description", "decision", "rationale"];

/// Which markers this prose carries, in the order they are declared.
pub fn open_question_markers(prose: &str) -> Vec<&'static str> {
    let low = prose.to_lowercase();
    OPEN_QUESTION_MARKERS
        .iter()
        .copied()
        .filter(|m| low.contains(m))
        .collect()
}

/// One governed node whose prose still reads as an open question.
pub struct OpenProse {
    pub node_id: String,
    pub node_type: String,
    pub field: &'static str,
    pub markers: Vec<&'static str>,
    pub excerpt: String,
}

/// Examine one governed node's prose. `None` when it carries none, or none of
/// it reads open.
///
/// Takes a LOOKUP rather than a property map so this module keeps depending on
/// nothing but `serde_json` — a stored node's properties are a `HashMap` and a
/// serialised one is a `serde_json::Map`, and both callers exist.
pub fn open_prose<'a>(
    node_id: &str,
    node_type: &str,
    prose_field: impl Fn(&str) -> Option<&'a str>,
) -> Option<OpenProse> {
    // The body first, as before; then the NAME, which no check read until
    // 2026-09-29 — a heading is the prose a reader meets first, in every list
    // (fact:root-cause-a-settled-decisions-name-still-reads-open-because-the-
    // 09-19-fix-reached-the-body-and-no-check-reads-names-2026-09-29).
    let body = GOVERNED_PROSE_FIELDS.iter().find_map(|f| {
        let s = prose_field(f)?;
        (!s.trim().is_empty()).then_some((*f, s))
    });
    let name = prose_field("name")
        .filter(|s| !s.trim().is_empty())
        .map(|s| ("name", s));
    [body, name]
        .into_iter()
        .flatten()
        .find_map(|(field, prose)| {
            let markers = open_question_markers(prose);
            let first = *markers.first()?;
            Some(OpenProse {
                node_id: node_id.to_string(),
                node_type: node_type.to_string(),
                field,
                excerpt: excerpt_around(prose, first),
                markers,
            })
        })
}

/// The block, or `None` when nothing governed reads open.
pub fn settled_question_block(
    decision_id: &str,
    hits: &[OpenProse],
) -> Option<JsonMap<String, JsonValue>> {
    if hits.is_empty() {
        return None;
    }
    let governs: Vec<JsonValue> = hits
        .iter()
        .map(|h| {
            let mut m = JsonMap::new();
            m.insert("node_id".into(), JsonValue::String(h.node_id.clone()));
            m.insert("node_type".into(), JsonValue::String(h.node_type.clone()));
            m.insert("field".into(), JsonValue::String(h.field.to_string()));
            m.insert(
                "markers".into(),
                JsonValue::Array(
                    h.markers
                        .iter()
                        .map(|s| JsonValue::String((*s).to_string()))
                        .collect(),
                ),
            );
            m.insert("excerpt".into(), JsonValue::String(h.excerpt.clone()));
            JsonValue::Object(m)
        })
        .collect();

    let n = hits.len();
    let mut note = JsonMap::new();
    note.insert("governs".into(), JsonValue::Array(governs));
    note.insert(
        "decision_id".into(),
        JsonValue::String(decision_id.to_string()),
    );
    note.insert(
        "note".into(),
        JsonValue::String(format!(
            "`{decision_id}` is now accepted, and {n} node(s) it governs still carry prose \
             asserting the question is open — quoted above with the phrase that matched, so you \
             can judge it here rather than in another call. DOES THAT PROSE STILL READ TRUE? \
             Nothing checks this and nothing here claims the text is wrong: prose that QUOTES an \
             old question, or leaves a DIFFERENT one open, is a correct node and looks identical \
             from the outside. Only a person can tell. If it has been overtaken, the settled \
             answer belongs in the node where the next reader will meet it — a paragraph saying \
             a question is open outlives the decision that closed it, and the next session \
             believes the paragraph."
        )),
    );
    Some(note)
}

/// [`EXCERPT_CHARS`] of prose CENTRED ON THE MATCH rather than taken from the
/// front.
///
/// The head is the wrong window here: the incident's marker sat in the LAST
/// paragraph of a 5 KB statement, so a leading excerpt would have quoted the
/// opening and shown the reader nothing of what fired. `dec:idea-prose-currency-
/// quotes-enough-to-judge-without-a-second-read` is the standing question about
/// the sibling's own 240-char head cut; this side answers it by moving the
/// window instead of widening it.
fn excerpt_around(prose: &str, marker: &str) -> String {
    let trimmed = prose.trim();
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= EXCERPT_CHARS {
        return trimmed.to_string();
    }
    // Locate the match on the lowercased CHAR sequence, so the index is a char
    // index and slicing can never split a multi-byte character.
    let low: Vec<char> = trimmed.to_lowercase().chars().collect();
    let m: Vec<char> = marker.chars().collect();
    let at = low
        .windows(m.len())
        .position(|w| w == m.as_slice())
        .unwrap_or(0);
    // Centre the window on the match, then clamp to the ends.
    let half = EXCERPT_CHARS / 2;
    let start = at.saturating_sub(half).min(chars.len() - EXCERPT_CHARS);
    let end = start + EXCERPT_CHARS;
    let body: String = chars[start..end].iter().collect();
    let head = if start > 0 { "…" } else { "" };
    let tail = if end < chars.len() { "…" } else { "" };
    format!("{head}{body}{tail}")
}

// ─────────────────────────────────────────────────────────────────────────────
// The third copy: a Decision's own NAME, still asking the question it settled.
// ─────────────────────────────────────────────────────────────────────────────
//
// # The finding this exists to fix
//
// `fact:root-cause-a-settled-decisions-name-still-reads-open-because-the-09-19-
// fix-reached-the-body-and-no-check-reads-names-2026-09-29`. The brainstorm
// skill names an idea as its open question — "OPEN — does X…?" — so the status
// is COPIED INTO THE NAME at birth, and no settle act updated the copy. flo2 F12
// (2026-09-19) reported exactly this; the fix gave `set_decision_status` a
// `chose` field for the body and nothing for the name, and neither check above
// read `name`. It recurred on 2026-09-29 (six settled decisions still named
// "OPEN —", 30 KB of replace_text to rename them), and reflow2's own design held
// 44 of 266 accepted decisions so named.
//
// # Why this one is precise where the markers above are not
//
// The phrase markers read English and fire on prose that QUOTES a question.
// This reads one thing: the leading STATUS WORD the convention writes, in the
// capitals it writes it in, against the node's own `status`. `OPEN` in caps as
// the first word of an accepted decision's name is a stale copy of a status,
// not a sentence to judge — which is why this may also run as a sweep
// (`settled_decision_named_open`) where the markers could not.
//
// A lower-case "Open the API to partners" is a decision ABOUT opening something
// and is deliberately not matched; "open question" in any case is.

/// Whether `name` begins with the status word the brainstorm convention writes
/// for an open question. ONE definition, in the core, shared with the sweep
/// (`settled_decision_named_open`) so the reply and the sweep cannot disagree.
#[must_use]
pub fn name_leads_with_open(name: &str) -> bool {
    reflow2_core::name_leads_with_open(name)
}

/// The block for a reply, or `None` when the name agrees with the status.
///
/// Fires only when the decision stands at `accepted` — the one status that
/// closes a question. A `proposed` decision named "OPEN —" is telling the
/// truth, and a `deferred` one is set aside with its question still open.
#[must_use]
pub fn name_still_reads_open(
    name: Option<&str>,
    status: Option<&str>,
) -> Option<JsonMap<String, JsonValue>> {
    if status != Some("accepted") {
        return None;
    }
    let name = name?;
    if !name_leads_with_open(name) {
        return None;
    }
    let mut block = JsonMap::new();
    block.insert("name".into(), JsonValue::String(name.to_string()));
    block.insert("status".into(), JsonValue::String("accepted".into()));
    block.insert(
        "note".into(),
        JsonValue::String(
            "This decision is `accepted` and its NAME still begins as an open question — a copy \
             of its status written when it was one, which no settle act updates. Anything that \
             lists decisions by name (search, what_next, a hub, where-am-i) will present a \
             settled question as open. Retitle it in the settling call: pass `name` to \
             set_decision_status. Or rename it now with replace_text on field `name`. Nothing \
             is renamed for you — the name is the owner's record."
                .to_string(),
        ),
    );
    Some(block)
}

#[cfg(test)]
mod name_tests {
    use super::*;

    #[test]
    fn the_convention_is_matched_and_a_decision_about_opening_is_not() {
        assert!(name_leads_with_open("OPEN — does X hold?"));
        assert!(name_leads_with_open("  OPEN: which store?"));
        assert!(name_leads_with_open("Open question — which store?"));
        assert!(!name_leads_with_open("Open the API to partners"));
        assert!(!name_leads_with_open("OPENING the second site"));
        assert!(!name_leads_with_open("We keep it OPEN — for now"));
    }

    #[test]
    fn only_an_accepted_decision_is_stale() {
        assert!(name_still_reads_open(Some("OPEN — x?"), Some("accepted")).is_some());
        assert!(name_still_reads_open(Some("OPEN — x?"), Some("proposed")).is_none());
        assert!(name_still_reads_open(Some("OPEN — x?"), Some("deferred")).is_none());
        assert!(name_still_reads_open(Some("Chosen: x"), Some("accepted")).is_none());
    }
}
