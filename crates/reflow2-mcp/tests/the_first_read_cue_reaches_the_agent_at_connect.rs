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

use reflow2_mcp::cues::{FIRST_READ_CUE, INSTRUCTIONS_OFFER};
use reflow2_mcp::service::ReflowService;
use rmcp::ServerHandler;

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
