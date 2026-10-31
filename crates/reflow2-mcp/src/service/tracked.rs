//! A DESIGN THAT PINS OTHERS SEARCHES AND READS THEM FROM THEIR COMMITTED
//! EXPORTS — `cap:search-design-reaches-the-designs-a-hub-tracks`, phase 7's
//! second step (`epoch:planned-p7-a-hub-on-one-machine`).
//!
//! A hub on one machine lists its designs as declared dependencies: one
//! `external_dependency` per member, naming the member's `graph_id` and the
//! committed export the hub watches it by. On a hub's address on a registry
//! (flo2.io) the host hands the view its members and `super::view` serves it.
//! Here there is no registry, no gateway and often no MCP at all (a work
//! hub of fifteen designs, reached only through `reflow2 read|write`), so the
//! design's OWN pins are the list, and each pinned design's committed export is
//! loaded READ-ONLY into memory for the one call:
//!
//! 1. **`search_design` with `tracked_designs: true`** runs the query on this
//!    design and on every pinned design whose export reads, and answers in the
//!    hub view's merged shape (`super::view::View::merged_search_payload`):
//!    one ranked list bounded by the one `limit`, each hit naming its
//!    `design`, a pinned design's ids prefixed `<graph_id>:`. Every pinned
//!    design it did NOT search is in `not_searched` with a `state` and why,
//!    so "nothing found" never reads as nothing anywhere.
//! 2. **A read aimed at a pinned design** — `member: "<graph_id>"`, or an id
//!    carrying that design's `<graph_id>:` prefix — runs on that design's
//!    export alone, so a hit can be opened (`get_node`).
//! 3. **A write aimed at one is refused before anything runs**: an export is
//!    the pinned design's record, read here and never written.
//!
//! 🛑 NOTHING IS WRITTEN AND NOTHING IS FETCHED. Each export is read into a
//! fresh in-memory design that serves read-only and reaches no server, and is
//! dropped when the call ends: no member's store or export is touched, no git
//! is run, no network is used. A pin at an address is named `not_reached`.
//! On a server holding many designs (a registry) none of this runs: an export
//! path there names a file on its host.
//!
//! ONE DESIGN'S GUARD AT A TIME, as in the view: every call is made on one
//! design and awaited to its end before the next starts.

use std::collections::HashSet;

use rmcp::model::{CallToolRequestParams, CallToolResponse};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer};
use serde_json::{Map as JsonMap, Value as JsonValue, json};

use super::ReflowService;
use super::view::{self, MEMBER_ARGUMENT, View};
use crate::content_policy::ContentPolicy;

/// The argument that asks `search_design` to search the pinned designs too.
pub(crate) const TRACKED_DESIGNS_ARGUMENT: &str = "tracked_designs";

/// What a hub's address says when asked for `tracked_designs`.
pub(crate) const THROUGH_A_HUBS_ADDRESS: &str = "`tracked_designs` searches the designs a \
     design PINS, read from their committed exports on the machine serving it. Through a hub's \
     address every member of the view is already searched, live, so it changes nothing here.";

/// What a server holding many designs says when asked for `tracked_designs`.
pub(crate) const ON_A_REGISTRY: &str = "`tracked_designs` was not served: this server holds \
     many designs, and an export path a pin names is a file on its host, which it does not \
     read. This search covered this design only; `not_searched` names its pinned designs. \
     Search them through a hub's address, which searches every member of its view, or from \
     the machine that holds their exports.";

/// What a search reached by a path that loads no export says (an in-process
/// caller calling the handler directly, never a client).
pub(crate) const NOT_AT_THE_CALL_LAYER: &str = "`tracked_designs` was not served on this \
     path: the pinned designs' exports are loaded where a tool call is received, and this \
     search reached the handler without passing there. It covered this design only.";

/// Whether a call asks `search_design` for the tracked designs.
pub(crate) fn asks_for_tracked_designs(tool: &str, args: Option<&rmcp::model::JsonObject>) -> bool {
    tool == "search_design"
        && args
            .and_then(|a| a.get(TRACKED_DESIGNS_ARGUMENT))
            .and_then(JsonValue::as_bool)
            .unwrap_or(false)
}

/// What [`serve`] did with a call.
pub(super) enum Served {
    /// Answered here, already shaped for the client.
    Answered(Result<CallToolResponse, McpError>),
    /// Not a call this serves: handed back untouched, to go where every call
    /// went before.
    NotMine(CallToolRequestParams, RequestContext<RoleServer>),
}

