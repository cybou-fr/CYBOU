// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include <QString>
#include <QStringList>

namespace cybou {

/// One user-visible capability, the components it depends on, and what its loss costs.
struct CapabilityDeclaration {
    QString capabilityId;
    QStringList components;
    /// Required capabilities make Mind unusable when lost; optional ones degrade it.
    bool required{false};
    QString unavailableImpact;
};

/// One Presence command and the capabilities it needs before it may be attempted.
struct CommandDeclaration {
    QString commandId;
    QStringList requiredCapabilities;
};

/// The single declaration of what Mind can do and what each ability rests on.
///
/// This is policy, not state. It says nothing about whether anything is currently healthy - healthd
/// remains the sole owner of that, and reading this registry does not make any other process a
/// second authority on capability health.
///
/// It exists because the same knowledge was previously written down three times: the dependency
/// graph in healthd, the command-to-capability map in the Presence projection, and again in the
/// capability gates of each Presence mutation. Three copies agreed by hand and by luck. M7 adds
/// perception, epistemic projection and retention capabilities to all of them at once, which is
/// where hand-maintained agreement stops holding.
namespace CapabilityRegistry {

/// Every capability, in the order Presence projects them.
QList<CapabilityDeclaration> capabilities();

/// Every component whose health can be observed.
QStringList componentIds();

/// Every command Presence exposes.
QList<CommandDeclaration> commands();

/// Capabilities a command requires, or an empty list for an unknown command.
///
/// An unknown command is deliberately unrestricted rather than blocked: this registry declares
/// requirements, and inventing one for a command it has never heard of would be a decision it has
/// no basis to make. A command that needs gating must say so here.
QStringList requiredCapabilitiesFor(const QString &commandId);

/// Capability ids alone, for projections that iterate them.
QStringList capabilityIds();

} // namespace CapabilityRegistry

} // namespace cybou
