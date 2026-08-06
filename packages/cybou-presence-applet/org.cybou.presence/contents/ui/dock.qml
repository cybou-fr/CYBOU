// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import org.kde.plasma.plasmoid 2.0
import org.cybou.presence 1.0

PlasmoidItem {
    id: root

    property bool ready: false

    Presence {
        id: mind
        Component.onCompleted: root.ready = wake()
    }

    // Being looked at is an event worth remembering, and it refreshes what the panel reads.
    onExpandedChanged: if (expanded && root.ready) mind.reflect()

    // The mind is not busy every second; a slow tick is enough and costs nothing noticeable.
    Timer {
        interval: 20000
        running: root.expanded && root.ready
        repeat: true
        onTriggered: mind.changed()
    }

    Plasmoid.status: root.ready ? PlasmaCore.Types.ActiveStatus
                                : PlasmaCore.Types.PassiveStatus

    // Main dock representation
    fullRepresentation: MindDock {
        width: 460
        height: childrenRect.height
    }

    // Compact representation (for panel)
    compactRepresentation: Item {
        Layout.minimumWidth: Kirigami.Units.iconSizes.small
        Layout.minimumHeight: Kirigami.Units.iconSizes.small

        Kirigami.Icon {
            id: mark
            anchors.centerIn: parent
            width: Math.min(parent.width, parent.height)
            height: width
            source: "cybou"
            opacity: root.ready ? 1.0 : 0.4

            SequentialAnimation on scale {
                running: root.ready
                loops: Animation.Infinite
                NumberAnimation { to: 1.06; duration: 2600; easing.type: Easing.InOutSine }
                NumberAnimation { to: 1.00; duration: 2600; easing.type: Easing.InOutSine }
            }
        }

        MouseArea {
            anchors.fill: parent
            onClicked: root.expanded = !root.expanded
        }
    }
}
