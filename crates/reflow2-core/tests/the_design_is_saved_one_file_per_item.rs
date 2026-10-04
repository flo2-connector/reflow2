//! The committed design saved one file per node and per edge, with per-item
//! lineage, and accepted checksums riding the change that accepted them.
//!
//! Anthony, 2026-10-03 ("yes, go with your recommendation for the 6"):
//! `dec:how-the-saved-design-is-laid-out-so-git-merges-it` (one file per node
//! and per edge), `dec:the-designs-lineage-is-kept-per-item` (each changed
//! item's `prev_item_hash` is its hash at the merge-base; the whole-design hash
//! is computed on read) and `dec:item-13-checksums-move-to-change-edges-and-main-converts-in-one-pr`
//! (an accepted checksum moves onto the change's CHANGED edge).
//!
//! These are the PURE halves — the layout's rules and the store's acceptance
//! record. Reading and writing a real directory, and git's merge over it, are
//! driven end to end on the real binary by `tools/test_item_layout_merges.py`.

use std::collections::BTreeMap;

use reflow2_core::graph::DesignGraph;
use reflow2_core::item_layout::{
    self, Anchor, Anchored, DesignStamp, OnDisk, ParsedItem, escape_id, node_rel_path,
};
use reflow2_core::nodes::{edge, node};
use reflow2_core::temporal::ChangeType;
use reflow2_core::{DriftDisposition, GraphExport, LinkArtifactOptions, Value};

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

fn link(g: &mut DesignGraph, checksum: &str) {
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
        checksum: Some(checksum.into()),
        content_ref: None,
        note_kind: None,
    })
    .expect("link");
}

/// Write `export` into an in-memory "directory" (`rel -> text`) the way the
/// server's writer does, and return the new directory.
fn write(
    export: &GraphExport,
    dir: &BTreeMap<String, String>,
    anchor: Option<&BTreeMap<String, String>>,
) -> BTreeMap<String, String> {
    let parsed: Vec<ParsedItem> = dir
        .iter()
        .map(|(rel, text)| item_layout::parse_item(rel, text.as_bytes()).expect("item parses"))
        .collect();
    let on_disk: BTreeMap<String, OnDisk> = parsed
        .iter()
        .map(|p| (p.rel_path.clone(), OnDisk::from(p)))
        .collect();
    let plan = match anchor {
        Some(committed) => {
            let mut lookup = |rels: &[String]| -> BTreeMap<String, Anchored> {
                rels.iter()
                    .filter_map(|r| {
                        let text = committed.get(r)?;
                        let p = item_layout::parse_item(r, text.as_bytes()).ok()?;
                        Some((
                            r.clone(),
                            Anchored {
                                content_hash: p.stated_hash.clone()?,
                                prev_item_hash: p.prev_item_hash.clone(),
                            },
                        ))
                    })
                    .collect()
            };
            item_layout::plan_write(export, &on_disk, Anchor::Committed(&mut lookup))
        }
        None => item_layout::plan_write(export, &on_disk, Anchor::Disk),
    };
    let mut out = dir.clone();
    for (rel, text) in plan.writes {
        out.insert(rel, text);
    }
    for rel in plan.deletes {
        out.remove(&rel);
    }
    out
}

fn read(dir: &BTreeMap<String, String>, graph_id: &str) -> item_layout::Assembled {
    let items = dir
        .iter()
        .map(|(rel, text)| item_layout::parse_item(rel, text.as_bytes()).expect("item parses"))
        .collect();
    item_layout::assemble(
        DesignStamp {
            graph_id: graph_id.into(),
            schema_version: 1,
            migrated_from: None,
        },
        items,
    )
    .expect("assembles")
}

fn prev_of(dir: &BTreeMap<String, String>, rel: &str) -> Option<String> {
    item_layout::parse_item(rel, dir[rel].as_bytes())
        .expect("parses")
        .prev_item_hash
}

fn hash_of(dir: &BTreeMap<String, String>, rel: &str) -> String {
    item_layout::parse_item(rel, dir[rel].as_bytes())
        .expect("parses")
        .computed_hash
}

