// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.cybou.presence 1.0

MindTab {
    id: workspaceTab
    title: "Workspace"
    icon: "folder-workspace"

    // Update data when mind changes
    Connections {
        target: mind
        function onChanged() {
            coalitions = mind.coalitions()
            moment = mind.moment()
        }
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
            value: moment.salience ? moment.salience.toFixed(2) : "0.00"
            icon: "chart-line"
            Layout.alignment: Qt.AlignHCenter
        }

        StatCard {
            title: "Organs Involved"
            value: moment.organs ? moment.organs.join(", ") : "None"
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

    property QVariantList coalitions: mind.coalitions()
    property QVariantMap moment: mind.moment()
}
