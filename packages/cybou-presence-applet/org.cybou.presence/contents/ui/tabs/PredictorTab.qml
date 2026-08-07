// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import "../utils"

Item {
    id: predictorTab

    required property var mind
    readonly property string title: "Predictor"
    readonly property string icon: "predictive-text"

    property string predictionSubject: ""
    property string predictionResult: ""

    function requestPrediction() {
        const subject = predictionSubject.trim()
        if (!subject)
            return

        const result = mind.predict(subject)
        if (!result || !result.subject) {
            predictionResult = "No prediction available."
            return
        }

        predictionResult = "Subject: %1, Estimate: %2, Margin: %3, Confidence: %4, Samples: %5"
            .arg(result.subject)
            .arg(Number(result.estimate).toFixed(2))
            .arg(Number(result.margin).toFixed(2))
            .arg(Number(result.confidence).toFixed(2))
            .arg(result.samples)
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 11
        spacing: 11

        Label {
            Layout.alignment: Qt.AlignHCenter
            text: "Prediction Calibration"
            font.pixelSize: 18
            font.bold: true
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: mind.calibrations

            delegate: ItemDelegate {
                required property var modelData
                width: ListView.view.width
                text: "%1: %2 settled, error: %3, bias: %4"
                    .arg(modelData.subject)
                    .arg(modelData.settled)
                    .arg(Number(modelData.meanError).toFixed(2))
                    .arg(Number(modelData.bias).toFixed(2))
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 11

            TextField {
                Layout.fillWidth: true
                placeholderText: "Enter subject to predict..."
                text: predictorTab.predictionSubject
                onTextChanged: predictorTab.predictionSubject = text
                onAccepted: predictorTab.requestPrediction()
            }

            Button {
                text: "Predict"
                enabled: predictorTab.predictionSubject.trim().length > 0
                onClicked: predictorTab.requestPrediction()
            }
        }

        Label {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignHCenter
            text: predictorTab.predictionResult
            font.pixelSize: 14
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
        }
    }
}
