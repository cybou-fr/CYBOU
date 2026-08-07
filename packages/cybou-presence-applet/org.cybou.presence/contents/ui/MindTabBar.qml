// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.kde.kirigami as Kirigami

ColumnLayout {
    id: root

    property int currentIndex: 0
    property bool navigationEnabled: true
    property bool runtimeAvailable: true

    readonly property var entries: [
        { "name": i18n("Dashboard"), "icon": "view-dashboard", "tooltip": i18n("Mind Dashboard") },
        { "name": i18n("Identity"), "icon": "user-identity", "tooltip": i18n("Identity") },
        { "name": i18n("Intentions"), "icon": "task-complete", "tooltip": i18n("Intentions") },
        { "name": i18n("Activity"), "icon": "history", "tooltip": i18n("Activity") },
        { "name": i18n("Self"), "icon": "user", "tooltip": i18n("Self assessment") },
        { "name": i18n("Predictor"), "icon": "predictive-text", "tooltip": i18n("Predictor") },
        { "name": i18n("Workspace"), "icon": "folder-workspace", "tooltip": i18n("Workspace") }
    ]

    readonly property string currentTitle:
        entries[currentIndex] ? entries[currentIndex].name : ""
    readonly property string currentIcon:
        entries[currentIndex] ? entries[currentIndex].icon : "cybou"

    spacing: 4

    Item {
        Layout.fillWidth: true
        Layout.preferredHeight: 52

        Kirigami.Icon {
            anchors.centerIn: parent
            width: 30
            height: 30
            source: "cybou"
        }
    }

    Repeater {
        model: root.entries

        delegate: ToolButton {
            id: button

            required property int index
            required property var modelData

            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 48
            Layout.preferredHeight: 44

            enabled: root.navigationEnabled
            checkable: true
            checked: index === root.currentIndex
            text: modelData.name
            display: AbstractButton.IconOnly
            icon.name: modelData.icon
            icon.width: 21
            icon.height: 21

            ToolTip.text: modelData.tooltip
            ToolTip.visible: hovered
            ToolTip.delay: 350

            background: Rectangle {
                radius: 10
                color: Kirigami.Theme.highlightColor
                opacity: button.checked ? 0.20 : button.hovered ? 0.08 : 0.0
            }

            Rectangle {
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                width: 3
                height: 24
                radius: 2
                visible: button.checked
                color: Kirigami.Theme.highlightColor
            }

            onClicked: root.currentIndex = index
        }
    }

    Item {
        Layout.fillHeight: true
    }

    Rectangle {
        Layout.alignment: Qt.AlignHCenter
        Layout.preferredWidth: 9
        Layout.preferredHeight: 9
        radius: width / 2
        color: root.runtimeAvailable
            ? Kirigami.Theme.highlightColor
            : Kirigami.Theme.disabledTextColor

        ToolTip.text: root.runtimeAvailable
            ? i18n("Mind connected")
            : i18n("Mind unavailable")
        ToolTip.visible: statusHover.containsMouse

        MouseArea {
            id: statusHover
            anchors.fill: parent
            hoverEnabled: true
        }
    }

    Item {
        Layout.preferredHeight: 4
    }
}
