//! `StorageEngine` — the server's own stamps on a node: when it was created and
//! when its content last changed, from THIS server's clock, sealed so that no
//! caller can set or move them.
//!
//! `req:reflow2-stamps-the-server-time-on-each-node-creation-and-content-change`
//! (accepted 2026-10-28), under
//! `dec:prediction-order-is-read-from-server-recorded-times-and-said-not-enforced-2026-10-28`.
//! Measured on main 73b29e4 before this existed: the store saved only what the
//! caller sent, and every dated property (`created_at`, `reported_at`,
//! `detected_at`, an AUTHORED_BY's `authored_at` from `acted_at`) was typed by
//! the caller. Nothing could say which of two records was written first.
//!
//! # Where the stamps live, and why there
//!
//! In the node's own stored property map, under three reserved names
//! ([`STAMP_KEYS`]), and STRIPPED FROM EVERY READ the engine serves
//! (`get_node`, `scan_nodes`, the index scan). So:
//! - nothing that reads, compares or echoes properties sees them — a revise's
//!   replaced fields, a receipt, a twin check, a snapshot's state, a content
//!   hash all read exactly what they read before;
//! - the store's format does not change: no new column family, so a store this
//!   build wrote still opens under an older reflow2, which reads the stamps as
//!   three extra properties and does nothing with them;
//! - an export carries them only because `export_graph` asks for them
//!   ([`StorageEngine::stamp_properties`]) and puts them back in the node's
//!   `properties`, which every older reader already accepts.
//!
//! # Why a seal, and what it proves
//!
//! An export's hash chain cannot vouch for a stamp: `content_hash` is an
//! unkeyed SHA-256 any writer can recompute after an edit, so it shows a file
//! agrees with itself, never who wrote it or when. The seal is an HMAC-SHA256,
//! under a key only this store holds, over the design, the node, both times and
//! the node's content hash. A stamp reads RECORDED HERE only when its seal
//! verifies under this store's key against the node's content as it stands.
//! Anything else is IMPORTED, NOT RECORDED HERE: a stamp copied in from another
//! store, typed into an export, or left on content that was edited outside
//! reflow2.
//!
//! The key is 32 random bytes kept INSIDE the store, under a reserved key in
//! the embeddings family that no graph's key can equal (graph ids are never
//! empty). Inside, so it travels with the store's directory — a copy taken for
//! a read, a backup, a move — and never with an export.
//!
//! # What a write does
//!
//! - A caller can never set them: a write naming one is REFUSED, on every path,
//!   because every node write reaches [`StorageEngine::create_node`].
//! - New content stamps `created` and `changed` now. Unchanged content keeps
//!   the stamps it had, so rewriting a node identically moves nothing.
//! - Changed content keeps `created` ONLY when the old stamp was recorded here
//!   (its seal verifies). An imported creation time is never laundered into one
//!   this store vouches for; it is dropped, and the node reads "created: not
//!   recorded" from then on.
//! - An import ([`StampMode::Import`]) keeps the stamps the document carries,
//!   as it carries them. Their seal decides how they read. A node the document
//!   carries none for gets none, unless its content is unchanged: an import is
//!   not the moment a statement was made.

use super::*;

/// The node's creation time, from this server's clock.
pub const STAMP_CREATED: &str = "server_created_at";
/// The time the node's content last changed, from this server's clock.
pub const STAMP_CHANGED: &str = "server_changed_at";
/// The seal over both, under this store's key.
pub const STAMP_SEAL: &str = "server_seal";
/// Every reserved name. A write that names one is refused.
pub const STAMP_KEYS: [&str; 3] = [STAMP_CREATED, STAMP_CHANGED, STAMP_SEAL];

/// Where this store's key lives: the embeddings family, under a key that
/// begins with the separator, so it is the key of no graph.
const STAMP_KEY_KEY: &[u8] = b"\x00reflow2:stamp-key:v1";

