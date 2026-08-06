// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import Cybou.Presence 1.0

MindTab {
    id: dashboardTab
    title: "Dashboard"
    icon: "view-dashboard"

    Presence {
        id: presence
        onNarrationChanged: updateData()
        onStatsChanged: updateData()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 11
        spacing: 11

        // Narration
        Label {
            Layout.alignment: Qt.AlignHCenter
            text: presence.narration
            font.pixelSize: 16
            wrapMode: Text.WordWrap
            elide: Text.ElideRight
            maximumLineCount: 3
        }

        // Stats row
        RowLayout {
            Layout.fillWidth: true
            spacing: 11

            StatCard {
                title: "Attention"
                value: presence.stats.attention
                icon: "eye"
            }

            StatCard {
                title: "Contributions"
                value: presence.stats.contributions
                icon: "star"
            }
        }

        // Additional stats grid
        GridLayout {
            Layout.fillWidth: true
            columns: 2
            spacing: 11

            StatCard { title: "Age"; value: presence.stats.ageInDays; icon: "calendar" }
            StatCard { title: "Sessions"; value: presence.stats.sessions; icon: "history" }
            StatCard { title: "Open Intentions"; value: presence.stats.openIntentions; icon: "task" }
            StatCard { title: "Settled Predictions"; value: presence.stats.settledPredictions; icon: "check" }
        }
    }

    function updateData() {
        // Data is automatically updated through Presence signals
    }
}
