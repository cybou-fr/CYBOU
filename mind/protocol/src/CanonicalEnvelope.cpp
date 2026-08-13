// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/protocol/CanonicalEnvelope.h"

#include <algorithm>
#include <bit>

namespace cybou {

namespace {

void appendU8(QByteArray &out, quint8 value)
{
    out.append(static_cast<char>(value));
}

void appendU16(QByteArray &out, quint16 value)
{
    out.append(static_cast<char>((value >> 8) & 0xff));
    out.append(static_cast<char>(value & 0xff));
}

void appendU32(QByteArray &out, quint32 value)
{
    for (int shift = 24; shift >= 0; shift -= 8) {
        out.append(static_cast<char>((value >> shift) & 0xff));
    }
}

void appendU64(QByteArray &out, quint64 value)
{
    for (int shift = 56; shift >= 0; shift -= 8) {
        out.append(static_cast<char>((value >> shift) & 0xff));
    }
}

void appendBytes(QByteArray &out, const QByteArray &value)
{
    appendU32(out, static_cast<quint32>(value.size()));
    out.append(value);
}

void appendString(QByteArray &out, const QString &value)
{
    appendBytes(out, value.toUtf8());
}

void appendUuid(QByteArray &out, const QUuid &value)
{
    out.append(value.toRfc4122());
}

} // namespace

QByteArray canonicalEnvelopeV2(const CognitiveEnvelope &envelope)
{
    QByteArray out;
    out.reserve(256 + envelope.payloadCbor.size() + envelope.evidence.size() * 16);
    out.append(QByteArray("CYBOU-ENVELOPE-V2"));

    appendU16(out, envelope.schemaVersion);
    appendUuid(out, envelope.messageId);
    appendUuid(out, envelope.correlationId);
    appendUuid(out, envelope.causationId);
    appendString(out, envelope.originOrgan);
    appendString(out, envelope.originNode);
    appendU16(out, static_cast<quint16>(envelope.kind));
    appendU64(out, static_cast<quint64>(envelope.wallTime.toUTC().toMSecsSinceEpoch()));
    appendU64(out, envelope.monotonicTime);
    appendU64(out, envelope.logicalClock);

    const double normalizedConfidence = envelope.confidence == 0.0 ? 0.0 : envelope.confidence;
    appendU64(out, std::bit_cast<quint64>(normalizedConfidence));

    QList<QByteArray> sortedEvidence;
    sortedEvidence.reserve(envelope.evidence.size());
    for (const QUuid &id : envelope.evidence) {
        sortedEvidence.append(id.toRfc4122());
    }
    std::sort(sortedEvidence.begin(), sortedEvidence.end());
    appendU32(out, static_cast<quint32>(sortedEvidence.size()));
    for (const QByteArray &id : sortedEvidence) {
        out.append(id);
    }

    appendBytes(out, envelope.payloadCbor);
    appendU8(out, static_cast<quint8>(envelope.privacy));
    appendString(out, envelope.capabilityScope);
    return out;
}

QByteArray canonicalNonErasableEnvelopeV3(const CognitiveEnvelope &envelope)
{
    QByteArray out;
    out.reserve(256 + envelope.evidence.size() * 16);
    out.append(QByteArray("CYBOU-ENVELOPE-NONERASABLE-V3"));

    appendU16(out, envelope.schemaVersion);
    appendUuid(out, envelope.messageId);
    appendUuid(out, envelope.correlationId);
    appendUuid(out, envelope.causationId);
    appendString(out, envelope.originOrgan);
    appendString(out, envelope.originNode);
    appendU16(out, static_cast<quint16>(envelope.kind));
    appendU64(out, static_cast<quint64>(envelope.wallTime.toUTC().toMSecsSinceEpoch()));
    appendU64(out, envelope.monotonicTime);
    appendU64(out, envelope.logicalClock);

    const double normalizedConfidence = envelope.confidence == 0.0 ? 0.0 : envelope.confidence;
    appendU64(out, std::bit_cast<quint64>(normalizedConfidence));

    // Sorted, as in v2: evidence is a set, and two orderings of one set are one fact.
    QList<QByteArray> sortedEvidence;
    sortedEvidence.reserve(envelope.evidence.size());
    for (const QUuid &id : envelope.evidence) {
        sortedEvidence.append(id.toRfc4122());
    }
    std::sort(sortedEvidence.begin(), sortedEvidence.end());
    appendU32(out, static_cast<quint32>(sortedEvidence.size()));
    for (const QByteArray &id : sortedEvidence) {
        out.append(id);
    }

    appendU8(out, static_cast<quint8>(envelope.privacy));
    appendString(out, envelope.capabilityScope);
    return out;
}

QByteArray canonicalJournalRowV2(
    quint64 sequence, const QByteArray &previousHash, const CognitiveEnvelope &envelope)
{
    QByteArray out;
    out.append(QByteArray("CYBOU-JOURNAL-ROW-V2"));
    appendU16(out, 2);
    appendU64(out, sequence);
    appendBytes(out, previousHash);
    appendBytes(out, canonicalEnvelopeV2(envelope));
    return out;
}

} // namespace cybou
