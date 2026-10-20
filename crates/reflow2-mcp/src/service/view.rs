//! A HUB'S ADDRESS SERVES A VIEW OVER ITS MEMBERS — layer 2 of the
//! cross-design work, at the tool layer (AGENTS.md, "A hub's address serves a
//! view over its members"; the door was settled 2026-10-11: the hub's address
//! IS the view).
//!
//! The trusted host passes `_meta["reflow2/view_graph_ids"]` on every call to
//! a hub's own design: the hub's members the caller may read. `call_tool` puts
//! that list in the `VIEW_GRAPH_IDS` task-local and hands the call here, and
//! the call is then served in exactly one of these ways:
//!
//! 1. **Not admitted** — no registry, or no trusted gateway and no bearer gate
//!    (the admission `ReflowService::other_design` uses). The hub answers
//!    alone, and a `view_note` says why the view was not served.
//! 2. **Aimed** — the call passes [`MEMBER_ARGUMENT`] (`member`), the graph_id
//!    of one member, or of the hub itself. Taken out of the arguments, it runs
//!    the SAME tool on that design alone, whatever the tool: one that takes no
//!    node id (`loop_status`, `detect_gaps`, `describe_schema`, `get_skill` …)
//!    and one the view does not otherwise serve (`claim_region`, `add_release`
//!    …) included. `tools/list` declares it on every tool when the listing
//!    request names the view (`req:through-a-hub-every-tool-can-run-on-one-named-member`).
//! 3. **Routed** — a string anywhere in the arguments is `"<member>:<rest>"`.
//!    The prefix is stripped and the SAME tool runs on that member alone, its
//!    ids prefixed back in the reply. This comes BEFORE the not-viewed list,
//!    so an id naming a member is never written into the hub under its
//!    literal prefixed form (measured 2026-10-19 with `add_release`). Two
//!    designs named is refused before anything runs; a member outside the
//!    readable set is refused naming the id and nothing of its content.
//! 4. **Not viewed** — a tool in [`NOT_VIEWED`], naming no member. The hub
//!    answers alone, and a `view_note` says so and how to aim it.
//! 5. **A write naming no member** — the hub's own design, exactly as before.
//! 6. **Fan-out** — a read naming no member runs on the hub, then on each
//!    readable member in turn, and answers `by_design`; `search_design` is
//!    the one MERGED reply, every design's hits ranked together.
//!
//! EVERY ID IN A MEMBER'S REPLY CARRIES ITS DESIGN'S PREFIX: under the
//! key-named rule ([`prefix_ids`]), AND any whole string value that is a node
//! id the member holds or an id the call passed in (a step's `assumes`, its
//! `conclusion`, `discharges`, `cases`, a summary list …), AND the ids in a
//! refusal's text ([`prefix_ids_in_text`]). Stored prose is never rewritten.
//!
//! ⭐ ONE DESIGN'S GUARD AT A TIME. Every call here is
//! `call_tool_unshaped` on ONE design, awaited to its end before the next
//! begins, and each takes and drops its own guard inside. So no call ever
//! holds two designs' guards, and never the hub's while a member's is taken —
//! which is what keeps two hubs viewing the same members in opposite orders
//! from deadlocking on tokio's writer-preferring `RwLock`.

use std::collections::{HashMap, HashSet};

use rmcp::model::{CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer};
use serde_json::{Map as JsonMap, Value as JsonValue, json};

use super::{
    ACTING_AGENT_META, READABLE_GRAPH_IDS_META, ReflowService, VIEW_GRAPH_IDS, VIEW_GRAPH_IDS_META,
    json_result, not_readable, readable_graph_ids_in,
};
use crate::content_policy::ContentPolicy;
use crate::registry_http::{Refusal, RegistryHandle};

/// The tools a hub's view does NOT serve: each acts on the hub's own design
/// only, whatever the view names, and its reply says so in `view_note`.
///
/// Each group is here for one reason, stated once:
pub(crate) const NOT_VIEWED: &[&str] = &[
    // EXPORT AND IMPORT: a design's own record, written or read whole — a
    // member's record is exported on the member.
    "export_graph",
    "import_graph",
    "export_surface",
    // RELEASES AND THE CUT: a release is one design's, cut on that design.
    "add_release",
    "release_includes",
    "release_includes_all",
    "release_report",
    "changelog_view",
    // CLAIMS AND HOLDS: who holds what on THIS design's ground.
    "claim_region",
    "release_claim",
    "claim_report",
    // SEATS: who this session is, and who it writes for.
    "mint_seat",
    "writes_for",
    // SERVED TEXT, NOT DESIGN CONTENT: the same for every design.
    "get_instructions",
    "get_skill",
    "list_skills",
    "find_tools",
    "find_skills",
    "describe_schema",
    // CROSS-DESIGN TOOLS: each already names its second design (by graph_id,
    // by path or as an inline export), so the view does not hand it a third.
    // Held to the served schemas by
    // `every_read_that_names_another_design_is_not_viewed` below: a read-only
    // tool taking any argument in OTHER_DESIGN_ARGUMENTS must be listed here.
    // (`seam_coverage` was listed by mistake until 2026-10-11, and
    // `seam_report` left out; the first reads one design's own seams and fans
    // out like any read, the second names its other design.)
    "compare_designs",
    "compose_and_analyse",
    "describe_designs",
    "pair_designs",
    "seam_report",
    "fork_point",
    "merge_designs",
    "apply_merge",
    "certify_preservation",
    "analyze_alternatives",
    "send_to_design",
    "receive_from_design",
];

/// The refusal a call naming ids in two members gets, before anything runs.
const TWO_DESIGNS: &str = "a write lands in one design; a relation across designs is a typed \
                           link record made in one of them — use review_relations with \
                           other_design and other_id";

/// THE ARGUMENT THAT AIMS ANY CALL AT ONE DESIGN OF A HUB'S VIEW: the graph_id
/// of a member, or of the hub itself. Honoured on every tool through a hub's
/// address and taken out of the arguments before the tool sees them; declared
/// on every tool by `tools/list` only when that listing names a view, so a
/// design served alone publishes exactly what it did before.
pub(crate) const MEMBER_ARGUMENT: &str = "member";

/// What `member` says in a hub's `tools/list`.
pub(crate) const MEMBER_DESCRIPTION: &str = "THROUGH A HUB'S ADDRESS ONLY: the graph_id of \
     one design of this hub's view (a member, or the hub itself) to run this tool on, alone. \
     It reaches every tool, the ones that take no node id included (loop_status, detect_gaps \
     with no scope, describe_schema, get_skill, claim_region, usage_report …), and a search or \
     scan then covers that design only. Ids in the reply carry the design's graph_id as a \
     prefix ('<graph_id>:<id>'). Omit it and the call routes by the '<graph_id>:' prefix on \
     the ids it names, or, naming none, a read runs over every design in the view.";

