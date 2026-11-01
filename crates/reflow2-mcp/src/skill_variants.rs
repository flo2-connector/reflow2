//! A skill line that differs when the engine is served for others carries a
//! local version and a hosted version, and the engine serves the one for how
//! it runs.
//!
//! `req:a-served-skill-says-what-is-true-where-it-is-served`, governed by
//! `dec:a-skill-line-that-differs-when-hosted-has-a-local-and-a-hosted-version`
//! (Anthony, 2026-11-01). Until then every skill was served unchanged wherever
//! the engine ran, and the bodies were written for a local install: on flo2.io,
//! `help` told a chat user "a program that runs on your own machine … Nothing
//! leaves your machine" about a design kept on flo2.io
//! (`fact:root-cause-a-served-skill-is-true-only-where-it-was-written-and-help-tells-flo2-chat-users-nothing-leaves-their-machine-2026-11-01`).
//!
//! THE MARKING, in a SKILL.md, each marker alone on its line:
//!
//! ```text
//! <!-- reflow2:local -->
//! lines true where the engine runs on the reader's own machine
//! <!-- reflow2:hosted -->
//! lines true where it is served for others (flo2.io, or any host)
//! <!-- reflow2:end -->
//! ```
//!
//! An HTML comment, so a SKILL.md read as Markdown shows neither marker. Either
//! side may be empty (a step that exists only one way), never both. Only the
//! lines that really differ are marked; a line that can be worded once so it
//! is true everywhere is worded once instead.
//!
//! This file is std-only on purpose: `build.rs` includes it (`#[path]`) to
//! refuse a malformed marking at build time, and the crate uses it to serve.
//! `tools/skill_variants.py` is its twin for the lint and the repository's own
//! skill mirrors, and `tests/a_served_skill_says_what_is_true_where_it_is_served.rs`
//! holds the two to one answer (each mirror is the Rust local rendering).

/// Where the reader is, as far as a skill's text is concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Served {
    /// The engine runs on the reader's own machine (stdio, `--shared`, `--http`
    /// on loopback).
    Local,
    /// The engine is served for others: behind a trusted gateway, as an OAuth
    /// resource server, or exposed with no caller rule.
    Hosted,
}

/// Opens a marked block; the local version follows.
pub const LOCAL: &str = "<!-- reflow2:local -->";
/// Ends the local version; the hosted version follows.
pub const HOSTED: &str = "<!-- reflow2:hosted -->";
/// Ends the block.
pub const END: &str = "<!-- reflow2:end -->";

#[derive(Clone, Copy, PartialEq, Eq)]
enum In {
    Both,
    Local,
    Hosted,
}

/// The text as it reads `served`: the marker lines gone, and of each marked
/// block only the side for `served`. Unmarked text is returned unchanged. Call
/// it on text [`problems`] passes; a malformed marking is rendered as far as it
/// can be read, never repaired.
pub fn render(text: &str, served: Served) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at = In::Both;
    for line in text.split_inclusive('\n') {
        match line.trim_end_matches(['\n', '\r']) {
            LOCAL => at = In::Local,
            HOSTED => at = In::Hosted,
            END => at = In::Both,
            _ => {
                let keep = match at {
                    In::Both => true,
                    In::Local => served == Served::Local,
                    In::Hosted => served == Served::Hosted,
                };
                if keep {
                    out.push_str(line);
                }
            }
        }
    }
    out
}

/// Whether `text` holds any marked block.
pub fn has_variants(text: &str) -> bool {
    text.lines().any(|l| l == LOCAL)
}

