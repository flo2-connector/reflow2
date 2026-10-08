//! DEPENDS — declare which version of another design you depend on, and check
//! that claim against the build (`req:design-dependencies-declared`).
//!
//! ## Why this is not optional bookkeeping
//!
//! A seam analysis compares your design against a dependency's published
//! surface. Both sides move. Without a recorded pin there is nothing to take a
//! surface *as of*, so the comparison silently answers a question nobody asked:
//! "are you compatible with whatever the provider's `main` happens to be right
//! now?" — when you are not on `main` and will not be until you bump.
//!
//! Proven, not supposed: reflow2 pins dynograph-foundation at `v0.11.0` while
//! storyflow pins `v0.9.4`, two minors apart, and the provider could not produce
//! an as-of-tag surface at all. An offer taken from `main` described **neither**
//! consumer's actual contract.
//!
//! ## Two different facts, and conflating them is the bug
//!
//! - **What you MEAN to depend on** — the declaration. Durable, reviewed,
//!   committed, and the thing a provider can acknowledge.
//! - **What your build ACTUALLY resolves** — the observation. Read fresh from
//!   the build files every time, because that is what ships.
//!
//! Storing only the first gives you a document that drifts from reality. Storing
//! only the second gives you a fact with no intent behind it, so nothing can
//! ever be *wrong*. Keeping both, and comparing them, is what makes
//! "am I relying on something I never declared?" answerable — the state the
//! cross-repo trial named as the dangerous one, because it breaks with nobody at
//! fault.
//!
//! ## Why core does not parse Cargo.toml
//!
//! The caller supplies the observation, exactly as [`reconcile_artifacts`] takes
//! `observed` and `coverage_report` takes paths. Two reasons, and the second is
//! the load-bearing one:
//!
//! 1. One parser per ecosystem in Rust is a maintenance burden with no analytic
//!    gain — an agent reads a manifest perfectly well.
//! 2. **The consumers are not all Rust.** storyflow pins foundation crates in a
//!    `Cargo.toml`, a container image in `docker-compose.yml`, and versions in a
//!    `versions.env` — three build files, one dependency. A core that understood
//!    only Cargo would model a third of that seam and report the rest as absent.

use std::collections::{BTreeMap, BTreeSet};

use crate::foundation::core::{DynoError, Value};

use crate::graph::DesignGraph;
use crate::nodes::{Props, edge, node};

/// A dependency on another design, as this design DECLARES it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DependencyDeclaration {
    /// Stable id for the dependency, e.g. `dep:dynograph-foundation`.
    pub id: String,
    /// What it is called.
    pub name: String,
    /// Where it comes from — a git URL, a registry, a path. Free text, because
    /// "where a dependency comes from" has no closed vocabulary.
    pub source: String,
    /// The version this design MEANS to depend on: a tag, a commit, a release.
    /// The whole point of the file.
    pub version: String,
    /// The parts actually taken — crate names, service names, whatever the
    /// dependency's unit of consumption is.
    pub components: Vec<String>,
    /// Build-level switches this design forwards to the dependency BY NAME.
    /// Recorded because they are contract whether or not the provider thinks so:
    /// a renamed feature is a downstream build break that no public-API diff or
    /// surface export would mention.
    pub features: Vec<String>,
    /// Which build file the pin actually lives in, so the claim can be rechecked
    /// at its source rather than trusted.
    pub declared_in: Option<String>,
    /// The `graph_id` of the dependency's OWN reflow2 design, when it has one.
    ///
    /// This is the link between two facts that already sit side by side in the
    /// same file and never touched: "my build pins v0.12.0 of this thing" and
    /// "that thing is also a design I can compose with". With it, the
    /// composition target is derivable from a committed, version-pinned manifest
    /// instead of being configured per machine — and it inherits the DIRECTION
    /// the dependency edge already carries, which a flat list of graph ids
    /// cannot express.
    ///
    /// ⚠️ OPTIONAL, AND ITS ABSENCE MEANS "NOBODY HAS SAID" — never "there is no
    /// design". Most dependencies will never have one: serde, tokio, rocksdb.
    /// A dependency without a graph_id is the ordinary case and must not read as
    /// a defect, which is the same rule `reconcile_dependencies` already applies
    /// to the manifest as a whole.
    pub graph_id: Option<String>,

    /// Path to the dependency design's COMMITTED EXPORT, when this design means
    /// to watch it.
    ///
    /// ⭐ WHY A SECOND FIELD AND NOT JUST `graph_id`. The id says WHICH design;
    /// this says WHERE ITS RECORD IS. An id alone is not resolvable — reflow2
    /// does no file navigation (`describe_designs` makes the caller find the
    /// candidate paths for exactly this reason), so a watch that had only an id
    /// would have to go looking, which is the rule this deliberately does not
    /// break. Naming the path in the committed manifest keeps the pointer where
    /// a person can review it in a diff.
    ///
    /// Absent means NOBODY HAS SAID, never "there is nothing to watch": a
    /// declaration naming a design and no export is reported as `not_watched`
    /// rather than passing quietly.
    #[serde(default)]
    pub design_export: Option<String>,
    /// The upstream export's content hash AS THE DECLARER LAST SAW IT.
    ///
    /// 🛑 THIS IS A BASELINE, NOT A CACHE, and nothing may refresh it on a read.
    /// A check that updated its own baseline would report `moved` exactly once
    /// and then be permanently quiet — the failure mode that makes a signal
    /// worse than no signal. Re-declaring is the acknowledgement
    /// (`dec:ask-not-repair`: name the remedy, never take it).
    #[serde(default)]
    pub design_export_hash: Option<String>,
    /// When that baseline was taken. Caller-supplied: the core takes no clock,
    /// so an undated baseline is REPORTED as undated and never assumed fresh.
    #[serde(default)]
    pub design_export_seen_at: Option<String>,
    /// The ADDRESS of the server that holds the dependency design —
    /// `https://api.flo2.io/g/<id>/mcp`, an organisation's own reflow2, or a
    /// local one — when this design means to watch it there
    /// (`req:a-design-watches-another-design-at-the-server-that-holds-it`).
    ///
    /// ⭐ WHY THIS AND NOT `design_export`. Under the rules settled on 2026-09-27
    /// (`dec:idea-one-blueprint-the-store-is-the-design-and-an-export-is-a-perishable-photocopy`)
    /// a design IS the data store that holds it, and an export is a copy of it
    /// at one moment that nothing tracks. A design that moved to a server left
    /// its last export behind, and watching that file reported the move
    /// itself and would then have reported "unchanged" forever
    /// (`fact:a-moved-design-cannot-be-watched-and-its-frozen-export-reads-as-live-2026-09-27`).
    /// The address is where the design actually is, so it is what gets asked.
    ///
    /// A dependency is watched in ONE place: declaring both this and
    /// `design_export` is refused, because two baselines for one design would
    /// disagree as soon as either copy moved and nothing could say which is
    /// true. The credential is never part of the declaration — the client
    /// carries it.
    #[serde(default)]
    pub design_address: Option<String>,
    /// The fingerprint the server gave for the design AS THE DECLARER LAST SAW
    /// IT — a baseline exactly like `design_export_hash`, and never refreshed
    /// on a read for the same reason.
    #[serde(default)]
    pub design_address_hash: Option<String>,
    /// When that baseline was taken, caller-supplied like
    /// `design_export_seen_at`.
    #[serde(default)]
    pub design_address_seen_at: Option<String>,
    /// Free-text note — why this pin, what was verified, what is owed.
    pub note: Option<String>,
    /// How this design stands to the dependency's design, when it is a member
    /// of a hub or a tier (`req:a-hub-records-each-members-relation-and-a-cross-design-ripple-follows-it`):
    /// `part_of` (the dependency is a part of this design — intent flows down,
    /// status flows up) and/or `uses` (a peer this design uses across named
    /// interfaces). Both may hold. EMPTY MEANS "NOT STATED", never a guess:
    /// the relation is the person's to say, so a pin without it is reported
    /// as unstated rather than defaulted.
    #[serde(default)]
    pub relation: Vec<String>,
    /// For a `uses` relation, the Interface ids the use crosses — the ids a
    /// cross-design ripple follows, so each one is meant to exist here as a
    /// mirrored Interface (`mirror_surface`, the `link-projects` skill).
    #[serde(default)]
    pub interfaces: Vec<String>,
}

