// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#include "cybou/storage/Journal.h"

#include <QCryptographicHash>
#include <QFileInfo>
#include <QDir>
#include <QSqlError>
#include <QSqlQuery>
#include <QUuid>

namespace cybou {

namespace {
QString defaultConnectionName()
{
    return QStringLiteral("cybou-journal-%1").arg(QUuid::createUuid().toString(QUuid::Id128));
}
}

Journal::Journal(const QString &path, const QString &connectionName)
    : m_connectionName(connectionName.isEmpty() ? defaultConnectionName() : connectionName)
{
    QDir().mkpath(QFileInfo(path).absolutePath());

    m_db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), m_connectionName);
    m_db.setDatabaseName(path);
    if (!m_db.open()) {
        m_lastError = m_db.lastError().text();
        return;
    }

    // WAL so a reader (presenced) never blocks the writer (eventd).
    QSqlQuery pragma(m_db);
    pragma.exec(QStringLiteral("PRAGMA journal_mode=WAL"));
    pragma.exec(QStringLiteral("PRAGMA synchronous=NORMAL"));

    ensureSchema();
}

Journal::~Journal()
{
    const QString name = m_connectionName;
    if (m_db.isOpen()) {
        m_db.close();
    }
    m_db = QSqlDatabase();
    QSqlDatabase::removeDatabase(name);
}

bool Journal::isOpen() const
{
    return m_db.isOpen();
}

QString Journal::lastError() const
{
    return m_lastError;
}

