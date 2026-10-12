//! ONE place a reply is bounded, so a tenth tool cannot quietly miss it.
//!
//! ⭐ WHY THIS MODULE EXISTS AT ALL. Measured 2026-09-13 against reflow2's own
//! 4,403-node design (`fact:the-unbounded-reply-sweep-2026-09-13`): of 181
//! served tools, THREE accepted a reply budget — `detect_gaps`, `reviewed_gaps`
//! and `topic_report` — and each of the three had rolled its own trimming by
//! hand, after somebody met that tool's overflow personally. Nine other read
//! tools returned more than the 30,000-character default with no bound at all:
//!
//! ```text
//!     126,377  confirmation_ledger     4.2x
//!     118,246  describe_schema         3.9x
//!      89,238  invalidated_findings    3.0x
//!      76,155  evidence_report         2.5x
//!      61,669  get_instructions        2.1x
//!      57,449  detect_defects          1.9x
//!      34,319  export_surface          1.1x
//!      30,489  list_skills             1.0x
//!       9-16MB compare_designs         (on two real exports)
//! ```
//!
//! Three fixed one at a time against 178 not fixed is the instance-not-class
//! pattern `fact:the-parallel-batch-class-recurred-because-only-its-instance-was-fixed`
//! already records on this project. A fourth bespoke trimmer would have been a
//! fourth instance, so the trimming lives here instead.
//!
//! 🛑 THE FAILURE THIS STOPS IS NOT A SLOW CALL. It is the CLIENT refusing the
//! reply, at which point the session sees a wall of harness text and reflow2
//! never gets to suggest narrowing. A reader who receives nothing cannot be
//! told to narrow their question.
//!
//! ⭐ THE RULE THE TRIM FOLLOWS: **a shorter answer is never a quieter one.**
//! Numbers are never touched — every count, total and tally survives at full
//! precision. What is withheld is PROSE, and only then list length — every
//! list by the characters it costs, each one named — and the reply always says
//! which happened and how much it dropped. The reply that has to fit is the
//! one the client receives, its `budget` block included. That ordering is
//! deliberate: on every tool measured above, the bytes are in long `statement`,
//! `rationale`, `reason`, `note` and `description` fields, while the part a
//! reader acts on is the counts and the ids.
//!
//! ⚠️ WHAT THIS DOES NOT COVER. `export_graph` is excluded on purpose and must
//! stay excluded: its job IS the whole document, and a truncated export is not
//! a smaller answer but a corrupt one. It needs streaming or a file path, which
//! is a different change. See the same finding.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

/// The default ceiling, re-exported so a caller never hard-codes `30_000`
/// the way `reviewed_gaps` did before this module existed.
pub use reflow2_core::detect::DEFAULT_REPLY_BUDGET_CHARS;

/// The parameter block for a tool whose ONLY argument is its reply budget.
///
/// Tools that already take arguments grow a `budget_chars` field of their own
/// instead; this exists for the ones that took none at all — which was most of
/// the overflowing set, and the reason they had nowhere to put a bound.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BudgetReq {
    /// How many characters of JSON this reply may spend before prose is
    /// withheld to make it fit (default 30,000).
    ///
    /// RAISE IT ONLY IF YOU KNOW THIS CLIENT HAS THE ROOM. The default sits
    /// below the smallest tool-output cap in use, because the failure it
    /// exists to stop is not a slow call — it is the CLIENT refusing the reply,
    /// at which point the session sees a wall of harness text and reflow2 never
    /// gets to suggest narrowing.
    ///
    /// Every trimmed reply says what it withheld, at `budget`; counts and ids
    /// are never budgeted away, so a shorter answer is never a quieter one.
    #[serde(default)]
    pub budget_chars: Option<usize>,
}

impl BudgetReq {
    /// The budget this request asks for, or the default.
    #[must_use]
    pub fn budget(&self) -> usize {
        self.budget_chars.unwrap_or(DEFAULT_REPLY_BUDGET_CHARS)
    }
}

/// String lengths tried in order before list length is touched at all.
///
/// Prose is cut progressively rather than in one step so a reply that is
/// slightly over budget keeps nearly all of its text, and only a reply that is
/// wildly over loses most of it. The last rung matches the 240 characters
/// `reviewed_gaps` chose by hand, so behaviour there is unchanged.
const PROSE_RUNGS: [usize; 6] = [4000, 2000, 800, 240, 120, 0];

