// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.kde.kirigami as Kirigami
import "../utils"

Item {
    id: intentionsTab

    required property var mind
    readonly property string title: "Intentions"
    readonly property string icon: "task-complete"

    property var intentionModel: []

    function refreshIntentions() {
        intentionModel = mind.detailedObligations()
    }

    Component.onCompleted: refreshIntentions()

    Connections {
        target: mind
        function onChanged() {
            intentionsTab.refreshIntentions()
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 8

        GridLayout {
            Layout.fillWidth: true
            columns: width >= 280 ? 2 : 1
            rowSpacing: 8
            columnSpacing: 8

            StatCard {
                Layout.fillWidth: true
                title: i18n("Open")
                value: intentionsTab.intentionModel.length
                emphasized: intentionsTab.intentionModel.length > 0
            }

            StatCard {
                Layout.fillWidth: true
                title: i18n("Oldest")
                value: i18n("%1 d", mind.stats.oldestObligationDays || 0)
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ListView {
                id: intentionsList

                anchors.fill: parent
                clip: true
                spacing: 5
                model: intentionsTab.intentionModel
                boundsBehavior: Flickable.StopAtBounds

                ScrollBar.vertical: ThinScrollBar {}

                delegate: ItemDelegate {
                    id: intentionDelegate

                    required property int index
                    required property var modelData

                    width: ListView.view.width
                    implicitHeight: intentionRow.implicitHeight + 14
                    activeFocusOnTab: true

                    background: Rectangle {
                        radius: 9
                        color: intentionDelegate.hovered
                            ? Kirigami.Theme.highlightColor
                            : Kirigami.Theme.alternateBackgroundColor
                        opacity: intentionDelegate.hovered ? 0.08 : 1.0

                        border.width: intentionDelegate.activeFocus ? 1 : 0
                        border.color: Kirigami.Theme.focusColor
                    }

                    contentItem: RowLayout {
                        id: intentionRow
                        spacing: 7

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2

                            Label {
                                Layout.fillWidth: true
                                text: modelData.description || i18n("Untitled intention")
                                font.bold: true
                                wrapMode: Text.WordWrap
                            }

                            Label {
                                Layout.fillWidth: true
                                text: modelData.trigger || i18n("No trigger recorded")
                                font.pixelSize: 10
                                opacity: 0.55
                                elide: Text.ElideRight
                            }
                        }

                        ToolButton {
                            icon.name: "checkmark"
                            display: AbstractButton.IconOnly
                            activeFocusOnTab: true
                            ToolTip.text: i18n("Fulfill")
                            ToolTip.visible: hovered || activeFocus
                            onClicked: {
                                if (mind.fulfillIndex(intentionDelegate.index))
                                    intentionsTab.refreshIntentions()
                            }
                        }

                        ToolButton {
                            icon.name: "edit-delete"
                            display: AbstractButton.IconOnly
                            activeFocusOnTab: true
                            ToolTip.text: i18n("Abandon")
                            ToolTip.visible: hovered || activeFocus
                            onClicked: {
                                if (mind.abandonIndex(intentionDelegate.index))
                                    intentionsTab.refreshIntentions()
                            }
                        }
                    }
                }
            }

            Label {
                anchors.centerIn: parent
                visible: intentionsTab.intentionModel.length === 0
                text: i18n("No open obligations")
                opacity: 0.48
            }
        }
    }
}
