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
    property bool showIcon: true
    property bool emphasized: false

    implicitWidth: 220
    implicitHeight: content.implicitHeight + 22
    radius: 10

    color: Kirigami.Theme.alternateBackgroundColor
    border.width: 0

    Rectangle {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: root.emphasized ? 3 : 2
        radius: width / 2
        color: Kirigami.Theme.highlightColor
        opacity: root.emphasized ? 0.95 : 0.32
    }

    RowLayout {
        id: content

        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 12
        anchors.topMargin: 11
        anchors.bottomMargin: 11
        spacing: 9

        Kirigami.Icon {
            Layout.alignment: Qt.AlignTop
            Layout.preferredWidth: 21
            Layout.preferredHeight: 21
            source: root.icon
            visible: root.showIcon && root.icon.length > 0
            opacity: 0.76
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 3

            Label {
                Layout.fillWidth: true
                text: root.title
                font.pixelSize: 10
                font.bold: true
                opacity: 0.60
                elide: Text.ElideRight
            }

            Label {
                Layout.fillWidth: true
                text: root.text
                wrapMode: Text.WordWrap
                font.pixelSize: 13
                lineHeight: 1.12
            }
        }
    }
}
