//! A served description that names a tool's argument names one the tool takes.
//!
//! # Why this exists, measured
//!
//! `fact:draw-edges-is-named-with-items-in-32-tool-descriptions-though-it-takes-edges-a-recurrence-2026-10-26`.
//! Every typed edge helper's served description ended "MANY AT ONCE:
//! `draw_edges`, items {\"tool\": …}" from #629 (2026-09-29) to 2026-10-26,
//! and `draw_edges` takes `edges`: an agent that followed the text was
//! refused. It is a RECURRENCE of
//! `fact:root-cause-instructions-route-records-to-paths-that-cannot-take-them-and-the-lint-checks-names-only-2026-09-28`,
//! which named the class — argument names written in served text are checked
//! against no schema; `skill_lint` checks tool NAMES only — and proposed this
//! check, which was not written then.
//!
//! # What it reads
//!
//! Every tool on the served listing (`tools/list`, list-time decorations
//! included, so the generated bulk sentence is read as a client reads it): the
//! description, and every `description` string anywhere in its input schema.
//!
//! # The shapes it reads, and why only these
//!
//! A census of the served surface on 2026-10-26 (199 tools) tried the looser
//! reading first — any backticked tool name followed closely by a backticked
//! word — and it named 24 pairs, 16 of them NOT arguments: reply fields
//! (`detect_defects`'s `swept.parked`), gap kinds (`detect_gaps` raises
//! `unallocated_capability`), statuses and values. That is noise, and a check
//! that is mostly noise gets silenced. These three shapes say "argument" in
//! their grammar, and on that day every one of them on the surface was right
//! (5 of 5), so a failure here is a finding, not a false alarm:
//!
//! 1. **WITH**: "`tool` with `a`", and a list after it ("with `a`, `b` and
//!    `c`") — `describe_schema` with `from` and `to`.
//! 2. **NAMED ARGUMENT**: "`tool`'s `a` argument" (or parameter) —
//!    `snapshot_before_change`'s `repair` argument.
//! 3. **EXAMPLE**: a JSON example after a tool's name in the same clause,
//!    "`tool` … {\"k\": …}" or "`tool` with `a`: [{\"k\": …}]". The word that
//!    introduces the example is an argument of the tool, and the example's keys
//!    are that argument's own (its items', for a list) — or, with no
//!    introducing word, the tool's. This is the shape the 2026-10-26 defect
//!    had ("`draw_edges`, items {\"tool\": …}").
//!
//! ⚠️ ITS LIMITS, STATED. A tool's argument named in prose outside these
//! shapes ("pass `true`", "the `role` on each edge") is not read: "pass `x`"
//! names a VALUE as often as an argument on this surface (3 of 25 on
//! 2026-10-26), so it is left out rather than allowlisted. Skills are not read
//! here. A name inside an argument (`a.b`) is checked at its root only.
//! `the_check_sees_every_shape_it_names` pins each shape on real sentences, in
//! both directions, so the reader cannot rot into silence without a red test.

use reflow2_mcp::service::ReflowService;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// A served tool's input schema, by tool name.
type Schemas = BTreeMap<String, Map<String, Value>>;

/// One argument a sentence names that its tool does not take.
#[derive(Debug, PartialEq)]
struct Wrong {
    tool: String,
    argument: String,
    shape: &'static str,
}

/// The backticked spans of `text`, with the byte range each covers (ticks
/// included).
fn spans(text: &str) -> Vec<(usize, usize, &str)> {
    let mut out = Vec::new();
    let mut rest = 0;
    while let Some(open) = text[rest..].find('`').map(|i| rest + i) {
        let Some(close) = text[open + 1..].find('`').map(|i| open + 1 + i) else {
            break;
        };
        out.push((open, close + 1, &text[open + 1..close]));
        rest = close + 1;
    }
    out
}

/// A plain argument-like name: lower-case identifier, optionally a path.
fn looks_like_a_name(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
}

/// The root of a name: `a` of `a.b`.
fn root(name: &str) -> &str {
    name.split('.').next().unwrap_or(name)
}

fn properties(schema: &Map<String, Value>) -> Option<&Map<String, Value>> {
    schema.get("properties").and_then(Value::as_object)
}

/// The object schema a property's VALUE has — its items' for a list — with
/// one `$ref` into the tool's `$defs` followed.
fn value_schema<'a>(
    tool: &'a Map<String, Value>,
    property: &'a Value,
) -> Option<&'a Map<String, Value>> {
    let inner = property.get("items").unwrap_or(property);
    match inner.get("$ref").and_then(Value::as_str) {
        Some(r) => {
            let name = r.rsplit('/').next()?;
            tool.get("$defs")?.get(name)?.as_object()
        }
        None => inner.as_object(),
    }
}

