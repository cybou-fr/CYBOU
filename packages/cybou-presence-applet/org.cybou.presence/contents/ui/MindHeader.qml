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

    implicitHeight: 68

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 14
        anchors.rightMargin: 14
        spacing: 10

        Kirigami.Icon {
            Layout.preferredWidth: 24
            Layout.preferredHeight: 24
            source: root.icon
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1

            Label {
                Layout.fillWidth: true
                text: root.title
                font.pixelSize: 16
                font.bold: true
                elide: Text.ElideRight
            }

            Label {
                Layout.fillWidth: true
                text: root.awake
                    ? i18n("Cognitive runtime connected")
                    : i18n("Waiting for cognitive services")
                font.pixelSize: 11
                opacity: 0.68
                elide: Text.ElideRight
            }
        }

        Rectangle {
            Layout.preferredWidth: stateRow.implicitWidth + 18
            Layout.preferredHeight: 28
            radius: 14
            color: "transparent"
            border.width: 1
            border.color: root.awake
                ? Kirigami.Theme.highlightColor
                : Kirigami.Theme.disabledTextColor

            RowLayout {
                id: stateRow
                anchors.centerIn: parent
                spacing: 6

                Rectangle {
                    Layout.preferredWidth: 7
                    Layout.preferredHeight: 7
                    radius: width / 2
                    color: root.awake
                        ? Kirigami.Theme.highlightColor
                        : Kirigami.Theme.disabledTextColor
                }

                Label {
                    text: root.awake ? i18n("Online") : i18n("Offline")
                    font.pixelSize: 11
                    font.bold: true
                }
            }
        }
    }
}
