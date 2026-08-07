// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.kde.kirigami as Kirigami

Rectangle {
    id: root

    property string title: ""
    property var value: ""
    property string icon: ""
    property bool showIcon: false
    property bool emphasized: false

    implicitWidth: 138
    implicitHeight: 70
    radius: 10

    color: Kirigami.Theme.alternateBackgroundColor
    border.width: root.emphasized ? 1 : 0
    border.color: Kirigami.Theme.highlightColor

    RowLayout {
        anchors.fill: parent
        anchors.margins: 11
        spacing: 9

        Rectangle {
            Layout.preferredWidth: 3
            Layout.preferredHeight: 30
            radius: 2
            color: Kirigami.Theme.highlightColor
            opacity: root.emphasized ? 0.90 : 0.40
        }

        Kirigami.Icon {
            Layout.preferredWidth: 20
            Layout.preferredHeight: 20
            source: root.icon
            visible: root.showIcon && root.icon.length > 0
            opacity: 0.76
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1

            Label {
                Layout.fillWidth: true
                text: root.title
                font.pixelSize: 10
                font.bold: true
                opacity: 0.58
                elide: Text.ElideRight
                maximumLineCount: 1
            }

            Label {
                Layout.fillWidth: true
                text: root.value === undefined || root.value === null
                    ? ""
                    : String(root.value)
                font.pixelSize: 17
                font.bold: true
                elide: Text.ElideRight
                maximumLineCount: 1
            }
        }
    }
}
