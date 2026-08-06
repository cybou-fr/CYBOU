// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.cybou.presence 1.0

Item {
    id: root
    property var mind  // Shared Presence instance from parent
    property int dockWidth: 460
    property int tabHeight: 48

    // Forward mind's changed signal to all tabs
    Connections {
        target: mind
        function onChanged() {
            // This will trigger re-evaluation of all bindings in tabs
            // that use mind.* properties
        }
    }

    // Main container
    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Tab bar at the top of the dock
        MindTabBar {
            id: tabBar
            Layout.fillWidth: true
            Layout.preferredHeight: tabHeight
        }

        // Stack for tab content
        StackLayout {
            id: tabStack
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: tabBar.currentIndex

            // Dashboard Tab
            DashboardTab {
                Layout.fillWidth: true
                Layout.fillHeight: true
                mind: root.mind
            }

            // Identity Tab
            IdentityTab {
                Layout.fillWidth: true
                Layout.fillHeight: true
                mind: root.mind
            }

            // Intentions Tab
            IntentionsTab {
                Layout.fillWidth: true
                Layout.fillHeight: true
                mind: root.mind
            }

            // Activity Tab
            ActivityTab {
                Layout.fillWidth: true
                Layout.fillHeight: true
                mind: root.mind
            }

            // Self Tab
            SelfTab {
                Layout.fillWidth: true
                Layout.fillHeight: true
                mind: root.mind
            }

            // Predictor Tab
            PredictorTab {
                Layout.fillWidth: true
                Layout.fillHeight: true
                mind: root.mind
            }

            // Workspace Tab
            WorkspaceTab {
                Layout.fillWidth: true
                Layout.fillHeight: true
                mind: root.mind
            }
        }
    }
}
