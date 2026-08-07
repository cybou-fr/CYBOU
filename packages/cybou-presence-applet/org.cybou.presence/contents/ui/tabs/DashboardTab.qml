// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import "../utils"

Item {
    id: dashboardTab

    required property var mind
    readonly property string title: "Dashboard"
    readonly property string icon: "view-dashboard"

    function runtimeSummary() {
        const health = mind.organHealth || {}
        const names = [
            "eventd",
            "identityd",
            "intentiond",
            "predictord",
            "selfd",
            "workspaced",
            "presenced"
        ]
        const unhealthy = []

        for (let i = 0; i < names.length; ++i) {
            const state = health[names[i]]
            if (state && state !== "healthy")
                unhealthy.push(names[i])
        }

        return unhealthy.length === 0
            ? i18n("All cognitive services report healthy.")
            : i18n("Attention required: %1", unhealthy.join(", "))
    }

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
            spacing: 9

            InfoCard {
                Layout.fillWidth: true
                title: i18n("Current self-narration")
                text: mind.narration || i18n("No narration yet.")
                icon: "user-identity"
                emphasized: true
            }

            InfoCard {
                Layout.fillWidth: true
                title: i18n("Runtime")
                text: dashboardTab.runtimeSummary()
                icon: "system-run"
            }

            GridLayout {
                Layout.fillWidth: true
                columns: width >= 280 ? 2 : 1
                rowSpacing: 8
                columnSpacing: 8

                StatCard {
                    Layout.fillWidth: true
                    title: i18n("Contributions")
                    value: mind.contributions
                }

                StatCard {
                    Layout.fillWidth: true
                    title: i18n("Open intentions")
                    value: mind.stats.openIntentions
                }

                StatCard {
                    Layout.fillWidth: true
                    title: i18n("Sessions")
                    value: mind.stats.sessions
                }

                StatCard {
                    Layout.fillWidth: true
                    title: i18n("Predictions settled")
                    value: mind.stats.settledPredictions
                }
            }

            InfoCard {
                Layout.fillWidth: true
                title: i18n("Attention")
                text: mind.attention || i18n("Quiet")
                icon: "view-visible"
            }
        }
    }
}