/// Where one call made to a hub's address runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Aim {
    /// The hub's own design alone, with the `view_note` to add, if any.
    Hub(Option<String>),
    /// One member alone, by graph_id.
    Member(String),
    /// The hub, then each readable member in turn.
    FanOut,
}

/// THE ROUTING RULE of a call made to a hub's address with the view admitted,
/// decided before anything runs. `member` is the `member` argument, already
/// taken out of `args`. Pure, so the class it decides — every served tool can
/// be aimed at one member — is held over the whole served list by a test.
pub(crate) fn aim(
    tool: &str,
    is_write: bool,
    arguments_refused: bool,
    member: Option<&str>,
    args: Option<&rmcp::model::JsonObject>,
    hub_id: &str,
    members: &[String],
) -> Result<Aim, String> {
    let named = members_named(args, members);
    let two = |designs: Vec<String>| {
        format!(
            "REFUSED, and nothing ran: this call names {} designs of this hub's view ({}): \
             {TWO_DESIGNS}.",
            designs.len(),
            designs.join(", ")
        )
    };
    // AIMED: `member` names the one design the call runs on.
    if let Some(m) = member {
        let mut designs = vec![m.to_string()];
        designs.extend(named.iter().filter(|n| n.as_str() != m).cloned());
        if designs.len() > 1 {
            return Err(two(designs));
        }
        if m == hub_id {
            return Ok(Aim::Hub(Some(format!(
                "`member` named the hub's own design ({hub_id}), so `{tool}` ran there alone."
            ))));
        }
        if members.iter().any(|x| x == m) {
            return Ok(Aim::Member(m.to_string()));
        }
        return Err(format!(
            "REFUSED, and nothing ran: `{MEMBER_ARGUMENT}` names design {m}, which is not a \
             design of this hub's view — the hub ({hub_id}) or one of the members the host \
             named on this call ({}).",
            if members.is_empty() {
                "none".to_string()
            } else {
                members.join(", ")
            }
        ));
    }
    // ROUTED: an id of exactly one member, for every tool — the not-viewed
    // ones included, so a member's id is never stored in the hub as written.
    match named.as_slice() {
        [] => {}
        [one] => return Ok(Aim::Member(one.clone())),
        several => return Err(two(several.to_vec())),
    }
    // NOT VIEWED: the hub alone, saying so and how to aim it.
    if NOT_VIEWED.contains(&tool) {
        return Ok(Aim::Hub(Some(format!(
            "`{tool}` is not served as a view: it acted on the hub's own design ({hub_id}) only. \
             To run it on a member, pass `{MEMBER_ARGUMENT}` with that member's graph_id, or \
             name the member's ids with their '<graph_id>:' prefix."
        ))));
    }
    // A WRITE NAMING NO MEMBER, or arguments the schema refuses: the hub's own
    // design, exactly as before the view existed (a refusal said once, not
    // once per design).
    if is_write || arguments_refused {
        return Ok(Aim::Hub(None));
    }
    Ok(Aim::FanOut)
}

/// Take `member` out of a call's arguments: `Ok(None)` when absent (or null),
/// the graph_id when a non-empty string, refused otherwise.
pub(crate) fn take_member(
    args: Option<&mut rmcp::model::JsonObject>,
) -> Result<Option<String>, String> {
    let Some(args) = args else {
        return Ok(None);
    };
    match args.remove(MEMBER_ARGUMENT) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::String(s)) if !s.trim().is_empty() => Ok(Some(s.trim().to_string())),
        Some(other) => Err(format!(
            "REFUSED, and nothing ran: `{MEMBER_ARGUMENT}` must be the graph_id of one design of \
             this hub's view, as a string; got {other}."
        )),
    }
}

/// Serve one call made to a hub's address with a view named
/// (`VIEW_GRAPH_IDS` set). Returns the answer already shaped for the client,
/// so `call_tool` hands it back as it is.
pub(super) async fn serve(
    hub: &ReflowService,
    mut request: CallToolRequestParams,
    context: RequestContext<RoleServer>,
    policy: ContentPolicy,
) -> Result<CallToolResponse, McpError> {
    let named = VIEW_GRAPH_IDS
        .try_with(Clone::clone)
        .ok()
        .flatten()
        .unwrap_or_default();
    let tool = request.name.to_string();
    let hub_id = hub.graph.read().await.graph_id().to_string();
    // Every call made on one design goes without the view key: a design
    // never serves a view from inside another's.
    let own = without_view(&context);
    // `member` is the view's own argument, never the tool's: out it comes,
    // before the schema check or any design reads the arguments.
    let member = take_member(request.arguments.as_mut())
        .map_err(|why| McpError::invalid_params(why, None))?;

    // 1. NOT ADMITTED: the hub alone, saying why — unless the call aimed at a
    // member, which this server cannot reach, so nothing runs.
    let registry = match admitted(hub) {
        Ok(registry) => registry,
        Err(why) => {
            if let Some(m) = member.as_deref().filter(|m| *m != hub_id) {
                return Err(McpError::invalid_params(
                    format!(
                        "REFUSED, and nothing ran: `{MEMBER_ARGUMENT}` names design {m}, and this \
                         server serves no hub's view: {why}."
                    ),
                    None,
                ));
            }
            let note = format!(
                "This call named a view (_meta[\"{VIEW_GRAPH_IDS_META}\"]) and this server does \
                 not serve one: {why}. This reply is design {hub_id} alone."
            );
            return shaped(policy, noted(on_one(hub, request, own).await, &note));
        }
    };

    // The members the host named, in its order, once each; the hub is not
    // its own member.
    let mut members: Vec<String> = Vec::new();
    for id in named {
        if id != hub_id && !members.contains(&id) {
            members.push(id);
        }
    }
    let readable = readable_graph_ids_in(context.meta.get(READABLE_GRAPH_IDS_META));
    let (reachable, unreachable): (Vec<String>, Vec<String>) = members
        .iter()
        .cloned()
        .partition(|m| not_readable(m, readable.as_deref()).is_none());
    // A CHANGE WALK CONTINUES AMONG THE VIEW'S DESIGNS (`super::far`,
    // req:reasoning-crosses-designs-within-a-hub): every call made below on one
    // design may carry its walk on into the hub and the members the caller may
    // read, and no further.
    let among: Vec<String> = std::iter::once(hub_id.clone())
        .chain(reachable.iter().cloned())
        .collect();
    super::far::WALK_AMONG
        .scope(
            Some(among),
            serve_in_view(
                hub,
                request,
                context,
                policy,
                tool,
                hub_id,
                own,
                member,
                registry,
                members,
                readable,
                reachable,
                unreachable,
            ),
        )
        .await
}

