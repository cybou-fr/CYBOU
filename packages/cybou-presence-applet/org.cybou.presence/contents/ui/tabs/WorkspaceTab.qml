// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.kde.kirigami as Kirigami
import "../utils"

Item {
    id: workspaceTab

    required property var mind
    readonly property string title: "Workspace"
    readonly property string icon: "folder"

    readonly property var currentMoment: mind.moment || ({})

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 8

        InfoCard {
            Layout.fillWidth: true
            title: i18n("Current focus")
            text: currentMoment.focus && currentMoment.focus.length > 0
                ? i18n(
                    "%1 · salience %2",
                    String(currentMoment.focus).slice(0, 12),
                    Number(currentMoment.salience || 0).toFixed(2)
                )
                : i18n("No coalition currently owns attention.")
            icon: "folder"
            emphasized: Boolean(currentMoment.focus && currentMoment.focus.length > 0)
        }

        StatCard {
            Layout.fillWidth: true
            title: i18n("Organs involved")
            value: currentMoment.organs && currentMoment.organs.length > 0
                ? currentMoment.organs.join(", ")
                : i18n("None")
        }

        Label {
            Layout.fillWidth: true
            text: i18n("Coalitions")
            font.pixelSize: 10
            font.bold: true
            opacity: 0.56
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ListView {
                id: coalitionList

                anchors.fill: parent
                clip: true
                spacing: 3
                model: mind.coalitions
                boundsBehavior: Flickable.StopAtBounds

                ScrollBar.vertical: ThinScrollBar {}

                delegate: ItemDelegate {
                    id: coalitionDelegate

                    required property var modelData

                    width: ListView.view.width
                    implicitHeight: 56
                    activeFocusOnTab: true

                    background: Rectangle {
                        radius: 8
                        color: coalitionDelegate.hovered
                            ? Kirigami.Theme.highlightColor
                            : "transparent"
                        opacity: coalitionDelegate.hovered ? 0.07 : 1.0

                        border.width: coalitionDelegate.activeFocus ? 1 : 0
                        border.color: Kirigami.Theme.focusColor
                    }

                    contentItem: ColumnLayout {
                        spacing: 2

                        Label {
                            Layout.fillWidth: true
                            text: String(modelData.correlationId).slice(0, 12)
                            font.bold: true
                            elide: Text.ElideRight
                        }

                        Label {
                            Layout.fillWidth: true
                            text: i18n(
                                "%1 organs · salience %2 · %3 threads",
                                modelData.organs ? modelData.organs.length : 0,
                                Number(modelData.salience || 0).toFixed(2),
                                modelData.threads || 0
                            )
                            font.pixelSize: 10
                            opacity: 0.52
                            elide: Text.ElideRight
                        }
                    }
                }
            }

            Label {
                anchors.centerIn: parent
                visible: mind.coalitions.length === 0
                text: i18n("No active coalitions")
                opacity: 0.48
            }
        }
    }
}