/// The relations a member can stand in. `part_of` is the tier (the member is a
/// part of this design); `uses` is a peer across interfaces.
pub const MEMBER_RELATIONS: [&str; 2] = ["part_of", "uses"];

/// Where a declared upstream design is watched: its committed export on disk,
/// or the server that holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WatchedAt<'a> {
    Export(&'a str),
    Address(&'a str),
}

impl WatchedAt<'_> {
    fn location(&self) -> &str {
        match self {
            Self::Export(p) | Self::Address(p) => p,
        }
    }
}

/// The non-empty value of an optional text field, or `None`. An empty string
/// is never a statement: it would claim a target, a hash or a date that is
/// blank, which is a different fact from nobody having said.
fn stated(v: &Option<String>) -> Option<&str> {
    v.as_deref().filter(|s| !s.trim().is_empty())
}

impl DependencyDeclaration {
    /// Where this dependency is watched, with the baseline and its date. `None`
    /// when it is not watched at all.
    fn watched_at(&self) -> Option<(WatchedAt<'_>, Option<&str>, Option<&str>)> {
        if let Some(a) = stated(&self.design_address) {
            return Some((
                WatchedAt::Address(a),
                stated(&self.design_address_hash),
                stated(&self.design_address_seen_at),
            ));
        }
        stated(&self.design_export).map(|p| {
            (
                WatchedAt::Export(p),
                stated(&self.design_export_hash),
                stated(&self.design_export_seen_at),
            )
        })
    }
}

/// What a build actually resolves, supplied by the caller at check time.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ObservedDependency {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub components: Vec<String>,
    #[serde(default)]
    pub features: Vec<String>,
    /// Where this was read from.
    #[serde(default)]
    pub observed_in: Option<String>,
}

/// One disagreement between what was declared and what the build resolves.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DependencyFinding {
    /// `undeclared` | `unobserved` | `version_mismatch` | `undeclared_component`
    /// | `undeclared_feature`
    pub kind: &'static str,
    pub dependency: String,
    pub detail: String,
}

/// The result of checking declarations against a build.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DependencyReport {
    pub declared: Vec<DependencyDeclaration>,
    pub findings: Vec<DependencyFinding>,
    /// Declarations an accepted Decision has WITHDRAWN, skipped by the
    /// `unobserved` check rather than reported as stale. Named rather than
    /// dropped: a dependency that ended is design history and stays readable,
    /// but it must not keep failing a gate for not being in a build it was
    /// deliberately removed from.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retired_declarations: Vec<String>,
    /// Said plainly whichever way it comes out — "nothing declared" and
    /// "nothing to declare" must never look alike.
    pub note: String,
}

/// The properties a declaration's WATCH owns: the target (an export path or a
/// server address), the baseline taken against it, and when. Rewritten as a
/// set on every declaration, because a baseline is only meaningful against the
/// target it was taken from, and a dependency is watched in ONE place.
const WATCH_PROPERTIES: [&str; 6] = [
    "design_export",
    "design_export_hash",
    "design_export_seen_at",
    "design_address",
    "design_address_hash",
    "design_address_seen_at",
];