/// [`serve`] past the admission and the view's membership, with the walk's
/// reach in force.
#[allow(clippy::too_many_arguments)]
async fn serve_in_view(
    hub: &ReflowService,
    request: CallToolRequestParams,
    context: RequestContext<RoleServer>,
    policy: ContentPolicy,
    tool: String,
    hub_id: String,
    own: RequestContext<RoleServer>,
    member: Option<String>,
    registry: RegistryHandle,
    members: Vec<String>,
    readable: Option<Vec<String>>,
    reachable: Vec<String>,
    unreachable: Vec<String>,
) -> Result<CallToolResponse, McpError> {
    // A name nothing serves: the hub refuses it, as it always has.
    let Some(served) = hub.tool_router.get(&tool) else {
        return shaped(policy, on_one(hub, request, own).await);
    };
    let is_write = crate::receipt::is_write(served);
    let empty = rmcp::model::JsonObject::new();
    let arguments_refused = !crate::arguments::check(
        &crate::receipt::published_input_schema(served),
        request.arguments.as_ref().unwrap_or(&empty),
    )
    .is_empty();

    let aimed = aim(
        &tool,
        is_write,
        arguments_refused,
        member.as_deref(),
        request.arguments.as_ref(),
        &hub_id,
        &members,
    )
    .map_err(|why| McpError::invalid_params(why, None))?;
    match aimed {
        Aim::Member(member) => {
            return route(
                hub,
                &registry,
                &member,
                readable.as_deref(),
                request,
                &context,
                policy,
            )
            .await;
        }
        Aim::Hub(Some(note)) => {
            return shaped(policy, noted(on_one(hub, request, own).await, &note));
        }
        Aim::Hub(None) => return shaped(policy, on_one(hub, request, own).await),
        Aim::FanOut => {}
    }

    // FAN-OUT: the hub, then each readable member, one at a time.
    let forwarded = forwarded(hub, &context);
    let mut replies: Vec<(String, Result<CallToolResponse, McpError>)> = Vec::new();
    replies.push((hub_id.clone(), on_one(hub, request.clone(), own).await));
    for member in &reachable {
        let answer = match registry.member(member).await {
            Ok(held) => {
                let answer = on_one(held.service(), request.clone(), forwarded.clone()).await;
                prefix_reply(answer, member, held.service(), &HashSet::new()).await
            }
            Err(refusal) => Err(member_refusal(member, refusal)),
        };
        replies.push((member.clone(), answer));
    }
    let view = View {
        hub_id: &hub_id,
        members: &members,
        unreachable: &unreachable,
        tool: &tool,
    };
    if tool == "search_design" {
        view.merged_search(replies, policy)
    } else {
        view.by_design(replies, policy)
    }
}

/// Whether a `tools/list` request names a view this server serves: the
/// listing then declares `member` on every tool.
pub(super) fn lists_a_view(hub: &ReflowService, context: &RequestContext<RoleServer>) -> bool {
    readable_graph_ids_in(context.meta.get(VIEW_GRAPH_IDS_META)).is_some() && admitted(hub).is_ok()
}

/// Declare `member` on every tool of a hub's listing.
pub(super) fn declare_member(tools: &mut [rmcp::model::Tool]) {
    for t in tools {
        let mut schema: rmcp::model::JsonObject = (*t.input_schema).clone();
        let props = schema
            .entry("properties")
            .or_insert_with(|| JsonValue::Object(JsonMap::new()));
        if let JsonValue::Object(p) = props {
            p.insert(
                MEMBER_ARGUMENT.into(),
                json!({
                    "type": ["string", "null"],
                    "default": null,
                    "description": MEMBER_DESCRIPTION,
                }),
            );
        }
        t.input_schema = std::sync::Arc::new(schema);
    }
}

/// The refusal a call passing `member` gets where no hub's view is served:
/// the argument aims a call at one design of a view, and there is none.
pub(super) fn member_without_a_view(tool: &str) -> String {
    format!(
        "REFUSED, and nothing ran: `{MEMBER_ARGUMENT}` aims a call at one design of a hub's \
         view, and is honoured only through a hub's address; this call to `{tool}` was not made \
         through one. Call it without `{MEMBER_ARGUMENT}`, or through the hub."
    )
}

/// The registry a view reaches its members through — or why this server
/// serves no view: the admission `ReflowService::other_design` uses.
fn admitted(hub: &ReflowService) -> Result<RegistryHandle, String> {
    let Some(registry) = hub.registry.as_ref() else {
        return Err(
            "it holds one design, not a registry of many, so it has no members to \
                    reach"
                .to_string(),
        );
    };
    match hub.caller_rule.as_ref() {
        crate::caller::CallerRule::TrustedGateway { .. }
        | crate::caller::CallerRule::BearerToken { .. } => Ok(registry.clone()),
        crate::caller::CallerRule::Local | crate::caller::CallerRule::Undeclared { .. } => Err(
            "it declares no trusted gateway and no bearer gate, so nothing trusted handed it \
             the view (the same admission as a cross-design read)"
                .to_string(),
        ),
    }
}

/// Run one call on one design, as that design serves any call, with no view
/// in force — the only way this module reaches a design.
async fn on_one(
    svc: &ReflowService,
    request: CallToolRequestParams,
    context: RequestContext<RoleServer>,
) -> Result<CallToolResponse, McpError> {
    VIEW_GRAPH_IDS
        .scope(None, svc.call_tool_unshaped(request, context))
        .await
}

/// The call's context without the view key.
fn without_view(context: &RequestContext<RoleServer>) -> RequestContext<RoleServer> {
    let mut ctx = context.clone();
    ctx.meta.0.0.remove(VIEW_GRAPH_IDS_META);
    ctx
}

/// The context a member call runs under: the caller's own `_meta` — who it
/// writes for, the agent it writes through, the readable set — and the
/// verified token, all carried as they came, so authorship on the member is
/// the caller's. An acting agent this hub session DECLARED (rather than sent)
/// is carried too, since the member's session is new and knows nothing of it.
fn forwarded(
    hub: &ReflowService,
    context: &RequestContext<RoleServer>,
) -> RequestContext<RoleServer> {
    let mut ctx = without_view(context);
    if !ctx.meta.0.0.contains_key(ACTING_AGENT_META)
        && let Some(agent) = hub
            .acting_agent
            .lock()
            .ok()
            .and_then(|a| a.clone())
            .filter(|a| !a.trim().is_empty())
    {
        ctx.meta
            .0
            .0
            .insert(ACTING_AGENT_META.to_string(), JsonValue::String(agent));
    }
    ctx
}

/// Run the call on `member` alone, its prefix stripped from the arguments and
/// put back on the ids in the reply.
async fn route(
    hub: &ReflowService,
    registry: &RegistryHandle,
    member: &str,
    readable: Option<&[String]>,
    mut request: CallToolRequestParams,
    context: &RequestContext<RoleServer>,
    policy: ContentPolicy,
) -> Result<CallToolResponse, McpError> {
    if let Some(why) = not_readable(member, readable) {
        return Err(McpError::invalid_params(why, None));
    }
    // What the call passed with the member's prefix: each is an id of that
    // member's, whether or not it holds it, so a reply or refusal echoing one
    // gives it back as it came.
    let mut passed = HashSet::new();
    if let Some(args) = request.arguments.as_mut() {
        for v in args.values_mut() {
            strip_member_noting(v, member, &mut passed);
        }
    }
    let held = registry
        .member(member)
        .await
        .map_err(|r| member_refusal(member, r))?;
    let answer = on_one(held.service(), request, forwarded(hub, context)).await;
    let answer = prefix_reply(answer, member, held.service(), &passed).await;
    drop(held);
    let note = format!(
        "This call ran on design {member} alone, a member of this hub's view. Ids that design \
         holds carry its graph_id as a prefix ('{member}:<id>'); pass one back through this hub, \
         or pass `{MEMBER_ARGUMENT}: \"{member}\"`, and the call goes to {member} again."
    );
    shaped(policy, noted(answer, &note))
}

