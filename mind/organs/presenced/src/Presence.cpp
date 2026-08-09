// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/presence/Presence.h"

#include "cybou/fabric/OrganClients.h"
#include "cybou/runtime/StatePaths.h"

namespace cybou {

namespace {

QStringList toStringList(const QVariant &value)
{
    if (value.canConvert<QStringList>()) {
        const QStringList direct = value.toStringList();
        if (!direct.isEmpty() || value.toList().isEmpty()) {
            return direct;
        }
    }

    QStringList result;
    for (const QVariant &entry : value.toList()) {
        result.append(entry.toString());
    }
    return result;
}

} // namespace

Presence::Presence(QObject *parent)
    : QObject(parent)
    , m_client(std::make_unique<PresenceClient>())
{
    connect(
        m_client.get(),
        &PresenceClient::changed,
        this,
        [this]() {
            remoteChanged();
        });
}

Presence::~Presence() = default;

bool Presence::wake()
{
    m_lastError.clear();

    QString migrationError;
    if (!StatePaths::migrateLegacy(&migrationError)) {
        m_lastError =
            QStringLiteral("cannot migrate legacy Mind state: %1")
                .arg(migrationError);
        m_awake = false;
        Q_EMIT changed();
        return false;
    }

    if (!m_client->ready()) {
        m_lastError = m_client->lastError();
        m_awake = false;
        Q_EMIT changed();
        return false;
    }

    if (!refresh()) {
        m_awake = false;
        Q_EMIT changed();
        return false;
    }

    m_awake = true;
    Q_EMIT changed();
    return true;
}

bool Presence::refresh()
{
    const QVariantMap snapshot = m_client->snapshot();
    if (snapshot.isEmpty()) {
        m_lastError = m_client->lastError();
        return false;
    }

    m_snapshot = snapshot;
    m_lastError.clear();
    return true;
}

void Presence::remoteChanged()
{
    if (refresh()) {
        m_awake =
            m_snapshot.value(QStringLiteral("awake")).toBool();
        Q_EMIT changed();
    }
}

QString Presence::narration() const
{
    return m_snapshot
        .value(QStringLiteral("narration"))
        .toString();
}

QStringList Presence::obligations() const
{
    return toStringList(
        m_snapshot.value(QStringLiteral("obligations")));
}

QString Presence::attention() const
{
    return m_snapshot
        .value(QStringLiteral("attention"))
        .toString();
}

int Presence::contributions() const
{
    return m_snapshot
        .value(QStringLiteral("contributions"))
        .toInt();
}

QList<Moment> Presence::recent(int limit) const
{
    QList<Moment> result;
    for (const QVariant &entry : activity(limit)) {
        const QVariantMap map = entry.toMap();

        Moment moment;
        moment.when =
            map.value(QStringLiteral("when")).toDateTime();
        moment.organ =
            map.value(QStringLiteral("organ")).toString();
        moment.kind =
            map.value(QStringLiteral("kind")).toString();
        moment.thread =
            QUuid::fromString(
                map.value(QStringLiteral("thread")).toString());
        result.append(moment);
    }
    return result;
}

QVariantList Presence::activity(int limit) const
{
    if (!m_awake) {
        return {};
    }
    return m_client->activity(limit);
}

QUuid Presence::promise(const QString &description)
{
    if (!m_awake) {
        return {};
    }

    const QString id = m_client->promise(description);
    if (id.isEmpty()) {
        m_lastError = m_client->lastError();
        return {};
    }

    refresh();
    return QUuid::fromString(id);
}

bool Presence::reflect()
{
    if (!m_awake) {
        return false;
    }

    const bool ok = m_client->reflect();
    if (!ok) {
        m_lastError = m_client->lastError();
    } else {
        refresh();
    }
    return ok;
}

bool Presence::fulfillIndex(int index)
{
    if (!m_awake) {
        return false;
    }

    const bool ok = m_client->fulfillIndex(index);
    if (!ok) {
        m_lastError = m_client->lastError();
    } else {
        refresh();
    }
    return ok;
}

bool Presence::abandonIndex(int index)
{
    if (!m_awake) {
        return false;
    }

    const bool ok = m_client->abandonIndex(index);
    if (!ok) {
        m_lastError = m_client->lastError();
    } else {
        refresh();
    }
    return ok;
}

QVariantList Presence::detailedObligations() const
{
    if (!m_awake) {
        return {};
    }
    return m_client->detailedObligations();
}

bool Presence::observe(
    const QString &subject,
    double value)
{
    if (!m_awake) {
        return false;
    }

    const bool ok = m_client->observe(subject, value);
    if (!ok) {
        m_lastError = m_client->lastError();
    } else {
        refresh();
    }
    return ok;
}

QVariantMap Presence::stats() const
{
    return m_snapshot
        .value(QStringLiteral("stats"))
        .toMap();
}

QVariantMap Presence::identityState() const
{
    return m_snapshot
        .value(QStringLiteral("identityState"))
        .toMap();
}

QVariantList Presence::calibrations() const
{
    return m_snapshot
        .value(QStringLiteral("calibrations"))
        .toList();
}

QVariantMap Presence::predict(const QString &subject)
{
    if (!m_awake) {
        return {};
    }

    const QVariantMap prediction =
        m_client->predict(subject);
    if (prediction.isEmpty()) {
        m_lastError = m_client->lastError();
    } else {
        refresh();
    }
    return prediction;
}

QVariantList Presence::coalitions() const
{
    return m_snapshot
        .value(QStringLiteral("coalitions"))
        .toList();
}

QVariantMap Presence::moment() const
{
    return m_snapshot
        .value(QStringLiteral("moment"))
        .toMap();
}

QVariantMap Presence::organHealth() const
{
    return m_snapshot
        .value(QStringLiteral("organHealth"))
        .toMap();
}

QString Presence::lifecycleMode() const
{
    return m_snapshot.value(QStringLiteral("lifecycleMode")).toString();
}

QString Presence::lifecycleStatus() const
{
    return m_snapshot.value(QStringLiteral("lifecycleStatus")).toString();
}

QVariantMap Presence::lifecycleState() const
{
    return m_snapshot.value(QStringLiteral("lifecycleState")).toMap();
}

} // namespace cybou
