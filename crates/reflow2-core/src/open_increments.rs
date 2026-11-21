//! OPEN INCREMENTS: every increment that was planned and never closed, oldest
//! first, with how much of what was scheduled for it is delivered and which
//! signs it shows of having been started and never finished.
//!
//! Two requirements, accepted 2026-11-14: a planned increment is reported
//! when a signal holds, each computed from edges and statuses alone; and
//! `loop_status`, `graph_report` and the where-am-i skill list every open
//! increment until it is closed. An INCREMENT is a `DesignEpoch` whose
//! `status` reads `planned`.
//!
//! # The signals
//!
//! - **done but not closed**: something is scheduled for it, nothing
//!   scheduled for it is outstanding, and it still reads planned.
//! - **overtaken**: an epoch it PRECEDES, directly or down a chain of
//!   PRECEDES edges, has arrived while it still reads planned. The DRAWN
//!   order only: `sequence` is never read, and where no order was drawn
//!   nothing is said (the owner's ruling, 2026-11-14).
//! - **empty**: nothing still intended is scheduled for it, so it can never
//!   be judged. Reported even when the increment only orders others; the row
//!   says how many epochs it precedes and is preceded by.
//!
//! # Nothing is called stalled
//!
//! The fourth signal (nothing scheduled for it has moved over the design's
//! last N recorded changes) waits on the owner picking N. Until then each
//! open increment carries the COUNT that N would be read against: how many
//! of the design's most recent recorded changes have passed since one named
//! anything scheduled for it. An item MOVED when a ChangeEvent names it among
//! what it CHANGED, or names something that delivers it: what SATISFIES it,
//! or a file that REALIZES it or realizes what satisfies it. No threshold is
//! applied and no finding is raised. Where the count cannot be worked out it
//! reads `never`, with the reason, and not a number.
//!
//! # No clock
//!
//! No date enters a signal. The count is a number of changes, never an age.
//! The only times read are the store's own creation stamps, and only to
//! ORDER things: which change is newer, which increment is older. A record
//! the store kept no creation time for (written before it kept one) is older
//! than every record it did; the answer counts those, and a count that rests
//! on one is a floor and says so.
//!
//! # One computation
//!
//! [`DesignGraph::open_increments`] is the only place any of this is worked
//! out, kept until the next write. `loop_status` and `graph_report` read its
//! bounded view ([`OpenIncrements::listed`]); how much is delivered is
//! `arrival_delta`'s own reading of each scheduled item
//! (`DesignGraph::scheduled_now`), never a second derivation of delivery.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Serialize, Serializer};

use crate::foundation::core::{DynoError, Value};
use crate::foundation::store::StoredNode;
use crate::graph::DesignGraph;
use crate::nodes::{edge, node};
use crate::temporal::ScheduleOutcome;

/// How many open increments `loop_status` lists in full, and `graph_report`
/// names by id, oldest first. The counts beside the list are always the whole
/// design's; `graph_report` with `include_open_increments` lists every one in
/// full. reflow2's own design held 31 on 2026-11-14, and a reply that listed
/// them all inside `loop_status` would crowd out the debt it exists to say.
pub const OPEN_INCREMENTS_LISTED: usize = 10;

/// Everything still intended for it is delivered, and it still reads planned.
pub const DONE_BUT_NOT_CLOSED: &str = "done_but_not_closed";
/// An epoch it precedes has arrived while it still reads planned.
pub const OVERTAKEN: &str = "overtaken";
/// Nothing still intended is scheduled for it.
pub const EMPTY: &str = "empty";

/// The longest name the bounded list shows whole. Under the 80 characters at
/// which a reply that must fit a budget starts cutting strings, so a listed
/// row is never cut a second time, silently, on its way out.
const NAME_CHARS: usize = 76;

/// How to read the list, said with it.
const READS: &str = "Oldest first. `changes_since_it_moved` counts the design's recorded changes, \
     newest first, back to the last one that named anything scheduled for the increment, or \
     something that delivers it; `never` means none did. Nothing here is called stalled.";

