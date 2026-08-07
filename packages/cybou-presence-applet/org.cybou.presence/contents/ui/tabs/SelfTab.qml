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
        anchors.margins: 12
        spacing: 9

        InfoCard {
            Layout.fillWidth: true
            title: i18n("Self assessment")
            text: mind.narration || i18n("No self-assessment is available yet.")
            icon: "user"
            emphasized: true
        }

        GridLayout {
            Layout.fillWidth: true
            columns: 2
            rowSpacing: 9
            columnSpacing: 9

            StatCard {
                Layout.fillWidth: true
                title: i18n("Age")
                value: i18n("%1 d", mind.stats.ageInDays || 0)
                icon: "calendar"
            }

            StatCard {
                Layout.fillWidth: true
                title: i18n("Sessions")
                value: mind.stats.sessions
                icon: "history"
            }

            StatCard {
                Layout.fillWidth: true
                title: i18n("Intentions")
                value: mind.stats.openIntentions
                icon: "task-complete"
            }

            StatCard {
                Layout.fillWidth: true
                title: i18n("Predictions")
                value: mind.stats.settledPredictions
                icon: "checkmark"
            }
        }

        InfoCard {
            Layout.fillWidth: true
            title: i18n("Journal integrity")
            text: mind.stats.journalIntact
                ? i18n("The durable memory chain verifies successfully.")
                : i18n("Memory verification failed at row %1.", mind.stats.firstBrokenAt)
            icon: mind.stats.journalIntact ? "security-high" : "dialog-warning"
        }

        Button {
            Layout.alignment: Qt.AlignHCenter
            text: i18n("Reflect now")
            icon.name: "view-refresh"
            onClicked: mind.reflect()
        }

        Item {
            Layout.fillHeight: true
        }
    }
}