#[test]
fn a_design_written_as_items_reads_back_as_the_same_design_with_the_same_hash() {
    let g = design();
    let export = g.export_graph().expect("export");
    let dir = write(&export, &BTreeMap::new(), None);
    assert_eq!(
        dir.len(),
        export.nodes.len() + export.edges.len(),
        "one file per node and one per edge"
    );
    let back = read(&dir, g.graph_id());
    assert_eq!(back.export.nodes, export.nodes);
    assert_eq!(back.export.edges, export.edges);
    assert_eq!(
        back.export.content_hash, export.content_hash,
        "the whole-design hash is computed on read and equals the single file's"
    );
    assert!(back.tampered.is_empty() && back.misplaced.is_empty());
}

#[test]
fn an_unchanged_design_rewrites_no_file_and_a_change_touches_only_its_item() {
    let mut g = design();
    let dir = write(&g.export_graph().unwrap(), &BTreeMap::new(), None);
    let again = write(&g.export_graph().unwrap(), &dir, None);
    assert_eq!(
        dir, again,
        "an unchanged design leaves every file byte for byte"
    );

    g.add_requirement("req:new", "New", "a new requirement")
        .expect("req");
    let after = write(&g.export_graph().unwrap(), &dir, None);
    let changed: Vec<&String> = after
        .keys()
        .filter(|k| dir.get(*k) != after.get(*k))
        .collect();
    assert_eq!(
        changed,
        vec![&node_rel_path("Requirement", "req:new")],
        "adding one node writes exactly one new file"
    );
}

#[test]
fn a_changed_item_names_its_hash_at_the_anchor_however_many_times_it_is_exported() {
    let mut g = design();
    let main = write(&g.export_graph().unwrap(), &BTreeMap::new(), None);
    let rel = node_rel_path("Requirement", "req:live");
    let at_main = hash_of(&main, &rel);

    // Three exports on a branch, each changing the same item again.
    let mut dir = main.clone();
    for (i, text) in ["first", "second", "third"].iter().enumerate() {
        g.upsert_node(
            node::REQUIREMENT,
            "req:live",
            reflow2_core::nodes::Props::new().set("statement", format!("scores update {text}")),
        )
        .expect("edit");
        dir = write(&g.export_graph().unwrap(), &dir, Some(&main));
        assert_eq!(
            prev_of(&dir, &rel),
            Some(at_main.clone()),
            "export {i}: the changed item chains from its MERGE-BASE version, not from the \
             intermediate before it — so a squash-merge lands one hop"
        );
    }
    // Changed back to what main holds: the file is main's file again exactly.
    g.upsert_node(
        node::REQUIREMENT,
        "req:live",
        reflow2_core::nodes::Props::new().set("statement", "scores update live"),
    )
    .expect("revert");
    dir = write(&g.export_graph().unwrap(), &dir, Some(&main));
    assert_eq!(dir[&rel], main[&rel], "an item changed back leaves no diff");
    // Every other item kept main's bytes.
    for (k, v) in &main {
        assert_eq!(
            dir.get(k),
            Some(v),
            "{k} was untouched and keeps main's file"
        );
    }
}

/// The plan for writing `export` over `dir`, anchored at `committed`, with
/// `recheck` — the item files that already differ from the anchor — re-checked.
fn plan_rechecking(
    export: &GraphExport,
    dir: &BTreeMap<String, String>,
    committed: &BTreeMap<String, String>,
    recheck: &std::collections::BTreeSet<String>,
) -> item_layout::WritePlan {
    let on_disk: BTreeMap<String, OnDisk> = dir
        .iter()
        .map(|(rel, text)| {
            let p = item_layout::parse_item(rel, text.as_bytes()).expect("item parses");
            (rel.clone(), OnDisk::from(&p))
        })
        .collect();
    let mut lookup = |rels: &[String]| -> BTreeMap<String, Anchored> {
        rels.iter()
            .filter_map(|r| {
                let p = item_layout::parse_item(r, committed.get(r)?.as_bytes()).ok()?;
                Some((
                    r.clone(),
                    Anchored {
                        content_hash: p.stated_hash.clone()?,
                        prev_item_hash: p.prev_item_hash.clone(),
                    },
                ))
            })
            .collect()
    };
    item_layout::plan_write_rechecking(export, &on_disk, Anchor::Committed(&mut lookup), recheck)
}

