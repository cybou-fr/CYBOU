// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/fabric/FabricCodec.h"

#include "cybou/fabric/OrganBus.h"

#include <QCborMap>
#include <QCborValue>

namespace cybou {

namespace {

void setError(QString *error, const QString &message)
{
    if (error) {
        *error = message;
    }
}

} // namespace

QByteArray FabricCodec::encode(const QVariant &value)
{
    QCborMap root;
    root.insert(
        QStringLiteral("version"),
        static_cast<qint64>(kFabricIpcVersion));
    root.insert(
        QStringLiteral("value"),
        QCborValue::fromVariant(value));
    return root.toCborValue().toCbor();
}

QVariant FabricCodec::decode(
    const QByteArray &encoded,
    QString *error)
{
    if (error) {
        error->clear();
    }

    const QCborValue value = QCborValue::fromCbor(encoded);
    if (!value.isMap()) {
        setError(error, QStringLiteral("fabric payload is not a CBOR map"));
        return {};
    }

    const QCborMap root = value.toMap();
    if (root.value(QStringLiteral("version")).toInteger(-1)
        != kFabricIpcVersion) {
        setError(
            error,
            QStringLiteral("unsupported cognitive fabric payload version"));
        return {};
    }

    if (!root.contains(QStringLiteral("value"))) {
        setError(error, QStringLiteral("fabric payload has no value"));
        return {};
    }

    return root.value(QStringLiteral("value")).toVariant();
}

QVariantMap FabricCodec::decodeMap(
    const QByteArray &encoded,
    QString *error)
{
    const QVariant value = decode(encoded, error);
    if (error && !error->isEmpty()) {
        return {};
    }

    if (!value.canConvert<QVariantMap>()) {
        setError(error, QStringLiteral("fabric payload is not a map"));
        return {};
    }

    return value.toMap();
}

QVariantList FabricCodec::decodeList(
    const QByteArray &encoded,
    QString *error)
{
    const QVariant value = decode(encoded, error);
    if (error && !error->isEmpty()) {
        return {};
    }

    if (!value.canConvert<QVariantList>()) {
        setError(error, QStringLiteral("fabric payload is not a list"));
        return {};
    }

    return value.toList();
}

} // namespace cybou
