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
    readonly property string icon: "history"

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
        spacing: 9

        StatCard {
            Layout.fillWidth: true
            title: i18n("Total contributions")
            value: mind.contributions
            icon: "database"
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ListView {
                id: activityList
                anchors.fill: parent
                clip: true
                spacing: 4
                model: activityTab.activityModel

                delegate: ItemDelegate {
                    id: activityDelegate

                    required property var modelData

                    width: ListView.view.width
                    implicitHeight: 54

                    background: Rectangle {
                        radius: 8
                        color: activityDelegate.hovered
                            ? Kirigami.Theme.highlightColor
                            : "transparent"
                        opacity: activityDelegate.hovered ? 0.08 : 1.0
                    }

                    contentItem: RowLayout {
                        spacing: 9

                        Label {
                            Layout.preferredWidth: 42
                            text: Qt.formatTime(modelData.when, "HH:mm")
                            font.pixelSize: 11
                            opacity: 0.58
                        }

                        Kirigami.Icon {
                            Layout.preferredWidth: 18
                            Layout.preferredHeight: 18
                            source: "media-record"
                            opacity: 0.70
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
                                font.pixelSize: 11
                                opacity: 0.60
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
                opacity: 0.55
            }
        }
    }
}
