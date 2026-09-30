//! Nothing reflow2 serves tells an agent to write a reader's persona into the
//! design.
//!
//! `req:a-design-holds-who-contributed-never-a-readers-persona` (accepted
//! 2026-09-28, @ajs): a design holds who contributed to it — attribution,
//! authority and, where it matters, a role on this design — and NOT a
//! reader's background, the vocabulary they bring, or how they like to think.
//! The persona comes from the agent's host (flo2 hands over the signed-in
//! person's persona with the design) or, with no host, from the agent's own
//! memory of its user. With neither, the agent follows the person's own words
//! and may ask once, without writing the answer into the design. *"reflow2's
//! served text says this, rather than telling the agent to record a background
//! in the design."*
//!
//! # Why this exists, measured
//!
//! `fact:the-served-lens-still-tells-the-agent-to-record-a-readers-background-in-the-design-2026-09-30`.
//! The lens on `list_skills`, `get_skill` and `loop_status` still said
//! *"Ask what they do day to day and what they trained in … then record it
//! with `add_contributor`"* two days after that requirement was accepted, and
//! `where-am-i`, `topic`, `why` and the served instructions said the same in
//! their own words. The sentence entered in #353, a month BEFORE the
//! requirement; the requirement was accepted with no capability and no check,
//! so nothing read the served text against it. This is that check.
//!
//! # What it reads — every surface reflow2 serves text on, through the handlers
//!
//! - the `lens` on `list_skills`, `get_skill` and `loop_status`, in every
//!   branch the lens has: an empty design, people with nothing on record
//!   beside an automated agent, one person with a description, two;
//! - `get_instructions`: the whole document AND every section in its manifest,
//!   the pointer included;
//! - every served skill: body, description and summary, fetched by name;
//! - every tool on the served listing (`tools/list`, with its list-time
//!   decorations): the description and every string in its input schema —
//!   so `add_contributor`'s own parameters are read;
//! - the handshake's `instructions`, and the whole `describe_schema` reply.
//!
//! # The class it pins — a small, stated set of shapes, not one sentence
//!
//! A sentence is flagged when it
//!
//! 1. **PAIRS a reader's persona with the design holding it**: a PERSONA term
//!    (background, persona, trained in, day to day, who they are, their
//!    vocabulary, how they like to think) and a HOLDS term (`add_contributor`,
//!    Contributor, record/records/recorded, update the record, in/into the
//!    design or graph, `get_node`) in one sentence, in either order, with no
//!    negation between them or earlier in the same clause as either. This
//!    covers the WRITE ("then record it with `add_contributor`") and the READ
//!    that presumes one was written ("Recorded backgrounds: …", "read the
//!    reader's recorded background"); or
//! 2. **WRITES "it" or "the answer" onto a Contributor**: record / write /
//!    save / store + it / the answer / their answer, followed in the sentence
//!    by `add_contributor` or Contributor, not negated — the anaphoric form
//!    ("records the answer on their `Contributor`") that names no persona
//!    word because the sentence before it did.
//!
//! ⚠️ A KEYWORD CHECK, AND ITS LIMITS ARE STATED RATHER THAN HIDDEN. It
//! cannot tell an instruction from a sentence that merely describes one, and
//! its negation rule is clause-local, so "not on a Contributor" passes and
//! "has no Contributor for them, make one with `add_contributor` (their
//! background)" does not. `the_check_sees_every_shape_it_names` pins both
//! halves on real sentences — every one the surface served on 2026-09-30 must
//! still be caught, and the corrected wording must pass — so the classifier
//! cannot rot into silence or into noise without a red test saying which.
//!
//! And the positive half: the lens a design with nobody described serves says
//! what the requirement says — the host hands the persona over, else the
//! agent's own memory, else the person's own words, ask at most once, never
//! written into the design.

use reflow2_mcp::service::*;
use reflow2_mcp::tools::skills_tools::{GetInstructionsReq, GetSkillReq, ListSkillsReq};
use rmcp::ServerHandler;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

macro_rules! j {
    ($call:expr) => {
        $call
            .await
            .expect("tool ok")
            .structured_content
            .expect("structured content present")
    };
}

// ─── the classifier ───────────────────────────────────────────────────────────

