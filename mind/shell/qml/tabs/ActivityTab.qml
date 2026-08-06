// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import Cybou.Presence 1.0

MindTab {
    id: activityTab
    title: "Activity"
    icon: "history"

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
            text: "Recent Moments"
            font.pixelSize: 18
            font.bold: true
        }

        // Stats
        StatCard {
            title: "Total Contributions"
            value: presence.contributions
            icon: "database"
            Layout.alignment: Qt.AlignHCenter
        }

        // Activity list
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: activity
            delegate: ItemDelegate {
                text: "[%1] %2: %3".arg(modelData.when.toLocaleTime().toString("HH:mm"))
                                      .arg(modelData.organ)
                                      .arg(modelData.kind)
                onClicked: console.log("Activity clicked:", modelData)
            }
        }
    }

    property QVariantList activity: presence.activity(20)

    function updateData() {
        activity = presence.activity(20)
    }

    Component.onCompleted: updateData()
}
