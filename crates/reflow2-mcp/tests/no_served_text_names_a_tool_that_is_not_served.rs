//! No text reflow2 serves names a tool it does not serve.
//!
//! # Why this exists, measured
//!
//! `fact:served-messages-still-name-record-change-after-0-92-0-removed-it-2026-10-26`.
//! 0.92.0 removed `record_change` and `manual_work_report` from the tool list
//! (`dec:the-two-renamed-stubs-are-removed-in-the-next-release`). The removal
//! was checked by the tool list and the changelog, and five texts the binary
//! serves went on telling the caller to call `record_change`: the coverage
//! note `propagate_change` attaches for every seed with no prior snapshot
//! (edge_prose.rs), the change-axis gap (detect.rs), the hierarchy move note
//! (hierarchy.rs), the planned-epoch refusal (temporal.rs) and the
//! `DesignEpoch.status` description in schema/temporal.yaml. A designer on
//! flo2.io followed one and was refused at the door. OBSERVED FAILING FIRST on
//! main 02e2739c8 with exactly those five.
//!
//! It is the sibling of
//! `a_served_description_names_only_arguments_its_tool_takes.rs` (#804), which
//! checks the ARGUMENTS a description names; this checks the TOOLS any served
//! text names.
//!
//! # What it reads
//!
//! 1. Every served tool's description and every `description` in its input
//!    schema, as `tools/list` serves them (lessons included).
//! 2. The served kit: every Markdown file under `getting-started/` (the skills
//!    `get_skill` serves and the instructions `get_instructions` serves, which
//!    build.rs embeds from there).
//! 3. The schema text `describe_schema` serves: `schema/*.yaml`.
//! 4. The fixed prose of every reply and refusal: every string literal in
//!    `crates/reflow2-core/src` and `crates/reflow2-mcp/src`. Driving every
//!    message from a fixture would reach only the ones a fixture happens to
//!    trigger, and the five above were each behind a different one; the
//!    literal is where the text is written, so it is read there. The one file
//!    left out is `deprecation.rs`, whose job is to name a removed tool in
//!    order to refuse it with what to do instead.
//!
//! # What counts as naming a tool that is not served
//!
//! - **A removed tool**, anywhere, as a whole word: every tool in the
//!   deprecation registry that the served list no longer carries. This is the
//!   class the five fell in, and it needs no guessing about what is a tool.
//! - **A counted refusing stub** (`deprecation::stub_names`): served, so a
//!   call to its old name is counted, and for that reason never named by
//!   served text — text that pointed agents at a stub would make the re-count
//!   measure the text rather than the need. A stub whose name carries an
//!   underscore is matched as a whole word, as a removed tool is; one that is
//!   an ordinary English word (`verifies`, `frontier`) only where text says to
//!   call or run it, in backticks — `verifies` is also an argument of
//!   `add_verification`, which text names as such. Served text names the
//!   survivor instead.
//! - **A call to a name nothing serves**: in tool descriptions and the kit, a
//!   backticked name right after "call" (or "Call", "calls") that looks like a
//!   tool (lower-case, with an underscore) and is not on the served list.
//!
//! ⚠️ ITS LIMITS, STATED. A tool named in plain prose with no backticks and
//! no "call" before it, which never was a tool, is not caught: nothing tells
//! a misspelt tool name from any other word. Graph text (what a design holds)
//! is data and is not read.

use reflow2_mcp::deprecation::{Item, REGISTRY, stub_names};
use reflow2_mcp::service::ReflowService;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

