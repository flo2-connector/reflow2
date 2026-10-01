//! WHO IS CALLING — established from how the engine is served, before any
//! signature is written (#616 fix 4).
//!
//! The settled ruling is
//! `dec:idea-authentication-is-somebody-elses-layer-and-the-line-is-the-contributor-id`
//! (option (e), accepted 2026-09-28), widened on 2026-09-30 to every write path
//! (fact:root-cause-the-settle-rule-guards-the-typed-doors-and-the-generic-writers-go-around-it-2026-09-29,
//! "SCOPE WIDENED"):
//!
//! · A LOCAL engine is unchanged: stdio, `--shared`, and `--http` answering
//!   only loopback. Anyone who can call it can already edit the store, so the
//!   caller's own word is all there is to go on, and no signer is installed.
//! · An engine SERVED FOR OTHERS — a registry of designs (`--registry-root`,
//!   flo2.io's shape), or one deliberately reachable from other machines
//!   (`--http-allow-host` naming a host that is not loopback) — must know who is
//!   calling before anyone can sign. Every AUTHORED_BY a call writes, author or
//!   approver, is then for that contributor, and a call naming anyone else is
//!   refused WHERE THE SIGNATURE IS WRITTEN (`reflow2_core::intent::Signer`),
//!   never at a tool's door.
//!
//! HOW THE CALLER IS ESTABLISHED on an engine served for others:
//! · `--http-trusted-gateway <NAME>` (or `REFLOW2_TRUSTED_GATEWAY`): the
//!   operator declares that a gateway in front authenticates every caller and
//!   names the person on EACH call, in the request's
//!   `_meta["reflow2/writes_for"]`. That name is the caller; a session's own
//!   `writes_for` declaration is not, because the gateway is the only party
//!   the operator trusted to say it.
//! · Nothing declared: reads and proposals work, and no call can sign or move
//!   intent into a settled state. The refusal names the flag.
//!
//! ⚠️ THE RESIDUAL RISK OF A TRUSTED GATEWAY, stated rather than discovered: the
//! engine cannot check that the gateway overwrote a client-sent
//! `reflow2/writes_for`, nor that nothing else reaches the engine's port. Both
//! are properties of the deployment, and declaring the gateway is the
//! operator's statement that they hold.

use reflow2_core::intent::Signer;

/// The request `_meta` key a trusted gateway names the caller with — the same
/// key that already carries who a request writes for.
pub const GATEWAY_NAMES_THE_CALLER_IN: &str = crate::service::WRITES_FOR_META;

/// How THIS engine establishes who is calling. A property of the server,
/// shared by every session it serves, like `read_only`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CallerRule {
    /// Local: the caller's word, as always. No signer is installed.
    #[default]
    Local,
    /// Served for others behind a declared trusted gateway, which names the
    /// caller on each call in `_meta["reflow2/writes_for"]`.
    TrustedGateway {
        /// The gateway's name as the operator declared it (`flo2.io`).
        name: String,
    },
    /// Served for others with no way of establishing the caller declared.
    Undeclared {
        /// What makes this engine one served for others, as a phrase:
        /// "`--registry-root` serves designs for others".
        exposure: String,
    },
}

impl CallerRule {
    /// The rule for an engine served as described. `registry` is
    /// `--registry-root`; `allow_hosts` is every `--http-allow-host`;
    /// `trusted_gateway` is `--http-trusted-gateway`. A trusted gateway on a
    /// LOCAL engine is still honoured — the operator said a gateway names the
    /// caller, and holding that server to it can only narrow what it accepts.
    pub fn for_serving(
        registry: bool,
        allow_hosts: &[String],
        trusted_gateway: Option<&str>,
    ) -> CallerRule {
        if let Some(name) = trusted_gateway.map(str::trim).filter(|n| !n.is_empty()) {
            return CallerRule::TrustedGateway {
                name: name.to_string(),
            };
        }
        let remote: Vec<&str> = allow_hosts
            .iter()
            .map(String::as_str)
            .filter(|h| !is_loopback_entry(h))
            .collect();
        if registry {
            CallerRule::Undeclared {
                exposure: "`--registry-root` serves designs for other people".to_string(),
            }
        } else if !remote.is_empty() {
            CallerRule::Undeclared {
                exposure: format!(
                    "`--http-allow-host {}` makes it reachable from other machines",
                    remote.join(", ")
                ),
            }
        } else {
            CallerRule::Local
        }
    }

    /// Whether this engine is served for others (anything but local).
    pub fn served_for_others(&self) -> bool {
        !matches!(self, CallerRule::Local)
    }

