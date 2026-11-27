//! WHICH TOOLS A SESSION IS LISTED: the full surface, or the lite profile.
//!
//! reflow2 serves about two hundred tools, and a host that loads every tool's
//! description pays roughly 140,000 tokens a session for the list. Usage
//! counted across three designs put 97.4% of 30,439 calls on the top 40
//! tools. So the LITE profile lists 45 tools — [`LITE_TOOLS`] and
//! [`USE_ANOTHER_TOOL`] — and every other tool stays one call away through
//! `use_another_tool`, which `find_tools` (listed in both profiles, searching
//! every tool) names. Approved by the owner on 2026-11-27.
//!
//! ⭐ FULL IS THE DEFAULT AND IS UNCHANGED. A session that asks for nothing is
//! listed exactly what it was; the profile only FILTERS the listing that
//! `list_tools` already builds (lessons, the hub's `member` argument, the cache
//! hints), and changes nothing a call does. A tool lite does not list is still
//! served: called directly, it answers as it always did.
//!
//! ⭐ CHOSEN WHEN A SESSION CONNECTS, NEVER MID-SESSION. Hosts such as claude.ai
//! ignore `notifications/tools/list_changed` for remote connectors, and most
//! hosts only call tools they list, so a list that grew mid-session would grow
//! for nobody. The profile is the server's `--profile` (`REFLOW2_PROFILE`), or,
//! over HTTP, the `?profile=` parameter of the MCP endpoint URL the session
//! connected to ([`from_query`]), fixed at that session's `initialize`.

use rmcp::model::Tool;

/// The tool every profile serves, and lite lists beside [`LITE_TOOLS`]: it
/// calls any served tool by name, so an unlisted tool is one call away.
pub const USE_ANOTHER_TOOL: &str = "use_another_tool";

/// The environment variable `--profile` reads.
pub const ENV: &str = "REFLOW2_PROFILE";

/// The query parameter of the MCP endpoint URL that chooses a session's
/// profile over HTTP: `…/mcp?profile=lite`, `/g/<graph_id>/?profile=lite`.
pub const QUERY_PARAMETER: &str = "profile";

/// THE LITE LIST, in one place: the tools a lite session is listed, besides
/// [`USE_ANOTHER_TOOL`]. Ordered by how often they were called (the top 40 of
/// 30,439 calls over three designs), then the four the owner added.
/// `tests/the_lite_profile_lists_45_tools.rs` holds every name to a served
/// tool, so a renamed tool fails the tests instead of silently leaving lite.
pub const LITE_TOOLS: &[&str] = &[
    "get_node",
    "search_design",
    "replace_text",
    "export_graph",
    "add_change_event",
    "set_artifact_checksums",
    "add_artifact",
    "loop_status",
    "record_finding",
    "add_decision",
    "add_requirement",
    "add_verification",
    "get_skill",
    "add_capability",
    "link_artifact",
    "set_decision_status",
    "scan_nodes",
    "review_relations",
    "draw_edges",
    "claim_region",
    "create_edge",
    "get_instructions",
    "create_edges",
    "set_verification_status",
    "governed_by",
    "realizes",
    "release_claim",
    "set_requirement_status",
    "set_requirement_lineage",
    "set_capability_status",
    "arrival_delta",
    "describe_schema",
    "propagate_from",
    "authored_by",
    "plan_epoch",
    "mint_seat",
    "release_includes_all",
    "delete_edge",
    "add_epoch",
    "add_release",
    "find_tools",
    "graph_report",
    "detect_gaps",
    "design_identity",
];

/// Which tools a session is listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Profile {
    /// Every served tool — the default, and exactly what was listed before
    /// profiles existed.
    #[default]
    Full,
    /// [`LITE_TOOLS`] and [`USE_ANOTHER_TOOL`]; every other tool through
    /// `use_another_tool`.
    Lite,
}

impl Profile {
    /// The values `--profile`, `REFLOW2_PROFILE` and `?profile=` take.
    pub const NAMES: &'static [&'static str] = &["full", "lite"];

    /// Parse a profile name, or say what would have worked.
    pub fn parse(raw: &str) -> Result<Profile, String> {
        match raw.trim() {
            "full" => Ok(Profile::Full),
            "lite" => Ok(Profile::Lite),
            other => Err(format!(
                "`{other}` is not a reflow2 profile. A profile is `full` (every tool, the \
                 default) or `lite` ({} tools, every other one reached through `{USE_ANOTHER_TOOL}`).",
                LITE_TOOLS.len() + 1
            )),
        }
    }

    /// The profile's own name, as it is typed.
    pub fn as_str(self) -> &'static str {
        match self {
            Profile::Full => "full",
            Profile::Lite => "lite",
        }
    }

    /// Whether a session on this profile is listed `tool`.
    pub fn lists(self, tool: &str) -> bool {
        match self {
            Profile::Full => true,
            Profile::Lite => tool == USE_ANOTHER_TOOL || LITE_TOOLS.contains(&tool),
        }
    }

    /// The listing a session on this profile is given, from the full one:
    /// filtered, never rebuilt, so every decoration the full listing carries
    /// stays on every tool lite keeps. The full profile returns it untouched.
    pub fn filter(self, tools: Vec<Tool>) -> Vec<Tool> {
        match self {
            Profile::Full => tools,
            Profile::Lite => tools
                .into_iter()
                .filter(|t| self.lists(t.name.as_ref()))
                .collect(),
        }
    }

    /// `url` with this profile asked for in its query, for a client that
    /// forwards a session to a server (`--shared`, `--remote`). The full
    /// profile is the server's default and leaves the URL as it was; a URL
    /// that already names a profile is left alone, so the caller's own words
    /// win.
    pub fn endpoint(self, url: &str) -> String {
        if self == Profile::Full || from_url(url).is_some() {
            return url.to_string();
        }
        let (base, fragment) = match url.find('#') {
            Some(i) => (&url[..i], &url[i..]),
            None => (url, ""),
        };
        let sep = if base.contains('?') { '&' } else { '?' };
        format!("{base}{sep}{QUERY_PARAMETER}={}{fragment}", self.as_str())
    }
}

