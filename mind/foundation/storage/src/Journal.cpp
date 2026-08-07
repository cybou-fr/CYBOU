// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/storage/Journal.h"

#include <QCryptographicHash>
#include <QDir>
#include <QFileInfo>
#include <QSqlError>
#include <QSqlQuery>
#include <QStringList>
#include <QUuid>

namespace cybou {

namespace {

QString defaultConnectionName()
{
    return QStringLiteral("cybou-journal-%1")
        .arg(QUuid::createUuid().toString(QUuid::Id128));
}

QString envelopeColumns()
{
    return QStringLiteral(
        "message_id, correlation_id, causation_id, origin_organ, origin_node, kind, "
        "wall_time, monotonic_time, logical_clock, confidence, evidence, payload, privacy, "
        "capability");
}

} // namespace

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

    QSqlQuery pragma(m_db);
    pragma.exec(QStringLiteral("PRAGMA journal_mode=WAL"));
    pragma.exec(QStringLiteral("PRAGMA synchronous=NORMAL"));

    if (!ensureSchema()) {
        m_db.close();
    }
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
    if (!q.exec(QStringLiteral(R"SQL(
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
    )SQL"))) {
        m_lastError = q.lastError().text();
        return false;
    }

    if (!q.exec(QStringLiteral(
            "CREATE INDEX IF NOT EXISTS idx_correlation ON contribution(correlation_id)"))) {
        m_lastError = q.lastError().text();
        return false;
    }

    if (!q.exec(QStringLiteral(
            "CREATE INDEX IF NOT EXISTS idx_causation ON contribution(causation_id)"))) {
        m_lastError = q.lastError().text();
        return false;
    }

    return true;
}

QByteArray Journal::rowHash(
    quint64 seq, const CognitiveEnvelope &e, const QByteArray &prev) const
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
    m_lastError.clear();

    if (!m_db.isOpen()) {
        m_lastError = QStringLiteral("journal is not open");
        return 0;
    }

    if (!e.isValid()) {
        m_lastError = QStringLiteral("refusing to append an invalid envelope");
        return 0;
    }

    QList<PrivacyClass> referencePrivacy;

    if (!e.causationId.isNull()) {
        const auto cause = contribution(e.causationId);
        if (!cause) {
            m_lastError = QStringLiteral("causal contribution does not exist");
            return 0;
        }
        referencePrivacy.append(cause->privacy);
    }

    for (const QUuid &evidenceId : e.evidence) {
        const auto evidenceEnvelope = contribution(evidenceId);
        if (!evidenceEnvelope) {
            m_lastError = QStringLiteral("evidence contribution does not exist");
            return 0;
        }
        referencePrivacy.append(evidenceEnvelope->privacy);
    }

    if (e.derivedPrivacy(referencePrivacy) != e.privacy) {
        m_lastError = QStringLiteral("contribution privacy is weaker than its references");
        return 0;
    }

    const QByteArray prev = head();
    const quint64 seq = count() + 1;

    QStringList evidenceIds;
    evidenceIds.reserve(e.evidence.size());
    for (const QUuid &id : e.evidence) {
        evidenceIds.append(id.toString(QUuid::WithoutBraces));
    }

    QSqlQuery q(m_db);
    q.prepare(QStringLiteral(
        "INSERT INTO contribution (message_id, correlation_id, causation_id, origin_organ, "
        "origin_node, kind, wall_time, monotonic_time, logical_clock, confidence, evidence, "
        "payload, privacy, capability, prev_hash, hash) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"));

