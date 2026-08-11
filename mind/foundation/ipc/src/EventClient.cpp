// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/ipc/EventClient.h"

#include "cybou/events/EnvelopeCodec.h"
#include "cybou/events/EventBus.h"

#include <QCborMap>
#include <QCborValue>
#include <QDBusPendingCall>

namespace cybou {

namespace {

constexpr int kCallTimeoutMs = 5000;

QString text(const char *value)
{
    return QString::fromLatin1(value);
}

} // namespace

EventClient::EventClient(QObject *parent)
    : EventStore(parent)
    , m_bus(QDBusConnection::sessionBus())
{
    if (!m_bus.isConnected()) {
        m_lastError = QStringLiteral("the user D-Bus session is unavailable");
        return;
    }

    const bool connected = m_bus.connect(
        text(kEventServiceName),
        text(kEventObjectPath),
        text(kEventInterfaceName),
        QStringLiteral("Accepted"),
        this,
        SLOT(onAccepted(QByteArray,qulonglong)));

    if (!connected) {
        m_lastError = QStringLiteral("cannot subscribe to the Event1 accepted stream");
    }
}

QDBusMessage EventClient::call(
    const QString &method,
    const QVariantList &arguments,
    int timeoutMs) const
{
    if (!m_bus.isConnected()) {
        m_lastError = QStringLiteral("the user D-Bus session is unavailable");
        return {};
    }

    QDBusMessage message = QDBusMessage::createMethodCall(
        text(kEventServiceName),
        text(kEventObjectPath),
        text(kEventInterfaceName),
        method);
    message.setArguments(arguments);

    const int boundedTimeout = timeoutMs > 0 ? timeoutMs : kCallTimeoutMs;
    QDBusPendingCall pending = m_bus.asyncCall(message, boundedTimeout);
    pending.waitForFinished();
    const QDBusMessage reply = pending.reply();

    if (reply.type() == QDBusMessage::ErrorMessage) {
        m_lastError = QStringLiteral("%1: %2")
                          .arg(reply.errorName(), reply.errorMessage());
    }
    return reply;
}

QByteArray EventClient::callBytes(
    const QString &method,
    const QVariantList &arguments,
    int timeoutMs) const
{
    const QDBusMessage reply = call(method, arguments, timeoutMs);
    if (reply.type() == QDBusMessage::ErrorMessage || reply.arguments().isEmpty()) {
        return {};
    }
    return reply.arguments().first().toByteArray();
}

bool EventClient::isOpen() const
{
    return isOpen(-1);
}

bool EventClient::isOpen(int timeoutMs) const
{
    m_lastError.clear();
    const QDBusMessage reply = call(QStringLiteral("Ready"), {}, timeoutMs);
    if (reply.type() == QDBusMessage::ErrorMessage || reply.arguments().isEmpty()) {
        return false;
    }
    return reply.arguments().first().toBool();
}

int EventClient::databaseSchemaVersion() const
{
    m_lastError.clear();
    const QDBusMessage reply = call(QStringLiteral("SchemaVersion"));
    if (reply.type() == QDBusMessage::ErrorMessage || reply.arguments().isEmpty()) {
        return 0;
    }
    return reply.arguments().first().toInt();
}

quint64 EventClient::append(const CognitiveEnvelope &envelope)
{
    return append(envelope, -1);
}

quint64 EventClient::append(const CognitiveEnvelope &envelope, int timeoutMs)
{
    m_lastError.clear();
    const QByteArray replyBytes = callBytes(
        QStringLiteral("Submit"),
        {EnvelopeCodec::encode(envelope)},
        timeoutMs);
    if (replyBytes.isEmpty()) {
        return 0;
    }

    const QCborValue value = QCborValue::fromCbor(replyBytes);
    if (!value.isMap()) {
        m_lastError = QStringLiteral("eventd returned an invalid Submit reply");
        return 0;
    }

    const QCborMap map = value.toMap();
    bool ok = false;
    const quint64 sequence =
        map.value(QStringLiteral("sequence")).toString().toULongLong(&ok);
    if (!ok) {
        m_lastError = QStringLiteral("eventd returned an invalid sequence");
        return 0;
    }

    const QString error = map.value(QStringLiteral("error")).toString();
    if (sequence == 0) {
        m_lastError =
            error.isEmpty() ? QStringLiteral("eventd rejected the contribution") : error;
    }
    return sequence;
}

quint64 EventClient::count() const
{
    return count(-1);
}

quint64 EventClient::count(int timeoutMs) const
{
    m_lastError.clear();
    const QDBusMessage reply = call(QStringLiteral("Count"), {}, timeoutMs);
    if (reply.type() == QDBusMessage::ErrorMessage || reply.arguments().isEmpty()) {
        return 0;
    }
    return reply.arguments().first().toULongLong();
}

QByteArray EventClient::head() const
{
    m_lastError.clear();
    return callBytes(QStringLiteral("Head"));
}

quint64 EventClient::verify() const
{
    m_lastError.clear();
    const QDBusMessage reply = call(QStringLiteral("Verify"));
    if (reply.type() == QDBusMessage::ErrorMessage || reply.arguments().isEmpty()) {
        return 1;
    }
    return reply.arguments().first().toULongLong();
}

bool EventClient::ensureConsumer(const QString &consumerId, quint64 initialOffset) const
{
    m_lastError.clear();
    const QDBusMessage reply = call(
        QStringLiteral("EnsureConsumer"),
        {consumerId, QVariant::fromValue<qulonglong>(initialOffset)});
    return reply.type() != QDBusMessage::ErrorMessage && !reply.arguments().isEmpty()
        && reply.arguments().first().toBool();
}

bool EventClient::advanceConsumer(const QString &consumerId, quint64 offset) const
{
    m_lastError.clear();
    const QDBusMessage reply = call(
        QStringLiteral("AdvanceConsumer"),
        {consumerId, QVariant::fromValue<qulonglong>(offset)});
    return reply.type() != QDBusMessage::ErrorMessage && !reply.arguments().isEmpty()
        && reply.arguments().first().toBool();
}

std::optional<quint64> EventClient::consumerBacklog(const QString &consumerId) const
{
    m_lastError.clear();
    const QByteArray encoded = callBytes(QStringLiteral("ConsumerBacklog"), {consumerId});
    const QCborValue value = QCborValue::fromCbor(encoded);
    if (!value.isMap() || !value.toMap().value(QStringLiteral("registered")).toBool()) {
        m_lastError = QStringLiteral("Event1 consumer is not registered");
        return std::nullopt;
    }
    bool ok = false;
    const quint64 backlog = value.toMap().value(QStringLiteral("backlog"))
                                .toString().toULongLong(&ok);
    if (!ok) {
        m_lastError = QStringLiteral("Event1 returned an invalid consumer backlog");
        return std::nullopt;
    }
    return backlog;
}

ContributionPage EventClient::after(quint64 afterSequence, int limit) const
{
    return after(afterSequence, limit, -1);
}

ContributionPage EventClient::after(quint64 afterSequence, int limit, int timeoutMs) const
{
    m_lastError.clear();
    ContributionPage page;

    const QByteArray replyBytes = callBytes(
        QStringLiteral("Replay"),
        {static_cast<qulonglong>(afterSequence), limit},
        timeoutMs);
    if (replyBytes.isEmpty()) {
        // An empty reply is a transport failure, not an empty page. Reporting it as the end of
        // history would let a caller rebuild state from a prefix and believe it was complete.
        if (m_lastError.isEmpty())
            m_lastError = QStringLiteral("eventd did not answer Replay");
        return page;
    }

    const QCborValue value = QCborValue::fromCbor(replyBytes);
    if (!value.isMap() || !value.toMap().value(QStringLiteral("ok")).toBool()) {
        m_lastError = QStringLiteral("eventd returned an invalid Replay page");
        return page;
    }

    const QCborMap map = value.toMap();
    page.lastSequence = map.value(QStringLiteral("to")).toString().toULongLong();
    page.head = map.value(QStringLiteral("head")).toString().toULongLong();
    page.hasMore = map.value(QStringLiteral("hasMore")).toBool();

    QString decodeError;
    page.envelopes = EnvelopeCodec::decodeList(
        map.value(QStringLiteral("envelopes")).toByteArray(), &decodeError);
    if (!decodeError.isEmpty()) {
        m_lastError = decodeError;
        page.envelopes.clear();
        return page;
    }

    page.ok = true;
    return page;
}

QList<CognitiveEnvelope> EventClient::recent(int limit) const
{
    return recent(limit, -1);
}

QList<CognitiveEnvelope> EventClient::recent(int limit, int timeoutMs) const
{
    m_lastError.clear();
    const QByteArray encoded =
        callBytes(QStringLiteral("Recent"), {limit}, timeoutMs);
    if (encoded.isEmpty() && !m_lastError.isEmpty()) {
        return {};
    }

    QString decodeError;
    const auto result = EnvelopeCodec::decodeList(encoded, &decodeError);
    if (!decodeError.isEmpty()) {
        m_lastError = decodeError;
    }
    return result;
}

QList<CognitiveEnvelope> EventClient::episode(const QUuid &correlationId) const
{
    m_lastError.clear();
    if (correlationId.isNull()) {
        return {};
    }

    const QByteArray encoded = callBytes(
        QStringLiteral("Episode"),
        {correlationId.toString(QUuid::WithoutBraces)});
    if (encoded.isEmpty() && !m_lastError.isEmpty()) {
        return {};
    }

    QString decodeError;
    const auto result = EnvelopeCodec::decodeList(encoded, &decodeError);
    if (!decodeError.isEmpty()) {
        m_lastError = decodeError;
    }
    return result;
}

std::optional<CognitiveEnvelope> EventClient::atSequence(quint64 sequence) const
{
    m_lastError.clear();
    if (sequence == 0) return std::nullopt;
    const QByteArray encoded = callBytes(
        QStringLiteral("AtSequence"), {QVariant::fromValue<qulonglong>(sequence)});
    if (encoded.isEmpty()) return std::nullopt;
    QString error;
    const auto result = EnvelopeCodec::decode(encoded, &error);
    if (!result) m_lastError = error;
    return result;
}

bool EventClient::contains(const QUuid &messageId) const
{
    m_lastError.clear();
    if (messageId.isNull()) {
        return false;
    }

    const QDBusMessage reply = call(
        QStringLiteral("Contains"),
        {messageId.toString(QUuid::WithoutBraces)});
    if (reply.type() == QDBusMessage::ErrorMessage || reply.arguments().isEmpty()) {
        return false;
    }
    return reply.arguments().first().toBool();
}

std::optional<CognitiveEnvelope> EventClient::contribution(
    const QUuid &messageId) const
{
    m_lastError.clear();
    if (messageId.isNull()) {
        return std::nullopt;
    }

    const QByteArray encoded = callBytes(
        QStringLiteral("Contribution"),
        {messageId.toString(QUuid::WithoutBraces)});
    if (encoded.isEmpty()) {
        return std::nullopt;
    }

    QString decodeError;
    const auto result = EnvelopeCodec::decode(encoded, &decodeError);
    if (!result) {
        m_lastError = decodeError;
    }
    return result;
}

QList<QUuid> EventClient::evidenceFor(const QUuid &messageId) const
{
    m_lastError.clear();
    if (messageId.isNull()) {
        return {};
    }

    const QByteArray encoded = callBytes(
        QStringLiteral("EvidenceFor"),
        {messageId.toString(QUuid::WithoutBraces)});
    if (encoded.isEmpty() && !m_lastError.isEmpty()) {
        return {};
    }

    QString decodeError;
    const auto result = EnvelopeCodec::decodeUuidList(encoded, &decodeError);
    if (!decodeError.isEmpty()) {
        m_lastError = decodeError;
    }
    return result;
}

bool EventClient::hasOutcomeFor(
    const QUuid &causeId,
    const QString &originOrgan) const
{
    m_lastError.clear();
    if (causeId.isNull()) {
        return false;
    }

    const QDBusMessage reply = call(
        QStringLiteral("HasOutcomeFor"),
        {
            causeId.toString(QUuid::WithoutBraces),
            originOrgan,
        });
    if (reply.type() == QDBusMessage::ErrorMessage || reply.arguments().isEmpty()) {
        return false;
    }
    return reply.arguments().first().toBool();
}

void EventClient::onAccepted(
    const QByteArray &encodedEnvelope,
    qulonglong sequence)
{
    QString error;
    const auto envelope = EnvelopeCodec::decode(encodedEnvelope, &error);
    if (!envelope) {
        m_lastError = QStringLiteral("invalid Event1 Accepted payload: %1").arg(error);
        return;
    }

    Q_EMIT accepted(*envelope, static_cast<quint64>(sequence));
}

} // namespace cybou