impl DesignGraph {
    /// Declare a dependency on another design (`req:design-dependencies-declared`).
    ///
    /// ⭐ A RE-DECLARE REVISES; IT DOES NOT REPLACE. Every property this call
    /// does not own survives it (a `description` written with `add_resource`,
    /// anything else on the Resource), and so does every optional one the
    /// declaration leaves out (`declared_in`, `graph_id`, `note`). What the
    /// declaration states is written: name, source, version, components,
    /// features, and the watch as one set ([`WATCH_PROPERTIES`]), so naming an
    /// address drops a stored export watch and the other way round.
    ///
    /// Until 2026-10-03 this wrote with a REPLACING `create_node`, and every
    /// re-pin silently dropped the Resource's description, 2 to 5 KB of
    /// history each time, eight times in the field across 0.66 to 0.77
    /// (fact:re-declaring-a-dependency-drops-the-resources-description-2026-09-23).
    /// The tool layer additionally carries an omitted `components`, `features`
    /// and watch target forward from the stored declaration, which a core
    /// declaration (whose lists are plain `Vec`s) cannot express.
    pub fn declare_external_dependency(
        &mut self,
        decl: &DependencyDeclaration,
    ) -> Result<(), DynoError> {
        if decl.version.trim().is_empty() {
            return Err(DynoError::Validation {
                node_type: node::RESOURCE.into(),
                property: "version".into(),
                message: "a dependency declaration without a version is not a declaration: the \
                          version is the whole point, because it is what a published surface can \
                          be taken AS OF"
                    .into(),
            });
        }
        if stated(&decl.design_export).is_some() && stated(&decl.design_address).is_some() {
            return Err(DynoError::Validation {
                node_type: node::RESOURCE.into(),
                property: "design_address".into(),
                message: "a dependency is watched in ONE place: name the server that holds the \
                          design (`design_address`) or, for a design still kept beside its \
                          repository, its committed export (`design_export`) — not both. Two \
                          baselines for one design disagree as soon as either copy moves, and \
                          nothing could then say which is the design"
                    .into(),
            });
        }
        if let Some(bad) = decl
            .relation
            .iter()
            .find(|r| !MEMBER_RELATIONS.contains(&r.as_str()))
        {
            return Err(DynoError::Validation {
                node_type: node::RESOURCE.into(),
                property: "relation".into(),
                message: format!(
                    "{bad:?} is not a relation a member can stand in. Use `part_of` (the \
                     dependency is a part of this design: intent flows down to it, status up from \
                     it) and/or `uses` (a peer this design uses across named interfaces)"
                ),
            });
        }
        if !decl.interfaces.is_empty() && !decl.relation.iter().any(|r| r == "uses") {
            return Err(DynoError::Validation {
                node_type: node::RESOURCE.into(),
                property: "interfaces".into(),
                message: "`interfaces` names what a `uses` relation crosses, and this declaration \
                          does not say `uses`. Add `uses` to `relation`, or leave `interfaces` out"
                    .into(),
            });
        }
        let mut props = Props::new()
            .set("name", decl.name.as_str())
            .set("resource_type", "design-dependency")
            .set("provider", decl.source.as_str())
            .set("version", decl.version.as_str())
            .set("components", decl.components.join(","))
            .set("features", decl.features.join(","));
        if let Some(d) = &decl.declared_in {
            props = props.set("declared_in", d.as_str());
        }
        // Only when stated. An empty string would be a claim that the dependency
        // has a design whose id happens to be blank, which is not the same as
        // nobody having said.
        if let Some(g) = decl.graph_id.as_deref().filter(|g| !g.trim().is_empty()) {
            props = props.set("dependency_graph_id", g);
        }
        // Only when stated, for the same reason `graph_id` is: an empty string
        // would claim a watch target whose path is blank, which is not the same
        // fact as nobody having named one.
        // ⚠️ THE BASELINE AND ITS DATE EXIST ONLY RELATIVE TO A PATH, so they are
        // stored only when there is one. A hash with nothing to compare it
        // against, or a date saying when a target nobody named was last seen,
        // is a record of a check that never happened — and it reads to a person
        // scanning the manifest exactly like one that did. Found by
        // `an_unwatched_dependency_emits_no_watch_fields_at_all`, which failed
        // on a leftover `design_export_seen_at`.
        if let Some(p) = decl
            .design_export
            .as_deref()
            .filter(|p| !p.trim().is_empty())
        {
            props = props.set("design_export", p);
            if let Some(h) = decl
                .design_export_hash
                .as_deref()
                .filter(|h| !h.trim().is_empty())
            {
                props = props.set("design_export_hash", h);
            }
            if let Some(a) = decl
                .design_export_seen_at
                .as_deref()
                .filter(|a| !a.trim().is_empty())
            {
                props = props.set("design_export_seen_at", a);
            }
        }
        // The same rule for an address: the baseline and its date exist only
        // relative to the address they were taken against.
        if let Some(a) = stated(&decl.design_address) {
            props = props.set("design_address", a);
            if let Some(h) = stated(&decl.design_address_hash) {
                props = props.set("design_address_hash", h);
            }
            if let Some(s) = stated(&decl.design_address_seen_at) {
                props = props.set("design_address_seen_at", s);
            }
        }
        if let Some(n) = &decl.note {
            props = props.set("description", n.as_str());
        }
        // Start from what the node holds, drop the watch as a set, then lay
        // the declaration over it.
        let mut merged: std::collections::HashMap<String, Value> = self
            .get_node(node::RESOURCE, &decl.id)?
            .map(|n| n.properties)
            .unwrap_or_default();
        for k in WATCH_PROPERTIES {
            merged.remove(k);
        }
        // The relation is stated as a whole or not at all: an empty list clears
        // it, so a stale relation never outlives the declaration that dropped it.
        for k in ["member_relation", "relation_interfaces"] {
            merged.remove(k);
        }
        if !decl.relation.is_empty() {
            props = props.set("member_relation", decl.relation.join(","));
        }
        if !decl.interfaces.is_empty() {
            props = props.set("relation_interfaces", decl.interfaces.join(","));
        }
        merged.extend(std::collections::HashMap::from(props));
        self.create_node(node::RESOURCE, &decl.id, merged)?;
        for p in self.scan_nodes(node::PROJECT)? {
            self.create_edge(
                edge::REQUIRES_RESOURCE,
                node::PROJECT,
                &p.node_id,
                node::RESOURCE,
                &decl.id,
                std::collections::HashMap::from([(
                    "criticality".to_string(),
                    Value::from("required"),
                )]),
            )?;
        }
        Ok(())
    }

