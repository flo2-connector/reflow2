//! Every field a person or agent writes prose into is one search reads — or it
//! is listed here as deliberately left out, with the reason.
//!
//! FOUND 2026-10-12 (fact:search-design-did-not-find-a-change-event-by-the-error-its-rationale-quotes-2026-10-12):
//! a search for the raw error a recorded fix quoted came back without that fix,
//! because a ChangeEvent's `rationale` was never marked `fulltext`. The root
//! cause was the CLASS, not the one field: whether search reads a property is a
//! hand-set flag, and nothing checked that the prose fields were covered. Six
//! were not (ChangeEvent.rationale and .stands_in_for, Requirement.rationale,
//! Constraint.rationale, DimensionAssessment.description and .evidence), all
//! from the first schema commit, and no decision was found that chose it.
//!
//! So this test makes the flag a decision. Every `string` property on every
//! node type is either searched or named below with the kind of value it holds
//! — a date, a reference, a hash, a location, a word from a small set, where a
//! record came from, or a payload written for a program. A new string property
//! fails here until somebody says which it is.

use reflow2_core::foundation::core::PropertyType;

/// Why a string property is NOT searched. Each kind is a value nobody reads as
/// what a record SAYS: searching it would match every record of a sort (a
/// date, a status word) or match ids rather than meaning.
#[derive(Debug, Clone, Copy)]
enum NotProse {
    /// A moment: when something was made, measured, asked or seen.
    When,
    /// Names another record or key: an id, a node type, a handle, a batch.
    Which,
    /// A digest or signature, compared rather than read.
    Hash,
    /// A path, address or place on disk.
    Where,
    /// One word from a small set (a level, a unit, a category, a version).
    Category,
    /// Where the record came from. Attribution is read by the provenance
    /// reads, and searching it would match every record from one session.
    Provenance,
    /// A structured value written for a program (JSON, a list of ids).
    Payload,
}
use NotProse::*;

const NOT_PROSE: &[(&str, NotProse)] = &[
    ("Artifact.checksum", Hash),
    ("Artifact.checksum_basis", Category),
    ("Artifact.created_at", When),
    ("Artifact.last_confirmed_at", When),
    ("Artifact.location", Where),
    ("Artifact.signature", Hash),
    ("Capability.capability_type", Category),
    ("Capability.inputs", Payload),
    ("Capability.outputs", Payload),
    ("ChangeEvent.commits", Hash),
    ("ChangeEvent.detected_at", When),
    ("Component.level", Category),
    ("Component.location", Where),
    ("Constraint.limit_measured_at", When),
    ("Constraint.limit_source", Provenance),
    ("Constraint.quantity", Category),
    ("Constraint.unit", Category),
    ("Contributor.handle", Which),
    ("DesignEpoch.checksum", Hash),
    ("DesignEpoch.created_at", When),
    ("DesignRule.category", Category),
    ("DimensionAssessment.assessed_at", When),
    ("DimensionAssessment.target_id", Which),
    ("DimensionAssessment.target_type", Which),
    ("DimensionObservation.observed_at", When),
    ("DimensionObservation.source_fragment_id", Which),
    ("DimensionObservation.target_id", Which),
    ("DimensionObservation.target_type", Which),
    ("DriftEvent.detected_at", When),
    ("Environment.location", Where),
    ("EnvironmentRule.authority", Category),
    ("EnvironmentRule.checker", Category),
    ("EnvironmentRule.jurisdiction", Category),
    ("EnvironmentRule.reference", Provenance),
    ("Fragment.content_ref", Which),
    ("Fragment.created_at", When),
    ("Fragment.created_by", Which),
    ("Inference.cases_exhaustive", Which),
    ("Inference.confirmed_at", When),
    ("Inference.confirmed_by", Which),
    ("Project.created_at", When),
    ("Project.domain", Category),
    ("Project.mirror_content_hash", Hash),
    ("Project.mirror_nodes", Payload),
    ("Project.mirror_of", Which),
    ("Project.mirrored_at", When),
    ("Question.answered_at", When),
    ("Question.answered_by", Which),
    ("Question.answered_via", Category),
    ("Question.asked_at", When),
    ("Question.asked_of", Which),
    ("Question.batch", Which),
    ("Question.gap_id", Which),
    ("Question.prompt_id", Which),
    ("ReadinessAssessment.assessed_at", When),
    ("ReadinessAssessment.target_id", Which),
    ("ReadinessAssessment.target_type", Which),
    ("Release.created_at", When),
    ("Release.version", Category),
    ("Requirement.source", Provenance),
    ("Resource.components", Payload),
    ("Resource.declared_in", Where),
    ("Resource.features", Payload),
    ("Resource.provider", Category),
    ("Resource.resource_type", Category),
    ("Resource.version", Category),
    ("Snapshot.captured_at", When),
    ("Snapshot.edges", Payload),
    ("Snapshot.state", Payload),
    ("Snapshot.target_id", Which),
    ("Snapshot.target_type", Which),
    ("TemporalFact.fact_type", Category),
    ("TemporalFact.source", Provenance),
    ("TemporalFact.subject_id", Which),
    ("TemporalFact.valid_from", When),
    ("TemporalFact.valid_to", When),
    ("TemporalFact.value", Payload),
    ("Verification.criteria_at_data", Payload),
    ("Verification.hypothesis_test", Payload),
    ("Verification.last_failed_run_at", When),
    ("Verification.last_reconciled_at", When),
    ("Verification.last_run_at", When),
    ("Verification.location", Where),
];

