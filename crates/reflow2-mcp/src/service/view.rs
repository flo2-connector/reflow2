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
//! 2. **Not viewed** — a tool in [`NOT_VIEWED`]. The hub answers alone, and a
//!    `view_note` says so.
//! 3. **Routed** — a string anywhere in the arguments is `"<member>:<rest>"`.
//!    The prefix is stripped and the SAME tool runs on that member alone, its
//!    ids prefixed back in the reply. Two members named is refused before
//!    anything runs; a member outside the readable set is refused naming the
//!    id and nothing of its content.
//! 4. **A write naming no member** — the hub's own design, exactly as before.
//! 5. **Fan-out** — a read naming no member runs on the hub, then on each
//!    readable member in turn, and answers `by_design`; `search_design` is
//!    the one MERGED reply, every design's hits ranked together.
//!
//! ⭐ ONE DESIGN'S GUARD AT A TIME. Every call here is
//! `call_tool_unshaped` on ONE design, awaited to its end before the next
//! begins, and each takes and drops its own guard inside. So no call ever
//! holds two designs' guards, and never the hub's while a member's is taken —
//! which is what keeps two hubs viewing the same members in opposite orders
//! from deadlocking on tokio's writer-preferring `RwLock`.

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

/// Serve one call made to a hub's address with a view named
/// (`VIEW_GRAPH_IDS` set). Returns the answer already shaped for the client,
/// so `call_tool` hands it back as it is.
pub(super) async fn serve(
    hub: &ReflowService,
    request: CallToolRequestParams,
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

    // 1. NOT ADMITTED: the hub alone, saying why.
    let registry = match admitted(hub) {
        Ok(registry) => registry,
        Err(why) => {
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

    // 2. NOT VIEWED: the hub alone, saying so.
    if NOT_VIEWED.contains(&tool.as_str()) {
        let note = format!(
            "`{tool}` is not served as a view: it acted on the hub's own design ({hub_id}) only, \
             whatever the view names. Call it on a member's own address to act there."
        );
        return shaped(policy, noted(on_one(hub, request, own).await, &note));
    }

    // 3. ROUTED: an id of exactly one member.
    let named_members = members_named(request.arguments.as_ref(), &members);
    match named_members.as_slice() {
        [] => {}
        [member] => {
            return route(
                hub,
                &registry,
                member,
                readable.as_deref(),
                request,
                &context,
                policy,
            )
            .await;
        }
        several => {
            return Err(McpError::invalid_params(
                format!(
                    "REFUSED, and nothing ran: this call names ids in {} designs of this hub's \
                     view ({}): {TWO_DESIGNS}.",
                    several.len(),
                    several.join(", ")
                ),
                None,
            ));
        }
    }

    // 4. A WRITE NAMING NO MEMBER, or arguments the schema refuses: the hub's
    // own design, exactly as before the view existed (a refusal said once,
    // not once per design).
    if is_write || arguments_refused {
        return shaped(policy, on_one(hub, request, own).await);
    }

    // 5. FAN-OUT: the hub, then each readable member, one at a time.
    let forwarded = forwarded(hub, &context);
    let mut replies: Vec<(String, Result<CallToolResponse, McpError>)> = Vec::new();
    replies.push((hub_id.clone(), on_one(hub, request.clone(), own).await));
    for member in &reachable {
        let answer = match registry.member(member).await {
            Ok(held) => on_one(held.service(), request.clone(), forwarded.clone()).await,
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
    if let Some(args) = request.arguments.as_mut() {
        for v in args.values_mut() {
            strip_member(v, member);
        }
    }
    let held = registry
        .member(member)
        .await
        .map_err(|r| member_refusal(member, r))?;
    let answer = on_one(held.service(), request, forwarded(hub, context)).await;
    drop(held);
    let answer = answer.map(|response| match response {
        CallToolResponse::Complete(mut r) if r.is_error != Some(true) => {
            if let Some(v) = r.structured_content.as_mut() {
                prefix_ids(v, member);
            }
            CallToolResponse::Complete(r)
        }
        other => other,
    });
    let note = format!(
        "This call named an id in design {member}, a member of this hub's view, so it ran \
         there alone. Ids that design holds carry its graph_id as a prefix ('{member}:<id>'); \
         pass one back through this hub and the call goes to {member} again."
    );
    shaped(policy, noted(answer, &note))
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
fn strip_member(v: &mut JsonValue, member: &str) {
    match v {
        JsonValue::String(s) => {
            if let Some(rest) = s
                .strip_prefix(member)
                .and_then(|r| r.strip_prefix(':'))
                .map(String::from)
            {
                *s = rest;
            }
        }
        JsonValue::Array(items) => items.iter_mut().for_each(|i| strip_member(i, member)),
        JsonValue::Object(o) => o.values_mut().for_each(|i| strip_member(i, member)),
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
                if elsewhere.contains(k) {
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

    /// A read of one design's own seams is a read like any other: a view fans
    /// it out, so each member's answer is in the reply.
    #[test]
    fn seam_coverage_is_fanned_out_and_seam_report_is_not() {
        assert!(!NOT_VIEWED.contains(&"seam_coverage"));
        assert!(NOT_VIEWED.contains(&"seam_report"));
    }
}
