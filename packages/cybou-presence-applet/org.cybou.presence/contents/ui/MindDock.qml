// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import "tabs"

Item {
    id: root

    required property var mind

    property int dockWidth: 460
    property int dockHeight: 640
    property int tabHeight: 48

    implicitWidth: dockWidth
    implicitHeight: dockHeight
    clip: true

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        MindTabBar {
            id: tabBar
            Layout.fillWidth: true
            Layout.preferredHeight: root.tabHeight
        }

        StackLayout {
            id: tabStack
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: tabBar.currentIndex

            DashboardTab { Layout.fillWidth: true; Layout.fillHeight: true; mind: root.mind }
            IdentityTab { Layout.fillWidth: true; Layout.fillHeight: true; mind: root.mind }
            IntentionsTab { Layout.fillWidth: true; Layout.fillHeight: true; mind: root.mind }
            ActivityTab { Layout.fillWidth: true; Layout.fillHeight: true; mind: root.mind }
            SelfTab { Layout.fillWidth: true; Layout.fillHeight: true; mind: root.mind }
            PredictorTab { Layout.fillWidth: true; Layout.fillHeight: true; mind: root.mind }
            WorkspaceTab { Layout.fillWidth: true; Layout.fillHeight: true; mind: root.mind }
        }
    }
}
