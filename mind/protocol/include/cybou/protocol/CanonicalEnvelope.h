// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/protocol/CognitiveEnvelope.h"

#include <QByteArray>

namespace cybou {

/// Stable, length-prefixed, big-endian representation of every semantic envelope field.
QByteArray canonicalEnvelopeV2(const CognitiveEnvelope &envelope);

/// Stable Journal row representation. It binds sequence, previous hash, and envelope bytes.
/// Everything about a contribution that erasure never removes.
///
/// This is the v3 hash's metadata half, and its membership is not a matter of taste: it is exactly
/// the set ADR-0028 declares non-erasable, because what survives erasure is precisely what has to
/// stay verifiable afterwards. Leaving these outside the chain would let a contribution's author,
/// causality or privacy be rewritten without disturbing a hash - which would trade the provenance
/// binding Event1 enforces for the ability to forget, when the design needs both.
///
/// The payload is deliberately absent. It is committed to separately, so that destroying a key can
/// make the content unverifiable without making the record unverifiable.
QByteArray canonicalNonErasableEnvelopeV3(const CognitiveEnvelope &envelope);

QByteArray canonicalJournalRowV2(
    quint64 sequence, const QByteArray &previousHash, const CognitiveEnvelope &envelope);

} // namespace cybou
