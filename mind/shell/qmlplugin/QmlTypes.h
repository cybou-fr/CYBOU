// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// Exposes Presence to QML without the mind knowing anything about QML.
//
// QML_FOREIGN declares the binding here rather than putting QML macros in Presence itself, so
// the organs stay linkable and testable in a headless build with no declarative module at all.
// qt_add_qml_module generates the plugin class; nothing here should try to be one.

#pragma once

#include "cybou/presence/Presence.h"

#include <QQmlEngine>

struct PresenceForeign {
    Q_GADGET
    QML_FOREIGN(cybou::Presence)
    QML_NAMED_ELEMENT(Presence)
};
