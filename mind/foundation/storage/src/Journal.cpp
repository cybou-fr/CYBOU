// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/storage/Journal.h"

#include "cybou/protocol/CanonicalEnvelope.h"

#include <QCryptographicHash>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QSet>
#include <QSqlError>
#include <QSqlQuery>
#include <QStringList>
#include <QUuid>
#include <QVariant>

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
        "schema_version, message_id, correlation_id, causation_id, origin_organ, origin_node, "
        "kind, wall_time, monotonic_time, logical_clock, confidence, payload, privacy, "
        "capability");
}

QString sqlStringLiteral(QString value)
{
    value.replace(QLatin1Char('\''), QStringLiteral("''"));
    return QStringLiteral("'%1'").arg(value);
}

} // namespace

Journal::Journal(
    const QString &path,
    const QString &connectionName,
    QObject *parent)
    : QObject(parent)
    , m_connectionName(connectionName.isEmpty() ? defaultConnectionName() : connectionName)
    , m_path(path == QLatin1String(":memory:") ? path : QFileInfo(path).absoluteFilePath())
{
    if (m_path != QLatin1String(":memory:")) {
        QDir().mkpath(QFileInfo(m_path).absolutePath());
    }

    m_db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), m_connectionName);
    m_db.setDatabaseName(m_path);
    if (!m_db.open()) {
        m_lastError = m_db.lastError().text();
        return;
    }

    QSqlQuery pragma(m_db);
    pragma.exec(QStringLiteral("PRAGMA busy_timeout=5000"));
    pragma.finish();
    pragma.exec(QStringLiteral("PRAGMA foreign_keys=ON"));
    pragma.finish();
    pragma.exec(QStringLiteral("PRAGMA journal_mode=WAL"));
    pragma.finish();
    pragma.exec(QStringLiteral("PRAGMA synchronous=NORMAL"));
    pragma.finish();

    if (!ensureSchema()) {
        m_db.close();
        return;
    }

    m_ready = true;
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
    return m_ready && m_db.isOpen();
}

QString Journal::lastError() const
{
    return m_lastError;
}

int Journal::databaseSchemaVersion() const
{
    return isOpen() ? userVersion() : 0;
}

bool Journal::execSql(const QString &sql)
{
    QSqlQuery query(m_db);
    if (query.exec(sql)) {
        return true;
    }
    m_lastError = query.lastError().text();
    return false;
}

bool Journal::beginImmediate()
{
    return execSql(QStringLiteral("BEGIN IMMEDIATE"));
}

bool Journal::commitTransaction()
{
    return execSql(QStringLiteral("COMMIT"));
}

void Journal::rollbackTransaction()
{
    QSqlQuery rollback(m_db);
    rollback.exec(QStringLiteral("ROLLBACK"));
}

int Journal::userVersion() const
{
    QSqlQuery query(m_db);
    if (query.exec(QStringLiteral("PRAGMA user_version")) && query.next()) {
        return query.value(0).toInt();
    }
    return -1;
}

