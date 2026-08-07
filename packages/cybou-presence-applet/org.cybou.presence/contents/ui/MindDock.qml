// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import "tabs"

Item {
    id: root
    required property var mind
    implicitWidth: 420
    implicitHeight: 720
    clip: true

    RowLayout {
        anchors.fill: parent
        spacing: 0

        MindTabBar {
            id: tabBar
            Layout.preferredWidth: 132
            Layout.minimumWidth: 116
            Layout.fillHeight: true
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.fillHeight: true
            color: palette.mid
            opacity: 0.45
        }

        StackLayout {
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
