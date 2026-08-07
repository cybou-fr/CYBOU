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
        anchors.margins: 12
        spacing: 9

        InfoCard {
            Layout.fillWidth: true
            title: i18n("Subject continuity")
            text: i18n("One persistent identity across daemon restarts and user sessions.")
            icon: "user-identity"
            emphasized: true
        }

        StatCard {
            Layout.fillWidth: true
            title: i18n("Identity UUID")
            value: mind.identityState.uuid || i18n("Unknown")
            icon: "fingerprint"
        }

        StatCard {
            Layout.fillWidth: true
            title: i18n("Origin")
            value: mind.identityState.origin || i18n("Unknown")
            icon: "calendar"
        }

        GridLayout {
            Layout.fillWidth: true
            columns: 2
            rowSpacing: 9
            columnSpacing: 9

            StatCard {
                Layout.fillWidth: true
                title: i18n("Sessions")
                value: mind.identityState.sessionCount
                icon: "history"
            }

            StatCard {
                Layout.fillWidth: true
                title: i18n("Born now")
                value: mind.identityState.wasBorn ? i18n("Yes") : i18n("No")
                icon: "heart"
            }
        }

        StatCard {
            Layout.fillWidth: true
            title: i18n("Architecture")
            value: mind.identityState.architectureVersion || i18n("Unknown")
            icon: "code-context"
        }

        Item {
            Layout.fillHeight: true
        }
    }
}
