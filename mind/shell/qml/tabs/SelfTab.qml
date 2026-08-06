// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import Cybou.Presence 1.0

MindTab {
    id: selfTab
    title: "Self"
    icon: "user"

    Presence {
        id: presence
        onChanged: updateData()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 11
        spacing: 11

        Label {
            Layout.alignment: Qt.AlignHCenter
            text: "Self Assessment"
            font.pixelSize: 18
            font.bold: true
        }

        // Self stats grid
        GridLayout {
            Layout.fillWidth: true
            columns: 2
            spacing: 11

            StatCard { title: "Age (days)"; value: selfStats.ageInDays; icon: "calendar" }
            StatCard { title: "Sessions"; value: selfStats.sessions; icon: "history" }
            StatCard { title: "Open Intentions"; value: selfStats.openIntentions; icon: "task" }
            StatCard { title: "Settled Predictions"; value: selfStats.settledPredictions; icon: "check" }
            StatCard { title: "Contributions"; value: selfStats.contributions; icon: "database" }
            StatCard { title: "Journal Intact"; value: selfStats.journalIntact ? "Yes" : "No"; icon: "shield-check" }
        }

        // First broken at
        StatCard {
            title: "First Broken At"
            value: selfStats.firstBrokenAt ? selfStats.firstBrokenAt.toLocaleDateTime().toString("yyyy-MM-dd HH:mm") : "Never"
            icon: "alert"
            Layout.alignment: Qt.AlignHCenter
        }
    }

    property QVariantMap selfStats: presence.stats()

    function updateData() {
        selfStats = presence.stats()
    }

    Component.onCompleted: updateData()
}