/// Every file under `dir` whose name ends in `ext`, sorted.
fn files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            files(&p, ext, out);
        } else if p.to_string_lossy().ends_with(ext) {
            out.push(p);
        }
    }
}

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Every string literal in Rust source, with the line it starts on. Comments
/// are skipped; normal, byte and raw strings are read; a `\` line
/// continuation is joined the way the compiler joins it.
fn string_literals(src: &str) -> Vec<(usize, String)> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let (mut i, mut line) = (0usize, 1usize);
    while i < b.len() {
        match b[i] {
            b'\n' => {
                line += 1;
                i += 1;
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let mut depth = 1;
                i += 2;
                while i < b.len() && depth > 0 {
                    if b[i] == b'\n' {
                        line += 1;
                    }
                    if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                        depth += 1;
                        i += 2;
                    } else if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                        depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
            }
            b'r' if (i == 0 || !is_ident(b[i - 1]))
                && matches!(b.get(i + 1), Some(b'"') | Some(b'#')) =>
            {
                let mut j = i + 1;
                let mut hashes = 0;
                while b.get(j) == Some(&b'#') {
                    hashes += 1;
                    j += 1;
                }
                if b.get(j) != Some(&b'"') {
                    i += 1; // a raw identifier, r#type
                    continue;
                }
                let start = j + 1;
                let mut k = start;
                while k < b.len()
                    && !(b[k] == b'"'
                        && b.len() >= k + 1 + hashes
                        && b[k + 1..k + 1 + hashes].iter().all(|&h| h == b'#'))
                {
                    k += 1;
                }
                let text = &src[start..k.min(b.len())];
                out.push((line, text.to_string()));
                line += text.matches('\n').count();
                i = k + 1 + hashes;
            }
            b'"' => {
                let start = i + 1;
                let mut k = start;
                while k < b.len() && b[k] != b'"' {
                    k += if b[k] == b'\\' { 2 } else { 1 };
                }
                let raw = &src[start..k.min(b.len())];
                // `\` + newline + indentation is nothing, as the compiler reads it.
                let mut text = String::new();
                let mut rest = raw;
                while let Some(at) = rest.find("\\\n") {
                    text.push_str(&rest[..at]);
                    rest = rest[at + 2..].trim_start();
                }
                text.push_str(rest);
                out.push((line, text));
                line += raw.matches('\n').count();
                i = k + 1;
            }
            b'\'' => {
                // A char literal ('x', '\n', '—') or a lifetime ('a).
                if b.get(i + 1) == Some(&b'\\') {
                    let mut k = i + 2;
                    while k < b.len() && b[k] != b'\'' {
                        k += 1;
                    }
                    i = k + 1;
                } else if let Some(ch) = src[i + 1..].chars().next() {
                    let l = ch.len_utf8();
                    i += if b.get(i + 1 + l) == Some(&b'\'') {
                        2 + l
                    } else {
                        1
                    };
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    out
}

/// Does `text` hold `name` as a whole word?
fn names(text: &str, name: &str) -> bool {
    let b = text.as_bytes();
    let mut from = 0;
    while let Some(at) = text[from..].find(name).map(|i| from + i) {
        let end = at + name.len();
        let before = at == 0 || !is_ident(b[at - 1]);
        let after = end >= b.len() || !is_ident(b[end]);
        if before && after {
            return true;
        }
        from = end;
    }
    false
}

/// Does `text` hold `name` as a whole word that is not a code path — after
/// `::` (`readiness.rs::gate_on`) or `function: ` (a schema file naming the
/// core function a rule reads), where it names the computation, which stays,
/// and not the tool?
fn names_as_tool(text: &str, name: &str) -> bool {
    let b = text.as_bytes();
    let mut from = 0;
    while let Some(at) = text[from..].find(name).map(|i| from + i) {
        let end = at + name.len();
        let before = at == 0 || !is_ident(b[at - 1]);
        let after = end >= b.len() || !is_ident(b[end]);
        let path = text[..at].ends_with("::") || text[..at].ends_with("function: ");
        if before && after && !path {
            return true;
        }
        from = end;
    }
    false
}

/// Text that names a counted refusing stub and is served ONLY inside that
/// stub's own retired computation's reply, so nothing serves it while the
/// stub stands: (file, name, why). `ility_report` labels each signal with the
/// report it came from.
const SERVED_ONLY_BY_A_RETIRED_COMPUTATION: &[(&str, &str, &str)] = &[
    (
        "crates/reflow2-core/src/ility.rs",
        "surprising_connections",
        "a signal's source label in ility_report's reply, retired with it",
    ),
    (
        "crates/reflow2-core/src/ility.rs",
        "granularity_report",
        "a signal's source label in ility_report's reply, retired with it",
    ),
];

/// The backticked names right after "call" in `text` that look like a tool.
fn called(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for lead in ["call `", "Call `", "calls `"] {
        let mut from = 0;
        while let Some(at) = text[from..].find(lead).map(|i| from + i) {
            let start = at + lead.len();
            if (at == 0 || !is_ident(text.as_bytes()[at - 1]))
                && let Some(len) = text[start..].find('`')
            {
                let name = &text[start..start + len];
                if name.contains('_')
                    && name
                        .bytes()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
                {
                    out.push(name);
                }
            }
            from = start;
        }
    }
    out
}

fn schema_descriptions(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                match (k.as_str(), x) {
                    ("description", Value::String(s)) => out.push(s.clone()),
                    _ => schema_descriptions(x, out),
                }
            }
        }
        Value::Array(xs) => xs.iter().for_each(|x| schema_descriptions(x, out)),
        _ => {}
    }
}