/// The keys of the first JSON object in `example` (which starts at a `[` or a
/// `{`), at that object's own depth.
fn example_keys(example: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut depth = 0usize;
    let mut object_depth = None;
    let mut chars = example.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            '[' | '{' => {
                depth += 1;
                if c == '{' && object_depth.is_none() {
                    object_depth = Some(depth);
                }
            }
            ']' | '}' => {
                if Some(depth) == object_depth {
                    return keys;
                }
                depth = depth.saturating_sub(1);
            }
            '"' if Some(depth) == object_depth => {
                let rest = &example[i + 1..];
                let Some(close) = rest.find('"') else {
                    return keys;
                };
                let after = rest[close + 1..].trim_start();
                if after.starts_with(':') {
                    keys.push(rest[..close].to_string());
                }
                // Skip the string's body.
                while chars.peek().is_some_and(|(j, _)| *j <= i + 1 + close) {
                    chars.next();
                }
            }
            _ => {}
        }
    }
    keys
}

/// Every argument `text` names in one of the three shapes, each with whether
/// its tool takes it (`true`) or not.
fn arguments_named(text: &str, served: &Schemas) -> Vec<(Wrong, bool)> {
    let mut named = Vec::new();
    let all = spans(text);
    let takes = |tool: &str, arg: &str| {
        served
            .get(tool)
            .and_then(properties)
            .is_some_and(|p| p.contains_key(root(arg)))
    };
    for (k, &(_, end, tool)) in all.iter().enumerate() {
        let Some(schema) = served.get(tool) else {
            continue;
        };
        let after = &text[end..];
        let check = |argument: &str, shape: &'static str| {
            let mention = Wrong {
                tool: tool.to_string(),
                argument: argument.to_string(),
                shape,
            };
            (mention, takes(tool, argument))
        };

        // 1. WITH: "`tool` with `a`[, `b`][ and `c`]".
        let mut introduced: Option<&str> = None;
        if after.starts_with(" with `") {
            let mut j = k + 1;
            let mut gap_ok = true;
            while gap_ok && j < all.len() {
                let (start, _, name) = all[j];
                let prev_end = all[j - 1].1;
                let gap = &text[prev_end..start];
                gap_ok = if j == k + 1 {
                    gap == " with "
                } else {
                    matches!(gap, ", " | " and " | ", and " | " or ")
                };
                if !gap_ok {
                    break;
                }
                if looks_like_a_name(name) && !served.contains_key(name) {
                    named.push(check(name, "with"));
                    introduced = Some(name);
                }
                j += 1;
            }
        }

        // 2. NAMED ARGUMENT: "`tool`'s `a` argument" / "parameter".
        if let Some(rest) = after.strip_prefix("'s `")
            && let Some(close) = rest.find('`')
        {
            let name = &rest[..close];
            let tail = rest[close + 1..].trim_start();
            if looks_like_a_name(name)
                && (tail.starts_with("argument") || tail.starts_with("parameter"))
            {
                named.push(check(name, "named argument"));
            }
        }

        // 3. EXAMPLE: a JSON example after the tool's name, in the same clause.
        let clause_end = after
            .find(['.', ';', '\n', '('])
            .unwrap_or(after.len())
            .min(60);
        let Some(open) = after
            .get(..clause_end.min(after.len()))
            .and_then(|c| c.find(['[', '{']))
        else {
            continue;
        };
        if !after[open..]
            .trim_start_matches(['[', '{', ' '])
            .starts_with('"')
        {
            continue;
        }
        let before = &after[..open];
        // Another tool named between the two: the example is that tool's.
        if spans(before)
            .iter()
            .any(|(_, _, n)| served.contains_key(*n))
        {
            continue;
        }
        let keys = example_keys(&after[open..]);
        let word = before
            .split_whitespace()
            .rfind(|w| *w != "with")
            .map(|w| w.trim_matches(|c: char| c == '`' || c == ':' || c == ','))
            .filter(|w| !w.is_empty());
        match word.or(introduced) {
            Some(word) => {
                if !takes(tool, word) {
                    if introduced != Some(word) {
                        named.push(check(word, "example"));
                    }
                    continue;
                }
                let own = properties(schema)
                    .and_then(|p| p.get(word))
                    .and_then(|p| value_schema(schema, p))
                    .and_then(properties);
                for key in keys {
                    let taken = own.is_some_and(|p| p.contains_key(&key));
                    let mention = Wrong {
                        tool: tool.to_string(),
                        argument: format!("{word}[].{key}"),
                        shape: "example",
                    };
                    named.push((mention, taken));
                }
            }
            None => {
                for key in keys {
                    named.push(check(&key, "example"));
                }
            }
        }
    }
    named
}

/// Every argument `text` names, in one of the three shapes, that its tool does
/// not take.
fn wrong_arguments(text: &str, served: &Schemas) -> Vec<Wrong> {
    arguments_named(text, served)
        .into_iter()
        .filter_map(|(w, taken)| (!taken).then_some(w))
        .collect()
}

