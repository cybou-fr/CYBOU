// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/protocol/CanonicalEnvelope.h"

#include <QCryptographicHash>
#include <QTextStream>

int main()
{
    cybou::CognitiveEnvelope envelope;
    envelope.schemaVersion = cybou::kClassifiedEnvelopeSchemaVersion;
    envelope.messageId = QUuid(QStringLiteral("11111111-1111-4111-8111-111111111111"));
    envelope.correlationId = QUuid(QStringLiteral("22222222-2222-4222-8222-222222222222"));
    envelope.causationId = QUuid(QStringLiteral("33333333-3333-4333-8333-333333333333"));
    envelope.originOrgan = QStringLiteral("predictord");
    envelope.originNode = QStringLiteral("local");
    envelope.kind = cybou::ContributionKind::Outcome;
    envelope.wallTime =
        QDateTime::fromString(QStringLiteral("2026-08-19T08:15:30.125Z"), Qt::ISODateWithMs);
    envelope.monotonicTime = 123;
    envelope.logicalClock = 456;
    envelope.confidence = 0.75;
    envelope.evidence = {
        QUuid(QStringLiteral("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb")),
        QUuid(QStringLiteral("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa")),
    };
    envelope.payloadCbor = QByteArray::fromHex("a1617801");
    envelope.privacy = cybou::PrivacyClass::Node;
    envelope.capabilityScope = QStringLiteral("mind.prediction.read");
    envelope.protection.sealed = true;
    envelope.protection.keyDomainId =
        QUuid(QStringLiteral("44444444-4444-4444-8444-444444444444"));
    envelope.protection.keyEpoch = 7;
    envelope.retentionClass = cybou::RetentionClass::Long;
    envelope.retentionPolicyVersion = 2;
    envelope.retainUntil =
        QDateTime::fromString(QStringLiteral("2026-09-19T08:15:30.125Z"), Qt::ISODateWithMs);
    envelope.sensitivity = cybou::SensitivityClass::Secret;

    const QByteArray v2 = cybou::canonicalEnvelopeV2(envelope);
    const QByteArray v3 = cybou::canonicalNonErasableEnvelopeV3(envelope);
    const QByteArray row = cybou::canonicalJournalRowV2(9, QByteArray(32, '\x5a'), envelope);
    const QByteArray metadata = QCryptographicHash::hash(v3, QCryptographicHash::Sha256);
    const QByteArray payload =
        QCryptographicHash::hash(envelope.payloadCbor, QCryptographicHash::Sha256);
    const QByteArray commitment =
        QCryptographicHash::hash(metadata + payload, QCryptographicHash::Sha256);
    QByteArray rowV3("CYBOU-JOURNAL-ROW-V3");
    rowV3.append('\0');
    rowV3.append('\3');
    for (int shift = 56; shift >= 0; shift -= 8) {
        rowV3.append(static_cast<char>((quint64(9) >> shift) & 0xff));
    }
    rowV3.append(QByteArray(32, '\x5a'));
    rowV3.append(commitment);
    QTextStream out(stdout);
    out << "envelope-v2=" << v2.toHex() << '\n';
    out << "nonerasable-v3=" << v3.toHex() << '\n';
    out << "journal-row-v2=" << row.toHex() << '\n';
    out << "envelope-v2-sha256="
        << QCryptographicHash::hash(v2, QCryptographicHash::Sha256).toHex() << '\n';
    out << "nonerasable-v3-sha256="
        << metadata.toHex() << '\n';
    out << "payload-v3-sha256=" << payload.toHex() << '\n';
    out << "commitment-v3=" << commitment.toHex() << '\n';
    out << "journal-row-v3=" << rowV3.toHex() << '\n';
    out << "journal-row-v3-sha256="
        << QCryptographicHash::hash(rowV3, QCryptographicHash::Sha256).toHex() << '\n';
    return 0;
}