/// Every served text as (where, text); `prose` marks the ones the "call"
/// reading applies to (tool descriptions and the kit).
async fn served_texts() -> (BTreeSet<String>, Vec<(String, String, bool)>) {
    let s = ReflowService::in_memory().expect("in-memory service");
    let tools = s.tools_with_lessons_for_test().await;
    let served: BTreeSet<String> = tools.iter().map(|t| t.name.to_string()).collect();
    let mut texts = Vec::new();
    for t in &tools {
        if let Some(d) = &t.description {
            texts.push((format!("tool {}: description", t.name), d.to_string(), true));
        }
        let mut inner = Vec::new();
        schema_descriptions(&Value::Object((*t.input_schema).clone()), &mut inner);
        for d in inner {
            texts.push((format!("tool {}: input schema", t.name), d, true));
        }
    }
    let root = repo();
    let mut kit = Vec::new();
    files(&root.join("getting-started"), ".md", &mut kit);
    for p in kit {
        let text = std::fs::read_to_string(&p).expect("a kit file");
        let rel = p.strip_prefix(&root).unwrap_or(&p).display().to_string();
        for (n, l) in text.lines().enumerate() {
            texts.push((format!("{rel}:{}", n + 1), l.to_string(), true));
        }
    }
    let mut yaml = Vec::new();
    files(&root.join("schema"), ".yaml", &mut yaml);
    for p in yaml {
        let text = std::fs::read_to_string(&p).expect("a schema file");
        let rel = p.strip_prefix(&root).unwrap_or(&p).display().to_string();
        for (n, l) in text.lines().enumerate() {
            texts.push((format!("{rel}:{}", n + 1), l.to_string(), false));
        }
    }
    let mut rs = Vec::new();
    files(&root.join("crates/reflow2-core/src"), ".rs", &mut rs);
    files(&root.join("crates/reflow2-mcp/src"), ".rs", &mut rs);
    for p in rs {
        if p.file_name().is_some_and(|f| f == "deprecation.rs") {
            continue;
        }
        let src = std::fs::read_to_string(&p).expect("a source file");
        let rel = p.strip_prefix(&root).unwrap_or(&p).display().to_string();
        for (line, lit) in string_literals(&src) {
            texts.push((format!("{rel}:{line}"), lit, false));
        }
    }
    (served, texts)
}

#[tokio::test]
async fn no_served_text_names_a_tool_that_is_not_served() {
    let (served, texts) = served_texts().await;
    assert!(served.len() >= 150, "a real tool list: {}", served.len());
    let removed: Vec<String> = REGISTRY
        .iter()
        .filter_map(|d| match &d.item {
            Item::Tool { tool } if !served.contains(tool.as_ref()) => Some(tool.to_string()),
            _ => None,
        })
        .collect();
    let stubs: Vec<&str> = stub_names(REGISTRY);
    assert!(
        stubs.iter().all(|s| served.contains(*s)),
        "a counted refusing stub is served: {stubs:?}"
    );
    let mut found = Vec::new();
    let mut calls_read = 0;
    for (place, text, prose) in &texts {
        for name in &removed {
            if names(text, name) {
                found.push(format!("{place}: names `{name}`, which was removed"));
            }
        }
        for name in &stubs {
            // A literal that IS the name is the code's key for the stub (its
            // handler answering under it), not text a reader follows.
            if text == name {
                continue;
            }
            if SERVED_ONLY_BY_A_RETIRED_COMPUTATION
                .iter()
                .any(|(file, n, _)| n == name && place.starts_with(&format!("{file}:")))
            {
                continue;
            }
            let named = if name.contains('_') {
                names_as_tool(text, name)
            } else {
                ["call `", "Call `", "calls `", "run `", "Run `", "runs `"]
                    .iter()
                    .any(|lead| text.contains(&format!("{lead}{name}`")))
            };
            if named {
                found.push(format!(
                    "{place}: names `{name}`, a counted refusing stub — name its survivor"
                ));
            }
        }
        if *prose {
            for name in called(text) {
                calls_read += 1;
                if !served.contains(name) || stubs.contains(&name) {
                    found.push(format!(
                        "{place}: says to call `{name}`, which nothing serves"
                    ));
                }
            }
        }
    }
    // A reader that finds nothing to read has gone quiet, not clean.
    let literals = texts
        .iter()
        .filter(|(p, _, _)| p.ends_with(|c: char| c.is_ascii_digit()) && p.contains(".rs:"))
        .count();
    assert!(
        literals > 5_000,
        "only {literals} string literal(s) were read"
    );
    assert!(
        calls_read >= 20,
        "only {calls_read} \"call `tool`\" sentence(s) were read"
    );
    found.sort();
    found.dedup();
    assert!(
        found.is_empty(),
        "{} served text(s) name a tool that is not served — an agent that follows one is \
         refused:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// The reader is held to real text in both directions.
#[test]
fn the_reader_sees_the_shapes_it_names() {
    let src = r##"
        // "record_change" in a comment is not served
        let a = "Call `record_change` first";
        let b = "one \
                 `snapshot_before_change` two";
        let c = r#"raw "quoted" text"#;
        let d = 'x'; let e = '\''; fn f<'a>(x: &'a str) {}
        let g = "after";
    "##;
    let lits: Vec<String> = string_literals(src).into_iter().map(|(_, s)| s).collect();
    assert_eq!(
        lits,
        vec![
            "Call `record_change` first".to_string(),
            "one `snapshot_before_change` two".to_string(),
            "raw \"quoted\" text".to_string(),
            "after".to_string(),
        ]
    );
    assert!(names("Call `record_change` first", "record_change"));
    assert!(!names("record_changes", "record_change"));
    assert!(!names("a_record_change", "record_change"));
    assert_eq!(called("call `get_node` then call `x`"), vec!["get_node"]);
    assert_eq!(called("Call `no_such_tool` first"), vec!["no_such_tool"]);
}
