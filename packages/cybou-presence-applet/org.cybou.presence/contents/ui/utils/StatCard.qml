// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.kde.kirigami as Kirigami

ColumnLayout {
    id: root

    property string title: ""
    property var value: ""
    property string icon: ""

    Layout.alignment: Qt.AlignHCenter
    spacing: 4

    Kirigami.Icon {
        Layout.alignment: Qt.AlignHCenter
        Layout.preferredWidth: 32
        Layout.preferredHeight: 32
        source: root.icon
    }

    Label {
        Layout.alignment: Qt.AlignHCenter
        Layout.fillWidth: true
        text: root.title
        font.pixelSize: 12
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideRight
        maximumLineCount: 1
    }

    Label {
        Layout.alignment: Qt.AlignHCenter
        Layout.fillWidth: true
        text: root.value === undefined || root.value === null ? "" : String(root.value)
        font.pixelSize: 18
        font.bold: true
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideRight
        maximumLineCount: 1
    }
}
