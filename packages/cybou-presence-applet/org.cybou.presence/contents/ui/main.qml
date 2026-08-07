// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.kde.plasma.plasmoid 2.0
import org.kde.plasma.core as PlasmaCore
import org.kde.kirigami as Kirigami
import org.cybou.presence 1.0

PlasmoidItem {
    id: root

    Presence {
        id: presenceBackend
        Component.onCompleted: wake()
    }

    Plasmoid.status: presenceBackend.awake
        ? PlasmaCore.Types.ActiveStatus
        : PlasmaCore.Types.PassiveStatus

    // Horizontal panel -> compact icon. Dedicated vertical panel -> embedded full Mind UI.
    preferredRepresentation:
        plasmoid.formFactor === PlasmaCore.Types.Vertical
            ? root.fullRepresentation
            : root.compactRepresentation

    toolTipMainText: i18n("Cybou")
    toolTipSubText: presenceBackend.awake
        ? presenceBackend.narration
        : i18n("Mind services are unavailable.")

    compactRepresentation: Item {
        Layout.minimumWidth: Kirigami.Units.iconSizes.small
        Layout.minimumHeight: Kirigami.Units.iconSizes.small

        Kirigami.Icon {
            anchors.centerIn: parent
            width: Math.min(parent.width, parent.height)
            height: width
            source: "cybou"
            opacity: presenceBackend.awake ? 1.0 : 0.45
        }

        Rectangle {
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            width: 7
            height: 7
            radius: width / 2
            color: presenceBackend.awake
                ? Kirigami.Theme.highlightColor
                : Kirigami.Theme.disabledTextColor
        }

        MouseArea {
            anchors.fill: parent
            onClicked: root.expanded = !root.expanded
        }
    }

    fullRepresentation: Component {
        MindDock {
            Layout.minimumWidth: 360
            Layout.preferredWidth: 420
            Layout.minimumHeight: 480
            Layout.fillWidth: true
            Layout.fillHeight: true
            mind: presenceBackend
        }
    }
}
