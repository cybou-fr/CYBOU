// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

// Temporary differential oracle built from the actual predecessor Observation implementation.
#include "cybou/protocol/Observation.h"

#include <QTextStream>

int main()
{
    cybou::ObservationV1 observation;
    observation.sourceId = QStringLiteral("nixos.system.generation");
    observation.subject = QStringLiteral("current-generation");
    observation.value = QCborValue(142);
    observation.acquiredAt =
        QDateTime::fromString(QStringLiteral("2026-08-11T09:00:00.000Z"), Qt::ISODateWithMs);
    observation.freshnessUntil = observation.acquiredAt.addSecs(300);
    observation.provenance = QStringLiteral("readlink /run/current-system");

    QTextStream out(stdout);
    out << "payload=" << cybou::encodeObservation(observation).toHex() << '\n';
    out << "message-id="
        << cybou::observationMessageId(
               observation.sourceId,
               observation.subject,
               observation.acquiredAt,
               observation.value)
               .toString(QUuid::WithoutBraces)
        << '\n';
    return 0;
}
