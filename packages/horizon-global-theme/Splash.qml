// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: CC-BY-SA-4.0
//
// Cybou Horizon splash.
//
// Deliberately minimal. docs/08-testing-acceptance.md makes a QML error during normal login a
// blocking defect, so this uses only QtQuick basics: no Plasma imports, no components that can
// go missing, nothing that needs a running shell. A splash that fails is worse than a plain one.
//
// The stage property is driven by ksplashqml; stage 1 is the earliest point at which anything
// should be shown.

import QtQuick

Rectangle {
    id: root

    // canvas token; the splash must not flash a different colour before the session appears
    color: "#0A0D12"

    property int stage

    onStageChanged: {
        if (stage >= 1) {
            mark.opacity = 1;
        }
    }

    Image {
        id: mark
        anchors.centerIn: parent
        source: "images/cybou-aperture.svg"
        sourceSize.width: 96
        sourceSize.height: 96
        opacity: 0

        // One short fade, no spinner, no pulse - "restrained motion" from the design system.
        Behavior on opacity {
            NumberAnimation {
                duration: 180
            }
        }
    }

    // The horizon, one pixel, at the same 61% of height as the wallpaper.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        y: Math.round(parent.height * 0.61)
        height: 1
        color: "#70E1C8"
        opacity: mark.opacity * 0.25
    }
}