/// Is this increment OPEN: planned, and not yet closed?
///
/// ⭐ THE ONE PLACE THIS IS DECIDED. An increment closes by arriving or, since
/// 2026-11-17, by being closed honestly without arriving (`close_increment`:
/// deferred, dropped, superseded), so open is still exactly `status:
/// planned`. A deferred increment is closed for now and counted apart
/// (`deferred`); reopening it makes it `planned`, and open, again.
fn increment_is_open(epoch: &StoredNode) -> bool {
    epoch.properties.get("status").and_then(Value::as_str) == Some("planned")
}

/// Has this epoch HAPPENED? Absent reads as arrived, as
/// [`DesignGraph::epoch_is_planned`] reads it. Asked on its own, and not as
/// "not open", so that a status that is neither stays neither.
fn epoch_has_arrived(epoch: &StoredNode) -> bool {
    epoch
        .properties
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("arrived")
        == "arrived"
}

/// How many recorded changes have passed since anything scheduled for an
/// increment moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinceMoved {
    /// This many changes were recorded after the last one that named it. `0`:
    /// the most recent change did.
    Changes(usize),
    /// No recorded change names anything scheduled for it, or nothing is
    /// scheduled for it. Written `"never"`, so it is never read as a number.
    Never,
}

impl Serialize for SinceMoved {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            SinceMoved::Changes(n) => s.serialize_u64(*n as u64),
            SinceMoved::Never => s.serialize_str("never"),
        }
    }
}

/// One open increment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpenIncrement {
    /// The planned epoch.
    pub epoch_id: String,
    /// Its name. Cut in the bounded list when long (`name_cut`).
    pub name: String,
    /// The name above was cut short; the whole list carries it whole.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub name_cut: bool,
    /// How many items are scheduled for it now.
    pub scheduled: usize,
    /// How many of those are delivered, as `arrival_delta` reads each.
    pub delivered: usize,
    /// How many of those were withdrawn (a question somebody withdrew): no
    /// longer intended, so neither delivered nor outstanding.
    #[serde(skip_serializing_if = "is_zero")]
    pub withdrawn: usize,
    /// Which unfinished signals it shows: `done_but_not_closed`, `overtaken`,
    /// `empty`. Empty when it shows none.
    pub signals: Vec<&'static str>,
    /// For `overtaken`: the nearest arrived epoch it precedes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overtaken_by: Option<String>,
    /// How many of the design's recorded changes, newest first, have passed
    /// since one named anything scheduled for it or something that delivers
    /// it. `"never"` when none did.
    pub changes_since_it_moved: SinceMoved,
    /// The count above is a FLOOR: the last change that named it has no
    /// recorded time, so it is older than every change that has one, and how
    /// many of the other changes without one came after it is not known.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub or_more: bool,
    /// Why the count reads `never`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why_never: Option<&'static str>,
    /// How many epochs it PRECEDES, directly.
    pub precedes: usize,
    /// How many epochs precede it, directly. With `precedes`, what tells an
    /// increment that only orders others from one that was forgotten.
    pub preceded_by: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

impl OpenIncrement {
    /// The row in a sentence, for a report a person reads.
    pub fn line(&self) -> String {
        let mut s = format!(
            "`{}` {}: {} of {} scheduled delivered",
            self.epoch_id, self.name, self.delivered, self.scheduled
        );
        if self.withdrawn > 0 {
            s.push_str(&format!(", {} withdrawn", self.withdrawn));
        }
        for signal in &self.signals {
            match *signal {
                DONE_BUT_NOT_CLOSED => s.push_str("; done but not closed"),
                OVERTAKEN => s.push_str(&format!(
                    "; overtaken ({} has arrived)",
                    self.overtaken_by.as_deref().unwrap_or("a later epoch")
                )),
                _ => s.push_str(&format!(
                    "; empty (it precedes {} epoch(s), {} precede it)",
                    self.precedes, self.preceded_by
                )),
            }
        }
        match self.changes_since_it_moved {
            SinceMoved::Changes(n) => s.push_str(&format!(
                "; {n}{} change(s) since it moved",
                if self.or_more { " or more" } else { "" }
            )),
            SinceMoved::Never => s.push_str("; never moved"),
        }
        s
    }
}

