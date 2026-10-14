//! EVERY DEPRECATION IS ANNOUNCED IN THE CHANGELOG, with what to do.
//!
//! The retirement rule's deprecating release "says it is going … and the
//! changelog carries a 'What to do' line"
//! (`req:the-mcp-server-keeps-working-as-is-and-door-improvements-are-additive-or-opt-in`,
//! amended 2026-10-13). So every entry in `deprecation::REGISTRY` must be named
//! under a `### Deprecated` heading, in an entry that has a "What to do" line:
//!
//! - in a `changelog.d/` fragment while its release is still to be cut (the
//!   fragment convention, `changelog.d/README.md`), and
//! - in CHANGELOG.md's section for the release it names once the cut has
//!   assembled the fragments and deleted them.
//!
//! An entry "names" the item when it carries each of the item's names in
//! backticks — the tool, the argument's path, the node type, the property —
//! so the release notes say exactly what is going.
//!
//! The first test reads the real files and runs in CI with the workspace
//! suite. The rest hold the check to each way it can fail, with test-only
//! entries and fragments.

use reflow2_mcp::deprecation::{Deprecation, REGISTRY, changelog_problems};

const DEC: &str = "dec:test-only-a-retirement";

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every fragment in changelog.d/, as (file name, text); the README is not one.
fn fragments() -> Vec<(String, String)> {
    let dir = root().join("changelog.d");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .expect("changelog.d/")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|x| x == "md")
                && p.file_name().is_some_and(|n| n != "README.md")
                && !p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with('.'))
        })
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().to_string(),
                std::fs::read_to_string(&p).expect("a fragment reads"),
            )
        })
        .collect();
    out.sort();
    out
}

/// THE CHECK, on the real files: every registry entry is announced.
#[test]
fn every_registry_entry_is_announced_with_what_to_do() {
    let changelog = std::fs::read_to_string(root().join("CHANGELOG.md")).expect("CHANGELOG.md");
    let problems = changelog_problems(
        REGISTRY,
        &fragments(),
        &changelog,
        env!("CARGO_PKG_VERSION"),
    );
    assert!(
        problems.is_empty(),
        "a deprecation the changelog does not announce:\n  {}\n\nWrite a changelog.d \
         fragment with a `### Deprecated` entry that names it in backticks and says \
         **What to do:** (changelog.d/README.md).",
        problems.join("\n  ")
    );
}

fn anchor(version: &'static str) -> Deprecation {
    Deprecation::node_type(
        "Anchor",
        version,
        "Record a Snapshot, which a cut already takes.",
        DEC,
    )
}

const GOOD: &str = "### Deprecated\n\n- **The `Anchor` node type is going.** Nothing creates \
                    it. **What to do:** record a Snapshot.\n";

/// An entry with no fragment fails, naming it.
#[test]
fn an_entry_without_its_fragment_fails() {
    let p = changelog_problems(
        &[anchor("0.99.0")],
        &[],
        "# Changelog\n\n## [Unreleased]\n",
        "0.84.0",
    );
    assert_eq!(p.len(), 1, "{p:?}");
    assert!(
        p[0].contains("`Anchor`"),
        "the problem names the item: {p:?}"
    );
}

/// The fragment that announces it passes.
#[test]
fn a_fragment_under_deprecated_with_what_to_do_passes() {
    let p = changelog_problems(
        &[anchor("0.99.0")],
        &[("retire-anchor.md".into(), GOOD.into())],
        "# Changelog\n\n## [Unreleased]\n",
        "0.84.0",
    );
    assert!(p.is_empty(), "{p:?}");
}

/// Naming it is not enough: under another heading, without a What to do
/// line, or naming only part of the item, it fails.
#[test]
fn a_fragment_that_does_not_announce_it_fails() {
    let cases = [
        GOOD.replace("### Deprecated", "### Changed"),
        GOOD.replace("**What to do:** record a Snapshot.", "Record a Snapshot."),
        GOOD.replace("`Anchor`", "Anchor"),
    ];
    for text in cases {
        let p = changelog_problems(
            &[anchor("0.99.0")],
            &[("retire-anchor.md".into(), text.clone())],
            "# Changelog\n\n## [Unreleased]\n",
            "0.84.0",
        );
        assert_eq!(p.len(), 1, "should fail on:\n{text}\n{p:?}");
    }
    let argument = Deprecation::argument(
        "scan_nodes",
        "brief",
        "0.99.0",
        "Pass `limit` instead.",
        DEC,
    );
    let half = "### Deprecated\n\n- **`scan_nodes` changes.** **What to do:** pass `limit`.\n";
    assert_eq!(
        changelog_problems(
            std::slice::from_ref(&argument),
            &[("x.md".into(), half.into())],
            "## [Unreleased]\n",
            "0.84.0"
        )
        .len(),
        1,
        "an argument is named by its tool AND its path"
    );
    let whole = "### Deprecated\n\n- **`scan_nodes`'s `brief` is going.** **What to do:** pass \
                 `limit`.\n";
    assert!(
        changelog_problems(
            &[argument],
            &[("x.md".into(), whole.into())],
            "## [Unreleased]\n",
            "0.84.0"
        )
        .is_empty()
    );
}

/// After the cut the fragment is gone and the release's section carries it:
/// that passes. A release already cut that never announced it fails, and so
/// does an entry the cut put under a different release than it names.
#[test]
fn after_the_cut_the_release_section_is_the_announcement() {
    let cut = format!(
        "# Changelog\n\n## [Unreleased]\n\n## [0.99.0] — 2026-11-01\n\n{GOOD}\n## [0.98.0] — \
         2026-10-20\n\n### Added\n\n- x\n"
    );
    assert!(changelog_problems(&[anchor("0.99.0")], &[], &cut, "0.99.0").is_empty());
    assert!(
        changelog_problems(&[anchor("0.99.0")], &[], &cut, "0.99.1").is_empty(),
        "still announced after later releases"
    );
    let p = changelog_problems(&[anchor("0.98.0")], &[], &cut, "0.99.0");
    assert_eq!(p.len(), 1, "0.98.0's section does not announce it: {p:?}");
    let p = changelog_problems(
        &[anchor("0.98.0")],
        &[("retire-anchor.md".into(), GOOD.into())],
        "# Changelog\n\n## [Unreleased]\n\n## [0.98.0] — 2026-10-20\n\n### Added\n\n- x\n",
        "0.99.0",
    );
    assert_eq!(
        p.len(),
        1,
        "a fragment cannot announce a release already cut: {p:?}"
    );
    let p = changelog_problems(
        &[Deprecation::node_type("Anchor", "soon", "Do X.", DEC)],
        &[("retire-anchor.md".into(), GOOD.into())],
        "## [Unreleased]\n",
        "0.84.0",
    );
    assert_eq!(p.len(), 1, "a version that is not one fails: {p:?}");
}
