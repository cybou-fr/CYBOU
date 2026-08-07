// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import "../utils"

Item {
    id: activityTab

    required property var mind
    readonly property string title: "Activity"
    readonly property string icon: "history"

    property var activityModel: []

    function refreshActivity() {
        activityModel = mind.activity(20)
    }

    Component.onCompleted: refreshActivity()

    Connections {
        target: mind
        function onChanged() { activityTab.refreshActivity() }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 11
        spacing: 11

        Label {
            Layout.alignment: Qt.AlignHCenter
            text: "Recent Moments"
            font.pixelSize: 18
            font.bold: true
        }

        StatCard {
            title: "Total Contributions"
            value: mind.contributions
            icon: "database"
            Layout.alignment: Qt.AlignHCenter
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: activityTab.activityModel

            delegate: ItemDelegate {
                required property var modelData
                width: ListView.view.width
                text: "[%1] %2: %3"
                    .arg(Qt.formatTime(modelData.when, "HH:mm"))
                    .arg(modelData.organ)
                    .arg(modelData.kind)
            }
        }
    }
}