/// Every `Type.property` the schema declares as a string, with whether search
/// reads it. List-of-string properties are left out: the index takes a node's
/// string values, and a list is a set of keys rather than prose.
fn string_properties() -> Vec<(String, bool)> {
    let schema = reflow2_core::schema::load_schema().expect("the embedded schema loads");
    let mut out: Vec<(String, bool)> = schema
        .node_types
        .iter()
        .flat_map(|(node_type, def)| {
            def.properties
                .iter()
                .filter(|(_, p)| p.prop_type == PropertyType::String)
                .map(move |(name, p)| (format!("{node_type}.{name}"), p.fulltext))
        })
        .collect();
    out.sort();
    out
}

#[test]
fn every_string_property_is_searched_or_named_as_not_prose() {
    let unclassified: Vec<String> = string_properties()
        .into_iter()
        .filter(|(prop, searched)| !searched && !NOT_PROSE.iter().any(|(p, _)| p == prop))
        .map(|(prop, _)| prop)
        .collect();
    assert!(
        unclassified.is_empty(),
        "these string properties are neither searched nor named as not prose: {unclassified:?}. \
         If people or agents write sentences into one, mark it `fulltext: true` in schema/*.yaml; \
         if it holds a date, a reference, a hash, a location, a word from a small set, where the \
         record came from, or a payload for a program, add it to NOT_PROSE with that reason."
    );
}

#[test]
fn every_name_on_the_not_prose_list_is_a_string_property_search_skips() {
    let props = string_properties();
    for (listed, why) in NOT_PROSE {
        match props.iter().find(|(p, _)| p == listed) {
            None => panic!(
                "{listed} ({why:?}) is on the not-prose list but the schema declares no such \
                 string property; take it off the list"
            ),
            Some((_, true)) => panic!(
                "{listed} ({why:?}) is on the not-prose list but search reads it; take it off \
                 the list or take its `fulltext` flag away"
            ),
            Some((_, false)) => {}
        }
    }
}

/// The observation that started it, in a fresh design: a word that appears
/// only in a change's rationale is found.
#[cfg(feature = "fulltext")]
#[test]
fn a_word_only_a_changes_rationale_holds_is_found() {
    use reflow2_core::DesignGraph;
    use reflow2_core::props;

    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_project("proj:1", "Teardown").expect("project");
    g.create_node(
        "ChangeEvent",
        "chg:teardowns-wait",
        props! {
            "name" => "Teardowns wait for the engine to exit",
            "change_type" => "defect_fix",
            "summary" => "The tests that stop an engine wait for it to exit before removing its store",
            "rationale" => "CI failed with ENOTEMPTY zqrationaletok while removing the store",
        },
    )
    .expect("change event");
    let found = g.search_design("zqrationaletok", None, 10).expect("search");
    assert_eq!(
        found.hits.first().map(|h| h.node_id.as_str()),
        Some("chg:teardowns-wait"),
        "a word only the rationale carries finds the change: {found:?}"
    );
}
