// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/protocol/Sensitivity.h"

namespace cybou {

QString sensitivityToString(SensitivityClass sensitivity)
{
    switch (sensitivity) {
    case SensitivityClass::Ordinary:
        return QStringLiteral("ordinary");
    case SensitivityClass::Personal:
        return QStringLiteral("personal");
    case SensitivityClass::Sensitive:
        return QStringLiteral("sensitive");
    case SensitivityClass::Secret:
        return QStringLiteral("secret");
    case SensitivityClass::Credential:
        return QStringLiteral("credential");
    }
    return QStringLiteral("unknown");
}

SensitivityClass derivedSensitivity(
    SensitivityClass declared,
    const QList<SensitivityClass> &evidence)
{
    SensitivityClass strongest = declared;
    for (const SensitivityClass source : evidence) {
        strongest = mostSensitive(strongest, source);
    }
    return strongest;
}

} // namespace cybou
