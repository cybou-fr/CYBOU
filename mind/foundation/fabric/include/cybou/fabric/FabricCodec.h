// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include <QByteArray>
#include <QString>
#include <QVariant>

namespace cybou {

class FabricCodec
{
public:
    static QByteArray encode(const QVariant &value);

    static QVariant decode(
        const QByteArray &encoded,
        QString *error = nullptr);

    static QByteArray encodeMap(const QVariantMap &map)
    {
        return encode(map);
    }

    static QByteArray encodeList(const QVariantList &list)
    {
        return encode(list);
    }

    static QVariantMap decodeMap(
        const QByteArray &encoded,
        QString *error = nullptr);

    static QVariantList decodeList(
        const QByteArray &encoded,
        QString *error = nullptr);
};

} // namespace cybou