    /// Every declared dependency, in id order.
    pub fn declared_dependencies(&self) -> Result<Vec<DependencyDeclaration>, DynoError> {
        let mut out = Vec::new();
        for n in self.scan_nodes(node::RESOURCE)? {
            let get = |k: &str| {
                n.properties
                    .get(k)
                    .and_then(Value::as_str)
                    .map(str::to_string)
            };
            if get("resource_type").as_deref() != Some("design-dependency") {
                continue;
            }
            let split = |k: &str| -> Vec<String> {
                get(k)
                    .unwrap_or_default()
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect()
            };
            out.push(DependencyDeclaration {
                id: n.node_id.clone(),
                name: get("name").unwrap_or_default(),
                source: get("provider").unwrap_or_default(),
                version: get("version").unwrap_or_default(),
                components: split("components"),
                features: split("features"),
                declared_in: get("declared_in"),
                graph_id: get("dependency_graph_id"),
                design_export: get("design_export"),
                design_export_hash: get("design_export_hash"),
                design_export_seen_at: get("design_export_seen_at"),
                design_address: get("design_address"),
                design_address_hash: get("design_address_hash"),
                design_address_seen_at: get("design_address_seen_at"),
                note: get("description"),
                relation: split("member_relation"),
                interfaces: split("relation_interfaces"),
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// Check the declarations against what a build actually resolves.
    ///
    /// Catches the two opposite failures the cross-repo trial named:
    /// **relying on something never declared**, and **declaring something the
    /// build no longer takes**.
    pub fn reconcile_dependencies(
        &self,
        observed: &[ObservedDependency],
    ) -> Result<DependencyReport, DynoError> {
        let declared = self.declared_dependencies()?;
        let by_name: BTreeMap<&str, &DependencyDeclaration> =
            declared.iter().map(|d| (d.name.as_str(), d)).collect();
        let observed_names: BTreeSet<&str> = observed.iter().map(|o| o.name.as_str()).collect();
        let mut findings = Vec::new();

        for o in observed {
            let Some(d) = by_name.get(o.name.as_str()) else {
                findings.push(DependencyFinding {
                    kind: "undeclared",
                    dependency: o.name.clone(),
                    detail: format!(
                        "the build depends on '{}' at {} and nothing declares it — this is the \
                         reliance nobody agreed to, and it breaks with nobody at fault",
                        o.name, o.version
                    ),
                });
                continue;
            };
            if d.version != o.version {
                findings.push(DependencyFinding {
                    kind: "version_mismatch",
                    dependency: o.name.clone(),
                    detail: format!(
                        "declared {} but the build resolves {} — a seam checked against the \
                         declared version would be answering about a version you do not ship",
                        d.version, o.version
                    ),
                });
            }
            let known: BTreeSet<&str> = d.components.iter().map(String::as_str).collect();
            for c in &o.components {
                if !known.contains(c.as_str()) {
                    findings.push(DependencyFinding {
                        kind: "undeclared_component",
                        dependency: o.name.clone(),
                        detail: format!(
                            "the build takes '{c}' and the declaration does not list it"
                        ),
                    });
                }
            }
            let known_f: BTreeSet<&str> = d.features.iter().map(String::as_str).collect();
            for f in &o.features {
                if !known_f.contains(f.as_str()) {
                    findings.push(DependencyFinding {
                        kind: "undeclared_feature",
                        dependency: o.name.clone(),
                        detail: format!(
                            "the build forwards feature '{f}' by name and the declaration does \
                             not list it — a renamed feature is a build break no API diff mentions"
                        ),
                    });
                }
            }
        }
        let mut retired = Vec::new();
        for d in &declared {
            if observed_names.contains(d.name.as_str()) {
                continue;
            }
            // A RETIRED DECLARATION IS NOT A STALE ONE, and reporting it as
            // `unobserved` forever is how a correct retirement becomes a
            // permanently red gate. `is_discontinued` is the design's existing
            // answer to "has an accepted Decision withdrawn this?" — the same
            // test `get_node` reports and the defect detectors already use.
            //
            // 🛑 THIS WAS FOUND BY DOGFOODING AND IT IS A CLASS, NOT A ONE-OFF.
            // reflow2 absorbed dynograph-foundation on 2026-08-24, retired
            // `dep:dynograph-foundation` correctly — deprecation ChangeEvent,
            // snapshot, OBSOLETES from the accepted Decision — and the gate went
            // on failing, because this reader never asked. The design already
            // records that `is_discontinued` is honoured at only a handful of
            // sites; this was another of them.
            //
            // IT IS STILL REPORTED, not silenced: `retired_declarations` says
            // which ones were skipped and why, because a declaration vanishing
            // from a report with no trace is the silent-success failure this
            // project spends most of its guards on.
            if self.is_discontinued(&d.id)? {
                retired.push(d.name.clone());
                continue;
            }
            findings.push(DependencyFinding {
                kind: "unobserved",
                dependency: d.name.clone(),
                detail: format!(
                    "'{}' is declared at {} and the build does not take it — either the \
                     declaration is stale or the observation is incomplete; both are worth \
                     knowing and neither is assumed",
                    d.name, d.version
                ),
            });
        }

        let note = if declared.is_empty() {
            "NOTHING IS DECLARED. Read that as \"nobody has said\", never as \"this design depends \
             on nothing\" — an empty declaration set is indistinguishable from one never written, \
             which is why it is stated rather than left to inference."
                .to_string()
        } else if observed.is_empty() {
            // NOTHING OBSERVED IS NOT AGREEMENT. `findings` is empty both when
            // the build agreed and when there was no build reading to disagree
            // WITH, and the reply claimed agreement for both — while the
            // `observed` parameter explicitly offers being omitted "to report
            // the declarations without checking them". A caller who omits it
            // was told the build agreed (measured 2026-09-11).
            format!(
                "{} dependency(ies) declared and NOTHING WAS OBSERVED, so nothing was checked \
                 against the build. This is the declarations read back, never a statement that \
                 they hold: pass `observed`, read fresh from the build files, to find out.",
                declared.len()
            )
        } else if findings.is_empty() {
            format!(
                "{} dependency(ies) declared, and the build agrees with every one \
                 ({} observed).",
                declared.len(),
                observed.len()
            )
        } else {
            format!(
                "{} dependency(ies) declared, {} disagreement(s) with the build.",
                declared.len(),
                findings.len()
            )
        };
        Ok(DependencyReport {
            declared,
            findings,
            retired_declarations: retired,
            note,
        })
    }

    /// The declarations as a `reflow2.toml` document.
    ///
    /// Carries **which reflow2 wrote it** (Anthony's ask, and the same reasoning
    /// as the export's version stamp): a file whose producer is unknown cannot
    /// be read safely by a tool that has since changed what the fields mean.
    pub fn dependency_manifest(&self) -> Result<String, DynoError> {
        let declared = self.declared_dependencies()?;
        let mut s = String::new();
        s.push_str("# reflow2 dependency declarations — which version of another design this\n");
        s.push_str("# design depends on. GENERATED: re-derive it from the build files rather\n");
        s.push_str("# than editing by hand, because a hand-kept pin drifts from the build and\n");
        s.push_str("# the build is what ships.\n\n");
        s.push_str("[reflow2]\n");
        s.push_str(&format!("version = \"{}\"\n", env!("CARGO_PKG_VERSION")));
        s.push_str(&format!("graph_id = \"{}\"\n", self.graph_id()));
        if declared.is_empty() {
            s.push_str(
                "\n# NOTHING DECLARED. This is \"nobody has said\", not \"depends on nothing\".\n",
            );
            return Ok(s);
        }
        for d in &declared {
            s.push_str(&format!("\n[dependencies.{}]\n", d.name));
            s.push_str(&format!("source = \"{}\"\n", d.source));
            s.push_str(&format!("version = \"{}\"\n", d.version));
            if !d.components.is_empty() {
                s.push_str(&format!("components = {:?}\n", d.components));
            }
            if !d.features.is_empty() {
                s.push_str(&format!("features = {:?}\n", d.features));
            }
            if let Some(x) = &d.declared_in {
                s.push_str(&format!("declared_in = \"{x}\"\n"));
            }
            // Emitted only when stated. A reader must be able to tell "this
            // dependency has no reflow2 design" from "nobody recorded whether it
            // does", and an always-present empty field would collapse the two.
            if let Some(g) = &d.graph_id {
                s.push_str(&format!("graph_id = \"{g}\"\n"));
            }
            // The watch pointer and the baseline taken against it. Emitted
            // together and only when stated: a path with no hash is a target
            // nobody has looked at yet, and the manifest must be able to say so
            // rather than implying a check that never ran.
            if let Some(x) = &d.design_export {
                s.push_str(&format!("design_export = \"{x}\"\n"));
                // Only inside the path block: see the note on the write side.
                // A baseline with no target is a check that never happened
                // wearing the clothes of one that did.
                if let Some(h) = &d.design_export_hash {
                    s.push_str(&format!("design_export_hash = \"{h}\"\n"));
                }
                if let Some(a) = &d.design_export_seen_at {
                    s.push_str(&format!("design_export_seen_at = \"{a}\"\n"));
                }
            }
            if let Some(x) = &d.design_address {
                s.push_str(&format!("design_address = \"{x}\"\n"));
                if let Some(h) = &d.design_address_hash {
                    s.push_str(&format!("design_address_hash = \"{h}\"\n"));
                }
                if let Some(a) = &d.design_address_seen_at {
                    s.push_str(&format!("design_address_seen_at = \"{a}\"\n"));
                }
            }
            if let Some(n) = &d.note {
                s.push_str(&format!("note = \"{}\"\n", n.replace('"', "'")));
            }
        }
        Ok(s)
    }
}

/// What a caller found at one declared upstream: the committed export on disk,
/// or the server that holds the design.
///
/// ⚠️ THE CALLER SUPPLIES THIS, exactly as `reconcile_dependencies` takes
/// `observed` and `reconcile_artifacts` takes hashes. `reflow2-core` does no
/// file I/O and no network I/O, deliberately and repeatedly, and reading
/// another design's record is not the exception that changes that — it is the
/// same split, one boundary along.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ObservedUpstream {
    /// The declaration's own id, so a finding names the thing that was declared.
    pub id: String,
    /// What the caller could establish. For an export on disk: `read` |
    /// `missing` | `unreadable`. For a design asked for at its server's
    /// address: `read` | `unreachable` (nobody answered, or the server failed)
    /// | `refused` (the server answered and would not show it) | `unreadable`
    /// (it answered with something that is not a reflow2 design).
    pub state: String,
    /// The export's COMPUTED content hash. Computed from content, never the
    /// hash the document states about itself — a record edited by anything
    /// other than `export_graph` keeps its old stamp, and trusting it is the
    /// defect `sync_debt` already had to fix once.
    #[serde(default)]
    pub content_hash: Option<String>,
    /// The `graph_id` the record at that place actually carries.
    #[serde(default)]
    pub graph_id: Option<String>,
    /// How many nodes it holds, for a reader who wants a sense of scale.
    #[serde(default)]
    pub nodes: Option<usize>,
    /// WHY a read did not succeed, in the words of whatever failed or refused:
    /// the server's own message for `refused`, what went wrong for
    /// `unreachable`. It never carries a credential. `None` when there is
    /// nothing to add.
    #[serde(default)]
    pub detail: Option<String>,
}

/// One thing to say about a declared dependency's upstream design.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpstreamFinding {
    /// `moved` | `unchanged` | `never_seen` | `missing` | `unreadable`
    /// | `unreachable` | `refused` | `graph_id_mismatch` | `not_watched`
    /// | `not_observed` — and two about the member's RELATION rather than its
    /// watch: `relation_not_stated` (the pin names a design and does not say
    /// whether it is part of this one or a peer it uses) and
    /// `no_interface_to_follow` (a `uses` link names no Interface here that a
    /// cross-design ripple could follow).
    pub kind: &'static str,
    /// The declaration's id.
    pub dependency: String,
    /// Its human name.
    pub name: String,
    /// The export path being watched, where the watch is on disk.
    pub design_export: Option<String>,
    /// The server address being watched, where the watch is at an address.
    /// Left out of the reply rather than written as null when the watch is an
    /// export, so a reader of an export watch sees exactly what it saw before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design_address: Option<String>,
    /// What a reader should do about it, in a sentence.
    pub detail: String,
}

impl UpstreamFinding {
    /// Whether this finding asks the reader to DO something.
    ///
    /// `unchanged` and `never_seen` do not: the first is the quiet ordinary
    /// case, and the second says nobody has looked yet, which is a statement
    /// about the record rather than about the upstream. `unreachable` and
    /// `refused` DO: whether the upstream moved is unknown, and something — a
    /// server, a credential — needs a person.
    pub fn is_actionable(&self) -> bool {
        matches!(
            self.kind,
            "moved"
                | "missing"
                | "unreadable"
                | "unreachable"
                | "refused"
                | "graph_id_mismatch"
                | "relation_not_stated"
                | "no_interface_to_follow"
                | "link_into_undeclared_design"
                | "link_far_end_moved"
                | "received_waiting"
        )
    }
}

/// How one member design stands to this one, as its pin says
/// (`cap:a-hub-member-records-its-relation-and-the-interfaces-it-crosses`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MemberRelation {
    /// The declaration's id.
    pub dependency: String,
    /// Its human name.
    pub name: String,
    /// The member's own design id.
    pub graph_id: String,
    /// `part_of` and/or `uses`, or `["not stated"]` when the pin does not say —
    /// written out, so an unstated relation never reads as an empty answer.
    pub relation: Vec<String>,
    /// For `uses`, the Interface ids the use crosses; omitted when none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<String>,
}

/// What the declared dependencies say about the designs upstream of them.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpstreamReport {
    pub findings: Vec<UpstreamFinding>,
    /// Every declared dependency that names another reflow2 design, with the
    /// relation its pin records. Omitted when there are none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<MemberRelation>,
    /// Every reference this design keeps to a node in another design, with
    /// its links (`crate::crosslink`). Omitted when there are none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<crate::crosslink::DesignReference>,
    /// Requirements another design sent here (crate::flowdown), with their
    /// status: `proposed` ones wait on this design's owner. Omitted when none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub received: Vec<crate::flowdown::ReceivedIntent>,
    /// What this design sent to others. Omitted when none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sent: Vec<crate::flowdown::SentIntent>,
    /// How many declarations name another reflow2 design at all.
    pub designs_declared: usize,
    /// How many of those name somewhere to watch it: an export path or a
    /// server address.
    pub watched: usize,
    /// Said plainly whichever way it comes out — "nothing watched" and
    /// "nothing has moved" must never look alike.
    pub note: String,
}

