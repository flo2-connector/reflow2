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
    /// Where the prose was cut, when it was cut in one list only
    /// ([`bound_reply_list_first`]); `None` means anywhere in the reply.
    prose_in: Option<&'a str>,
}

impl Block<'_> {
    fn value(&self, reply_chars: usize) -> Value {
        let place = self
            .prose_in
            .map(|p| format!(" in `{p}`"))
            .unwrap_or_default();
        // No text cut (rows alone made it fit) says so, rather than "0 text
        // field(s) cut to 0 characters".
        let prose = if self.fields_trimmed == 0 && !self.lists_shortened.is_empty() {
            "No text was cut.".to_string()
        } else {
            format!(
                "What was withheld is PROSE{place}: {} text field(s) cut to {} characters.",
                self.fields_trimmed, self.prose_limit
            )
        };
        let note = format!(
            "WITHHELD TO FIT. The whole answer is {} characters against a budget of {}, which \
             is the size at which a client refuses the reply outright — and a reader who \
             receives nothing cannot be told to narrow their question. {prose} Every number, \
             count and id is untouched and at full precision, so a shorter answer here is never \
             a quieter one.{} Raise `budget_chars` if this client has the room. {}",
            self.full_chars, self.budget, self.list_note, self.hint
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
    bound_reply_with(full, budget, hint, Sampling::Forbidden, None)
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
    bound_reply_with(full, budget, hint, Sampling::Allowed, None)
}

/// [`bound_reply_sampling`], shortening ONLY the lists named in `only` — a
/// path ending in `.` names every list directly under that object, any other
/// is one list's exact path. Every other list stays whole, at any budget: for
/// a reply that is a sample of findings beside lists that say WHAT was asked
/// (which designs were composed, which were not), where a sample of the
/// second kind would be a quieter answer wearing a complete one's shape.
#[must_use]
pub fn bound_reply_sampling_only(full: Value, budget: usize, hint: &str, only: &[&str]) -> Value {
    bound_reply_with(full, budget, hint, Sampling::Allowed, Some(only))
}

#[must_use]
fn bound_reply_with(
    full: Value,
    budget: usize,
    hint: &str,
    sampling: Sampling,
    only: Option<&[&str]>,
) -> Value {
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
        prose_in: None,
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
    if let Some(only) = only {
        lists.retain(|l| {
            only.iter().any(|o| match o.strip_suffix('.') {
                Some(parent) => l
                    .path
                    .strip_prefix(parent)
                    .and_then(|rest| rest.strip_prefix('.'))
                    .is_some_and(|key| !key.contains('.')),
                None => l.path == *o,
            })
        });
    }
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

/// Bound a reply whose weight is ONE named top-level list of items, giving
/// in this order until it fits: that list's prose, then ROWS from every list
/// in the reply but those in `keep_whole` (that list included), and only then
/// prose anywhere else.
///
/// For a reply that is mostly a fixed report with one list that grows with
/// the design — `loop_status`'s owner-scoped `awaiting_word`, measured
/// 2026-10-14 at 27,951 of the 38,939 characters it brought the scoped answer
/// to on reflow2's own design, against flo2.io's 30,000. Cutting prose across
/// the whole reply first, as [`bound_reply`] does, would cut the to-do lines in
/// `next` before the names in the list that caused the overflow.
///
/// ROWS FROM EVERY LIST, NOT ONLY THE NAMED ONE
/// (fact:loop-status-answered-10605-chars-to-a-3000-char-budget-2026-10-26):
/// until 2026-10-26 only `list` gave rows, and with it empty (every unscoped
/// `loop_status`) the reply fell straight to [`bound_reply`], which cut 104
/// texts to nothing, `next`'s to-do lines with them, and kept all 20 breach
/// rows: 11,767 characters to a budget of 3,000. Each list the reply holds is
/// counted beside it, so a row withheld from its tail is counted, never lost.
/// The lists give by what they cost, the costliest first, from the tail, to a
/// floor of one row each.
///
/// The same rules as everywhere here: numbers and ids are never touched (a
/// string under [`NEVER_TRIM_BELOW`] survives every rung, and a row is dropped
/// whole, never thinned), no list is emptied (a floor of one row), and the
/// `budget` block says which prose was cut, which rows were withheld
/// (`lists_shortened: [{path, shown, total}]`) and how to see the rest. The
/// reply's own counts of each list are the caller's to keep, beside the list.
#[must_use]
pub fn bound_reply_list_first(
    full: Value,
    budget: usize,
    list: &str,
    keep_whole: &[&str],
    hint: &str,
) -> Value {
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
        prose_in: Some(list),
    };
    let rows = full
        .get(list)
        .and_then(Value::as_array)
        .cloned()
        .filter(|r| !r.is_empty());
    let with_rows = |rows: Vec<Value>| -> Value {
        let mut v = full.clone();
        if let Some(o) = v.as_object_mut() {
            o.insert(list.to_string(), Value::Array(rows));
        }
        v
    };

    // ① The list's prose, progressively — its long names and evidence text.
    let mut body = full.clone();
    if let Some(rows) = &rows {
        for rung in PROSE_RUNGS {
            let mut cut = 0usize;
            let trimmed: Vec<Value> = rows
                .iter()
                .map(|r| truncate_prose(r, rung, &mut cut))
                .collect();
            block.prose_limit = rung;
            block.fields_trimmed = cut;
            body = with_rows(trimmed);
            let out = block.assemble(&body);
            if chars_of(&out) <= budget {
                return out;
            }
        }
    } else {
        // Nothing in the list: no prose was cut there.
        block.prose_in = None;
        block.fields_trimmed = 0;
    }

    // ② Rows, from every list but those kept whole, the costliest first, each
    //    withheld whole from its tail, to a floor of one.
    let mut lists = Vec::new();
    collect_lists(&body, "", &mut lists);
    lists.retain(|l| !keep_whole.contains(&l.path.as_str()));
    let mut capped = body.clone();
    let note_rows = |block: &mut Block, lists: &[ListSlot]| {
        block.lists_shortened = lists
            .iter()
            .filter(|l| l.keep < l.total())
            .map(|l| json!({"path": l.path, "shown": l.keep, "total": l.total()}))
            .collect();
        // The block names each list once, in `lists_shortened`; the note
        // says how many, so the explanation does not cost what it explains.
        let n = block.lists_shortened.len();
        block.list_note = if n == 0 {
            String::new()
        } else {
            format!(
                " ROWS WITHHELD from {n} list(s), the costliest first, each whole from its tail: \
                 `lists_shortened` says how many each shows of its total."
            )
        };
    };
    for _ in 0..256 {
        note_rows(&mut block, &lists);
        let out = block.assemble(&capped);
        let now = chars_of(&out);
        if now <= budget {
            return out;
        }
        if shorten_by_chars(&mut lists, now - budget) == 0 {
            break;
        }
        capped = apply_caps(&body, &lists);
    }
    note_rows(&mut block, &lists);
    let body = capped;

    // ③ Every list is at its floor: prose anywhere else in the reply.
    block.prose_in = None;
    for rung in PROSE_RUNGS {
        let mut cut = 0usize;
        let everywhere = truncate_prose(&body, rung, &mut cut);
        block.prose_limit = rung;
        block.fields_trimmed = cut;
        let out = block.assemble(&everywhere);
        if chars_of(&out) <= budget {
            return out;
        }
        if rung == *PROSE_RUNGS.last().unwrap() {
            block.list_note.push_str(&format!(
                " AND THE REPLY IS STILL {} CHARACTERS with every list at one row and every \
                 long text cut: what remains is the report's own counts and structure, which \
                 this budget cannot hold — raise `budget_chars`.",
                chars_of(&out)
            ));
            return block.assemble(&everywhere);
        }
    }
    block.assemble(&body)
}

/// How many withheld rows a list names by id, at most: past it the count
/// (`withheld_not_named`) stands for the rest, so the names cannot outgrow the
/// budget they were withheld to fit.
const WITHHELD_NAMED_MAX: usize = 50;

/// Bound a reply by withholding ROWS first, each whole, from the tail of the
/// lists named in `only` (exact paths), by the characters each costs, to a
/// floor of one row per list — and cut prose only if that was not enough.
///
/// For a reply whose rows are each an answer read whole (a step a change
/// breaks, with its rule and premises), where cutting every row's prose
/// would leave every row unreadable to keep them all. Each shortened list is
/// named in `budget.lists_shortened` with `shown`, `total` and the ids of the
/// rows withheld (`withheld`: each row's `id`, `node_id` or `inference`, the
/// first [`WITHHELD_NAMED_MAX`] of them, `withheld_not_named` counting the
/// rest), so each can be read on its own. A list in `paged` is read on by an
/// offset instead, so its withheld rows are counted and not named
/// (`read_on: "offset"`). Every count elsewhere in the reply is untouched. A
/// reply that fits is returned unchanged, byte for byte.
#[must_use]
pub fn bound_reply_rows_first(
    full: Value,
    budget: usize,
    hint: &str,
    only: &[&str],
    paged: &[&str],
) -> Value {
    let full_chars = chars_of(&full);
    if full_chars <= budget {
        return full;
    }
    let mut lists = Vec::new();
    collect_lists(&full, "", &mut lists);
    lists.retain(|l| only.contains(&l.path.as_str()));
    let row_id = |r: &Value| -> Option<Value> {
        // A list of ids (a radius's `seeds`) is its own names.
        if r.is_string() {
            return Some(r.clone());
        }
        ["id", "node_id", "inference"]
            .iter()
            .find_map(|k| r.get(*k).filter(|v| v.is_string()).cloned())
    };
    let assemble = |lists: &[ListSlot], capped: &Value| -> Value {
        let shortened: Vec<Value> = lists
            .iter()
            .filter(|l| l.keep < l.total())
            .map(|l| {
                if paged.contains(&l.path.as_str()) {
                    // Read on by offset: naming every row a page leaves for
                    // the next would cost what the page saved.
                    return json!({"path": l.path, "shown": l.keep, "total": l.total(),
                                  "read_on": "offset"});
                }
                let rows = path_array(&full, &l.path);
                let ids: Vec<Value> = rows[l.keep..].iter().filter_map(row_id).collect();
                let more = ids.len().saturating_sub(WITHHELD_NAMED_MAX);
                let withheld: Vec<Value> = ids.into_iter().take(WITHHELD_NAMED_MAX).collect();
                let mut row = json!({"path": l.path, "shown": l.keep, "total": l.total(),
                                     "withheld": withheld});
                if more > 0 {
                    row["withheld_not_named"] = json!(more);
                }
                row
            })
            .collect();
        let mut out = capped.clone();
        let mut reply_chars = chars_of(&out);
        for _ in 0..4 {
            let named = shortened
                .iter()
                .map(|l| {
                    format!(
                        "`{}` shows {} of {}",
                        l["path"].as_str().unwrap_or(""),
                        l["shown"],
                        l["total"]
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            let block = json!({
                "applied": true,
                "budget_chars": budget,
                "full_chars": full_chars,
                "reply_chars": reply_chars,
                "lists_shortened": shortened,
                "note": format!(
                    "WITHHELD TO FIT. The whole answer is {full_chars} characters against a \
                     budget of {budget}. What was withheld is ROWS, each whole, from the end of \
                     its list: {named}. Each row withheld is named by id in \
                     `lists_shortened[].withheld`; no row shown was cut, and every count is \
                     untouched. Raise `budget_chars` if this client has the room. {hint}"
                ),
            });
            out = capped.clone();
            if let Some(o) = out.as_object_mut() {
                o.insert("budget".into(), block);
            }
            let n = chars_of(&out);
            if n == reply_chars {
                break;
            }
            reply_chars = n;
        }
        out
    };
    let mut capped = full.clone();
    for _ in 0..64 {
        let out = assemble(&lists, &capped);
        let now = chars_of(&out);
        if now <= budget {
            return out;
        }
        if shorten_by_chars(&mut lists, now - budget) == 0 {
            // Every named list is at one row: what remains is prose, which
            // the usual rungs cut, saying so in their own block.
            let rows_note = assemble(&lists, &capped);
            let mut out = bound_reply(capped, budget, hint);
            if let (Some(o), Some(b)) = (out.as_object_mut(), rows_note.get("budget")) {
                match o.get_mut("budget").and_then(Value::as_object_mut) {
                    Some(prose) => {
                        prose.insert("lists_shortened".into(), b["lists_shortened"].clone());
                    }
                    None => {
                        o.insert("budget".into(), b.clone());
                    }
                }
            }
            return out;
        }
        capped = apply_caps(&full, &lists);
    }
    assemble(&lists, &capped)
}

/// The array at a dotted object path, or an empty slice.
fn path_array<'a>(v: &'a Value, path: &str) -> &'a [Value] {
    let mut at = v;
    for part in path.split('.').filter(|p| !p.is_empty()) {
        match at.get(part) {
            Some(x) => at = x,
            None => return &[],
        }
    }
    at.as_array().map(Vec::as_slice).unwrap_or(&[])
}

/// Hold a reply to `budget` by withholding rows of ONE RANKED LIST from its
/// tail, each WHOLE, down to none, and touch nothing else: for a reply whose
/// every other part must arrive whole (a skill's body) beside a list that
/// grows with the design (the lessons it holds for that skill).
///
/// The list lives at `list` inside the object at `at` (`at` empty: the
/// reply's top level). When rows are withheld, that object says so:
/// `shown` and `total` (counts, never budgeted away), `withheld_ids` (each
/// withheld row's `id`, so it can be read on its own), and `withheld_note`,
/// which says how many and how to get them (`how`). A reply that already
/// fits is returned unchanged, byte for byte.
///
/// Measured 2026-10-21 (fact:root-cause-get-skill-attaches-every-lesson-
/// unbounded-so-six-skills-outgrow-a-chat-reply-on-reflow2s-design-2026-10-21):
/// get_skill root-cause answered 54,054 characters on reflow2's own design,
/// a 19,230-character body and 26 lessons, because nothing bounded the list.
#[must_use]
pub fn keep_ranked_rows_whole(
    full: Value,
    budget: usize,
    at: &str,
    list: &str,
    how: &str,
) -> Value {
    if chars_of(&full) <= budget {
        return full;
    }
    let holder = |v: &Value| -> Option<Value> {
        if at.is_empty() {
            Some(v.clone())
        } else {
            v.get(at).cloned()
        }
    };
    let Some(rows) = holder(&full)
        .and_then(|h| h.get(list).and_then(Value::as_array).cloned())
        .filter(|r| !r.is_empty())
    else {
        return full;
    };
    let total = rows.len();
    let with = |keep: usize| -> Value {
        let mut v = full.clone();
        let target = if at.is_empty() {
            Some(&mut v)
        } else {
            v.get_mut(at)
        };
        if let Some(o) = target.and_then(Value::as_object_mut) {
            o.insert(list.to_string(), Value::Array(rows[..keep].to_vec()));
            if keep < total {
                let withheld: Vec<Value> = rows[keep..]
                    .iter()
                    .filter_map(|r| r.get("id").cloned())
                    .collect();
                o.insert("shown".into(), json!(keep));
                o.insert("total".into(), json!(total));
                o.insert("withheld_ids".into(), Value::Array(withheld));
                o.insert(
                    "withheld_note".into(),
                    Value::String(format!(
                        "WITHHELD TO FIT budget_chars {budget}: {} of the {total} in `{list}`, \
                         whole, from the end of the ranking, so {keep} are shown. Nothing else \
                         in this reply was cut. {how}",
                        total - keep
                    )),
                );
            }
        }
        v
    };
    let mut keep = total;
    let mut out = with(keep);
    while keep > 0 && chars_of(&out) > budget {
        keep -= 1;
        out = with(keep);
    }
    out
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
    fn the_named_list_gives_its_prose_then_its_rows_before_anything_else_does() {
        // The loop_status shape: a to-do list whose lines must survive, and a
        // list that grows with the design and is where the bytes are.
        let rows: Vec<Value> = (0..60)
            .map(|i| json!({"node_id": format!("req:r{i}"), "name": long(600), "closed_by": "set_requirement_status"}))
            .collect();
        let next = vec![json!(long(300))];
        let v = json!({"count": 60, "next": next.clone(), "awaiting_word": rows});
        let out = bound_reply_list_first(
            v.clone(),
            12_000,
            "awaiting_word",
            &["next"],
            "get_node on its id",
        );
        assert!(chars_of(&out) <= 12_000, "{} chars", chars_of(&out));
        assert_eq!(
            out["next"],
            json!(next),
            "prose outside the list is untouched"
        );
        let got = out["awaiting_word"].as_array().unwrap();
        assert_eq!(
            got.len(),
            60,
            "its prose alone was enough, so every row stays"
        );
        assert_eq!(got[59]["node_id"], "req:r59");
        assert_eq!(got[59]["closed_by"], "set_requirement_status");
        assert!(
            out["budget"]["note"]
                .as_str()
                .unwrap()
                .contains("in `awaiting_word`")
        );

        // Smaller: rows go, from the tail, named with shown and total.
        let out =
            bound_reply_list_first(v, 3_000, "awaiting_word", &["next"], "get_node on its id");
        assert!(chars_of(&out) <= 3_000, "{} chars", chars_of(&out));
        assert_eq!(out["count"], 60, "a count is never budgeted away");
        let shown = out["awaiting_word"].as_array().unwrap().len();
        assert!((1..60).contains(&shown));
        assert_eq!(
            out["awaiting_word"][0]["node_id"], "req:r0",
            "the head is kept"
        );
        assert_eq!(
            out["budget"]["lists_shortened"],
            json!([{"path": "awaiting_word", "shown": shown, "total": 60}])
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
