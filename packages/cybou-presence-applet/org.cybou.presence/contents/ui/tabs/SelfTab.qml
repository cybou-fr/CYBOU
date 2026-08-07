// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import "../utils"

Item {
    id: selfTab

    required property var mind
    readonly property string title: "Self"
    readonly property string icon: "user"

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

        GridLayout {
            Layout.fillWidth: true
            columns: 2
            rowSpacing: 11
            columnSpacing: 11

            StatCard { Layout.fillWidth: true; title: "Age (days)"; value: mind.stats.ageInDays; icon: "calendar" }
            StatCard { Layout.fillWidth: true; title: "Sessions"; value: mind.stats.sessions; icon: "history" }
            StatCard { Layout.fillWidth: true; title: "Open Intentions"; value: mind.stats.openIntentions; icon: "task" }
            StatCard { Layout.fillWidth: true; title: "Settled Predictions"; value: mind.stats.settledPredictions; icon: "check" }
            StatCard { Layout.fillWidth: true; title: "Contributions"; value: mind.stats.contributions; icon: "database" }
            StatCard { Layout.fillWidth: true; title: "Journal Intact"; value: mind.stats.journalIntact ? "Yes" : "No"; icon: "shield-check" }
        }

        StatCard {
            title: "First Broken Row"
            value: mind.stats.firstBrokenAt > 0 ? "#" + mind.stats.firstBrokenAt : "Never"
            icon: "alert"
            Layout.alignment: Qt.AlignHCenter
        }

        Item { Layout.fillHeight: true }
    }
}
