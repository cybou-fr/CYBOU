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

/// Payload discriminator written into every ObservationV1 and required when reading one.
///
/// `ContributionKind::Observation` already carries unrelated payload shapes - predictord writes
/// `{subject, actual}`, presenced writes `{event, ...}` - so kind alone cannot tell an epistemic
/// observation from the rest of the biography. Without a discriminator, an epistemic projection
/// scanning Observations would have to guess from the shape of each payload, and would eventually
/// guess wrong about history that can no longer be changed.
///
/// The tag lives in the payload rather than in CognitiveEnvelope deliberately. An envelope field
/// would be the more general answer for future payload families, but it changes the canonical hash
/// and needs a schema migration; this closes the ambiguity now, before any adapter writes, without
/// touching contributions that already exist. Existing Observation payloads simply lack the tag and
/// are therefore correctly not ObservationV1.
inline constexpr char kObservationPayloadType[] = "cybou.observation.v1";

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

    /// Whether this observation speaks for the instant `at`.
    ///
    /// Bounded at both ends. An observation says nothing about a time before it was acquired, and
    /// checking only the upper bound made a reading taken at 15:00 report as fresh at 10:00 the
    /// same day. Distributed nodes will need an explicit clock-skew tolerance; that is a separate
    /// decision and must not arrive by way of a missing check.
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

/// Deterministic identity for one acquisition of one value.
///
/// Re-reporting the same reading - after an adapter restart, a retry, or a replayed queue - resolves
/// to the same contribution, so Event1's existing duplicate rejection makes it a durable no-op.
/// Without this, "observed twice" and "observed once, reported twice" would be indistinguishable.
///
/// The value participates, and must. An earlier version excluded it on the reasoning that two
/// different values for one subject at one instant should become a contradiction for the projection
/// to reconcile. They could not: both mapped to one messageId, Event1 rejected the second as a
/// duplicate, and the contradicting evidence never reached the Journal at all. Including the value
/// keeps a repeat idempotent while letting a disagreement arrive as the second contribution it is.
///
/// The fields are hashed as a canonical CBOR array rather than joined with a separator. A separator
/// only works if it cannot occur inside the fields, nothing here forbids that, and
/// ("a", "b<sep>c") against ("a<sep>b", "c") would otherwise be one identity.
QUuid observationMessageId(
    const QString &sourceId,
    const QString &subject,
    const QDateTime &acquiredAt,
    const QCborValue &value);

} // namespace cybou
