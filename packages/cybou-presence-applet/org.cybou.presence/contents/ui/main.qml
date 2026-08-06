// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Layouts 1.15
import org.kde.plasma.plasmoid 2.0
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.components as PlasmaComponents
import org.kde.plasma.extras as PlasmaExtras
import org.kde.kirigami as Kirigami
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

    toolTipMainText: i18n("Cybou")
    toolTipSubText: root.ready ? mind.narration : i18n("Not awake.")

    compactRepresentation: Item {
        Layout.minimumWidth: Kirigami.Units.iconSizes.small
        Layout.minimumHeight: Kirigami.Units.iconSizes.small

        Kirigami.Icon {
            id: mark
            anchors.centerIn: parent
            width: Math.min(parent.width, parent.height)
            height: width
            source: "cybou"
            // Dimmed while asleep: the icon must never look alive when nothing is behind it.
            opacity: root.ready ? 1.0 : 0.4

            // A slow breath, driven by nothing but time. It signals "running", not "thinking" -
            // an animation tied to activity the system is not doing would be a lie.
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

    // Main dock representation - MindDock with organ tabs
    // Using Component to load MindDock.qml from the same directory
    fullRepresentation: Component {
        MindDock {
            width: 460
            height: childrenRect.height
        }
    }

    // Nothing behind the panel: say so plainly instead of showing an empty frame.
    PlasmaExtras.PlaceholderMessage {
        anchors.centerIn: parent
        width: parent.width - Kirigami.Units.gridUnit * 4
        visible: !root.ready
        iconName: "dialog-cancel"
        text: i18n("Not awake")
        explanation: i18n("The journal could not be opened, so there is nothing to show.")
    }
}