/// Terms that name a READER'S persona. Matched as whole-word token sequences
/// after lower-casing, possessive `'s` stripped and hyphens read as spaces
/// ("day-to-day" is "day to day").
const PERSONA: &[&str] = &[
    "background",
    "backgrounds",
    "persona",
    "personas",
    "trained in",
    "day to day",
    "who they are",
    "their vocabulary",
    "the vocabulary they",
    "how they like to think",
];

/// Terms that say the DESIGN holds it — the write, or the read that presumes a
/// write.
const HOLDS: &[&str] = &[
    "add_contributor",
    "contributor",
    "contributors",
    "record",
    "records",
    "recorded",
    "update the record",
    "into the design",
    "in the design",
    "into the graph",
    "in the graph",
    "get_node",
];

/// Shape 2's verbs and objects: "record it", "records the answer", …
const WRITE_VERBS: &[&str] = &[
    "record", "records", "write", "writes", "save", "saves", "store", "stores",
];
const ANAPHORA: &[&str] = &["it", "the answer", "their answer"];
const CONTRIBUTOR_TARGETS: &[&str] = &["add_contributor", "contributor"];

/// How far apart, in words, the two halves of a shape may sit. Every served
/// instance measured on 2026-09-30 was within 16; a wider window began pairing
/// a heading's "RECORD" with a later clause about something else.
const MAX_GAP: usize = 20;

/// A negation governs a term when it sits earlier in the same clause, or
/// between the two terms of a pair.
const NEGATIONS: &[&str] = &[
    "never",
    "not",
    "no",
    "don't",
    "dont",
    "doesn't",
    "nor",
    "without",
    "cannot",
    "can't",
    "rather than",
    "instead of",
];

/// One token of a sentence: its text and the clause it sits in.
#[derive(Debug, Clone)]
struct Tok {
    word: String,
    clause: usize,
}

/// Split served text into sentences. A sentence ends at `.`, `!` or `?`
/// followed (after any closing `**`, quote or bracket) by whitespace — so ids
/// like `who:ann` and `v0.75.0` survive — and at a blank line or a list item,
/// which is where markdown ends a thought.
fn sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.split("\n\n") {
        for line_block in split_list_items(para) {
            let flat = line_block.split_whitespace().collect::<Vec<_>>().join(" ");
            let chars: Vec<char> = flat.chars().collect();
            let mut cur = String::new();
            let mut i = 0;
            while i < chars.len() {
                let c = chars[i];
                cur.push(c);
                if matches!(c, '.' | '!' | '?') {
                    // Closing emphasis, quotes and brackets belong to the
                    // sentence they close: `**… WORD.**` ends there.
                    let mut k = i + 1;
                    while k < chars.len()
                        && matches!(
                            chars[k],
                            '*' | '_' | '"' | '\'' | ')' | ']' | '`' | '\u{201d}' | '\u{2019}'
                        )
                    {
                        k += 1;
                    }
                    if chars.get(k).is_none_or(|n| n.is_whitespace()) {
                        cur.extend(&chars[i + 1..k]);
                        if !cur.trim().is_empty() {
                            out.push(cur.trim().to_string());
                        }
                        cur.clear();
                        i = k;
                        continue;
                    }
                }
                i += 1;
            }
            if !cur.trim().is_empty() {
                out.push(cur.trim().to_string());
            }
        }
    }
    out
}

/// Markdown list items and table rows are separate thoughts even without a
/// full stop.
fn split_list_items(para: &str) -> Vec<String> {
    let mut blocks: Vec<String> = Vec::new();
    for line in para.lines() {
        let t = line.trim_start();
        let starts_item = t.starts_with("- ")
            || t.starts_with("* ")
            || t.starts_with('|')
            || t.starts_with('#')
            || t.split_once(". ")
                .is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        if starts_item || blocks.is_empty() {
            blocks.push(line.to_string());
        } else {
            let last = blocks.last_mut().expect("non-empty");
            last.push('\n');
            last.push_str(line);
        }
    }
    blocks
}

