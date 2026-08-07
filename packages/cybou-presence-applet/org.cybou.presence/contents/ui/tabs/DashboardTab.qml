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

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10

        InfoCard {
            Layout.fillWidth: true
            title: i18n("Current self-narration")
            text: mind.narration || i18n("No narration yet.")
            icon: "user"
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
            columns: 2
            rowSpacing: 9
            columnSpacing: 9

            StatCard {
                Layout.fillWidth: true
                title: i18n("Contributions")
                value: mind.contributions
                icon: "database"
            }

            StatCard {
                Layout.fillWidth: true
                title: i18n("Open intentions")
                value: mind.stats.openIntentions
                icon: "task-complete"
            }

            StatCard {
                Layout.fillWidth: true
                title: i18n("Sessions")
                value: mind.stats.sessions
                icon: "history"
            }

            StatCard {
                Layout.fillWidth: true
                title: i18n("Predictions settled")
                value: mind.stats.settledPredictions
                icon: "checkmark"
            }
        }

        InfoCard {
            Layout.fillWidth: true
            title: i18n("Attention")
            text: mind.attention || i18n("Quiet")
            icon: "eye"
        }

        Item {
            Layout.fillHeight: true
        }
    }
}
