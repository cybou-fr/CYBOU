// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.kde.plasma.plasmoid 2.0
import org.cybou.presence 1.0

PlasmoidItem {
    id: root
    preferredRepresentation: fullRepresentation

    property int dockWidth: 460
    property int tabHeight: 48

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
            }

            // Identity Tab
            IdentityTab {
                Layout.fillWidth: true
                Layout.fillHeight: true
            }

            // Intentions Tab
            IntentionsTab {
                Layout.fillWidth: true
                Layout.fillHeight: true
            }

            // Activity Tab
            ActivityTab {
                Layout.fillWidth: true
                Layout.fillHeight: true
            }

            // Self Tab
            SelfTab {
                Layout.fillWidth: true
                Layout.fillHeight: true
            }

            // Predictor Tab
            PredictorTab {
                Layout.fillWidth: true
                Layout.fillHeight: true
            }

            // Workspace Tab
            WorkspaceTab {
                Layout.fillWidth: true
                Layout.fillHeight: true
            }
        }
    }

    // Register the dock widget
    Plasmoid.fullRepresentation: Component {
        Item {
            width: dockWidth
            height: childrenRect.height
            MindDock {
                anchors.fill: parent
            }
        }
    }
}