/// Lower-case words, with clause numbers. Clauses break at `, ; : — – ( ) [ ]`
/// (a colon only when followed by a space, so `who:ann` stays one word).
fn tokens(sentence: &str) -> Vec<Tok> {
    let lower = sentence
        .to_lowercase()
        .replace(['\u{2019}', '\u{2018}'], "'");
    let chars: Vec<char> = lower.chars().collect();
    let mut toks = Vec::new();
    let mut clause = 0usize;
    let mut word = String::new();
    let flush = |word: &mut String, toks: &mut Vec<Tok>, clause: usize| {
        if !word.is_empty() {
            let mut w = word.trim_matches('\'').to_string();
            if let Some(stem) = w.strip_suffix("'s") {
                w = stem.to_string();
            }
            if !w.is_empty() {
                toks.push(Tok { word: w, clause });
            }
            word.clear();
        }
    };
    for (i, &c) in chars.iter().enumerate() {
        let clause_break = matches!(c, ',' | ';' | '—' | '–' | '(' | ')' | '[' | ']')
            || (c == ':' && chars.get(i + 1).is_none_or(|n| n.is_whitespace()));
        if clause_break {
            flush(&mut word, &mut toks, clause);
            clause += 1;
        } else if c.is_alphanumeric() || c == '_' || c == '\'' || c == ':' {
            word.push(c);
        } else {
            // Hyphens, backticks, asterisks, quotes, slashes: a word boundary.
            flush(&mut word, &mut toks, clause);
        }
    }
    flush(&mut word, &mut toks, clause);
    toks
}

/// Every (start, end) span where `phrase` occurs as whole words.
fn find(toks: &[Tok], phrase: &str) -> Vec<(usize, usize)> {
    let want: Vec<&str> = phrase.split(' ').collect();
    (0..toks.len())
        .filter(|&i| {
            i + want.len() <= toks.len()
                && want.iter().enumerate().all(|(k, w)| toks[i + k].word == *w)
        })
        .map(|i| (i, i + want.len()))
        .collect()
}

fn find_any(toks: &[Tok], phrases: &[&str]) -> Vec<(usize, usize)> {
    phrases.iter().flat_map(|p| find(toks, p)).collect()
}

fn negated_in(toks: &[Tok], from: usize, to: usize) -> bool {
    let negs = find_any(toks, NEGATIONS);
    negs.iter().any(|&(s, _)| s >= from && s < to)
}

/// Is the term starting at `at` governed by a negation earlier in its clause?
fn clause_negated(toks: &[Tok], at: usize) -> bool {
    let clause = toks[at].clause;
    let start = (0..at)
        .rev()
        .take_while(|&i| toks[i].clause == clause)
        .last()
        .unwrap_or(at);
    negated_in(toks, start, at)
}

/// The shapes a sentence matches, by name; empty when it is clean.
fn persona_shapes(sentence: &str) -> Vec<&'static str> {
    let toks = tokens(sentence);
    let mut hits = Vec::new();

    // Shape 1: a persona term and a holds term, not negated.
    let personas = find_any(&toks, PERSONA);
    let holds = find_any(&toks, HOLDS);
    'pairs: for &(ps, pe) in &personas {
        for &(hs, he) in &holds {
            let (first_end, second_start) = if ps < hs { (pe, hs) } else { (he, ps) };
            if second_start < first_end || second_start - first_end > MAX_GAP {
                continue; // overlapping phrases, or too far apart to be one thought
            }
            if negated_in(&toks, first_end, second_start)
                || clause_negated(&toks, ps)
                || clause_negated(&toks, hs)
            {
                continue;
            }
            hits.push("a reader's persona held in the design");
            break 'pairs;
        }
    }

    // Shape 2: "record it / the answer" … onto a Contributor, not negated.
    'writes: for &(vs, ve) in &find_any(&toks, WRITE_VERBS) {
        let takes_anaphor = ANAPHORA
            .iter()
            .any(|a| find(&toks, a).iter().any(|&(s, _)| s == ve));
        if !takes_anaphor || clause_negated(&toks, vs) {
            continue;
        }
        for &(ts, _) in &find_any(&toks, CONTRIBUTOR_TARGETS) {
            if ts > ve
                && ts - ve <= MAX_GAP
                && !negated_in(&toks, ve, ts)
                && !clause_negated(&toks, ts)
            {
                hits.push("the answer written onto a Contributor");
                break 'writes;
            }
        }
    }
    hits
}

