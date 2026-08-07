// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import "../utils"

Item {
    id: intentionsTab

    required property var mind
    readonly property string title: "Intentions"
    readonly property string icon: "task-complete"

    property var intentionModel: []

    function refreshIntentions() {
        intentionModel = mind.detailedObligations()
    }

    Component.onCompleted: refreshIntentions()

    Connections {
        target: mind
        function onChanged() { intentionsTab.refreshIntentions() }
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

        RowLayout {
            Layout.fillWidth: true
            spacing: 11

            StatCard {
                Layout.fillWidth: true
                title: "Total"
                value: intentionsTab.intentionModel.length
                icon: "list"
            }

            StatCard {
                Layout.fillWidth: true
                title: "Oldest"
                value: mind.stats.oldestObligationDays
                icon: "calendar"
            }
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: intentionsTab.intentionModel

            delegate: ItemDelegate {
                required property var modelData
                width: ListView.view.width
                text: modelData.description
                Accessible.description: modelData.trigger
            }
        }
    }
}
