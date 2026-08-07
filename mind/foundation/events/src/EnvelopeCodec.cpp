// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/events/EnvelopeCodec.h"

#include "cybou/events/EventBus.h"

#include <QCborArray>
#include <QCborMap>
#include <QCborValue>

namespace cybou {

namespace {

void setError(QString *error, const QString &message)
{
    if (error) {
        *error = message;
    }
}

QString uuidText(const QUuid &id)
{
    return id.toString(QUuid::WithoutBraces);
}

QCborMap toMap(const CognitiveEnvelope &envelope)
{
    QCborMap map;
    map.insert(QStringLiteral("ipcVersion"), kEventIpcVersion);
    map.insert(QStringLiteral("schemaVersion"), static_cast<qint64>(envelope.schemaVersion));
    map.insert(QStringLiteral("messageId"), uuidText(envelope.messageId));
    map.insert(QStringLiteral("correlationId"), uuidText(envelope.correlationId));
    map.insert(QStringLiteral("causationId"), uuidText(envelope.causationId));
    map.insert(QStringLiteral("originOrgan"), envelope.originOrgan);
    map.insert(QStringLiteral("originNode"), envelope.originNode);
    map.insert(QStringLiteral("kind"), static_cast<qint64>(envelope.kind));
    map.insert(QStringLiteral("wallTime"), envelope.wallTime.toString(Qt::ISODateWithMs));
    // QCborValue integers are signed. Decimal strings preserve the full quint64 domain.
    map.insert(QStringLiteral("monotonicTime"), QString::number(envelope.monotonicTime));
    map.insert(QStringLiteral("logicalClock"), QString::number(envelope.logicalClock));
    map.insert(QStringLiteral("confidence"), envelope.confidence);

    QCborArray evidence;
    for (const QUuid &id : envelope.evidence) {
        evidence.append(uuidText(id));
    }
    map.insert(QStringLiteral("evidence"), evidence);

    map.insert(QStringLiteral("payloadCbor"), envelope.payloadCbor);
    map.insert(QStringLiteral("privacy"), static_cast<qint64>(envelope.privacy));
    map.insert(QStringLiteral("capabilityScope"), envelope.capabilityScope);
    return map;
}

bool parseU64(const QCborValue &value, quint64 *result)
{
    bool ok = false;
    const quint64 parsed = value.toString().toULongLong(&ok);
    if (ok && result) {
        *result = parsed;
    }
    return ok;
}

std::optional<CognitiveEnvelope> fromMap(const QCborMap &map, QString *error)
{
    if (map.value(QStringLiteral("ipcVersion")).toInteger(-1) != kEventIpcVersion) {
        setError(error, QStringLiteral("unsupported Event1 CBOR version"));
        return std::nullopt;
    }

    CognitiveEnvelope envelope;
    envelope.schemaVersion =
        static_cast<quint16>(map.value(QStringLiteral("schemaVersion")).toInteger(0));
    envelope.messageId =
        QUuid::fromString(map.value(QStringLiteral("messageId")).toString());
    envelope.correlationId =
        QUuid::fromString(map.value(QStringLiteral("correlationId")).toString());
    envelope.causationId =
        QUuid::fromString(map.value(QStringLiteral("causationId")).toString());
    envelope.originOrgan = map.value(QStringLiteral("originOrgan")).toString();
    envelope.originNode = map.value(QStringLiteral("originNode")).toString();
    envelope.kind = static_cast<ContributionKind>(
        map.value(QStringLiteral("kind")).toInteger(0));
    envelope.wallTime = QDateTime::fromString(
        map.value(QStringLiteral("wallTime")).toString(),
        Qt::ISODateWithMs);

    if (!parseU64(map.value(QStringLiteral("monotonicTime")), &envelope.monotonicTime)
        || !parseU64(map.value(QStringLiteral("logicalClock")), &envelope.logicalClock)) {
        setError(error, QStringLiteral("invalid Event1 clock field"));
        return std::nullopt;
    }

    envelope.confidence = map.value(QStringLiteral("confidence")).toDouble();

    const QCborValue evidenceValue = map.value(QStringLiteral("evidence"));
    if (!evidenceValue.isArray()) {
        setError(error, QStringLiteral("invalid Event1 evidence field"));
        return std::nullopt;
    }
    for (const QCborValue &value : evidenceValue.toArray()) {
        envelope.evidence.append(QUuid::fromString(value.toString()));
    }

    envelope.payloadCbor = map.value(QStringLiteral("payloadCbor")).toByteArray();
    envelope.privacy = static_cast<PrivacyClass>(
        map.value(QStringLiteral("privacy")).toInteger(0));
    envelope.capabilityScope =
        map.value(QStringLiteral("capabilityScope")).toString();

    if (error) {
        error->clear();
    }
    return envelope;
}

} // namespace

QByteArray EnvelopeCodec::encode(const CognitiveEnvelope &envelope)
{
    return toMap(envelope).toCborValue().toCbor();
}

std::optional<CognitiveEnvelope> EnvelopeCodec::decode(
    const QByteArray &encoded,
    QString *error)
{
    const QCborValue value = QCborValue::fromCbor(encoded);
    if (!value.isMap()) {
        setError(error, QStringLiteral("Event1 envelope is not a CBOR map"));
        return std::nullopt;
    }
    return fromMap(value.toMap(), error);
}

QByteArray EnvelopeCodec::encodeList(const QList<CognitiveEnvelope> &envelopes)
{
    QCborArray array;
    for (const CognitiveEnvelope &envelope : envelopes) {
        array.append(toMap(envelope));
    }
    return array.toCborValue().toCbor();
}

QList<CognitiveEnvelope> EnvelopeCodec::decodeList(
    const QByteArray &encoded,
    QString *error)
{
    QList<CognitiveEnvelope> result;
    const QCborValue value = QCborValue::fromCbor(encoded);
    if (!value.isArray()) {
        setError(error, QStringLiteral("Event1 envelope list is not a CBOR array"));
        return result;
    }

    for (const QCborValue &item : value.toArray()) {
        if (!item.isMap()) {
            setError(error, QStringLiteral("Event1 envelope list contains a non-map"));
            return {};
        }

        QString itemError;
        const auto envelope = fromMap(item.toMap(), &itemError);
        if (!envelope) {
            setError(error, itemError);
            return {};
        }
        result.append(*envelope);
    }

    if (error) {
        error->clear();
    }
    return result;
}

QByteArray EnvelopeCodec::encodeUuidList(const QList<QUuid> &ids)
{
    QCborArray array;
    for (const QUuid &id : ids) {
        array.append(uuidText(id));
    }
    return array.toCborValue().toCbor();
}

QList<QUuid> EnvelopeCodec::decodeUuidList(
    const QByteArray &encoded,
    QString *error)
{
    QList<QUuid> result;
    const QCborValue value = QCborValue::fromCbor(encoded);
    if (!value.isArray()) {
        setError(error, QStringLiteral("Event1 UUID list is not a CBOR array"));
        return result;
    }

    for (const QCborValue &item : value.toArray()) {
        const QUuid id = QUuid::fromString(item.toString());
        if (id.isNull()) {
            setError(error, QStringLiteral("Event1 UUID list contains an invalid UUID"));
            return {};
        }
        result.append(id);
    }

    if (error) {
        error->clear();
    }
    return result;
}

} // namespace cybou
