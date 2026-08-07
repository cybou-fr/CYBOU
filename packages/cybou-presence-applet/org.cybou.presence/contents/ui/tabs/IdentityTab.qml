// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import "../utils"

Item {
    id: identityTab

    required property var mind
    readonly property string title: "Identity"
    readonly property string icon: "user-identity"

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 11
        spacing: 11

        Label {
            Layout.alignment: Qt.AlignHCenter
            text: "Subject Continuity"
            font.pixelSize: 18
            font.bold: true
        }

        GridLayout {
            Layout.fillWidth: true
            columns: 1
            rowSpacing: 11
            columnSpacing: 11

            StatCard { Layout.fillWidth: true; title: "UUID"; value: mind.identityState.uuid; icon: "fingerprint" }
            StatCard { Layout.fillWidth: true; title: "Origin"; value: mind.identityState.origin; icon: "calendar" }
            StatCard { Layout.fillWidth: true; title: "Sessions"; value: mind.identityState.sessionCount; icon: "history" }
            StatCard { Layout.fillWidth: true; title: "Architecture Version"; value: mind.identityState.architectureVersion; icon: "code" }
            StatCard { Layout.fillWidth: true; title: "Was Born"; value: mind.identityState.wasBorn ? "Yes" : "No"; icon: "heart" }
        }

        Item { Layout.fillHeight: true }
    }
}
