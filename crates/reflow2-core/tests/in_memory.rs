//! The `in_memory` test program: every test file of this crate that needs no
//! whole-file feature gate, compiled and linked as ONE program.
//!
//! WHY ONE PROGRAM. Until 2026-10-15 each file in `tests/` was its own test
//! program: ~225 of them, each ~22 MB in a debug build without features (or
//! ~50 MB with fulltext, ~271 MB with RocksDB), each linking the whole crate
//! and every dependency again — and cargo keeps every old copy. Each file now
//! stays exactly where it was (the design's artifacts, checks and docs name
//! these paths) and is a module here, so its tests are named
//! `<file>::<test>`:
//!
//!     cargo test -p reflow2-core --test in_memory <file>::     # one file's tests
//!
//! THIS PROGRAM BUILDS WITH WHATEVER FEATURES THE USER PASSES. Tests that are
//! individually gated on `#[cfg(feature = "...")]` compile out when the feature
//! is absent, which is correct: `cargo dev-test` (no features) runs the
//! no-feature side, and a featured build runs the featured side.
//!
//! A file whose ENTIRE body is gated — `#![cfg(feature = "...")]` at the crate
//! level — belongs in its own `[[test]]` program with `required-features`
//! instead, so cargo skips it whole rather than compiling an empty module.
//!
//! ⚠️ THESE TESTS SHARE ONE PROCESS, on its threads. A file that changes
//! process-wide state — an environment variable, the current directory, a
//! one-way latch — or needs a process of its own is its own program in
//! Cargo.toml, not a module here.
//!
//! ADD A NEW TEST FILE HERE (or to one of the feature-gated programs): with
//! `autotests = false`, a file declared in no program compiles nowhere and runs
//! nowhere. `every_test_file_runs_in_exactly_one_program` fails until it is
//! declared.

