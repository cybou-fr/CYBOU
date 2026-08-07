// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/fabric/OrganBus.h"

#include <QObject>
#include <QString>

namespace cybou {

class ServiceHost
{
public:
    static bool publish(
        QObject *object,
        const BusEndpoint &endpoint,
        QString *error = nullptr);
};

} // namespace cybou