/// Measured 2026-10-04 on #672, with the real binary and real branches: an
/// item both sides changed, resolved by an export taken against an OLDER
/// anchor, names a predecessor main has already replaced — and because its
/// content no longer moves, a plain re-export left it that way while the gate
/// failed LINEAGE on it and told the author to re-export. The lineage is the
/// anchor's to state: an item that already differs from the anchor has its
/// `prev_item_hash` re-checked on every write.
#[test]
fn a_write_relinks_an_item_whose_lineage_names_the_wrong_predecessor() {
    let mut g = design();
    let old_base = write(&g.export_graph().unwrap(), &BTreeMap::new(), None);
    let rel = node_rel_path("Requirement", "req:live");
    let at_old_base = hash_of(&old_base, &rel);

    // Main moved the item since: the anchor (the merge-base after the merge)
    // holds main's version, chained from the old base.
    g.upsert_node(
        node::REQUIREMENT,
        "req:live",
        reflow2_core::nodes::Props::new().set("statement", "scores update as main says"),
    )
    .expect("main's edit");
    let new_base = write(&g.export_graph().unwrap(), &old_base, Some(&old_base));
    let at_new_base = hash_of(&new_base, &rel);
    assert_ne!(at_new_base, at_old_base);

    // The branch's resolution, exported against the OLD anchor: it names the
    // version main replaced.
    g.upsert_node(
        node::REQUIREMENT,
        "req:live",
        reflow2_core::nodes::Props::new().set("statement", "scores update, both sides merged"),
    )
    .expect("the resolution");
    let resolved = write(&g.export_graph().unwrap(), &new_base, Some(&old_base));
    assert_eq!(prev_of(&resolved, &rel), Some(at_old_base.clone()));

    // A plain write against the right anchor leaves it: its content did not move.
    let export = g.export_graph().unwrap();
    let plain = plan_rechecking(
        &export,
        &resolved,
        &new_base,
        &std::collections::BTreeSet::new(),
    );
    assert!(
        plain.writes.is_empty(),
        "nothing re-checked, nothing rewritten"
    );

    // Re-checked, it names its hash at the anchor, and nothing else is touched.
    let recheck: std::collections::BTreeSet<String> = [rel.clone()].into_iter().collect();
    let plan = plan_rechecking(&export, &resolved, &new_base, &recheck);
    assert_eq!(plan.relinked, 1, "{plan:?}");
    assert_eq!(plan.changed_items, 0, "the content did not change");
    assert_eq!(plan.writes.len(), 1);
    let (written_rel, text) = &plan.writes[0];
    assert_eq!(written_rel, &rel);
    let fixed = item_layout::parse_item(&rel, text.as_bytes()).expect("parses");
    assert_eq!(fixed.prev_item_hash, Some(at_new_base));
    assert!(fixed.intact());

    // Written, it is stable: the next re-checked write changes nothing.
    let mut dir = resolved.clone();
    dir.insert(rel.clone(), text.clone());
    let again = plan_rechecking(&export, &dir, &new_base, &recheck);
    assert!(again.writes.is_empty() && again.relinked == 0, "{again:?}");

    // An item back to EXACTLY the anchor's content gets the anchor's file
    // back, byte for byte, whatever lineage it carried.
    let at_anchor = write(&export, &BTreeMap::new(), None);
    let mut forged = at_anchor.clone();
    let mut data: serde_json::Value = serde_json::from_str(&forged[&rel]).unwrap();
    data["prev_item_hash"] = serde_json::json!(format!("sha256:{}", "0".repeat(64)));
    forged.insert(
        rel.clone(),
        format!("{}\n", serde_json::to_string_pretty(&data).unwrap()),
    );
    let restore = plan_rechecking(&export, &forged, &at_anchor, &recheck);
    assert_eq!(restore.relinked, 1);
    assert_eq!(restore.writes[0].1, at_anchor[&rel]);
}

#[test]
fn an_item_new_on_the_branch_carries_no_prev_and_outside_git_an_item_chains_from_disk() {
    let mut g = design();
    let main = write(&g.export_graph().unwrap(), &BTreeMap::new(), None);
    g.add_requirement("req:new", "New", "a new requirement")
        .expect("req");
    let dir = write(&g.export_graph().unwrap(), &main, Some(&main));
    assert_eq!(
        prev_of(&dir, &node_rel_path("Requirement", "req:new")),
        None
    );

    // No committed anchor: chain from the version on disk.
    let rel = node_rel_path("Requirement", "req:new");
    let before = hash_of(&dir, &rel);
    g.upsert_node(
        node::REQUIREMENT,
        "req:new",
        reflow2_core::nodes::Props::new().set("statement", "changed"),
    )
    .expect("edit");
    let after = write(&g.export_graph().unwrap(), &dir, None);
    assert_eq!(prev_of(&after, &rel), Some(before));
}

