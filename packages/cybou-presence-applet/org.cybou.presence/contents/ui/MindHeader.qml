// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.kde.kirigami as Kirigami

Item {
    id: root

    required property var mind
    property string title: ""
    property string icon: "cybou"

    readonly property bool awake: Boolean(mind && mind.awake)
    readonly property string lifecycleMode: mind && mind.lifecycleMode
        ? String(mind.lifecycleMode) : ""
    readonly property var lifecycle: mind && mind.lifecycleProjection
        ? mind.lifecycleProjection : ({})

    function modeLabel(mode) {
        const labels = {
            "awake": i18n("Awake"), "idle": i18n("Idle"),
            "consolidating": i18n("Consolidating"), "recovering": i18n("Recovering"),
            "degraded": i18n("Degraded"), "maintenance": i18n("Maintenance"),
            "suspended": i18n("Suspended")
        }
        return labels[mode] || i18n("Unknown")
    }

    function modeColor(mode) {
        if (mode === "degraded") return Kirigami.Theme.negativeTextColor
        if (mode === "recovering" || mode === "maintenance") return Kirigami.Theme.neutralTextColor
        if (mode === "awake" || mode === "idle") return Kirigami.Theme.positiveTextColor
        return Kirigami.Theme.highlightColor
    }

    function lifecycleSummary() {
        if (!root.awake) return i18n("Waiting for cognitive services")
        const label = root.modeLabel(root.lifecycleMode)
        if (root.lifecycle.progressClass === "running"
                || root.lifecycle.progressClass === "recovering")
            return i18n("Cognitive runtime connected · %1 · %2%", label,
                        Number(root.lifecycle.progressPercent || 0))
        if (root.lifecycle.freshnessClass && root.lifecycle.freshnessClass !== "unknown")
            return i18n("Cognitive runtime connected · %1 · %2", label,
                        String(root.lifecycle.freshnessClass))
        return i18n("Cognitive runtime connected · %1", label)
    }

    implicitHeight: 58

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 13
        anchors.rightMargin: 12
        spacing: 8

        Kirigami.Icon {
            Layout.preferredWidth: 20
            Layout.preferredHeight: 20
            source: root.icon
            opacity: 0.86
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            Label {
                Layout.fillWidth: true
                text: root.title
                font.pixelSize: 15
                font.bold: true
                elide: Text.ElideRight
            }

            Label {
                Layout.fillWidth: true
                text: root.lifecycleSummary()
                font.pixelSize: 10
                opacity: 0.54
                elide: Text.ElideRight
            }
        }

        Item {
            Layout.preferredWidth: stateRow.implicitWidth + 16
            Layout.preferredHeight: 24

            Rectangle {
                anchors.fill: parent
                radius: 12
                color: root.awake
                    ? root.modeColor(root.lifecycleMode)
                    : Kirigami.Theme.disabledTextColor
                opacity: root.awake ? 0.12 : 0.08
            }

            RowLayout {
                id: stateRow
                anchors.centerIn: parent
                spacing: 5

                Rectangle {
                    Layout.preferredWidth: 6
                    Layout.preferredHeight: 6
                    radius: width / 2
                    color: root.awake
                        ? root.modeColor(root.lifecycleMode)
                        : Kirigami.Theme.disabledTextColor
                }

                Label {
                    text: root.awake
                        ? root.modeLabel(root.lifecycleMode)
                        : i18n("Offline")
                    font.pixelSize: 10
                    font.bold: true
                }
            }
        }
    }
}
