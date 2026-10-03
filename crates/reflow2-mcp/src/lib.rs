//! reflow2-mcp — the agent-native MCP surface for Reflow 2.0 (surface-plan.md SP-3).
//!
//! Library half of the crate: [`service::ReflowService`] is the MCP tool surface
//! over a single reflow2 design graph. The `reflow2-mcp` binary (`main.rs`) is a
//! thin stdio entry point over it; integration tests drive the service directly.

pub mod arguments;
pub mod auto_export;
pub mod bearer;
pub mod bulk_edges;
pub mod caller;
pub mod client_setup;
pub mod content_policy;
pub mod degraded;
pub mod drain;
pub mod drawn_edges;
pub mod dto;
pub mod enum_schema;
pub mod export_write;
pub mod git;
pub mod handshake;
pub mod host_gate;
pub mod latent;
pub mod lessons;
pub mod mcp_http;
pub mod measure;
pub mod nudge;
pub mod one_shot;
pub mod pointer;
pub mod prose_currency;
pub mod proxy;
pub mod read_only_client;
pub mod readiness;
pub mod receipt;
pub mod registry;
pub mod registry_http;
pub mod reply_budget;
pub mod service;
pub mod settles;
pub mod shared;
pub mod skills;
pub mod sync_debt;
pub mod tool_listing;
pub mod tools;
pub mod upstream;
pub mod usage;
pub mod wall_check;
pub mod writers;
