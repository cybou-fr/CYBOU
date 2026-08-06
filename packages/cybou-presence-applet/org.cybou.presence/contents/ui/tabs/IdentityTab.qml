// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.cybou.presence 1.0

MindTab {
    id: identityTab
    title: "Identity"
    icon: "user-identity"

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

        // Identity state
        GridLayout {
            Layout.fillWidth: true
            columns: 1
            spacing: 11

            StatCard { title: "UUID"; value: mind.identityState.uuid; icon: "fingerprint" }
            StatCard { title: "Origin"; value: mind.identityState.origin; icon: "calendar" }
            StatCard { title: "Sessions"; value: mind.identityState.sessionCount; icon: "history" }
            StatCard { title: "Architecture Version"; value: mind.identityState.architectureVersion; icon: "code" }
            StatCard { title: "Was Born"; value: mind.identityState.wasBorn ? "Yes" : "No"; icon: "heart" }
        }
    }
}
