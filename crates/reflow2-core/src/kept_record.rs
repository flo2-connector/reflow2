//! A COMPUTED FACT COUNTS ONLY THROUGH ITS KEPT RECORD.
//!
//! `req:a-computed-fact-counts-only-through-its-kept-record` (Anthony,
//! 2026-10-17, after the primes trial, item 1 of
//! `fact:the-primes-trial-rung-1-ran-on-flo2-io-and-its-friction-names-what-counts-2026-10-17`).
//! In the trial, a computed fact held an inference premise up through a
//! Verification an agent set `passing` by hand, and nothing tied that status to
//! the calculator's kept record. So a check made by computation now reads
//! `passing` as a premise only through its record.
//!
//! # Which checks are computed
//!
//! The schema has no `computation` method, and adding one would move the
//! schema stamp, so a check says it was computed in one of two ways:
//!
//! - it sets `computed: true`;
//! - or its `location`, or a record it names, is a calculator record: a file
//!   whose name ends [`CALC_RECORD_SUFFIX`] (what flo2-calc keeps).
//!
//! Every other check reads exactly as it did in 0.85.0.
//!
//! # What names a record
//!
//! An Artifact counts as the check's record when it is linked by one of:
//!
//! - `DOCUMENTS` (the Artifact documents the check);
//! - `PRODUCES` (the check produced the Artifact; the edge's `outcome` is the
//!   record stating its result);
//! - the check's `location`: the location holds the Artifact's id (`art:…`) or
//!   is the Artifact's own `location`.
//!
//! # When a record is KEPT
//!
//! A record is kept when it carries a checksum and no unresolved
//! `checksum_change` or `missing_artifact` DriftEvent depends on it. The server
//! never opens the file: on a hosted design it cannot, and on a local one the
//! drift detection already compares the file with that checksum. A record that
//! is kept STATES THE RESULT through the `outcome` on its PRODUCES edge when
//! there is one, and otherwise through the check's `findings`, which restate
//! what the record holds, or its `hypothesis_test`, the record's own block for
//! a statistical test ([`crate::hypothesis`]).
//!
//! # What it does not check
//!
//! That the record says what the check claims. A host such as flo2 can re-run
//! the record on request; reflow2 reads the binding (the checksum and the
//! link), never the bytes.

use std::collections::BTreeMap;

use crate::foundation::core::{DynoError, Value};
use crate::graph::DesignGraph;
use crate::inference::Truth;
use crate::nodes::{edge, node};

/// The file-name ending of a calculator's kept record (flo2-calc's
/// `<name>.calc.json`). A check whose location or record ends with it is a
/// computed check whether or not it says `computed: true`.
pub const CALC_RECORD_SUFFIX: &str = ".calc.json";

/// One Artifact a check names as its record, and how it stands.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CheckRecord {
    /// The Artifact's id.
    pub id: String,
    /// Its name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Where a person opens it, when recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// How the check names it: `documents`, `produces` or `location`. A
    /// record named more than one way lists each.
    pub named_by: Vec<&'static str>,
    /// Whether a checksum pins the version that was kept.
    pub pinned: bool,
    /// Whether an unresolved drift says the file no longer matches.
    pub drifted: bool,
    /// The `outcome` on the PRODUCES edge (`pass`, `fail`, `partial`,
    /// `error`), when the check produced it and said.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
}

impl CheckRecord {
    /// Pinned by a checksum, and no drift says the file moved.
    pub fn is_kept(&self) -> bool {
        self.pinned && !self.drifted
    }
}

fn str_prop<'a>(props: &'a std::collections::HashMap<String, Value>, key: &str) -> Option<&'a str> {
    props
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
}

/// The `art:` ids a location string holds, in order: `"x.calc.json
/// (art:file-x)"` gives `art:file-x`.
fn artifact_ids_in(location: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = location;
    while let Some(at) = rest.find("art:") {
        let tail = &rest[at..];
        let end = tail
            .find(|c: char| c.is_whitespace() || matches!(c, ')' | '(' | ',' | ';' | '"' | '\''))
            .unwrap_or(tail.len());
        let id = tail[..end].trim_end_matches(['.', ':']);
        if id.len() > "art:".len() {
            out.push(id.to_string());
        }
        rest = &tail[end.max(1)..];
    }
    out
}