/// EVERY ID IN A MEMBER'S REPLY CARRIES ITS DESIGN'S PREFIX. Applied to one
/// member's answer while the caller still holds that member (never the
/// hub's guard: the member's read guard is taken here, alone, after its call
/// has finished):
///
/// - a structured reply gets the key-named rule ([`prefix_ids`]), then every
///   whole string value that is an id `passed` in or a node id the member
///   holds ([`prefix_held_values`]) — a step's `assumes`, `conclusion`,
///   `discharges`, `cases`, a summary list, whatever its key is called;
/// - a refusal — an `isError` reply's text or a protocol error's message —
///   gets the same ids prefixed where they stand in its words
///   ([`prefix_ids_in_text`]).
///
/// A successful reply's text blocks are left as they are: they are a
/// signpost, or served prose, and the payload is the reply.
async fn prefix_reply(
    answer: Result<CallToolResponse, McpError>,
    member: &str,
    svc: &ReflowService,
    passed: &HashSet<String>,
) -> Result<CallToolResponse, McpError> {
    let g = svc.graph.read().await;
    let mut seen: HashMap<String, bool> = HashMap::new();
    let mut holds = |s: &str| -> bool {
        if passed.contains(s) {
            return true;
        }
        if !looks_like_an_id(s) {
            return false;
        }
        if let Some(known) = seen.get(s) {
            return *known;
        }
        let held = g
            .node_types_holding(s)
            .map(|types| !types.is_empty())
            .unwrap_or(false);
        seen.insert(s.to_string(), held);
        held
    };
    match answer {
        Err(mut e) => {
            e.message = prefix_ids_in_text(&e.message, member, &mut holds).into();
            Err(e)
        }
        Ok(CallToolResponse::Complete(mut r)) if r.is_error == Some(true) => {
            r.content = r
                .content
                .into_iter()
                .map(|b| match b.as_text() {
                    Some(t) => ContentBlock::text(prefix_ids_in_text(&t.text, member, &mut holds)),
                    None => b,
                })
                .collect();
            Ok(CallToolResponse::Complete(r))
        }
        Ok(CallToolResponse::Complete(mut r)) => {
            if let Some(v) = r.structured_content.as_mut() {
                prefix_ids(v, member);
                prefix_held_values(v, member, &mut holds);
            }
            Ok(CallToolResponse::Complete(r))
        }
        other => other,
    }
}

/// Could `s` be a node id? Ids follow the typed-prefix convention
/// (`req:…`, `who:…`), so one holds a `:` and no whitespace, and is short.
/// Only such a string is looked up; anything else is prose or a value.
pub(super) fn looks_like_an_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 512 && s.contains(':') && !s.contains(char::is_whitespace)
}

/// Every whole string value in `v` for which `holds` says it is the member's
/// id gets the member's prefix — except under a key naming a design
/// (`…graph_id`, `…graph_ids`) or an id a cross-design record places in
/// ANOTHER design ([`names_a_node_elsewhere`]), as in [`prefix_ids`].
pub(super) fn prefix_held_values(
    v: &mut JsonValue,
    member: &str,
    holds: &mut impl FnMut(&str) -> bool,
) {
    match v {
        JsonValue::String(s) => {
            if !s.starts_with(&format!("{member}:")) && holds(s) {
                *s = prefixed(s, member);
            }
        }
        JsonValue::Array(items) => items
            .iter_mut()
            .for_each(|i| prefix_held_values(i, member, holds)),
        JsonValue::Object(o) => {
            let elsewhere: Vec<String> = o
                .keys()
                .filter(|k| names_a_node_elsewhere(k, o))
                .cloned()
                .collect();
            for (k, val) in o.iter_mut() {
                if elsewhere.contains(k)
                    || k == ACROSS_DESIGNS
                    || k.ends_with("graph_id")
                    || k.ends_with("graph_ids")
                {
                    continue;
                }
                prefix_held_values(val, member, holds);
            }
        }
        _ => {}
    }
}

/// The member's ids prefixed where they stand in a refusal's words. A word
/// is a run of non-space characters; the quotes, brackets and punctuation
/// around an id (`'req:x'`, `(req:x)`, `req:x,`, `req:x:`) are kept and only
/// the id between them is prefixed, when `holds` says it is the member's.
pub(crate) fn prefix_ids_in_text(
    text: &str,
    member: &str,
    holds: &mut impl FnMut(&str) -> bool,
) -> String {
    const OPEN: &[char] = &['"', '\'', '`', '(', '[', '{', '<'];
    const CLOSE: &[char] = &[
        '"', '\'', '`', ')', ']', '}', '>', ',', ';', '.', ':', '!', '?',
    ];
    let mine = format!("{member}:");
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        let start = rest
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(rest.len());
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let word = &rest[..end];
        rest = &rest[end..];
        let core_start = word.len() - word.trim_start_matches(OPEN).len();
        let inner = &word[core_start..];
        let core = inner.trim_end_matches(CLOSE);
        if !core.is_empty() && !core.starts_with(&mine) && holds(core) {
            out.push_str(&word[..core_start]);
            out.push_str(&mine);
            out.push_str(core);
            out.push_str(&inner[core.len()..]);
        } else {
            out.push_str(word);
        }
    }
    out
}

/// What reaching a member the caller may read said, as the caller hears it.
fn member_refusal(member: &str, refusal: Refusal) -> McpError {
    match refusal {
        Refusal::NotADesign(_) => McpError::invalid_params(
            format!(
                "design {member} is in the set the host handed over, but no design by that id \
                 is served here."
            ),
            None,
        ),
        Refusal::Busy(why) | Refusal::CouldNotOpen(why) => McpError::internal_error(
            format!("design {member} could not be reached right now: {why}"),
            None,
        ),
    }
}

/// The per-client shaping `call_tool` applies to every other answer.
fn shaped(
    policy: ContentPolicy,
    answer: Result<CallToolResponse, McpError>,
) -> Result<CallToolResponse, McpError> {
    crate::content_policy::shape(policy, answer)
}

