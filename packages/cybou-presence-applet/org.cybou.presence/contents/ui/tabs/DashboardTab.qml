// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.cybou.presence 1.0

MindTab {
    id: dashboardTab
    title: "Dashboard"
    icon: "view-dashboard"

    // Update data when mind changes
    Connections {
        target: mind
        function onChanged() { }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 11
        spacing: 11

        // Narration
        Label {
            Layout.alignment: Qt.AlignHCenter
            text: mind.narration
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
                value: mind.attention
                icon: "eye"
            }

            StatCard {
                title: "Contributions"
                value: mind.contributions
                icon: "star"
            }
        }

        // Additional stats grid
        GridLayout {
            Layout.fillWidth: true
            columns: 2
            spacing: 11

            StatCard { title: "Age"; value: mind.stats.ageInDays; icon: "calendar" }
            StatCard { title: "Sessions"; value: mind.stats.sessions; icon: "history" }
            StatCard { title: "Open Intentions"; value: mind.stats.openIntentions; icon: "task" }
            StatCard { title: "Settled Predictions"; value: mind.stats.settledPredictions; icon: "check" }
        }
    }
}
