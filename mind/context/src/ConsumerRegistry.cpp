// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/context/ConsumerRegistry.h"

#include <QSet>

namespace cybou {

ConsumerTrust ConsumerRegistry::ceilingFor(const QString &verifiedBinaryName)
{
    if (verifiedBinaryName.isEmpty()) {
        return ConsumerTrust::Untrusted;
    }

    // Mind's own organs are bounded consumers. Bounded rather than Full even for first-party code:
    // an organ needs enough context to do its work, and no organ so far has a reason to see the
    // person's most restricted material. Granting Full by default would make the level meaningless
    // on the day something actually deserved it.
    static const QSet<QString> boundedOrgans{
        QStringLiteral("eventd"),
        QStringLiteral("healthd"),
        QStringLiteral("lifecycled"),
        QStringLiteral("identityd"),
        QStringLiteral("intentiond"),
        QStringLiteral("predictord"),
        QStringLiteral("selfd"),
        QStringLiteral("workspaced"),
        QStringLiteral("presenced"),
        QStringLiteral("perceptiond"),
        QStringLiteral("epistemicd"),
        QStringLiteral("contextd"),
    };

    if (boundedOrgans.contains(verifiedBinaryName)) {
        return ConsumerTrust::Bounded;
    }

    // Nothing is granted Full yet. The person-facing inspector is the consumer that will need it,
    // and it does not exist: naming it here in advance would grant the level to whatever later
    // happened to carry that name.
    return ConsumerTrust::Untrusted;
}

} // namespace cybou
