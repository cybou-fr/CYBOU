// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/storage/Journal.h"

#include <QCryptographicHash>
#include <QFile>
#include <QSqlDatabase>
#include <QSqlQuery>
#include <QTemporaryDir>
#include <QTest>

#include <atomic>
#include <thread>
#include <vector>

using namespace cybou;

namespace {

CognitiveEnvelope observation(
    PrivacyClass privacy = PrivacyClass::Node,
    const QString &organ = QStringLiteral("perceptiond"))
{
    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = e.messageId;
    e.originOrgan = organ;
    e.originNode = QStringLiteral("test-node");
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.kind = ContributionKind::Observation;
    e.privacy = privacy;
    return e;
}

// An observation whose payload is not empty.
//
// The default fixture leaves payloadCbor empty, which makes any test about erasing a payload
// vacuous: erasing nothing changes nothing, and a verifier that had lost the ability to check the
// row would still agree with itself. A sabotage run caught exactly that, so erasure tests use this.
CognitiveEnvelope observationWithPayload(const QByteArray &payload)
{
    CognitiveEnvelope e = observation();
    e.payloadCbor = payload;
    return e;
}

CognitiveEnvelope derived(
    ContributionKind kind,
    const CognitiveEnvelope &cause,
    const QString &organ = QStringLiteral("modeld"))
{
    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = cause.correlationId;
    e.causationId = cause.messageId;
    e.originOrgan = organ;
    e.originNode = cause.originNode;
    e.kind = kind;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.privacy = cause.privacy;
    return e;
}

QByteArray legacyHash(
    quint64 sequence, const CognitiveEnvelope &e, const QByteArray &previousHash = {})
{
    QCryptographicHash hash(QCryptographicHash::Sha256);
    hash.addData(previousHash);
    hash.addData(QByteArray::number(static_cast<qulonglong>(sequence)));
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

QByteArray createLegacyDatabase(
    const QString &path, const CognitiveEnvelope &legacyEnvelope, const QString &evidence = {})
{
    const QString connection = QStringLiteral("legacy-%1")
                                   .arg(QUuid::createUuid().toString(QUuid::Id128));
    const QByteArray storedHash = legacyHash(1, legacyEnvelope);
    {
        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), connection);
        db.setDatabaseName(path);
        if (!db.open()) {
            return {};
        }

        QSqlQuery schema(db);
        if (!schema.exec(QStringLiteral(R"SQL(
            CREATE TABLE contribution (
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
            return {};
        }

        QSqlQuery insert(db);
        insert.prepare(QStringLiteral(
            "INSERT INTO contribution (seq, message_id, correlation_id, causation_id, "
            "origin_organ, origin_node, kind, wall_time, monotonic_time, logical_clock, "
            "confidence, evidence, payload, privacy, capability, prev_hash, hash) "
            "VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"));
        insert.addBindValue(1);
        insert.addBindValue(legacyEnvelope.messageId.toString(QUuid::WithoutBraces));
        insert.addBindValue(legacyEnvelope.correlationId.toString(QUuid::WithoutBraces));
        insert.addBindValue(QVariant());
        insert.addBindValue(legacyEnvelope.originOrgan);
        insert.addBindValue(legacyEnvelope.originNode);
        insert.addBindValue(static_cast<int>(legacyEnvelope.kind));
        insert.addBindValue(legacyEnvelope.wallTime.toString(Qt::ISODateWithMs));
        insert.addBindValue(static_cast<qulonglong>(legacyEnvelope.monotonicTime));
        insert.addBindValue(static_cast<qulonglong>(legacyEnvelope.logicalClock));
        insert.addBindValue(legacyEnvelope.confidence);
        insert.addBindValue(evidence);
        insert.addBindValue(legacyEnvelope.payloadCbor);
        insert.addBindValue(static_cast<int>(legacyEnvelope.privacy));
        insert.addBindValue(legacyEnvelope.capabilityScope);
        insert.addBindValue(QByteArray());
        insert.addBindValue(storedHash);
        if (!insert.exec()) {
            return {};
        }
        db.close();
    }
    QSqlDatabase::removeDatabase(connection);
    return storedHash;
}

bool rawExec(const QString &path, const QString &sql)
{
    const QString connection = QStringLiteral("raw-%1")
                                   .arg(QUuid::createUuid().toString(QUuid::Id128));
    bool ok = false;
    {
        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), connection);
        db.setDatabaseName(path);
        if (db.open()) {
            QSqlQuery query(db);
            ok = query.exec(sql);
            db.close();
        }
    }
    QSqlDatabase::removeDatabase(connection);
    return ok;
}

QString persistedJournalMode(const QString &path)
{
    const QString connection = QStringLiteral("mode-%1")
                                   .arg(QUuid::createUuid().toString(QUuid::Id128));
    QString mode;
    {
        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), connection);
        db.setDatabaseName(path);
        if (db.open()) {
            QSqlQuery query(db);
            if (query.exec(QStringLiteral("PRAGMA journal_mode")) && query.next()) {
                mode = query.value(0).toString().toLower();
            }
            db.close();
        }
    }
    QSqlDatabase::removeDatabase(connection);
    return mode;
}

