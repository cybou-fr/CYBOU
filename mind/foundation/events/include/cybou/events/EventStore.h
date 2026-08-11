// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/protocol/CognitiveEnvelope.h"

#include <QDateTime>
#include <QObject>

#include <functional>
#include <optional>

namespace cybou {

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

inline QString verificationStatusToString(VerificationStatus status)
{
    switch (status) {
    case VerificationStatus::FullyVerified: return QStringLiteral("fully-verified");
    case VerificationStatus::VerifiedThrough: return QStringLiteral("verified-through");
    case VerificationStatus::InvalidAt: return QStringLiteral("invalid-at");
    case VerificationStatus::CheckpointMismatch: return QStringLiteral("checkpoint-mismatch");
    }
    return QStringLiteral("unknown");
}

/// One page of a paged replay.
///
/// `ok` distinguishes "no more contributions" from "could not read". Without it an empty page means
/// both, and a replay whose transport died halfway would look exactly like one that finished -
/// state rebuilt from that has a hole in it and nothing indicates anything went wrong.
struct ContributionPage {
    QList<CognitiveEnvelope> envelopes;
    /// Sequence of the last envelope in this page; 0 when the page is empty. This is the cursor to
    /// resume from, taken from the store rather than counted, so a gap cannot cause a silent skip.
    quint64 lastSequence{0};
    /// Highest sequence the store held when the page was read, so a caller can see its own lag.
    quint64 head{0};
    bool hasMore{false};
    bool ok{false};
};

/// Transport-neutral view of the durable cognitive event store.
///
/// M2 Journal implements this locally. M3 EventClient implements it over D-Bus. Organs depend on
/// this contract rather than on SQLite/Journal, which makes eventd the production persistence
/// boundary without rewriting organ semantics.
class EventStore : public QObject
{
    Q_OBJECT

public:
    explicit EventStore(QObject *parent = nullptr)
        : QObject(parent)
    {
    }

    ~EventStore() override = default;

    EventStore(const EventStore &) = delete;
    EventStore &operator=(const EventStore &) = delete;

    virtual bool isOpen() const = 0;
    virtual QString lastError() const = 0;
    virtual int databaseSchemaVersion() const = 0;

    virtual quint64 append(const CognitiveEnvelope &envelope) = 0;

    virtual quint64 count() const = 0;
    virtual QByteArray head() const = 0;
    virtual quint64 verify() const = 0;

    /// Verify the chain, using a checkpoint if the implementation has one.
    ///
    /// The result says which claim it established. An implementation with no checkpoint owner
    /// answers FullyVerified, because that is what it did - never VerifiedThrough, which would
    /// assert a prefix was trusted when none was.
    virtual VerificationResult verifyIncremental() const = 0;

    /// Most recent contributions first, newest to oldest.
    ///
    /// A limit of 0 or less means the entire biography. That is how organs used to rebuild their
    /// state, and it is why `replayAll` exists: this call materialises the whole history at once
    /// and carries it across the transport in a single reply. Prefer it only for genuinely recent
    /// activity, such as what the UI shows.
    virtual QList<CognitiveEnvelope> recent(int limit = 50) const = 0;

    /// One page of contributions after `afterSequence`, oldest first.
    ///
    /// Sequence is the natural cursor: monotonic, assigned by the single writer, and never reused.
    /// Note the order is the opposite of `recent`.
    virtual ContributionPage after(quint64 afterSequence, int limit) const = 0;

    /// Replay the whole biography in pages, oldest first, without holding it all in memory.
    ///
    /// The paged replacement for `recent(0)`. Callers migrating from that must account for the
    /// reversed order: `recent(0)` yields newest first, this yields oldest first.
    ///
    /// Returns false if any page failed, in which case `handle` has seen a prefix of history and
    /// the caller must discard whatever it built rather than treat it as complete.
    bool replayAll(
        const std::function<void(const CognitiveEnvelope &)> &handle,
        int pageSize = 1000) const
    {
        // The cursor advances by the last contribution's own sequence rather than by counting
        // rows. Sequences happen to be dense today, but nothing in this interface promises that,
        // and a future gap would make a counting cursor skip contributions silently.
        quint64 cursor = 0;
        for (;;) {
            const ContributionPage page = after(cursor, pageSize);
            if (!page.ok) {
                return false;
            }
            if (page.envelopes.isEmpty()) {
                return true;
            }
            for (const CognitiveEnvelope &envelope : page.envelopes) {
                handle(envelope);
            }
            if (page.lastSequence <= cursor) {
                // The store did not advance, so continuing would loop forever. Treated as failure
                // rather than completion: the caller has an arbitrary prefix, not a whole history.
                return false;
            }
            cursor = page.lastSequence;
            if (!page.hasMore) {
                return true;
            }
        }
    }
    virtual QList<CognitiveEnvelope> episode(const QUuid &correlationId) const = 0;
    virtual std::optional<CognitiveEnvelope> atSequence(quint64 sequence) const = 0;

    virtual bool contains(const QUuid &messageId) const = 0;
    virtual std::optional<CognitiveEnvelope> contribution(const QUuid &messageId) const = 0;
    virtual QList<QUuid> evidenceFor(const QUuid &messageId) const = 0;
    virtual bool hasOutcomeFor(
        const QUuid &causeId,
        const QString &originOrgan = QString()) const = 0;

Q_SIGNALS:
    /// Durable acceptance boundary. A proposal that did not commit must never appear here.
    void accepted(const CognitiveEnvelope &envelope, quint64 sequence);
};

} // namespace cybou
