//! The server's async runtime, with a stack chosen for its threads.
//!
//! EVERY TOOL CALL RUNS ON ONE OF THIS RUNTIME'S THREADS. rmcp runs each
//! request as a task on a worker, over HTTP (`--http`, `--registry-root`), on
//! stdio, and through `--call`, whose in-process server is spawned onto it
//! too; the main thread only awaits. Until 2026-10-26 the runtime came from
//! `#[tokio::main]`, which leaves the thread stack unset, so every worker got
//! std's default, 2 MiB, a size nobody chose. On a release build a reasoning
//! chain about 2,000 steps deep overflowed it and the whole process aborted
//! (`fact:a-release-server-overflows-its-2-mib-worker-stack-on-a-reasoning-chain-about-2000-steps-deep-and-the-whole-process-aborts-2026-10-25`).
//!
//! The CAUSE is fixed where it was: the walks that recursed once per step keep
//! their own work stack now (`foundations_walk`, `proof_chain_walk` in
//! reflow2-core), and `tests/a_deep_reasoning_chain_is_answered_on_a_2_mib_worker.rs`
//! proves them on a 2 MiB worker. This stack is the second half of
//! `dec:deep-reasoning-walks-keep-their-own-stack-and-the-runtime-gets-headroom`
//! (Anthony, 2026-10-26: "Go with 1 + 3"): HEADROOM, for the recursion that is
//! bounded but deep and for frames nobody has measured.

/// The stack each of the server's runtime threads gets: 16 MiB.
///
/// WHY 16 MiB, measured on this branch (2026-10-26):
/// - The deepest recursion left is the truth evaluation, bounded at
///   `INFERENCE_DEPTH` (64) steps. On a release build it fits in 2 MiB. On a
///   DEBUG build it needs between 4 and 6 MiB for a plain chain past 64 steps
///   (overflowed at 4 MiB, answered at 6 MiB), and the Shop-shaped design of
///   #801's test overflowed 2 MiB well before its 56 steps. 16 MiB is about
///   three times the largest need measured, in the build that needs most.
/// - Nothing else in reflow2-core or reflow2-mcp recurses along links a
///   person writes without a bound (the audit in this change), so no design
///   can grow its way past this. What remains deep is bounded: the truth
///   evaluation (64), `design_trace_gap_within` and
///   `requirement_delivered_by_within` (64), and walks over a JSON value,
///   whose nesting serde_json caps at 128 levels on every way in.
///
/// WHAT IT COSTS. A thread's stack is ADDRESS SPACE reserved when the thread
/// starts; memory is used only for the pages a call actually touches, and a
/// shallow call touches a few kilobytes, as before. The runtime starts one
/// worker per core, plus blocking threads on demand (opening a store, stdio),
/// which exit after ten idle seconds. On a 4-core host that is 64 MiB of
/// address space for the workers and 16 MiB per blocking thread while it
/// lives; resident memory moves only on a call that recurses deep. The flo2.io
/// host has 7.9 GB shared by several services, so the cost that matters is
/// resident, and that is unchanged for every call that was not about to
/// overflow.
pub const SERVER_THREAD_STACK: usize = 16 << 20;

/// The multi-threaded runtime the server runs on — the one `#[tokio::main]`
/// built, with each thread's stack set to `thread_stack` bytes. `main` passes
/// [`SERVER_THREAD_STACK`].
pub fn server_runtime(thread_stack: usize) -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(thread_stack)
        .build()
}