/// Every flagged sentence in `text`, labelled with where it was served.
fn scan(label: &str, text: &str, out: &mut Vec<String>) {
    for s in sentences(text) {
        for shape in persona_shapes(&s) {
            out.push(format!("{label} [{shape}]: {s}"));
        }
    }
}

/// Every string inside a JSON value — a tool's schema, a reply.
fn strings_in(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| strings_in(x, out)),
        Value::Object(o) => o.values().for_each(|x| strings_in(x, out)),
        _ => {}
    }
}

// ─── the classifier's own net ────────────────────────────────────────────────

/// ⭐ THE CHECK MUST SEE WHAT IT CLAIMS TO SEE. Every sentence below was served
/// on 2026-09-30 (origin/main 01fe82a) and is the defect this file exists for;
/// each must be caught. The corrected wordings must pass. A classifier loosened
/// until the surface goes green fails the first list; one tightened into noise
/// fails the second.
#[test]
fn the_check_sees_every_shape_it_names() {
    let served_on_2026_09_30 = [
        // the lens, empty case (skills.rs:382)
        "NOBODY'S BACKGROUND IS RECORDED — No person is recorded in this design yet, so nothing \
         here tells you whose words to use.",
        "Ask what they do day to day and what they trained in (those often differ and both \
         matter), then record it with `add_contributor`.",
        // the lens, described case
        "Recorded backgrounds: who:ann (Ann).",
        // where-am-i
        "`scan_nodes` for `Contributor` — who is in this design, and whether the person you are \
         talking to has a recorded `description` of who they are.",
        "Record it **in their own words** with `add_contributor` (their `description`), not \
         your paraphrase of them.",
        "People show you their vocabulary by using it, so when their own words tell you more \
         than their answer did, update the record.",
        // served AGENTS.md
        "The **where-am-i** skill asks at the start of a session and records the answer on \
         their `Contributor`; read it before you narrate anything.",
        "If nobody has recorded one, ask — what they do day to day and what they trained in, \
         which are often different and both matter.",
        // detect-and-ask
        "Read the reader's recorded `description` on their `Contributor` and match it; absent \
         one, follow the vocabulary they use with you.",
        // topic
        "Read the reader's recorded background (the lens on this skill) and say what the \
         design holds in THEIR words.",
        // why
        "If the design has no Contributor for them, make one with `add_contributor` (their \
         own description of their background).",
    ];
    for s in served_on_2026_09_30 {
        assert!(
            !persona_shapes(s).is_empty(),
            "the check must catch this sentence, served on 2026-09-30: {s}"
        );
    }

    let corrected = [
        "It keeps who wrote and approved the design, never a reader's persona.",
        "With neither, follow the words they use with you; you may ask once what they do day to \
         day and what they trained in, and keep the answer yourself: never write it into the \
         design, not on a Contributor and not in any other node.",
        "Do not record their background in the design.",
        "Never write a reader's background into the design.",
        "Keep the answer in your own memory, not in the design.",
        "If the design has no Contributor for them, make one with `add_contributor`, carrying \
         their name and, where it matters, their role on this design — never their background \
         or how they like to be spoken to, which stays in your own memory.",
        "Record a Contributor — who authors and decides the DESIGN itself: a person, an \
         automated coding agent, or an organization.",
        "A background noted once and never revisited goes stale the same way any other fact \
         does.",
        // capture-intent's alias rule: a domain noun on a node is design
        // content, not a reader's persona, and must not be caught.
        "`record_alias` puts the user's own noun on the node — *\"we call that a query\"* — so \
         the next session, and the next reader, meets their vocabulary instead of re-deriving it.",
    ];
    for s in corrected {
        assert!(
            persona_shapes(s).is_empty(),
            "the check must pass this sentence, which keeps the persona out of the design: {s} \
             — flagged as {:?}",
            persona_shapes(s)
        );
    }
}

// ─── the served surface ──────────────────────────────────────────────────────

async fn service() -> ReflowService {
    let s = ReflowService::in_memory().expect("in-memory service");
    j!(s.add_project(Parameters(
        serde_json::from_value(json!({"id": "proj:x", "name": "X"})).unwrap()
    )));
    s
}

async fn contributor(s: &ReflowService, id: &str, kind: &str, description: Option<&str>) {
    let mut args = json!({"id": id, "name": format!("Name of {id}"), "kind": kind});
    if let Some(d) = description {
        args["description"] = json!(d);
    }
    j!(s.add_contributor(Parameters(serde_json::from_value(args).unwrap())));
}