    q.addBindValue(e.messageId.toString(QUuid::WithoutBraces));
    q.addBindValue(e.correlationId.toString(QUuid::WithoutBraces));
    q.addBindValue(e.causationId.isNull()
                       ? QVariant()
                       : QVariant(e.causationId.toString(QUuid::WithoutBraces)));
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

std::optional<CognitiveEnvelope> Journal::readOne(QSqlQuery &query) const
{
    if (!query.next()) {
        return std::nullopt;
    }

    CognitiveEnvelope e;
    e.messageId = QUuid::fromString(query.value(0).toString());
    e.correlationId = QUuid::fromString(query.value(1).toString());
    e.causationId = QUuid::fromString(query.value(2).toString());
    e.originOrgan = query.value(3).toString();
    e.originNode = query.value(4).toString();
    e.kind = static_cast<ContributionKind>(query.value(5).toInt());
    e.wallTime = QDateTime::fromString(query.value(6).toString(), Qt::ISODateWithMs);
    e.monotonicTime = query.value(7).toULongLong();
    e.logicalClock = query.value(8).toULongLong();
    e.confidence = query.value(9).toDouble();

    const QString evidenceText = query.value(10).toString();
    for (const QString &id : evidenceText.split(QLatin1Char(','), Qt::SkipEmptyParts)) {
        e.evidence.append(QUuid::fromString(id));
    }

    e.payloadCbor = query.value(11).toByteArray();
    e.privacy = static_cast<PrivacyClass>(query.value(12).toInt());
    e.capabilityScope = query.value(13).toString();
    return e;
}

QList<CognitiveEnvelope> Journal::recent(int limit) const
{
    QList<CognitiveEnvelope> out;
    QSqlQuery q(m_db);

    QString sql = QStringLiteral("SELECT %1 FROM contribution ORDER BY seq DESC")
                      .arg(envelopeColumns());
    if (limit > 0) {
        sql.append(QStringLiteral(" LIMIT %1").arg(limit));
    }

    if (!q.exec(sql)) {
        return out;
    }

    while (true) {
        const auto e = readOne(q);
        if (!e) {
            break;
        }
        out.append(*e);
    }
    return out;
}

QList<CognitiveEnvelope> Journal::episode(const QUuid &correlationId) const
{
    QList<CognitiveEnvelope> out;
    QSqlQuery q(m_db);
    q.prepare(QStringLiteral("SELECT %1 FROM contribution WHERE correlation_id = ? ORDER BY seq")
                  .arg(envelopeColumns()));
    q.addBindValue(correlationId.toString(QUuid::WithoutBraces));
    if (!q.exec()) {
        return out;
    }

    while (true) {
        const auto e = readOne(q);
        if (!e) {
            break;
        }
        out.append(*e);
    }
    return out;
}

bool Journal::contains(const QUuid &messageId) const
{
    if (messageId.isNull()) {
        return false;
    }

    QSqlQuery q(m_db);
    q.prepare(QStringLiteral("SELECT 1 FROM contribution WHERE message_id = ? LIMIT 1"));
    q.addBindValue(messageId.toString(QUuid::WithoutBraces));
    return q.exec() && q.next();
}

std::optional<CognitiveEnvelope> Journal::contribution(const QUuid &messageId) const
{
    if (messageId.isNull()) {
        return std::nullopt;
    }

    QSqlQuery q(m_db);
    q.prepare(QStringLiteral("SELECT %1 FROM contribution WHERE message_id = ? LIMIT 1")
                  .arg(envelopeColumns()));
    q.addBindValue(messageId.toString(QUuid::WithoutBraces));
    if (!q.exec()) {
        return std::nullopt;
    }
    return readOne(q);
}

bool Journal::hasOutcomeFor(const QUuid &causeId, const QString &originOrgan) const
{
    if (causeId.isNull()) {
        return false;
    }

    QSqlQuery q(m_db);
    QString sql = QStringLiteral(
        "SELECT 1 FROM contribution WHERE kind = ? AND causation_id = ?");
    if (!originOrgan.isEmpty()) {
        sql.append(QStringLiteral(" AND origin_organ = ?"));
    }
    sql.append(QStringLiteral(" LIMIT 1"));

    q.prepare(sql);
    q.addBindValue(static_cast<int>(ContributionKind::Outcome));
    q.addBindValue(causeId.toString(QUuid::WithoutBraces));
    if (!originOrgan.isEmpty()) {
        q.addBindValue(originOrgan);
    }
    return q.exec() && q.next();
}

} // namespace cybou
