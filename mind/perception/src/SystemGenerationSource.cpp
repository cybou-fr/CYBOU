// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/perception/SystemGenerationSource.h"

#include <QCborValue>
#include <QFileInfo>

namespace cybou {

QString acquisitionStatusToString(AcquisitionStatus status)
{
    switch (status) {
    case AcquisitionStatus::Acquired: return QStringLiteral("acquired");
    case AcquisitionStatus::SourceUnavailable: return QStringLiteral("source-unavailable");
    case AcquisitionStatus::SourceMalformed: return QStringLiteral("source-malformed");
    }
    return QStringLiteral("unknown");
}

SystemGenerationSource::SystemGenerationSource(QString systemLinkPath, int freshnessSeconds)
    : m_systemLinkPath(std::move(systemLinkPath))
    , m_freshnessSeconds(freshnessSeconds > 0 ? freshnessSeconds : 300)
{
}

QString SystemGenerationSource::sourceId()
{
    return QStringLiteral("nixos.system");
}

QString SystemGenerationSource::subject()
{
    return QStringLiteral("current-system");
}

AcquisitionResult SystemGenerationSource::acquire(const QDateTime &now) const
{
    AcquisitionResult result;

    const QFileInfo info(m_systemLinkPath);
    if (!info.isSymLink()) {
        // Absent, or present but not the symlink this source is defined in terms of. Either way
        // there is nothing here to observe, and saying so is the honest answer.
        result.status = AcquisitionStatus::SourceUnavailable;
        result.detail = info.exists()
            ? QStringLiteral("%1 is not a symbolic link").arg(m_systemLinkPath)
            : QStringLiteral("%1 does not exist").arg(m_systemLinkPath);
        return result;
    }

    const QString target = info.symLinkTarget();
    if (target.isEmpty()) {
        result.status = AcquisitionStatus::SourceMalformed;
        result.detail = QStringLiteral("%1 resolves to nothing").arg(m_systemLinkPath);
        return result;
    }

    // The store path is the build identity: two systems with the same path are the same build, and
    // any configuration change produces a different one. That is what makes this worth observing
    // and what makes a later observation able to supersede this one.
    const QString buildIdentity = QFileInfo(target).fileName();
    if (buildIdentity.isEmpty()) {
        result.status = AcquisitionStatus::SourceMalformed;
        result.detail = QStringLiteral("%1 resolves to %2, which has no final component")
                            .arg(m_systemLinkPath, target);
        return result;
    }

    ObservationV1 observation;
    observation.sourceId = sourceId();
    observation.subject = subject();
    observation.value = QCborValue(buildIdentity);
    observation.acquiredAt = now.toUTC();
    observation.freshnessUntil = observation.acquiredAt.addSecs(m_freshnessSeconds);
    observation.provenance =
        QStringLiteral("symlink target of %1 resolved to %2").arg(m_systemLinkPath, target);

    if (!observation.isValid()) {
        // Should be unreachable, but an adapter must never hand an invalid observation onward:
        // Event1 would reject it anyway, and a malformed payload reported as a successful
        // acquisition would be the adapter lying about its own outcome.
        result.status = AcquisitionStatus::SourceMalformed;
        result.detail = QStringLiteral("constructed observation failed structural validation");
        return result;
    }

    result.status = AcquisitionStatus::Acquired;
    result.observation = observation;
    return result;
}

} // namespace cybou
