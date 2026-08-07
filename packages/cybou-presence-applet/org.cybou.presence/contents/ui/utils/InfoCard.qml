// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.kde.kirigami as Kirigami

Rectangle {
    id: root

    property string title: ""
    property string text: ""
    property string icon: ""
    property bool emphasized: false

    implicitWidth: 220
    implicitHeight: content.implicitHeight + 24
    radius: 10

    color: Kirigami.Theme.backgroundColor
    border.width: 1
    border.color: root.emphasized
        ? Kirigami.Theme.highlightColor
        : Kirigami.Theme.disabledTextColor

    RowLayout {
        id: content
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        Kirigami.Icon {
            Layout.alignment: Qt.AlignTop
            Layout.preferredWidth: 24
            Layout.preferredHeight: 24
            source: root.icon
            visible: root.icon.length > 0
            opacity: 0.82
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 4

            Label {
                Layout.fillWidth: true
                text: root.title
                font.pixelSize: 11
                font.bold: true
                opacity: 0.72
                elide: Text.ElideRight
            }

            Label {
                Layout.fillWidth: true
                text: root.text
                wrapMode: Text.WordWrap
                font.pixelSize: 13
            }
        }
    }
}
