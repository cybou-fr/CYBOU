// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.kde.kirigami as Kirigami

Item {
    id: root

    required property var mind

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.max(220, Math.min(parent.width - 36, 300))
        spacing: 12

        Kirigami.Icon {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 54
            Layout.preferredHeight: 54
            source: "network-disconnect"
            opacity: 0.72
        }

        Label {
            Layout.fillWidth: true
            text: i18n("Mind unavailable")
            font.pixelSize: 20
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
        }

        Label {
            Layout.fillWidth: true
            text: i18n("The panel is ready, but the cognitive services have not connected yet.")
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
            opacity: 0.72
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: errorText.implicitHeight + 20
            visible: mind.lastError && mind.lastError.length > 0
            radius: 8
            color: Kirigami.Theme.backgroundColor
            border.width: 1
            border.color: Kirigami.Theme.disabledTextColor

            Label {
                id: errorText
                anchors.fill: parent
                anchors.margins: 10
                text: mind.lastError || ""
                wrapMode: Text.WordWrap
                font.pixelSize: 11
                opacity: 0.76
                horizontalAlignment: Text.AlignHCenter
            }
        }

        Button {
            Layout.alignment: Qt.AlignHCenter
            text: i18n("Retry connection")
            icon.name: "view-refresh"
            onClicked: mind.wake()
        }
    }
}
