// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.kde.kirigami as Kirigami
import "../utils"

Item {
    id: predictorTab

    required property var mind
    readonly property string title: "Predictor"
    readonly property string icon: "edit-find"

    property string predictionSubject: ""
    property var predictionResult: ({})

    function requestPrediction() {
        const subject = predictionSubject.trim()
        if (!subject)
            return

        const result = mind.predict(subject)
        predictionResult = result || ({})
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 8

        InfoCard {
            Layout.fillWidth: true
            title: i18n("Prediction")
            text: predictorTab.predictionResult.subject
                ? i18n(
                    "%1 ≈ %2 ± %3 · confidence %4 · %5 samples",
                    predictorTab.predictionResult.subject,
                    Number(predictorTab.predictionResult.estimate).toFixed(2),
                    Number(predictorTab.predictionResult.margin).toFixed(2),
                    Number(predictorTab.predictionResult.confidence).toFixed(2),
                    predictorTab.predictionResult.samples
                )
                : i18n("Choose a subject to ask the predictor.")
            icon: "edit-find"
            emphasized: Boolean(predictorTab.predictionResult.subject)
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 7

            TextField {
                Layout.fillWidth: true
                placeholderText: i18n("Subject")
                text: predictorTab.predictionSubject
                selectByMouse: true
                activeFocusOnTab: true
                onTextChanged: predictorTab.predictionSubject = text
                onAccepted: predictorTab.requestPrediction()
            }

            Button {
                text: i18n("Predict")
                icon.name: "go-next"
                activeFocusOnTab: true
                enabled: predictorTab.predictionSubject.trim().length > 0
                onClicked: predictorTab.requestPrediction()
            }
        }

        Label {
            Layout.fillWidth: true
            text: i18n("Calibration")
            font.pixelSize: 10
            font.bold: true
            opacity: 0.56
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ListView {
                id: calibrationList

                anchors.fill: parent
                clip: true
                spacing: 3
                model: mind.calibrations
                boundsBehavior: Flickable.StopAtBounds

                ScrollBar.vertical: ThinScrollBar {}

                delegate: ItemDelegate {
                    id: calibrationDelegate

                    required property var modelData

                    width: ListView.view.width
                    implicitHeight: 56
                    activeFocusOnTab: true

                    background: Rectangle {
                        radius: 8
                        color: calibrationDelegate.hovered
                            ? Kirigami.Theme.highlightColor
                            : "transparent"
                        opacity: calibrationDelegate.hovered ? 0.07 : 1.0

                        border.width: calibrationDelegate.activeFocus ? 1 : 0
                        border.color: Kirigami.Theme.focusColor
                    }

                    contentItem: ColumnLayout {
                        spacing: 2

                        Label {
                            Layout.fillWidth: true
                            text: modelData.subject || i18n("Unnamed subject")
                            font.bold: true
                            elide: Text.ElideRight
                        }

                        Label {
                            Layout.fillWidth: true
                            text: i18n(
                                "%1 settled · mean error %2 · bias %3",
                                modelData.settled,
                                Number(modelData.meanError).toFixed(2),
                                Number(modelData.bias).toFixed(2)
                            )
                            font.pixelSize: 10
                            opacity: 0.52
                            elide: Text.ElideRight
                        }
                    }
                }
            }

            Label {
                anchors.centerIn: parent
                visible: mind.calibrations.length === 0
                text: i18n("No settled predictions yet")
                opacity: 0.48
            }
        }
    }
}
