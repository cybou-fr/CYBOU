// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/protocol/CognitiveEnvelope.h"

#include <QByteArray>

namespace cybou {

/// Stable, length-prefixed, big-endian representation of every semantic envelope field.
QByteArray canonicalEnvelopeV2(const CognitiveEnvelope &envelope);

/// Stable Journal row representation. It binds sequence, previous hash, and envelope bytes.
QByteArray canonicalJournalRowV2(
    quint64 sequence, const QByteArray &previousHash, const CognitiveEnvelope &envelope);

} // namespace cybou
