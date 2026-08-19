// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Capability declarations, component dependencies, and command gates.

use serde::Serialize;

/// One user-visible capability, the components it depends on, and what its loss costs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityDeclaration {
    /// Stable capability identifier.
    pub capability_id: &'static str,
    /// System component/daemon names required for this capability.
    pub components: &'static [&'static str],
    /// Required capabilities make Mind unusable when lost; optional ones degrade it.
    pub required: bool,
    /// Human-readable explanation of impact when unavailable.
    pub unavailable_impact: &'static str,
}

/// One Presence command and the capabilities it needs before it may be attempted.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandDeclaration {
    /// Command identifier.
    pub command_id: &'static str,
    /// Required capability identifiers.
    pub required_capabilities: &'static [&'static str],
}

/// Registry of all standard Mind capabilities, component dependencies, and commands.
pub struct CapabilityRegistry;

impl CapabilityRegistry {
    /// Return all capability declarations in canonical projection order.
    #[must_use]
    pub const fn capabilities() -> &'static [CapabilityDeclaration] {
        &[
            CapabilityDeclaration {
                capability_id: "accepted-biography",
                components: &["eventd"],
                required: true,
                unavailable_impact: "accepted cognitive history is unavailable",
            },
            CapabilityDeclaration {
                capability_id: "identity-continuity",
                components: &["eventd", "identityd"],
                required: true,
                unavailable_impact: "identity continuity cannot be verified",
            },
            CapabilityDeclaration {
                capability_id: "commitment-access",
                components: &["eventd", "intentiond"],
                required: true,
                unavailable_impact: "accepted commitments are unavailable",
            },
            CapabilityDeclaration {
                capability_id: "prediction",
                components: &["predictord"],
                required: false,
                unavailable_impact: "new predictions are unavailable",
            },
            CapabilityDeclaration {
                capability_id: "self-assessment",
                components: &["selfd"],
                required: false,
                unavailable_impact: "self assessment is unavailable",
            },
            CapabilityDeclaration {
                capability_id: "attention-workspace",
                components: &["workspaced"],
                required: false,
                unavailable_impact: "bounded attention is unavailable",
            },
            CapabilityDeclaration {
                capability_id: "consolidation",
                components: &["lifecycled", "predictord", "workspaced"],
                required: false,
                unavailable_impact: "consolidation is limited by an unavailable owner",
            },
            CapabilityDeclaration {
                capability_id: "presence-presentation",
                components: &["presenced"],
                required: false,
                unavailable_impact: "Mind presentation is unavailable",
            },
            CapabilityDeclaration {
                capability_id: "epistemic-projection",
                components: &["eventd", "epistemicd"],
                required: false,
                unavailable_impact: "what is known, and how stale or disputed it is, cannot be told",
            },
            CapabilityDeclaration {
                capability_id: "associative-context",
                components: &["eventd", "contextd"],
                required: false,
                unavailable_impact: "what is related to what cannot be retrieved",
            },
            CapabilityDeclaration {
                capability_id: "local-perception",
                components: &["eventd", "perceptiond"],
                required: false,
                unavailable_impact: "grounded observation of the local system is unavailable",
            },
        ]
    }

    /// Return all components whose health is observed.
    #[must_use]
    pub const fn component_ids() -> &'static [&'static str] {
        &[
            "eventd",
            "lifecycled",
            "identityd",
            "intentiond",
            "predictord",
            "selfd",
            "workspaced",
            "presenced",
            "perceptiond",
            "epistemicd",
            "contextd",
        ]
    }

    /// Return all Presence commands.
    #[must_use]
    pub const fn commands() -> &'static [CommandDeclaration] {
        &[
            CommandDeclaration {
                command_id: "activity",
                required_capabilities: &["accepted-biography"],
            },
            CommandDeclaration {
                command_id: "promise",
                required_capabilities: &["accepted-biography", "commitment-access"],
            },
            CommandDeclaration {
                command_id: "reflect",
                required_capabilities: &["accepted-biography", "self-assessment"],
            },
            CommandDeclaration {
                command_id: "fulfill",
                required_capabilities: &["commitment-access"],
            },
            CommandDeclaration {
                command_id: "abandon",
                required_capabilities: &["commitment-access"],
            },
            CommandDeclaration {
                command_id: "observe",
                required_capabilities: &["prediction"],
            },
            CommandDeclaration {
                command_id: "predict",
                required_capabilities: &["prediction"],
            },
            CommandDeclaration {
                command_id: "identity",
                required_capabilities: &["identity-continuity"],
            },
            CommandDeclaration {
                command_id: "attention",
                required_capabilities: &["attention-workspace"],
            },
        ]
    }

    /// Required capabilities for a command, or `None` if unknown (fail-closed gating).
    #[must_use]
    pub fn required_capabilities_for(command_id: &str) -> Option<&'static [&'static str]> {
        for cmd in Self::commands() {
            if cmd.command_id == command_id {
                return Some(cmd.required_capabilities);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_registry_invariants() {
        let caps = CapabilityRegistry::capabilities();
        assert_eq!(caps.len(), 11);
        assert!(caps[0].required);
        assert_eq!(caps[0].capability_id, "accepted-biography");

        let cmds = CapabilityRegistry::commands();
        assert_eq!(cmds.len(), 9);

        let required = CapabilityRegistry::required_capabilities_for("identity");
        assert_eq!(required, Some(&["identity-continuity"][..]));

        // Unknown command must fail closed by returning None
        let unknown = CapabilityRegistry::required_capabilities_for("unknown-cmd");
        assert_eq!(unknown, None);
    }
}