/// One declared upstream a caller should go and look at.
///
/// EXACTLY ONE of `design_export` and `design_address` is set: the declaration
/// refuses both, and a declaration with neither is not a target.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpstreamTarget {
    pub id: String,
    pub name: String,
    /// The committed export to read, when the design is watched on disk.
    pub design_export: Option<String>,
    /// The server to ask, when the design is watched at its address.
    pub design_address: Option<String>,
    /// The declared design's id, where one was named.
    pub graph_id: Option<String>,
    /// The baseline this design last saw, absent when nobody has looked.
    pub baseline_hash: Option<String>,
}

impl DesignGraph {
    /// The upstream designs a caller should look at, from the committed
    /// manifest: each one's export path or server address, and its baseline.
    ///
    /// This is the "who are my children" list, and it is deliberately NOT a new
    /// vocabulary: the dependency manifest already holds it, version-pinned and
    /// reviewable in a diff, and it carries the direction a flat list of ids
    /// could not express.
    pub fn upstream_targets(&self) -> Result<Vec<UpstreamTarget>, DynoError> {
        let mut out = Vec::new();
        for d in self.declared_dependencies()? {
            let Some((at, baseline, _)) = d.watched_at() else {
                continue;
            };
            let (design_export, design_address) = match at {
                WatchedAt::Export(p) => (Some(p.to_string()), None),
                WatchedAt::Address(a) => (None, Some(a.to_string())),
            };
            let baseline_hash = baseline.map(str::to_string);
            out.push(UpstreamTarget {
                id: d.id.clone(),
                name: d.name.clone(),
                design_export,
                design_address,
                graph_id: d.graph_id.clone(),
                baseline_hash,
            });
        }
        Ok(out)
    }

