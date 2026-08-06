// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import Cybou.Presence 1.0

MindTab {
    id: intentionsTab
    title: "Intentions"
    icon: "task-complete"

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
            text: "Open Obligations"
            font.pixelSize: 18
            font.bold: true
        }

        // Stats
        RowLayout {
            Layout.fillWidth: true
            spacing: 11

            StatCard {
                title: "Total"
                value: detailedObligations.length
                icon: "list"
            }

            StatCard {
                title: "Oldest"
                value: oldestObligationDays
                icon: "calendar"
            }
        }

        // List of intentions
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: detailedObligations
            delegate: ItemDelegate {
                text: modelData.description
                onClicked: console.log("Intention clicked:", modelData.description)
            }
        }
    }

    property QVariantList detailedObligations: presence.detailedObligations()
    property int oldestObligationDays: presence.stats.oldestObligationDays

    function updateData() {
        detailedObligations = presence.detailedObligations()
        oldestObligationDays = presence.stats.oldestObligationDays
    }

    Component.onCompleted: updateData()
}
