//! The `on_disk` test program: every test file of this crate whose tests open
//! a design store in a folder — a store that survives a restart, the on-disk
//! format and its version stamp, the identity file, the store's lock, a
//! store's import and export, the shared daemon, and the real binary serving a
//! real design. Compiled and linked as ONE program; see `in_memory.rs` for why.
//!
//!     cargo test -p reflow2-mcp --test on_disk <file>::     # one file's tests
//!
//! IT NEEDS THE `rocksdb` FEATURE (`required-features` in Cargo.toml), which
//! default features turn on: `cargo test -p reflow2-mcp`, `cargo full-test`
//! and CI's full job run it; `cargo dev-test` and `--no-default-features` skip
//! it whole, because a build without on-disk storage refuses every store in a
//! folder (`opening::refuse_without_on_disk_storage`). A file belongs here when
//! its tests fail to build, or fail with that refusal, without the feature.
//!
//! ⚠️ THESE TESTS SHARE ONE PROCESS, as in `in_memory.rs`.

mod a_code_folder_attaches_to_the_design_it_names;
mod a_degraded_server_recovers_when_its_cause_clears;
mod a_held_designs_read_is_as_true_as_the_live_one;
mod a_hubs_address_serves_a_view_over_its_members;
mod a_one_shot_open_reports_its_version_verdict;
mod a_read_only_server_with_no_store_serves_the_latent_surface;
mod a_registry_names_every_store_it_cannot_serve;
mod a_server_closes_designs_nobody_is_using;
mod a_server_drains_before_it_stops;
mod a_signature_is_the_callers_own_through_every_writer;
mod a_spawned_daemon_inherits_nothing_it_should_not;
mod a_store_an_older_reflow2_wrote_is_rechecked;
mod a_stores_identity_follows_its_real_path;
mod an_exposed_server_verifies_bearer_tokens;
mod design_identity;
mod loop_status_says_what_is_not_exported;
mod the_remote_client_measures_the_folders_own_files;
mod the_server_keeps_the_export_current;
mod two_designs_stay_apart_over_the_transport;
