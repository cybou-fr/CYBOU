// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/protocol/CognitiveEnvelope.h"

#include <QByteArray>
#include <QList>
#include <QString>

#include <optional>

namespace cybou {

/// Versioned CBOR used only for the local Event1 IPC boundary.
///
/// This is deliberately separate from canonical Journal hashing. Changing IPC representation must
/// not rewrite or reinterpret biography hashes.
class EnvelopeCodec
{
public:
    static QByteArray encode(const CognitiveEnvelope &envelope);
    static std::optional<CognitiveEnvelope> decode(
        const QByteArray &encoded,
        QString *error = nullptr);

    static QByteArray encodeList(const QList<CognitiveEnvelope> &envelopes);
    static QList<CognitiveEnvelope> decodeList(
        const QByteArray &encoded,
        QString *error = nullptr);

    static QByteArray encodeUuidList(const QList<QUuid> &ids);
    static QList<QUuid> decodeUuidList(
        const QByteArray &encoded,
        QString *error = nullptr);
};

} // namespace cybou