/// Every way the marking in `text` is malformed, each naming its line. Empty
/// when every block is `local`, `hosted`, `end` in that order, none is nested,
/// none sits in the frontmatter, and none is empty on both sides.
pub fn problems(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut open: Option<(usize, In, usize, usize)> = None; // (line, side, local lines, hosted lines)
    let mut frontmatter_end = 0;
    let lines: Vec<&str> = text.lines().collect();
    if lines.first() == Some(&"---") {
        frontmatter_end = lines
            .iter()
            .skip(1)
            .position(|l| *l == "---")
            .map(|p| p + 2)
            .unwrap_or(0);
    }
    for (i, line) in lines.iter().enumerate() {
        let n = i + 1;
        let marker = matches!(*line, LOCAL | HOSTED | END);
        if !marker && line.contains("<!-- reflow2:") {
            out.push(format!(
                "line {n}: a reflow2 marker must stand alone on its line, exactly as written: \
                 {line:?}"
            ));
            continue;
        }
        if marker && i < frontmatter_end {
            out.push(format!("line {n}: a marker inside the frontmatter"));
            continue;
        }
        match (*line, open) {
            (LOCAL, None) => open = Some((n, In::Local, 0, 0)),
            (LOCAL, Some((at, ..))) => out.push(format!(
                "line {n}: a block opened inside the one opened at line {at}"
            )),
            (HOSTED, Some((at, In::Local, l, h))) => open = Some((at, In::Hosted, l, h)),
            (HOSTED, _) => out.push(format!(
                "line {n}: `{HOSTED}` with no `{LOCAL}` open before it"
            )),
            (END, Some((at, In::Hosted, l, h))) => {
                if l == 0 && h == 0 {
                    out.push(format!("line {at}: a block with nothing on either side"));
                }
                open = None;
            }
            (END, _) => out.push(format!(
                "line {n}: `{END}` with no `{LOCAL}` … `{HOSTED}` before it"
            )),
            (_, Some((at, side, l, h))) if !line.trim().is_empty() => {
                open = Some(match side {
                    In::Local => (at, side, l + 1, h),
                    _ => (at, side, l, h + 1),
                })
            }
            _ => {}
        }
    }
    if let Some((at, ..)) = open {
        out.push(format!(
            "line {at}: a block that is never closed with `{END}`"
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARKED: &str = "a\n<!-- reflow2:local -->\nhere\n<!-- reflow2:hosted -->\nthere\n\
                          <!-- reflow2:end -->\nb\n";

    #[test]
    fn each_side_reads_its_own_lines_and_neither_reads_a_marker() {
        assert_eq!(render(MARKED, Served::Local), "a\nhere\nb\n");
        assert_eq!(render(MARKED, Served::Hosted), "a\nthere\nb\n");
        assert!(problems(MARKED).is_empty(), "{:?}", problems(MARKED));
        assert!(has_variants(MARKED));
    }

    #[test]
    fn unmarked_text_is_served_unchanged_both_ways() {
        let plain = "---\nname: x\n---\nbody\nno newline at the end";
        assert_eq!(render(plain, Served::Local), plain);
        assert_eq!(render(plain, Served::Hosted), plain);
        assert!(!has_variants(plain));
    }

    #[test]
    fn a_side_may_be_empty_and_a_block_may_not_be() {
        let local_only =
            "<!-- reflow2:local -->\nstep\n<!-- reflow2:hosted -->\n<!-- reflow2:end -->\n";
        assert!(problems(local_only).is_empty());
        assert_eq!(render(local_only, Served::Hosted), "");
        let empty = "<!-- reflow2:local -->\n\n<!-- reflow2:hosted -->\n<!-- reflow2:end -->\n";
        assert_eq!(problems(empty).len(), 1);
    }

    #[test]
    fn a_malformed_marking_is_named_by_line() {
        for (text, said) in [
            ("<!-- reflow2:local -->\nx\n", "never closed"),
            (
                "<!-- reflow2:hosted -->\nx\n<!-- reflow2:end -->\n",
                "no `<!-- reflow2:local -->`",
            ),
            (
                "<!-- reflow2:local -->\n<!-- reflow2:local -->\n",
                "inside the one",
            ),
            (
                "<!-- reflow2:local -->\nx\n<!-- reflow2:end -->\n",
                "no `<!-- reflow2:local -->` …",
            ),
            ("text <!-- reflow2:local -->\n", "alone on its line"),
            ("---\n<!-- reflow2:local -->\n---\n", "frontmatter"),
        ] {
            let found = problems(text).join(" | ");
            assert!(found.contains(said), "{text:?}: {found}");
        }
    }
}
