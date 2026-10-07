//! A root cause in a family of designs looks in every member, and records the
//! cause where it lives; a new idea or requirement is asked whether it belongs
//! under a whole already recorded.
//!
//! `req:a-root-cause-across-a-family-looks-in-every-member-and-records-the-cause-where-it-lives`
//! and F4 of `req:the-pieces-of-one-picture-are-found-together`, both accepted
//! 2026-10-07. Measured the same day
//! (fact:a-root-cause-run-from-a-hub-investigates-one-design-and-the-hub-chooses-which-2026-10-07):
//! the root-cause skill never mentioned another design, and the hub skill sent
//! it to the one design chosen as the place to RECORD, so where to look was
//! decided by where to write.
//!
//! These pin the served words, matched case-insensitively and by the tool or
//! field each step names, so a rewording that keeps the step passes and a
//! deletion of the step fails.

use reflow2_mcp::skills::SKILLS;

fn body(name: &str) -> String {
    SKILLS
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("{name} must be served"))
        .body
        .to_lowercase()
}

#[test]
fn the_root_cause_skill_walks_the_member_designs() {
    let b = body("root-cause");
    for (what, needle) in [
        (
            "a neighbours pass that names member designs",
            "member design",
        ),
        (
            "past causes searched in every member, not only here",
            "every member",
        ),
        (
            "the members that moved since this design last looked",
            "upstream_status",
        ),
        (
            "the interfaces at the seam against the other side",
            "other side",
        ),
        (
            "impact walked backwards from a member that moved",
            "arriving_from",
        ),
        (
            "a member it could not reach is named, never skipped",
            "not looked at",
        ),
    ] {
        assert!(
            b.contains(needle),
            "the root-cause skill must carry {what} (looked for {needle:?})"
        );
    }
}

#[test]
fn the_root_cause_skill_records_the_cause_where_it_lives() {
    let b = body("root-cause");
    assert!(
        b.contains("where the cause lives") || b.contains("where it lives"),
        "step 8 must record the cause in the design where it lives, not where it was asked"
    );
}

#[test]
fn the_hub_skill_separates_where_to_look_from_where_to_record() {
    let b = body("hub");
    assert!(
        b.contains("where to look") && b.contains("where to record"),
        "the hub skill must say that where to record does not decide where to look"
    );
}

#[test]
fn a_new_piece_is_asked_whether_it_belongs_under_a_whole_already_recorded() {
    for skill in ["capture-intent", "brainstorm", "link-ideas"] {
        let b = body(skill);
        assert!(
            b.contains("belongs under") && b.contains("whole"),
            "{skill} must ask whether a new piece belongs under a whole already recorded (F4)"
        );
    }
}
