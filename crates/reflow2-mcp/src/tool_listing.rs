//! The one place a `tools/list` reply is built for an overridden listing.
//!
//! rmcp's `#[tool_handler]` generates `list_tools` with a protocol-version rule
//! written INLINE in the macro (rmcp-macros `tool_handler.rs`): on 2026-07-28 and
//! later the result must carry `ttlMs` and `cacheScope` (SEP-2549, "Required by
//! spec version 2026-07-28"), and an older peer gets neither. There is no rmcp
//! helper to call, so a handler that overrides `list_tools` has to reproduce the
//! rule — and reflow2 overrides it twice: the full surface (lessons on tools) and
//! the latent surface (the two-tool listing in a folder with no design).
//!
//! Until 2026-09-26 each override carried its own copy, and only one had the rule.
//! Claude Code 2.1.283, the first client here to open on 2026-07-28, refused the
//! latent listing ("Invalid result for tools/list: ttlMs … cacheScope") — so
//! /genesis could not start a design from the machine-wide entry
//! (fact:root-cause-the-latent-tool-list-omits-the-cache-fields-the-2026-07-28-spec-requires-2026-09-26).
//! Every override builds its reply HERE, and
//! tests/every_tool_list_carries_the_cache_hints_its_protocol_requires.rs checks
//! the output of every surface under both protocol versions.

use rmcp::model::{CacheScope, ListToolsResult, ProtocolVersion, ResultType, Tool};
use rmcp::service::{RequestContext, RoleServer};

/// A complete `tools/list` result for this peer: the cache hints when it
/// negotiated 2026-07-28 or later (`ttlMs` 0 — re-list whenever the surface
/// changes — and `public`, the values rmcp's generated listing uses), none for
/// an older peer.
pub(crate) fn tools_result(
    tools: Vec<Tool>,
    context: &RequestContext<RoleServer>,
) -> ListToolsResult {
    let cache_hints = context
        .protocol_version()
        .is_some_and(|v| v >= ProtocolVersion::V_2026_07_28);
    ListToolsResult {
        result_type: Some(ResultType::COMPLETE),
        tools,
        meta: None,
        next_cursor: None,
        ttl_ms: cache_hints.then_some(0),
        cache_scope: cache_hints.then_some(CacheScope::Public),
    }
}