/// The profile this PROCESS was started with — `--profile`, else
/// `REFLOW2_PROFILE`, else full — set once by `main` before anything is
/// served, the way the operator's content policy is
/// (`crate::content_policy::set_override`). Every service this process builds
/// starts from it (`ReflowService::wrap_at`), so every door — stdio, `--http`,
/// a registry, a design opened after a held lock frees — lists the same; and a
/// client that forwards its session to a server (`--shared`, `--remote`) asks
/// that server for it in the URL ([`Profile::endpoint`]).
static PROCESS_PROFILE: std::sync::OnceLock<Profile> = std::sync::OnceLock::new();

/// Set the process's profile. The first call wins; `main` makes it once.
pub fn set_process_profile(profile: Profile) {
    let _ = PROCESS_PROFILE.set(profile);
}

/// The process's profile: full unless `main` was told otherwise.
pub fn process_profile() -> Profile {
    PROCESS_PROFILE.get().copied().unwrap_or_default()
}

/// The profile a URL's query names, if it names one: `Some(Err)` for a value
/// that is not a profile, so it is refused rather than ignored.
pub fn from_query(query: Option<&str>) -> Option<Result<Profile, String>> {
    let query = query?;
    query
        .split('&')
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (k == QUERY_PARAMETER).then_some(v)
        })
        .next_back()
        .map(Profile::parse)
}

/// [`from_query`] over a whole URL.
fn from_url(url: &str) -> Option<Result<Profile, String>> {
    let without_fragment = url.split('#').next().unwrap_or(url);
    from_query(without_fragment.split_once('?').map(|(_, q)| q))
}

/// The profile the HTTP request behind `context` asks for, if it is one and
/// its URL names a profile.
pub(crate) fn asked_over_http(
    context: &rmcp::service::RequestContext<rmcp::RoleServer>,
) -> Option<Result<Profile, String>> {
    context
        .extensions
        .get::<http::request::Parts>()
        .and_then(|parts| from_query(parts.uri.query()))
}

/// The arguments of `use_another_tool`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UseAnotherToolReq {
    /// The name of the reflow2 tool to call, exactly as `find_tools` gives it
    /// — "topic_report", "what_next". Any served tool; not `use_another_tool`.
    pub tool: String,
    /// That tool's own arguments, as one object, exactly as you would pass
    /// them to it directly. `find_tools` gives each tool's input schema.
    /// Omitted, the tool is called with no arguments.
    #[serde(default)]
    pub arguments: serde_json::Map<String, serde_json::Value>,
}

/// THE REWRITE that makes `use_another_tool` a door rather than a second
/// implementation: a call of it becomes a call of the tool it names, with the
/// arguments it carries, before anything else in `call_tool` reads the
/// request. Its `_meta` and every other field are kept, and the context is
/// untouched, so a hub's view, the usage ledger, the content policy and every
/// refusal treat the call exactly as a direct one — an unknown name included,
/// which is refused as a direct call of it is.
///
/// Left as it came — and so answered by `use_another_tool`'s own handler, or
/// by the argument check that names it — when the call is not of
/// `use_another_tool`, when its arguments do not fit its schema, and when it
/// names itself.
pub(crate) fn reach_another_tool(
    mut request: rmcp::model::CallToolRequestParams,
    published_schema: Option<&serde_json::Map<String, serde_json::Value>>,
) -> rmcp::model::CallToolRequestParams {
    if request.name != USE_ANOTHER_TOOL {
        return request;
    }
    let mut outer = request.arguments.clone().unwrap_or_default();
    // Two arguments the listing declares on this tool as on every other it
    // decorates: a hub's `member` (`view::MEMBER_ARGUMENT`) and a write's
    // `echo` (`crate::receipt`). Given here, each is about the tool called, so
    // it travels with that tool's arguments unless they carry their own — and
    // is then refused or honoured exactly as on a direct call of it.
    let carried: Vec<(&str, serde_json::Value)> = [crate::service::MEMBER_ARGUMENT, "echo"]
        .into_iter()
        .filter_map(|k| outer.remove(k).map(|v| (k, v)))
        .collect();
    if let Some(schema) = published_schema
        && !crate::arguments::check(schema, &outer).is_empty()
    {
        return request;
    }
    let Some(tool) = outer.get("tool").and_then(serde_json::Value::as_str) else {
        return request;
    };
    if tool == USE_ANOTHER_TOOL {
        return request;
    }
    let tool = tool.to_string();
    let mut arguments = match outer.remove("arguments") {
        Some(serde_json::Value::Object(a)) => a,
        None | Some(serde_json::Value::Null) => serde_json::Map::new(),
        Some(_) => return request,
    };
    for (k, v) in carried {
        arguments.entry(k).or_insert(v);
    }
    request.name = tool.into();
    request.arguments = Some(arguments);
    request
}