    /// Who the call now arriving writes for. Local: the request's `_meta`,
    /// else the session's declaration (unchanged). Behind a trusted gateway:
    /// ONLY the name the gateway put on this request. Undeclared: unchanged —
    /// attribution only, and it signs nothing there.
    pub fn writes_for(
        &self,
        from_request: Option<&str>,
        from_session: Option<String>,
    ) -> Option<String> {
        let named = from_request
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from);
        match self {
            CallerRule::TrustedGateway { .. } => named,
            CallerRule::Local | CallerRule::Undeclared { .. } => named.or(from_session),
        }
    }

    /// The signer the call now arriving is held to, given who it writes for
    /// (already resolved by [`Self::writes_for`]). `None` on a local engine.
    pub fn signer(&self, writes_for: Option<&str>) -> Option<Signer> {
        match self {
            CallerRule::Local => None,
            CallerRule::TrustedGateway { name } => Some(match writes_for {
                Some(who) => Signer::Caller {
                    contributor: who.to_string(),
                    how: format!(
                        "the trusted gateway `{name}` named them on this call in \
                         _meta[\"{GATEWAY_NAMES_THE_CALLER_IN}\"]"
                    ),
                },
                None => Signer::Nobody {
                    why: format!(
                        "this server takes the caller from its trusted gateway `{name}`, which \
                         named nobody on this call (no _meta[\"{GATEWAY_NAMES_THE_CALLER_IN}\"]). \
                         The gateway must name the signed-in person on every call; a call it \
                         names nobody on can read and propose, and cannot sign or settle."
                    ),
                },
            }),
            CallerRule::Undeclared { exposure } => Some(Signer::Nobody {
                why: format!(
                    "this server is served for others ({exposure}) and declares no way of \
                     establishing who is calling, so nobody can sign or settle intent through \
                     it. An owner signs through a server that knows who they are: start this \
                     one with `--http-trusted-gateway <NAME>` (or REFLOW2_TRUSTED_GATEWAY) when \
                     a gateway in front authenticates every caller and names them on each call \
                     in _meta[\"{GATEWAY_NAMES_THE_CALLER_IN}\"], or sign on a local session \
                     (stdio, --shared, or --http answering loopback only)."
                ),
            }),
        }
    }

    /// The sentence the handshake leads with on an engine served for others,
    /// so every session knows what a signature means here before it writes
    /// one. Empty on a local engine, whose handshake is unchanged.
    pub fn handshake_note(&self) -> String {
        match self {
            CallerRule::Local => String::new(),
            CallerRule::TrustedGateway { name } => format!(
                "🔏 THIS SERVER IS SERVED FOR OTHERS BEHIND A TRUSTED GATEWAY (`{name}`), which \
                 authenticates every caller and names them on each call. Every AUTHORED_BY a \
                 call writes, author or approver, is the caller's own: an approval, an \
                 `approver`, or any signature naming someone else is REFUSED, and nothing is \
                 written. Sign as yourself.\n\n"
            ),
            CallerRule::Undeclared { exposure } => format!(
                "🔏 THIS SERVER IS SERVED FOR OTHERS ({exposure}) AND DOES NOT KNOW WHO IS \
                 CALLING. Reads and proposals work. Nothing can be signed or settled here: an \
                 approver, an accepted or deferred decision, a requirement past proposed, or a \
                 rule's `enforced` is REFUSED, and nothing is written. The owner settles intent \
                 on a server that establishes who they are.\n\n"
            ),
        }
    }

    /// One line for the operator's startup banner.
    pub fn banner(&self) -> Option<String> {
        match self {
            CallerRule::Local => None,
            CallerRule::TrustedGateway { name } => Some(format!(
                "reflow2: signatures are the caller's own — the trusted gateway `{name}` names \
                 the caller on each call in _meta[\"{GATEWAY_NAMES_THE_CALLER_IN}\"], and any \
                 AUTHORED_BY naming someone else is refused. The gateway MUST overwrite that key \
                 on every call and be the only thing that reaches this port."
            )),
            CallerRule::Undeclared { exposure } => Some(format!(
                "reflow2: WARNING — served for others ({exposure}) with no way of establishing \
                 who is calling: reads and proposals work, and every signature or settle is \
                 REFUSED. Declare --http-trusted-gateway <NAME> if a gateway in front \
                 authenticates callers and names them on each call."
            )),
        }
    }
}

/// Whether one `--http-allow-host` entry names loopback only — the hosts
/// every server answers anyway, so listing one exposes nothing.
fn is_loopback_entry(entry: &str) -> bool {
    let host = entry.trim().trim_start_matches('[');
    let host = match host.split_once(']') {
        Some((h, _)) => h,
        None => host.rsplit_once(':').map_or(host, |(h, port)| {
            if port.chars().all(|c| c.is_ascii_digit()) && !h.contains(':') {
                h
            } else {
                host
            }
        }),
    };
    crate::host_gate::LOOPBACK
        .iter()
        .any(|l| l.eq_ignore_ascii_case(host))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_is_every_form_of_loopback_and_nothing_else() {
        assert_eq!(CallerRule::for_serving(false, &[], None), CallerRule::Local);
        for h in [
            "localhost",
            "127.0.0.1:8900",
            "[::1]:80",
            "::1",
            "LOCALHOST:1",
        ] {
            assert_eq!(
                CallerRule::for_serving(false, &[h.to_string()], None),
                CallerRule::Local,
                "{h}"
            );
        }
        assert!(
            CallerRule::for_serving(false, &["reflow2.example.com".into()], None)
                .served_for_others()
        );
        assert!(CallerRule::for_serving(true, &[], None).served_for_others());
        assert_eq!(
            CallerRule::for_serving(true, &[], Some("flo2.io")),
            CallerRule::TrustedGateway {
                name: "flo2.io".into()
            }
        );
    }

    #[test]
    fn behind_a_gateway_only_the_gateways_name_is_the_caller() {
        let g = CallerRule::TrustedGateway { name: "gw".into() };
        assert_eq!(g.writes_for(None, Some("who:session".into())), None);
        assert_eq!(
            g.writes_for(Some("who:alice"), Some("who:session".into())),
            Some("who:alice".into())
        );
        assert!(matches!(
            g.signer(Some("who:alice")),
            Some(Signer::Caller { contributor, .. }) if contributor == "who:alice"
        ));
        assert!(matches!(g.signer(None), Some(Signer::Nobody { .. })));
        assert_eq!(CallerRule::Local.signer(Some("who:alice")), None);
        assert_eq!(
            CallerRule::Local.writes_for(None, Some("who:session".into())),
            Some("who:session".into())
        );
    }
}
