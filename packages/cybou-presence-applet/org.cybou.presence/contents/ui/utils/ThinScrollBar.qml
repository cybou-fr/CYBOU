// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import org.kde.kirigami as Kirigami

ScrollBar {
    id: root

    policy: ScrollBar.AsNeeded
    interactive: true
    hoverEnabled: true
    width: 6

    contentItem: Rectangle {
        implicitWidth: 3
        implicitHeight: 48
        radius: width / 2
        color: Kirigami.Theme.textColor
        opacity: root.pressed
            ? 0.44
            : root.hovered
                ? 0.30
                : 0.16

        Behavior on opacity {
            NumberAnimation { duration: 100 }
        }
    }

    background: Item {}
}
