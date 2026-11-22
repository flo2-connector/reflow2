//! How long `loop_status` may take, and the question its check points ask —
//! and, below ([`ViewBudget`]), how long a read through a hub's address that
//! fans out over its designs may take.
//!
//! **Why it exists.** `loop_status` on reflow2's own design (8,696 nodes) took
//! 38.1 s on flo2.io's hardware on 2026-11-07, and the gateway in front of it
//! gives up at 30 s, so the call every session is told to make answered
//! nothing at all, twenty times in twelve days
//! (`fact:root-cause-loop-status-on-reflow2s-design-takes-38-s-on-flo2-io-past-its-30-s-door-gaps-and-defects-each-about-half-2026-11-07`).
//! The class fix, accepted that day
//! (`req:loop-status-answers-within-a-time-budget-and-names-the-sections-it-did-not-compute`),
//! is a budget: the call computes its floor and then its whole-design analyses
//! cheapest first, and when the time is gone it answers with what it has and
//! names the rest (`reflow2_core::LoopStatus::not_computed`).
//!
//! **The default is 20 s, two thirds of the door** (`DEFAULT_MS`). The clock
//! starts when the CALL arrives (`crate::service::call_started`), before the
//! handler waits for the design's lock, so a wait behind a writer is spent from
//! the budget rather than added to it — and a hub's view, which runs one call
//! on the hub and then on each member in turn, spends ONE budget across all of
//! them rather than a fresh one per design. The
//! other 10 s of flo2.io's 30 are for what this clock cannot see: opening a
//! design the server does not hold open yet (3.8 s for opening plus a trivial
//! call on that hardware, measured the same day), the stretch from the last
//! check point to the end of the analysis it stops (the analyses check
//! between their detectors), and the reply's serialisation and transport.
//!
//! **Three ways to set it, nearest wins:** the call's own `time_budget_ms`;
//! the server's `REFLOW2_LOOP_STATUS_BUDGET_MS` (an operator serving behind a
//! different door); this default. `0` is a budget of nothing: the floor alone.
//!
//! **And a caller who has gone stops it too**, whatever the budget: the same
//! check points ask whether rmcp has cancelled the request
//! ([`crate::service::CallerGone`]).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use reflow2_core::stop::StopReason;
use serde_json::{Value, json};

/// The budget a `loop_status` call gets when neither it nor the server names one.
pub const DEFAULT_MS: u64 = 20_000;

/// The server-wide budget, in milliseconds, for an operator whose door is not
/// flo2.io's 30 s.
pub const ENV: &str = "REFLOW2_LOOP_STATUS_BUDGET_MS";

/// Where a call's budget came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetBy {
    /// The call's own `time_budget_ms`.
    Argument,
    /// The server's [`ENV`].
    Server,
    /// [`DEFAULT_MS`].
    Default,
}

impl SetBy {
    fn as_str(self) -> &'static str {
        match self {
            SetBy::Argument => "argument",
            SetBy::Server => "server",
            SetBy::Default => "default",
        }
    }
}

/// The server's budget: [`ENV`] when it holds a whole number of milliseconds,
/// else [`DEFAULT_MS`]. Read once per process. A value that is not a number is
/// said on stderr once and ignored, never read as zero.
fn server_budget() -> (u64, SetBy) {
    static READ: std::sync::OnceLock<(u64, SetBy)> = std::sync::OnceLock::new();
    *READ.get_or_init(|| read_budget_env(ENV, DEFAULT_MS, "loop_status"))
}

/// A budget from the environment variable `env`, or `default` — what `who`
/// then keeps, said on stderr once — when it is not a whole number of
/// milliseconds.
fn read_budget_env(env: &str, default: u64, who: &str) -> (u64, SetBy) {
    match std::env::var(env) {
        Ok(v) => match v.trim().parse::<u64>() {
            Ok(ms) => (ms, SetBy::Server),
            Err(_) => {
                eprintln!(
                    "reflow2: {env}={v:?} is not a whole number of milliseconds; {who} keeps \
                     its default budget of {default} ms"
                );
                (default, SetBy::Default)
            }
        },
        Err(_) => (default, SetBy::Default),
    }
}

