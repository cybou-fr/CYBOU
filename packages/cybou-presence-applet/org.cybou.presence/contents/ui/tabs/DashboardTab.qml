// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import "../utils"

Item {
    id: dashboardTab

    required property var mind
    readonly property string title: "Dashboard"
    readonly property string icon: "view-dashboard"

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 11
        spacing: 11

        Label {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignHCenter
            text: mind.narration
            font.pixelSize: 16
            wrapMode: Text.WordWrap
            elide: Text.ElideRight
            maximumLineCount: 3
            horizontalAlignment: Text.AlignHCenter
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 11

            StatCard {
                Layout.fillWidth: true
                title: "Attention"
                value: mind.attention || "Quiet"
                icon: "eye"
            }

            StatCard {
                Layout.fillWidth: true
                title: "Contributions"
                value: mind.contributions
                icon: "star"
            }
        }

        GridLayout {
            Layout.fillWidth: true
            columns: 2
            rowSpacing: 11
            columnSpacing: 11

            StatCard { Layout.fillWidth: true; title: "Age"; value: mind.stats.ageInDays; icon: "calendar" }
            StatCard { Layout.fillWidth: true; title: "Sessions"; value: mind.stats.sessions; icon: "history" }
            StatCard { Layout.fillWidth: true; title: "Open Intentions"; value: mind.stats.openIntentions; icon: "task" }
            StatCard { Layout.fillWidth: true; title: "Settled Predictions"; value: mind.stats.settledPredictions; icon: "check" }
        }

        Item { Layout.fillHeight: true }
    }
}