    /// Has the design this one depends on moved since the declaration was made?
    ///
    /// The second check `req:design-dependencies-declared` names in its own
    /// statement — *"declared-versus-upstream answers has what I depend on moved
    /// since"* — and the half that was never built. `reconcile_dependencies`
    /// answers the other one, against the BUILD.
    ///
    /// 🛑 IT NEVER UPDATES THE BASELINE. The recorded hash is what the declarer
    /// last looked at; refreshing it here would make this report `moved` exactly
    /// once and then go quiet forever. Re-declaring is the acknowledgement.
    ///
    /// ⚠️ SILENCE IS REPORTED, NOT ASSUMED. A dependency naming another design
    /// with nowhere to watch it comes back as `not_watched`; a target the caller
    /// did not look at comes back as `not_observed`; and a server that could not
    /// be reached, or would not answer, comes back as `unreachable` or
    /// `refused` — never as `unchanged`, because "nothing has moved" and
    /// "nothing was checked" must never share an answer.
    pub fn reconcile_upstream(
        &self,
        observed: &[ObservedUpstream],
    ) -> Result<UpstreamReport, DynoError> {
        let declared = self.declared_dependencies()?;
        let by_id: BTreeMap<&str, &ObservedUpstream> =
            observed.iter().map(|o| (o.id.as_str(), o)).collect();

        let mut findings = Vec::new();
        let mut designs_declared = 0usize;
        let mut watched = 0usize;

        for d in &declared {
            let names_a_design = stated(&d.graph_id).is_some();
            if names_a_design {
                designs_declared += 1;
            }
            let Some((at, baseline, seen_at)) = d.watched_at() else {
                // A dependency with no design and nowhere to watch is an
                // ordinary code dependency and says nothing here. One that NAMES
                // a design and gives nowhere to watch it is the silence worth
                // reporting.
                if names_a_design {
                    findings.push(UpstreamFinding {
                        kind: "not_watched",
                        dependency: d.id.clone(),
                        name: d.name.clone(),
                        design_export: None,
                        design_address: None,
                        detail: format!(
                            "'{}' names the reflow2 design '{}' but gives nowhere to watch it, so \
                             nothing here can tell you when it moves. Declare `design_address`: \
                             the server that holds that design. For a design still kept beside \
                             its repository, `design_export` pointing at its committed export \
                             also works.",
                            d.name,
                            d.graph_id.as_deref().unwrap_or("")
                        ),
                    });
                }
                continue;
            };
            watched += 1;

            let loc = at.location();
            let finding = |kind: &'static str, detail: String| UpstreamFinding {
                kind,
                dependency: d.id.clone(),
                name: d.name.clone(),
                design_export: matches!(at, WatchedAt::Export(_)).then(|| loc.to_string()),
                design_address: matches!(at, WatchedAt::Address(_)).then(|| loc.to_string()),
                detail,
            };
            // The failing party's own words, appended after a space, or nothing.
            let why = |o: &ObservedUpstream| {
                o.detail
                    .as_deref()
                    .filter(|w| !w.trim().is_empty())
                    .map(|w| format!(" {w}"))
                    .unwrap_or_default()
            };

            let Some(o) = by_id.get(d.id.as_str()) else {
                let detail = match at {
                    WatchedAt::Export(_) => format!(
                        "Nobody looked at {loc} on this pass, so this says nothing about whether \
                         '{}' has moved.",
                        d.name
                    ),
                    // Not a failure: `loop_status` deliberately does not go over
                    // the network, and says so rather than implying agreement.
                    WatchedAt::Address(_) => format!(
                        "'{}' is watched at {loc}. A design at an address is read only by \
                         `upstream_status`, which goes over the network, and this pass did not, \
                         so it says nothing about whether '{}' has moved.",
                        d.name, d.name
                    ),
                };
                findings.push(finding("not_observed", detail));
                continue;
            };

            match o.state.as_str() {
                "missing" => findings.push(finding(
                    "missing",
                    format!(
                        "'{}' is declared to be watched at {loc} and there is no file there. \
                         Either the upstream moved its record or the pointer is wrong.",
                        d.name
                    ),
                )),
                "unreachable" => findings.push(finding(
                    "unreachable",
                    format!(
                        "The server at {loc} could not be reached, so whether '{}' has moved is \
                         UNKNOWN, not unchanged.{}",
                        d.name,
                        why(o)
                    ),
                )),
                "refused" => findings.push(finding(
                    "refused",
                    format!(
                        "The server at {loc} would not show '{}', so whether it has moved is \
                         UNKNOWN, not unchanged.{}",
                        d.name,
                        why(o)
                    ),
                )),
                "unreadable" => findings.push(finding(
                    "unreadable",
                    match at {
                        WatchedAt::Export(_) => format!(
                            "What is at {loc} is not a readable reflow2 export, so '{}' cannot be \
                             watched from there.",
                            d.name
                        ),
                        WatchedAt::Address(_) => format!(
                            "The server at {loc} answered, but not with a reflow2 design, so '{}' \
                             cannot be watched there.{}",
                            d.name,
                            why(o)
                        ),
                    },
                )),
                "read" => {
                    // A target that holds a DIFFERENT design is worth more than a
                    // moved hash: the two designs would otherwise be compared
                    // forever and always disagree. Checked before movement, and
                    // only when both sides said which design they mean.
                    if let (Some(want), Some(got)) = (stated(&d.graph_id), o.graph_id.as_deref())
                        && want != got
                    {
                        let what = match at {
                            WatchedAt::Export(_) => format!("the export at {loc}"),
                            WatchedAt::Address(_) => format!("the design served at {loc}"),
                        };
                        findings.push(finding(
                            "graph_id_mismatch",
                            format!(
                                "'{}' is declared against design '{want}' but {what} belongs to \
                                 '{got}'. Watching it would compare two different designs and \
                                 always disagree.",
                                d.name
                            ),
                        ));
                        continue;
                    }
                    let Some(baseline) = baseline else {
                        findings.push(finding(
                            "never_seen",
                            format!(
                                "{loc} is readable but this design has never recorded what it \
                                 looked like, so movement cannot be computed. Re-declare '{}' to \
                                 take the baseline.",
                                d.name
                            ),
                        ));
                        continue;
                    };
                    let found = o.content_hash.as_deref().unwrap_or_default();
                    if found == baseline {
                        findings.push(finding(
                            "unchanged",
                            format!(
                                "'{}' is exactly as this design last saw it{}.",
                                d.name,
                                seen_at
                                    .map(|a| format!(", on {a}"))
                                    .unwrap_or_else(|| ", on a date nobody recorded".into())
                            ),
                        ));
                    } else {
                        findings.push(finding(
                            "moved",
                            format!(
                                "'{}' HAS MOVED since this design last looked{}. It is pinned at \
                                 version '{}'. Read what changed before assuming that pin still \
                                 describes it, then re-declare to take a new baseline — nothing \
                                 here updates it for you.",
                                d.name,
                                seen_at.map(|a| format!(" on {a}")).unwrap_or_default(),
                                d.version
                            ),
                        ));
                    }
                }
                // No silent fallback: a state this check does not know is
                // reported, never read as a successful read.
                other => findings.push(finding(
                    "unreadable",
                    format!(
                        "An observation of '{}' at {loc} came back in state '{other}', which this \
                         check does not know, so it is reported rather than read as agreement.",
                        d.name
                    ),
                )),
            }
        }

