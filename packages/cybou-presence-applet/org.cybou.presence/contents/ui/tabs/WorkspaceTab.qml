// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import "../utils"

Item {
    id: workspaceTab

    required property var mind
    readonly property string title: "Workspace"
    readonly property string icon: "folder-workspace"

    readonly property var currentMoment: mind.moment

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

        StatCard {
            title: "Focus Coalition"
            value: currentMoment.focus && currentMoment.focus.length > 0
                ? currentMoment.focus.slice(0, 8)
                : "None"
            icon: "target"
            Layout.alignment: Qt.AlignHCenter
        }

        StatCard {
            title: "Salience"
            value: currentMoment.salience ? Number(currentMoment.salience).toFixed(2) : "0.00"
            icon: "chart-line"
            Layout.alignment: Qt.AlignHCenter
        }

        StatCard {
            title: "Organs Involved"
            value: currentMoment.organs && currentMoment.organs.length > 0
                ? currentMoment.organs.join(", ")
                : "None"
            icon: "nodes"
            Layout.alignment: Qt.AlignHCenter
        }

        Label {
            Layout.alignment: Qt.AlignHCenter
            text: "Coalitions"
            font.pixelSize: 16
            font.bold: true
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: mind.coalitions

            delegate: ItemDelegate {
                required property var modelData
                width: ListView.view.width
                text: "%1: %2 organs, salience: %3"
                    .arg(String(modelData.correlationId).slice(0, 8))
                    .arg(modelData.organs.length)
                    .arg(Number(modelData.salience).toFixed(2))
            }
        }
    }
}
