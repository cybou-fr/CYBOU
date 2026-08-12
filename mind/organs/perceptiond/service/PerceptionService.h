// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/events/EventStore.h"
#include "cybou/perception/SystemGenerationSource.h"
#include "cybou/protocol/Observation.h"

#include <QCborValue>
#include <QDateTime>
#include <QObject>
#include <QString>

namespace cybou {

/// The first perception adapter: it reads one local source and proposes what it read.
///
/// ADR-0027 bounds what this is allowed to be. It is the *producer* — `originOrgan` is `perceptiond`
/// and Event1 binds that to this executable — while the thing observed is named separately by
/// `sourceId`. It may propose an Observation. It may not own the Journal, mutate system
/// configuration, or decide whether what it reported is still true: freshness, contradiction and
/// supersession belong to the epistemic projection, which does not exist yet.
///
/// So this deliberately does very little. It reads, it reports, and it says whether it could read.
class PerceptionService : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.cybou.Mind.Perception1")

public:
    PerceptionService(
        EventStore *events,
        SystemGenerationSource source,
        QObject *parent = nullptr);

    /// Read the source once and contribute what that produced. Public so the poll timer and the
    /// tests drive the same path, rather than the tests exercising a private shortcut.
    void acquireOnce();

public Q_SLOTS:
    bool Ready() const;
    QString Health() const;
    QString LastError() const;

    /// What the last acquisition did, as a CBOR map: status, acquiredAt, observed, sourceId.
    QByteArray State() const;

Q_SIGNALS:
    void Changed();

private:
    /// Record that the source became readable or stopped being readable.
    ///
    /// Only the change is durable. ADR-0027 settles this: repeating an unchanged failure every poll
    /// would write thousands of contributions saying nothing happened, and "still unreachable one
    /// poll later" is not a second fact. The transition carries its own payload type and is not an
    /// ObservationV1 — it describes this adapter's ability to observe, not the subject it observes.
    void recordAvailabilityTransition(AcquisitionStatus status, const QDateTime &at);

    EventStore *m_events{nullptr};
    SystemGenerationSource m_source;

    /// Whether an unchanged reading is worth contributing again.
    ///
    /// Reading often and contributing every time are different things. Acquisition identity includes
    /// the instant, so an unchanged system polled every ten seconds would produce thousands of
    /// distinct contributions a day, each restating one fact - the same noise the transition rule
    /// exists to avoid, which this originally applied to failures and not to observations.
    ///
    /// Nor is contributing only on change right: within its declared freshness horizon the previous
    /// observation still speaks for the present, but once that lapses nothing does, and a projection
    /// would have to call it stale forever while the adapter watched it be true. So an unchanged
    /// value is re-affirmed no more than once per horizon.
    bool shouldContribute(const ObservationV1 &observation) const;

    /// Empty until the first acquisition, which is why the first result always counts as a change.
    /// Starting from an assumed state would either suppress a real transition or invent one.
    bool m_haveObserved{false};
    AcquisitionStatus m_lastStatus{AcquisitionStatus::SourceUnavailable};
    QDateTime m_lastAcquiredAt;

    /// The last value contributed, and until when it speaks for itself.
    bool m_haveContributed{false};
    QCborValue m_lastContributedValue;
    QDateTime m_lastContributedFreshUntil;

    QString m_lastError;
};

} // namespace cybou
