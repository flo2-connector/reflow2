//! A NULL UNSETS, AT EVERY TYPED CONSTRUCTOR — the 2026-09-07 rule, extended
//! (`dec:null-unsets-on-every-typed-constructor`, Anthony 2026-10-24: "null
//! unsets everywhere").
//!
//! # The failure
//!
//! The Shop sample trial (2026-10-24) sent `limit: null` to `add_constraint`
//! to clear a numeric limit, and the limit stayed. A typed request takes an
//! optional field as an `Option`, and serde reads an explicit `null` and an
//! absent key as the same `None`, so the constructor saw "not given" and its
//! merge kept the stored value. The 2026-09-07 rule ("a null UNSETS rather
//! than being refused", `chg:eight-field-feedback-fixes-land-for-v0510`) had
//! been built into `create_node`'s props bag only: a rule settled at one door
//! and never reached the others.
//!
//! # The cure
//!
//! A typed constructor takes its request as [`Nullable<T>`], which reads the
//! arguments once as a JSON object, keeps the names of the fields sent as an
//! explicit `null`, and then reads the typed request from the same object.
//! The published schema is `T`'s, unchanged. After the constructor's own
//! writes, [`unset_nulled`] removes every such field that is a property the
//! schema declares on the node's type, under the call's one write unit:
//!
//! - an explicit `null` on an optional property UNSETS it (a property with a
//!   schema default reads its default again, as at `create_node`);
//! - a field left out keeps its stored value, exactly as before;
//! - a `null` on a REQUIRED property is refused by name, and the call's unit
//!   discards everything it staged, so nothing is written. Required means the
//!   schema's `required`, or a field the constructor needs to create the node
//!   (`RequiredFields::refuse_nulled`, before anything is written): unsetting
//!   `DesignEpoch.sequence` would leave a node its own constructor could not
//!   revise;
//! - a `null` on a property the schema calls ADDITIVE history
//!   (`distinct_from`) or on a `status` is refused by name and writes
//!   nothing; a status refusal names the `set_*_status` tool that moves it
//!   (Anthony's ruling, 2026-10-25: "Go with your recommendation on null
//!   unsets");
//! - a `null` on a field that is not a property (an edge to draw, `echo`,
//!   `approver`) means what leaving it out means — there is no stored value
//!   to unset.
//!
//! The reply lists what was unset under `unset`, as `create_node`'s does.
//!
//! `tests/a_null_unsets_at_every_typed_constructor.rs` drives every served
//! `add_*` tool (and `plan_epoch`) over every optional property its schema
//! publishes, and fails on a constructor it has no row for — so a new
//! constructor cannot skip the rule unnoticed.

use reflow2_core::{DesignGraph, StoredNode};
use rmcp::ErrorData as McpError;
use serde::Deserialize;
use serde::de::{DeserializeOwned, Deserializer, Error as _};

use crate::dto::NodeDto;
use crate::service::dyno_err;

/// A typed constructor's request, with the names of the fields the caller
/// sent as an explicit `null` beside it.
///
/// Its JSON schema IS `T`'s (name and body), so wrapping a request changes
/// nothing a client sees.
#[derive(Debug, Clone)]
pub struct Nullable<T> {
    /// The typed request, read exactly as `T` reads it (a `null` is `None`).
    pub req: T,
    /// The top-level fields sent as an explicit `null`, sorted.
    pub nulled: Vec<String>,
}

impl<T> Nullable<T> {
    /// A request with nothing sent as null — for a caller inside the server
    /// that builds the typed request itself.
    pub fn plain(req: T) -> Self {
        Self {
            req,
            nulled: Vec::new(),
        }
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for Nullable<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let map = serde_json::Map::<String, serde_json::Value>::deserialize(d)?;
        let mut nulled: Vec<String> = map
            .iter()
            .filter(|(_, v)| v.is_null())
            .map(|(k, _)| k.clone())
            .collect();
        nulled.sort();
        // The typed request reads the same object, so its own refusals (an
        // unknown field, a wrong type) keep their words.
        let req = T::deserialize(serde_json::Value::Object(map)).map_err(D::Error::custom)?;
        Ok(Self { req, nulled })
    }
}

impl<T: schemars::JsonSchema> schemars::JsonSchema for Nullable<T> {
    fn inline_schema() -> bool {
        T::inline_schema()
    }

    fn schema_name() -> std::borrow::Cow<'static, str> {
        T::schema_name()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        T::schema_id()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        T::json_schema(generator)
    }
}

