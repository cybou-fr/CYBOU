// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/protocol/Observation.h"

#include <QCborArray>
#include <QCborMap>

namespace cybou {

namespace {

// Distinct from every other deterministic identity in Mind. Sharing a namespace with lifecycle
// operation keys would let an observation and a consolidation collide on one messageId.
const QUuid kObservationNamespace(QStringLiteral("9f2c1d84-6b3a-5e07-bc41-0d2a7f9e5c13"));

void setError(QString *error, const QString &message)
{
    if (error) {
        *error = message;
    }
}

} // namespace

bool ObservationV1::isValid() const
{
    if (schemaVersion != kCurrentObservationSchemaVersion) {
        return false;
    }
    if (sourceId.trimmed().isEmpty() || subject.trimmed().isEmpty()) {
        return false;
    }
    if (provenance.trimmed().isEmpty()) {
        return false;
    }
    // An absent value is not an observation of nothing; it is a failure to observe, which an
    // adapter must report as a typed source failure rather than smuggle in as a contribution.
    if (value.isNull() || value.isUndefined()) {
        return false;
    }
    if (!acquiredAt.isValid() || !freshnessUntil.isValid()) {
        return false;
    }
    // A horizon at or before acquisition would describe an observation that was never current, so
    // nothing could ever legitimately act on it.
    return freshnessUntil > acquiredAt;
}

bool ObservationV1::isFreshAt(const QDateTime &at) const
{
    if (!at.isValid() || !acquiredAt.isValid() || !freshnessUntil.isValid()) {
        return false;
    }
    return acquiredAt <= at && at < freshnessUntil;
}

QByteArray encodeObservation(const ObservationV1 &observation)
{
    QCborMap map;
    map.insert(
        QStringLiteral("@type"), QString::fromLatin1(kObservationPayloadType));
    map.insert(QStringLiteral("schemaVersion"), observation.schemaVersion);
    map.insert(QStringLiteral("sourceId"), observation.sourceId);
    map.insert(QStringLiteral("subject"), observation.subject);
    map.insert(QStringLiteral("value"), observation.value);
    map.insert(
        QStringLiteral("acquiredAt"),
        observation.acquiredAt.toUTC().toString(Qt::ISODateWithMs));
    map.insert(
        QStringLiteral("freshnessUntil"),
        observation.freshnessUntil.toUTC().toString(Qt::ISODateWithMs));
    map.insert(QStringLiteral("provenance"), observation.provenance);
    return map.toCborValue().toCbor();
}

std::optional<ObservationV1> decodeObservation(const QByteArray &encoded, QString *error)
{
    setError(error, QString());

    const QCborValue root = QCborValue::fromCbor(encoded);
    if (!root.isMap()) {
        setError(error, QStringLiteral("observation payload is not a map"));
        return std::nullopt;
    }
    const QCborMap map = root.toMap();

    // Checked before anything else. A payload without this tag is not a malformed observation, it
    // is some other kind of Observation payload entirely, and reading further would be an attempt
    // to interpret it as something it never claimed to be.
    if (map.value(QStringLiteral("@type")).toString()
        != QString::fromLatin1(kObservationPayloadType)) {
        setError(error, QStringLiteral("payload is not an ObservationV1"));
        return std::nullopt;
    }

    const QCborValue version = map.value(QStringLiteral("schemaVersion"));
    if (!version.isInteger()) {
        setError(error, QStringLiteral("observation has no schema version"));
        return std::nullopt;
    }
    if (version.toInteger() != kCurrentObservationSchemaVersion) {
        // Deliberately refuses both older and newer. There is exactly one schema so far, so any
        // other number means the payload was written by something this build cannot interpret, and
        // guessing at evidence is worse than declining to read it.
        setError(
            error,
            QStringLiteral("observation schema %1 is not supported")
                .arg(version.toInteger()));
        return std::nullopt;
    }

    ObservationV1 observation;
    observation.schemaVersion = static_cast<quint16>(version.toInteger());
    observation.sourceId = map.value(QStringLiteral("sourceId")).toString();
    observation.subject = map.value(QStringLiteral("subject")).toString();
    observation.value = map.value(QStringLiteral("value"));
    observation.acquiredAt = QDateTime::fromString(
        map.value(QStringLiteral("acquiredAt")).toString(), Qt::ISODateWithMs);
    observation.freshnessUntil = QDateTime::fromString(
        map.value(QStringLiteral("freshnessUntil")).toString(), Qt::ISODateWithMs);
    observation.provenance = map.value(QStringLiteral("provenance")).toString();

    if (!observation.isValid()) {
        setError(error, QStringLiteral("observation failed structural validation"));
        return std::nullopt;
    }

    return observation;
}

QUuid observationMessageId(
    const QString &sourceId,
    const QString &subject,
    const QDateTime &acquiredAt,
    const QCborValue &value)
{
    // A CBOR array, not a delimited string. Each element carries its own length, so no content
    // inside a field can be mistaken for the boundary between two fields. The separator this
    // replaces was only sound if it could not occur inside sourceId or subject, and nothing forbade
    // that: ("a", "b<sep>c") and ("a<sep>b", "c") serialised identically.
    //
    // Acquisition time is reduced to UTC milliseconds so one instant expressed in two zones is one
    // acquisition; otherwise every observation would duplicate across a DST change.
    QCborArray key;
    key.append(sourceId);
    key.append(subject);
    key.append(acquiredAt.toUTC().toMSecsSinceEpoch());
    key.append(value);
    return QUuid::createUuidV5(kObservationNamespace, key.toCborValue().toCbor());
}

} // namespace cybou
