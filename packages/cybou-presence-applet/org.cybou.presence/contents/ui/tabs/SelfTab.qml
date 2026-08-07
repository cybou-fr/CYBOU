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
    readonly property string icon: "user-identity"

    Flickable {
        id: scroll

        anchors.fill: parent
        clip: true
        contentWidth: width
        contentHeight: contentColumn.implicitHeight + 24
        boundsBehavior: Flickable.StopAtBounds

        ScrollBar.vertical: ThinScrollBar {}

        ColumnLayout {
            id: contentColumn

            x: 12
            y: 12
            width: Math.max(0, scroll.width - 28)
            spacing: 8

            InfoCard {
                Layout.fillWidth: true
                title: i18n("Self assessment")
                text: mind.narration || i18n("No self-assessment is available yet.")
                icon: "user-identity"
                emphasized: true
            }

            GridLayout {
                Layout.fillWidth: true
                columns: width >= 280 ? 2 : 1
                rowSpacing: 8
                columnSpacing: 8

                StatCard {
                    Layout.fillWidth: true
                    title: i18n("Age")
                    value: i18n("%1 d", mind.stats.ageInDays || 0)
                }

                StatCard {
                    Layout.fillWidth: true
                    title: i18n("Sessions")
                    value: mind.stats.sessions
                }

                StatCard {
                    Layout.fillWidth: true
                    title: i18n("Intentions")
                    value: mind.stats.openIntentions
                }

                StatCard {
                    Layout.fillWidth: true
                    title: i18n("Predictions")
                    value: mind.stats.settledPredictions
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
                activeFocusOnTab: true
                onClicked: mind.reflect()
            }
        }
    }
}