/// Every open increment, with the counts over all of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpenIncrements {
    /// How many increments are open: planned, and never closed.
    pub open: usize,
    /// Of those, how many show `done_but_not_closed`.
    pub done_but_not_closed: usize,
    /// Of those, how many show `overtaken`.
    pub overtaken: usize,
    /// Of those, how many show `empty`.
    pub empty: usize,
    /// Of those, how many show none of the three. An increment can show two
    /// (empty and overtaken), so the four do not sum to `open`.
    pub no_signal: usize,
    /// Of those, how many read `never` for `changes_since_it_moved`.
    pub never_moved: usize,
    /// How many increments were DEFERRED by their owner (`close_increment`):
    /// still meant, later, and closed for now, so neither open nor listed.
    /// Counted apart so a deferral is never mistaken for work that is done or
    /// gone. `set_epoch_status planned` reopens one. Absent when none is, so a
    /// design that never deferred one reads exactly as it did before.
    #[serde(skip_serializing_if = "is_zero")]
    pub deferred: usize,
    /// How many changes the design has recorded: what the counts run over.
    pub changes_recorded: usize,
    /// Of those, how many the store kept no creation time for. They are older
    /// than every change it did keep one for.
    pub changes_with_no_recorded_time: usize,
    /// How many open increments the store kept no creation time for. They are
    /// listed first, as the oldest, by id among themselves.
    pub increments_with_no_recorded_time: usize,
    /// The increments, oldest first by when the store recorded each one's
    /// creation. All of them from [`DesignGraph::open_increments`]; the oldest
    /// few from [`listed`](Self::listed).
    ///
    /// ⚠️ NOTHING HERE SAYS HOW MANY ROWS THE LIST HOLDS, on purpose. A reply
    /// that must fit a budget withholds rows from this list like any other
    /// (measured 2026-11-14 on reflow2's own design: `loop_status` kept one
    /// row of ten), and a number written before that cut would then be wrong.
    /// `open` is the whole count; the rows are counted by reading them.
    pub increments: Vec<OpenIncrement>,
    /// How to read the list. Absent when nothing is open.
    #[serde(skip_serializing_if = "str::is_empty")]
    pub reads: &'static str,
    /// The call that lists every one, when some are not listed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_list: Option<String>,
}

impl OpenIncrements {
    /// The bounded view a report carries: the `at_most` oldest, names cut to
    /// a line, every count still the whole design's, and the call that lists
    /// the rest named.
    pub fn listed(&self, at_most: usize) -> OpenIncrements {
        let mut view = self.clone();
        view.increments.truncate(at_most);
        for i in &mut view.increments {
            if let Some(cut) = cut_name(&i.name) {
                i.name = cut;
                i.name_cut = true;
            }
        }
        if self.open > view.increments.len() {
            view.full_list = Some(format!(
                "graph_report with include_open_increments: true lists all {}.",
                self.open
            ));
        }
        view
    }

    /// The overview's view, which `graph_report`'s JSON carries unless every
    /// one is asked for: every count, the `at_most` oldest BY ID ONLY, and
    /// where each is in full.
    ///
    /// Anthony, 2026-11-18, option A
    /// (dec:graph-reports-default-names-open-increments-by-id-and-count-2026-11-18):
    /// the rows say what `loop_status` already says at a session's boundary,
    /// and in the overview they cost room without adding a reading. Measured
    /// that day on flo2's design under 0.100.0: 2,647 of the report's 28,728
    /// characters were its seven rows, and the report no longer fit the
    /// 28,000 flo2's chat door reads it in.
    pub fn by_id(&self, at_most: usize) -> OpenIncrementsById {
        OpenIncrementsById {
            open: self.open,
            done_but_not_closed: self.done_but_not_closed,
            overtaken: self.overtaken,
            empty: self.empty,
            no_signal: self.no_signal,
            never_moved: self.never_moved,
            deferred: self.deferred,
            changes_recorded: self.changes_recorded,
            changes_with_no_recorded_time: self.changes_with_no_recorded_time,
            increments_with_no_recorded_time: self.increments_with_no_recorded_time,
            increments: self
                .increments
                .iter()
                .take(at_most)
                .map(|i| IncrementId {
                    epoch_id: i.epoch_id.clone(),
                })
                .collect(),
            // Kept under the 80 characters below which a budgeted reply never
            // cuts a string, so the pointer survives the smallest budget: a
            // longer sentence was measured cut to "…" at budget_chars 2500.
            full_list: (self.open > 0).then(|| {
                format!(
                    "graph_report with include_open_increments: true lists all {} in full.",
                    self.open
                )
            }),
        }
    }

