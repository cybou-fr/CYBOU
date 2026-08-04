// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.components as PlasmaComponents
import org.kde.plasma.extras as PlasmaExtras
import org.kde.kirigami as Kirigami
import org.cybou.presence

PlasmoidItem {
    id: root

    property bool ready: false

    Presence {
        id: mind
        Component.onCompleted: root.ready = wake()
    }

    // Being looked at is an event worth remembering, and it refreshes what the panel reads.
    onExpandedChanged: if (expanded && root.ready) mind.reflect()

    // The mind is not busy every second; a slow tick is enough and costs nothing noticeable.
    Timer {
        interval: 20000
        running: root.expanded && root.ready
        repeat: true
        onTriggered: mind.changed()
    }

    Plasmoid.status: root.ready ? PlasmaCore.Types.ActiveStatus
                                : PlasmaCore.Types.PassiveStatus

    toolTipMainText: i18n("Cybou")
    toolTipSubText: root.ready ? mind.narration : i18n("Not awake.")

    compactRepresentation: Item {
        Layout.minimumWidth: Kirigami.Units.iconSizes.small
        Layout.minimumHeight: Kirigami.Units.iconSizes.small

        Kirigami.Icon {
            id: mark
            anchors.centerIn: parent
            width: Math.min(parent.width, parent.height)
            height: width
            source: "cybou"
            // Dimmed while asleep: the icon must never look alive when nothing is behind it.
            opacity: root.ready ? 1.0 : 0.4

            // A slow breath, driven by nothing but time. It signals "running", not "thinking" -
            // an animation tied to activity the system is not doing would be a lie.
            SequentialAnimation on scale {
                running: root.ready
                loops: Animation.Infinite
                NumberAnimation { to: 1.06; duration: 2600; easing.type: Easing.InOutSine }
                NumberAnimation { to: 1.00; duration: 2600; easing.type: Easing.InOutSine }
            }
        }

        MouseArea {
            anchors.fill: parent
            onClicked: root.expanded = !root.expanded
        }
    }

    fullRepresentation: PlasmaExtras.Representation {
        Layout.minimumWidth: Kirigami.Units.gridUnit * 22
        Layout.minimumHeight: Kirigami.Units.gridUnit * 26
        Layout.preferredWidth: Kirigami.Units.gridUnit * 24
        Layout.preferredHeight: Kirigami.Units.gridUnit * 30

        header: PlasmaExtras.PlasmoidHeading {
            RowLayout {
                anchors.fill: parent
                spacing: Kirigami.Units.smallSpacing

                Kirigami.Heading {
                    text: i18n("Cybou")
                    level: 2
                    Layout.fillWidth: true
                }

                PlasmaComponents.Label {
                    visible: root.ready
                    text: i18np("%1 record", "%1 records", mind.contributions)
                    opacity: 0.6
                    font: Kirigami.Theme.smallFont
                }
            }
        }

        // Nothing behind the panel: say so plainly instead of showing an empty frame.
        PlasmaExtras.PlaceholderMessage {
            anchors.centerIn: parent
            width: parent.width - Kirigami.Units.gridUnit * 4
            visible: !root.ready
            iconName: "dialog-cancel"
            text: i18n("Not awake")
            explanation: i18n("The journal could not be opened, so there is nothing to show.")
        }

        contentItem: PlasmaComponents.ScrollView {
            visible: root.ready

            ColumnLayout {
                width: parent.width
                spacing: Kirigami.Units.largeSpacing

                // What it can truthfully say about itself.
                PlasmaComponents.Label {
                    Layout.fillWidth: true
                    Layout.margins: Kirigami.Units.largeSpacing
                    text: mind.narration
                    wrapMode: Text.WordWrap
                    lineHeight: 1.3
                }

                // What it is attending to. Absent, not empty, when nothing is going on.
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: Kirigami.Units.largeSpacing
                    Layout.rightMargin: Kirigami.Units.largeSpacing
                    spacing: Kirigami.Units.smallSpacing
                    visible: mind.attention.length > 0

                    Kirigami.Heading { text: i18n("Attention"); level: 5; opacity: 0.7 }
                    PlasmaComponents.Label { Layout.fillWidth: true; text: mind.attention; wrapMode: Text.WordWrap }
                }

                // Obligations. The list is the point; an empty one is stated in words above.
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: Kirigami.Units.largeSpacing
                    Layout.rightMargin: Kirigami.Units.largeSpacing
                    spacing: Kirigami.Units.smallSpacing
                    visible: mind.obligations.length > 0

                    Kirigami.Heading { text: i18n("Open intentions"); level: 5; opacity: 0.7 }

                    Repeater {
                        model: mind.obligations
                        delegate: RowLayout {
                            required property string modelData
                            Layout.fillWidth: true
                            spacing: Kirigami.Units.smallSpacing
                            Kirigami.Icon { source: "checkbox"; width: Kirigami.Units.iconSizes.small; height: width }
                            PlasmaComponents.Label { Layout.fillWidth: true; text: parent.modelData; wrapMode: Text.WordWrap }
                        }
                    }
                }

                Kirigami.Separator { Layout.fillWidth: true; Layout.topMargin: Kirigami.Units.smallSpacing }

                // The biography, most recent first. Every line here is a row in the journal.
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: Kirigami.Units.largeSpacing
                    Layout.rightMargin: Kirigami.Units.largeSpacing
                    Layout.bottomMargin: Kirigami.Units.largeSpacing
                    spacing: Kirigami.Units.smallSpacing

                    Kirigami.Heading { text: i18n("Recently"); level: 5; opacity: 0.7 }

                    Repeater {
                        model: root.expanded ? mind.activity(12) : []
                        delegate: RowLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            spacing: Kirigami.Units.smallSpacing

                            PlasmaComponents.Label {
                                text: Qt.formatTime(parent.modelData.when, Qt.DefaultLocaleShortDate)
                                opacity: 0.5
                                font: Kirigami.Theme.smallFont
                            }
                            PlasmaComponents.Label {
                                Layout.fillWidth: true
                                text: parent.modelData.kind
                                elide: Text.ElideRight
                            }
                            PlasmaComponents.Label {
                                text: parent.modelData.organ
                                opacity: 0.5
                                font: Kirigami.Theme.smallFont
                            }
                        }
                    }
                }
            }
        }

        footer: PlasmaExtras.PlasmoidHeading {
            position: PlasmaExtras.PlasmoidHeading.Position.Footer
            visible: root.ready

            RowLayout {
                anchors.fill: parent
                spacing: Kirigami.Units.smallSpacing

                PlasmaComponents.TextField {
                    id: promiseField
                    Layout.fillWidth: true
                    placeholderText: i18n("Ask it to remember something…")
                    onAccepted: if (text.length > 0) { mind.promise(text); text = "" }
                }

                PlasmaComponents.Button {
                    icon.name: "list-add"
                    enabled: promiseField.text.length > 0
                    onClicked: { mind.promise(promiseField.text); promiseField.text = "" }
                }
            }
        }
    }
}