/// A string this short is NEVER trimmed, at any rung — it is a name, an id, an
/// enum value or a status, not prose.
///
/// ⭐ THIS IS WHAT MAKES THE 0 RUNG SAFE, and the 0 rung is what lets a reply
/// keep a COMPLETE list while dropping the prose hanging off it. Without the
/// floor, trimming to 0 would blank every type name in `describe_schema` and
/// leave a vocabulary of `…` — complete in shape and empty of content, which is
/// the quiet-answer failure in its purest form. Measured: reflow2's longest id
/// is well under this, and a `sha256:` hash is 71.
const NEVER_TRIM_BELOW: usize = 80;

/// Serialized size of a value, in characters — the same measure every budget
/// in this codebase is stated in.
#[must_use]
pub fn chars_of(v: &Value) -> usize {
    v.to_string().len()
}

/// Truncate every string longer than `limit`, recursively, leaving numbers,
/// booleans, nulls and object KEYS untouched.
///
/// Keys are preserved because they are structure rather than prose: a reader
/// who gets `statement` back as `stat…` cannot tell what was withheld from
/// what was never there. Short strings are left alone, which is what keeps ids
/// intact — every id convention in this design (`req:`, `cap:`, `sha256:…`) is
/// far below the shortest rung.
fn truncate_prose(v: &Value, limit: usize, cut: &mut usize) -> Value {
    match v {
        Value::String(s) => {
            let n = s.chars().count();
            if n > limit && n > NEVER_TRIM_BELOW {
                *cut += 1;
                let kept: String = s.chars().take(limit).collect();
                Value::String(format!("{kept}…"))
            } else {
                v.clone()
            }
        }
        Value::Array(a) => Value::Array(a.iter().map(|x| truncate_prose(x, limit, cut)).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, x)| (k.clone(), truncate_prose(x, limit, cut)))
                .collect(),
        ),
        _ => v.clone(),
    }
}

/// One list the budget may shorten: an array reached through object keys
/// alone (`design.added`, `edges_removed`), never one nested inside another
/// list's element — a sub-list inside an item is part of that item, and
/// shortening it would leave the item quietly incomplete.
struct ListSlot {
    path: String,
    /// What each element costs on the wire, in characters, in order.
    costs: Vec<usize>,
    /// How many entries are kept, from the front.
    keep: usize,
}

impl ListSlot {
    fn total(&self) -> usize {
        self.costs.len()
    }
    /// The characters the kept entries cost, commas included.
    fn kept_chars(&self) -> usize {
        self.costs[..self.keep].iter().sum::<usize>() + self.keep.saturating_sub(1)
    }
}

/// Every list the budget may shorten, with what each entry costs.
fn collect_lists(v: &Value, path: &str, out: &mut Vec<ListSlot>) {
    match v {
        Value::Array(a) if !a.is_empty() => out.push(ListSlot {
            path: path.to_string(),
            costs: a.iter().map(chars_of).collect(),
            keep: a.len(),
        }),
        Value::Object(o) => {
            for (k, x) in o {
                let p = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                collect_lists(x, &p, out);
            }
        }
        _ => {}
    }
}

/// Take `excess` characters out of the lists, by the characters they cost:
/// the list costing the most gives first, one entry from its tail at a time,
/// so a short list stays whole for as long as a long one can still give. Stops
/// at a floor of one entry per list. Returns how much was taken.
fn shorten_by_chars(lists: &mut [ListSlot], mut excess: usize) -> usize {
    use std::collections::BinaryHeap;
    let mut heap: BinaryHeap<(usize, usize)> = lists
        .iter()
        .enumerate()
        .filter(|(_, l)| l.keep > 1)
        .map(|(i, l)| (l.kept_chars(), i))
        .collect();
    let mut taken = 0usize;
    while excess > 0 {
        let Some((_, i)) = heap.pop() else { break };
        let l = &mut lists[i];
        // The entry's characters and the comma before it.
        let saved = l.costs[l.keep - 1] + 1;
        l.keep -= 1;
        taken += saved;
        excess = excess.saturating_sub(saved);
        if l.keep > 1 {
            heap.push((l.kept_chars(), i));
        }
    }
    taken
}

