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
        { "name": i18n("Activity"), "icon": "view-history", "tooltip": i18n("Activity") },
        { "name": i18n("Self"), "icon": "user-identity", "tooltip": i18n("Self assessment") },
        { "name": i18n("Predictor"), "icon": "edit-find", "tooltip": i18n("Predictor") },
        { "name": i18n("Workspace"), "icon": "folder", "tooltip": i18n("Workspace") }
    ]

    readonly property string currentTitle:
        entries[currentIndex] ? entries[currentIndex].name : ""
    readonly property string currentIcon:
        entries[currentIndex] ? entries[currentIndex].icon : "cybou"

    spacing: 2

    function select(index, moveFocus) {
        const bounded = Math.max(0, Math.min(entries.length - 1, index))
        currentIndex = bounded

        if (moveFocus) {
            const item = tabRepeater.itemAt(bounded)
            if (item)
                item.forceActiveFocus()
        }
    }

    function move(delta) {
        select((currentIndex + delta + entries.length) % entries.length, true)
    }

    Item {
        Layout.fillWidth: true
        Layout.preferredHeight: 48

        Kirigami.Icon {
            anchors.centerIn: parent
            width: 27
            height: 27
            source: "cybou"
            opacity: 0.92
        }
    }

    Repeater {
        id: tabRepeater
        model: root.entries

        delegate: ToolButton {
            id: button

            required property int index
            required property var modelData

            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 48
            Layout.preferredHeight: 42

            enabled: root.navigationEnabled
            activeFocusOnTab: true
            checkable: true
            checked: index === root.currentIndex

            text: modelData.name
            display: AbstractButton.IconOnly
            icon.name: modelData.icon
            icon.width: 20
            icon.height: 20
            icon.color: button.checked
                ? Kirigami.Theme.highlightColor
                : Kirigami.Theme.textColor

            Accessible.name: modelData.name
            Accessible.description: modelData.tooltip

            ToolTip.text: modelData.tooltip
            ToolTip.visible: hovered || activeFocus
            ToolTip.delay: activeFocus ? 0 : 300

            background: Rectangle {
                anchors.centerIn: parent
                width: 34
                height: 34
                radius: 9

                color: Kirigami.Theme.highlightColor
                opacity: button.checked
                    ? 0.07
                    : button.hovered
                        ? 0.06
                        : 0.0

                border.width: button.activeFocus ? 1 : 0
                border.color: Kirigami.Theme.focusColor

                Behavior on opacity {
                    NumberAnimation { duration: 110 }
                }
            }

            Rectangle {
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                width: 2
                height: 26
                radius: 1
                visible: button.checked
                color: Kirigami.Theme.highlightColor
            }

            onClicked: root.select(index, false)

            Keys.onPressed: function(event) {
                if (event.key === Qt.Key_Up) {
                    root.move(-1)
                    event.accepted = true
                } else if (event.key === Qt.Key_Down) {
                    root.move(1)
                    event.accepted = true
                } else if (event.key === Qt.Key_Home) {
                    root.select(0, true)
                    event.accepted = true
                } else if (event.key === Qt.Key_End) {
                    root.select(root.entries.length - 1, true)
                    event.accepted = true
                }
            }
        }
    }

    Item {
        Layout.fillHeight: true
    }

    Item {
        Layout.alignment: Qt.AlignHCenter
        Layout.preferredWidth: 24
        Layout.preferredHeight: 24

        Rectangle {
            anchors.centerIn: parent
            width: 8
            height: 8
            radius: width / 2
            color: root.runtimeAvailable
                ? Kirigami.Theme.highlightColor
                : Kirigami.Theme.disabledTextColor

            Behavior on color {
                ColorAnimation { duration: 140 }
            }
        }

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
