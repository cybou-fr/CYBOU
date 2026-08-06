// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import org.cybou.presence 1.0

MindTab {
    id: predictorTab
    title: "Predictor"
    icon: "predictive-text"

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

        // Calibrations list
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: mind.calibrations
            delegate: ItemDelegate {
                text: "%1: %2 settled, error: %3, bias: %4".arg(modelData.subject)
                                                                 .arg(modelData.settled)
                                                                 .arg(modelData.meanError.toFixed(2))
                                                                 .arg(modelData.bias.toFixed(2))
                onClicked: console.log("Calibration clicked:", modelData)
            }
        }

        // Prediction input
        RowLayout {
            Layout.fillWidth: true
            spacing: 11

            TextField {
                Layout.fillWidth: true
                placeholderText: "Enter subject to predict..."
                text: predictionSubject
                onTextChanged: predictionSubject = text
            }

            Button {
                text: "Predict"
                onClicked: predict()
            }
        }

        // Prediction result
        Label {
            Layout.alignment: Qt.AlignHCenter
            text: predictionResult
            font.pixelSize: 14
            wrapMode: Text.WordWrap
        }
    }

    property string predictionSubject: ""
    property string predictionResult: ""

    function predict() {
        if (predictionSubject) {
            const result = mind.predict(predictionSubject)
            predictionResult = "Subject: %1, Estimate: %2, Margin: %3, Confidence: %4, Samples: %5".arg(result.subject)
                                                                     .arg(result.estimate.toFixed(2))
                                                                     .arg(result.margin.toFixed(2))
                                                                     .arg(result.confidence.toFixed(2))
                                                                     .arg(result.samples)
        }
    }
}