/// `body` with every shortened list capped at what it keeps.
fn apply_caps(body: &Value, lists: &[ListSlot]) -> Value {
    let mut out = body.clone();
    for l in lists.iter().filter(|l| l.keep < l.total()) {
        out = cap_array_at(&out, &l.path, l.keep);
    }
    out
}

/// Cap the array at `path` to `keep` entries.
fn cap_array_at(v: &Value, path: &str, keep: usize) -> Value {
    fn walk(v: &Value, parts: &[&str], keep: usize) -> Value {
        if parts.is_empty() {
            if let Value::Array(a) = v {
                return Value::Array(a.iter().take(keep).cloned().collect());
            }
            return v.clone();
        }
        match v {
            Value::Object(o) => Value::Object(
                o.iter()
                    .map(|(k, x)| {
                        if k == parts[0] {
                            (k.clone(), walk(x, &parts[1..], keep))
                        } else {
                            (k.clone(), x.clone())
                        }
                    })
                    .collect(),
            ),
            _ => v.clone(),
        }
    }
    let parts: Vec<&str> = path.split('.').filter(|p| !p.is_empty()).collect();
    walk(v, &parts, keep)
}

/// What a bounded reply says about itself, assembled once per candidate so
/// its own size is counted against the budget — the client receives the block
/// too, so a reply that fit "before its budget block" did not fit.
struct Block<'a> {
    budget: usize,
    full_chars: usize,
    prose_limit: usize,
    fields_trimmed: usize,
    list_note: String,
    lists_shortened: Vec<Value>,
    hint: &'a str,
}

impl Block<'_> {
    fn value(&self, reply_chars: usize) -> Value {
        let note = format!(
            "WITHHELD TO FIT. The whole answer is {} characters against a budget of {}, which \
             is the size at which a client refuses the reply outright — and a reader who \
             receives nothing cannot be told to narrow their question. What was withheld is \
             PROSE: {} text field(s) cut to {} characters. Every number, count and id is \
             untouched and at full precision, so a shorter answer here is never a quieter \
             one.{} Raise `budget_chars` if this client has the room. {}",
            self.full_chars,
            self.budget,
            self.fields_trimmed,
            self.prose_limit,
            self.list_note,
            self.hint
        );
        let mut block = json!({
            "applied": true,
            "budget_chars": self.budget,
            "full_chars": self.full_chars,
            "reply_chars": reply_chars,
            "prose_limit": self.prose_limit,
            "fields_trimmed": self.fields_trimmed,
            "note": note,
        });
        if !self.lists_shortened.is_empty() {
            block["lists_shortened"] = Value::Array(self.lists_shortened.clone());
        }
        block
    }

    /// The reply: `body` carrying the block, its `reply_chars` the size of
    /// the reply it sits in (settled by a short fixpoint, since the number is
    /// part of what it measures).
    fn assemble(&self, body: &Value) -> Value {
        let mut reply_chars = chars_of(body);
        let mut out = Value::Null;
        for _ in 0..4 {
            out = match body {
                Value::Object(o) => {
                    let mut o = o.clone();
                    o.insert("budget".to_string(), self.value(reply_chars));
                    Value::Object(o)
                }
                // A bare array or scalar has nowhere to carry the block, so it
                // is wrapped rather than silently returned without its own
                // explanation.
                other => json!({"items": other, "budget": self.value(reply_chars)}),
            };
            let n = chars_of(&out);
            if n == reply_chars {
                break;
            }
            reply_chars = n;
        }
        out
    }
}

/// Bound `full` to `budget` characters, withholding prose first and list
/// length only if prose was not enough.
///
/// Returns the value unchanged when it already fits — including the absence of
/// any `budget` key, so a reply that never needed trimming is byte-identical to
/// what the tool returned before this module existed.
///
/// `hint` is the one tool-specific sentence: where to read the withheld detail
/// (`"read one in full with get_node on its id"`, `"narrow with scope"`). It is
/// required rather than optional because a reply that says what it dropped and
/// not how to get it back has told the reader only the bad half.
/// Whether shortening a LIST is an honest answer for this tool.
///
/// 🛑 THIS IS OPT-IN AND THE DEFAULT IS NO, because the first version of this
/// module capped lists for everybody and a committed test caught it within the
/// hour: `describe_schema` went from 65 edge types to ONE, still shaped like a
/// complete vocabulary. That is the exact failure this module's own rule
/// forbids — a shorter answer that is also a quieter one — and no note attached
/// to a gutted list makes it safe, because the caller's next move is to believe
/// the design has one edge type.
///
/// Say `Sampling::Allowed` only where a partial list still answers the question
/// asked and a `total` beside it makes the sample legible. Say `Forbidden`
/// wherever completeness IS the answer: a vocabulary, a catalogue an agent
/// picks from, a list of everything that exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sampling {
    /// A partial list plus its true count still answers the question.
    Allowed,
    /// Completeness is the answer. Over budget, overflow LOUDLY instead.
    Forbidden,
}

