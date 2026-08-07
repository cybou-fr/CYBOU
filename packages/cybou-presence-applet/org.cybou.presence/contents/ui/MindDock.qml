// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.kde.kirigami as Kirigami
import "tabs"

Item {
    id: root

    required property var mind

    readonly property bool awake: Boolean(mind && mind.awake)

    implicitWidth: 420
    implicitHeight: 720
    clip: true

    RowLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.preferredWidth: 64
            Layout.minimumWidth: 64
            Layout.fillHeight: true

            color: Kirigami.Theme.backgroundColor

            MindTabBar {
                id: tabBar
                anchors.fill: parent
                anchors.topMargin: 8
                anchors.bottomMargin: 8
                navigationEnabled: root.awake
                runtimeAvailable: root.awake
            }
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.fillHeight: true
            color: Kirigami.Theme.disabledTextColor
            opacity: 0.25
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            MindHeader {
                Layout.fillWidth: true
                mind: root.mind
                title: root.awake ? tabBar.currentTitle : i18n("Cybou")
                icon: root.awake ? tabBar.currentIcon : "cybou"
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
                color: Kirigami.Theme.disabledTextColor
                opacity: 0.18
            }

            StackLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                currentIndex: root.awake ? tabBar.currentIndex : 7

                DashboardTab {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    mind: root.mind
                }

                IdentityTab {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    mind: root.mind
                }

                IntentionsTab {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    mind: root.mind
                }

                ActivityTab {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    mind: root.mind
                }

                SelfTab {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    mind: root.mind
                }

                PredictorTab {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    mind: root.mind
                }

                WorkspaceTab {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    mind: root.mind
                }

                MindUnavailable {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    mind: root.mind
                }
            }
        }
    }
}