/// One call's budget, started when the handler starts.
pub struct CallBudget {
    started: Instant,
    budget: Duration,
    set_by: SetBy,
    gone: Option<crate::service::CallerGone>,
    /// Set the first time a check point is told to stop, so the log says it once.
    told: Arc<AtomicBool>,
}

impl CallBudget {
    /// Start the clock for one `loop_status` call. `asked` is its
    /// `time_budget_ms`.
    pub fn start(asked: Option<u64>) -> Self {
        let (ms, set_by) = match asked {
            Some(ms) => (ms, SetBy::Argument),
            None => server_budget(),
        };
        CallBudget {
            // The CALL's arrival when one is being served (`call_started`), so
            // a hub's view spends one budget across its designs.
            started: crate::service::call_started().unwrap_or_else(Instant::now),
            budget: Duration::from_millis(ms),
            set_by,
            gone: crate::service::caller_gone(),
            told: Arc::new(AtomicBool::new(false)),
        }
    }

    /// No budget at all, only the caller: for a read that has no budget of
    /// its own and still stops when nobody is waiting (`detect_gaps`,
    /// `detect_defects`).
    pub fn caller_only() -> Self {
        CallBudget {
            started: Instant::now(),
            budget: Duration::MAX,
            set_by: SetBy::Default,
            gone: crate::service::caller_gone(),
            told: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Run `work` with this budget asked at every check point it reaches
    /// (`reflow2_core::stop`).
    pub fn run<T>(&self, work: impl FnOnce() -> T) -> T {
        let started = self.started;
        let budget = self.budget;
        let gone = self.gone.clone();
        let told = self.told.clone();
        reflow2_core::stop::within(
            move |at| {
                let elapsed = started.elapsed();
                // Where the time goes, at debug: RUST_LOG=reflow2_mcp::time_budget=debug.
                tracing::debug!(at, elapsed_ms = elapsed.as_millis() as u64, "check point");
                let reason = if gone.as_ref().is_some_and(|g| g()) {
                    Some(StopReason::CallerGone)
                } else if elapsed >= budget {
                    Some(StopReason::Budget)
                } else {
                    None
                };
                if let Some(r) = reason
                    && !told.swap(true, Ordering::Relaxed)
                {
                    tracing::info!(
                        at,
                        elapsed_ms = elapsed.as_millis() as u64,
                        budget_ms = budget.as_millis() as u64,
                        why = r.as_str(),
                        "a long read stopped at a check point"
                    );
                }
                reason
            },
            work,
        )
    }

    /// The reply's `time_budget` block: what the call was given, what it
    /// spent, and whether the answer is whole.
    pub fn block(&self, not_computed: &[reflow2_core::NotComputed]) -> Value {
        let mut b = json!({
            "budget_ms": self.budget.as_millis() as u64,
            "set_by": self.set_by.as_str(),
            "elapsed_ms": self.started.elapsed().as_millis() as u64,
            "complete": not_computed.is_empty(),
        });
        if !not_computed.is_empty() {
            b["note"] = json!(format!(
                "This answer is PARTIAL: {} of the {} whole-design analyses were not computed \
                 within the budget (or the caller gave up), named in `not_computed` with the call \
                 that computes each alone. Their counts read null, never zero, and the answer is \
                 not clean. The floor and the analyses before them were computed in full.",
                not_computed.len(),
                reflow2_core::LOOP_SECTIONS.len()
            ));
        }
        b
    }
}

// ─── A HUB'S FAN-OUT READ ───────────────────────────────────────────────────
//
// `req:a-hub-wide-read-answers-within-its-time-budget-and-names-the-designs-it-did-not-reach`,
// answering
// `fact:root-cause-a-hub-wide-read-after-a-quiet-spell-reopens-and-reindexes-every-member-one-at-a-time-2026-11-22`:
// a read through a hub's address runs on the hub and then on each member in
// turn, and a member closed for idleness is reopened (its full-text index
// rebuilt) inside that turn. A cold search over 29 designs on flo2.io took a
// median of 13.5 s and four took past the 30 s door, losing the whole answer
// although most designs had been reached. The view now spends ONE budget
// across its designs ([`ViewBudget`]) and answers with the designs it reached
// when it runs out, naming the rest (`service::view`).

/// The budget a read through a hub's address that fans out over its designs
/// gets when the server names none: 25 s, five below flo2.io's 30 s door.
///
/// Why five and not loop_status's ten: this budget is a DEADLINE, not a
/// question asked at check points. When it passes, the view stops waiting at
/// once — an open still under way, a member's call still running — so what
/// is left after it is only building the merged reply (milliseconds), the
/// synchronous stretch of a member's call that cannot be interrupted between
/// two awaits (a search on an open design: well under a second), and the
/// reply's way back through the gateway. And it is five ABOVE loop_status's
/// own [`DEFAULT_MS`], so loop_status through a hub, which stops its
/// analyses at 20 s on every design, still has time to answer its floor on
/// the design it is on.
pub const VIEW_DEFAULT_MS: u64 = 25_000;

/// The server-wide budget of a hub's fan-out read, in milliseconds, for an
/// operator whose door is not flo2.io's 30 s. Set it with [`ENV`]: the two
/// are read apart, and loop_status through a hub stops at the smaller.
pub const VIEW_ENV: &str = "REFLOW2_HUB_VIEW_BUDGET_MS";

/// The view's budget: [`VIEW_ENV`] when it holds a whole number of
/// milliseconds, else [`VIEW_DEFAULT_MS`]. Read once per process.
fn view_server_budget() -> (u64, SetBy) {
    static VIEW_READ: std::sync::OnceLock<(u64, SetBy)> = std::sync::OnceLock::new();
    *VIEW_READ.get_or_init(|| read_budget_env(VIEW_ENV, VIEW_DEFAULT_MS, "a hub's fan-out read"))
}

/// One fan-out read's budget through a hub's address, started when the CALL
/// arrived (`crate::service::call_started`), so the hub's own turn is spent
/// from it as every member's is.
pub struct ViewBudget {
    started: Instant,
    budget: Duration,
    set_by: SetBy,
}

impl ViewBudget {
    /// Start the clock for one fan-out. `asked` is the call's own
    /// `time_budget_ms`, for a tool that takes one (`loop_status`): the view
    /// then allows that plus the margin the two defaults keep
    /// ([`VIEW_DEFAULT_MS`] − [`DEFAULT_MS`]), so a caller who raises or
    /// lowers loop_status's budget moves the view's with it. Any other tool
    /// takes the server's budget.
    pub fn start(asked: Option<u64>) -> Self {
        let (ms, set_by) = match asked {
            Some(ms) => (
                ms.saturating_add(VIEW_DEFAULT_MS - DEFAULT_MS),
                SetBy::Argument,
            ),
            None => view_server_budget(),
        };
        ViewBudget {
            started: crate::service::call_started().unwrap_or_else(Instant::now),
            budget: Duration::from_millis(ms),
            set_by,
        }
    }

    /// The moment the view stops waiting.
    pub fn deadline(&self) -> tokio::time::Instant {
        let at = self
            .started
            .checked_add(self.budget)
            // A budget too large to add is no deadline at all: a century.
            .unwrap_or_else(|| Instant::now() + Duration::from_secs(100 * 365 * 24 * 3600));
        tokio::time::Instant::from_std(at)
    }

    /// Whether the budget is gone: a design not begun by then is not begun.
    pub fn spent(&self) -> bool {
        self.started.elapsed() >= self.budget
    }

    /// The reply's `time_budget` block for a fan-out that did not reach
    /// `not_reached` of its `designs` designs: loop_status's block, in its
    /// words, saying the answer is partial.
    pub fn block(&self, not_reached: usize, designs: usize) -> Value {
        json!({
            "budget_ms": self.budget.as_millis() as u64,
            "set_by": self.set_by.as_str(),
            "elapsed_ms": self.started.elapsed().as_millis() as u64,
            "complete": not_reached == 0,
            "note": format!(
                "This answer is PARTIAL: {not_reached} of the {designs} designs of this view were \
                 not reached in time — the hub's time budget ran out first — and are named in \
                 `not_reached`, each with the call that reaches it alone. Nothing they hold is \
                 in this answer, and no count here covers them. Every other design answered in \
                 full. An open already begun goes on, so the same call made again may reach \
                 more."
            ),
        })
    }
}