/// Bound a reply, trimming prose only — never list length.
///
/// The right default: on every tool measured, the bytes are in long prose
/// fields while the part a reader acts on is the counts, the ids and the
/// completeness of the list.
#[must_use]
pub fn bound_reply(full: Value, budget: usize, hint: &str) -> Value {
    bound_reply_with(full, budget, hint, Sampling::Forbidden)
}

/// Bound a reply, and permit shortening its lists if prose alone could not
/// make it fit: every list gives by the characters it costs, the longest
/// first, down to a floor of one entry, and the budget block names each one
/// shortened (`lists_shortened: [{path, shown, total}]`). See [`Sampling`]
/// before reaching for this.
///
/// Until 2026-10-12 only the ONE list with the most entries was shortened,
/// halved down to one entry, and every other list stayed whole: measured on
/// flo2.io, `compare_designs` with a 4,000 budget answered 1,421,523
/// characters, one id list of 608,049 characters among several.
#[must_use]
pub fn bound_reply_sampling(full: Value, budget: usize, hint: &str) -> Value {
    bound_reply_with(full, budget, hint, Sampling::Allowed)
}

#[must_use]
fn bound_reply_with(full: Value, budget: usize, hint: &str, sampling: Sampling) -> Value {
    let full_chars = chars_of(&full);
    if full_chars <= budget {
        return full;
    }
    let mut block = Block {
        budget,
        full_chars,
        prose_limit: *PROSE_RUNGS.last().unwrap(),
        fields_trimmed: 0,
        list_note: String::new(),
        lists_shortened: Vec::new(),
        hint,
    };

    // ① Prose first, progressively. Numbers and ids are never candidates. The
    //    reply that has to fit is the one the client receives, block included.
    let mut body = full.clone();
    for rung in PROSE_RUNGS {
        let mut cut = 0usize;
        body = truncate_prose(&full, rung, &mut cut);
        block.prose_limit = rung;
        block.fields_trimmed = cut;
        if chars_of(&block.assemble(&body)) <= budget {
            return block.assemble(&body);
        }
    }

    // ② Only if prose was not enough, and only where a sample is honest for
    //    this tool: shorten the lists, by the characters each costs, until the
    //    reply fits or every list is at its floor — and name every one.
    if sampling == Sampling::Forbidden {
        block.list_note = format!(
            " THE LIST ITSELF IS NOT shortened AND THE REPLY IS STILL {} CHARACTERS: for this \
             tool completeness IS the answer, so returning part of the list would be a quieter \
             answer wearing a complete one's shape. Narrow the question instead, or raise \
             `budget_chars`.",
            chars_of(&body)
        );
        return block.assemble(&body);
    }
    let mut lists = Vec::new();
    collect_lists(&body, "", &mut lists);
    let mut capped = body.clone();
    // Each round takes the measured excess out of the lists and re-measures,
    // because the block that names them grows as they are named; a round that
    // can take nothing more ends it.
    for _ in 0..32 {
        let out = block.assemble(&capped);
        let now = chars_of(&out);
        if now <= budget {
            return out;
        }
        if shorten_by_chars(&mut lists, now - budget) == 0 {
            // Every list is at its floor of one entry and the reply is still
            // over: what remains is structure, not length.
            block.list_note.push_str(&format!(
                " EVERY LIST IS AT ITS FLOOR OF ONE ENTRY AND THE REPLY IS STILL {now} \
                 CHARACTERS: what remains is its structure, which this budget cannot hold — \
                 raise `budget_chars`, or ask for less."
            ));
            return block.assemble(&capped);
        }
        capped = apply_caps(&body, &lists);
        let shortened: Vec<&ListSlot> = lists.iter().filter(|l| l.keep < l.total()).collect();
        block.lists_shortened = shortened
            .iter()
            .map(|l| json!({"path": l.path, "shown": l.keep, "total": l.total()}))
            .collect();
        let named = shortened
            .iter()
            .map(|l| {
                format!(
                    "`{}` is shown as {} of {} entries",
                    l.path,
                    l.keep,
                    l.total()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        block.list_note = format!(
            " THEN LIST LENGTH, because trimming prose alone still did not fit: {} list(s) \
             shortened by the characters each costs, the longest giving first — {named}. \
             `lists_shortened` names each with `shown` and `total`.",
            shortened.len()
        );
    }
    block.assemble(&capped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn long(n: usize) -> String {
        "x".repeat(n)
    }

    #[test]
    fn a_reply_that_fits_is_returned_byte_identical() {
        let v = json!({"count": 3, "items": ["a", "b"]});
        let out = bound_reply(v.clone(), 30_000, "hint");
        assert_eq!(out, v, "a reply under budget must not be rewritten at all");
        assert!(
            out.get("budget").is_none(),
            "no budget block when nothing was withheld"
        );
    }

    #[test]
    fn prose_is_cut_but_numbers_never_are() {
        let v = json!({"count": 12345, "total": 999, "statement": long(50_000)});
        let out = bound_reply(v, 1_000, "hint");
        assert_eq!(out["count"], 12345, "counts must survive trimming exactly");
        assert_eq!(out["total"], 999);
        assert!(chars_of(&out) <= 2_000, "must be near the budget");
        assert_eq!(out["budget"]["applied"], true);
    }

    #[test]
    fn ids_are_short_enough_to_survive_every_rung() {
        let v = json!({
            "node_id": "req:one-open-design-costs-a-deliberate-amount-of-memory",
            "checksum": "sha256:acccfe9a30a4c45a273299fdf8cc35c8f722ff76dea00e8adf003824f3422e2e",
            "statement": long(80_000),
        });
        let out = bound_reply(v, 500, "hint");
        assert_eq!(
            out["node_id"], "req:one-open-design-costs-a-deliberate-amount-of-memory",
            "an id must never be truncated — it is structure, not prose"
        );
        assert!(out["checksum"].as_str().unwrap().starts_with("sha256:"));
        assert!(!out["checksum"].as_str().unwrap().ends_with('…'));
    }

    #[test]
    fn a_list_is_never_shortened_unless_the_tool_opted_in() {
        // THE REGRESSION THIS PINS. The first version of this module capped
        // lists for every caller, and `describe_schema` came back with ONE of
        // its 65 edge types — still shaped exactly like a whole vocabulary.
        // A committed test caught it; this one keeps it caught.
        let items: Vec<Value> = (0..4000).map(|i| json!({"id": format!("n:{i}")})).collect();
        let v = json!({"count": 4000, "items": items});
        let out = bound_reply(v, 2_000, "narrow the question");
        assert_eq!(
            out["items"].as_array().unwrap().len(),
            4000,
            "completeness is the default: a list must survive whole unless the tool opted in"
        );
        assert!(
            out["budget"]["note"]
                .as_str()
                .unwrap()
                .contains("NOT shortened"),
            "and the reply must say it chose to overflow rather than go quiet"
        );
    }

    #[test]
    fn list_length_is_touched_only_after_prose_and_says_so() {
        // Many rows of SHORT strings: prose trimming cannot help, so the list
        // itself has to give. This is the path that must announce itself.
        let items: Vec<Value> = (0..4000).map(|i| json!({"id": format!("n:{i}")})).collect();
        let v = json!({"count": 4000, "items": items});
        let out = bound_reply_sampling(v, 2_000, "narrow with scope");
        assert_eq!(out["count"], 4000, "the COUNT still reports every row");
        assert!(
            out["budget"]["note"]
                .as_str()
                .unwrap()
                .contains("THEN LIST LENGTH"),
            "a shortened list must be named, not silent"
        );
        assert!(out["items"].as_array().unwrap().len() < 4000);
    }

    #[test]
    fn every_long_list_gives_by_the_characters_it_costs_until_the_reply_fits() {
        // MEASURED 2026-10-11 on flo2.io: compare_designs with budget_chars
        // 4,000 against an unrelated design answered 1,421,523 characters —
        // edges_removed alone was 608,049 and design.added 340,314 — because
        // only the ONE list with the most entries was shortened and every
        // other list stayed whole. Two long lists under one object, short ids:
        // prose trimming cannot help, so both lists have to give.
        let a: Vec<Value> = (0..4000).map(|i| json!({"id": format!("a:{i}")})).collect();
        let b: Vec<Value> = (0..4000).map(|i| json!({"id": format!("b:{i}")})).collect();
        let v = json!({"count_a": 4000, "count_b": 4000, "a": a, "b": b});
        let out = bound_reply_sampling(v, 2_000, "page one list with list and offset");
        assert!(
            chars_of(&out) <= 2_000,
            "the WHOLE reply, budget block included, fits the budget: {} chars",
            chars_of(&out)
        );
        assert_eq!(out["count_a"], 4000, "counts are never budgeted away");
        assert_eq!(out["count_b"], 4000);
        let shortened = out["budget"]["lists_shortened"]
            .as_array()
            .unwrap_or_else(|| panic!("the block names every shortened list: {out}"));
        let mut paths: Vec<&str> = shortened
            .iter()
            .map(|l| l["path"].as_str().expect("a path"))
            .collect();
        paths.sort_unstable();
        assert_eq!(
            paths,
            ["a", "b"],
            "BOTH lists were shortened and named: {out}"
        );
        for l in shortened {
            assert_eq!(l["total"], 4000, "each names its true length: {l}");
            let shown = l["shown"].as_u64().expect("shown") as usize;
            assert!((1..4000).contains(&shown), "{l}");
            let path = l["path"].as_str().unwrap();
            assert_eq!(
                out[path].as_array().map(Vec::len),
                Some(shown),
                "`shown` is what the list holds: {out}"
            );
        }
        let note = out["budget"]["note"].as_str().unwrap();
        assert!(
            note.contains("THEN LIST LENGTH") && note.contains("`a`") && note.contains("`b`"),
            "the note names every shortened list: {note}"
        );
    }

    #[test]
    fn a_shortened_reply_fits_with_its_own_budget_block() {
        // The block is part of the reply the client receives, so it counts.
        let items: Vec<Value> = (0..4000).map(|i| json!({"id": format!("n:{i}")})).collect();
        let v = json!({"count": 4000, "items": items});
        let out = bound_reply_sampling(v, 1_500, "narrow with scope");
        assert!(chars_of(&out) <= 1_500, "{} chars", chars_of(&out));
        assert_eq!(out["budget"]["reply_chars"], chars_of(&out));
        assert!(
            !out["items"].as_array().unwrap().is_empty(),
            "a floor of one entry"
        );
    }

    #[test]
    fn the_hint_always_reaches_the_reader() {
        let v = json!({"statement": long(50_000)});
        let out = bound_reply(v, 500, "read one in full with get_node on its id");
        assert!(
            out["budget"]["note"]
                .as_str()
                .unwrap()
                .contains("read one in full with get_node on its id"),
            "saying what was dropped without saying how to get it back is half an answer"
        );
    }

    #[test]
    fn a_name_survives_the_zero_rung_that_drops_every_hint() {
        // The vocabulary case: describe_schema must come back COMPLETE — every
        // type and edge name present — even when its prose is dropped whole.
        // Without the floor, the 0 rung would blank the names too and return a
        // vocabulary of ellipses that still counted 65 edge types.
        let types: Vec<Value> = (0..200)
            .map(|i| json!({"edge_type": format!("EDGE_TYPE_{i}"), "hint": long(3_000)}))
            .collect();
        let out = bound_reply(
            json!({"edge_types": types}),
            5_000,
            "narrow with from and to",
        );
        let got = out["edge_types"].as_array().unwrap();
        assert_eq!(got.len(), 200, "every edge type must still be listed");
        assert_eq!(
            got[7]["edge_type"], "EDGE_TYPE_7",
            "a NAME is not prose and must survive every rung"
        );
        assert!(
            got[7]["hint"].as_str().unwrap().chars().count() < 100,
            "its hint, which is prose, should be gone"
        );
    }

    #[test]
    fn a_bare_array_is_wrapped_so_it_can_carry_its_own_explanation() {
        let v = json!([{"statement": long(50_000)}]);
        let out = bound_reply(v, 500, "hint");
        assert!(out.get("items").is_some());
        assert_eq!(out["budget"]["applied"], true);
    }
}