#[test]
fn a_deleted_item_loses_its_file() {
    let mut g = design();
    g.add_requirement("req:gone", "Gone", "to be deleted")
        .expect("req");
    let dir = write(&g.export_graph().unwrap(), &BTreeMap::new(), None);
    let rel = node_rel_path("Requirement", "req:gone");
    assert!(dir.contains_key(&rel));
    g.delete_node(node::REQUIREMENT, "req:gone")
        .expect("delete");
    let after = write(&g.export_graph().unwrap(), &dir, None);
    assert!(!after.contains_key(&rel));
}

#[test]
fn a_tampered_or_misplaced_item_is_named_and_two_copies_of_one_item_are_refused() {
    let g = design();
    let mut dir = write(&g.export_graph().unwrap(), &BTreeMap::new(), None);
    let rel = node_rel_path("Requirement", "req:live");
    let edited = dir[&rel].replace("scores update live", "scores update by hand");
    dir.insert(rel.clone(), edited);
    let read_back = read(&dir, g.graph_id());
    assert_eq!(read_back.tampered, vec![rel.clone()]);
    assert!(read_back.integrity_note().is_some());

    let mut copied = dir.clone();
    copied.insert("nodes/Requirement/copy.json".into(), dir[&rel].clone());
    let items = copied
        .iter()
        .map(|(r, t)| item_layout::parse_item(r, t.as_bytes()).unwrap())
        .collect();
    let err = item_layout::assemble(
        DesignStamp {
            graph_id: "x".into(),
            schema_version: 1,
            migrated_from: None,
        },
        items,
    )
    .expect_err("two files holding one node must not be silently reduced to one");
    assert!(err.contains("held by two files"), "{err}");
}

#[test]
fn an_item_file_with_a_field_the_reader_does_not_know_is_refused_not_dropped() {
    let text = r#"{"node_type":"Requirement","node_id":"req:x","properties":{},"surprise":1}"#;
    let err = item_layout::parse_item("nodes/Requirement/req%3Ax.json", text.as_bytes())
        .expect_err("unknown field");
    assert!(err.contains("surprise"), "{err}");
}

#[test]
fn an_id_becomes_a_file_name_every_filesystem_takes_and_a_long_one_stays_unique() {
    assert_eq!(escape_id("req:a-b_c.d"), "req%3Aa-b_c.d");
    assert_eq!(
        escape_id("BL-1"),
        "%42%4C-1",
        "upper case is escaped, so a case-insensitive disk cannot fold two ids"
    );
    let long_a = format!("snap:{}", "a".repeat(300));
    let long_b = format!("snap:{}b", "a".repeat(299));
    let (ea, eb) = (escape_id(&long_a), escape_id(&long_b));
    assert!(ea.len() <= 150 && eb.len() <= 150);
    assert_ne!(
        ea, eb,
        "two long ids sharing a prefix still get different names"
    );
    assert_eq!(ea, escape_id(&long_a), "and the name is stable");
}

// ---- accepted checksums ride the accepting change's CHANGED edge ----------

fn acceptance(g: &DesignGraph, event: &str) -> BTreeMap<String, Value> {
    g.outgoing(event, Some(edge::CHANGED))
        .expect("edges")
        .into_iter()
        .find(|e| e.to_id == "art:score")
        .expect("the accept drew a CHANGED edge to the artifact")
        .properties
        .into_iter()
        .collect()
}

