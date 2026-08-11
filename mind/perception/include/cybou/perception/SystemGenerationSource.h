// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/protocol/Observation.h"

#include <QString>

namespace cybou {

/// Why an acquisition did not produce an observation.
///
/// ADR-0027 requires that a source which cannot be read yields a typed result rather than an
/// observation. An adapter that reported "unknown" as a value would put its own failure into the
/// biography as a fact about the world, and nothing downstream could tell the two apart.
enum class AcquisitionStatus {
    /// The source was read and the observation is well formed.
    Acquired,
    /// The source is not present. On a system that is not NixOS, or before the profile exists,
    /// this is the ordinary answer and not an error.
    SourceUnavailable,
    /// The source is present but does not say what it is supposed to say. This is different from
    /// being absent: something is there and it is wrong, which is worth surfacing differently.
    SourceMalformed,
};

QString acquisitionStatusToString(AcquisitionStatus status);

struct AcquisitionResult {
    AcquisitionStatus status{AcquisitionStatus::SourceUnavailable};
    /// Meaningful only when `status` is `Acquired`.
    ObservationV1 observation;
    /// Human-readable detail for the two failure cases. Never used to carry a value.
    QString detail;

    bool acquired() const { return status == AcquisitionStatus::Acquired; }
};

/// Reads the identity of the currently active NixOS system.
///
/// The first perception source, chosen because it is local, non-sensitive, cheaply verifiable, and
/// naturally contradictory: the system generation changes while an earlier observation still claims
/// to be current, which exercises staleness and supersession without any privacy question needing
/// an answer first. ADR-0027 forbids ingesting sensitive observations until a retention ADR exists,
/// and this source is chosen so that constraint costs nothing.
///
/// It only reads. It does not switch generations, modify configuration, or write anything.
class SystemGenerationSource
{
public:
    /// The path is injectable so tests can exercise the real logic against a constructed tree
    /// rather than the host's actual system, which a build sandbox does not have.
    explicit SystemGenerationSource(
        QString systemLinkPath = QStringLiteral("/run/current-system"),
        int freshnessSeconds = 300);

    /// Stable identity of this source, independent of which process runs the adapter.
    static QString sourceId();
    static QString subject();

    AcquisitionResult acquire(const QDateTime &now) const;

private:
    QString m_systemLinkPath;
    int m_freshnessSeconds;
};

} // namespace cybou