/// What `use_another_tool`'s own handler answers: it runs only when the call
/// was not passed on ([`reach_another_tool`]), which for arguments that fit
/// its schema means it named itself.
pub(crate) fn not_passed_on(req: &UseAnotherToolReq) -> String {
    if req.tool == USE_ANOTHER_TOOL {
        format!(
            "`{USE_ANOTHER_TOOL}` calls another reflow2 tool and does not call itself: put the \
             name of the tool you want in `tool` (`find_tools` finds it from a sentence in your \
             own words), and its arguments in `arguments`."
        )
    } else {
        format!(
            "`{USE_ANOTHER_TOOL}` was not passed on to `{}` on this path, so nothing ran. Call \
             `{}` directly.",
            req.tool, req.tool
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_profile_parses_from_its_name_and_a_wrong_one_names_the_two() {
        assert_eq!(Profile::parse("full"), Ok(Profile::Full));
        assert_eq!(Profile::parse("lite"), Ok(Profile::Lite));
        let refused = Profile::parse("tiny").unwrap_err();
        assert!(refused.contains("`tiny`") && refused.contains("`full`"));
        assert!(refused.contains("`lite`"), "{refused}");
    }

    #[test]
    fn the_lite_list_names_44_tools_once_each_and_not_the_door() {
        let names: std::collections::BTreeSet<_> = LITE_TOOLS.iter().collect();
        assert_eq!(LITE_TOOLS.len(), 44);
        assert_eq!(names.len(), LITE_TOOLS.len(), "a name is listed twice");
        assert!(!LITE_TOOLS.contains(&USE_ANOTHER_TOOL));
        assert!(Profile::Lite.lists(USE_ANOTHER_TOOL));
        assert!(Profile::Lite.lists("find_tools"));
        assert!(!Profile::Lite.lists("topic_report"));
        assert!(Profile::Full.lists("topic_report"));
    }

    #[test]
    fn the_query_names_a_profile_and_a_url_gains_one_only_when_lite_is_asked() {
        assert_eq!(from_query(None), None);
        assert_eq!(from_query(Some("a=b")), None);
        assert_eq!(
            from_query(Some("a=b&profile=lite")),
            Some(Ok(Profile::Lite))
        );
        assert!(matches!(from_query(Some("profile=huge")), Some(Err(_))));
        let url = "http://127.0.0.1:7/mcp";
        assert_eq!(Profile::Full.endpoint(url), url);
        assert_eq!(Profile::Lite.endpoint(url), format!("{url}?profile=lite"));
        assert_eq!(
            Profile::Lite.endpoint("https://h/g/x/?k=1"),
            "https://h/g/x/?k=1&profile=lite"
        );
        assert_eq!(
            Profile::Lite.endpoint("https://h/mcp?profile=full"),
            "https://h/mcp?profile=full",
            "a URL that names its profile keeps it"
        );
    }

    #[test]
    fn a_call_of_the_door_becomes_a_call_of_the_tool_it_names() {
        let req = rmcp::model::CallToolRequestParams::new(USE_ANOTHER_TOOL).with_arguments(
            serde_json::json!({"tool": "topic_report", "arguments": {"topic": "x"}, "member": "m", "echo": "receipt"})
                .as_object()
                .unwrap()
                .clone(),
        );
        let out = reach_another_tool(req, None);
        assert_eq!(out.name, "topic_report");
        assert_eq!(
            serde_json::Value::Object(out.arguments.unwrap()),
            serde_json::json!({"topic": "x", "member": "m", "echo": "receipt"})
        );
        let itself = rmcp::model::CallToolRequestParams::new(USE_ANOTHER_TOOL).with_arguments(
            serde_json::json!({"tool": USE_ANOTHER_TOOL})
                .as_object()
                .unwrap()
                .clone(),
        );
        assert_eq!(reach_another_tool(itself, None).name, USE_ANOTHER_TOOL);
        let other = rmcp::model::CallToolRequestParams::new("get_node");
        assert_eq!(reach_another_tool(other, None).name, "get_node");
    }
}
