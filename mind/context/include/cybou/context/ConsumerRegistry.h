// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/context/ContextDelivery.h"

#include <QString>

namespace cybou {

/// What a verified consumer is allowed to ask for.
///
/// ADR-0030's B7 gave every destination a trust level; nothing yet decided who may claim one. A
/// caller passing its own trust across the wire is describing itself, and self-description is not
/// authorization however carefully the enum is named.
///
/// This maps a *verified binary name* -- resolved from the caller's process, not from anything it
/// said -- to the most it may ever receive. A caller may ask for less than its ceiling; asking for
/// more is refused rather than quietly reduced, because a consumer handed a narrowed answer to a
/// request it thought was granted would reason as though nothing had been withheld.
class ConsumerRegistry
{
public:
    /// The ceiling for a verified consumer. An unknown or unverifiable caller gets the least
    /// privilege there is, which is the only safe reading of "I could not tell who that was".
    static ConsumerTrust ceilingFor(const QString &verifiedBinaryName);

    /// Whether a request is within a ceiling. Ordering is by enum value, least trusted first.
    static bool permitsRequest(ConsumerTrust ceiling, ConsumerTrust requested)
    {
        return static_cast<int>(requested) <= static_cast<int>(ceiling);
    }
};

} // namespace cybou
