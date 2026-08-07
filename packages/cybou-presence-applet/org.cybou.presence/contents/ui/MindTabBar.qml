// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

ColumnLayout {
    id: root
    spacing: 2
    property int currentIndex: 0
    signal tabSelected(int index)

    Repeater {
        model: [
            { "name": "Dashboard", "icon": "view-dashboard", "tooltip": "Mind Dashboard" },
            { "name": "Identity", "icon": "user-identity", "tooltip": "Identity Organ" },
            { "name": "Intentions", "icon": "task-complete", "tooltip": "Intentions Organ" },
            { "name": "Activity", "icon": "history", "tooltip": "Activity Journal" },
            { "name": "Self", "icon": "user", "tooltip": "Self Assessment" },
            { "name": "Predictor", "icon": "predictive-text", "tooltip": "Predictor Organ" },
            { "name": "Workspace", "icon": "folder-workspace", "tooltip": "Workspace Organ" }
        ]

        delegate: ToolButton {
            required property int index
            required property var modelData
            Layout.fillWidth: true
            Layout.preferredHeight: 48
            text: modelData.name
            icon.name: modelData.icon
            ToolTip.text: modelData.tooltip
            ToolTip.visible: hovered
            ToolTip.delay: 500
            checkable: true
            checked: index === root.currentIndex
            onClicked: {
                root.currentIndex = index
                root.tabSelected(index)
            }
        }
    }

    Item { Layout.fillHeight: true }
}
