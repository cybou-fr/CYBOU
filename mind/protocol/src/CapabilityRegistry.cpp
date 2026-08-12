// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/protocol/CapabilityRegistry.h"

namespace cybou {
namespace CapabilityRegistry {

QList<CapabilityDeclaration> capabilities()
{
    static const QList<CapabilityDeclaration> declarations{
        {QStringLiteral("accepted-biography"),
         {QStringLiteral("eventd")},
         true,
         QStringLiteral("accepted cognitive history is unavailable")},
        {QStringLiteral("identity-continuity"),
         {QStringLiteral("eventd"), QStringLiteral("identityd")},
         true,
         QStringLiteral("identity continuity cannot be verified")},
        {QStringLiteral("commitment-access"),
         {QStringLiteral("eventd"), QStringLiteral("intentiond")},
         true,
         QStringLiteral("accepted commitments are unavailable")},
        {QStringLiteral("prediction"),
         {QStringLiteral("predictord")},
         false,
         QStringLiteral("new predictions are unavailable")},
        {QStringLiteral("self-assessment"),
         {QStringLiteral("selfd")},
         false,
         QStringLiteral("self assessment is unavailable")},
        {QStringLiteral("attention-workspace"),
         {QStringLiteral("workspaced")},
         false,
         QStringLiteral("bounded attention is unavailable")},
        {QStringLiteral("consolidation"),
         {QStringLiteral("lifecycled"),
          QStringLiteral("predictord"),
          QStringLiteral("workspaced")},
         false,
         QStringLiteral("consolidation is limited by an unavailable owner")},
        {QStringLiteral("presence-presentation"),
         {QStringLiteral("presenced")},
         false,
         QStringLiteral("Mind presentation is unavailable")},
        {QStringLiteral("local-perception"),
         {QStringLiteral("eventd"), QStringLiteral("perceptiond")},
         false,
         QStringLiteral("grounded observation of the local system is unavailable")},
    };
    return declarations;
}

QStringList componentIds()
{
    static const QStringList components{
        QStringLiteral("eventd"),
        QStringLiteral("lifecycled"),
        QStringLiteral("identityd"),
        QStringLiteral("intentiond"),
        QStringLiteral("predictord"),
        QStringLiteral("selfd"),
        QStringLiteral("workspaced"),
        QStringLiteral("presenced"),
        QStringLiteral("perceptiond"),
    };
    return components;
}

QList<CommandDeclaration> commands()
{
    // Every command that reads or writes through Presence. `interruptLifecycle` is absent on
    // purpose: it is gated on lifecycled being reachable rather than on a capability, because
    // lifecycle mode is orthogonal to capability health and a run must remain interruptible even
    // when other capabilities are degraded.
    static const QList<CommandDeclaration> declarations{
        {QStringLiteral("activity"), {QStringLiteral("accepted-biography")}},
        {QStringLiteral("promise"),
         {QStringLiteral("accepted-biography"), QStringLiteral("commitment-access")}},
        {QStringLiteral("reflect"),
         {QStringLiteral("accepted-biography"), QStringLiteral("self-assessment")}},
        {QStringLiteral("fulfill"), {QStringLiteral("commitment-access")}},
        {QStringLiteral("abandon"), {QStringLiteral("commitment-access")}},
        {QStringLiteral("observe"), {QStringLiteral("prediction")}},
        {QStringLiteral("predict"), {QStringLiteral("prediction")}},
        {QStringLiteral("identity"), {QStringLiteral("identity-continuity")}},
        {QStringLiteral("attention"), {QStringLiteral("attention-workspace")}},
    };
    return declarations;
}

QStringList requiredCapabilitiesFor(const QString &commandId)
{
    for (const CommandDeclaration &command : commands()) {
        if (command.commandId == commandId) {
            return command.requiredCapabilities;
        }
    }
    return {};
}

QStringList capabilityIds()
{
    QStringList ids;
    for (const CapabilityDeclaration &capability : capabilities()) {
        ids.append(capability.capabilityId);
    }
    return ids;
}

} // namespace CapabilityRegistry
} // namespace cybou
