// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include <QCborValue>
#include <QDateTime>
#include <QString>
#include <QUuid>

#include <optional>

namespace cybou {

inline constexpr quint16 kCurrentObservationSchemaVersion = 1;

/// One typed observation of something outside Mind.
///
/// This is the payload of an Observation contribution, not the contribution itself. The envelope
/// around it answers who brought this into Mind - `originOrgan`, which Event1 binds to the calling
/// process - while this answers what was observed. ADR-0027 keeps those apart deliberately:
/// if the producer were also the source, replacing an adapter would silently rewrite the provenance
/// of everything it had ever reported, and two adapters reading one source would look like two
/// independent sources, which is the condition under which a contradiction check agrees with itself.
struct ObservationV1 {
    quint16 schemaVersion{kCurrentObservationSchemaVersion};

    /// What was observed. Stable across adapter replacement.
    QString sourceId;

    /// The key this observation is about, so successive observations of it are comparable and a
    /// later one can supersede an earlier one.
    QString subject;

    /// Typed, not a string a later reader has to guess the shape of.
    QCborValue value;

    /// When the source was read. Distinct from the envelope's wallTime, which is when Mind accepted
    /// the contribution: acceptance is a fact about Mind, acquisition is a fact about the world, and
    /// a slow adapter must not be able to make a stale reading look recent.
    QDateTime acquiredAt;

    /// When this observation stops being current by declaration rather than by inference. The
    /// adapter knows how fast its source changes; nothing downstream should have to guess.
    QDateTime freshnessUntil;

    /// How the value was obtained, in enough detail to re-derive or challenge it.
    QString provenance;

    /// Structural validity only. This does not say the observation is true, current, or worth
    /// believing - only that it is well formed enough to reason about.
    bool isValid() const;

    /// Whether the declared freshness horizon still covers `at`.
    ///
    /// Freshness is a property of the observation, not of the projection reading it. A projection
    /// may still choose to present a fresh observation as disputed; it may not present a stale one
    /// as current.
    bool isFreshAt(const QDateTime &at) const;
};

QByteArray encodeObservation(const ObservationV1 &observation);

/// Decode, failing closed.
///
/// A schema version this build does not know is rejected rather than best-effort parsed. An
/// observation is evidence, and evidence read under guessed rules is worse than no evidence.
std::optional<ObservationV1> decodeObservation(
    const QByteArray &encoded,
    QString *error = nullptr);

/// Deterministic identity for one acquisition.
///
/// Derived from source, subject and acquisition time, so re-reporting the same reading - after an
/// adapter restart, a retry, or a replayed queue - resolves to the same contribution and Event1's
/// existing duplicate rejection makes it a durable no-op. Without this, "observed twice" and
/// "observed once, reported twice" would be indistinguishable in the biography.
///
/// The value is deliberately excluded. Two different values acquired at the same instant from the
/// same subject is a contradiction to be surfaced, not two contributions to be recorded separately.
QUuid observationMessageId(
    const QString &sourceId,
    const QString &subject,
    const QDateTime &acquiredAt);

} // namespace cybou