mod a_budget_adds_only_numbers_in_its_own_unit;
mod a_budget_says_how_it_composes;
mod a_capability_can_state_what_it_takes_and_gives;
mod a_change_ripples_across_uses_and_up_part_of_in_three_designs;
mod a_check_that_could_not_run_is_blocked;
mod a_choice_is_not_a_musing;
mod a_claim_carries_its_age;
mod a_claim_is_not_hidden_by_its_parent;
mod a_claimed_check_is_counted_apart_from_a_runnable_one;
mod a_conclusion_shows_the_axioms_it_rests_on;
mod a_dangling_reference_is_found_whatever_its_degree;
mod a_deliberate_state_is_not_a_defect;
mod a_deliberately_open_edge_declares_what_it_is_for;
mod a_derived_scan_is_computed_once_per_write;
mod a_design_closes_against_a_declared_threshold;
mod a_design_says_what_fraction_carries_a_relation;
mod a_design_says_what_it_is_for;
mod a_design_says_whether_a_result_ever_came_back;
mod a_design_that_publishes_contracts_publishes_intent;
mod a_detector_consults_what_the_graph_already_holds;
mod a_documentation_change_has_a_name;
mod a_drawn_relation_is_read_back_as_a_sentence;
mod a_fields_own_terms_have_one_home;
mod a_file_realizes_only_what_it_implements;
mod a_finding_says_when_the_state_is_normal_for_the_phase;
mod a_first_baseline_is_read_as_one;
mod a_fix_says_whether_it_corrected_the_cause;
mod a_floating_root_part_is_told_the_cure;
mod a_follow_up_is_owed_until_it_is_settled;
mod a_hub_member_records_its_relation;
mod a_jot_carries_the_tag_its_word_gave_it;
mod a_link_into_another_design_is_recorded_and_checked;
mod a_mirror_can_be_refreshed;
mod a_node_records_what_the_user_calls_it;
mod a_parked_report_that_produced_nothing_is_reported;
mod a_procedural_boundary_has_a_word;
mod a_project_declares_its_unit_system_and_the_sweep_reports;
mod a_project_is_asked_what_its_artifacts_must_be_in;
mod a_proof_counts_what_it_should;
mod a_proof_says_what_it_assumes;
mod a_property_that_names_a_node_is_guarded;
mod a_published_promise_carries_its_evidence;
mod a_quantity_says_how_it_was_obtained;
mod a_question_is_answerable_by_either_id;
mod a_question_names_whom_it_was_put_to_and_who_answered;
mod a_recalled_reason_is_not_read_as_one_written_at_the_time;
mod a_record_can_say_which_question_it_answers;
mod a_recorded_questions_wording_is_immutable;
mod a_refusal_names_what_would_have_worked;
mod a_relation_suggestion_weighs_real_neighbours_and_compares_ideas_with_requirements;
mod a_repair_can_say_what_it_invalidated;
mod a_repos_know_how_is_delivered_at_the_step_it_bears_on;
mod a_requirement_linked_to_no_other_intent_is_noticed;
mod a_rule_tests_the_condition_its_message_names;
mod a_run_is_fed_back_by_file;
mod a_schema_narrowing_ships_its_migration;
mod a_scoped_answer_actually_narrows;
mod a_session_reports_what_it_did_by_hand;
mod a_session_with_no_seed_can_find_one;
mod a_settled_choice_can_be_gone_back_to;
mod a_settled_decision_has_a_change_type;
mod a_signature_is_the_callers_own_where_it_is_written;
mod a_stored_item_the_schema_now_refuses_is_reported;
mod a_stored_twin_has_one_authority;
mod a_superseded_finding_does_not_read_as_current;
mod a_taxonomy_is_decided_once_before_bulk_capture;
mod a_unit_crosses_a_seam_and_the_two_sides_are_compared;
mod a_vocabulary_change_is_never_silent;
mod a_write_is_credited_to_who_it_was_for;
mod a_write_records_the_agent_it_went_through;
mod a_write_says_what_it_did_not_recognise;
mod a_write_unit_is_all_or_nothing;
mod absence_means_nobody_said_and_a_plan_holds_a_check;
mod acknowledgement_not_structure;
mod adopt_is_incremental_with_a_frontier_and_a_deferred_derivation_marker;
mod agent;
mod allocate;
mod allocation;
mod alternatives;
mod an_acknowledgement_says_whose;
mod an_approval_does_not_erase_authorship;
mod an_edge_says_only_what_it_was_told;
mod an_edge_says_whether_anyone_checked;
mod an_export_says_where_it_was_taken;
mod an_inference_says_how_premises_support_a_conclusion;
mod an_upstream_design_is_watched_not_imported;
mod artifact;
mod artifact_intent;
mod awaiting_the_owners_word;
mod bad_news_reaches_the_decision_behind_it;
mod budget;
mod built_work_reads_as_built;
mod bulk;
mod candidates_are_offered_never_drawn;
mod change_axis_unstated;
mod change_event_carries_its_own_text;
mod change_type_splits_its_axis;
mod changelog;
mod checksum_dialect;
mod claims;
mod closure_reads_a_design_phase_and_a_build_phase;
mod coherence_loop;
mod compare;
mod component_verified;
mod compose;
mod confirm;
mod confirmation_vocabulary;
mod constructors_preserve;
mod consumed_by_nothing;
mod contradiction_alignment;
mod corpus;
mod coverage;
mod cycles;
mod debt_since_the_record;
mod decision_overtaken_by_promotion;
mod defect_overtaken_by_change;
mod delivery;
mod delivery_kind;
mod depends;
mod derived_relations_are_declared_and_checked;
mod detect;
mod dimensions;
mod discontinued_is_read;
mod discover;
mod drift;
mod drift_direction;
mod edge_prose_is_governed_like_node_prose;
mod edges_need_both_ends;
mod enforced_rule_needs_a_detector;
mod every_edge_type_declares_what_it_means;
mod every_finding_that_reads_parking_names_it;
mod every_prose_field_is_searched;
mod every_rule_a_project_parent_cures_says_so;
mod every_test_file_runs_in_exactly_one_program;
mod evidence;
mod evidence_has_somewhere_to_go;
mod evidence_quality;
mod export;
mod fielded;
mod fix_without_recorded_cause;
mod flow;
mod follow_through_reads_one_arrival;
mod gap_review;
mod genesis;
mod genesis_islands_are_the_plan_not_a_defect;
mod golden_thread;
mod granularity;
mod graph_construction_is_setup;
mod heal;
mod hierarchy;
mod hydrate_cost_from_the_document_alone;
mod ility;
mod ingest;
mod ingest_handshake;
mod intent_flows_down_by_move_piece_or_derived_and_binds_on_acceptance;
mod interface;
mod interface_spec;
mod internal_only_delivery;
mod key_interfaces;
mod kpp;
mod linking_report;
mod llm;
mod lost_update;
mod maturity;
mod merge;
mod mirror_surface;
mod move_component;
mod no_fabricated_repairs;
mod nothing_asks_what_a_session_made_false;
mod one_nodes_edges_are_read_back_as_edges;
mod orphan_attachment;
mod persistence;
mod preserve;
mod prior_state_coverage;
mod prohibition_in_prose;
mod propagate;
mod proposed_asserts_nothing;
mod provenance;
mod publishable_promise;
mod readiness;
mod record_change_can_state_its_axis;
mod record_to_record_relations;
mod registering_the_file_that_is_a_check_gives_it_an_executable_form;
mod reply_budget;
mod report;
mod requirement_lineage;
mod review_relations;
mod reviewed_defects;
mod sanitize;
mod scheduling_a_question;
mod schema_defaults;
mod scope;
mod seam;
mod seam_coverage;
mod search;
mod seat_liveness_shared;
mod serialize;
mod sibling_merge;
mod stale_seat;
mod structural;
mod structure_with_no_function_is_asked_about;
mod surface_export;
mod surface_says_what_it_orphaned;
mod surprises;
mod sweep_coverage;
mod temporal;
mod the_committed_design_round_trips_through_the_item_layout;
mod the_compliance_layer_is_whole;
mod the_confirmation_ledger_reads_each_history_once;
mod the_design_is_saved_one_file_per_item;
mod the_design_says_whose_words_to_use;
mod the_engine_proposes_a_merge_and_a_person_picks_the_survivor;
mod the_schema_serves_the_rules_that_tell_the_types_apart;
mod the_settle_predicate_has_one_answer;
mod the_surface_says_what_it_already_knew;
mod understated_status;
mod unexported_changes_say_whose_they_are;
mod unreachable_decision;
mod unverified_published_contract;
mod upsert;
mod verifies_reaches_a_constraint;
mod verifies_reaches_an_interface;
mod verify_drift;
mod vocabulary_coverage_reports_what_was_never_used;
mod what_next;
mod write_side;
