// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.cybou.presence 1.0

MindTab {
    id: selfTab
    title: "Self"
    icon: "user"

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

            StatCard { title: "Age (days)"; value: mind.stats.ageInDays; icon: "calendar" }
            StatCard { title: "Sessions"; value: mind.stats.sessions; icon: "history" }
            StatCard { title: "Open Intentions"; value: mind.stats.openIntentions; icon: "task" }
            StatCard { title: "Settled Predictions"; value: mind.stats.settledPredictions; icon: "check" }
            StatCard { title: "Contributions"; value: mind.stats.contributions; icon: "database" }
            StatCard { title: "Journal Intact"; value: mind.stats.journalIntact ? "Yes" : "No"; icon: "shield-check" }
        }

        // First broken at
        StatCard {
            title: "First Broken At"
            value: mind.stats.firstBrokenAt ? mind.stats.firstBrokenAt.toLocaleDateTime().toString("yyyy-MM-dd HH:mm") : "Never"
            icon: "alert"
            Layout.alignment: Qt.AlignHCenter
        }
    }
}
