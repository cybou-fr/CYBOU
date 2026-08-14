// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/events/EventStore.h"

#include <QSqlDatabase>
#include <QSqlQuery>
#include <QString>

#include <optional>

namespace cybou {

inline constexpr int kCurrentDatabaseSchemaVersion = 2;
inline constexpr int kLegacyJournalHashVersion = 1;
inline constexpr int kEnvelopeByValueJournalHashVersion = 2;

/// v3 chains a split commitment: a digest of the fields erasure never touches, combined with a
/// separate commitment to the payload. That separation is what lets a payload be erased while its
/// row stays verifiable, and it is why erasure is only offered for rows written at this version -
/// a v1 or v2 hash covers the payload by value and cannot be recomputed without it.
inline constexpr int kCurrentJournalHashVersion = 3;

/// SQLite `synchronous` level at or above which a returned COMMIT has reached storage. Below this,
/// Event1 would publish acceptance for a commit that a power loss can still discard.
inline constexpr int kRequiredSynchronousLevel = 2; // FULL


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
    VerificationResult verifyIncremental() const override;

    /// Verify only the contributions after `anchor`, having first confirmed the anchor still
    /// describes this journal.
    ///
    /// An empty anchor means verify everything, which is what a caller that has no checkpoint yet -
    /// or lost one - should do. Nothing here writes a checkpoint: choosing when to trust a prefix
    /// is the caller's decision, not the storage layer's.
    VerificationResult verifyFrom(const VerifiedCheckpoint &anchor) const;

    /// Record an intent to erase, before anything irreversible happens.
    ///
    /// ADR-0028's protocol is durable intent, then idempotent key destruction, then the redaction
    /// transaction. This is step one, and it is the only step a crash can leave alone: a request
    /// with no application claims nothing about what was destroyed, so resuming it is always safe.
    quint64 requestErasure(const QUuid &target, const QString &reason);

    /// Redact a payload and record that it happened, in one transaction with the epoch bump.
    ///
    /// Step three. The caller must have destroyed the key first; this makes no attempt to check,
    /// because a key store is not reachable from a database transaction and pretending otherwise is
    /// what the three-step protocol exists to avoid.
    bool applyErasure(const QUuid &target);

    /// Everything whose retention depends on a target, including the target itself.
    ///
    /// ADR-0028: erasing a payload and leaving the contributions derived from it would destroy the
    /// record Mind was asked to forget and keep the reasoning that restates it. A Learning that
    /// says "because X" is not a cache to be rebuilt - it is biography, and it carries the content
    /// forward.
    ///
    /// Dependencies are derived from causation and evidence, which is where derivation actually
    /// travels. Deliberately not the whole causal graph: a contribution that merely happened
    /// afterwards is not a descendant of what was erased.
    QList<QUuid> retentionDependents(const QUuid &target) const;

    /// Targets whose erasure was requested and never applied.
    ///
    /// The only state a crash can produce, and the reason recovery is a question the Journal can
    /// answer by itself rather than a flag someone has to remember to set.
    QList<QUuid> incompleteErasures() const;

    /// How many erasures have been applied. Every persisted projection records the epoch it was
    /// built under, and one that is behind is discarded rather than repaired.
    quint64 erasureEpoch() const;


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
    /// The metadata half of a v3 commitment, and the payload half, and the row hash over both.
    ///
    /// Kept separate rather than folded into one function because the two halves have different
    /// lifetimes: the metadata digest is recomputable forever, the payload commitment only while
    /// the payload survives.
    static QByteArray metadataDigestV3(const CognitiveEnvelope &envelope);
    static QByteArray payloadCommitmentV3(const CognitiveEnvelope &envelope);
    static QByteArray commitmentV3(const CognitiveEnvelope &envelope);
    /// Combine a metadata digest with an already-stored payload commitment.
    ///
    /// This is the form verification needs after an erasure: the payload is gone, so its commitment
    /// can only be read back, but the metadata is still there and must still be proven to be the
    /// metadata the row committed to.
    static QByteArray commitmentFrom(
        const QByteArray &metadataDigest, const QByteArray &payloadCommitment);
    QByteArray rowHashV3(
        quint64 seq, const QByteArray &commitment, const QByteArray &prev) const;

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
