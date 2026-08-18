// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

// Temporary differential oracle. Remove with the C++ implementation after the Rust cutover gate.
#include "cybou/fabric/FabricCodec.h"

#include <QTextStream>
#include <QVariantList>
#include <QVariantMap>

int main()
{
    const QVariantMap capability{
        {QStringLiteral("capability"), QStringLiteral("mind.identity.read")},
        {QStringLiteral("state"), QStringLiteral("available")},
    };
    const QVariantList states{
        QStringLiteral("available"),
        QStringLiteral("unknown"),
        QStringLiteral("unavailable"),
    };

    QTextStream out(stdout);
    out << "map=" << cybou::FabricCodec::encodeMap(capability).toHex() << '\n';
    out << "list=" << cybou::FabricCodec::encodeList(states).toHex() << '\n';
    return 0;
}
