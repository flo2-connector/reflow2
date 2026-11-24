//! The root of reflow2-core's `on_disk` test program (Cargo.toml): every file
//! in tests/ whose whole body needs the `rocksdb` feature is a module of it.
//!
//! 🛑 NOT tests/every_open_store_shares_one_memory_budget.rs. That test reads a
//! budget the FIRST store a process opens fixes for the whole process, so it
//! must be the only test in its program, and it has a program of its own
//! (`store_memory_budget`). Until 2026-11-23 it was this program's root: two
//! modules declared there that day opened stores before it set its budget, and
//! it failed on every run with its program's tests together
//! (fact:root-cause-the-memory-budget-test-shared-its-process-once-modules-were-declared-in-its-file-2026-11-23).

#![cfg(feature = "rocksdb")]

mod a_store_is_copied_while_it_stays_open;
mod an_open_times_its_own_steps;
