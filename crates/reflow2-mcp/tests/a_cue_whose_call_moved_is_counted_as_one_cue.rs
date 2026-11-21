//! A cue whose call moved to another tool is counted as ONE cue across the
//! move, and a call to the old tool acts on it.
//!
//! `cap:every-reply-carries-one-ranked-budgeted-cues-block`, under
//! `dec:idea-what-the-cue-count-counts-reference-hints-out-a-carried-pointer-in`
//! (option C): the handshake's report cue moved with its tool on 2026-10-30,
//! when `graph_report_markdown` became a counted refusing stub folded into
//! `graph_report` with `format: "markdown"`. A count read across that day
//! reads the two ids as one cue, under the survivor's.
//!
//! WHY THIS IS A FILE OF ITS OWN, not a unit test in `src/cues.rs`: a test of
//! the fold has to name the stub, and every string literal in `src/` is read
//! as served text by `no_served_text_names_a_tool_that_is_not_served.rs`,
//! which refuses served text that names a counted refusing stub. These
//! literals are never served; here they are not read as if they were.

use reflow2_mcp::cues::{Standing, canonical, standing, tally};
use reflow2_mcp::usage::{Outcome, UsageLine};

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

#[test]
fn a_cue_whose_call_moved_is_counted_as_one_cue() {
    assert_eq!(
        canonical("instructions/graph_report_markdown"),
        "instructions/graph_report"
    );
    assert_eq!(canonical("next/propagate_change"), "next/propagate_from");
    assert_eq!(canonical("loop_hint/loop_status"), "loop_hint/loop_status");
    assert_eq!(
        canonical("hint/get_skill:root-cause"),
        "hint/get_skill:root-cause"
    );
    let lines = [
        // Before the fold: the old id, followed by the old tool.
        line("cue", "", "s1", &["instructions/graph_report_markdown"]),
        line("call", "graph_report_markdown", "s1", &[]),
        // After it: the new id, followed by the survivor.
        line("cue", "", "s2", &["instructions/graph_report"]),
        line("call", "get_node", "s2", &[]),
        line("call", "graph_report", "s2", &[]),
    ];
    let refs: Vec<&UsageLine> = lines.iter().collect();
    let t = tally(&refs);
    assert!(!t.by_cue.contains_key("instructions/graph_report_markdown"));
    let c = &t.by_cue["instructions/graph_report"];
    assert_eq!((c.offered, c.followed_within_5), (2, 2), "{c:?}");
    assert_eq!(
        t.moved["instructions/graph_report_markdown"],
        "instructions/graph_report"
    );
}

/// The counts table is read under the survivor's id too: the old id stands
/// where the new one does (never followed, in the 2026-11-21 counts).
#[test]
fn the_old_id_reads_the_counts_table_under_the_survivors() {
    assert_eq!(
        standing("instructions/graph_report_markdown"),
        standing("instructions/graph_report")
    );
    assert_eq!(
        standing("instructions/graph_report_markdown"),
        Standing::NeverFollowed
    );
}