#[test]
fn an_accept_writes_the_checksum_on_its_change_and_numbers_it() {
    let mut g = design();
    link(&mut g, "sha256:aaaa");
    let (_, first) = g
        .set_artifact_checksum(
            "art:score",
            "sha256:bbbb",
            DriftDisposition::DesignHolds {
                change_type: ChangeType::Refactor,
            },
            None,
            None,
        )
        .expect("accept");
    let props = acceptance(&g, &first);
    assert_eq!(props["checksum_after"].as_str(), Some("sha256:bbbb"));
    assert_eq!(props["accepted_seq"].as_i64(), Some(1));

    let (_, second) = g
        .set_artifact_checksum(
            "art:score",
            "sha256:cccc",
            DriftDisposition::DesignHolds {
                change_type: ChangeType::Refactor,
            },
            None,
            None,
        )
        .expect("accept again");
    assert_eq!(acceptance(&g, &second)["accepted_seq"].as_i64(), Some(2));
    assert_eq!(
        g.current_acceptance("art:score")
            .unwrap()
            .expect("current")
            .from_id,
        second
    );

    // Re-stating the current acceptance is idempotent: same number.
    g.set_artifact_checksum(
        "art:score",
        "sha256:cccc",
        DriftDisposition::DesignHolds {
            change_type: ChangeType::Refactor,
        },
        None,
        None,
    )
    .expect("re-accept");
    assert_eq!(acceptance(&g, &second)["accepted_seq"].as_i64(), Some(2));

    // Going back to an earlier content re-uses that change and makes it current.
    g.set_artifact_checksum(
        "art:score",
        "sha256:bbbb",
        DriftDisposition::DesignHolds {
            change_type: ChangeType::Refactor,
        },
        None,
        None,
    )
    .expect("revert accepted");
    assert_eq!(acceptance(&g, &first)["accepted_seq"].as_i64(), Some(3));
    let node = g.get_node(node::ARTIFACT, "art:score").unwrap().unwrap();
    assert_eq!(node.properties["checksum"].as_str(), Some("sha256:bbbb"));
}

#[test]
fn the_record_carries_an_accepted_checksum_once_and_the_import_derives_the_node_copy() {
    let mut g = design();
    link(&mut g, "sha256:aaaa");
    g.set_artifact_checksum(
        "art:score",
        "sha256:bbbb",
        DriftDisposition::DesignHolds {
            change_type: ChangeType::Refactor,
        },
        None,
        None,
    )
    .expect("accept");
    g.set_checksum_basis("art:score", "measured")
        .expect("basis");
    let export = g.export_graph().expect("export");
    let art = export
        .nodes
        .iter()
        .find(|n| n.node_id == "art:score")
        .unwrap();
    assert!(
        !art.properties.contains_key("checksum") && !art.properties.contains_key("checksum_basis"),
        "the node's copy equals the accepting edge's, so the record holds it once: {:?}",
        art.properties
    );

    let mut restored = DesignGraph::open_in_memory().expect("open");
    let report = restored.import_graph(&export).expect("import");
    assert!(
        !report.materialized.contains_key("Artifact.checksum"),
        "a derived checksum is the document's own statement, not a default: {:?}",
        report.materialized
    );
    let node = restored
        .get_node(node::ARTIFACT, "art:score")
        .unwrap()
        .unwrap();
    assert_eq!(node.properties["checksum"].as_str(), Some("sha256:bbbb"));
    assert_eq!(node.properties["checksum_basis"].as_str(), Some("measured"));
    let again = restored.export_graph().expect("re-export");
    assert_eq!(
        again.content_hash, export.content_hash,
        "the round trip is byte-identical"
    );
}

/// compare_designs reads a record written BEFORE the checksum moved (stated on
/// the Artifact) and one written AFTER (on the accepting change) as the same
/// design: measured 2026-10-04, reflow2's own conversion read as 541 artifacts
/// "changed" though no checksum moved. The edge that now carries the checksum
/// is still a difference, and a checksum that really moved still shows.
#[test]
fn compare_reads_a_checksum_on_the_node_and_on_its_change_as_one_design() {
    let mut g = design();
    link(&mut g, "sha256:aaaa");
    g.set_artifact_checksum(
        "art:score",
        "sha256:bbbb",
        DriftDisposition::DesignHolds {
            change_type: ChangeType::Refactor,
        },
        None,
        None,
    )
    .expect("accept");
    let after = g.export_graph().expect("export");
    // The same design as a build before the move wrote it: the checksum on
    // the node, the change edge without it.
    let mut before = after.clone();
    for n in before.nodes.iter_mut().filter(|n| n.node_id == "art:score") {
        n.properties
            .insert("checksum".into(), Value::String("sha256:bbbb".into()));
    }
    for e in before
        .edges
        .iter_mut()
        .filter(|e| e.edge_type == edge::CHANGED)
    {
        for k in ["checksum_after", "checksum_basis", "accepted_seq"] {
            e.properties.remove(k);
        }
    }
    let diff = reflow2_core::compare_designs(&before, &after, "before", "after");
    assert_eq!(
        diff.summary.design_changed + diff.summary.supporting_changed,
        0,
        "no node changed: {:?}",
        diff.summary
    );
    assert_eq!(
        diff.summary.edges_changed, 1,
        "the CHANGED edge now carrying the checksum is a real record change"
    );

    // A checksum that really moved is still a change.
    for n in before.nodes.iter_mut().filter(|n| n.node_id == "art:score") {
        n.properties
            .insert("checksum".into(), Value::String("sha256:cccc".into()));
    }
    let diff = reflow2_core::compare_designs(&before, &after, "before", "after");
    assert_eq!(
        diff.summary.design_changed + diff.summary.supporting_changed,
        1,
        "{:?}",
        diff.summary
    );
}

