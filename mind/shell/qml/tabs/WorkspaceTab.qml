// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import Cybou.Presence 1.0

MindTab {
    id: workspaceTab
    title: "Workspace"
    icon: "folder-workspace"

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
            text: "Current Moment"
            font.pixelSize: 18
            font.bold: true
        }

        // Current focus
        StatCard {
            title: "Focus Coalition"
            value: moment.focus ? moment.focus.toString().left(8) : "None"
            icon: "target"
            Layout.alignment: Qt.AlignHCenter
        }

        StatCard {
            title: "Salience"
            value: moment.salience.toFixed(2)
            icon: "chart-line"
            Layout.alignment: Qt.AlignHCenter
        }

        StatCard {
            title: "Organs Involved"
            value: moment.organs.join(", ")
            icon: "nodes"
            Layout.alignment: Qt.AlignHCenter
        }

        Label {
            Layout.alignment: Qt.AlignHCenter
            text: "Coalitions"
            font.pixelSize: 16
            font.bold: true
        }

        // Coalitions list
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: coalitions
            delegate: ItemDelegate {
                text: "%1: %2 organs, salience: %3".arg(modelData.correlationId.toString().left(8))
                                                     .arg(modelData.organs.length)
                                                     .arg(modelData.salience.toFixed(2))
                onClicked: console.log("Coalition clicked:", modelData)
            }
        }
    }

    property QVariantList coalitions: presence.coalitions()
    property QVariantMap moment: presence.moment()

    function updateData() {
        coalitions = presence.coalitions()
        moment = presence.moment()
    }

    Component.onCompleted: updateData()
}
