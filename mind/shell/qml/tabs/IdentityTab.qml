// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import Cybou.Presence 1.0

MindTab {
    id: identityTab
    title: "Identity"
    icon: "user-identity"

    Presence {
        id: presence
        onChanged: updateData()
    }

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

            StatCard { title: "UUID"; value: identityState.uuid; icon: "fingerprint" }
            StatCard { title: "Origin"; value: identityState.origin; icon: "calendar" }
            StatCard { title: "Sessions"; value: identityState.sessionCount; icon: "history" }
            StatCard { title: "Architecture Version"; value: identityState.archVersion; icon: "code" }
            StatCard { title: "Was Born"; value: identityState.wasBorn ? "Yes" : "No"; icon: "heart" }
        }
    }

    property QVariantMap identityState: presence.identityState()

    function updateData() {
        identityState = presence.identityState()
    }

    Component.onCompleted: updateData()
}
