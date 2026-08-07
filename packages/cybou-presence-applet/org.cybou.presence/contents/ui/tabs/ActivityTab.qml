// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.kde.kirigami as Kirigami
import "../utils"

Item {
    id: activityTab

    required property var mind
    readonly property string title: "Activity"
    readonly property string icon: "view-history"

    property var activityModel: []

    function refreshActivity() {
        activityModel = mind.activity(30)
    }

    Component.onCompleted: refreshActivity()

    Connections {
        target: mind
        function onChanged() {
            activityTab.refreshActivity()
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 8

        StatCard {
            Layout.fillWidth: true
            title: i18n("Total contributions")
            value: mind.contributions
            emphasized: true
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ListView {
                id: activityList

                anchors.fill: parent
                clip: true
                spacing: 3
                model: activityTab.activityModel
                boundsBehavior: Flickable.StopAtBounds

                ScrollBar.vertical: ThinScrollBar {}

                delegate: ItemDelegate {
                    id: activityDelegate

                    required property var modelData

                    width: ListView.view.width
                    implicitHeight: 52
                    activeFocusOnTab: true

                    background: Rectangle {
                        radius: 8
                        color: activityDelegate.hovered
                            ? Kirigami.Theme.highlightColor
                            : "transparent"
                        opacity: activityDelegate.hovered ? 0.07 : 1.0

                        border.width: activityDelegate.activeFocus ? 1 : 0
                        border.color: Kirigami.Theme.focusColor
                    }

                    contentItem: RowLayout {
                        spacing: 9

                        Label {
                            Layout.preferredWidth: 38
                            text: Qt.formatTime(modelData.when, "HH:mm")
                            font.pixelSize: 10
                            opacity: 0.48
                        }

                        Rectangle {
                            Layout.preferredWidth: 6
                            Layout.preferredHeight: 6
                            radius: width / 2
                            color: Kirigami.Theme.highlightColor
                            opacity: 0.62
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 1

                            Label {
                                Layout.fillWidth: true
                                text: modelData.kind || i18n("Contribution")
                                font.bold: true
                                elide: Text.ElideRight
                            }

                            Label {
                                Layout.fillWidth: true
                                text: modelData.organ || i18n("Unknown organ")
                                font.pixelSize: 10
                                opacity: 0.52
                                elide: Text.ElideRight
                            }
                        }
                    }
                }
            }

            Label {
                anchors.centerIn: parent
                visible: activityTab.activityModel.length === 0
                text: i18n("No activity yet")
                opacity: 0.48
            }
        }
    }
}
