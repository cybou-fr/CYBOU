// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
//
// Write a fixed set of contributions through the predecessor Journal and dump every stored row.
//
// The canonical bytes and their digests are already proven byte-for-byte by
// canonical_envelope_fixture.cpp. What this oracle exists to prove is the part no canonical form
// covers: how the writer spells what it stores. UUID punctuation, instant format, which absent
// value becomes NULL and which becomes an empty string, and the fixed hash version are decisions
// made at the INSERT, and two writers can agree on every hash while disagreeing on all of them.
//
// The dump goes through SQLite's own quote() so that neither side formats anything. A REAL printed
// by Qt and a REAL printed by Rust could differ in their last digit for reasons that have nothing
// to do with what was stored; asking the database to render its own value removes that whole class
// of false difference.

#include "cybou/storage/Journal.h"

#include <QCoreApplication>
#include <QDir>
#include <QSqlQuery>
#include <QSqlRecord>
#include <QTemporaryDir>
#include <QTextStream>

namespace {

/// The columns dumped, in a fixed order. Named explicitly rather than taken from the table so that
/// a column added by a future migration makes this oracle fail rather than silently widen.
const QStringList kColumns = {
    QStringLiteral("seq"),
    QStringLiteral("message_id"),
    QStringLiteral("correlation_id"),
    QStringLiteral("causation_id"),
    QStringLiteral("origin_organ"),
    QStringLiteral("origin_node"),
    QStringLiteral("kind"),
    QStringLiteral("wall_time"),
    QStringLiteral("monotonic_time"),
    QStringLiteral("logical_clock"),
    QStringLiteral("confidence"),
    QStringLiteral("evidence"),
    QStringLiteral("payload"),
    QStringLiteral("privacy"),
    QStringLiteral("capability"),
    QStringLiteral("schema_version"),
    QStringLiteral("hash_version"),
    QStringLiteral("prev_hash"),
    QStringLiteral("hash"),
    QStringLiteral("commitment"),
    QStringLiteral("payload_commitment"),
    QStringLiteral("erased_at"),
    QStringLiteral("sealed"),
    QStringLiteral("key_domain"),
    QStringLiteral("key_epoch"),
    QStringLiteral("retention_class"),
    QStringLiteral("retention_policy"),
    QStringLiteral("retain_until"),
    QStringLiteral("sensitivity"),
};

QUuid id(const char *text)
{
    return QUuid(QString::fromLatin1(text));
}

QDateTime instant(const char *text)
{
    return QDateTime::fromString(QString::fromLatin1(text), Qt::ISODateWithMs).toUTC();
}

/// The first observation. A root kind: no cause, no evidence, unbounded retention.
cybou::CognitiveEnvelope first()
{
    cybou::CognitiveEnvelope envelope;
    envelope.schemaVersion = cybou::kCurrentEnvelopeSchemaVersion;
    envelope.messageId = id("11111111-1111-4111-8111-111111111111");
    envelope.correlationId = id("22222222-2222-4222-8222-222222222222");
    envelope.originOrgan = QStringLiteral("perceptiond");
    envelope.originNode = QString();
    envelope.kind = cybou::ContributionKind::Observation;
    envelope.wallTime = instant("2026-08-19T08:15:30.125Z");
    envelope.monotonicTime = 123;
    envelope.logicalClock = 1;
    envelope.confidence = 1.0;
    envelope.payloadCbor = QByteArray::fromHex("a1617801");
    envelope.privacy = cybou::PrivacyClass::Local;
    envelope.retentionClass = cybou::RetentionClass::Standard;
    envelope.retentionPolicyVersion = 1;
    return envelope;
}

/// A second observation, carrying the values the first one leaves at their defaults: a node name,
/// a capability scope, a bounded retention, a fractional confidence.
cybou::CognitiveEnvelope second()
{
    cybou::CognitiveEnvelope envelope = first();
    envelope.messageId = id("33333333-3333-4333-8333-333333333333");
    envelope.originNode = QStringLiteral("local");
    envelope.logicalClock = 2;
    envelope.confidence = 0.75;
    envelope.payloadCbor = QByteArray::fromHex("a2617801617902");
    envelope.privacy = cybou::PrivacyClass::Node;
    envelope.capabilityScope = QStringLiteral("mind.perception.read");
    envelope.retentionClass = cybou::RetentionClass::Long;
    envelope.retentionPolicyVersion = 2;
    envelope.retainUntil = instant("2026-09-19T08:15:30.125Z");
    return envelope;
}

/// A derived contribution citing both, so the evidence join table and its ordinals are exercised.
///
/// Its retention is the earliest of its references rather than its own declaration, and its privacy
/// the most restrictive: the writer refuses anything else, so a fixture that declared otherwise
/// would test the refusal rather than the row.
cybou::CognitiveEnvelope third()
{
    cybou::CognitiveEnvelope envelope = first();
    envelope.messageId = id("44444444-4444-4444-8444-444444444444");
    envelope.kind = cybou::ContributionKind::Learning;
    envelope.causationId = id("11111111-1111-4111-8111-111111111111");
    envelope.evidence = {id("33333333-3333-4333-8333-333333333333")};
    envelope.originOrgan = QStringLiteral("selfd");
    envelope.logicalClock = 3;
    envelope.confidence = 0.5;
    envelope.payloadCbor = QByteArray::fromHex("a1617a03");
    envelope.privacy = cybou::PrivacyClass::Local;
    envelope.retentionClass = cybou::RetentionClass::Long;
    envelope.retentionPolicyVersion = 2;
    envelope.retainUntil = instant("2026-09-19T08:15:30.125Z");
    return envelope;
}

} // namespace