/// Unset, on `node_type`/`id`, every field in `nulled` that the schema
/// declares as a property of that type, and say which.
///
/// Called by a typed constructor AFTER its own writes, so nothing the
/// constructor merges can put the value back. A required property is refused
/// by name (`DesignGraph::remove_properties`), and the call's write unit then
/// discards the whole call. A field that is not a property is passed over: a
/// null there means what omitting it means. Writes only when one of the
/// properties is actually stored, so a null on a field already unset moves
/// nothing.
pub(crate) fn unset_nulled(
    g: &mut DesignGraph,
    node_type: &str,
    id: &str,
    nulled: &[String],
) -> Result<Vec<String>, McpError> {
    if nulled.is_empty() {
        return Ok(Vec::new());
    }
    let declared: Vec<String> = match g.schema().node_types.get(node_type) {
        Some(def) => nulled
            .iter()
            .filter(|k| def.properties.contains_key(k.as_str()))
            .cloned()
            .collect(),
        None => Vec::new(),
    };
    if declared.is_empty() {
        return Ok(declared);
    }
    refuse_history_and_status(g, node_type, &declared)?;
    let required: Vec<&String> = declared
        .iter()
        .filter(|k| {
            g.schema()
                .node_types
                .get(node_type)
                .and_then(|d| d.properties.get(k.as_str()))
                .is_some_and(|p| p.required)
        })
        .collect();
    let stored = g.get_node(node_type, id).map_err(dyno_err)?;
    let present = stored
        .as_ref()
        .is_some_and(|n| declared.iter().any(|k| n.properties.contains_key(k)));
    if present || !required.is_empty() {
        // remove_properties refuses a required key by name before it writes.
        g.remove_properties(node_type, id, &declared)
            .map_err(dyno_err)?;
    }
    Ok(declared)
}

/// The tool that moves a status on `node_type`, where one is served.
fn status_setter(node_type: &str) -> Option<&'static str> {
    match node_type {
        "Requirement" => Some("set_requirement_status"),
        "Capability" => Some("set_capability_status"),
        "Decision" => Some("set_decision_status"),
        "Verification" => Some("set_verification_status"),
        "DesignEpoch" => Some("set_epoch_status"),
        _ => None,
    }
}

/// Anthony's ruling, 2026-10-25 ("Go with your recommendation on null
/// unsets"): a null never ERASES HISTORY and never silently RESETS A STATUS.
/// A property the schema calls ADDITIVE (`distinct_from`: a judgement is
/// history) and a `status` are refused by name; nothing is written, because
/// the call's unit discards it. A status refusal names the tool that moves it.
fn refuse_history_and_status(
    g: &DesignGraph,
    node_type: &str,
    declared: &[String],
) -> Result<(), McpError> {
    let Some(def) = g.schema().node_types.get(node_type) else {
        return Ok(());
    };
    for k in declared {
        let additive = def
            .properties
            .get(k.as_str())
            .and_then(|p| p.description.as_deref())
            .is_some_and(|d| d.contains("ADDITIVE"));
        if additive {
            return Err(McpError::invalid_params(
                format!(
                    "`{k}` on {node_type} is history and a null cannot erase it: the schema \
                     keeps it ADDITIVE. Omit the field to keep it. Nothing was written."
                ),
                None,
            ));
        }
        if k == "status" {
            let how = match status_setter(node_type) {
                Some(set) => format!("move it with `{set}`"),
                None => "re-send this call with the status you mean".to_string(),
            };
            return Err(McpError::invalid_params(
                format!(
                    "`status` on {node_type} cannot be unset: a null never resets a status to \
                     its default. To change it, {how}; omit the field to keep it. Nothing was \
                     written."
                ),
                None,
            ));
        }
    }
    Ok(())
}

/// [`unset_nulled`], then the node as it now stands with `unset` filled in —
/// for a constructor whose reply is the node itself. `stored` is what the
/// constructor's last write returned, used when nothing was unset.
pub(crate) fn node_after_unset(
    g: &mut DesignGraph,
    node_type: &str,
    id: &str,
    nulled: &[String],
    stored: StoredNode,
) -> Result<NodeDto, McpError> {
    let unset = unset_nulled(g, node_type, id, nulled)?;
    let node = if unset.is_empty() {
        stored
    } else {
        g.get_node(node_type, id)
            .map_err(dyno_err)?
            .unwrap_or(stored)
    };
    let mut dto = NodeDto::from(node);
    dto.unset = unset;
    Ok(dto)
}

impl<T> From<T> for Nullable<T> {
    /// A typed request built in code carries no explicit null.
    fn from(req: T) -> Self {
        Self::plain(req)
    }
}