/// The lens as served on each of its three rails, for one design.
async fn lenses(s: &ReflowService) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    let listed = j!(s.list_skills(Parameters(ListSkillsReq { budget_chars: None })));
    let got = j!(s.get_skill(Parameters(GetSkillReq {
        name: "where-am-i".into(),
    })));
    let looped = j!(s.loop_status(Parameters(
        serde_json::from_value::<LoopScopeReq>(json!({})).unwrap()
    )));
    for (rail, reply) in [
        ("list_skills", listed),
        ("get_skill", got),
        ("loop_status", looped),
    ] {
        let lens = reply
            .get("lens")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("{rail} carries no lens: {reply}"))
            .to_string();
        out.push((rail, lens));
    }
    out
}

/// Every lens branch: an empty design; people with nothing on record beside a
/// described automated agent; one described person; two.
async fn every_lens() -> Vec<(String, String)> {
    let mut out = Vec::new();

    let empty = service().await;
    for (rail, l) in lenses(&empty).await {
        out.push((format!("lens on {rail}, empty design"), l));
    }

    let undescribed = service().await;
    contributor(&undescribed, "who:ann", "person", None).await;
    contributor(&undescribed, "who:bea", "person", None).await;
    contributor(
        &undescribed,
        "who:bot",
        "automated_agent",
        Some("An agent working the design."),
    )
    .await;
    for (rail, l) in lenses(&undescribed).await {
        out.push((format!("lens on {rail}, people with nothing on record"), l));
    }

    let one = service().await;
    contributor(&one, "who:ann", "person", Some("Owner of this design.")).await;
    for (rail, l) in lenses(&one).await {
        out.push((format!("lens on {rail}, one described person"), l));
    }

    let two = service().await;
    contributor(&two, "who:ann", "person", Some("Owner of this design.")).await;
    contributor(&two, "who:bea", "person", Some("Reviewer.")).await;
    for (rail, l) in lenses(&two).await {
        out.push((format!("lens on {rail}, two described people"), l));
    }
    out
}

