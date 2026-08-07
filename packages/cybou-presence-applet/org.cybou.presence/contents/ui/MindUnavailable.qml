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
        spacing: 10

        Kirigami.Icon {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 44
            Layout.preferredHeight: 44
            source: "network-disconnect"
            opacity: 0.58
        }

        Label {
            Layout.fillWidth: true
            text: i18n("Mind unavailable")
            font.pixelSize: 18
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
        }

        Label {
            Layout.fillWidth: true
            text: i18n("The panel is ready, but the cognitive services have not connected yet.")
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
            opacity: 0.58
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: errorText.implicitHeight + 18
            visible: mind.lastError && mind.lastError.length > 0
            radius: 9
            color: Kirigami.Theme.alternateBackgroundColor
            border.width: 0

            Label {
                id: errorText

                anchors.fill: parent
                anchors.margins: 9
                text: mind.lastError || ""
                wrapMode: Text.WordWrap
                font.pixelSize: 10
                opacity: 0.68
                horizontalAlignment: Text.AlignHCenter
            }
        }

        Button {
            Layout.alignment: Qt.AlignHCenter
            text: i18n("Retry connection")
            icon.name: "view-refresh"
            activeFocusOnTab: true
            onClicked: mind.wake()
        }
    }
}