/// The domain of the seal's message, versioned.
const SEAL_DOMAIN: &str = "reflow2-stamp-v1";

/// A clock that answers the current time, UTC, as fixed-width RFC 3339 with
/// microseconds (`2026-10-28T02:31:54.123456Z`), so two stamps compare as
/// strings.
pub type StampClock = Arc<dyn Fn() -> String + Send + Sync>;

/// How a write treats stamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StampMode {
    /// Every ordinary write: stamps are the server's.
    Write,
    /// A restore from a document: the document's stamps are kept as stated.
    Import,
}

/// The stamping a store does: its key and its clock.
#[derive(Clone)]
pub(crate) struct Stamper {
    key: [u8; 32],
    clock: StampClock,
}

/// The stamps a stored node carries, read raw, and whether this store vouches
/// for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeStamp {
    /// When it was created, if that was recorded.
    pub created: Option<String>,
    /// When its content last changed.
    pub changed: Option<String>,
    /// The seal verifies under this store's key against the content as it
    /// stands: this store recorded these times.
    pub recorded_here: bool,
}

/// The system clock, in the stamp format.
pub fn system_clock() -> StampClock {
    Arc::new(|| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        format_stamp_time(now.as_secs() as i64, now.subsec_micros())
    })
}

/// Seconds since the epoch and microseconds, as `YYYY-MM-DDTHH:MM:SS.ffffffZ`.
pub fn format_stamp_time(secs: i64, micros: u32) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let date = crate::dates::civil_from_days(days);
    format!(
        "{date}T{:02}:{:02}:{:02}.{micros:06}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// HMAC-SHA256 (RFC 2104), written out: sha2 is already a dependency and the
/// construction is two hashes, so a crate for it would be a rebuild for every
/// consumer and nothing else.
fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut block = [0u8; 64];
    if key.len() > 64 {
        block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha256::new();
    inner.update(block.map(|b| b ^ 0x36));
    inner.update(message);
    let inner = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(block.map(|b| b ^ 0x5c));
    outer.update(inner);
    let mut out = [0u8; 32];
    out.copy_from_slice(&outer.finalize());
    out
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// The node's content: its properties without the stamps.
pub(crate) fn content_of(props: &HashMap<String, Value>) -> HashMap<String, Value> {
    let mut content = props.clone();
    strip(&mut content);
    content
}

/// Remove the stamps from a property map, as every read does.
pub(crate) fn strip(props: &mut HashMap<String, Value>) {
    for k in STAMP_KEYS {
        props.remove(k);
    }
}

fn stamp_str<'a>(props: &'a HashMap<String, Value>, key: &str) -> Option<&'a str> {
    props.get(key).and_then(Value::as_str)
}