        let moved = findings.iter().filter(|f| f.kind == "moved").count();
        let unknown = findings
            .iter()
            .filter(|f| {
                matches!(
                    f.kind,
                    "missing" | "unreadable" | "unreachable" | "refused" | "not_observed"
                )
            })
            .count();
        // THE RELATION, per member (slice 1 of
        // req:a-hub-records-each-members-relation-and-a-cross-design-ripple-follows-it).
        // Reported whether or not the member is watched: a hosted design cannot
        // watch another, and its members still need a relation for a ripple to
        // follow. Never defaulted — the relation is the person's to state.
        let mut members = Vec::new();
        let mut unlinked = 0usize;
        for d in declared.iter() {
            let Some(graph_id) = stated(&d.graph_id) else {
                continue;
            };
            let finding = |kind: &'static str, detail: String| UpstreamFinding {
                kind,
                dependency: d.id.clone(),
                name: d.name.clone(),
                design_export: None,
                design_address: None,
                detail,
            };
            if d.relation.is_empty() {
                unlinked += 1;
                findings.push(finding(
                    "relation_not_stated",
                    format!(
                        "'{}' (design {graph_id}) does not say how it stands to this design, so a \
                         change in one cannot be followed into the other. Ask the person whether \
                         it is PART OF this design or a PEER this design USES (or both), and for \
                         a peer which interfaces it crosses; record it with external_dependency \
                         `relation` and `interfaces`.",
                        d.name
                    ),
                ));
            } else if d.relation.iter().any(|r| r == "uses") {
                let missing: Vec<&str> = d
                    .interfaces
                    .iter()
                    .map(String::as_str)
                    .filter(|i| !matches!(self.get_node(node::INTERFACE, i), Ok(Some(_))))
                    .collect();
                if d.interfaces.is_empty() || !missing.is_empty() {
                    unlinked += 1;
                    let detail = if d.interfaces.is_empty() {
                        format!(
                            "'{}' is recorded as a peer this design USES, and the pin names no \
                             interface the use crosses, so a ripple has nothing to follow into \
                             it. Name them in `interfaces` and mirror each one here \
                             (`mirror_surface`, the link-projects skill).",
                            d.name
                        )
                    } else {
                        format!(
                            "'{}' is used across {}, and this design has no Interface {}, so a \
                             ripple cannot follow the use into it. Mirror the member's surface \
                             here (`mirror_surface`, the link-projects skill).",
                            d.name,
                            d.interfaces.join(", "),
                            missing.join(", ")
                        )
                    };
                    findings.push(finding("no_interface_to_follow", detail));
                }
            }
            members.push(MemberRelation {
                dependency: d.id.clone(),
                name: d.name.clone(),
                graph_id: graph_id.to_string(),
                relation: if d.relation.is_empty() {
                    vec!["not stated".to_string()]
                } else {
                    d.relation.clone()
                },
                interfaces: d.interfaces.clone(),
            });
        }

        let note = if declared.is_empty() {
            "Nothing is declared, so nothing can be watched. This is \"nobody has said\", never \
             \"depends on nothing\"."
                .to_string()
        } else if watched == 0 {
            format!(
                "{designs_declared} dependency(ies) name another reflow2 design and NONE names \
                 anywhere to watch it (an address or an export), so this report is silent for want \
                 of a target rather than because nothing moved."
            )
        } else if moved == 0 && unknown > 0 {
            format!(
                "{watched} upstream design(s) watched. None has been SEEN to move, and {unknown} \
                 could not be read on this pass, so for those nothing is known either way."
            )
        } else if moved == 0 {
            format!(
                "{watched} upstream design(s) watched, none moved since this design last looked. \
                 Findings that say no baseline exists are listed rather than counted as agreement."
            )
        } else {
            format!("{moved} of {watched} watched upstream design(s) have moved.")
        };

        // LINKS INTO OTHER DESIGNS (crate::crosslink). A link is checked only
        // against what this design can say about the far design: its pin's
        // baseline and what was observed on this pass. A link into a design
        // nothing here declares cannot be checked at all, and says so.
        let references = self.design_references()?;
        for r in &references {
            let finding = |kind: &'static str, detail: String| UpstreamFinding {
                kind,
                dependency: r.id.clone(),
                name: r.name.clone(),
                design_export: None,
                design_address: None,
                detail,
            };
            let Some(pin) = declared
                .iter()
                .find(|d| d.graph_id.as_deref() == Some(r.design.as_str()))
            else {
                findings.push(finding(
                    "link_into_undeclared_design",
                    format!(
                        "'{}' links into design {}, which this design does not declare, so \
                         nothing here can say whether {} still exists or has changed. Declare \
                         that design with external_dependency (its graph_id, and where to watch \
                         it).",
                        r.name, r.design, r.node_id
                    ),
                ));
                continue;
            };
            let Some(now) = by_id
                .get(pin.id.as_str())
                .and_then(|o| o.content_hash.as_deref())
            else {
                continue;
            };
            match r.fingerprint_at_link.as_deref() {
                Some(then) if then != now => findings.push(finding(
                    "link_far_end_moved",
                    format!(
                        "Design {} has changed since the link to {} ('{}') was made. Re-read {} \
                         there; if the link still holds, make it again to acknowledge, and if \
                         not, remove it.",
                        r.design, r.node_id, r.name, r.node_id
                    ),
                )),
                Some(_) => {}
                None => findings.push(finding(
                    "link_unbaselined",
                    format!(
                        "The link to {} in design {} was made before this design recorded a \
                         baseline of that design, so whether it moved since cannot be told. Make \
                         the link again to take one.",
                        r.node_id, r.design
                    ),
                )),
            }
        }

        // INTENT ANOTHER DESIGN SENT HERE (crate::flowdown): what waits on this
        // design's owner, one finding per sending design.
        let received = self.received_intents()?;
        let sent = self.sent_intents()?;
        let mut waiting: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for r in received.iter().filter(|r| r.status == "proposed") {
            waiting
                .entry(r.from_design.as_str())
                .or_default()
                .push(r.requirement_id.as_str());
        }
        for (from, ids) in &waiting {
            findings.push(UpstreamFinding {
                kind: "received_waiting",
                dependency: format!("received:{from}"),
                name: format!("sent from {from}"),
                design_export: None,
                design_address: None,
                detail: format!(
                    "{} requirement(s) sent from design {from} wait on this design's owner: {}. \
                     Accept each (set_requirement_status) or drop it; until then it does not \
                     bind here.",
                    ids.len(),
                    ids.join(", ")
                ),
            });
        }

        let note = if unlinked > 0 {
            format!(
                "{note} {unlinked} member design(s) are linked by nothing a cross-design ripple \
                 could follow: see the relation_not_stated and no_interface_to_follow findings."
            )
        } else {
            note
        };

        Ok(UpstreamReport {
            findings,
            members,
            references,
            received,
            sent,
            designs_declared,
            watched,
            note,
        })
    }
}
