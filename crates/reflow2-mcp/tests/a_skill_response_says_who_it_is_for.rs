//! Does the skill response tell the agent whose words to use?
//!
//! Anthony, 2026-08-19: *"how come we can't attach a parameter to each skill
//! request that returns something to the user. like 'lens' or 'session_user'
//! that reminds the agent on every skill request?"*
//!
//! The rule being carried — speak the reader's domain, never reflow2's —
//! shipped as served prose, and prose has one failure mode this project has
//! measured twice in a week: **read once, then drifting as the conversation
//! grows**, with nothing able to observe whether it held. A field on the
//! response the agent fetches immediately before doing the work is re-read every
//! call. Same content, a carrier that does not decay — which is the argument
//! `loop_hint` already won.
//!
//! `the_design_says_whose_words_to_use.rs` (core) pins the COMPUTATION. This
//! pins the CONTRACT: that the field actually reaches the wire, in both states,
//! and that it never withholds a skill.

use reflow2_mcp::service::*;
use reflow2_mcp::tools::skills_tools::{GetSkillReq, ListSkillsReq};
use rmcp::handler::server::wrapper::Parameters;

macro_rules! j {
    ($call:expr) => {
        $call
            .await
            .expect("tool ok")
            .structured_content
            .expect("structured content present")
    };
}

async fn service() -> ReflowService {
    let s = ReflowService::in_memory().expect("in-memory service");
    j!(s.add_project(Parameters(ProjectReq {
        id: "proj:x".into(),
        name: Some("X".to_string()),
        description: None,
        decomposition_levels: None,
        status: None,
    })));
    s
}

async fn described(s: &ReflowService, id: &str, kind: &str, description: Option<&str>) {
    j!(s.add_contributor(Parameters(ContributorReq {
        id: id.into(),
        name: Some(id.to_string()),
        kind: Some(kind.into()),
        handle: None,
        description: description.map(str::to_string),
    })));
}

/// ⭐ THE CASE THE WHOLE THING EXISTS FOR. When nobody in the design has a
/// role written down, the response SAYS that the design does not say who is
/// reading — and is not meant to — rather than omitting the field and letting
/// silence read as "nothing to worry about". Where the reader's lens DOES come
/// from (host, own memory, their words) is pinned by
/// `no_served_text_writes_a_readers_persona_into_the_design.rs`.
#[tokio::test]
async fn a_skill_says_plainly_that_the_design_does_not_say_who_is_reading() {
    let s = service().await;
    described(&s, "who:ann", "person", None).await;

    let out = j!(s.get_skill(Parameters(GetSkillReq {
        name: "where-am-i".into(),
    })));
    let lens = out
        .get("lens")
        .and_then(|v| v.as_str())
        .expect("a skill response carries a lens");

    assert!(
        lens.contains("THIS DESIGN DOES NOT SAY WHO IS READING"),
        "the silent case must be stated, not implied by an empty list: {lens}"
    );
    assert!(
        lens.contains("who:ann"),
        "and it must NAME who is recorded — a bare count is not actionable: {lens}"
    );
}

/// The ordinary case: the response names who is described, for attribution
/// and their role on this design.
#[tokio::test]
async fn a_skill_names_the_people_the_design_can_describe() {
    let s = service().await;
    described(&s, "who:ann", "person", Some("Vet. Cattle. Not software.")).await;

    let out = j!(s.get_skill(Parameters(GetSkillReq {
        name: "where-am-i".into(),
    })));
    let lens = out.get("lens").and_then(|v| v.as_str()).expect("lens");

    assert!(lens.contains("who:ann"), "{lens}");
    assert!(
        !lens.contains("THIS DESIGN DOES NOT SAY WHO IS READING"),
        "a design that CAN answer must not report itself silent: {lens}"
    );
}

/// ⭐ IT NEVER CLAIMS TO KNOW WHO IS AT THE KEYBOARD. reflow2 has no such
/// notion — a seat names a session, never a person — so the lens reports what
/// the DESIGN holds and leaves matching the reader to the agent. Confidently
/// addressing the wrong person would be worse than saying nothing.
#[tokio::test]
async fn the_lens_reports_what_the_design_holds_never_who_is_present() {
    let s = service().await;
    described(&s, "who:ann", "person", Some("Vet.")).await;

    let out = j!(s.get_skill(Parameters(GetSkillReq {
        name: "where-am-i".into(),
    })));
    let lens = out.get("lens").and_then(|v| v.as_str()).expect("lens");

    assert!(
        lens.contains("never who is at the keyboard"),
        "the limit must be stated to the agent, not merely respected in code: {lens}"
    );
}

/// The catalogue carries it too: an agent choosing WHICH skill to read is
/// already deciding how to proceed, and that is early enough to matter.
#[tokio::test]
async fn the_catalogue_carries_the_lens_as_well() {
    let s = service().await;
    described(&s, "who:ann", "person", Some("Beamline scientist.")).await;

    let out = j!(s.list_skills(Parameters(ListSkillsReq { budget_chars: None })));
    assert!(
        out.get("lens")
            .and_then(|v| v.as_str())
            .is_some_and(|l| l.contains("who:ann")),
        "list_skills must carry the lens too"
    );
}

/// 🛑 THE LENS MUST NEVER WITHHOLD A SKILL. The body is what the agent came
/// for; the reminder is an addition. This pins that the skill still arrives
/// whole alongside it — a reminder that could cost a skill would trade a nudge
/// for an outage.
#[tokio::test]
async fn the_skill_body_still_arrives_whole() {
    let s = service().await;

    let out = j!(s.get_skill(Parameters(GetSkillReq {
        name: "detect-and-ask".into(),
    })));

    assert_eq!(
        out.get("name").and_then(|v| v.as_str()),
        Some("detect-and-ask")
    );
    assert!(
        out.get("body")
            .and_then(|v| v.as_str())
            .is_some_and(|b| b.contains("gap_to_prompt")),
        "the skill body must be served in full regardless of the lens"
    );
}
