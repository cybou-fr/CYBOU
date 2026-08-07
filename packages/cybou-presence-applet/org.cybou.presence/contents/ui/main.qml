// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Layouts 1.15
import org.kde.plasma.plasmoid 2.0
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.extras as PlasmaExtras
import org.kde.kirigami as Kirigami
import org.cybou.presence 1.0

PlasmoidItem {
    id: root
    property bool ready: false

    Presence {
        id: presenceBackend
        Component.onCompleted: root.ready = wake()
    }

    Plasmoid.status: root.ready ? PlasmaCore.Types.ActiveStatus
                                : PlasmaCore.Types.PassiveStatus

    // Horizontal panel -> compact icon. Dedicated vertical panel -> embedded full Mind UI.
    // Plasma 6: preferredRepresentation belongs to PlasmoidItem itself.
    preferredRepresentation:
        plasmoid.formFactor === PlasmaCore.Types.Vertical
            ? root.fullRepresentation
            : root.compactRepresentation

    toolTipMainText: i18n("Cybou")
    toolTipSubText: root.ready ? presenceBackend.narration : i18n("Not awake.")

    compactRepresentation: Item {
        Layout.minimumWidth: Kirigami.Units.iconSizes.small
        Layout.minimumHeight: Kirigami.Units.iconSizes.small

        Kirigami.Icon {
            anchors.centerIn: parent
            width: Math.min(parent.width, parent.height)
            height: width
            source: "cybou"
            opacity: root.ready ? 1.0 : 0.4
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

    PlasmaExtras.PlaceholderMessage {
        anchors.centerIn: parent
        width: parent.width - Kirigami.Units.gridUnit * 4
        visible: !root.ready
        iconName: "dialog-cancel"
        text: i18n("Not awake")
        explanation: i18n("The journal could not be opened, so there is nothing to show.")
    }
}
