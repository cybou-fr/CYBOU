// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.kde.plasma.plasmoid 2.0
import org.kde.plasma.core as PlasmaCore
import org.kde.kirigami as Kirigami
import org.cybou.presence 1.0

PlasmoidItem {
    id: root

    property bool onboardingVisible: false

    activationTogglesExpanded: false
    preferredRepresentation: root.fullRepresentation

    toolTipMainText: i18n("Cybou Mind")
    toolTipSubText: dockAccess.pinned
        ? i18n("Pinned open · click or Meta+M to return to auto-hide")
        : i18n("Hover to peek · click or Meta+M to pin")

    Plasmoid.backgroundHints: PlasmaCore.Types.NoBackground

    DockAccess {
        id: dockAccess
    }

    function dismissOnboarding() {
        onboardingVisible = false
        onboardingDismiss.stop()

        if (!plasmoid.configuration.onboardingSeen)
            plasmoid.configuration.onboardingSeen = true
    }

    Connections {
        target: plasmoid

        function onActivated() {
            root.dismissOnboarding()
            dockAccess.togglePinned()
        }
    }

    Timer {
        id: onboardingDelay
        interval: 900
        repeat: false

        onTriggered: {
            if (!plasmoid.configuration.onboardingSeen) {
                root.onboardingVisible = true
                onboardingDismiss.start()
            }
        }
    }

    Timer {
        id: onboardingDismiss
        interval: 7000
        repeat: false

        onTriggered: root.dismissOnboarding()
    }

    Component.onCompleted: {
        if (!plasmoid.configuration.onboardingSeen)
            onboardingDelay.start()
    }

    fullRepresentation: Component {
        Item {
            id: handleSurface

            Layout.minimumWidth: 12
            Layout.preferredWidth: 18
            Layout.maximumWidth: 24
            Layout.minimumHeight: 64
            Layout.preferredHeight: 82
            Layout.maximumHeight: 96

            Accessible.name: i18n("Cybou Mind handle")
            Accessible.description: dockAccess.pinned
                ? i18n("Mind is pinned open")
                : i18n("Hover to peek, click to pin, or press Meta+M")

            Rectangle {
                id: capsule

                anchors.centerIn: parent
                width: handleMouse.containsMouse || dockAccess.pinned ? 9 : 7
                height: handleMouse.containsMouse || dockAccess.pinned ? 54 : 48
                radius: width / 2

                color: Kirigami.Theme.highlightColor
                opacity: dockAccess.pinned
                    ? 0.96
                    : handleMouse.containsMouse
                        ? 0.86
                        : 0.56

                Behavior on width {
                    NumberAnimation { duration: 110 }
                }

                Behavior on height {
                    NumberAnimation { duration: 110 }
                }

                Behavior on opacity {
                    NumberAnimation { duration: 110 }
                }

                Rectangle {
                    anchors.centerIn: parent
                    width: 3
                    height: 3
                    radius: width / 2
                    color: Kirigami.Theme.highlightedTextColor
                    opacity: dockAccess.pinned ? 0.95 : 0.68
                }

                ToolTip.text: root.onboardingVisible
                    ? i18n("Cybou Mind lives here · hover to peek · Meta+M to pin")
                    : dockAccess.lastError.length > 0
                        ? i18n("Mind access error: %1", dockAccess.lastError)
                        : dockAccess.pinned
                            ? i18n("Mind pinned open · click to unpin")
                            : i18n("Hover to peek · click to pin · Meta+M")
                ToolTip.visible:
                    root.onboardingVisible
                    || handleMouse.containsMouse
                ToolTip.delay: root.onboardingVisible ? 0 : 250
            }

            MouseArea {
                id: handleMouse

                anchors.fill: parent
                hoverEnabled: true
                acceptedButtons: Qt.LeftButton
                cursorShape: Qt.PointingHandCursor

                onEntered: {
                    root.dismissOnboarding()
                    dockAccess.peek()
                }

                onClicked: {
                    root.dismissOnboarding()
                    dockAccess.togglePinned()
                }
            }
        }
    }
}