/// Every `description` string in a schema, at any depth.
fn schema_descriptions<'a>(v: &'a Value, out: &mut Vec<&'a str>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                match (k.as_str(), x) {
                    ("description", Value::String(s)) => out.push(s),
                    _ => schema_descriptions(x, out),
                }
            }
        }
        Value::Array(xs) => xs.iter().for_each(|x| schema_descriptions(x, out)),
        _ => {}
    }
}

async fn served() -> (Schemas, Vec<(String, Vec<String>)>) {
    let s = ReflowService::in_memory().expect("in-memory service");
    let tools = s.tools_with_lessons_for_test().await;
    let schemas: Schemas = tools
        .iter()
        .map(|t| (t.name.to_string(), (*t.input_schema).clone()))
        .collect();
    let texts = tools
        .iter()
        .map(|t| {
            let mut texts: Vec<String> = t.description.iter().map(|d| d.to_string()).collect();
            let mut inner = Vec::new();
            let schema = Value::Object((*t.input_schema).clone());
            schema_descriptions(&schema, &mut inner);
            texts.extend(inner.into_iter().map(str::to_string));
            (t.name.to_string(), texts)
        })
        .collect();
    (schemas, texts)
}

#[tokio::test]
async fn every_argument_a_served_description_names_is_one_its_tool_takes() {
    let (schemas, texts) = served().await;
    assert!(schemas.len() >= 150, "a real tool list: {}", schemas.len());
    let mut found = Vec::new();
    let mut read = 0;
    for (name, texts) in &texts {
        for text in texts {
            let named = arguments_named(text, &schemas);
            read += named.len();
            for (w, _) in named.into_iter().filter(|(_, taken)| !taken) {
                found.push(format!(
                    "`{name}`'s served text names `{}` with `{}` ({}), which `{}` does not take",
                    w.tool, w.argument, w.shape, w.tool
                ));
            }
        }
    }
    found.sort();
    found.dedup();
    // 32 bulk sentences (two names each: the list and its items' keys) and
    // the hand-written ones: a reader that finds far fewer has gone quiet.
    assert!(
        read >= 64,
        "only {read} argument name(s) were read on the whole served surface: the reader no \
         longer finds the shapes it names, so this check has gone quiet"
    );
    assert!(
        found.is_empty(),
        "{} argument name(s) in served descriptions that their tool does not take — an agent \
         that follows the text is refused:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// The reader is held to real sentences in both directions, so it can rot
/// neither into silence nor into noise without this going red.
#[tokio::test]
async fn the_check_sees_every_shape_it_names() {
    let (schemas, _) = served().await;
    let wrong = |text: &str| wrong_arguments(text, &schemas);

    // The 2026-10-26 defect, verbatim: caught, naming `items`.
    let old = "MANY AT ONCE: `draw_edges`, items {\"tool\": \"invalidates\", \
               \"arguments\": {…}}, same checks, all or nothing.";
    assert_eq!(
        wrong(old),
        vec![Wrong {
            tool: "draw_edges".into(),
            argument: "items".into(),
            shape: "example",
        }],
        "the sentence that refused an agent on 2026-10-26 must be caught"
    );
    // Its correction passes, and a wrong key inside the example is caught.
    let new = "MANY AT ONCE: `draw_edges` with `edges`: [{\"tool\": \"invalidates\", \
               \"arguments\": {…}}], same checks, all or nothing.";
    assert_eq!(wrong(new), vec![], "the corrected sentence must pass");
    assert_eq!(
        wrong("`draw_edges` with `edges`: [{\"helper\": \"x\", \"arguments\": {}}]"),
        vec![Wrong {
            tool: "draw_edges".into(),
            argument: "edges[].helper".into(),
            shape: "example",
        }],
        "an example key the list's items do not take must be caught"
    );
    // An example with no introducing word: its keys are the tool's own.
    assert_eq!(
        wrong("call `draw_edges` {\"edges\": [], \"check_only\": true}"),
        vec![]
    );
    assert_eq!(
        wrong("call `draw_edges` {\"items\": []}").len(),
        1,
        "a top-level example key the tool does not take must be caught"
    );

    // WITH, as the surface says it.
    let with = "call `describe_schema` with `from` and `to`: it names which edge types";
    assert_eq!(wrong(with), vec![]);
    assert_eq!(
        wrong("call `describe_schema` with `from` and `towards`"),
        vec![Wrong {
            tool: "describe_schema".into(),
            argument: "towards".into(),
            shape: "with",
        }]
    );
    // A tool after "with" is not an argument.
    assert_eq!(wrong("pair `satisfies` with `verifies`"), vec![]);

    // NAMED ARGUMENT.
    assert_eq!(
        wrong("`snapshot_before_change`'s `repair` argument"),
        vec![]
    );
    assert_eq!(
        wrong("`snapshot_before_change`'s `mend` argument").len(),
        1,
        "a named argument the tool does not take must be caught"
    );
    // A reply field in the possessive is not an argument, and is not read.
    assert_eq!(wrong("`detect_defects`'s `swept.parked`"), vec![]);
}
