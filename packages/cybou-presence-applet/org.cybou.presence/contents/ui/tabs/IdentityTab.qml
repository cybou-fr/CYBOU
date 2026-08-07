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

    Flickable {
        id: scroll

        anchors.fill: parent
        clip: true
        contentWidth: width
        contentHeight: contentColumn.implicitHeight + 24
        boundsBehavior: Flickable.StopAtBounds

        ScrollBar.vertical: ThinScrollBar {}

        ColumnLayout {
            id: contentColumn

            x: 12
            y: 12
            width: Math.max(0, scroll.width - 28)
            spacing: 8

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
                emphasized: true
            }

            StatCard {
                Layout.fillWidth: true
                title: i18n("Origin")
                value: mind.identityState.origin || i18n("Unknown")
            }

            GridLayout {
                Layout.fillWidth: true
                columns: width >= 280 ? 2 : 1
                rowSpacing: 8
                columnSpacing: 8

                StatCard {
                    Layout.fillWidth: true
                    title: i18n("Sessions")
                    value: mind.identityState.sessionCount
                }

                StatCard {
                    Layout.fillWidth: true
                    title: i18n("Born now")
                    value: mind.identityState.wasBorn ? i18n("Yes") : i18n("No")
                }
            }

            StatCard {
                Layout.fillWidth: true
                title: i18n("Architecture")
                value: mind.identityState.architectureVersion || i18n("Unknown")
            }
        }
    }
}