/// One design this one pins by graph_id, as its pin states it.
#[derive(Debug, Clone)]
pub(crate) struct Pin {
    pub(crate) design: String,
    pub(crate) name: String,
    pub(crate) dependency: String,
    pub(crate) export: Option<String>,
    pub(crate) address: Option<String>,
}

/// A pinned design read from its export, or why it was not.
enum Reading {
    Read {
        service: Box<ReflowService>,
        path: String,
    },
    NotRead {
        state: &'static str,
        why: String,
    },
}

/// Serve a call made to a design that is NOT a hub's address (no view named),
/// when it is one of the three this module serves; hand anything else back.
pub(super) async fn serve(
    hub: &ReflowService,
    mut request: CallToolRequestParams,
    context: RequestContext<RoleServer>,
    policy: ContentPolicy,
) -> Served {
    // A registry's paths name files on its host: none of this runs there.
    if hub.served_from_registry() {
        return Served::NotMine(request, context);
    }
    let tool = request.name.to_string();
    let Some(served) = hub.tool_router.get(&tool) else {
        return Served::NotMine(request, context);
    };
    let args_value = request.arguments.clone().map(JsonValue::Object);
    let reads = !crate::receipt::is_write(served)
        && !crate::service::writes_a_file(&tool, args_value.as_ref());
    let tracked = asks_for_tracked_designs(&tool, request.arguments.as_ref());
    let has_member = request
        .arguments
        .as_ref()
        .is_some_and(|a| a.contains_key(MEMBER_ARGUMENT));
    let may_route = reads && args_value.as_ref().is_some_and(names_any_prefix);
    if !tracked && !has_member && !may_route {
        return Served::NotMine(request, context);
    }

    // The design's own pins: each design named once, by its first pin.
    let (hub_id, pins) = {
        let g = hub.graph.read().await;
        let declared = match g.declared_dependencies() {
            Ok(d) => d,
            Err(e) => {
                return Served::Answered(Err(McpError::internal_error(
                    format!("could not read this design's declared dependencies: {e}"),
                    None,
                )));
            }
        };
        (g.graph_id().to_string(), pins_of(&declared))
    };
    let pinned: Vec<String> = pins.iter().map(|p| p.design.clone()).collect();

    // AIMED: `member` names the one design the call runs on.
    if has_member {
        let member = match view::take_member(request.arguments.as_mut()) {
            Ok(m) => m,
            Err(why) => return Served::Answered(Err(McpError::invalid_params(why, None))),
        };
        let Some(member) = member else {
            // `member: null` is no aim at all.
            return Served::NotMine(request, context);
        };
        if pins.is_empty() {
            // A design that pins nothing has no member to aim at: the
            // refusal every such call got before.
            return Served::Answered(view::shaped(
                policy,
                Ok(CallToolResponse::Complete(
                    rmcp::model::CallToolResult::error(vec![rmcp::model::ContentBlock::text(
                        view::member_without_a_view(&tool),
                    )]),
                )),
            ));
        }
        let mut designs = vec![member.clone()];
        designs.extend(
            view::members_named(request.arguments.as_ref(), &pinned)
                .into_iter()
                .filter(|n| *n != member),
        );
        if designs.len() > 1 {
            return Served::Answered(Err(two_designs(&designs)));
        }
        if member == hub_id {
            let note = format!(
                "`{MEMBER_ARGUMENT}` named this design's own graph_id ({hub_id}), so `{tool}` ran \
                 here alone."
            );
            let answer = view::on_one(hub, request, context).await;
            return Served::Answered(view::shaped(policy, view::noted(answer, &note)));
        }
        let Some(pin) = pins.iter().find(|p| p.design == member) else {
            return Served::Answered(Err(McpError::invalid_params(
                format!(
                    "REFUSED, and nothing ran: `{MEMBER_ARGUMENT}` names design {member}, which \
                     this design ({hub_id}) does not pin. Outside a hub's address, `{MEMBER_ARGUMENT}` \
                     aims a READ at a design this one pins by its committed export: {}.",
                    pinned.join(", ")
                ),
                None,
            )));
        };
        return Served::Answered(route(pin, reads, &tool, request, context, policy).await);
    }

    // THE TRACKED SEARCH.
    if tracked {
        return Served::Answered(search(hub, &hub_id, &pins, request, context, policy).await);
    }

    // ROUTED: a read naming ids of exactly one pinned design.
    match view::members_named(request.arguments.as_ref(), &pinned).as_slice() {
        [] => Served::NotMine(request, context),
        [one] => {
            let pin = pins
                .iter()
                .find(|p| &p.design == one)
                .expect("named among the pinned designs")
                .clone();
            Served::Answered(route(&pin, reads, &tool, request, context, policy).await)
        }
        several => Served::Answered(Err(two_designs(several))),
    }
}

