// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/events/EventStore.h"

#include <QDateTime>
#include <QSqlDatabase>
#include <QSqlQuery>
#include <QString>

#include <optional>

namespace cybou {

inline constexpr int kCurrentDatabaseSchemaVersion = 2;
inline constexpr int kLegacyJournalHashVersion = 1;
inline constexpr int kCurrentJournalHashVersion = 2;

/// SQLite `synchronous` level at or above which a returned COMMIT has reached storage. Below this,
/// Event1 would publish acceptance for a commit that a power loss can still discard.
inline constexpr int kRequiredSynchronousLevel = 2; // FULL

/// A position in the hash chain that was verified, and the hash observed there.
///
/// This is an accelerator, never an authority. The Journal remains the only source of truth about
/// its own integrity: a checkpoint lets verification skip a prefix it has already checked, and
/// losing one costs a full verification rather than any correctness.
struct VerifiedCheckpoint {
    quint64 sequence{0};
    QByteArray hash;
    QDateTime verifiedAt;

    bool isEmpty() const { return sequence == 0 || hash.isEmpty(); }
};

/// What a verification actually established.
///
/// The distinction is the point. Full verification rechains from the beginning; incremental
/// verification trusts a prefix it checked earlier. Both are useful, but presenting the second as
/// the first would claim evidence that was not gathered - so the result says which happened, and a
/// caller that needs a whole-history guarantee can tell it did not get one.
enum class VerificationStatus {
    /// The chain was rebuilt from the first contribution and holds throughout.
    FullyVerified,
    /// The chain holds from the checkpoint forward. The prefix was not re-examined.
    VerifiedThrough,
    /// The chain is broken. `brokenAt` is the first bad sequence.
    InvalidAt,
    /// The checkpoint does not describe this journal: the anchor row is missing or its hash differs.
    /// The journal is not thereby proven bad - the checkpoint is proven unusable, and the caller
    /// must fall back to full verification rather than trust either.
    CheckpointMismatch,
};

struct VerificationResult {
    VerificationStatus status{VerificationStatus::InvalidAt};
    /// Exclusive lower bound actually examined; 0 when verification started from the beginning.
    quint64 verifiedFrom{0};
    /// Highest sequence confirmed good.
    quint64 verifiedThrough{0};
    /// First bad sequence, or 0 when nothing is known to be bad.
    quint64 brokenAt{0};

    bool intact() const
    {
        return status == VerificationStatus::FullyVerified
            || status == VerificationStatus::VerifiedThrough;
    }
};

QString verificationStatusToString(VerificationStatus status);

/// Low-level SQLite implementation.
///
/// Production organs do not depend on this class after M3. cybou-eventd owns the production
/// instance; tests may still instantiate Journal directly behind the EventStore contract.
class Journal : public EventStore
{
    Q_OBJECT

public:
    explicit Journal(
        const QString &path,
        const QString &connectionName = QString(),
        QObject *parent = nullptr);
    ~Journal() override;

    Journal(const Journal &) = delete;
    Journal &operator=(const Journal &) = delete;

    bool isOpen() const override;
    QString lastError() const override;
    int databaseSchemaVersion() const override;

    quint64 append(const CognitiveEnvelope &envelope) override;

    /// Append many contributions under one transaction, returning the last accepted sequence.
    ///
    /// Every contribution is validated, hashed and chained exactly as `append` does; only the
    /// commit - and therefore the fsync - is shared. This exists so a large Journal can be built
    /// for measurement without spending one fsync per row.
    ///
    /// Not exposed over Event1, and must not be: acceptance there is per-contribution, and batching
    /// it would publish Accepted for contributions whose commit had not yet returned. The batch is
    /// atomic, so a failure anywhere leaves the Journal unchanged.
    quint64 appendBatch(const QList<CognitiveEnvelope> &envelopes);

    quint64 count() const override;
    QByteArray head() const override;
    quint64 verify() const override;

    /// Verify only the contributions after `anchor`, having first confirmed the anchor still
    /// describes this journal.
    ///
    /// An empty anchor means verify everything, which is what a caller that has no checkpoint yet -
    /// or lost one - should do. Nothing here writes a checkpoint: choosing when to trust a prefix
    /// is the caller's decision, not the storage layer's.
    VerificationResult verifyFrom(const VerifiedCheckpoint &anchor) const;

    /// The checkpoint describing the current head, suitable for persisting after a successful
    /// verification. Empty when the journal is empty or unreadable.
    VerifiedCheckpoint checkpointAtHead() const;

    QList<CognitiveEnvelope> recent(int limit = 50) const override;
    ContributionPage after(quint64 afterSequence, int limit) const override;
    QList<CognitiveEnvelope> episode(const QUuid &correlationId) const override;
    std::optional<CognitiveEnvelope> atSequence(quint64 sequence) const override;

    /// Number of contributions after `offset` whose capability scope is not `excludedCapability`.
    ///
    /// This exists so a consumer backlog can be answered by one aggregate query instead of reading
    /// every envelope after the offset. It is not part of the EventStore contract: it is a counting
    /// shortcut for the Journal owner, not a new way to read biography.
    quint64 countAfterExcludingCapability(
        quint64 offset,
        const QString &excludedCapability) const;

    bool contains(const QUuid &messageId) const override;
    std::optional<CognitiveEnvelope> contribution(const QUuid &messageId) const override;
    QList<QUuid> evidenceFor(const QUuid &messageId) const override;
    bool hasOutcomeFor(
        const QUuid &causeId,
        const QString &originOrgan = QString()) const override;

private:
    /// Validate and write one contribution inside a transaction the caller already opened.
    /// Returns the assigned sequence, or 0 with `m_lastError` set. Never commits or rolls back.
    quint64 appendWithinTransaction(const CognitiveEnvelope &envelope);

    bool ensureDurability();
    bool ensureSchema();
    bool createSchemaV2();
    bool migrateV1ToV2();
    bool ensureV2Indexes();
    bool createMigrationBackup();

    bool beginImmediate();
    bool commitTransaction();
    void rollbackTransaction();
    bool execSql(const QString &sql);

    int userVersion() const;
    bool tableExists(const QString &table) const;
    bool columnExists(const QString &table, const QString &column) const;

    QByteArray rowHashV1(
        quint64 seq, const CognitiveEnvelope &envelope, const QByteArray &previousHash) const;
    QByteArray rowHashV2(
        quint64 seq, const CognitiveEnvelope &envelope, const QByteArray &previousHash) const;

    CognitiveEnvelope envelopeFromQuery(const QSqlQuery &query, int offset) const;
    std::optional<CognitiveEnvelope> readOne(QSqlQuery &query) const;

    QSqlDatabase m_db;
    QString m_connectionName;
    QString m_path;
    QString m_lastError;
    bool m_ready{false};
};

} // namespace cybou