bool Journal::tableExists(const QString &table) const
{
    QSqlQuery query(m_db);
    query.prepare(QStringLiteral(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ? LIMIT 1"));
    query.addBindValue(table);
    return query.exec() && query.next();
}

bool Journal::columnExists(const QString &table, const QString &column) const
{
    QSqlQuery query(m_db);
    if (!query.exec(QStringLiteral("PRAGMA table_info(%1)").arg(table))) {
        return false;
    }
    while (query.next()) {
        if (query.value(1).toString() == column) {
            return true;
        }
    }
    return false;
}

bool Journal::ensureSchema()
{
    const int version = userVersion();
    if (version < 0) {
        m_lastError = QStringLiteral("cannot read the journal schema version");
        return false;
    }
    if (version > kCurrentDatabaseSchemaVersion) {
        m_lastError = QStringLiteral("journal schema %1 is newer than supported schema %2")
                          .arg(version)
                          .arg(kCurrentDatabaseSchemaVersion);
        return false;
    }

    const bool hasContribution = tableExists(QStringLiteral("contribution"));
    if (!hasContribution) {
        if (version != 0) {
            m_lastError = QStringLiteral("journal declares schema %1 but has no contribution table")
                              .arg(version);
            return false;
        }
        return createSchemaV2();
    }

    if (version == 0 || version == 1) {
        if (columnExists(QStringLiteral("contribution"), QStringLiteral("schema_version"))
            || columnExists(QStringLiteral("contribution"), QStringLiteral("hash_version"))) {
            m_lastError = QStringLiteral("journal has a partially versioned schema; refusing repair");
            return false;
        }
        return migrateV1ToV2();
    }

    if (!columnExists(QStringLiteral("contribution"), QStringLiteral("schema_version"))
        || !columnExists(QStringLiteral("contribution"), QStringLiteral("hash_version"))
        || !tableExists(QStringLiteral("contribution_evidence"))) {
        m_lastError = QStringLiteral("journal schema v2 is incomplete");
        return false;
    }

    return ensureV2Indexes();
}

bool Journal::createSchemaV2()
{
    if (!beginImmediate()) {
        return false;
    }

    if (!execSql(QStringLiteral(R"SQL(
        CREATE TABLE contribution (
            seq            INTEGER PRIMARY KEY AUTOINCREMENT,
            message_id     TEXT    NOT NULL UNIQUE,
            correlation_id TEXT    NOT NULL,
            causation_id   TEXT,
            origin_organ   TEXT    NOT NULL,
            origin_node    TEXT    NOT NULL DEFAULT '',
            kind           INTEGER NOT NULL,
            wall_time      TEXT    NOT NULL,
            monotonic_time INTEGER NOT NULL,
            logical_clock  INTEGER NOT NULL,
            confidence     REAL    NOT NULL,
            evidence       TEXT,
            payload        BLOB,
            privacy        INTEGER NOT NULL,
            capability     TEXT,
            schema_version INTEGER NOT NULL,
            hash_version   INTEGER NOT NULL,
            prev_hash      BLOB,
            hash           BLOB    NOT NULL
        )
    )SQL"))
        || !execSql(QStringLiteral(R"SQL(
        CREATE TABLE contribution_evidence (
            contribution_id TEXT    NOT NULL,
            evidence_id     TEXT    NOT NULL,
            ordinal         INTEGER NOT NULL,
            PRIMARY KEY (contribution_id, evidence_id),
            UNIQUE (contribution_id, ordinal),
            FOREIGN KEY (contribution_id) REFERENCES contribution(message_id) ON DELETE RESTRICT,
            FOREIGN KEY (evidence_id) REFERENCES contribution(message_id) ON DELETE RESTRICT
        )
    )SQL"))
        || !ensureV2Indexes()
        || !execSql(QStringLiteral("PRAGMA user_version = 2"))
        || !commitTransaction()) {
        rollbackTransaction();
        return false;
    }

    return true;
}

bool Journal::ensureV2Indexes()
{
    if (!execSql(QStringLiteral(
            "CREATE INDEX IF NOT EXISTS idx_correlation ON contribution(correlation_id)"))
        || !execSql(QStringLiteral(
            "CREATE INDEX IF NOT EXISTS idx_causation ON contribution(causation_id)"))
        || !execSql(QStringLiteral(
            "CREATE INDEX IF NOT EXISTS idx_kind ON contribution(kind)"))
        || !execSql(QStringLiteral(
            "CREATE INDEX IF NOT EXISTS idx_evidence_target "
            "ON contribution_evidence(evidence_id)"))) {
        return false;
    }

    return execSql(
        QStringLiteral(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_one_outcome_per_cause "
            "ON contribution(causation_id) "
            "WHERE kind = %1 AND causation_id IS NOT NULL")
            .arg(static_cast<int>(ContributionKind::Outcome)));
}

bool Journal::createMigrationBackup()
{
    if (m_path == QLatin1String(":memory:")) {
        m_lastError = QStringLiteral("cannot migrate an in-memory v1 journal");
        return false;
    }

    QSqlQuery checkpoint(m_db);
    if (!checkpoint.exec(QStringLiteral("PRAGMA wal_checkpoint(FULL)"))) {
        m_lastError = checkpoint.lastError().text();
        return false;
    }

    checkpoint.finish();

    const QString backup = m_path + QStringLiteral(".v1.bak");
    if (QFile::exists(backup) && !QFile::remove(backup)) {
        m_lastError = QStringLiteral("cannot replace migration backup %1").arg(backup);
        return false;
    }

    return execSql(QStringLiteral("VACUUM INTO %1").arg(sqlStringLiteral(backup)));
}

bool Journal::migrateV1ToV2()
{
    if (!createMigrationBackup() || !beginImmediate()) {
        return false;
    }

    if (!execSql(QStringLiteral(
            "ALTER TABLE contribution ADD COLUMN schema_version INTEGER NOT NULL DEFAULT 1"))
        || !execSql(QStringLiteral(
            "ALTER TABLE contribution ADD COLUMN hash_version INTEGER NOT NULL DEFAULT 1"))
        || !execSql(QStringLiteral(R"SQL(
        CREATE TABLE contribution_evidence (
            contribution_id TEXT    NOT NULL,
            evidence_id     TEXT    NOT NULL,
            ordinal         INTEGER NOT NULL,
            PRIMARY KEY (contribution_id, evidence_id),
            UNIQUE (contribution_id, ordinal),
            FOREIGN KEY (contribution_id) REFERENCES contribution(message_id) ON DELETE RESTRICT,
            FOREIGN KEY (evidence_id) REFERENCES contribution(message_id) ON DELETE RESTRICT
        )
    )SQL"))) {
        rollbackTransaction();
        return false;
    }

    QSqlQuery legacy(m_db);
    if (!legacy.exec(QStringLiteral(
            "SELECT message_id, evidence FROM contribution ORDER BY seq"))) {
        m_lastError = legacy.lastError().text();
        rollbackTransaction();
        return false;
    }

    while (legacy.next()) {
        const QString contributionId = legacy.value(0).toString();
        const QString evidenceText = legacy.value(1).toString();
        const QStringList rawIds = evidenceText.split(QLatin1Char(','), Qt::SkipEmptyParts);
        QSet<QString> seen;

        for (int ordinal = 0; ordinal < rawIds.size(); ++ordinal) {
            const QUuid parsed = QUuid::fromString(rawIds.at(ordinal).trimmed());
            if (parsed.isNull()) {
                m_lastError = QStringLiteral("legacy evidence contains an invalid UUID");
                rollbackTransaction();
                return false;
            }

            const QString evidenceId = parsed.toString(QUuid::WithoutBraces);
            if (seen.contains(evidenceId)) {
                m_lastError = QStringLiteral("legacy evidence contains a duplicate UUID");
                rollbackTransaction();
                return false;
            }
            seen.insert(evidenceId);

            QSqlQuery target(m_db);
            target.prepare(QStringLiteral(
                "SELECT 1 FROM contribution WHERE message_id = ? LIMIT 1"));
            target.addBindValue(evidenceId);
            if (!target.exec() || !target.next()) {
                m_lastError = QStringLiteral("legacy evidence references a missing contribution");
                rollbackTransaction();
                return false;
            }

            QSqlQuery insert(m_db);
            insert.prepare(QStringLiteral(
                "INSERT INTO contribution_evidence "
                "(contribution_id, evidence_id, ordinal) VALUES (?, ?, ?)"));
            insert.addBindValue(contributionId);
            insert.addBindValue(evidenceId);
            insert.addBindValue(ordinal);
            if (!insert.exec()) {
                m_lastError = insert.lastError().text();
                rollbackTransaction();
                return false;
            }
        }
    }
    legacy.finish();

    QSqlQuery duplicateOutcome(m_db);
    duplicateOutcome.prepare(QStringLiteral(
        "SELECT causation_id FROM contribution "
        "WHERE kind = ? AND causation_id IS NOT NULL "
        "GROUP BY causation_id HAVING COUNT(*) > 1 LIMIT 1"));
    duplicateOutcome.addBindValue(static_cast<int>(ContributionKind::Outcome));
    if (!duplicateOutcome.exec()) {
        m_lastError = duplicateOutcome.lastError().text();
        rollbackTransaction();
        return false;
    }
    if (duplicateOutcome.next()) {
        duplicateOutcome.finish();
        m_lastError = QStringLiteral(
            "legacy journal contains multiple terminal Outcomes for one cause");
        rollbackTransaction();
        return false;
    }
    duplicateOutcome.finish();

    if (!execSql(QStringLiteral(
            "UPDATE contribution SET schema_version = 1, hash_version = 1"))
        || !ensureV2Indexes()
        || !execSql(QStringLiteral("PRAGMA user_version = 2"))) {
        rollbackTransaction();
        return false;
    }

    const quint64 brokenAt = verify();
    if (brokenAt != 0) {
        m_lastError = QStringLiteral("legacy hash chain is broken at row %1").arg(brokenAt);
        rollbackTransaction();
        return false;
    }

    if (!commitTransaction()) {
        rollbackTransaction();
        return false;
    }
    return true;
}

QByteArray Journal::rowHashV1(
    quint64 seq, const CognitiveEnvelope &e, const QByteArray &prev) const
{
    QCryptographicHash hash(QCryptographicHash::Sha256);
    hash.addData(prev);
    hash.addData(QByteArray::number(static_cast<qulonglong>(seq)));
    hash.addData(e.messageId.toByteArray());
    hash.addData(e.correlationId.toByteArray());
    hash.addData(e.causationId.toByteArray());
    hash.addData(e.originOrgan.toUtf8());
    hash.addData(QByteArray::number(static_cast<int>(e.kind)));
    hash.addData(e.wallTime.toString(Qt::ISODateWithMs).toUtf8());
    hash.addData(QByteArray::number(static_cast<qulonglong>(e.logicalClock)));
    hash.addData(e.payloadCbor);
    return hash.result();
}

QByteArray Journal::rowHashV2(
    quint64 seq, const CognitiveEnvelope &e, const QByteArray &prev) const
{
    return QCryptographicHash::hash(
        canonicalJournalRowV2(seq, prev, e), QCryptographicHash::Sha256);
}

quint64 Journal::append(const CognitiveEnvelope &e)
{
    m_lastError.clear();

    if (!isOpen()) {
        m_lastError = QStringLiteral("journal is not open");
        return 0;
    }
    if (!e.isValid()) {
        m_lastError = QStringLiteral("refusing to append an invalid envelope");
        return 0;
    }
    if (e.schemaVersion != kCurrentEnvelopeSchemaVersion) {
        m_lastError = QStringLiteral("new contributions must use envelope schema v2");
        return 0;
    }
    if (!beginImmediate()) {
        return 0;
    }

    const auto fail = [this](const QString &message) -> quint64 {
        rollbackTransaction();
        m_lastError = message;
        return 0;
    };

    if (contains(e.messageId)) {
        return fail(QStringLiteral("messageId already exists"));
    }

    QList<PrivacyClass> referencePrivacy;
    if (!e.causationId.isNull()) {
        const auto cause = contribution(e.causationId);
        if (!cause) {
            return fail(QStringLiteral("causal contribution does not exist"));
        }
        referencePrivacy.append(cause->privacy);
    }

    for (const QUuid &evidenceId : e.evidence) {
        const auto evidenceEnvelope = contribution(evidenceId);
        if (!evidenceEnvelope) {
            return fail(QStringLiteral("evidence contribution does not exist"));
        }
        referencePrivacy.append(evidenceEnvelope->privacy);
    }

    if (e.derivedPrivacy(referencePrivacy) != e.privacy) {
        return fail(QStringLiteral("contribution privacy is weaker than its references"));
    }

    if (e.kind == ContributionKind::Outcome && hasOutcomeFor(e.causationId)) {
        return fail(QStringLiteral("the causal contribution already has a terminal Outcome"));
    }

    quint64 sequence = 1;
    QByteArray previousHash;
    QSqlQuery tail(m_db);
    if (!tail.exec(QStringLiteral(
            "SELECT seq, hash FROM contribution ORDER BY seq DESC LIMIT 1"))) {
        return fail(tail.lastError().text());
    }
    if (tail.next()) {
        sequence = tail.value(0).toULongLong() + 1;
        previousHash = tail.value(1).toByteArray();
    }

    const QByteArray hash = rowHashV2(sequence, e, previousHash);
    QSqlQuery insert(m_db);
    insert.prepare(QStringLiteral(
        "INSERT INTO contribution (seq, message_id, correlation_id, causation_id, "
        "origin_organ, origin_node, kind, wall_time, monotonic_time, logical_clock, "
        "confidence, evidence, payload, privacy, capability, schema_version, hash_version, "
        "prev_hash, hash) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"));

    insert.addBindValue(static_cast<qulonglong>(sequence));
    insert.addBindValue(e.messageId.toString(QUuid::WithoutBraces));
    insert.addBindValue(e.correlationId.toString(QUuid::WithoutBraces));
    insert.addBindValue(e.causationId.isNull()
                            ? QVariant()
                            : QVariant(e.causationId.toString(QUuid::WithoutBraces)));
    insert.addBindValue(e.originOrgan);
    insert.addBindValue(
        e.originNode.isNull() ? QStringLiteral("") : e.originNode);
    insert.addBindValue(static_cast<int>(e.kind));
    insert.addBindValue(e.wallTime.toString(Qt::ISODateWithMs));
    insert.addBindValue(static_cast<qulonglong>(e.monotonicTime));
    insert.addBindValue(static_cast<qulonglong>(e.logicalClock));
    insert.addBindValue(e.confidence);
    insert.addBindValue(QVariant());
    insert.addBindValue(e.payloadCbor);
    insert.addBindValue(static_cast<int>(e.privacy));
    insert.addBindValue(e.capabilityScope);
    insert.addBindValue(static_cast<int>(e.schemaVersion));
    insert.addBindValue(kCurrentJournalHashVersion);
    insert.addBindValue(previousHash);
    insert.addBindValue(hash);

    if (!insert.exec()) {
        return fail(insert.lastError().text());
    }

    for (int ordinal = 0; ordinal < e.evidence.size(); ++ordinal) {
        QSqlQuery evidenceInsert(m_db);
        evidenceInsert.prepare(QStringLiteral(
            "INSERT INTO contribution_evidence "
            "(contribution_id, evidence_id, ordinal) VALUES (?, ?, ?)"));
        evidenceInsert.addBindValue(e.messageId.toString(QUuid::WithoutBraces));
        evidenceInsert.addBindValue(e.evidence.at(ordinal).toString(QUuid::WithoutBraces));
        evidenceInsert.addBindValue(ordinal);
        if (!evidenceInsert.exec()) {
            return fail(evidenceInsert.lastError().text());
        }
    }

    if (!commitTransaction()) {
        rollbackTransaction();
        return 0;
    }

    Q_EMIT accepted(e, sequence);
    return sequence;
}

quint64 Journal::count() const
{
    QSqlQuery query(m_db);
    if (query.exec(QStringLiteral("SELECT COUNT(*) FROM contribution")) && query.next()) {
        return query.value(0).toULongLong();
    }
    return 0;
}

QByteArray Journal::head() const
{
    QSqlQuery query(m_db);
    if (query.exec(QStringLiteral("SELECT hash FROM contribution ORDER BY seq DESC LIMIT 1"))
        && query.next()) {
        return query.value(0).toByteArray();
    }
    return {};
}

CognitiveEnvelope Journal::envelopeFromQuery(const QSqlQuery &query, int offset) const
{
    CognitiveEnvelope e;
    e.schemaVersion = static_cast<quint16>(query.value(offset).toUInt());
    e.messageId = QUuid::fromString(query.value(offset + 1).toString());
    e.correlationId = QUuid::fromString(query.value(offset + 2).toString());
    e.causationId = QUuid::fromString(query.value(offset + 3).toString());
    e.originOrgan = query.value(offset + 4).toString();
    e.originNode = query.value(offset + 5).toString();
    e.kind = static_cast<ContributionKind>(query.value(offset + 6).toInt());
    e.wallTime = QDateTime::fromString(query.value(offset + 7).toString(), Qt::ISODateWithMs);
    e.monotonicTime = query.value(offset + 8).toULongLong();
    e.logicalClock = query.value(offset + 9).toULongLong();
    e.confidence = query.value(offset + 10).toDouble();
    e.payloadCbor = query.value(offset + 11).toByteArray();
    e.privacy = static_cast<PrivacyClass>(query.value(offset + 12).toInt());
    e.capabilityScope = query.value(offset + 13).toString();
    e.evidence = evidenceFor(e.messageId);
    return e;
}

quint64 Journal::verify() const
{
    if (!m_db.isOpen()) {
        return 1;
    }

    QSqlQuery query(m_db);
    const QString sql = QStringLiteral(
                            "SELECT seq, hash_version, %1, prev_hash, hash "
                            "FROM contribution ORDER BY seq")
                            .arg(envelopeColumns());
    if (!query.exec(sql)) {
        return 1;
    }

    quint64 expectedSequence = 1;
    QByteArray expectedPrevious;
    while (query.next()) {
        const quint64 sequence = query.value(0).toULongLong();
        if (sequence != expectedSequence) {
            return expectedSequence;
        }

        const int hashVersion = query.value(1).toInt();
        const CognitiveEnvelope e = envelopeFromQuery(query, 2);
        const QByteArray storedPrevious = query.value(16).toByteArray();
        const QByteArray storedHash = query.value(17).toByteArray();
        if (storedPrevious != expectedPrevious) {
            return sequence;
        }

        QByteArray expectedHash;
        if (hashVersion == kLegacyJournalHashVersion) {
            expectedHash = rowHashV1(sequence, e, storedPrevious);
        } else if (hashVersion == kCurrentJournalHashVersion
                   && e.schemaVersion == kCurrentEnvelopeSchemaVersion) {
            expectedHash = rowHashV2(sequence, e, storedPrevious);
        } else {
            return sequence;
        }

        if (expectedHash != storedHash) {
            return sequence;
        }

        expectedPrevious = storedHash;
        ++expectedSequence;
    }

    return 0;
}

std::optional<CognitiveEnvelope> Journal::readOne(QSqlQuery &query) const
{
    if (!query.next()) {
        return std::nullopt;
    }
    return envelopeFromQuery(query, 0);
}

QList<CognitiveEnvelope> Journal::recent(int limit) const
{
    QList<CognitiveEnvelope> out;
    QSqlQuery query(m_db);

    QString sql = QStringLiteral("SELECT %1 FROM contribution ORDER BY seq DESC")
                      .arg(envelopeColumns());
    if (limit > 0) {
        sql.append(QStringLiteral(" LIMIT %1").arg(limit));
    }

    if (!query.exec(sql)) {
        return out;
    }
    while (const auto e = readOne(query)) {
        out.append(*e);
    }
    return out;
}

QList<CognitiveEnvelope> Journal::episode(const QUuid &correlationId) const
{
    QList<CognitiveEnvelope> out;
    QSqlQuery query(m_db);
    query.prepare(QStringLiteral(
                      "SELECT %1 FROM contribution WHERE correlation_id = ? ORDER BY seq")
                      .arg(envelopeColumns()));
    query.addBindValue(correlationId.toString(QUuid::WithoutBraces));
    if (!query.exec()) {
        return out;
    }
    while (const auto e = readOne(query)) {
        out.append(*e);
    }
    return out;
}

bool Journal::contains(const QUuid &messageId) const
{
    if (messageId.isNull()) {
        return false;
    }

    QSqlQuery query(m_db);
    query.prepare(QStringLiteral("SELECT 1 FROM contribution WHERE message_id = ? LIMIT 1"));
    query.addBindValue(messageId.toString(QUuid::WithoutBraces));
    return query.exec() && query.next();
}

std::optional<CognitiveEnvelope> Journal::contribution(const QUuid &messageId) const
{
    if (messageId.isNull()) {
        return std::nullopt;
    }

    QSqlQuery query(m_db);
    query.prepare(QStringLiteral("SELECT %1 FROM contribution WHERE message_id = ? LIMIT 1")
                      .arg(envelopeColumns()));
    query.addBindValue(messageId.toString(QUuid::WithoutBraces));
    if (!query.exec()) {
        return std::nullopt;
    }
    return readOne(query);
}

QList<QUuid> Journal::evidenceFor(const QUuid &messageId) const
{
    QList<QUuid> result;
    if (messageId.isNull()) {
        return result;
    }

    QSqlQuery query(m_db);
    query.prepare(QStringLiteral(
        "SELECT evidence_id FROM contribution_evidence "
        "WHERE contribution_id = ? ORDER BY ordinal"));
    query.addBindValue(messageId.toString(QUuid::WithoutBraces));
    if (!query.exec()) {
        return result;
    }
    while (query.next()) {
        result.append(QUuid::fromString(query.value(0).toString()));
    }
    return result;
}

bool Journal::hasOutcomeFor(const QUuid &causeId, const QString &originOrgan) const
{
    if (causeId.isNull()) {
        return false;
    }

    QSqlQuery query(m_db);
    QString sql = QStringLiteral(
        "SELECT 1 FROM contribution WHERE kind = ? AND causation_id = ?");
    if (!originOrgan.isEmpty()) {
        sql.append(QStringLiteral(" AND origin_organ = ?"));
    }
    sql.append(QStringLiteral(" LIMIT 1"));

    query.prepare(sql);
    query.addBindValue(static_cast<int>(ContributionKind::Outcome));
    query.addBindValue(causeId.toString(QUuid::WithoutBraces));
    if (!originOrgan.isEmpty()) {
        query.addBindValue(originOrgan);
    }
    return query.exec() && query.next();
}

} // namespace cybou