/// Whether any string in the arguments has a `:` — the cheap test before the
/// pins are read, since a pinned design's ids are `<graph_id>:<id>`.
fn names_any_prefix(v: &JsonValue) -> bool {
    match v {
        JsonValue::String(s) => s.contains(':'),
        JsonValue::Array(items) => items.iter().any(names_any_prefix),
        JsonValue::Object(o) => o.values().any(names_any_prefix),
        _ => false,
    }
}

fn two_designs(designs: &[String]) -> McpError {
    McpError::invalid_params(
        format!(
            "REFUSED, and nothing ran: this call names {} designs ({}). A read runs on one \
             design at a time: name one, or search them together with search_design and \
             `{TRACKED_DESIGNS_ARGUMENT}: true`.",
            designs.len(),
            designs.join(", ")
        ),
        None,
    )
}

/// The designs a declaration list pins, once each, in declaration order. A
/// pin naming no graph_id is a code dependency, not a design.
pub(crate) fn pins_of(declared: &[reflow2_core::DependencyDeclaration]) -> Vec<Pin> {
    let stated = |v: &Option<String>| {
        v.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let mut out: Vec<Pin> = Vec::new();
    for d in declared {
        let Some(design) = stated(&d.graph_id) else {
            continue;
        };
        if out.iter().any(|p| p.design == design) {
            continue;
        }
        out.push(Pin {
            design,
            name: d.name.clone(),
            dependency: d.id.clone(),
            export: stated(&d.design_export),
            address: stated(&d.design_address),
        });
    }
    out
}

/// Read one pinned design's committed export into a fresh in-memory design
/// that serves read-only and reaches no server — or say, in a `state` and a
/// sentence, why it could not be.
fn read(pin: &Pin) -> Reading {
    let (doc, path) = match read_document(pin) {
        Ok(read) => read,
        Err((state, why)) => return Reading::NotRead { state, why },
    };
    let opened = reflow2_core::DesignGraph::open_in_memory_as(&pin.design).and_then(|mut g| {
        g.import_graph(&doc)?;
        Ok(g)
    });
    match opened {
        Ok(g) => Reading::Read {
            service: Box::new(
                ReflowService::wrap(g)
                    .without_reaching_out()
                    .into_read_only(),
            ),
            path,
        },
        Err(e) => Reading::NotRead {
            state: "unreadable",
            why: format!("the export at {path} could not be loaded: {e}"),
        },
    }
}

/// THE ONE RESOLUTION OF A PIN TO ITS COMMITTED EXPORT: the document and the
/// path it was read from, or a `state` and a sentence saying why it could not
/// be read. The tracked search loads the document into a searchable design
/// ([`read`]); a composition needs only the document, so it stops here.
pub(crate) fn read_document(
    pin: &Pin,
) -> Result<(reflow2_core::GraphExport, String), (&'static str, String)> {
    let not = |state: &'static str, why: String| Err((state, why));
    let Some(path) = pin.export.as_deref() else {
        return match pin.address.as_deref() {
            Some(address) => not(
                "not_reached",
                format!(
                    "pinned at the server {address}; this reads committed exports on this \
                     machine and reaches no server, so search that design where it is served, \
                     or pin it by its committed export"
                ),
            ),
            None => not(
                "nowhere",
                "pinned with nowhere to read it: the pin names neither an export nor an address"
                    .to_string(),
            ),
        };
    };
    if !std::path::Path::new(path).exists() {
        return not(
            "missing",
            format!(
                "pinned by a committed export at {path}, which is missing on this machine, so \
                 nothing could read it"
            ),
        );
    }
    let doc = match crate::saved_design::read_export(path) {
        Ok(doc) => doc,
        Err(e) => {
            return not(
                "unreadable",
                format!("the file at {path} could not be read as a design: {e}"),
            );
        }
    };
    if doc.is_unidentified() {
        return not(
            "unreadable",
            format!("the file at {path} names no design (it carries no graph_id)"),
        );
    }
    if doc.graph_id != pin.design {
        return not(
            "unreadable",
            format!(
                "the export at {path} holds design {}, not {} as the pin {} says, so it was not \
                 read as that design; re-declare the pin with the right graph_id or export",
                doc.graph_id, pin.design, pin.dependency
            ),
        );
    }
    Ok((doc, path.to_string()))
}

/// One pinned design that was not searched, in `not_searched`'s shape — and
/// not composed, in `not_composed`'s, which is the same.
pub(crate) fn not_searched_entry(pin: &Pin, state: &str, why: &str) -> JsonValue {
    let read_at = match (&pin.export, &pin.address) {
        (Some(path), _) => json!({"export": path}),
        (None, Some(address)) => json!({"address": address}),
        (None, None) => json!({}),
    };
    json!({
        "design": pin.design,
        "name": pin.name,
        "dependency": pin.dependency,
        "read_at": read_at,
        "state": state,
        "why": why,
    })
}

/// Run a call on one pinned design's export alone: a READ, its prefix taken
/// off the ids it passes and put back on the ids in the reply.
async fn route(
    pin: &Pin,
    reads: bool,
    tool: &str,
    mut request: CallToolRequestParams,
    context: RequestContext<RoleServer>,
    policy: ContentPolicy,
) -> Result<CallToolResponse, McpError> {
    let member = pin.design.as_str();
    if !reads {
        return Err(McpError::invalid_params(
            format!(
                "REFUSED, and nothing ran: `{tool}` writes, and design {member} is pinned here by \
                 its committed export, which this design reads and never writes. Write it in \
                 that design's own store, export it there, then re-declare this pin \
                 (external_dependency with the same id) to take the new baseline."
            ),
            None,
        ));
    }
    let (service, path) = match read(pin) {
        Reading::Read { service, path } => (service, path),
        Reading::NotRead { state, why } => {
            return Err(McpError::invalid_params(
                format!(
                    "REFUSED, and nothing ran: `{tool}` was aimed at design {member} ({state}): \
                     {why}."
                ),
                None,
            ));
        }
    };
    let mut passed = HashSet::new();
    if let Some(args) = request.arguments.as_mut() {
        // Searching one export alone is not a tracked search.
        args.remove(TRACKED_DESIGNS_ARGUMENT);
        for v in args.values_mut() {
            view::strip_member_noting(v, member, &mut passed);
        }
    }
    let answer = view::on_one(&service, request, context).await;
    let answer = view::prefix_reply(answer, member, &service, &passed).await;
    let note = format!(
        "This read ran on design {member} alone, from its committed export at {path}, read-only: \
         the design as of when that export was taken, not its live store. Ids that design holds \
         carry its graph_id as a prefix ('{member}:<id>'); pass one back here, or pass \
         `{MEMBER_ARGUMENT}: \"{member}\"`, and the read goes to that export again."
    );
    view::shaped(policy, view::noted(answer, &note))
}

/// `search_design` over this design and every design it pins whose export
/// reads, merged as a hub's view merges.
async fn search(
    hub: &ReflowService,
    hub_id: &str,
    pins: &[Pin],
    mut request: CallToolRequestParams,
    context: RequestContext<RoleServer>,
    policy: ContentPolicy,
) -> Result<CallToolResponse, McpError> {
    if let Some(args) = request.arguments.as_mut() {
        args.remove(TRACKED_DESIGNS_ARGUMENT);
    }
    let own = view::on_one(hub, request.clone(), context.clone()).await;
    if pins.is_empty() {
        let note = "This design pins no other design (no external_dependency here names a \
                    graph_id), so `tracked_designs` had nothing to add: the search covered this \
                    design alone.";
        return view::shaped(policy, with_key(own, "tracked_note", note));
    }
    let mut replies = vec![(hub_id.to_string(), own)];
    let mut searched: Vec<String> = Vec::new();
    let mut read_from = JsonMap::new();
    let mut names = JsonMap::new();
    let mut unsearched: Vec<JsonValue> = Vec::new();
    for (i, pin) in pins.iter().enumerate() {
        if i >= crate::upstream::MAX_UPSTREAMS_READ {
            unsearched.push(not_searched_entry(
                pin,
                "not_read",
                &format!(
                    "this search reads at most {} pinned designs' exports, because each is a \
                     whole document loaded for the call; search it with `{MEMBER_ARGUMENT}: \
                     \"{}\"`",
                    crate::upstream::MAX_UPSTREAMS_READ,
                    pin.design
                ),
            ));
            continue;
        }
        match read(pin) {
            Reading::Read { service, path } => {
                let answer = view::on_one(&service, request.clone(), context.clone()).await;
                let answer =
                    view::prefix_reply(answer, &pin.design, &service, &HashSet::new()).await;
                replies.push((pin.design.clone(), answer));
                searched.push(pin.design.clone());
                read_from.insert(pin.design.clone(), JsonValue::String(path));
                names.insert(pin.design.clone(), JsonValue::String(pin.name.clone()));
            }
            Reading::NotRead { state, why } => {
                unsearched.push(not_searched_entry(pin, state, &why));
            }
        }
    }
    let merged = View {
        hub_id,
        members: &searched,
        unreachable: &[],
        tool: "search_design",
    };
    let mut payload = merged.merged_search_payload(replies);
    let Some(o) = payload.as_object_mut() else {
        return Err(McpError::internal_error(
            "the merged search did not answer with an object",
            None,
        ));
    };
    // The hub's own reply named every pin as not searched; the list is now
    // the designs this search could not reach. A pinned design's OWN pins are
    // kept, each saying it was not followed.
    let mut beyond: Vec<JsonValue> = o
        .remove("not_searched")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p["pinned_by"].as_str() != Some(hub_id))
        .filter(|p| {
            let d = p["design"].as_str().unwrap_or_default();
            d != hub_id && !pins.iter().any(|x| x.design == d)
        })
        .collect();
    for p in &mut beyond {
        let by = p["pinned_by"].as_str().unwrap_or_default().to_string();
        let design = p["design"].as_str().unwrap_or_default().to_string();
        p["state"] = json!("not_followed");
        p["why"] = JsonValue::String(format!(
            "pinned by design {by}, whose export was searched; a pinned design's own pins are \
             not followed from here, so search {design} where it is held"
        ));
    }
    unsearched.extend(beyond);
    o.remove("view");
    o.remove("unreachable");
    o.remove("view_note");
    o.remove("not_searched_note");
    let k = searched.len();
    let missed = unsearched.len();
    let not_sentence = if missed == 0 {
        "Every design it pins was searched.".to_string()
    } else {
        format!(
            "{missed} design(s) it pins were NOT searched, each named in `not_searched` with its \
             `state` and why, so a hit missing here says nothing about them."
        )
    };
    if o.contains_key("empty_because") {
        o.insert(
            "empty_because".into(),
            JsonValue::String(format!(
                "no design searched matched the query: this design and {k} pinned design(s) \
                 read from their exports; `searched_by_design` says how many nodes each search \
                 ran over. {not_sentence}"
            )),
        );
    }
    o.insert(
        "view_note".into(),
        JsonValue::String(format!(
            "search_design over this design ({hub_id}) and the {k} design(s) it pins whose \
             committed exports were read on this machine (`read_from`), read-only and as of when \
             each was exported: every design's hits ranked together by score, `design` naming \
             where each came from, a pinned design's node_id prefixed with its graph_id \
             ('<graph_id>:<id>', which get_node opens) and this design's own unprefixed. Scores \
             come from each design's own index, so the order across designs is a guide, not a \
             measurement. The one `limit` bounds the merged list: `found_by_design` says how \
             many each returned and `hits_withheld` counts what fell outside it. {not_sentence}"
        )),
    );
    o.insert("read_from".into(), JsonValue::Object(read_from));
    o.insert("design_names".into(), JsonValue::Object(names));
    o.insert("not_searched".into(), JsonValue::Array(unsearched));
    let sentence = format!(
        "search_design over this design and {k} pinned design(s) read from their committed \
         exports, the hits merged and ranked together; {missed} pinned design(s) not searched."
    );
    view::view_result(payload, policy, &sentence)
}

/// Put `value` under `key` in a successful reply's structured payload.
fn with_key(
    answer: Result<CallToolResponse, McpError>,
    key: &str,
    value: &str,
) -> Result<CallToolResponse, McpError> {
    answer.map(|response| match response {
        CallToolResponse::Complete(mut r) if r.is_error != Some(true) => {
            if let Some(o) = r
                .structured_content
                .as_mut()
                .and_then(JsonValue::as_object_mut)
            {
                o.insert(key.into(), JsonValue::String(value.to_string()));
            }
            CallToolResponse::Complete(r)
        }
        other => other,
    })
}