/// Put `note` on a reply as its `view_note`: inside the structured payload
/// when there is one, else as a text block of its own after the document. A
/// refusal is left exactly as it was — its text already says what happened.
fn noted(
    answer: Result<CallToolResponse, McpError>,
    note: &str,
) -> Result<CallToolResponse, McpError> {
    answer.map(|response| match response {
        CallToolResponse::Complete(mut r) if r.is_error != Some(true) => {
            match r
                .structured_content
                .as_mut()
                .and_then(JsonValue::as_object_mut)
            {
                Some(o) => {
                    o.insert("view_note".into(), JsonValue::String(note.to_string()));
                }
                None => r
                    .content
                    .push(ContentBlock::text(format!("view_note: {note}"))),
            }
            CallToolResponse::Complete(r)
        }
        other => other,
    })
}

/// The text of a result's content blocks, joined.
fn text_of(r: &CallToolResult) -> String {
    r.content
        .iter()
        .filter_map(|b| b.as_text().map(|t| t.text.clone()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// One design's answer as a JSON value in a view: its structured payload (a
/// member's ids prefixed), its text when it has no payload, or `{"error"}`
/// saying why it did not answer — never dropped.
fn reply_value(answer: Result<CallToolResponse, McpError>, member: Option<&str>) -> JsonValue {
    match answer {
        Err(e) => json!({ "error": e.message }),
        Ok(CallToolResponse::Complete(r)) if r.is_error == Some(true) => {
            json!({ "error": text_of(&r) })
        }
        Ok(CallToolResponse::Complete(r)) => match r.structured_content.clone() {
            Some(mut v) => {
                if let Some(m) = member {
                    prefix_ids(&mut v, m);
                }
                v
            }
            None => json!({ "text": text_of(&r) }),
        },
        Ok(_) => json!({
            "error": "this design did not complete the call in one reply (it asked for more \
                      input or made a task), which a view cannot relay; call it on that \
                      design's own address"
        }),
    }
}

/// What a fan-out reply is built from.
struct View<'a> {
    hub_id: &'a str,
    members: &'a [String],
    unreachable: &'a [String],
    tool: &'a str,
}

impl View<'_> {
    fn unreachable_sentence(&self) -> String {
        if self.unreachable.is_empty() {
            String::new()
        } else {
            format!(
                " {} member(s) the caller may not read are named in `unreachable` and were not \
                 asked.",
                self.unreachable.len()
            )
        }
    }

    /// The reply: `view`, `by_design` (the hub first), `unreachable`.
    fn by_design(
        &self,
        replies: Vec<(String, Result<CallToolResponse, McpError>)>,
        policy: ContentPolicy,
    ) -> Result<CallToolResponse, McpError> {
        let mut by_design = JsonMap::new();
        let mut asked = Vec::new();
        for (id, answer) in replies {
            let member = (id != self.hub_id).then_some(id.as_str());
            by_design.insert(id.clone(), reply_value(answer, member));
            asked.push(id);
        }
        let note = format!(
            "A VIEW over hub {} and its members: `{}` ran on each design in turn, and \
             `by_design` holds each design's own reply, keyed by graph_id ({} is the hub). Ids \
             in a member's reply carry its graph_id as a prefix ('<graph_id>:<id>'); pass one \
             back through this hub and the call goes to that design alone.{}",
            self.hub_id,
            self.tool,
            self.hub_id,
            self.unreachable_sentence()
        );
        let payload = json!({
            "view": self.members,
            "by_design": by_design,
            "unreachable": self.unreachable,
            "view_note": note,
        });
        let sentence = format!(
            "This reply is a VIEW over {} designs ({}): `{}` answered by each in turn, under \
             `by_design`.",
            asked.len(),
            asked.join(", "),
            self.tool
        );
        view_result(payload, policy, &sentence)
    }

    /// `search_design`'s one merged reply: every design's hits ranked
    /// together by score, each member's prefixed and every hit naming its
    /// design, and the caller's ONE `limit` bounding the merged list.
    ///
    /// The hub's address is one design to the caller, so one limit. Each
    /// design is asked with that same limit (its top-k), the hits are merged
    /// and ranked, and the top-k of the merged list is returned: exact,
    /// because no hit outside a design's top-k can be in the merged top-k.
    /// Measured 2026-10-11 on flo2.io before this: `limit: 10` through a hub
    /// of seven designs answered 65 hits, the reply saying the limit applied
    /// to each design.
    fn merged_search(
        &self,
        replies: Vec<(String, Result<CallToolResponse, McpError>)>,
        policy: ContentPolicy,
    ) -> Result<CallToolResponse, McpError> {
        let mut hits: Vec<JsonValue> = Vec::new();
        let mut stale: Vec<JsonValue> = Vec::new();
        let mut searched = JsonMap::new();
        let mut found = JsonMap::new();
        let mut failed = JsonMap::new();
        let mut loop_hints = JsonMap::new();
        let mut designs_searched: Vec<String> = Vec::new();
        let mut limit = JsonValue::Null;
        for (id, answer) in replies {
            let member = (id != self.hub_id).then_some(id.as_str());
            let v = reply_value(answer, member);
            if let Some(error) = v.get("error") {
                failed.insert(id, error.clone());
                continue;
            }
            let own = v["hits"].as_array().cloned().unwrap_or_default();
            found.insert(id.clone(), JsonValue::from(own.len()));
            for mut hit in own {
                if let Some(o) = hit.as_object_mut() {
                    o.insert("design".into(), JsonValue::String(id.clone()));
                }
                hits.push(hit);
            }
            // `stale` holds node ids under a key the prefix rule does not
            // name, so it is prefixed here, where its meaning is known.
            for s in v["stale"].as_array().cloned().unwrap_or_default() {
                stale.push(match (s, member) {
                    (JsonValue::String(s), Some(m)) => JsonValue::String(prefixed(&s, m)),
                    (other, _) => other,
                });
            }
            searched.insert(id.clone(), v["searched"].clone());
            if limit.is_null() {
                limit = v["limit"].clone();
            }
            if let Some(hint) = v.get("loop_hint") {
                loop_hints.insert(id.clone(), hint.clone());
            }
            designs_searched.push(id);
        }
        // Stable, so equal scores keep the hub-first design order.
        hits.sort_by(|a, b| {
            let score = |h: &JsonValue| h["score"].as_f64().unwrap_or(f64::NEG_INFINITY);
            score(b)
                .partial_cmp(&score(a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        // ONE limit, the caller's, on the merged list. Every design was asked
        // with the same limit, so the merged top-k is exact; what falls off
        // the end is counted, and `found_by_design` still names every design
        // that matched.
        let ranked = hits.len();
        if let Some(n) = limit.as_u64().map(|n| n as usize) {
            hits.truncate(n);
        }
        let withheld = ranked - hits.len();
        let note = format!(
            "search_design through hub {}'s view: every design's hits ranked together by score, \
             each member's node_id prefixed with its graph_id ('<graph_id>:<id>') and the hub's \
             own unprefixed; `design` names where each hit came from. Scores come from each \
             design's own index, so the order across designs is a guide, not a measurement. The \
             one `limit` bounds this merged list: each design was asked for that many, \
             `found_by_design` says how many each returned (a design at the limit may hold \
             more), and `hits_withheld` counts the ranked hits that fell outside it — raise \
             `limit`, or search a member's own address, to see them.{}",
            self.hub_id,
            self.unreachable_sentence()
        );
        let mut payload = json!({
            "hits": hits,
            "limit": limit,
            "hits_withheld": withheld,
            "found_by_design": found,
            "designs_searched": designs_searched,
            "searched_by_design": searched,
            "stale": stale,
            "view": self.members,
            "unreachable": self.unreachable,
            "view_note": note,
        });
        if let Some(o) = payload.as_object_mut() {
            if hits_empty(o) {
                o.insert(
                    "empty_because".into(),
                    JsonValue::String(
                        "no design in the view matched the query; `searched_by_design` says \
                         how many nodes each design's search ran over"
                            .to_string(),
                    ),
                );
            }
            if !failed.is_empty() {
                o.insert("failed".into(), JsonValue::Object(failed));
            }
            if !loop_hints.is_empty() {
                o.insert("loop_hints".into(), JsonValue::Object(loop_hints));
            }
        }
        let sentence = format!(
            "This reply is a VIEW: search_design over {} design(s) ({}), the hits merged and \
             ranked together.",
            designs_searched_len(&payload),
            payload["designs_searched"]
                .as_array()
                .map(|a| a
                    .iter()
                    .filter_map(JsonValue::as_str)
                    .collect::<Vec<_>>()
                    .join(", "))
                .unwrap_or_default()
        );
        view_result(payload, policy, &sentence)
    }
}

fn hits_empty(o: &JsonMap<String, JsonValue>) -> bool {
    o.get("hits")
        .and_then(JsonValue::as_array)
        .is_some_and(Vec::is_empty)
}

fn designs_searched_len(payload: &JsonValue) -> usize {
    payload["designs_searched"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0)
}

/// A view's reply: the payload as `structuredContent`, `content` shaped for
/// the client as every reply is, and — wherever there is a text block — the
/// sentence saying it is a view, first. (A client served an EMPTY `content`
/// fills it from the payload itself, so it gets none; the payload's own
/// `view_note` says it there.)
fn view_result(
    payload: JsonValue,
    policy: ContentPolicy,
    sentence: &str,
) -> Result<CallToolResponse, McpError> {
    let mut result = json_result(payload)?;
    crate::content_policy::apply(policy, &mut result);
    if let Some(text) = result
        .content
        .first()
        .and_then(|b| b.as_text())
        .map(|t| t.text.clone())
    {
        result.content[0] = ContentBlock::text(format!("{sentence}\n\n{text}"));
    }
    Ok(CallToolResponse::Complete(result))
}

/// The members a call names, in the order its arguments first name them: a
/// string value anywhere in the arguments that is `"<member>:<rest>"`.
fn members_named(args: Option<&rmcp::model::JsonObject>, members: &[String]) -> Vec<String> {
    fn walk(v: &JsonValue, members: &[String], out: &mut Vec<String>) {
        match v {
            JsonValue::String(s) => {
                if let Some(m) = member_of(s, members)
                    && !out.iter().any(|o| o == m)
                {
                    out.push(m.clone());
                }
            }
            JsonValue::Array(items) => items.iter().for_each(|i| walk(i, members, out)),
            JsonValue::Object(o) => o.values().for_each(|i| walk(i, members, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    if let Some(args) = args {
        for v in args.values() {
            walk(v, members, &mut out);
        }
    }
    out
}

/// The member whose graph_id `s` starts with, followed by `:` — an exact
/// graph_id, never a prefix of one.
fn member_of<'a>(s: &str, members: &'a [String]) -> Option<&'a String> {
    members
        .iter()
        .find(|m| s.len() > m.len() && s.starts_with(m.as_str()) && s.as_bytes()[m.len()] == b':')
}

/// Strip `"<member>:"` from every string value that carries it.
#[cfg(test)]
fn strip_member(v: &mut JsonValue, member: &str) {
    strip_member_noting(v, member, &mut HashSet::new());
}

/// Strip `"<member>:"` from every string value that carries it, noting in
/// `passed` each value as it is now, unprefixed.
fn strip_member_noting(v: &mut JsonValue, member: &str, passed: &mut HashSet<String>) {
    match v {
        JsonValue::String(s) => {
            if let Some(rest) = s
                .strip_prefix(member)
                .and_then(|r| r.strip_prefix(':'))
                .map(String::from)
            {
                passed.insert(rest.clone());
                *s = rest;
            }
        }
        JsonValue::Array(items) => items
            .iter_mut()
            .for_each(|i| strip_member_noting(i, member, passed)),
        JsonValue::Object(o) => o
            .values_mut()
            .for_each(|i| strip_member_noting(i, member, passed)),
        _ => {}
    }
}

/// THE PREFIX RULE for a member's reply, kept small and stated (AGENTS.md):
///
/// - a STRING under a key named `id`, or ending in `_id` (`node_id`,
///   `from_id`, `to_id`, `target_id`, `subject_id`, `source_id`, `other_id`,
///   `artifact_id`, `decision_id`, `verification_id`, `capability_id`, …), is
///   a node id and gets `"<graph_id>:"`;
/// - every string ITEM of an array under a key ending in `_ids` does too;
/// - under `drawn`, `edges_drawn` and `edges_already_present`, each item is an
///   edge sentence `"A REL B"`, and A and B get it;
/// - a key ending in `graph_id` / `graph_ids` names a DESIGN, not a node, and
///   is left alone, as is every other string — free text is never rewritten;
/// - an id that a cross-design record says lives in ANOTHER design is that
///   design's, and is left alone: `<p>_node_id` (`design_node_id`,
///   `to_node_id`, `from_node_id`) when the same object names its design in a
///   non-empty `<p>`, `<p>_design` or `<p>_graph_id`.
///
/// An empty string, or one already carrying the prefix, is left as it is.
pub(crate) fn prefix_ids(v: &mut JsonValue, graph_id: &str) {
    match v {
        JsonValue::Object(o) => {
            let elsewhere: Vec<String> = o
                .keys()
                .filter(|k| names_a_node_elsewhere(k, o))
                .cloned()
                .collect();
            for (k, val) in o.iter_mut() {
                if elsewhere.contains(k) || k == ACROSS_DESIGNS {
                    continue;
                }
                if is_id_key(k) {
                    match val {
                        JsonValue::String(s) => *s = prefixed(s, graph_id),
                        other => prefix_ids(other, graph_id),
                    }
                } else if is_id_list_key(k) || is_edge_sentence_key(k) {
                    let sentence = is_edge_sentence_key(k);
                    match val {
                        JsonValue::Array(items) => {
                            for item in items {
                                match item {
                                    JsonValue::String(s) if sentence => {
                                        *s = prefixed_sentence(s, graph_id);
                                    }
                                    JsonValue::String(s) => *s = prefixed(s, graph_id),
                                    other => prefix_ids(other, graph_id),
                                }
                            }
                        }
                        other => prefix_ids(other, graph_id),
                    }
                } else {
                    prefix_ids(val, graph_id);
                }
            }
        }
        JsonValue::Array(items) => items.iter_mut().for_each(|i| prefix_ids(i, graph_id)),
        _ => {}
    }
}

/// Whether `key` is `<p>_node_id` in an object that names the design that
/// node lives in (`<p>`, `<p>_design` or `<p>_graph_id`, a non-empty string).
fn names_a_node_elsewhere(key: &str, object: &JsonMap<String, JsonValue>) -> bool {
    let Some(p) = key.strip_suffix("_node_id") else {
        return false;
    };
    [
        p.to_string(),
        format!("{p}_design"),
        format!("{p}_graph_id"),
    ]
    .iter()
    .any(|k| {
        object
            .get(k)
            .and_then(JsonValue::as_str)
            .is_some_and(|s| !s.is_empty())
    })
}

/// The key a change walk carries the designs it continued into under
/// (`super::far`): each entry's ids already carry THAT design's prefix, so a
/// member's prefix is never put on them.
pub(crate) const ACROSS_DESIGNS: &str = "across_designs";

fn is_id_key(k: &str) -> bool {
    k == "id" || (k.ends_with("_id") && !k.ends_with("graph_id"))
}

fn is_id_list_key(k: &str) -> bool {
    k.ends_with("_ids") && !k.ends_with("graph_ids")
}

fn is_edge_sentence_key(k: &str) -> bool {
    matches!(k, "drawn" | "edges_drawn" | "edges_already_present")
}

fn prefixed(s: &str, graph_id: &str) -> String {
    let prefix = format!("{graph_id}:");
    if s.is_empty() || s.starts_with(&prefix) {
        s.to_string()
    } else {
        format!("{prefix}{s}")
    }
}

/// `"A REL B"` → `"<g>:A REL <g>:B"`; a string that is not a sentence of
/// three or more words is prefixed whole, as an id.
fn prefixed_sentence(s: &str, graph_id: &str) -> String {
    match (s.find(' '), s.rfind(' ')) {
        (Some(first), Some(last)) if first < last => format!(
            "{}{}{}",
            prefixed(&s[..first], graph_id),
            &s[first..=last],
            prefixed(&s[last + 1..], graph_id)
        ),
        _ => prefixed(s, graph_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prefix_rule_names_ids_id_lists_and_edge_sentences_and_nothing_else() {
        let mut v = json!({
            "id": "req:a",
            "node_id": "req:b",
            "from_id": "req:c",
            "contributor_id": "who:x",
            "graph_id": "g",
            "imported_graph_id": "g",
            "affected_ids": ["req:d", "req:e"],
            "readable_graph_ids": ["g"],
            "edges_drawn": ["dec:a DEPENDS_ON dec:b"],
            "drawn": ["one"],
            "statement": "req:a is free text",
            "nested": [{"to_id": "req:f", "name": "req:g"}],
            "empty_id": "",
            "already_id": "G:req:h",
        });
        prefix_ids(&mut v, "G");
        assert_eq!(v["id"], "G:req:a");
        assert_eq!(v["node_id"], "G:req:b");
        assert_eq!(v["from_id"], "G:req:c");
        assert_eq!(v["contributor_id"], "G:who:x");
        assert_eq!(v["graph_id"], "g", "a design id is not a node id");
        assert_eq!(v["imported_graph_id"], "g");
        assert_eq!(v["affected_ids"], json!(["G:req:d", "G:req:e"]));
        assert_eq!(v["readable_graph_ids"], json!(["g"]));
        assert_eq!(v["edges_drawn"], json!(["G:dec:a DEPENDS_ON G:dec:b"]));
        assert_eq!(v["drawn"], json!(["G:one"]));
        assert_eq!(
            v["statement"], "req:a is free text",
            "free text is left alone"
        );
        assert_eq!(v["nested"][0]["to_id"], "G:req:f");
        assert_eq!(v["nested"][0]["name"], "req:g");
        assert_eq!(v["empty_id"], "");
        assert_eq!(v["already_id"], "G:req:h", "never prefixed twice");

        // A cross-design record's far id is the other design's, not this one's.
        let mut v = json!({
            "linked": [
                {"node_id": "ref:x", "design": "OTHER", "design_node_id": "req:far"},
                {"node_id": "req:near"}
            ],
            "sent": {"node_id": "req:s", "to_design": "OTHER", "to_node_id": "req:t"},
            "received": {"from_design": "", "from_node_id": "req:u"},
            "properties": {"design_graph_id": "OTHER", "design_node_id": "req:v"},
        });
        prefix_ids(&mut v, "G");
        assert_eq!(v["linked"][0]["node_id"], "G:ref:x");
        assert_eq!(v["linked"][0]["design_node_id"], "req:far");
        assert_eq!(v["linked"][1]["node_id"], "G:req:near");
        assert_eq!(v["sent"]["node_id"], "G:req:s");
        assert_eq!(v["sent"]["to_node_id"], "req:t");
        assert_eq!(
            v["received"]["from_node_id"], "G:req:u",
            "no design named beside it: it is this design's"
        );
        assert_eq!(v["properties"]["design_node_id"], "req:v");
    }

    #[test]
    fn a_member_is_named_by_its_exact_graph_id_then_a_colon() {
        let members = vec!["abc".to_string(), "abcd".to_string()];
        assert_eq!(member_of("abc:req:x", &members), Some(&members[0]));
        assert_eq!(member_of("abcd:req:x", &members), Some(&members[1]));
        assert_eq!(member_of("abcde:req:x", &members), None);
        assert_eq!(member_of("abc", &members), None);
        assert_eq!(member_of("req:abc:x", &members), None);

        let args: rmcp::model::JsonObject = serde_json::from_value(json!({
            "from_id": "abc:req:x",
            "nested": {"to": ["abcd:req:y", "abc:req:z"]},
            "statement": "plain"
        }))
        .unwrap();
        assert_eq!(members_named(Some(&args), &members), vec!["abc", "abcd"]);

        let mut v = JsonValue::Object(args);
        strip_member(&mut v, "abc");
        assert_eq!(v["from_id"], "req:x");
        assert_eq!(v["nested"]["to"], json!(["abcd:req:y", "req:z"]));
    }

    /// Every argument by which a tool names a design other than the one it is
    /// called on: by graph_id, by path, or as an inline export.
    const OTHER_DESIGN_ARGUMENTS: &[&str] = &[
        "other_graph_id",
        "other_graph_ids",
        "base_graph_id",
        "base_path",
        "other_path",
        "ours_path",
        "theirs_path",
        "paths",
        "export_path",
        "design",
    ];

    /// A READ that names its own second design must not be fanned out by a
    /// view: run once per member it would compare each member with that same
    /// design and answer N comparisons nobody asked for. A write naming no
    /// member already acts on the hub alone (step 4), so only reads are held.
    /// The list is read from the served schemas, so a new cross-design read
    /// cannot slip past NOT_VIEWED the way `seam_report` did in 0.83.0.
    #[test]
    fn every_read_that_names_another_design_is_not_viewed() {
        let svc = ReflowService::in_memory().expect("in-memory service");
        let mut missing = Vec::new();
        let mut held = Vec::new();
        for tool in svc.tool_router.list_all() {
            let schema = crate::receipt::published_input_schema(&tool);
            let Some(props) = schema.get("properties").and_then(|p| p.as_object()) else {
                continue;
            };
            let names: Vec<&str> = OTHER_DESIGN_ARGUMENTS
                .iter()
                .copied()
                .filter(|a| props.contains_key(*a))
                .collect();
            if names.is_empty() || crate::receipt::is_write(&tool) {
                continue;
            }
            held.push(tool.name.to_string());
            if !NOT_VIEWED.contains(&tool.name.as_ref()) {
                missing.push(format!("{} (takes {})", tool.name, names.join(", ")));
            }
        }
        assert!(
            missing.is_empty(),
            "these reads name another design but a view would fan them out: {missing:?}"
        );
        for named in ["seam_report", "compare_designs", "pair_designs"] {
            assert!(
                held.iter().any(|t| t == named),
                "the guard must see {named} as a cross-design read, or it guards nothing: {held:?}"
            );
        }
    }

    /// THE CLASS (req:through-a-hub-every-tool-can-run-on-one-named-member):
    /// through a hub's address, EVERY served tool — read or write, viewed or
    /// not, taking a node id or none — runs on the one member `member` names,
    /// and on the one member an id's prefix names. Read off the served list,
    /// so a tool added later is held to it without anyone listing it.
    #[test]
    fn every_served_tool_can_be_aimed_at_one_member() {
        let svc = ReflowService::in_memory().expect("in-memory service");
        let members = vec!["A".to_string(), "B".to_string()];
        let by_id: rmcp::model::JsonObject =
            serde_json::from_value(json!({"any": ["A:req:x"]})).unwrap();
        let tools = svc.tool_router.list_all();
        assert!(tools.len() > 100, "the whole surface: {}", tools.len());
        for tool in &tools {
            let name = tool.name.as_ref();
            let write = crate::receipt::is_write(tool);
            for refused in [false, true] {
                assert_eq!(
                    aim(name, write, refused, Some("A"), None, "H", &members),
                    Ok(Aim::Member("A".into())),
                    "`member` aims {name} at A"
                );
                assert_eq!(
                    aim(name, write, refused, None, Some(&by_id), "H", &members),
                    Ok(Aim::Member("A".into())),
                    "an id prefixed A: aims {name} at A"
                );
                assert!(
                    matches!(
                        aim(name, write, refused, Some("H"), None, "H", &members),
                        Ok(Aim::Hub(Some(_)))
                    ),
                    "`member` naming the hub runs {name} on the hub alone"
                );
                assert!(
                    aim(name, write, refused, Some("Z"), None, "H", &members).is_err(),
                    "`member` naming no design of the view is refused for {name}"
                );
                let b_id: rmcp::model::JsonObject =
                    serde_json::from_value(json!({"id": "B:req:y"})).unwrap();
                assert!(
                    aim(name, write, refused, Some("A"), Some(&b_id), "H", &members).is_err(),
                    "`member` A beside an id of B is two designs, refused for {name}"
                );
            }
        }
    }

    /// `member` is the view's own argument: no tool may take one of its own,
    /// or a hub would strip the tool's argument as an aim.
    #[test]
    fn no_served_tool_takes_an_argument_named_member() {
        let svc = ReflowService::in_memory().expect("in-memory service");
        let taking: Vec<String> = svc
            .tool_router
            .list_all()
            .iter()
            .filter(|t| {
                crate::receipt::published_input_schema(t)
                    .get("properties")
                    .and_then(|p| p.as_object())
                    .is_some_and(|p| p.contains_key(MEMBER_ARGUMENT))
            })
            .map(|t| t.name.to_string())
            .collect();
        assert!(taking.is_empty(), "{taking:?}");
    }

    #[test]
    fn a_whole_value_or_a_refusals_word_naming_a_held_id_is_prefixed() {
        let held = ["req:a", "inf:s", "req:passed"];
        let mut holds = |s: &str| held.contains(&s);
        let mut v = json!({
            "assumes": ["req:a"],
            "conclusion": "req:a",
            "cases": [{"case": "inf:s"}],
            "statement": "req:a is prose",
            "free": "not:held",
            "graph_id": "req:a",
            "linked": {"design": "OTHER", "design_node_id": "req:a"},
            "already": "G:req:a",
        });
        prefix_held_values(&mut v, "G", &mut holds);
        assert_eq!(v["assumes"], json!(["G:req:a"]));
        assert_eq!(v["conclusion"], "G:req:a");
        assert_eq!(v["cases"][0]["case"], "G:inf:s");
        assert_eq!(v["statement"], "req:a is prose", "prose is never rewritten");
        assert_eq!(v["free"], "not:held");
        assert_eq!(v["graph_id"], "req:a", "a design id is not a node id");
        assert_eq!(v["linked"]["design_node_id"], "req:a", "another design's");
        assert_eq!(v["already"], "G:req:a", "never prefixed twice");

        let text = "no node 'req:passed' (premise of inf:s): see req:a, req:a. and `req:a`; \
                    not:held stays";
        assert_eq!(
            prefix_ids_in_text(text, "G", &mut holds),
            "no node 'G:req:passed' (premise of G:inf:s): see G:req:a, G:req:a. and `G:req:a`; \
             not:held stays"
        );
        assert_eq!(
            prefix_ids_in_text("  G:req:a  x", "G", &mut holds),
            "  G:req:a  x"
        );
    }

    #[test]
    fn member_is_taken_out_of_the_arguments_and_must_be_a_graph_id() {
        let mut args: rmcp::model::JsonObject =
            serde_json::from_value(json!({"member": " A ", "query": "q"})).unwrap();
        assert_eq!(take_member(Some(&mut args)), Ok(Some("A".into())));
        assert!(!args.contains_key("member"));
        let mut null: rmcp::model::JsonObject =
            serde_json::from_value(json!({"member": null})).unwrap();
        assert_eq!(take_member(Some(&mut null)), Ok(None));
        for bad in [json!(""), json!(3), json!(["A"])] {
            let mut a = rmcp::model::JsonObject::new();
            a.insert("member".into(), bad);
            assert!(take_member(Some(&mut a)).is_err());
        }
        assert_eq!(take_member(None), Ok(None));
    }

    /// A read of one design's own seams is a read like any other: a view fans
    /// it out, so each member's answer is in the reply.
    #[test]
    fn seam_coverage_is_fanned_out_and_seam_report_is_not() {
        assert!(!NOT_VIEWED.contains(&"seam_coverage"));
        assert!(NOT_VIEWED.contains(&"seam_report"));
    }
}