bool indexExists(const QString &path, const QString &indexName)
{
    const QString connection = QStringLiteral("index-%1")
                                   .arg(QUuid::createUuid().toString(QUuid::Id128));
    bool found = false;
    {
        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), connection);
        db.setDatabaseName(path);
        if (db.open()) {
            QSqlQuery query(db);
            query.prepare(QStringLiteral(
                "SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = ?"));
            query.addBindValue(indexName);
            found = query.exec() && query.next();
            db.close();
        }
    }
    QSqlDatabase::removeDatabase(connection);
    return found;
}

} // namespace

class TestJournal : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void appendsAndFindsContributions()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        const CognitiveEnvelope root = observation();

        QVERIFY(journal.isOpen());
        QCOMPARE(journal.databaseSchemaVersion(), kCurrentDatabaseSchemaVersion);
        QCOMPARE(journal.append(root), 1u);
        QVERIFY(journal.contains(root.messageId));
        const auto stored = journal.contribution(root.messageId);
        QVERIFY(stored.has_value());
        QCOMPARE(stored->schemaVersion, kCurrentEnvelopeSchemaVersion);
        const auto bySequence = journal.atSequence(1);
        QVERIFY(bySequence.has_value());
        QCOMPARE(bySequence->messageId, root.messageId);
        QVERIFY(!journal.atSequence(0).has_value());
        QVERIFY(!journal.atSequence(2).has_value());
        QCOMPARE(journal.verify(), 0u);
    }

    void nullOriginNodeIsStoredAsEmptyText()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));

        CognitiveEnvelope root = observation();
        root.originNode = QString();
        QVERIFY(root.originNode.isNull());
        QVERIFY2(journal.append(root) > 0, qPrintable(journal.lastError()));

        const auto stored = journal.contribution(root.messageId);
        QVERIFY(stored.has_value());
        QVERIFY(!stored->originNode.isNull());
        QVERIFY(stored->originNode.isEmpty());
    }

    void missingCauseAndEvidenceAreRejected()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));

        CognitiveEnvelope withMissingCause = observation();
        withMissingCause.kind = ContributionKind::Decision;
        withMissingCause.causationId = QUuid::createUuid();
        QCOMPARE(journal.append(withMissingCause), 0u);
        QVERIFY(journal.lastError().contains(QStringLiteral("causal")));

        CognitiveEnvelope withMissingEvidence = observation();
        withMissingEvidence.kind = ContributionKind::Prediction;
        withMissingEvidence.evidence = {QUuid::createUuid()};
        QCOMPARE(journal.append(withMissingEvidence), 0u);
        QVERIFY(journal.lastError().contains(QStringLiteral("evidence")));
    }

    void evidenceAndCapabilityRoundTrip()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));

        const CognitiveEnvelope first = observation();
        const CognitiveEnvelope second = observation();
        QVERIFY(journal.append(first) > 0);
        QVERIFY(journal.append(second) > 0);

        CognitiveEnvelope prediction = observation();
        prediction.kind = ContributionKind::Prediction;
        prediction.evidence = {first.messageId, second.messageId};
        prediction.capabilityScope = QStringLiteral("system.observe");
        QVERIFY(journal.append(prediction) > 0);

        const auto stored = journal.contribution(prediction.messageId);
        QVERIFY(stored.has_value());
        QCOMPARE(stored->evidence, prediction.evidence);
        QCOMPARE(stored->capabilityScope, prediction.capabilityScope);
    }

    void privacyCannotBeWeakened()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));

        const CognitiveEnvelope local = observation(PrivacyClass::Local);
        QVERIFY(journal.append(local) > 0);

        CognitiveEnvelope publicConclusion = derived(ContributionKind::Learning, local);
        publicConclusion.privacy = PrivacyClass::Public;
        QCOMPARE(journal.append(publicConclusion), 0u);
    }

    void fullEnvelopeHashDetectsSemanticMutation()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));
        {
            Journal journal(path);
            CognitiveEnvelope root = observation();
            root.confidence = 0.75;
            root.capabilityScope = QStringLiteral("system.observe");
            QVERIFY(journal.append(root) > 0);
            QCOMPARE(journal.verify(), 0u);
        }

        QVERIFY(rawExec(path, QStringLiteral(
            "UPDATE contribution SET privacy = 3, confidence = 0.25 WHERE seq = 1")));

        Journal journal(path);
        QCOMPARE(journal.verify(), 1u);
    }

    void evidenceTableMutationBreaksV2Hash()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));
        QUuid predictionId;
        {
            Journal journal(path);
            const CognitiveEnvelope first = observation();
            const CognitiveEnvelope second = observation();
            QVERIFY(journal.append(first) > 0);
            QVERIFY(journal.append(second) > 0);

            CognitiveEnvelope prediction = observation();
            prediction.kind = ContributionKind::Prediction;
            prediction.evidence = {first.messageId, second.messageId};
            predictionId = prediction.messageId;
            QVERIFY(journal.append(prediction) > 0);
        }

        QVERIFY(rawExec(
            path,
            QStringLiteral(
                "DELETE FROM contribution_evidence "
                "WHERE contribution_id = '%1' AND ordinal = 0")
                .arg(predictionId.toString(QUuid::WithoutBraces))));

        Journal journal(path);
        QCOMPARE(journal.verify(), 3u);
    }

    // E1: an untouched v3 row verifies both its place in the chain and its content.
    //
    // Two answers now, not one. Until a payload can be erased they always agree, which is exactly
    // why this has to be asserted before erasure exists: afterwards, a bug that conflated them
    // would look like the feature working.
    void aV3RowVerifiesChainAndContent()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());

        for (int i = 0; i < 5; ++i) {
            QVERIFY(journal.append(observationWithPayload(QByteArray("payload-") + char('0' + i)))
                    > 0);
        }

        QCOMPARE(journal.verify(), 0u);

        const VerificationResult result = journal.verifyFrom({});
        QVERIFY(result.intact());
        QCOMPARE(result.verifiedThrough, 5u);
        // Every row still holds its payload, so every row was content-checked. A number lower than
        // this would mean something went unverified while the chain still passed.
        QCOMPARE(result.contentVerified, 5u);
    }

    // The v3 commitment covers the fields erasure never touches, and this is what that buys.
    //
    // An earlier design committed to the payload alone. Under it, rewriting a contribution's author
    // would leave every hash undisturbed - the provenance binding Event1 enforces at submission
    // would survive exactly until someone edited the database. Forgetting must not cost that.
    void rewritingTheAuthorOfAV3RowBreaksTheChain()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));
        {
            Journal journal(path);
            QVERIFY(journal.isOpen());
            QVERIFY(journal.append(observationWithPayload(QByteArray("first"))) > 0);
            QVERIFY(journal.append(observationWithPayload(QByteArray("second"))) > 0);
            QCOMPARE(journal.verify(), 0u);
        }

        QVERIFY(rawExec(
            path,
            QStringLiteral("UPDATE contribution SET origin_organ = 'impostor' WHERE seq = 2")));

        Journal reopened(path);
        QVERIFY(reopened.isOpen());
        QCOMPARE(reopened.verify(), 2u);
    }

    // Payload tampering is a content failure, not a broken chain, and the difference is the point.
    //
    // Folding it into InvalidAt would say the biography's structure is damaged when one record's
    // contents are - and after erasure it would make every legitimately forgotten row look
    // identical to a corrupted one.
    void rewritingThePayloadOfAV3RowIsAContentFailureNotAChainFailure()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));
        {
            Journal journal(path);
            QVERIFY(journal.isOpen());
            QVERIFY(journal.append(observationWithPayload(QByteArray("first"))) > 0);
            QVERIFY(journal.append(observationWithPayload(QByteArray("second"))) > 0);
            QCOMPARE(journal.verify(), 0u);
        }

        QVERIFY(rawExec(
            path, QStringLiteral("UPDATE contribution SET payload = X'00' WHERE seq = 1")));

        Journal reopened(path);
        QVERIFY(reopened.isOpen());
        const VerificationResult result = reopened.verifyFrom({});

        QVERIFY2(result.intact(), "the chain is untouched; only one payload changed");
        QCOMPARE(result.brokenAt, 0u);
        QCOMPARE(result.contentBrokenAt, 1u);
        QVERIFY(!result.contentIntact());
        QCOMPARE(result.contentVerified, 1u);
    }

    // E2: an erased row verifies its chain and its metadata, and reports its content as skipped.
    //
    // Redacting the payload directly is what the erasure path will do; doing it here proves the
    // storage semantics before the state machine that will drive them exists, which is the order
    // the review argued for and it is the right one - a state machine built on storage that cannot
    // survive erasure would pass its own tests and fail the thing it is for.
    void anErasedRowKeepsItsChainAndReportsContentSkipped()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));
        {
            Journal journal(path);
            QVERIFY(journal.isOpen());
            for (int i = 0; i < 3; ++i) {
                QVERIFY(
                    journal.append(observationWithPayload(QByteArray("payload-") + char('0' + i)))
                    > 0);
            }
            QCOMPARE(journal.verify(), 0u);
        }

        QVERIFY(rawExec(
            path,
            QStringLiteral("UPDATE contribution SET payload = NULL, "
                           "erased_at = '2026-08-14T00:00:00.000Z' WHERE seq = 2")));

        Journal reopened(path);
        QVERIFY(reopened.isOpen());
        const VerificationResult result = reopened.verifyFrom({});

        QVERIFY2(result.intact(), "erasing a payload must not break the chain");
        QCOMPARE(result.brokenAt, 0u);
        QCOMPARE(result.verifiedThrough, 3u);

        // Skipped, never verified. The distinction is the whole reason for the second axis.
        QCOMPARE(result.contentSkipped, 1u);
        QCOMPARE(result.contentVerified, 2u);
        QCOMPARE(result.contentBrokenAt, 0u);
    }

    // And the metadata of an erased row is still provably the metadata it committed to.
    //
    // This is what the separately stored payload commitment buys. Without it the surviving metadata
    // could only be checked while the payload was there - so the provenance binding would evaporate
    // at exactly the moment forgetting made it unrecomputable, and an erased row's author could be
    // rewritten freely.
    void anErasedRowStillProvesItsAuthor()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));
        {
            Journal journal(path);
            QVERIFY(journal.isOpen());
            QVERIFY(journal.append(observationWithPayload(QByteArray("first"))) > 0);
            QVERIFY(journal.append(observationWithPayload(QByteArray("secret"))) > 0);
            QCOMPARE(journal.verify(), 0u);
        }

        QVERIFY(rawExec(
            path,
            QStringLiteral("UPDATE contribution SET payload = NULL, "
                           "erased_at = '2026-08-14T00:00:00.000Z' WHERE seq = 2")));
        QVERIFY(rawExec(
            path,
            QStringLiteral("UPDATE contribution SET origin_organ = 'impostor' WHERE seq = 2")));

        Journal reopened(path);
        QVERIFY(reopened.isOpen());
        QCOMPARE(reopened.verify(), 2u);
    }

    void migratesV1WithoutRehashingHistory()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));
        CognitiveEnvelope legacy = observation();
        legacy.schemaVersion = kLegacyEnvelopeSchemaVersion;
        const QByteArray originalHash = createLegacyDatabase(path, legacy);
        QVERIFY(!originalHash.isEmpty());

        Journal journal(path);
        QVERIFY2(journal.isOpen(), qPrintable(journal.lastError()));
        QCOMPARE(journal.databaseSchemaVersion(), kCurrentDatabaseSchemaVersion);
        QVERIFY(QFile::exists(path + QStringLiteral(".v1.bak")));
        QCOMPARE(journal.head(), originalHash);
        QCOMPARE(journal.verify(), 0u);

        const auto restored = journal.contribution(legacy.messageId);
        QVERIFY(restored.has_value());
        QCOMPARE(restored->schemaVersion, kLegacyEnvelopeSchemaVersion);

        const CognitiveEnvelope current = observation();
        QCOMPARE(journal.append(current), 2u);
        QCOMPARE(journal.verify(), 0u);
    }

    void malformedLegacyEvidenceFailsClosed()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));
        CognitiveEnvelope legacy = observation();
        legacy.schemaVersion = kLegacyEnvelopeSchemaVersion;
        QVERIFY(!createLegacyDatabase(
                     path, legacy, QUuid::createUuid().toString(QUuid::WithoutBraces))
                     .isEmpty());

        Journal journal(path);
        QVERIFY(!journal.isOpen());
        QVERIFY(journal.lastError().contains(QStringLiteral("missing contribution")));
        QVERIFY(QFile::exists(path + QStringLiteral(".v1.bak")));
    }

    void concurrentWritersProduceOneContinuousChain()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));
        {
            Journal initialize(path);
            QVERIFY(initialize.isOpen());
        }

        constexpr int writerCount = 4;
        constexpr int writesPerWriter = 8;
        std::atomic<bool> start{false};
        std::atomic<int> written{0};
        std::vector<std::thread> workers;

        for (int writer = 0; writer < writerCount; ++writer) {
            workers.emplace_back([&, writer]() {
                Journal journal(path, QStringLiteral("writer-%1").arg(writer));
                while (!start.load()) {
                    std::this_thread::yield();
                }
                for (int i = 0; i < writesPerWriter; ++i) {
                    if (journal.append(observation()) > 0) {
                        ++written;
                    }
                }
            });
        }

        start = true;
        for (std::thread &worker : workers) {
            worker.join();
        }

        QCOMPARE(written.load(), writerCount * writesPerWriter);
        Journal journal(path);
        QCOMPARE(journal.count(), static_cast<quint64>(writerCount * writesPerWriter));
        QCOMPARE(journal.verify(), 0u);
    }

    void oneTerminalOutcomeIsProtectedBySqlite()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));
        Journal journal(path);

        const CognitiveEnvelope root = observation();
        QVERIFY(journal.append(root) > 0);
        const CognitiveEnvelope intention = derived(
            ContributionKind::Intention, root, QStringLiteral("intentiond"));
        QVERIFY(journal.append(intention) > 0);

        const CognitiveEnvelope first = derived(
            ContributionKind::Outcome, intention, QStringLiteral("intentiond"));
        QVERIFY(journal.append(first) > 0);
        const CognitiveEnvelope second = derived(
            ContributionKind::Outcome, intention, QStringLiteral("predictord"));
        QCOMPARE(journal.append(second), 0u);

        QVERIFY(indexExists(path, QStringLiteral("idx_one_outcome_per_cause")));
    }

    // Event1 publishes Accepted only after COMMIT returns, so the commit mode is part of the
    // "durable before visible" invariant rather than a tuning detail. Journal mode is persisted in
    // the database file, so a silent fallback away from WAL is observable here; the synchronous
    // level is connection-scoped and is enforced by ensureDurability() at open, which is what a
    // successful open on a file-backed path demonstrates.
    void durableCommitModeIsEnforcedAtOpen()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));
        Journal journal(path);

        QVERIFY2(
            journal.isOpen(),
            qPrintable(QStringLiteral("journal refused to open: %1").arg(journal.lastError())));
        QVERIFY(journal.append(observation()) > 0);
        QCOMPARE(persistedJournalMode(path), QStringLiteral("wal"));
    }

    // The exemption is deliberate: an in-memory journal is test scaffolding and makes no durability
    // claim, so requiring WAL of it would only break the suites that use it.
    void inMemoryJournalIsExemptFromDurabilityEnforcement()
    {
        Journal journal(QStringLiteral(":memory:"));

        QVERIFY(journal.isOpen());
        QVERIFY(journal.append(observation()) > 0);
    }

    // Consolidation must not count its own output as new input. Answering that with one aggregate
    // query rather than decoding every envelope after the offset removes an unbounded per-call cost
    // from the process that owns the only write path.
    void capabilityExcludingCountSkipsOnlyTheNamedScope()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        const QString consolidation = QStringLiteral("lifecycle.consolidation");

        CognitiveEnvelope unscoped = observation();
        QVERIFY(journal.append(unscoped) > 0);

        CognitiveEnvelope owned = observation();
        owned.capabilityScope = consolidation;
        QVERIFY(journal.append(owned) > 0);

        CognitiveEnvelope other = observation();
        other.capabilityScope = QStringLiteral("presence.promise");
        QVERIFY(journal.append(other) > 0);

        // A contribution with no capability scope is stored as NULL, and a plain SQL inequality
        // would silently drop it through three-valued logic. It is not the excluded scope, so it
        // must count.
        QCOMPARE(journal.countAfterExcludingCapability(0, consolidation), 2u);

        // The offset is exclusive, matching the consumer high-water mark.
        QCOMPARE(journal.countAfterExcludingCapability(1, consolidation), 1u);
        QCOMPARE(journal.countAfterExcludingCapability(3, consolidation), 0u);

        // Excluding a scope no contribution carries leaves every row counted.
        QCOMPARE(journal.countAfterExcludingCapability(0, QStringLiteral("absent.scope")), 3u);
    }

    // Paging is the replacement for recent(0). What has to hold is that the pages tile the history
    // exactly: no contribution seen twice, none skipped, and the end distinguishable from a failure.
    void pagedReplayTilesHistoryExactly()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());

        QList<QUuid> written;
        for (int i = 0; i < 25; ++i) {
            CognitiveEnvelope e = observation();
            QVERIFY(journal.append(e) > 0);
            written.append(e.messageId);
        }

        // A page smaller than the history reports more to come and stops exactly on the boundary.
        const ContributionPage first = journal.after(0, 10);
        QVERIFY(first.ok);
        QCOMPARE(first.envelopes.size(), 10);
        QVERIFY(first.hasMore);
        QCOMPARE(first.lastSequence, 10u);
        QCOMPARE(first.head, 25u);
        QCOMPARE(first.envelopes.first().messageId, written.first());

        // Resuming from the cursor continues rather than repeats.
        const ContributionPage second = journal.after(first.lastSequence, 10);
        QVERIFY(second.ok);
        QCOMPARE(second.envelopes.first().messageId, written.at(10));

        // The final page reports no more, which is how the end is told apart from a failure.
        const ContributionPage last = journal.after(20, 10);
        QVERIFY(last.ok);
        QCOMPARE(last.envelopes.size(), 5);
        QVERIFY(!last.hasMore);
        QCOMPARE(last.lastSequence, 25u);

        // Past the end: empty, but still a successful read.
        const ContributionPage beyond = journal.after(25, 10);
        QVERIFY(beyond.ok);
        QVERIFY(beyond.envelopes.isEmpty());
        QVERIFY(!beyond.hasMore);

        // replayAll walks the whole history, oldest first, across page boundaries.
        QList<QUuid> replayed;
        QVERIFY(journal.replayAll(
            [&replayed](const CognitiveEnvelope &e) { replayed.append(e.messageId); }, 7));
        QCOMPARE(replayed, written);

        // recent(0) yields the same contributions in the opposite order. Stating it here is what
        // makes the hazard explicit for anything still being migrated off it.
        QList<QUuid> reversed;
        for (const CognitiveEnvelope &e : journal.recent(0)) {
            reversed.prepend(e.messageId);
        }
        QCOMPARE(reversed, written);
    }

    // Incremental verification exists because a full rechain costs ~10.9 us per contribution and is
    // reachable from selfd's ordinary self-assessment, so it exhausts the Presence command budget
    // near 460k contributions. What must not happen is that the cheaper check quietly reports the
    // stronger claim.
    void incrementalVerificationReportsWhatItActuallyChecked()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));
        VerifiedCheckpoint anchor;

        {
            Journal journal(path);
            QVERIFY(journal.isOpen());
            for (int i = 0; i < 10; ++i) {
                QVERIFY(journal.append(observation()) > 0);
            }

            // No checkpoint means verify everything, and say so.
            const VerificationResult full = journal.verifyFrom({});
            QCOMPARE(full.status, VerificationStatus::FullyVerified);
            QCOMPARE(full.verifiedFrom, 0u);
            QCOMPARE(full.verifiedThrough, 10u);
            QVERIFY(full.intact());

            anchor = journal.checkpointAtHead();
            QCOMPARE(anchor.sequence, 10u);
            QVERIFY(!anchor.hash.isEmpty());

            for (int i = 0; i < 5; ++i) {
                QVERIFY(journal.append(observation()) > 0);
            }

            // With a checkpoint only the suffix is examined, and the status says so rather than
            // claiming the whole history was rebuilt.
            const VerificationResult incremental = journal.verifyFrom(anchor);
            QCOMPARE(incremental.status, VerificationStatus::VerifiedThrough);
            QCOMPARE(incremental.verifiedFrom, 10u);
            QCOMPARE(incremental.verifiedThrough, 15u);
            QVERIFY(incremental.intact());

            // A checkpoint that does not describe this journal is unusable. That is not the same as
            // the journal being bad, and reporting it as corruption would send a caller looking for
            // damage that is not there.
            VerifiedCheckpoint wrong = anchor;
            wrong.hash = QByteArray("not-the-hash-that-was-recorded");
            const VerificationResult mismatch = journal.verifyFrom(wrong);
            QCOMPARE(mismatch.status, VerificationStatus::CheckpointMismatch);
            QVERIFY(!mismatch.intact());

            // An anchor past the end is equally unusable.
            VerifiedCheckpoint beyond = anchor;
            beyond.sequence = 999;
            QCOMPARE(journal.verifyFrom(beyond).status, VerificationStatus::CheckpointMismatch);
        }

        // Corrupt a contribution inside the checkpointed prefix, then verify both ways. This is the
        // honest limit of the optimisation: incremental verification trusts the prefix, so it
        // cannot see this, and only the full walk can. The test states that limit rather than
        // pretending the cheap check is equivalent.
        {
            QSqlDatabase db = QSqlDatabase::addDatabase(
                QStringLiteral("QSQLITE"), QStringLiteral("corrupt"));
            db.setDatabaseName(path);
            QVERIFY(db.open());
            QSqlQuery query(db);
            QVERIFY(query.exec(
                QStringLiteral("UPDATE contribution SET origin_organ = 'tampered' WHERE seq = 3")));
            db.close();
        }
        QSqlDatabase::removeDatabase(QStringLiteral("corrupt"));

        Journal reopened(path);
        QVERIFY(reopened.isOpen());

        const VerificationResult afterTampering = reopened.verifyFrom(anchor);
        QCOMPARE(afterTampering.status, VerificationStatus::VerifiedThrough);
        QVERIFY(afterTampering.intact());

        const VerificationResult fullAfterTampering = reopened.verifyFrom({});
        QCOMPARE(fullAfterTampering.status, VerificationStatus::InvalidAt);
        QCOMPARE(fullAfterTampering.brokenAt, 3u);
        QVERIFY(!fullAfterTampering.intact());

        // The whole-history contract is unchanged for existing callers.
        QCOMPARE(reopened.verify(), 3u);
    }

    void episodeReplaysInOrder()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));

        const CognitiveEnvelope root = observation();
        QVERIFY(journal.append(root) > 0);
        const CognitiveEnvelope hypothesis = derived(ContributionKind::Hypothesis, root);
        QVERIFY(journal.append(hypothesis) > 0);
        const CognitiveEnvelope decision = derived(ContributionKind::Decision, hypothesis);
        QVERIFY(journal.append(decision) > 0);

        const auto episode = journal.episode(root.correlationId);
        QCOMPARE(episode.size(), 3);
        QCOMPARE(episode.at(0).messageId, root.messageId);
        QCOMPARE(episode.at(1).causationId, root.messageId);
        QCOMPARE(episode.at(2).causationId, hypothesis.messageId);
    }
};

QTEST_MAIN(TestJournal)
#include "tst_journal.moc"