    /// The counts in one clause: "6 open increment(s) (planned and never
    /// closed): 2 done but not closed, 1 overtaken, 2 empty".
    pub fn counts_line(&self) -> String {
        let mut line = format!(
            "{} open increment(s) (planned and never closed): {} done but not closed, {} \
             overtaken, {} empty",
            self.open, self.done_but_not_closed, self.overtaken, self.empty
        );
        if self.deferred > 0 {
            line.push_str(&format!(
                "; {} deferred by their owner (closed for now, not listed)",
                self.deferred
            ));
        }
        line
    }
}

/// One open increment, named by id: a row of [`OpenIncrementsById`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IncrementId {
    /// The planned epoch.
    pub epoch_id: String,
}

/// Every count of [`OpenIncrements`], and the oldest few by id only — what
/// [`OpenIncrements::by_id`] gives the overview. The counts are the same
/// fields, named the same, so a reader of either reads both.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpenIncrementsById {
    /// How many increments are open: planned, and never closed.
    pub open: usize,
    /// Of those, how many show `done_but_not_closed`.
    pub done_but_not_closed: usize,
    /// Of those, how many show `overtaken`.
    pub overtaken: usize,
    /// Of those, how many show `empty`.
    pub empty: usize,
    /// Of those, how many show none of the three.
    pub no_signal: usize,
    /// Of those, how many read `never` for `changes_since_it_moved`.
    pub never_moved: usize,
    /// How many were deferred by their owner. Absent when none is.
    #[serde(skip_serializing_if = "is_zero")]
    pub deferred: usize,
    /// How many changes the design has recorded.
    pub changes_recorded: usize,
    /// Of those, how many the store kept no creation time for.
    pub changes_with_no_recorded_time: usize,
    /// How many open increments the store kept no creation time for.
    pub increments_with_no_recorded_time: usize,
    /// The oldest few, oldest first, by id. As in [`OpenIncrements`], nothing
    /// here says how many rows the list holds: `open` is the whole count.
    pub increments: Vec<IncrementId>,
    /// Where each one is in full. Absent when nothing is open.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_list: Option<String>,
}

/// `name` cut to [`NAME_CHARS`] at a word boundary, with an ellipsis; `None`
/// when it already fits.
fn cut_name(name: &str) -> Option<String> {
    if name.chars().count() <= NAME_CHARS {
        return None;
    }
    let head: String = name.chars().take(NAME_CHARS).collect();
    let kept = match head.rfind(char::is_whitespace) {
        Some(at) if at > NAME_CHARS / 2 => head[..at].trim_end(),
        _ => head.as_str(),
    };
    Some(format!("{kept}…"))
}

/// The design's recorded changes, by how recent each is.
struct ChangeOrder {
    /// Change id → (how many changes were recorded after it, whether that is
    /// a floor because the store kept no time for it).
    after: HashMap<String, (usize, bool)>,
    /// How many changes there are.
    recorded: usize,
    /// How many of them have no recorded time.
    untimed: usize,
}

impl DesignGraph {
    /// Every open increment, oldest first, with what each shows — see the
    /// module docs. Worked out once and kept until the next write.
    pub fn open_increments(&self) -> Result<OpenIncrements, DynoError> {
        self.kept("open_increments", || self.open_increments_now())
    }