impl Stamper {
    /// A stamper with this key and the system clock.
    pub(crate) fn new(key: [u8; 32]) -> Self {
        Self {
            key,
            clock: system_clock(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn seal(
        &self,
        graph_id: &str,
        node_type: &str,
        node_id: &str,
        created: Option<&str>,
        changed: Option<&str>,
        content: &HashMap<String, Value>,
    ) -> String {
        let message = format!(
            "{SEAL_DOMAIN}\0{graph_id}\0{node_type}\0{node_id}\0{}\0{}\0{}",
            created.unwrap_or(""),
            changed.unwrap_or(""),
            crate::graph::node_content_hash(content)
        );
        format!(
            "hmac-sha256:{}",
            hex(&hmac_sha256(&self.key, message.as_bytes()))
        )
    }

    /// Does the stamp on `props` verify here, against the content it carries?
    fn verifies(
        &self,
        graph_id: &str,
        node_type: &str,
        node_id: &str,
        props: &HashMap<String, Value>,
    ) -> bool {
        let Some(seal) = stamp_str(props, STAMP_SEAL) else {
            return false;
        };
        let content = content_of(props);
        let expected = self.seal(
            graph_id,
            node_type,
            node_id,
            stamp_str(props, STAMP_CREATED),
            stamp_str(props, STAMP_CHANGED),
            &content,
        );
        // Not a timing-safe comparison, and it need not be: a reader learns the
        // verdict anyway, and the seal is no secret once written.
        seal == expected
    }

    /// Put the stamps `new` should carry onto it, given what the store held.
    fn apply(
        &self,
        graph_id: &str,
        node_type: &str,
        node_id: &str,
        old: Option<&HashMap<String, Value>>,
        new: &mut HashMap<String, Value>,
        mode: StampMode,
    ) {
        if mode == StampMode::Import && STAMP_KEYS.iter().any(|k| new.contains_key(*k)) {
            // The document's stamps, kept as stated; their seal decides how
            // they read.
            return;
        }
        let unchanged = old.is_some_and(|o| content_of(o) == *new);
        if unchanged {
            if let Some(o) = old {
                for k in STAMP_KEYS {
                    if let Some(v) = o.get(k) {
                        new.insert(k.to_string(), v.clone());
                    }
                }
            }
            return;
        }
        if mode == StampMode::Import {
            // Changed or new content from a document that states no time: the
            // import is not when it was stated, so nothing is recorded.
            return;
        }
        let now = (self.clock)();
        let created = match old {
            None => Some(now.clone()),
            Some(o) if self.verifies(graph_id, node_type, node_id, o) => {
                stamp_str(o, STAMP_CREATED).map(str::to_string)
            }
            Some(_) => None,
        };
        let seal = self.seal(
            graph_id,
            node_type,
            node_id,
            created.as_deref(),
            Some(&now),
            new,
        );
        if let Some(c) = created {
            new.insert(STAMP_CREATED.into(), Value::String(c));
        }
        new.insert(STAMP_CHANGED.into(), Value::String(now));
        new.insert(STAMP_SEAL.into(), Value::String(seal));
    }
}

impl StorageEngine {
    /// Start stamping, with this store's key: read it from the store, or make
    /// one and keep it there when the store has none.
    pub fn enable_stamps(&mut self) -> Result<(), DynoError> {
        let key = match self.read_stamp_key()? {
            Some(k) => k,
            None => {
                let mut k = [0u8; 32];
                k[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
                k[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
                self.put(CF_EMBEDDINGS, STAMP_KEY_KEY.to_vec(), k.to_vec())?;
                k
            }
        };
        let clock = self.stamper.as_ref().map(|s| s.clock.clone());
        let mut stamper = Stamper::new(key);
        if let Some(c) = clock {
            stamper.clock = c;
        }
        self.stamper = Some(stamper);
        Ok(())
    }

    /// Verify stamps with this store's key, when it has one, without writing:
    /// a read-only open.
    pub fn load_stamps_read_only(&mut self) -> Result<(), DynoError> {
        if let Some(key) = self.read_stamp_key()? {
            self.stamper = Some(Stamper::new(key));
        }
        Ok(())
    }

    fn read_stamp_key(&self) -> Result<Option<[u8; 32]>, DynoError> {
        Ok(self
            .get(CF_EMBEDDINGS, STAMP_KEY_KEY)?
            .and_then(|bytes| <[u8; 32]>::try_from(&bytes[..]).ok()))
    }

    /// Replace the clock stamps are read from. For tests and nothing else: a
    /// golden reading cannot hold the wall clock. A store that is not stamping
    /// starts to, with a key of its own.
    pub fn set_stamp_clock(&mut self, clock: StampClock) -> Result<(), DynoError> {
        if self.stamper.is_none() {
            self.enable_stamps()?;
        }
        if let Some(s) = self.stamper.as_mut() {
            s.clock = clock;
        }
        Ok(())
    }

    /// Whether this store stamps its writes.
    pub fn stamps_enabled(&self) -> bool {
        self.stamper.is_some()
    }

    /// Refuse a write that names a stamp. Called on every ordinary write.
    pub(crate) fn refuse_caller_stamps(
        node_type: &str,
        props: &HashMap<String, Value>,
    ) -> Result<(), DynoError> {
        if let Some(k) = STAMP_KEYS.iter().find(|k| props.contains_key(**k)) {
            return Err(DynoError::Validation {
                node_type: node_type.to_string(),
                property: (*k).to_string(),
                message: format!(
                    "REFUSED: `{k}` is a stamp reflow2 records from its own clock when a node is \
                     created or its content changes, and a write cannot set or move it. Leave it \
                     out; date what you observed in the node's own fields (reported_at, \
                     valid_from, acted_at) instead."
                ),
            });
        }
        Ok(())
    }

    /// Stamp `new` for a write over `old` (both before stripping).
    pub(crate) fn stamp_for_write(
        &self,
        graph_id: &str,
        node_type: &str,
        node_id: &str,
        old: Option<&HashMap<String, Value>>,
        new: &mut HashMap<String, Value>,
        mode: StampMode,
    ) {
        if let Some(s) = &self.stamper {
            s.apply(graph_id, node_type, node_id, old, new, mode);
        } else if mode == StampMode::Write {
            // Not stamping: an unchanged rewrite still keeps what it had, so a
            // store opened without stamping never erases them by rewriting.
            if let Some(o) = old
                && content_of(o) == *new
            {
                for k in STAMP_KEYS {
                    if let Some(v) = o.get(k) {
                        new.insert(k.to_string(), v.clone());
                    }
                }
            }
        }
    }

    /// The node's stamps, read raw, and whether this store vouches for them.
    /// `None` when the node does not exist.
    pub fn node_stamp(
        &self,
        graph_id: &str,
        node_type: &str,
        node_id: &str,
    ) -> Result<Option<NodeStamp>, DynoError> {
        let Some(props) = self.raw_node_properties(graph_id, node_type, node_id)? else {
            return Ok(None);
        };
        let recorded_here = self
            .stamper
            .as_ref()
            .is_some_and(|s| s.verifies(graph_id, node_type, node_id, &props));
        Ok(Some(NodeStamp {
            created: stamp_str(&props, STAMP_CREATED).map(str::to_string),
            changed: stamp_str(&props, STAMP_CHANGED).map(str::to_string),
            recorded_here,
        }))
    }

    /// The stamp properties the node carries, exactly as stored — what an
    /// export writes back into the node so they travel.
    pub fn stamp_properties(
        &self,
        graph_id: &str,
        node_type: &str,
        node_id: &str,
    ) -> Result<Vec<(String, Value)>, DynoError> {
        let Some(props) = self.raw_node_properties(graph_id, node_type, node_id)? else {
            return Ok(Vec::new());
        };
        Ok(STAMP_KEYS
            .iter()
            .filter_map(|k| props.get(*k).map(|v| ((*k).to_string(), v.clone())))
            .collect())
    }

    /// A node's property map as stored, stamps included.
    pub(crate) fn raw_node_properties(
        &self,
        graph_id: &str,
        node_type: &str,
        node_id: &str,
    ) -> Result<Option<HashMap<String, Value>>, DynoError> {
        let key = crate::foundation::store::keys::node_key(graph_id, node_type, node_id);
        match self.get(CF_NODES, &key)? {
            Some(bytes) => Ok(Some(
                rmp_serde::from_slice(&bytes)
                    .map_err(|e| DynoError::Serialization(e.to_string()))?,
            )),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_matches_rfc_4231_case_2() {
        let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            hex(&mac),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn a_stamp_time_is_fixed_width_utc() {
        assert_eq!(format_stamp_time(0, 0), "1970-01-01T00:00:00.000000Z");
        assert_eq!(
            format_stamp_time(1_793_154_714, 7),
            "2026-10-28T02:31:54.000007Z"
        );
    }
}
