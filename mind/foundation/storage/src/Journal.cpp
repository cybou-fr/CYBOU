// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/storage/Journal.h"

#include "cybou/protocol/CanonicalEnvelope.h"

#include <algorithm>

#include <QCborMap>
#include <QCborValue>

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

// How many columns envelopeColumns() names. Every positional read after an envelope depends on it,
// so it lives beside the list rather than in the heads of the people who write those reads.
inline constexpr int kEnvelopeColumnCount = 20;

QString envelopeColumns()
{
    return QStringLiteral(
        "schema_version, message_id, correlation_id, causation_id, origin_organ, origin_node, "
        "kind, wall_time, monotonic_time, logical_clock, confidence, payload, privacy, "
        "capability, sealed, key_domain, key_epoch, retention_class, retention_policy, "
        "retain_until");
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
    : EventStore(parent)
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
    pragma.exec(QStringLiteral("PRAGMA synchronous=FULL"));
    pragma.finish();

    if (!ensureDurability()) {
        m_db.close();
        return;
    }

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

// Mind Model invariant 1 says a contribution is durable before it is visible: Event1 publishes
// Accepted only after COMMIT returns. That ordering is only worth anything if COMMIT has reached
// storage. In WAL mode `synchronous=NORMAL` does not fsync the log at commit, so a power loss can
// drop a contribution that Presence already displayed as accepted; the fault matrix cannot catch
// that, because killing a process leaves the page cache intact and a clean reboot flushes it.
//
// So the pragmas above are not advisory. SQLite silently keeps the previous mode when it cannot
// apply one - a filesystem without shared-memory support falls back from WAL, for instance - and a
// silent fallback would leave the invariant stated more strongly than the storage supports. Read
// both values back and refuse to open the Journal rather than weaken the guarantee unannounced.
//
// An in-memory Journal is exempt: it is test scaffolding with no durability claim to make.
bool Journal::ensureDurability()
{
    if (m_path == QLatin1String(":memory:")) {
        return true;
    }

    QSqlQuery query(m_db);
    if (!query.exec(QStringLiteral("PRAGMA journal_mode")) || !query.next()) {
        m_lastError = QStringLiteral("cannot read the journal commit mode");
        return false;
    }
    const QString mode = query.value(0).toString().toLower();
    query.finish();
    if (mode != QLatin1String("wal")) {
        m_lastError =
            QStringLiteral("journal commit mode is %1, not the required write-ahead log").arg(mode);
        return false;
    }

    if (!query.exec(QStringLiteral("PRAGMA synchronous")) || !query.next()) {
        m_lastError = QStringLiteral("cannot read the journal synchronisation level");
        return false;
    }
    const int synchronous = query.value(0).toInt();
    query.finish();
    if (synchronous < kRequiredSynchronousLevel) {
        m_lastError = QStringLiteral(
                          "journal synchronisation level %1 does not survive power loss; "
                          "acceptance cannot be published as durable")
                          .arg(synchronous);
        return false;
    }

    return true;
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

    // Additive, and additive on purpose. The commitment column only ever holds a value for rows
    // written at hash v3; existing v1 and v2 rows keep NULL and keep verifying exactly as they did,
    // because their hash covers the payload by value and nothing about them has changed. A hash
    // chain that could be migrated retroactively would not be a hash chain.
    if (!columnExists(QStringLiteral("contribution"), QStringLiteral("commitment"))
        && !execSql(QStringLiteral("ALTER TABLE contribution ADD COLUMN commitment BLOB"))) {
        m_lastError = QStringLiteral("could not add the commitment column");
        return false;
    }
    if (!columnExists(QStringLiteral("contribution"), QStringLiteral("payload_commitment"))
        && !execSql(
            QStringLiteral("ALTER TABLE contribution ADD COLUMN payload_commitment BLOB"))) {
        m_lastError = QStringLiteral("could not add the payload commitment column");
        return false;
    }
    if (!columnExists(QStringLiteral("contribution"), QStringLiteral("sealed"))
        && !execSql(QStringLiteral("ALTER TABLE contribution ADD COLUMN sealed INTEGER NOT NULL DEFAULT 0"))) {
        m_lastError = QStringLiteral("could not add the sealed column");
        return false;
    }
    if (!columnExists(QStringLiteral("contribution"), QStringLiteral("key_domain"))
        && !execSql(QStringLiteral("ALTER TABLE contribution ADD COLUMN key_domain TEXT"))) {
        m_lastError = QStringLiteral("could not add the key_domain column");
        return false;
    }
    if (!columnExists(QStringLiteral("contribution"), QStringLiteral("key_epoch"))
        && !execSql(QStringLiteral("ALTER TABLE contribution ADD COLUMN key_epoch INTEGER NOT NULL DEFAULT 0"))) {
        m_lastError = QStringLiteral("could not add the key_epoch column");
        return false;
    }
    if (!columnExists(QStringLiteral("contribution"), QStringLiteral("retention_class"))
        && !execSql(QStringLiteral("ALTER TABLE contribution ADD COLUMN retention_class INTEGER NOT NULL DEFAULT 2"))) {
        m_lastError = QStringLiteral("could not add the retention_class column");
        return false;
    }
    if (!columnExists(QStringLiteral("contribution"), QStringLiteral("retention_policy"))
        && !execSql(QStringLiteral("ALTER TABLE contribution ADD COLUMN retention_policy INTEGER NOT NULL DEFAULT 0"))) {
        m_lastError = QStringLiteral("could not add the retention_policy column");
        return false;
    }
    if (!columnExists(QStringLiteral("contribution"), QStringLiteral("retain_until"))
        && !execSql(QStringLiteral("ALTER TABLE contribution ADD COLUMN retain_until TEXT"))) {
        m_lastError = QStringLiteral("could not add the retain_until column");
        return false;
    }
    if (!columnExists(QStringLiteral("contribution"), QStringLiteral("erased_at"))
        && !execSql(QStringLiteral("ALTER TABLE contribution ADD COLUMN erased_at TEXT"))) {
        m_lastError = QStringLiteral("could not add the erasure marker column");
        return false;
    }

    // The epoch lives in its own row rather than in a pragma, because it is bumped inside the same
    // transaction as a redaction: a projection must never be able to see a redacted payload while
    // still believing its cached view is current.
    if (tableExists(QStringLiteral("journal_meta"))
        && !columnExists(QStringLiteral("journal_meta"), QStringLiteral("rotated_epoch"))
        && !execSql(QStringLiteral(
            "ALTER TABLE journal_meta ADD COLUMN rotated_epoch INTEGER NOT NULL DEFAULT 0"))) {
        m_lastError = QStringLiteral("could not add the backup rotation column");
        return false;
    }

    if (!tableExists(QStringLiteral("journal_meta"))
        && (!execSql(QStringLiteral(
                "CREATE TABLE journal_meta ("
                "id INTEGER PRIMARY KEY CHECK (id = 1), "
                "erasure_epoch INTEGER NOT NULL DEFAULT 0, "
                "rotated_epoch INTEGER NOT NULL DEFAULT 0)"))
            || !execSql(QStringLiteral("INSERT OR IGNORE INTO journal_meta (id) VALUES (1)")))) {
        m_lastError = QStringLiteral("could not create the journal metadata table");
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
            hash           BLOB    NOT NULL,
            commitment     BLOB,
            payload_commitment BLOB,
            erased_at      TEXT,
            sealed         INTEGER NOT NULL DEFAULT 0,
            key_domain     TEXT,
            key_epoch      INTEGER NOT NULL DEFAULT 0,
            retention_class INTEGER NOT NULL DEFAULT 2,
            retention_policy INTEGER NOT NULL DEFAULT 0,
            retain_until   TEXT
        )
    )SQL"))
        || !execSql(QStringLiteral(R"SQL(
        CREATE TABLE journal_meta (
            id             INTEGER PRIMARY KEY CHECK (id = 1),
            erasure_epoch  INTEGER NOT NULL DEFAULT 0,
            rotated_epoch  INTEGER NOT NULL DEFAULT 0
        )
    )SQL"))
        || !execSql(QStringLiteral("INSERT OR IGNORE INTO journal_meta (id) VALUES (1)"))
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
        || !execSql(QStringLiteral("ALTER TABLE contribution ADD COLUMN commitment BLOB"))
        || !execSql(QStringLiteral(
            "ALTER TABLE contribution ADD COLUMN payload_commitment BLOB"))
        || !execSql(QStringLiteral("ALTER TABLE contribution ADD COLUMN erased_at TEXT"))
        || !execSql(QStringLiteral(
            "ALTER TABLE contribution ADD COLUMN sealed INTEGER NOT NULL DEFAULT 0"))
        || !execSql(QStringLiteral("ALTER TABLE contribution ADD COLUMN key_domain TEXT"))
        || !execSql(QStringLiteral(
            "ALTER TABLE contribution ADD COLUMN key_epoch INTEGER NOT NULL DEFAULT 0"))
        || !execSql(QStringLiteral(
            "ALTER TABLE contribution ADD COLUMN retention_class INTEGER NOT NULL DEFAULT 2"))
        || !execSql(QStringLiteral(
            "ALTER TABLE contribution ADD COLUMN retention_policy INTEGER NOT NULL DEFAULT 0"))
        || !execSql(QStringLiteral("ALTER TABLE contribution ADD COLUMN retain_until TEXT"))
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

QByteArray Journal::metadataDigestV3(const CognitiveEnvelope &envelope)
{
    return QCryptographicHash::hash(
        canonicalNonErasableEnvelopeV3(envelope), QCryptographicHash::Sha256);
}

// Today every payload commits to its own bytes. When ADR-0028's sensitive path lands, a sensitive
// payload will commit to `nonce || ciphertext || tag` instead, so that destroying the key leaves a
// commitment nobody can test a guess against. The split exists now so that change is a change of
// one function rather than of the chain format.
QByteArray Journal::payloadCommitmentV3(const CognitiveEnvelope &envelope)
{
    // Both branches hash the bytes that are stored. For a sealed payload those bytes are the nonce
    // and ciphertext, which depend on randomness a guesser does not have - which is the entire
    // reason a destroyed key leaves nothing testable behind.
    return QCryptographicHash::hash(envelope.payloadCbor, QCryptographicHash::Sha256);
}

QByteArray Journal::commitmentFrom(
    const QByteArray &metadataDigest, const QByteArray &payloadCommitment)
{
    QCryptographicHash hash(QCryptographicHash::Sha256);
    hash.addData(metadataDigest);
    hash.addData(payloadCommitment);
    return hash.result();
}

QByteArray Journal::commitmentV3(const CognitiveEnvelope &envelope)
{
    return commitmentFrom(metadataDigestV3(envelope), payloadCommitmentV3(envelope));
}

QByteArray Journal::rowHashV3(
    quint64 seq, const QByteArray &commitment, const QByteArray &prev) const
{
    QByteArray out;
    out.append(QByteArray("CYBOU-JOURNAL-ROW-V3"));
    out.append(static_cast<char>(0));
    out.append(static_cast<char>(3));
    for (int shift = 56; shift >= 0; shift -= 8) {
        out.append(static_cast<char>((seq >> shift) & 0xff));
    }
    out.append(prev);
    out.append(commitment);
    return QCryptographicHash::hash(out, QCryptographicHash::Sha256);
}

QByteArray Journal::rowHashV2(
    quint64 seq, const CognitiveEnvelope &e, const QByteArray &prev) const
{
    return QCryptographicHash::hash(
        canonicalJournalRowV2(seq, prev, e), QCryptographicHash::Sha256);
}

// Validate and write one contribution inside an already-open transaction.
//
// Split out of append() so a batch can reuse the identical validation, hashing and chaining. The
// caller owns the transaction, so this never commits: that is what lets appendBatch amortise one
// fsync across many contributions without any of them skipping a check.
quint64 Journal::appendWithinTransaction(const CognitiveEnvelope &e)
{
    const auto fail = [this](const QString &message) -> quint64 {
        m_lastError = message;
        return 0;
    };

    if (!e.isValid()) {
        return fail(QStringLiteral("refusing to append an invalid envelope"));
    }
    if (e.schemaVersion != kCurrentEnvelopeSchemaVersion
        && e.schemaVersion != kProtectedEnvelopeSchemaVersion) {
        return fail(QStringLiteral("new contributions must use envelope schema v2 or v3"));
    }
    if (contains(e.messageId)) {
        return fail(QStringLiteral("messageId already exists"));
    }

    QList<PrivacyClass> referencePrivacy;
    QList<QDateTime> referenceRetainUntil;
    if (!e.causationId.isNull()) {
        const auto cause = contribution(e.causationId);
        if (!cause) {
            return fail(QStringLiteral("causal contribution does not exist"));
        }
        referencePrivacy.append(cause->privacy);
        referenceRetainUntil.append(cause->retainUntil);
    }

    for (const QUuid &evidenceId : e.evidence) {
        const auto evidenceEnvelope = contribution(evidenceId);
        if (!evidenceEnvelope) {
            return fail(QStringLiteral("evidence contribution does not exist"));
        }
        referencePrivacy.append(evidenceEnvelope->privacy);
        referenceRetainUntil.append(evidenceEnvelope->retainUntil);
    }

    if (e.derivedPrivacy(referencePrivacy) != e.privacy) {
        return fail(QStringLiteral("contribution privacy is weaker than its references"));
    }

    // A conclusion may not outlive the evidence it rests on. Refused rather than silently clamped,
    // exactly as a weaker privacy class is: the envelope's declaration is the contract, and quietly
    // correcting it would leave the caller believing something the Journal does not.
    if (e.derivedRetainUntil(referenceRetainUntil) != e.retainUntil) {
        return fail(QStringLiteral("contribution outlives the retention of its references"));
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

    // Sealing happens here, before anything is hashed, so the commitment is over what will actually
    // be stored. A sealed contribution whose commitment covered the plaintext would be the guessing
    // oracle ADR-0028 exists to remove.
    CognitiveEnvelope stored = e;
    if (e.protection.sealed) {
        if (!m_keys || !m_keys->isUsable()) {
            return fail(QStringLiteral(
                "refusing a sealed contribution: this journal has no key store"));
        }
        const auto dataKey = m_keys->createKeyFor(e.messageId, m_keyEncryptionKey);
        if (!dataKey.has_value()) {
            return fail(QStringLiteral("could not create a data key for the payload"));
        }
        const auto sealedPayload = Seal::seal(e.payloadCbor, *dataKey);
        if (!sealedPayload.has_value()) {
            return fail(QStringLiteral("could not seal the payload"));
        }
        stored.payloadCbor = sealedPayload->nonce + sealedPayload->ciphertext;
        stored.protection.keyDomainId = m_keyDomain.keyDomainId;
        stored.protection.keyEpoch = m_keyDomain.keyEpoch;
    }
    const CognitiveEnvelope &e2 = stored;

    const QByteArray payloadCommitment = payloadCommitmentV3(e2);
    const QByteArray commitment = commitmentFrom(metadataDigestV3(e2), payloadCommitment);
    const QByteArray hash = rowHashV3(sequence, commitment, previousHash);
    QSqlQuery insert(m_db);
    insert.prepare(QStringLiteral(
        "INSERT INTO contribution (seq, message_id, correlation_id, causation_id, "
        "origin_organ, origin_node, kind, wall_time, monotonic_time, logical_clock, "
        "confidence, evidence, payload, privacy, capability, schema_version, hash_version, "
        "prev_hash, hash, commitment, payload_commitment, sealed, key_domain, key_epoch, "
        "retention_class, retention_policy, retain_until) "
        "VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"));

    insert.addBindValue(static_cast<qulonglong>(sequence));
    insert.addBindValue(e2.messageId.toString(QUuid::WithoutBraces));
    insert.addBindValue(e2.correlationId.toString(QUuid::WithoutBraces));
    insert.addBindValue(e2.causationId.isNull()
                            ? QVariant()
                            : QVariant(e.causationId.toString(QUuid::WithoutBraces)));
    insert.addBindValue(e2.originOrgan);
    insert.addBindValue(
        e.originNode.isNull() ? QStringLiteral("") : e.originNode);
    insert.addBindValue(static_cast<int>(e.kind));
    insert.addBindValue(e2.wallTime.toString(Qt::ISODateWithMs));
    insert.addBindValue(static_cast<qulonglong>(e.monotonicTime));
    insert.addBindValue(static_cast<qulonglong>(e.logicalClock));
    insert.addBindValue(e2.confidence);
    insert.addBindValue(QVariant());
    insert.addBindValue(e2.payloadCbor);
    insert.addBindValue(static_cast<int>(e.privacy));
    insert.addBindValue(e2.capabilityScope);
    insert.addBindValue(static_cast<int>(e.schemaVersion));
    insert.addBindValue(kCurrentJournalHashVersion);
    insert.addBindValue(previousHash);
    insert.addBindValue(hash);
    insert.addBindValue(commitment);
    insert.addBindValue(payloadCommitment);
    insert.addBindValue(e2.protection.sealed ? 1 : 0);
    insert.addBindValue(
        e2.protection.keyDomainId.isNull()
            ? QVariant()
            : QVariant(e2.protection.keyDomainId.toString(QUuid::WithoutBraces)));
    insert.addBindValue(static_cast<uint>(e2.protection.keyEpoch));
    insert.addBindValue(static_cast<int>(e2.retentionClass));
    insert.addBindValue(static_cast<uint>(e2.retentionPolicyVersion));
    insert.addBindValue(
        e2.retainUntil.isValid()
            ? QVariant(e2.retainUntil.toUTC().toString(Qt::ISODateWithMs))
            : QVariant());

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

    return sequence;
}

quint64 Journal::append(const CognitiveEnvelope &e)
{
    m_lastError.clear();

    if (!isOpen()) {
        m_lastError = QStringLiteral("journal is not open");
        return 0;
    }
    if (!beginImmediate()) {
        return 0;
    }

    const quint64 sequence = appendWithinTransaction(e);
    if (sequence == 0) {
        rollbackTransaction();
        return 0;
    }
    if (!commitTransaction()) {
        rollbackTransaction();
        return 0;
    }

    Q_EMIT accepted(e, sequence);
    return sequence;
}

// Append many contributions under one transaction, and therefore one fsync.
//
// This exists so a large Journal can be constructed for measurement without spending an fsync per
// contribution, which at a million rows is the difference between minutes and hours. Every
// contribution still goes through exactly the same validation, hashing and chaining as append();
// only the commit is shared.
//
// Deliberately not exposed over Event1. Acceptance there is per-contribution and must remain so:
// batching acceptance would mean publishing Accepted for contributions whose commit had not yet
// returned, which is the durability ordering this Journal exists to preserve. The batch is atomic,
// so a failure anywhere leaves the Journal exactly as it was.
quint64 Journal::appendBatch(const QList<CognitiveEnvelope> &envelopes)
{
    m_lastError.clear();

    if (!isOpen()) {
        m_lastError = QStringLiteral("journal is not open");
        return 0;
    }
    if (envelopes.isEmpty()) {
        return 0;
    }
    if (!beginImmediate()) {
        return 0;
    }

    QList<quint64> sequences;
    sequences.reserve(envelopes.size());
    for (const CognitiveEnvelope &envelope : envelopes) {
        const quint64 sequence = appendWithinTransaction(envelope);
        if (sequence == 0) {
            rollbackTransaction();
            return 0;
        }
        sequences.append(sequence);
    }

    if (!commitTransaction()) {
        rollbackTransaction();
        return 0;
    }

    // Each contribution carries its own sequence. Announcing them all under the batch's last one
    // would hand every subscriber the wrong position in the biography.
    for (int i = 0; i < envelopes.size(); ++i) {
        Q_EMIT accepted(envelopes.at(i), sequences.at(i));
    }
    return sequences.last();
}

// ADR-0028 step one: durable intent.
//
// An ordinary append, deliberately. The request is a contribution like any other - it is hashed,
// chained and verifiable - because the fact that a forgetting was asked for is itself part of the
// biography, and a side channel that mutated the Journal without leaving a trace in it would undo
// the property the single-writer rule exists to provide.
void Journal::setKeyStore(
    KeyStore *keys, const QByteArray &keyEncryptionKey, const KeyDomain &domain)
{
    m_keys = keys;
    m_keyEncryptionKey = keyEncryptionKey;
    m_keyDomain = domain;
}

std::optional<QByteArray> Journal::unsealPayload(const CognitiveEnvelope &envelope) const
{
    if (!envelope.protection.sealed) {
        return envelope.payloadCbor;
    }
    if (!m_keys || !m_keys->isUsable() || envelope.payloadCbor.size() <= kSealNonceBytes) {
        return std::nullopt;
    }

    const auto dataKey = m_keys->keyFor(envelope.messageId, m_keyEncryptionKey);
    if (!dataKey.has_value()) {
        // The key is gone, so the payload is unreadable and stays that way. Not an error to report
        // loudly: after an erasure this is the correct and expected answer.
        return std::nullopt;
    }

    SealedPayload sealed;
    sealed.nonce = envelope.payloadCbor.left(kSealNonceBytes);
    sealed.ciphertext = envelope.payloadCbor.mid(kSealNonceBytes);
    return Seal::unseal(sealed, *dataKey);
}

quint64 Journal::requestErasure(const QUuid &target, const QString &reason)
{
    if (target.isNull() || !contribution(target).has_value()) {
        m_lastError = QStringLiteral("cannot request erasure of a contribution that does not exist");
        return 0;
    }

    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = e.messageId;
    e.causationId = target;
    e.originOrgan = QStringLiteral("eventd");
    e.originNode = QStringLiteral("local");
    e.kind = ContributionKind::ErasureRequested;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.confidence = 1.0;
    e.privacy = PrivacyClass::Local;

    QCborMap payload;
    payload.insert(QStringLiteral("target"), target.toString(QUuid::WithoutBraces));
    // A closed set, never free text: an erasure record is permanent, so a reason that described
    // what was being forgotten would restate it in the one place that can never be erased.
    payload.insert(QStringLiteral("reason"), reason);
    e.payloadCbor = payload.toCborValue().toCbor();

    return append(e);
}

// ADR-0028 step three: redact, mark, bump, record - or none of it.
bool Journal::applyErasure(const QUuid &target)
{
    const auto existing = contribution(target);
    if (!existing.has_value()) {
        m_lastError = QStringLiteral("cannot apply an erasure to a contribution that does not exist");
        return false;
    }

    // Recomputed here rather than carried from the request, so a descendant derived *after* the
    // request was recorded is still reached. A closure frozen at request time would let a race
    // preserve exactly the restatement the erasure was asked to remove.
    const QList<QUuid> closure = retentionDependents(target);
    if (closure.isEmpty()) {
        // The closure always contains the target itself, so an empty one means the query failed.
        // Erasing nothing while reporting success would be the worst available outcome; erasing a
        // subset would be the second worst.
        m_lastError = QStringLiteral("could not determine what depends on the erasure target");
        return false;
    }

    if (!beginImmediate()) {
        return false;
    }

    const QString erasedAt = QDateTime::currentDateTimeUtc().toString(Qt::ISODateWithMs);
    for (const QUuid &id : closure) {
        QSqlQuery redact(m_db);
        redact.prepare(QStringLiteral(
            "UPDATE contribution SET payload = NULL, erased_at = ? "
            "WHERE message_id = ? AND erased_at IS NULL"));
        redact.addBindValue(erasedAt);
        redact.addBindValue(id.toString(QUuid::WithoutBraces));
        if (!redact.exec()) {
            m_lastError = redact.lastError().text();
            rollbackTransaction();
            return false;
        }
    }

    CognitiveEnvelope applied;
    applied.messageId = QUuid::createUuid();
    applied.correlationId = applied.messageId;
    applied.causationId = target;
    applied.originOrgan = QStringLiteral("eventd");
    applied.originNode = QStringLiteral("local");
    applied.kind = ContributionKind::ErasureApplied;
    applied.wallTime = QDateTime::currentDateTimeUtc();
    applied.confidence = 1.0;
    applied.privacy = PrivacyClass::Local;

    QCborMap payload;
    payload.insert(QStringLiteral("target"), target.toString(QUuid::WithoutBraces));
    // The epoch this erasure will occupy once the bump below lands. Recorded here so the backup
    // axis can later be answered by comparing against a declared rotation, without the Journal
    // having to observe backups it cannot see.
    payload.insert(
        QStringLiteral("appliedAtEpoch"), QString::number(erasureEpoch() + 1));
    applied.payloadCbor = payload.toCborValue().toCbor();

    if (appendWithinTransaction(applied) == 0) {
        rollbackTransaction();
        return false;
    }

    // The epoch is bumped inside the same transaction as the redaction, so a projection can never
    // observe a redacted payload while still believing its cached view is current.
    if (!execSql(QStringLiteral("UPDATE journal_meta SET erasure_epoch = erasure_epoch + 1"))) {
        m_lastError = QStringLiteral("could not advance the erasure epoch");
        rollbackTransaction();
        return false;
    }

    if (!commitTransaction()) {
        rollbackTransaction();
        return false;
    }
    return true;
}

QList<QUuid> Journal::retentionDependents(const QUuid &target) const
{
    QList<QUuid> closure;
    if (target.isNull()) {
        return closure;
    }

    // Transitive closure over both kinds of derivation edge, computed in SQLite rather than by
    // repeated queries: the set can be large and a resumed erasure recomputes it from scratch.
    //
    // Erasure records are excluded. They name what was forgotten and carry no observation content,
    // and an erasure that erased its own audit trail would make the trail a suggestion.
    QSqlQuery query(m_db);
    query.prepare(QStringLiteral(R"SQL(
        WITH RECURSIVE dependents(id) AS (
            SELECT ?
            UNION
            SELECT c.message_id FROM contribution c, dependents d
              WHERE c.causation_id = d.id AND c.kind NOT IN (?, ?)
            UNION
            SELECT e.contribution_id FROM contribution_evidence e, dependents d
              WHERE e.evidence_id = d.id
        )
        SELECT id FROM dependents
    )SQL"));
    query.addBindValue(target.toString(QUuid::WithoutBraces));
    query.addBindValue(static_cast<int>(ContributionKind::ErasureRequested));
    query.addBindValue(static_cast<int>(ContributionKind::ErasureApplied));
    if (!query.exec()) {
        // Deliberately silent about the cause here: this is a const read, and the caller that
        // matters - applyErasure - fails closed on an empty closure rather than erasing a subset.
        return {};
    }

    while (query.next()) {
        const QUuid id = QUuid::fromString(query.value(0).toString());
        if (!id.isNull() && !closure.contains(id)) {
            closure.append(id);
        }
    }
    return closure;
}

QList<QUuid> Journal::incompleteErasures() const
{
    QList<QUuid> pending;
    QSqlQuery query(m_db);
    if (!query.exec(QStringLiteral(
            "SELECT r.causation_id FROM contribution r "
            "WHERE r.kind = %1 AND NOT EXISTS ("
            "  SELECT 1 FROM contribution a "
            "  WHERE a.kind = %2 AND a.causation_id = r.causation_id)")
                        .arg(static_cast<int>(ContributionKind::ErasureRequested))
                        .arg(static_cast<int>(ContributionKind::ErasureApplied)))) {
        return pending;
    }
    while (query.next()) {
        const QUuid target = QUuid::fromString(query.value(0).toString());
        if (!target.isNull() && !pending.contains(target)) {
            pending.append(target);
        }
    }
    return pending;
}

bool Journal::declareBackupRotation(quint64 throughEpoch)
{
    // Clamped to what has actually happened. Declaring that backups through some future epoch are
    // gone is a claim about backups that do not exist yet, and honouring it would make every
    // erasure until that epoch report Complete the instant it was applied - the exact reassuring
    // lie the third axis exists to prevent.
    const quint64 declarable = std::min(throughEpoch, erasureEpoch());

    QSqlQuery query(m_db);
    query.prepare(QStringLiteral(
        "UPDATE journal_meta SET rotated_epoch = ? WHERE rotated_epoch < ?"));
    query.addBindValue(static_cast<qulonglong>(declarable));
    query.addBindValue(static_cast<qulonglong>(declarable));
    if (!query.exec()) {
        m_lastError = query.lastError().text();
        return false;
    }
    // Monotonic on purpose: a declaration cannot be walked back. Claiming that older backups have
    // reappeared would be a claim nobody could act on, and the interesting direction is the one
    // where erasure becomes more complete rather than less.
    return true;
}

quint64 Journal::rotatedBackupEpoch() const
{
    QSqlQuery query(m_db);
    if (query.exec(QStringLiteral("SELECT rotated_epoch FROM journal_meta")) && query.next()) {
        return query.value(0).toULongLong();
    }
    return 0;
}

Journal::ErasureStatus Journal::erasureStatus(const QUuid &target) const
{
    ErasureStatus status;
    status.target = target;
    if (target.isNull()) {
        return status;
    }

    quint64 appliedAtEpoch = 0;
    bool requested = false;
    bool applied = false;

    QSqlQuery query(m_db);
    query.prepare(QStringLiteral(
        "SELECT kind, payload FROM contribution WHERE causation_id = ? AND kind IN (?, ?)"));
    query.addBindValue(target.toString(QUuid::WithoutBraces));
    query.addBindValue(static_cast<int>(ContributionKind::ErasureRequested));
    query.addBindValue(static_cast<int>(ContributionKind::ErasureApplied));
    if (!query.exec()) {
        return status;
    }

    while (query.next()) {
        const auto kind = static_cast<ContributionKind>(query.value(0).toInt());
        if (kind == ContributionKind::ErasureRequested) {
            requested = true;
            continue;
        }
        applied = true;
        const QCborMap payload = QCborValue::fromCbor(query.value(1).toByteArray()).toMap();
        appliedAtEpoch =
            payload.value(QStringLiteral("appliedAtEpoch")).toString().toULongLong();
    }

    if (!requested && !applied) {
        return status;
    }
    if (!applied) {
        // Intent is durable and nothing irreversible has happened. Reporting this as any kind of
        // completion would claim a forgetting that has not occurred.
        status.liveState = ErasurePhase::Requested;
        status.projectionsState = ErasurePhase::Requested;
        status.backupState = ErasurePhase::Requested;
        return status;
    }

    status.liveState = ErasurePhase::Complete;
    status.projectionsState = ErasurePhase::Complete;

    // The one axis the Journal cannot settle by itself. A backup taken before this erasure, with a
    // recovery root that still unwraps its keys, defeats it - so until someone declares those
    // backups gone, the honest answer is that this is not finished.
    status.backupState = rotatedBackupEpoch() >= appliedAtEpoch && appliedAtEpoch > 0
        ? ErasurePhase::Complete
        : ErasurePhase::PendingRotation;
    return status;
}

quint64 Journal::erasureEpoch() const
{
    QSqlQuery query(m_db);
    if (query.exec(QStringLiteral("SELECT erasure_epoch FROM journal_meta")) && query.next()) {
        return query.value(0).toULongLong();
    }
    return 0;
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

std::optional<CognitiveEnvelope> Journal::atSequence(quint64 sequence) const
{
    if (sequence == 0) return std::nullopt;
    QSqlQuery query(m_db);
    query.prepare(QStringLiteral("SELECT %1 FROM contribution WHERE seq = ?").arg(envelopeColumns()));
    query.addBindValue(static_cast<qulonglong>(sequence));
    if (!query.exec()) return std::nullopt;
    return readOne(query);
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

    // The protection descriptor round-trips, because it is part of the non-erasable metadata the
    // commitment covers: a reloaded envelope that forgot how it was sealed would not rehash to what
    // was stored, and verification would report every sealed row as broken.
    e.protection.sealed = query.value(offset + 14).toInt() != 0;
    e.protection.keyDomainId = QUuid::fromString(query.value(offset + 15).toString());
    e.protection.keyEpoch = static_cast<quint32>(query.value(offset + 16).toUInt());

    e.retentionClass = static_cast<RetentionClass>(query.value(offset + 17).toInt());
    e.retentionPolicyVersion = static_cast<quint16>(query.value(offset + 18).toUInt());
    const QString retainUntil = query.value(offset + 19).toString();
    e.retainUntil = retainUntil.isEmpty()
        ? QDateTime()
        : QDateTime::fromString(retainUntil, Qt::ISODateWithMs);
    e.evidence = evidenceFor(e.messageId);
    return e;
}

VerifiedCheckpoint Journal::checkpointAtHead() const
{
    VerifiedCheckpoint checkpoint;
    if (!isOpen()) {
        return checkpoint;
    }

    QSqlQuery query(m_db);
    if (query.exec(QStringLiteral("SELECT seq, hash FROM contribution ORDER BY seq DESC LIMIT 1"))
        && query.next()) {
        checkpoint.sequence = query.value(0).toULongLong();
        checkpoint.hash = query.value(1).toByteArray();
        checkpoint.verifiedAt = QDateTime::currentDateTimeUtc();
    }
    return checkpoint;
}

// Walk the chain from `anchor` forward.
//
// Full verification is this with an empty anchor: the chain is rebuilt from the first contribution,
// so the previous hash starts empty and every row is recomputed. With an anchor, the prefix is
// taken on trust and the walk starts from the hash recorded there - which is why the anchor is
// checked against the stored row first. If it does not match, nothing is concluded about the
// journal; the checkpoint is simply unusable and the caller is told so.
VerificationResult Journal::verifyFrom(const VerifiedCheckpoint &anchor) const
{
    VerificationResult result;

    if (!m_db.isOpen()) {
        result.status = VerificationStatus::InvalidAt;
        result.brokenAt = 1;
        return result;
    }

    quint64 startAfter = 0;
    QByteArray expectedPrevious;

    if (!anchor.isEmpty()) {
        QSqlQuery anchorQuery(m_db);
        anchorQuery.prepare(
            QStringLiteral("SELECT hash FROM contribution WHERE seq = ? LIMIT 1"));
        anchorQuery.addBindValue(static_cast<qulonglong>(anchor.sequence));
        if (!anchorQuery.exec() || !anchorQuery.next()
            || anchorQuery.value(0).toByteArray() != anchor.hash) {
            result.status = VerificationStatus::CheckpointMismatch;
            result.brokenAt = anchor.sequence;
            return result;
        }
        startAfter = anchor.sequence;
        expectedPrevious = anchor.hash;
    }

    QSqlQuery query(m_db);
    query.prepare(QStringLiteral(
                      "SELECT seq, hash_version, %1, prev_hash, hash, commitment, "
                      "payload_commitment, erased_at "
                      "FROM contribution WHERE seq > ? ORDER BY seq")
                      .arg(envelopeColumns()));
    query.addBindValue(static_cast<qulonglong>(startAfter));
    if (!query.exec()) {
        result.status = VerificationStatus::InvalidAt;
        result.brokenAt = startAfter + 1;
        return result;
    }

    result.verifiedFrom = startAfter;
    result.verifiedThrough = startAfter;

    quint64 expectedSequence = startAfter + 1;
    while (query.next()) {
        const quint64 sequence = query.value(0).toULongLong();
        if (sequence != expectedSequence) {
            result.status = VerificationStatus::InvalidAt;
            result.brokenAt = expectedSequence;
            return result;
        }

        const int hashVersion = query.value(1).toInt();
        const CognitiveEnvelope e = envelopeFromQuery(query, 2);
        // Positional, and therefore tied to envelopeColumns(): seq and hash_version take slots 0
        // and 1, the envelope takes the next kEnvelopeColumnCount, and these follow. Adding a
        // column to the envelope without moving these is how a verifier ends up comparing a hash
        // against a key epoch.
        const int trailing = 2 + kEnvelopeColumnCount;
        const QByteArray storedPrevious = query.value(trailing).toByteArray();
        const QByteArray storedHash = query.value(trailing + 1).toByteArray();
        if (storedPrevious != expectedPrevious) {
            result.status = VerificationStatus::InvalidAt;
            result.brokenAt = sequence;
            return result;
        }

        QByteArray expectedHash;
        if (hashVersion == kLegacyJournalHashVersion) {
            expectedHash = rowHashV1(sequence, e, storedPrevious);
        } else if (hashVersion == kEnvelopeByValueJournalHashVersion
                   && e.schemaVersion == kCurrentEnvelopeSchemaVersion) {
            expectedHash = rowHashV2(sequence, e, storedPrevious);
        } else if (hashVersion == kCurrentJournalHashVersion
                   && (e.schemaVersion == kCurrentEnvelopeSchemaVersion
                       || e.schemaVersion == kProtectedEnvelopeSchemaVersion)) {
            const QByteArray storedCommitment = query.value(trailing + 2).toByteArray();
            const QByteArray storedPayloadCommitment = query.value(trailing + 3).toByteArray();
            const bool erased = !query.value(trailing + 4).isNull();
            expectedHash = rowHashV3(sequence, storedCommitment, storedPrevious);

            if (expectedHash == storedHash) {
                // Metadata binding, checked against the *stored* payload commitment rather than a
                // recomputed one. That is the whole reason the payload commitment is a column: once
                // a payload is erased it can never be recomputed, and combining the two halves live
                // would leave the surviving metadata unverifiable exactly when it matters most -
                // which is the property committing to metadata was added to obtain.
                if (commitmentFrom(metadataDigestV3(e), storedPayloadCommitment)
                    != storedCommitment) {
                    result.status = VerificationStatus::InvalidAt;
                    result.brokenAt = sequence;
                    return result;
                }

                // Content is the other axis. A payload that is gone is skipped, never counted as
                // verified; a payload that is present and disagrees with its commitment is a
                // content failure and not a broken chain.
                if (erased) {
                    ++result.contentSkipped;
                } else if (payloadCommitmentV3(e) != storedPayloadCommitment) {
                    if (result.contentBrokenAt == 0) {
                        result.contentBrokenAt = sequence;
                    }
                } else {
                    ++result.contentVerified;
                }
            }
        } else {
            result.status = VerificationStatus::InvalidAt;
            result.brokenAt = sequence;
            return result;
        }

        if (expectedHash != storedHash) {
            result.status = VerificationStatus::InvalidAt;
            result.brokenAt = sequence;
            return result;
        }

        expectedPrevious = storedHash;
        result.verifiedThrough = sequence;
        ++expectedSequence;
    }

    result.status = anchor.isEmpty()
        ? VerificationStatus::FullyVerified
        : VerificationStatus::VerifiedThrough;
    return result;
}

// The original whole-history contract, kept as-is: 0 means intact, otherwise the first bad
// sequence. Expressed through verifyFrom with no anchor so there is one chain walk to be right
// about rather than two that can drift.
// A Journal used directly has no checkpoint owner - deciding when to trust a prefix belongs to
// whoever persists the checkpoint, which in production is eventd. So this verifies fully and says
// so, rather than inventing an anchor and reporting a weaker claim as if it were the same.
VerificationResult Journal::verifyIncremental() const
{
    return verifyFrom({});
}

quint64 Journal::verify() const
{
    const VerificationResult result = verifyFrom({});
    return result.intact() ? 0 : result.brokenAt;
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

// One page, oldest first, driven off the sequence primary key. Reading `limit + 1` rows is how
// hasMore is answered without a second COUNT over the tail of the journal.
ContributionPage Journal::after(quint64 afterSequence, int limit) const
{
    ContributionPage page;
    if (!isOpen() || limit <= 0) {
        return page;
    }

    QSqlQuery query(m_db);
    query.prepare(QStringLiteral(
                      "SELECT seq, %1 FROM contribution WHERE seq > ? ORDER BY seq LIMIT ?")
                      .arg(envelopeColumns()));
    query.addBindValue(static_cast<qulonglong>(afterSequence));
    query.addBindValue(limit + 1);
    if (!query.exec()) {
        return page;
    }

    while (query.next()) {
        if (page.envelopes.size() == limit) {
            page.hasMore = true;
            break;
        }
        page.lastSequence = query.value(0).toULongLong();
        page.envelopes.append(envelopeFromQuery(query, 1));
    }

    page.head = count();
    page.ok = true;
    return page;
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

quint64 Journal::countAfterExcludingCapability(
    quint64 offset,
    const QString &excludedCapability) const
{
    QSqlQuery query(m_db);
    // A NULL or empty capability scope is not the excluded one, so it counts. Comparing in SQL
    // would drop NULL rows through three-valued logic, which is why the IS NULL arm is explicit.
    query.prepare(QStringLiteral(
        "SELECT COUNT(*) FROM contribution "
        "WHERE seq > ? AND (capability IS NULL OR capability <> ?)"));
    query.addBindValue(static_cast<qulonglong>(offset));
    query.addBindValue(excludedCapability);
    if (!query.exec() || !query.next()) {
        return 0;
    }
    return query.value(0).toULongLong();
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
