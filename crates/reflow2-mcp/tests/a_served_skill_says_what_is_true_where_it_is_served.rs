//! A served skill says what is true where it is served: a local install, or an
//! engine served for others (flo2.io's chat, its technical door, any host).
//!
//! `req:a-served-skill-says-what-is-true-where-it-is-served`, governed by
//! `dec:a-skill-line-that-differs-when-hosted-has-a-local-and-a-hosted-version`.
//!
//! MEASURED 2026-11-01
//! (`fact:root-cause-a-served-skill-is-true-only-where-it-was-written-and-help-tells-flo2-chat-users-nothing-leaves-their-machine-2026-11-01`):
//! through flo2.io's chat connector, `help` told a person "It is a program that
//! runs on your own machine … Nothing leaves your machine" about a design kept
//! on flo2.io, and 34 of 35 skills carried at least one line true only on a
//! local install, most of them "The standing rule is in AGENTS.md" to a chat
//! that has no AGENTS.md. Every skill was served unchanged wherever the engine
//! ran.
//!
//! So every skill is served both ways here, and the hosted text is read for
//! each claim below, each one true only where the engine runs on the reader's
//! own machine. A new claim of that kind joins the list in the same change
//! that writes its local and hosted versions.

use reflow2_mcp::caller::CallerRule;
use reflow2_mcp::service::ReflowService;
use reflow2_mcp::skills::{SKILLS, Served};
use rmcp::handler::server::wrapper::Parameters;
use std::path::PathBuf;

/// What only a local install can truthfully say, and why it is false when the
/// engine is served for others. Matched with whitespace collapsed.
const LOCAL_ONLY: &[(&str, &str)] = &[
    (
        "The standing rule is in AGENTS.md",
        "a chat has no AGENTS.md; the rule rides the server's own instructions",
    ),
    (
        "(see AGENTS.md)",
        "the loop is in the server's instructions, which every reader has",
    ),
    (
        "runs on your own machine",
        "a hosted engine runs on its host",
    ),
    (
        "leaves your machine",
        "a hosted design is kept by its host, not on the reader's machine",
    ),
    (
        "a file in your own repo",
        "a hosted design is kept by its host, not as a file in a repository",
    ),
    (
        "installs once per machine",
        "nothing is installed for a hosted design",
    ),
    (
        "delete a directory",
        "a hosted design is not a directory on the reader's machine",
    ),
    ("There is no server.", "a hosted design is served by one"),
    (
        "Claims travel in the export",
        "a hosted design's claims are read live by every session",
    ),
    (
        "cannot hold the same graph at once",
        "every session writes the one live design through its host",
    ),
    (
        "out of `.reflow2/graph.client.json`",
        "that file is on the host's disk, out of the reader's reach",
    ),
    (
        "the install line in `getting-started/SETUP.md`",
        "nothing is installed for a hosted design",
    ),
    (
        "with a `path` inside the repo",
        "a hosted engine's path names a file on its host, never in the reader's repository",
    ),
];

fn collapsed(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn no_skill_served_for_others_states_a_claim_only_a_local_install_can_make() {
    let mut found = Vec::new();
    for skill in SKILLS {
        let hosted = collapsed(&skill.served_body(Served::Hosted));
        for (claim, why) in LOCAL_ONLY {
            if hosted.contains(&collapsed(claim)) {
                found.push(format!("{}: {claim:?} — {why}", skill.name));
            }
        }
    }
    assert!(
        found.is_empty(),
        "{} local-only claim(s) in what an engine served for others serves; write the line once \
         so it is true everywhere, or mark a local and a hosted version \
         (src/skill_variants.rs):\n  {}",
        found.len(),
        found.join("\n  ")
    );
}

#[test]
fn an_unmarked_skill_is_served_byte_for_byte_both_ways() {
    for skill in SKILLS {
        let local = skill.served_body(Served::Local);
        let hosted = skill.served_body(Served::Hosted);
        if !reflow2_mcp::skill_variants::has_variants(skill.body) {
            assert_eq!(local, skill.body, "{}", skill.name);
            assert_eq!(hosted, skill.body, "{}", skill.name);
        } else {
            assert_ne!(
                local, hosted,
                "{}: a marked block that changes nothing",
                skill.name
            );
            for side in [&local, &hosted] {
                assert!(
                    !side.contains("<!-- reflow2:"),
                    "{}: a marker reached the reader",
                    skill.name
                );
            }
        }
        assert!(
            reflow2_mcp::skill_variants::problems(skill.body).is_empty(),
            "{}",
            skill.name
        );
    }
}

fn rules() -> Vec<(CallerRule, Served)> {
    vec![
        (CallerRule::Local, Served::Local),
        (
            CallerRule::TrustedGateway {
                name: "flo2.io".to_string(),
            },
            Served::Hosted,
        ),
        (
            CallerRule::BearerToken {
                issuer: "https://accounts.example.org".to_string(),
                mapping: "`who:{sub}`".to_string(),
            },
            Served::Hosted,
        ),
        (
            CallerRule::Undeclared {
                exposure: "`--registry-root` serves designs for other people".to_string(),
            },
            Served::Hosted,
        ),
    ]
}

#[tokio::test]
async fn get_skill_serves_the_version_for_how_the_engine_runs() {
    for (rule, served) in rules() {
        let label = format!("{rule:?}");
        assert_eq!(Served::for_rule(&rule), served, "{label}");
        let svc = ReflowService::in_memory()
            .expect("an in-memory design opens")
            .with_caller_rule(rule);
        for skill in SKILLS {
            let reply = svc
                .get_skill(Parameters(
                    serde_json::from_value(serde_json::json!({ "name": skill.name }))
                        .expect("request shape"),
                ))
                .await
                .expect("get_skill")
                .structured_content
                .expect("structured");
            assert_eq!(
                reply["body"].as_str(),
                Some(skill.served_body(served).as_ref()),
                "{label}: {} is served the {served:?} version",
                skill.name
            );
        }
    }
    // And the skill the finding was filed on is one that differs: help's
    // "what it physically is" has a version for each.
    let help = SKILLS.iter().find(|s| s.name == "help").expect("help");
    assert!(
        reflow2_mcp::skill_variants::has_variants(help.body),
        "help states what reflow2 physically is, which differs when it is hosted"
    );
}

#[test]
fn the_repositorys_own_skill_mirrors_hold_the_local_version() {
    // `.claude/skills` and `.grok/skills` are what this repository's own local
    // sessions load, so they hold what a local engine serves — rendered by
    // `tools/skill_variants.py --write-mirrors`, the Python twin of the
    // renderer, which this holds to the Rust one.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root")
        .to_path_buf();
    for mirror in [".claude/skills", ".grok/skills"] {
        for skill in SKILLS {
            let path = root.join(mirror).join(skill.name).join("SKILL.md");
            let on_disk = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(
                on_disk,
                skill.served_body(Served::Local),
                "{mirror}/{}: not the local version; run python3 tools/skill_variants.py \
                 --write-mirrors",
                skill.name
            );
        }
    }
}
