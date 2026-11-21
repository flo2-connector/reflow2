//! An export that leaves an Artifact's checksum off its record says, in the
//! document, where the checksum is.
//!
//! Since 2026-10-03 an accepted checksum rides the accepting change's CHANGED
//! edge, and the export leaves the Artifact's own copy off while the two are
//! equal (`cap:an-accepted-checksum-rides-the-change-that-accepted-it`). The
//! import derives it back, so reflow2 loses nothing. A person or a script
//! reading the FILE did not know that: twice, on 2026-10-14 and 2026-11-05,
//! flo2's committed export was read with a plain `json.load`, the missing
//! `checksum` field was read as a dropped checksum, and the second reading
//! concluded the design gate could not see those files drift. It could
//! (`fact:root-cause-a-design-copys-checksums-read-as-dropped-because-the-reader-read-the-record-not-the-accepting-change-2026-11-05`).
//!
//! These tests read the export as that reader did, through its JSON, because
//! the reader the note is for never sees a Rust struct.

use reflow2_core::graph::DesignGraph;
use reflow2_core::nodes::node;
use reflow2_core::temporal::ChangeType;
use reflow2_core::{DriftDisposition, GraphExport, LinkArtifactOptions};
use serde_json::Value as Json;

fn design() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_project("proj:1", "Scoreboard").expect("project");
    g.add_requirement("req:live", "Live scores", "scores update live")
        .expect("req");
    g.add_capability("cap:score", "Scoring", "tracks the score", None)
        .expect("cap");
    g.satisfies("cap:score", "req:live").expect("satisfies");
    g
}

fn link(g: &mut DesignGraph, checksum: Option<&str>) {
    g.link_artifact(LinkArtifactOptions {
        artifact_id: "art:score".into(),
        name: Some("Score.cs".into()),
        description: None,
        location: Some("src/Score.cs".into()),
        artifact_type: Some("code".into()),
        target_type: node::CAPABILITY.into(),
        target_id: "cap:score".into(),
        completeness: None,
        conformance: None,
        provenance: None,
        fragment_id: None,
        checksum: checksum.map(str::to_string),
        content_ref: None,
        note_kind: None,
    })
    .expect("link");
}

fn accept(g: &mut DesignGraph, checksum: &str) {
    g.set_artifact_checksum(
        "art:score",
        checksum,
        DriftDisposition::DesignHolds {
            change_type: ChangeType::Refactor,
        },
        None,
        None,
    )
    .expect("accept");
}

/// The document as a script that opens the file sees it.
fn as_read_from_the_file(export: &GraphExport) -> Json {
    serde_json::to_value(export).expect("an export serializes")
}

fn record_of<'a>(doc: &'a Json, id: &str) -> &'a serde_json::Map<String, Json> {
    doc["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|n| n["node_id"] == id)
        .and_then(|n| n["properties"].as_object())
        .expect("the record is in the document")
}

#[test]
fn an_export_that_leaves_a_checksum_off_its_record_says_where_it_is() {
    let mut g = design();
    link(&mut g, Some("sha256:aaaa"));
    accept(&mut g, "sha256:bbbb");
    let doc = as_read_from_the_file(&g.export_graph().expect("export"));

    // The precondition: the record really does not carry it.
    assert!(
        !record_of(&doc, "art:score").contains_key("checksum"),
        "the record holds the checksum once, on its change: {:?}",
        record_of(&doc, "art:score")
    );

    let note = doc["reading_notes"]["artifact_checksums"]
        .as_str()
        .unwrap_or_else(|| {
            panic!(
                "a reader of this file is told nowhere that the checksum it cannot find \
                 on art:score is on the change that accepted it; top-level keys: {:?}",
                doc.as_object().map(|o| o.keys().collect::<Vec<_>>())
            )
        });
    // Everything a reader needs to find it without reflow2: which edge, which
    // fields, which edge wins, and the reader that already applies the rule.
    for needed in [
        "CHANGED",
        "checksum_after",
        "checksum_basis",
        "accepted_seq",
        "import",
        "current_acceptances",
    ] {
        assert!(
            note.contains(needed),
            "the note does not say `{needed}`: {note}"
        );
    }
}

#[test]
fn an_export_that_leaves_no_checksum_off_carries_no_note() {
    // A checksum stated on the record and never accepted on a change stays on
    // the record, so there is nothing to explain.
    let mut g = design();
    link(&mut g, Some("sha256:aaaa"));
    let doc = as_read_from_the_file(&g.export_graph().expect("export"));
    assert_eq!(
        record_of(&doc, "art:score")
            .get("checksum")
            .and_then(Json::as_str),
        Some("sha256:aaaa"),
        "the precondition: the checksum is still on the record"
    );
    assert!(
        doc.get("reading_notes").is_none(),
        "no checksum was left off a record, so the document carries no note: {:?}",
        doc.get("reading_notes")
    );

    // Nor does a design with no files at all.
    let doc = as_read_from_the_file(&design().export_graph().expect("export"));
    assert!(doc.get("reading_notes").is_none());
}

#[test]
fn the_note_is_outside_the_design_and_survives_the_round_trip() {
    let mut g = design();
    link(&mut g, Some("sha256:aaaa"));
    accept(&mut g, "sha256:bbbb");
    let export = g.export_graph().expect("export");
    let doc = as_read_from_the_file(&export);
    assert!(doc.get("reading_notes").is_some(), "the precondition");

    // Not part of the design's identity: the same content with the note taken
    // out still verifies against the hash the export carries.
    let mut without = doc.clone();
    without
        .as_object_mut()
        .expect("an object")
        .remove("reading_notes");
    let without: GraphExport = serde_json::from_value(without).expect("still a document");
    assert_eq!(
        without.verify_content_hash(),
        Some(true),
        "the note changed the content hash"
    );

    // A document carrying the note imports, and the import still puts the
    // checksum back on the record.
    let mut restored = DesignGraph::open_in_memory().expect("open");
    let read_back: GraphExport = serde_json::from_value(doc.clone()).expect("reads back");
    restored.import_graph(&read_back).expect("import");
    let node = restored
        .get_node(node::ARTIFACT, "art:score")
        .unwrap()
        .unwrap();
    assert_eq!(node.properties["checksum"].as_str(), Some("sha256:bbbb"));

    // And the re-export is the same document, note included, so two exports
    // of one design stay byte-identical.
    let again = as_read_from_the_file(&restored.export_graph().expect("re-export"));
    assert_eq!(
        serde_json::to_string(&again).unwrap(),
        serde_json::to_string(&doc).unwrap(),
        "the round trip changed the document"
    );
}