/// The path part of a location: its first whitespace-separated word.
fn location_path(location: &str) -> &str {
    location.split_whitespace().next().unwrap_or("")
}

impl DesignGraph {
    /// Every Artifact the check `verification_id` names as its record, sorted
    /// by id. Empty when it names none.
    pub fn records_of_check(&self, verification_id: &str) -> Result<Vec<CheckRecord>, DynoError> {
        let Some(check) = self.get_node(node::VERIFICATION, verification_id)? else {
            return Ok(Vec::new());
        };
        let mut named: BTreeMap<String, (Vec<&'static str>, Option<String>)> = BTreeMap::new();
        for e in self.incoming(verification_id, Some(edge::DOCUMENTS))? {
            named.entry(e.from_id).or_default().0.push("documents");
        }
        for e in self.outgoing(verification_id, Some(edge::PRODUCES))? {
            let outcome = e
                .properties
                .get("outcome")
                .and_then(Value::as_str)
                .map(str::to_string);
            let slot = named.entry(e.to_id).or_default();
            slot.0.push("produces");
            if outcome.is_some() {
                slot.1 = outcome;
            }
        }
        if let Some(loc) = str_prop(&check.properties, "location") {
            let ids = artifact_ids_in(loc);
            let mut found = false;
            for id in ids {
                if self.get_node(node::ARTIFACT, &id)?.is_some() {
                    named.entry(id).or_default().0.push("location");
                    found = true;
                }
            }
            // No id in the location: match it against the Artifacts' own
            // locations. Read only here, so a check that names its record by
            // id never pays for the scan.
            if !found {
                let path = location_path(loc);
                for a in self.scan_nodes(node::ARTIFACT)? {
                    if let Some(theirs) = str_prop(&a.properties, "location")
                        && (theirs == loc || theirs == path)
                    {
                        named.entry(a.node_id).or_default().0.push("location");
                    }
                }
            }
        }
        let mut out = Vec::with_capacity(named.len());
        for (id, (mut by, outcome)) in named {
            let Some(a) = self.get_node(node::ARTIFACT, &id)? else {
                continue;
            };
            by.sort();
            by.dedup();
            out.push(CheckRecord {
                name: str_prop(&a.properties, "name").map(str::to_string),
                location: str_prop(&a.properties, "location").map(str::to_string),
                named_by: by,
                pinned: str_prop(&a.properties, "checksum").is_some(),
                drifted: self.record_has_drifted(&id)?,
                outcome,
                id,
            });
        }
        Ok(out)
    }

    /// An unresolved `checksum_change` or `missing_artifact` DriftEvent
    /// depends on the Artifact: the file no longer matches what was kept.
    fn record_has_drifted(&self, artifact_id: &str) -> Result<bool, DynoError> {
        for e in self.incoming(artifact_id, Some(edge::DEPENDS_ON))? {
            let Some(d) = self.get_node(node::DRIFT_EVENT, &e.from_id)? else {
                continue;
            };
            let resolved = d
                .properties
                .get("resolved")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let kind = str_prop(&d.properties, "drift_type");
            if !resolved && matches!(kind, Some("checksum_change" | "missing_artifact")) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether the check says it was made by computation: `computed: true`,
    /// or its location or one of its records is a calculator record.
    pub fn check_is_computed(
        &self,
        props: &std::collections::HashMap<String, Value>,
        records: &[CheckRecord],
    ) -> bool {
        if props.get("computed").and_then(Value::as_bool) == Some(true) {
            return true;
        }
        let calc = |s: &str| location_path(s).ends_with(CALC_RECORD_SUFFIX);
        str_prop(props, "location").is_some_and(calc)
            || records
                .iter()
                .any(|r| r.location.as_deref().is_some_and(calc))
    }

    /// A `passing` check's truth as a premise, when it is a computed one:
    /// `None` for a check that is not computed (it reads as it always did).
    pub(crate) fn computed_check_truth(
        &self,
        verification_id: &str,
        props: &std::collections::HashMap<String, Value>,
    ) -> Result<Option<(Truth, String)>, DynoError> {
        let records = self.records_of_check(verification_id)?;
        if !self.check_is_computed(props, &records) {
            return Ok(None);
        }
        let kept: Vec<&CheckRecord> = records.iter().filter(|r| r.is_kept()).collect();
        if kept.is_empty() {
            let mut why = "asserted, not passing: the check was made by computation and is set \
                           passing with no kept record behind it. Name its record — an Artifact \
                           with a checksum that DOCUMENTS the check, that the check PRODUCES, or \
                           that its `location` names"
                .to_string();
            let unkept: Vec<String> = records
                .iter()
                .map(|r| {
                    let state = if !r.pinned {
                        "carries no checksum"
                    } else {
                        "has drifted from its checksum"
                    };
                    format!("{} {state}", r.id)
                })
                .collect();
            if !unkept.is_empty() {
                why.push_str(&format!(" ({})", unkept.join("; ")));
            }
            return Ok(Some((Truth::Unknown, why)));
        }
        // The result, as the record states it: an outcome on a PRODUCES edge
        // wins; otherwise the check's findings restate it.
        let stated: Vec<(&str, &str)> = kept
            .iter()
            .filter_map(|r| r.outcome.as_deref().map(|o| (r.id.as_str(), o)))
            .collect();
        if let Some((id, o)) = stated.iter().find(|(_, o)| matches!(*o, "fail" | "error")) {
            return Ok(Some((
                Truth::Fails,
                format!(
                    "the check is set passing, but its kept record {id} states the result `{o}`"
                ),
            )));
        }
        if let Some((id, _)) = stated.iter().find(|(_, o)| *o == "pass") {
            return Ok(Some((
                Truth::Holds,
                format!(
                    "the computed check is passing, and its kept record {id} states the result `pass`"
                ),
            )));
        }
        if let Some((id, o)) = stated.first() {
            return Ok(Some((
                Truth::Unknown,
                format!(
                    "its kept record {id} states the result `{o}`, which is neither pass nor fail"
                ),
            )));
        }
        let ids: Vec<&str> = kept.iter().map(|r| r.id.as_str()).collect();
        if str_prop(props, "findings").is_some() {
            return Ok(Some((
                Truth::Holds,
                format!(
                    "the computed check is passing through its kept record {}, and its findings \
                     state the result",
                    ids.join(", ")
                ),
            )));
        }
        // A HYPOTHESIS TEST'S BLOCK IS THE RECORD'S OWN STATEMENT of its
        // result (crate::hypothesis), copied from the record when the test was
        // recorded, so it states the result as findings would.
        if str_prop(props, crate::hypothesis::HYPOTHESIS_TEST).is_some() {
            return Ok(Some((
                Truth::Holds,
                format!(
                    "the computed check is passing through its kept record {}, and its \
                     hypothesis test states the result",
                    ids.join(", ")
                ),
            )));
        }
        Ok(Some((
            Truth::Unknown,
            format!(
                "asserted, not passing: its record {} is kept, but nothing states the result — \
                 record the run's findings, or the `outcome` on the check's PRODUCES edge to it",
                ids.join(", ")
            ),
        )))
    }
}

#[cfg(test)]
mod a_location_names_its_record {
    use super::*;

    #[test]
    fn an_id_in_brackets_and_a_bare_path_are_both_read() {
        assert_eq!(
            artifact_ids_in("x-n40.calc.json (art:file-x-n40-calc-json)"),
            vec!["art:file-x-n40-calc-json".to_string()]
        );
        assert_eq!(artifact_ids_in("tests/a.rs"), Vec::<String>::new());
        assert_eq!(
            artifact_ids_in("art:a, art:b."),
            vec!["art:a".to_string(), "art:b".to_string()]
        );
        assert_eq!(location_path("x.calc.json (art:y)"), "x.calc.json");
    }
}
