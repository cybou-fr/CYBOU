// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import Cybou.Presence 1.0

MindTab {
    id: predictorTab
    title: "Predictor"
    icon: "predictive-text"

    Presence {
        id: presence
        onChanged: updateData()
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

        // Calibrations list
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: calibrations
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

    property QVariantList calibrations: presence.calibrations()
    property string predictionSubject: ""
    property string predictionResult: ""

    function updateData() {
        calibrations = presence.calibrations()
    }

    function predict() {
        if (predictionSubject) {
            const result = presence.predict(predictionSubject)
            predictionResult = "Subject: %1, Value: %2, Confidence: %3".arg(result.subject)
                                                                     .arg(result.value.toFixed(2))
                                                                     .arg(result.confidence.toFixed(2))
        }
    }

    Component.onCompleted: updateData()
}
