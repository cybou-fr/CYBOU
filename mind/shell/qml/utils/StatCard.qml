// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

ColumnLayout {
    Layout.alignment: Qt.AlignHCenter
    spacing: 4

    Icon {
        Layout.alignment: Qt.AlignHCenter
        name: icon
        width: 32
        height: 32
    }

    Label {
        Layout.alignment: Qt.AlignHCenter
        text: title
        font.pixelSize: 12
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideRight
        maximumLineCount: 1
    }

    Label {
        Layout.alignment: Qt.AlignHCenter
        text: value
        font.pixelSize: 18
        font.bold: true
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideRight
        maximumLineCount: 1
    }
}
