//! The connect-time instructions name the first read a newcomer makes to learn
//! where the design stands.
//!
//! `cap:the-first-read-cue-reaches-the-agent-before-its-first-call`, for
//! `req:reflow2-places-cues-on-the-agents-side-so-they-reach-an-agent-before-its-first-call`.
//! The handshake is the one channel that reaches every session before its
//! first call. A reader with a reading budget scored 5.5/25 from a design that
//! held the answers, and 17.5/25 when one line told it where the current
//! state was kept
//! (`fact:root-cause-a-budget-limited-reader-was-not-cued-to-the-designs-current-state-record-2026-10-29`).
//! The kit's session-start hooks carry the same sentence
//! (`tools/test_loop_nudge.py` pins that half).
//!
//! AND THE SERVED INSTRUCTIONS AGREE. #813 put the cue in the handshake and the
//! hooks and left `get_instructions` saying "Start with `open_questions`" at
//! step 0a, so an agent reading both got two first steps — and the next
//! measurement (the recall pilot's re-run) needs one. The second test holds
//! the served text to the cue: the first call step 0a names is the cue's first
//! read, `search_design` is named beside it, and the terminal route's session
//! start begins with the same read.

use reflow2_mcp::cues::{FIRST_READ_CUE, INSTRUCTIONS_OFFER};
use reflow2_mcp::service::ReflowService;
use rmcp::ServerHandler;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

/// The served instructions, or one section of them, as `get_instructions`
/// answers.
async fn served_instructions(svc: &ReflowService, section: Option<&str>) -> String {
    let v: Value = svc
        .get_instructions(Parameters(
            serde_json::from_value(json!({ "section": section })).expect("request shape"),
        ))
        .await
        .expect("get_instructions")
        .structured_content
        .expect("structured");
    v["instructions"]
        .as_str()
        .expect("the instructions text")
        .to_string()
}

/// The backquoted words of `text` that are served tool names, in order.
fn calls_named(text: &str, served: &[String]) -> Vec<String> {
    text.split('`')
        .skip(1)
        .step_by(2)
        .flat_map(|quoted| quoted.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')))
        .filter(|w| served.iter().any(|t| t == w))
        .map(String::from)
        .collect()
}

#[tokio::test]
async fn the_handshake_names_the_first_read_and_the_served_calls_it_names_exist() {
    let svc = ReflowService::in_memory().expect("an in-memory design opens");
    let instructions = svc
        .get_info()
        .instructions
        .expect("the handshake carries instructions");
    assert!(
        instructions.contains(FIRST_READ_CUE),
        "the first-read cue rides the handshake: {}",
        &instructions[..instructions.len().min(1500)]
    );
    // Every call the cue names is one this server serves, so a cue that is
    // counted is a cue that can be followed.
    let served: Vec<String> = svc
        .tools_with_lessons_for_test()
        .await
        .into_iter()
        .map(|t| t.name.to_string())
        .collect();
    for offer in INSTRUCTIONS_OFFER {
        let call = reflow2_mcp::cues::target_of(offer);
        assert!(served.iter().any(|t| t == call), "{call} is served");
    }
}

#[tokio::test]
async fn the_served_instructions_name_the_same_first_read() {
    let svc = ReflowService::in_memory().expect("an in-memory design opens");
    let served: Vec<String> = svc
        .tools_with_lessons_for_test()
        .await
        .into_iter()
        .map(|t| t.name.to_string())
        .collect();
    let first = reflow2_mcp::cues::target_of(INSTRUCTIONS_OFFER[0]);
    let second = reflow2_mcp::cues::target_of(INSTRUCTIONS_OFFER[1]);

    // Step 0a of the loop, as get_instructions serves it: from "0a." to the
    // next blank line.
    let doc = served_instructions(&svc, None).await;
    let at = doc.find("\n0a. ").expect("the loop's step 0a is served");
    let step = &doc[at + 1..];
    let step = &step[..step.find("\n\n").unwrap_or(step.len())];
    let calls = calls_named(step, &served);
    assert_eq!(
        calls.first().map(String::as_str),
        Some(first),
        "step 0a's first call is the cue's first read, so an agent reading the handshake \
         and the instructions gets ONE first step; it names {calls:?}:\n{step}"
    );
    assert!(
        calls.iter().any(|c| c == second),
        "step 0a names {second} for the design's own current-state record: {step}"
    );
    // Read as prose: a phrase wrapped across two lines is still the phrase.
    let flat = step.split_whitespace().collect::<Vec<_>>().join(" ");
    for words in ["current state", "status as of"] {
        assert!(flat.contains(words), "step 0a carries {words:?}: {step}");
    }
    if let Some(i) = calls.iter().position(|c| c == "open_questions") {
        assert!(
            i > 0,
            "open_questions comes after the first read: {calls:?}"
        );
    }

    // The terminal route (`reflow2 init --harness vscode-cli`), whose agents
    // read no handshake: its session start begins with the same read.
    let route = served_instructions(&svc, Some("vscode-terminal-route")).await;
    let at = route
        .find("## Start of a session")
        .expect("the terminal route has a session start");
    let start = &route[at..];
    let step_one = &start[start.find("\n1. ").expect("a first step")..];
    let step_one = &step_one[..step_one.find("\n2. ").unwrap_or(step_one.len())];
    assert!(
        step_one.contains(&format!("reflow2 read {first}")),
        "the terminal route's first step is the first read: {step_one}"
    );
    assert!(step_one.contains(second), "{step_one}");
}
