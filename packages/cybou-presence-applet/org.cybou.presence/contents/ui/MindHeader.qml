// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.kde.kirigami as Kirigami

Item {
    id: root

    required property var mind
    property string title: ""
    property string icon: "cybou"

    readonly property bool awake: Boolean(mind && mind.awake)

    implicitHeight: 58

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 13
        anchors.rightMargin: 12
        spacing: 8

        Kirigami.Icon {
            Layout.preferredWidth: 20
            Layout.preferredHeight: 20
            source: root.icon
            opacity: 0.86
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            Label {
                Layout.fillWidth: true
                text: root.title
                font.pixelSize: 15
                font.bold: true
                elide: Text.ElideRight
            }

            Label {
                Layout.fillWidth: true
                text: root.awake
                    ? i18n("Cognitive runtime connected")
                    : i18n("Waiting for cognitive services")
                font.pixelSize: 10
                opacity: 0.54
                elide: Text.ElideRight
            }
        }

        Item {
            Layout.preferredWidth: stateRow.implicitWidth + 16
            Layout.preferredHeight: 24

            Rectangle {
                anchors.fill: parent
                radius: 12
                color: root.awake
                    ? Kirigami.Theme.highlightColor
                    : Kirigami.Theme.disabledTextColor
                opacity: root.awake ? 0.12 : 0.08
            }

            RowLayout {
                id: stateRow
                anchors.centerIn: parent
                spacing: 5

                Rectangle {
                    Layout.preferredWidth: 6
                    Layout.preferredHeight: 6
                    radius: width / 2
                    color: root.awake
                        ? Kirigami.Theme.highlightColor
                        : Kirigami.Theme.disabledTextColor
                }

                Label {
                    text: root.awake ? i18n("Online") : i18n("Offline")
                    font.pixelSize: 10
                    font.bold: true
                }
            }
        }
    }
}