#[test]
fn a_node_checksum_that_disagrees_with_its_acceptances_stays_on_the_record() {
    let mut g = design();
    link(&mut g, "sha256:aaaa");
    g.set_artifact_checksum(
        "art:score",
        "sha256:bbbb",
        DriftDisposition::DesignHolds {
            change_type: ChangeType::Refactor,
        },
        None,
        None,
    )
    .expect("accept");
    // A direct write the acceptances do not say.
    g.upsert_node(
        node::ARTIFACT,
        "art:score",
        reflow2_core::nodes::Props::new().set("checksum", "sha256:dddd"),
    )
    .expect("direct write");
    let export = g.export_graph().expect("export");
    let art = export
        .nodes
        .iter()
        .find(|n| n.node_id == "art:score")
        .unwrap();
    assert_eq!(
        art.properties["checksum"].as_str(),
        Some("sha256:dddd"),
        "eliding a value the edges do not derive would lose it"
    );
}

#[test]
fn two_branches_accepting_one_file_each_keep_their_own_edge_and_the_pick_is_stable() {
    // Main: one registered artifact. Branch A and branch B each accept a new
    // checksum for it. Their records differ only in THEIR OWN acceptance edge
    // (and the derived node checksum the record elides), so the union is the
    // merge — no shared value was rewritten.
    let mut a = design();
    link(&mut a, "sha256:aaaa");
    let base = a.export_graph().expect("base");
    let mut b = DesignGraph::open_in_memory().unwrap();
    b.import_graph(&base).unwrap();
    for (g, sum) in [(&mut a, "sha256:1111"), (&mut b, "sha256:2222")] {
        g.set_artifact_checksum(
            "art:score",
            sum,
            DriftDisposition::DesignHolds {
                change_type: ChangeType::Refactor,
            },
            None,
            None,
        )
        .expect("accept");
    }
    let (ea, eb) = (a.export_graph().unwrap(), b.export_graph().unwrap());
    let base_art = base
        .nodes
        .iter()
        .find(|n| n.node_id == "art:score")
        .unwrap();
    let a_art = ea.nodes.iter().find(|n| n.node_id == "art:score").unwrap();
    let b_art = eb.nodes.iter().find(|n| n.node_id == "art:score").unwrap();
    assert_eq!(
        a_art, b_art,
        "both branches leave the Artifact node identical, so git merges it without conflict"
    );
    assert_ne!(a_art, base_art);

    // The merged document: base + both sides' edges.
    let mut merged = ea.clone();
    for n in &eb.nodes {
        if !merged.nodes.iter().any(|m| m.node_id == n.node_id) {
            merged.nodes.push(n.clone());
        }
    }
    for e in &eb.edges {
        if !merged.edges.contains(e) {
            merged.edges.push(e.clone());
        }
    }
    let mut store = DesignGraph::open_in_memory().unwrap();
    store.import_graph(&merged).expect("the union imports");
    let node = store
        .get_node(node::ARTIFACT, "art:score")
        .unwrap()
        .unwrap();
    let derived = node.properties["checksum"].as_str().unwrap().to_string();
    assert!(
        derived == "sha256:1111" || derived == "sha256:2222",
        "{derived}"
    );
    let mut again = DesignGraph::open_in_memory().unwrap();
    again.import_graph(&merged).unwrap();
    assert_eq!(
        again
            .get_node(node::ARTIFACT, "art:score")
            .unwrap()
            .unwrap()
            .properties["checksum"]
            .as_str(),
        Some(derived.as_str()),
        "the same document always derives the same current acceptance"
    );
}
