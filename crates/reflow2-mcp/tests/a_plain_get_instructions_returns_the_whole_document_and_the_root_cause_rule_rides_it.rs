//! The working instructions arrive whole, and the root-cause rule rides every
//! channel a session reads without being asked.
//!
//! Two failures measured on 2026-10-06, one inside the other:
//!
//! 1. **A plain `get_instructions` returned no instructions from v0.78.0 to
//!    v0.80.0.** The document passed 30,000 bytes on 2026-09-30, the shared
//!    reply budget was 30,000, and the tool withholds an over-budget document
//!    whole. So the call the handshake tells every session to make FIRST got a
//!    table of contents. A 2026-09-28 record said a test pinned exactly this;
//!    no such test was ever committed. This file is that pin.
//! 2. **The root-cause rule reached no project but reflow2's own.** It lived
//!    in the repo's contributor AGENTS.md; the served instructions, the
//!    handshake and the VS Code route file never named it. A hub session wrote
//!    a cause in chat, called no skill and no tool, and the person had to ask
//!    (fact:a-cause-written-in-a-reply-is-seen-by-nothing-and-the-root-cause-
//!    rule-was-served-to-no-consumer-2026-10-06).

use reflow2_mcp::service::ReflowService;
use reflow2_mcp::skills::{INSTRUCTIONS, VSCODE_TERMINAL_ROUTE};
use reflow2_mcp::tools::skills_tools::GetInstructionsReq;
use rmcp::ServerHandler;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

async fn get_instructions(svc: &ReflowService, args: Value) -> Value {
    svc.get_instructions(Parameters(
        serde_json::from_value::<GetInstructionsReq>(args).expect("valid arguments"),
    ))
    .await
    .expect("get_instructions answers")
    .structured_content
    .expect("structured content present")
}

#[tokio::test]
async fn a_plain_call_returns_the_whole_document_however_long_it_grows() {
    let svc = ReflowService::in_memory().expect("an in-memory design opens");
    let reply = get_instructions(&svc, json!({})).await;
    assert_eq!(
        reply["instructions"].as_str(),
        Some(INSTRUCTIONS),
        "a plain get_instructions must return the whole document ({} bytes); it returned {}",
        INSTRUCTIONS.len(),
        reply.get("budget").unwrap_or(&Value::Null)
    );
    assert_eq!(reply["returned_bytes"], json!(INSTRUCTIONS.len()));
    assert!(
        INSTRUCTIONS.len() > 30_000,
        "this pin is only worth having while the document is past the shared 30,000 reply \
         budget; if it has shrunk below it, the default-limit case is no longer exercised"
    );
}

#[tokio::test]
async fn a_caller_that_names_its_cap_still_gets_the_manifest_not_half_a_document() {
    let svc = ReflowService::in_memory().expect("an in-memory design opens");
    let reply = get_instructions(&svc, json!({"budget_chars": 20_000})).await;
    assert!(
        reply["instructions"].is_null(),
        "a document longer than the caller's cap is withheld whole, never trimmed"
    );
    assert_eq!(reply["budget"]["detail"], json!("whole_document_withheld"));
    assert!(
        reply["sections"].as_array().is_some_and(|s| !s.is_empty()),
        "and the manifest comes back, so it can be fetched a section at a time"
    );
}

#[test]
fn the_served_instructions_carry_both_tiers_of_the_root_cause_rule() {
    for needle in [
        "## When something fails, find its cause before you fix it",
        "`search_design` the exact error text",
        "`get_skill root-cause`",
        "in a reply to the person",
    ] {
        assert!(
            INSTRUCTIONS.contains(needle),
            "the served instructions must say {needle:?}"
        );
    }
    // Ahead of the tail a capped client loses: a ~22 KB cap (measured on a
    // consumer's client, 2026-08-14) must still deliver it.
    let at = INSTRUCTIONS
        .find("## When something fails")
        .expect("the section is present");
    assert!(
        at < 8_000,
        "the root-cause section sits at byte {at}; keep it among the standing rules up front"
    );
}

#[test]
fn the_handshake_names_the_root_cause_skill_before_a_client_would_cut_it() {
    let svc = ReflowService::in_memory().expect("an in-memory design opens");
    let instructions = svc
        .get_info()
        .instructions
        .expect("the handshake carries instructions");
    let needle = "`get_skill root-cause` and follow it before the cause is written";
    let at = instructions
        .find(needle)
        .expect("the handshake must name the root-cause skill and its trigger");
    // Claude Code shows about the first 2,000 characters of a server's
    // instructions. A host may put ~800 characters ahead of reflow2's own (the
    // flo2.io gateway does), and an ephemeral design opens with its warning.
    let chars = instructions[..at + needle.len()].chars().count();
    assert!(
        chars < 1_200,
        "the root-cause sentence ends at character {chars} of the handshake; past ~1,200 a \
         hosted design's prefix pushes it beyond what a client shows"
    );
}

#[test]
fn the_vscode_route_file_names_the_root_cause_skill_too() {
    assert!(
        VSCODE_TERMINAL_ROUTE.contains(r#"reflow2 read get_skill '{"name": "root-cause"}'"#),
        "a VS Code agent on the terminal route never sees the handshake; its always-loaded \
         instructions file must carry the trigger itself"
    );
}