    fn open_increments_now(&self) -> Result<OpenIncrements, DynoError> {
        // ONE pass over the epochs: which are open, and which have arrived.
        let epochs = self.scan_nodes(node::DESIGN_EPOCH)?;
        let arrived: HashSet<&str> = epochs
            .iter()
            .filter(|e| epoch_has_arrived(e))
            .map(|e| e.node_id.as_str())
            .collect();
        let created: HashMap<String, Option<String>> = self
            .created_stamps(node::DESIGN_EPOCH)?
            .into_iter()
            .collect();
        // Oldest first by when the store recorded each one. One it kept no
        // time for was written before it kept any, so it sorts first (`None`
        // orders before every `Some`), by id among its own kind.
        let mut open: Vec<(Option<&str>, &StoredNode)> = epochs
            .iter()
            .filter(|e| increment_is_open(e))
            .map(|e| (created.get(&e.node_id).and_then(|c| c.as_deref()), e))
            .collect();
        open.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.node_id.cmp(&b.1.node_id)));

        let mut report = OpenIncrements {
            open: open.len(),
            done_but_not_closed: 0,
            overtaken: 0,
            empty: 0,
            no_signal: 0,
            never_moved: 0,
            deferred: epochs
                .iter()
                .filter(|e| e.properties.get("status").and_then(Value::as_str) == Some("deferred"))
                .count(),
            changes_recorded: 0,
            changes_with_no_recorded_time: 0,
            increments_with_no_recorded_time: open.iter().filter(|(c, _)| c.is_none()).count(),
            increments: Vec::with_capacity(open.len()),
            reads: "",
            full_list: None,
        };
        if open.is_empty() {
            // Nothing is open: the changes are not read at all.
            return Ok(report);
        }
        report.reads = READS;

        // ONE pass over the changes, and one id→type index for every
        // schedule read below.
        let order = self.changes_by_recency()?;
        report.changes_recorded = order.recorded;
        report.changes_with_no_recorded_time = order.untimed;
        let index = self.node_type_index()?;
        // What the last change to name each watched record was, looked up
        // once however many increments schedule it.
        let mut last_named: HashMap<String, Option<(usize, bool)>> = HashMap::new();

        for (_, epoch) in open {
            let id = epoch.node_id.as_str();
            let items = self.scheduled_now(&index, id)?;
            let scheduled = items.len();
            let delivered = items
                .iter()
                .filter(|i| i.outcome == ScheduleOutcome::Delivered)
                .count();
            let withdrawn = items
                .iter()
                .filter(|i| i.outcome == ScheduleOutcome::Discontinued)
                .count();
            let still_intended = scheduled - withdrawn;

            let mut signals = Vec::new();
            if still_intended > 0 && delivered == still_intended {
                signals.push(DONE_BUT_NOT_CLOSED);
                report.done_but_not_closed += 1;
            }
            let overtaken_by = self.overtaken_by(id, &arrived)?;
            if overtaken_by.is_some() {
                signals.push(OVERTAKEN);
                report.overtaken += 1;
            }
            if still_intended == 0 {
                signals.push(EMPTY);
                report.empty += 1;
            }
            if signals.is_empty() {
                report.no_signal += 1;
            }

            let mut moved: Option<(usize, bool)> = None;
            for item in &items {
                for watched in self.what_moves(&item.item_type, &item.item_id)? {
                    let named = match last_named.get(&watched) {
                        Some(known) => *known,
                        None => {
                            let found = self.last_change_naming(&order, &watched)?;
                            last_named.insert(watched, found);
                            found
                        }
                    };
                    moved = match (moved, named) {
                        (Some(a), Some(b)) => Some(a.min(b)),
                        (a, b) => a.or(b),
                    };
                }
            }
            let (changes_since_it_moved, or_more, why_never) = match moved {
                Some((n, floor)) => (SinceMoved::Changes(n), floor, None),
                None => {
                    report.never_moved += 1;
                    let why = if scheduled == 0 {
                        "nothing is scheduled for it"
                    } else {
                        "no recorded change names anything scheduled for it, or anything that \
                         delivers it"
                    };
                    (SinceMoved::Never, false, Some(why))
                }
            };

            report.increments.push(OpenIncrement {
                epoch_id: id.to_string(),
                name: epoch
                    .properties
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                name_cut: false,
                scheduled,
                delivered,
                withdrawn,
                signals,
                overtaken_by,
                changes_since_it_moved,
                or_more,
                why_never,
                precedes: self.outgoing(id, Some(edge::PRECEDES))?.len(),
                preceded_by: self.incoming(id, Some(edge::PRECEDES))?.len(),
            });
        }
        Ok(report)
    }

    /// The nearest ARRIVED epoch `start` precedes, down any chain of PRECEDES
    /// edges; `None` when there is none. Breadth first with neighbours in id
    /// order, so the answer is the same every time, and each epoch is visited
    /// once, so a cycle ends.
    fn overtaken_by(
        &self,
        start: &str,
        arrived: &HashSet<&str>,
    ) -> Result<Option<String>, DynoError> {
        let mut seen: HashSet<String> = HashSet::from([start.to_string()]);
        let mut queue: VecDeque<String> = VecDeque::from([start.to_string()]);
        while let Some(at) = queue.pop_front() {
            let mut later: Vec<String> = self
                .outgoing(&at, Some(edge::PRECEDES))?
                .into_iter()
                .map(|e| e.to_id)
                .collect();
            later.sort();
            for next in later {
                if !seen.insert(next.clone()) {
                    continue;
                }
                if arrived.contains(next.as_str()) {
                    return Ok(Some(next));
                }
                queue.push_back(next);
            }
        }
        Ok(None)
    }

    /// Every recorded change, ranked by how many were recorded after it:
    /// newest first by the store's creation stamp, ties by id. A change with
    /// no recorded time is older than every change with one; its rank is the
    /// number of those, as a floor.
    fn changes_by_recency(&self) -> Result<ChangeOrder, DynoError> {
        let mut timed: Vec<(String, String)> = Vec::new();
        let mut untimed: Vec<String> = Vec::new();
        for (id, created) in self.created_stamps(node::CHANGE_EVENT)? {
            match created {
                Some(at) => timed.push((at, id)),
                None => untimed.push(id),
            }
        }
        timed.sort_by(|a, b| b.cmp(a));
        let floor = timed.len();
        let mut order = ChangeOrder {
            after: HashMap::with_capacity(floor + untimed.len()),
            recorded: floor + untimed.len(),
            untimed: untimed.len(),
        };
        for (rank, (_, id)) in timed.into_iter().enumerate() {
            order.after.insert(id, (rank, false));
        }
        for id in untimed {
            order.after.insert(id, (floor, true));
        }
        Ok(order)
    }

    /// The records whose change MOVES a scheduled item: the item itself, what
    /// SATISFIES it (a requirement's capabilities), and the files that
    /// REALIZE it or realize what satisfies it.
    fn what_moves(&self, item_type: &str, item_id: &str) -> Result<Vec<String>, DynoError> {
        let mut watched = vec![item_id.to_string()];
        for file in self.incoming(item_id, Some(edge::REALIZES))? {
            watched.push(file.from_id);
        }
        if item_type == node::REQUIREMENT {
            for by in self.incoming(item_id, Some(edge::SATISFIES))? {
                for file in self.incoming(&by.from_id, Some(edge::REALIZES))? {
                    watched.push(file.from_id);
                }
                watched.push(by.from_id);
            }
        }
        Ok(watched)
    }

    /// The most recent change that names `id` among what it CHANGED: how many
    /// changes were recorded after it, and whether that is a floor. `None`
    /// when no change names it.
    fn last_change_naming(
        &self,
        order: &ChangeOrder,
        id: &str,
    ) -> Result<Option<(usize, bool)>, DynoError> {
        Ok(self
            .incoming(id, Some(edge::CHANGED))?
            .iter()
            .filter_map(|e| order.after.get(&e.from_id).copied())
            .min())
    }
}
