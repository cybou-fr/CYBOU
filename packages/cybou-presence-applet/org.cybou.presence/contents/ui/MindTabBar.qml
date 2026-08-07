// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

RowLayout {
    id: root
    spacing: 0

    property int currentIndex: 0
    signal tabSelected(int index)

    // Tab buttons
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
            id: tabButton
            text: modelData.name
            icon.name: modelData.icon

            ToolTip.text: modelData.tooltip
            ToolTip.visible: hovered
            ToolTip.delay: 500

            checkable: true
            checked: index === root.currentIndex
            Layout.fillWidth: true
            Layout.preferredHeight: parent.Layout.preferredHeight

            onClicked: {
                root.currentIndex = index
                root.tabSelected(index)
            }
        }
    }
}
