//! THE RETIREMENT RULE (inert skeleton: types only, so the tests compile and
//! fail first).

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Text that is `&'static str` in the registry and owned when read back.
pub type Text = Cow<'static, str>;

/// What a client can call or read, and so what can be retired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Item {
    Tool { tool: Text },
    Argument { tool: Text, path: Text },
    ReplyField { tool: Text, path: Text },
    NodeType { node_type: Text },
    EdgeType { edge_type: Text },
    NodeProperty { node_type: Text, property: Text },
}

/// One deprecation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deprecation {
    pub item: Item,
    pub deprecated_in: Text,
    pub what_to_do: Text,
    pub decision: Text,
}

impl Deprecation {
    const fn new(
        item: Item,
        deprecated_in: &'static str,
        what_to_do: &'static str,
        decision: &'static str,
    ) -> Self {
        Self {
            item,
            deprecated_in: Cow::Borrowed(deprecated_in),
            what_to_do: Cow::Borrowed(what_to_do),
            decision: Cow::Borrowed(decision),
        }
    }

    pub const fn tool(
        tool: &'static str,
        deprecated_in: &'static str,
        what_to_do: &'static str,
        decision: &'static str,
    ) -> Self {
        Self::new(
            Item::Tool {
                tool: Cow::Borrowed(tool),
            },
            deprecated_in,
            what_to_do,
            decision,
        )
    }

    pub const fn argument(
        tool: &'static str,
        path: &'static str,
        deprecated_in: &'static str,
        what_to_do: &'static str,
        decision: &'static str,
    ) -> Self {
        Self::new(
            Item::Argument {
                tool: Cow::Borrowed(tool),
                path: Cow::Borrowed(path),
            },
            deprecated_in,
            what_to_do,
            decision,
        )
    }

    pub const fn reply_field(
        tool: &'static str,
        path: &'static str,
        deprecated_in: &'static str,
        what_to_do: &'static str,
        decision: &'static str,
    ) -> Self {
        Self::new(
            Item::ReplyField {
                tool: Cow::Borrowed(tool),
                path: Cow::Borrowed(path),
            },
            deprecated_in,
            what_to_do,
            decision,
        )
    }

    pub const fn node_type(
        node_type: &'static str,
        deprecated_in: &'static str,
        what_to_do: &'static str,
        decision: &'static str,
    ) -> Self {
        Self::new(
            Item::NodeType {
                node_type: Cow::Borrowed(node_type),
            },
            deprecated_in,
            what_to_do,
            decision,
        )
    }

    pub const fn edge_type(
        edge_type: &'static str,
        deprecated_in: &'static str,
        what_to_do: &'static str,
        decision: &'static str,
    ) -> Self {
        Self::new(
            Item::EdgeType {
                edge_type: Cow::Borrowed(edge_type),
            },
            deprecated_in,
            what_to_do,
            decision,
        )
    }

    pub const fn node_property(
        node_type: &'static str,
        property: &'static str,
        deprecated_in: &'static str,
        what_to_do: &'static str,
        decision: &'static str,
    ) -> Self {
        Self::new(
            Item::NodeProperty {
                node_type: Cow::Borrowed(node_type),
                property: Cow::Borrowed(property),
            },
            deprecated_in,
            what_to_do,
            decision,
        )
    }
}

/// The registry.
pub const REGISTRY: &[Deprecation] = &[];

/// Inert.
pub fn used_by<'a>(
    _entries: &'a [Deprecation],
    _tool: &str,
    _arguments: Option<&Value>,
    _reply: Option<&Value>,
) -> Vec<&'a Deprecation> {
    Vec::new()
}

/// Inert.
pub fn validate(
    _entries: &[Deprecation],
    _served: &[rmcp::model::Tool],
    _vocabulary: &Value,
) -> Vec<String> {
    Vec::new()
}

/// Inert.
pub fn changelog_problems(
    _entries: &[Deprecation],
    _fragments: &[(String, String)],
    _changelog: &str,
    _current_version: &str,
) -> Vec<String> {
    Vec::new()
}
