// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/protocol/Health.h"

#include <QList>
#include <QMap>
#include <QStringList>

namespace cybou {

struct CapabilityDefinition {
    QString capabilityId;
    QStringList dependencies;
    bool required{false};
    QString unavailableImpact;
};

class HealthPolicy
{
public:
    static QList<CapabilityDefinition> definitions();
    static QStringList componentIds();
    static CapabilitySnapshot evaluate(
        const QMap<QString, ComponentHealthRecord> &observations,
        const QDateTime &observedAt = QDateTime::currentDateTimeUtc());
};

} // namespace cybou