int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    QTextStream out(stdout);
    QTextStream err(stderr);

    QTemporaryDir directory;
    if (!directory.isValid()) {
        err << "cannot create a temporary directory\n";
        return 1;
    }
    const QString path = directory.filePath(QStringLiteral("journal.db"));

    cybou::Journal journal(path);
    if (!journal.isOpen()) {
        err << "cannot open the journal: " << journal.lastError() << '\n';
        return 1;
    }

    for (const cybou::CognitiveEnvelope &envelope : {first(), second(), third()}) {
        if (journal.append(envelope) == 0) {
            err << "append refused: " << journal.lastError() << '\n';
            return 1;
        }
    }

    QStringList quoted;
    quoted.reserve(kColumns.size());
    for (const QString &column : kColumns) {
        quoted.append(QStringLiteral("quote(%1)").arg(column));
    }

    QSqlQuery rows(QSqlDatabase::database(QSqlDatabase::defaultConnection));
    if (!rows.exec(QStringLiteral("SELECT %1 FROM contribution ORDER BY seq")
                       .arg(quoted.join(QStringLiteral(", "))))) {
        err << "cannot read the written rows: " << rows.lastError().text() << '\n';
        return 1;
    }
    int index = 0;
    while (rows.next()) {
        ++index;
        for (int column = 0; column < kColumns.size(); ++column) {
            out << "row." << index << '.' << kColumns.at(column) << '='
                << rows.value(column).toString() << '\n';
        }
    }

    QSqlQuery links(QSqlDatabase::database(QSqlDatabase::defaultConnection));
    if (!links.exec(QStringLiteral(
            "SELECT quote(contribution_id), quote(evidence_id), quote(ordinal) "
            "FROM contribution_evidence ORDER BY contribution_id, ordinal"))) {
        err << "cannot read the evidence links: " << links.lastError().text() << '\n';
        return 1;
    }
    int link = 0;
    while (links.next()) {
        ++link;
        out << "evidence." << link << '=' << links.value(0).toString() << ' '
            << links.value(1).toString() << ' ' << links.value(2).toString() << '\n';
    }

    return 0;
}