/// ⭐ THE GATE. Every surface, every hit reported at once.
#[tokio::test]
async fn no_served_surface_tells_the_agent_to_write_a_readers_persona_into_the_design() {
    let mut hits = Vec::new();
    let mut read = 0usize;

    // 1. the lens, on all three rails, in every branch
    let every = every_lens().await;
    assert_eq!(every.len(), 12, "four designs × three rails");
    for (label, lens) in &every {
        scan(label, lens, &mut hits);
        read += 1;
    }

    let s = service().await;

    // 2. get_instructions — the whole document and every section it lists
    let whole = j!(s.get_instructions(Parameters(
        serde_json::from_value::<GetInstructionsReq>(json!({"budget_chars": 10_000_000})).unwrap()
    )));
    let body = whole["instructions"]
        .as_str()
        .expect("the whole document is served under a large budget");
    scan("get_instructions (whole)", body, &mut hits);
    read += 1;
    let slugs: Vec<String> = whole["sections"]
        .as_array()
        .expect("a sections manifest")
        .iter()
        .filter_map(|m| m["section"].as_str().map(str::to_string))
        .collect();
    assert!(
        slugs.len() >= 5,
        "a real manifest, not an empty one: {slugs:?}"
    );
    for slug in &slugs {
        let one = j!(s.get_instructions(Parameters(
            serde_json::from_value::<GetInstructionsReq>(json!({"section": slug})).unwrap()
        )));
        let text = one["instructions"]
            .as_str()
            .unwrap_or_else(|| panic!("section {slug} served no text: {one}"));
        scan(&format!("get_instructions section {slug}"), text, &mut hits);
        read += 1;
    }

    // 3. every served skill, fetched by the name the catalogue gives
    let listed = j!(s.list_skills(Parameters(ListSkillsReq {
        budget_chars: Some(10_000_000),
    })));
    let names: Vec<String> = listed["skills"]
        .as_array()
        .expect("skills")
        .iter()
        .filter_map(|k| k["name"].as_str().map(str::to_string))
        .collect();
    assert_eq!(
        names.len(),
        reflow2_mcp::skills::SKILLS.len(),
        "the catalogue lists every compiled-in skill"
    );
    assert!(names.len() >= 25, "a real catalogue: {}", names.len());
    for name in &names {
        let skill = j!(s.get_skill(Parameters(GetSkillReq { name: name.clone() })));
        for field in ["body", "description", "summary"] {
            let text = skill[field]
                .as_str()
                .unwrap_or_else(|| panic!("skill {name} has no {field}"));
            scan(&format!("get_skill {name} ({field})"), text, &mut hits);
        }
        read += 1;
    }

    // 4. every tool on the served listing: description and schema strings
    let tools = s.tools_with_lessons_for_test().await;
    assert!(tools.len() >= 150, "a real tool list: {}", tools.len());
    assert!(
        tools.iter().any(|t| t.name == "add_contributor"),
        "add_contributor is on the listing this reads"
    );
    for t in &tools {
        if let Some(d) = t.description.as_deref() {
            scan(&format!("tool {} (description)", t.name), d, &mut hits);
        }
        let schema = serde_json::to_value(&t.input_schema).expect("schema");
        let mut strings = Vec::new();
        strings_in(&schema, &mut strings);
        for text in strings {
            scan(&format!("tool {} (input schema)", t.name), &text, &mut hits);
        }
        read += 1;
    }

    // 5. the handshake, and the schema describe_schema reads out
    let info = s.get_info();
    scan(
        "handshake instructions",
        info.instructions
            .as_deref()
            .expect("the handshake carries instructions"),
        &mut hits,
    );
    let schema = j!(s.describe_schema(Parameters(
        serde_json::from_value::<DescribeSchemaReq>(json!({"budget_chars": 10_000_000})).unwrap()
    )));
    let mut strings = Vec::new();
    strings_in(&schema, &mut strings);
    for text in strings {
        scan("describe_schema", &text, &mut hits);
    }
    read += 2;

    assert!(
        read > 200,
        "read {read} surfaces — a broken read, not a small surface"
    );
    assert!(
        hits.is_empty(),
        "reflow2 serves {} sentence(s) telling an agent to hold a reader's persona in the \
         design, which req:a-design-holds-who-contributed-never-a-readers-persona forbids. The \
         reader's lens comes from the agent's host or its own memory; with neither, the agent \
         follows the person's words and may ask once, never writing the answer into the \
         design:\n  {}",
        hits.len(),
        hits.join("\n  ")
    );
}

/// ⭐ THE POSITIVE HALF: with nobody described, the lens says what the
/// requirement says, on every rail — not merely nothing forbidden.
#[tokio::test]
async fn the_lens_says_where_the_readers_lens_comes_from_and_that_it_stays_out_of_the_design() {
    let s = service().await;
    contributor(&s, "who:ann", "person", None).await;
    for (rail, lens) in lenses(&s).await {
        let l = lens.to_lowercase();
        for (what, phrase) in [
            ("a host may hand over the persona", "hands over"),
            ("the host is named as the first source", "your host"),
            ("with no host, the agent's own memory", "your own memory"),
            (
                "with neither, the person's own words",
                "follow the words they use",
            ),
            ("ask at most once", "ask once"),
            (
                "never written into the design",
                "never write it into the design",
            ),
        ] {
            assert!(
                l.contains(phrase),
                "the lens on {rail} must say {what} ({phrase:?}): {lens}"
            );
        }
    }
}

/// The described case says the same about the READER, and reads a
/// contributor's record for what it is — attribution and a role on this
/// design — never as the lens for whoever is reading.
#[tokio::test]
async fn a_described_contributor_is_read_for_their_role_never_as_the_readers_lens() {
    let s = service().await;
    contributor(&s, "who:ann", "person", Some("Owner of this design.")).await;
    for (rail, lens) in lenses(&s).await {
        let l = lens.to_lowercase();
        assert!(l.contains("who:ann"), "the contributor is named: {lens}");
        assert!(
            l.contains("role on this design"),
            "a contributor's record is read for their role on {rail}: {lens}"
        );
        assert!(
            l.contains("your host") && l.contains("your own memory"),
            "the reader's lens still comes from the host or memory on {rail}: {lens}"
        );
        assert!(
            l.contains("never write it into the design"),
            "and it stays out of the design on {rail}: {lens}"
        );
    }
}
