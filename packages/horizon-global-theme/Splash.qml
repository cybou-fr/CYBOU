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
        id: horizon
        anchors.left: parent.left
        anchors.right: parent.right
        y: Math.round(parent.height * 0.61)
        height: 1
        color: "#70E1C8"
        opacity: mark.opacity * 0.25
    }

    // Progress: a mint segment travelling along the horizon. Tied to `stage`, so it reports
    // real startup progress rather than pretending with a loop - ksplashqml raises the stage
    // as the session comes up, and the bar lands at the far edge when it is done.
    Rectangle {
        id: bar
        y: horizon.y
        height: 1
        width: Math.round(parent.width * 0.18)
        color: "#70E1C8"
        opacity: mark.opacity

        // stage runs 0..6; map it onto the width of the screen.
        x: Math.round((parent.width - width) * Math.min(root.stage, 6) / 6)

        Behavior on x {
            NumberAnimation {
                duration: 320
                easing.type: Easing.OutCubic
            }
        }
    }

    // A slow breath on the mark, so a long startup does not look frozen. One property,
    // no particles, no spinner - "no perpetual animation" allows a single calm cycle.
    SequentialAnimation {
        running: mark.opacity > 0
        loops: Animation.Infinite
        NumberAnimation {
            target: mark
            property: "scale"
            from: 1.0
            to: 1.045
            duration: 1400
            easing.type: Easing.InOutSine
        }
        NumberAnimation {
            target: mark
            property: "scale"
            from: 1.045
            to: 1.0
            duration: 1400
            easing.type: Easing.InOutSine
        }
    }
}