bool Journal::ensureSchema()
{
    QSqlQuery q(m_db);
    // No UPDATE or DELETE is ever issued against this table. The hash chain makes a rewrite
    // outside the process detectable rather than merely discouraged.
    const bool ok = q.exec(QStringLiteral(R"SQL(
        CREATE TABLE IF NOT EXISTS contribution (
            seq            INTEGER PRIMARY KEY AUTOINCREMENT,
            message_id     TEXT    NOT NULL UNIQUE,
            correlation_id TEXT,
            causation_id   TEXT,
            origin_organ   TEXT    NOT NULL,
            origin_node    TEXT,
            kind           INTEGER NOT NULL,
            wall_time      TEXT    NOT NULL,
            monotonic_time INTEGER NOT NULL,
            logical_clock  INTEGER NOT NULL,
            confidence     REAL    NOT NULL,
            evidence       TEXT,
            payload        BLOB,
            privacy        INTEGER NOT NULL,
            capability     TEXT,
            prev_hash      BLOB,
            hash           BLOB    NOT NULL
        )
    )SQL"));

    if (!ok) {
        m_lastError = q.lastError().text();
        return false;
    }
    q.exec(QStringLiteral("CREATE INDEX IF NOT EXISTS idx_correlation "
                          "ON contribution(correlation_id)"));
    return true;
}

QByteArray Journal::rowHash(quint64 seq, const CognitiveEnvelope &e, const QByteArray &prev) const
{
    QCryptographicHash h(QCryptographicHash::Sha256);
    h.addData(prev);
    h.addData(QByteArray::number(static_cast<qulonglong>(seq)));
    h.addData(e.messageId.toByteArray());
    h.addData(e.correlationId.toByteArray());
    h.addData(e.causationId.toByteArray());
    h.addData(e.originOrgan.toUtf8());
    h.addData(QByteArray::number(static_cast<int>(e.kind)));
    h.addData(e.wallTime.toString(Qt::ISODateWithMs).toUtf8());
    h.addData(QByteArray::number(static_cast<qulonglong>(e.logicalClock)));
    h.addData(e.payloadCbor);
    return h.result();
}

quint64 Journal::append(const CognitiveEnvelope &e)
{
    if (!m_db.isOpen()) {
        m_lastError = QStringLiteral("journal is not open");
        return 0;
    }
    if (!e.isValid()) {
        // Refused rather than stored: this is the one place a malformed contribution would
        // become permanent.
        m_lastError = QStringLiteral("refusing to append an invalid envelope");
        return 0;
    }

    const QByteArray prev = head();
    const quint64 seq = count() + 1;

    QStringList evidenceIds;
    evidenceIds.reserve(e.evidence.size());
    for (const QUuid &id : e.evidence) {
        evidenceIds << id.toString(QUuid::WithoutBraces);
    }

    QSqlQuery q(m_db);
    q.prepare(QStringLiteral(
        "INSERT INTO contribution (message_id, correlation_id, causation_id, origin_organ, "
        "origin_node, kind, wall_time, monotonic_time, logical_clock, confidence, evidence, "
        "payload, privacy, capability, prev_hash, hash) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"));

    q.addBindValue(e.messageId.toString(QUuid::WithoutBraces));
    q.addBindValue(e.correlationId.isNull() ? QVariant()
                                            : e.correlationId.toString(QUuid::WithoutBraces));
    q.addBindValue(e.causationId.isNull() ? QVariant()
                                          : e.causationId.toString(QUuid::WithoutBraces));
    q.addBindValue(e.originOrgan);
    q.addBindValue(e.originNode);
    q.addBindValue(static_cast<int>(e.kind));
    q.addBindValue(e.wallTime.toString(Qt::ISODateWithMs));
    q.addBindValue(static_cast<qulonglong>(e.monotonicTime));
    q.addBindValue(static_cast<qulonglong>(e.logicalClock));
    q.addBindValue(e.confidence);
    q.addBindValue(evidenceIds.join(QLatin1Char(',')));
    q.addBindValue(e.payloadCbor);
    q.addBindValue(static_cast<int>(e.privacy));
    q.addBindValue(e.capabilityScope);
    q.addBindValue(prev);
    q.addBindValue(rowHash(seq, e, prev));

    if (!q.exec()) {
        m_lastError = q.lastError().text();
        return 0;
    }
    return seq;
}

quint64 Journal::count() const
{
    QSqlQuery q(m_db);
    if (q.exec(QStringLiteral("SELECT COUNT(*) FROM contribution")) && q.next()) {
        return q.value(0).toULongLong();
    }
    return 0;
}

QByteArray Journal::head() const
{
    QSqlQuery q(m_db);
    if (q.exec(QStringLiteral("SELECT hash FROM contribution ORDER BY seq DESC LIMIT 1"))
        && q.next()) {
        return q.value(0).toByteArray();
    }
    return {};
}

quint64 Journal::verify() const
{
    QSqlQuery q(m_db);
    if (!q.exec(QStringLiteral(
            "SELECT seq, message_id, correlation_id, causation_id, origin_organ, kind, "
            "wall_time, logical_clock, payload, prev_hash, hash FROM contribution ORDER BY seq"))) {
        return 1;
    }

    QByteArray expectedPrev;
    while (q.next()) {
        const quint64 seq = q.value(0).toULongLong();

        CognitiveEnvelope e;
        e.messageId = QUuid::fromString(q.value(1).toString());
        e.correlationId = QUuid::fromString(q.value(2).toString());
        e.causationId = QUuid::fromString(q.value(3).toString());
        e.originOrgan = q.value(4).toString();
        e.kind = static_cast<ContributionKind>(q.value(5).toInt());
        e.wallTime = QDateTime::fromString(q.value(6).toString(), Qt::ISODateWithMs);
        e.logicalClock = q.value(7).toULongLong();
        e.payloadCbor = q.value(8).toByteArray();

        const QByteArray storedPrev = q.value(9).toByteArray();
        const QByteArray storedHash = q.value(10).toByteArray();

        if (storedPrev != expectedPrev || rowHash(seq, e, storedPrev) != storedHash) {
            return seq;
        }
        expectedPrev = storedHash;
    }
    return 0;
}

QList<CognitiveEnvelope> Journal::recent(int limit) const
{
    QList<CognitiveEnvelope> out;
    QSqlQuery q(m_db);
    const QString sql = QStringLiteral(
        "SELECT message_id, correlation_id, causation_id, origin_organ, origin_node, kind, "
        "wall_time, monotonic_time, logical_clock, confidence, payload, privacy "
        "FROM contribution ORDER BY seq DESC%1");
    q.prepare(sql.arg(limit > 0 ? QStringLiteral(" LIMIT %1").arg(limit) : QString()));
    if (!q.exec()) {
        return out;
    }
    while (q.next()) {
        CognitiveEnvelope e;
        e.messageId = QUuid::fromString(q.value(0).toString());
        e.correlationId = QUuid::fromString(q.value(1).toString());
        e.causationId = QUuid::fromString(q.value(2).toString());
        e.originOrgan = q.value(3).toString();
        e.originNode = q.value(4).toString();
        e.kind = static_cast<ContributionKind>(q.value(5).toInt());
        e.wallTime = QDateTime::fromString(q.value(6).toString(), Qt::ISODateWithMs);
        e.monotonicTime = q.value(7).toULongLong();
        e.logicalClock = q.value(8).toULongLong();
        e.confidence = q.value(9).toDouble();
        e.payloadCbor = q.value(10).toByteArray();
        e.privacy = static_cast<PrivacyClass>(q.value(11).toInt());
        out.append(e);
    }
    return out;
}

QList<CognitiveEnvelope> Journal::episode(const QUuid &correlationId) const
{
    QList<CognitiveEnvelope> out;
    QSqlQuery q(m_db);
    q.prepare(QStringLiteral(
        "SELECT message_id, causation_id, origin_organ, kind, wall_time, confidence "
        "FROM contribution WHERE correlation_id = ? ORDER BY seq"));
    q.addBindValue(correlationId.toString(QUuid::WithoutBraces));
    if (!q.exec()) {
        return out;
    }
    while (q.next()) {
        CognitiveEnvelope e;
        e.messageId = QUuid::fromString(q.value(0).toString());
        e.correlationId = correlationId;
        e.causationId = QUuid::fromString(q.value(1).toString());
        e.originOrgan = q.value(2).toString();
        e.kind = static_cast<ContributionKind>(q.value(3).toInt());
        e.wallTime = QDateTime::fromString(q.value(4).toString(), Qt::ISODateWithMs);
        e.confidence = q.value(5).toDouble();
        out.append(e);
    }
    return out;
}

} // namespace cybou
