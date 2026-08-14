// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/eventd/EventService.h"

#include "cybou/events/EnvelopeCodec.h"
#include "cybou/events/EventStore.h"

#include <QCborMap>
#include <QCborValue>
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QDBusReply>
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QRegularExpression>
#include <QSaveFile>

#include <algorithm>

namespace cybou {

namespace {

QByteArray submitReply(quint64 sequence, const QString &error)
{
    QCborMap map;
    map.insert(QStringLiteral("sequence"), QString::number(sequence));
    map.insert(QStringLiteral("error"), error);
    return map.toCborValue().toCbor();
}

bool validConsumerId(const QString &consumerId)
{
    static const QRegularExpression pattern(
        QStringLiteral("^[a-z0-9][a-z0-9.-]{0,63}$"));
    return pattern.match(consumerId).hasMatch();
}

// The directory holding the Mind binaries, taken from eventd's own executable.
//
// Organs are installed together, so the genuine predictord is the one sitting beside the eventd
// that is doing the checking. Deriving this rather than configuring it matters: a trusted path read
// from the environment would be settable by any process able to restart the service, which is the
// same user this check exists to constrain.
//
// Empty when eventd cannot resolve itself, which makes every organ claim fail closed.
QString trustedOrganDirectory()
{
    static const QString directory = [] {
        const QString self = QFile::symLinkTarget(QStringLiteral("/proc/self/exe"));
        return self.isEmpty() ? QString() : QFileInfo(self).absolutePath();
    }();
    return directory;
}

// Map a Mind executable to the organ identity it is entitled to claim.
//
// The name alone is not enough, and an earlier version of this check trusted it. Any user can build
// an ELF, call it cybou-predictord, and put it in /tmp; matching on the basename let that process
// attribute contributions to the prediction organ, which made the provenance guarantee this
// function exists to provide considerably weaker than it was described as being.
//
// So the executable must also *be* the installed one: the same directory as eventd itself, compared
// on canonical paths that /proc resolution has already followed through symlinks. A user cannot
// write into that directory without already being able to replace Mind outright.
//
// The Nix build wraps Qt applications, so the running executable is `.cybou-identityd-wrapped`
// rather than `cybou-identityd`. Undoing that decoration is what makes this work against the
// installed package rather than only a development build.
QString organIdentityForExecutable(const QString &executablePath)
{
    if (executablePath.isEmpty()) {
        return {};
    }

    const QString trusted = trustedOrganDirectory();
    if (trusted.isEmpty() || QFileInfo(executablePath).absolutePath() != trusted) {
        return {};
    }

    QString name = QFileInfo(executablePath).fileName();
    if (name.startsWith(QLatin1Char('.'))) {
        name.remove(0, 1);
    }
    if (name.endsWith(QLatin1String("-wrapped"))) {
        name.chop(QStringLiteral("-wrapped").size());
    }
    if (!name.startsWith(QLatin1String("cybou-"))) {
        return {};
    }
    name.remove(0, QStringLiteral("cybou-").size());
    return reservedOrganIdentities().contains(name) ? name : QString();
}

} // namespace

QStringList reservedOrganIdentities()
{
    static const QStringList identities{
        QStringLiteral("eventd"),
        QStringLiteral("healthd"),
        QStringLiteral("lifecycled"),
        QStringLiteral("identityd"),
        QStringLiteral("intentiond"),
        QStringLiteral("predictord"),
        QStringLiteral("selfd"),
        QStringLiteral("workspaced"),
        QStringLiteral("presenced"),
        // The perception adapter. Reserved from the start: an identity that only becomes protected
        // once something claims it leaves a window in which anything may claim it first, and
        // provenance is the whole point of this organ.
        QStringLiteral("perceptiond"),
        // Reserved although it never writes: the projection derives and does not contribute. An
        // unreserved name is one anything may speak under, and a claim attributed to the organ that
        // decides what is known would be worth forging.
        QStringLiteral("epistemicd"),
    };
    return identities;
}

// Bind the claimed origin to the process that actually made the call.
//
// The obvious binding - require the caller to own the organ's well-known D-Bus name - does not
// work: identityd writes "session began" to Event1 from its constructor, before ServiceHost
// publishes its name. Enforcing name ownership would reject that and break identity continuity at
// startup, which is a worse failure than the forgery it prevents.
//
// So the binding is to the caller's executable. A same-user process cannot fake /proc/<pid>/exe
// without actually being that binary, and the answer does not depend on startup ordering.
QString EventService::callerOrganIdentity() const
{
    if (!calledFromDBus()) {
        return {};
    }

    const QString sender = message().service();
    if (sender.isEmpty()) {
        return {};
    }
    const auto cached = m_resolvedCallers.constFind(sender);
    if (cached != m_resolvedCallers.constEnd()) {
        return *cached;
    }

    QString identity;
    QDBusConnectionInterface *bus = connection().interface();
    if (bus) {
        // One bus round trip per connection, not per submission. eventd owns the only write path,
        // so asking the bus about the same peer on every append would put an avoidable cost on it.
        const QDBusReply<uint> pid = bus->servicePid(sender);
        if (pid.isValid()) {
            identity = organIdentityForExecutable(
                QFile::symLinkTarget(QStringLiteral("/proc/%1/exe").arg(pid.value())));
        }
    }

    m_resolvedCallers.insert(sender, identity);
    return identity;
}

bool EventService::originIsAuthentic(const QString &claimedOrigin) const
{
    // An in-process caller is the owning service itself, not a peer: unit tests construct
    // EventService directly and there is no sender to check against.
    if (!calledFromDBus()) {
        return true;
    }

    const QString caller = callerOrganIdentity();
    if (!caller.isEmpty()) {
        // A Mind organ may only speak as itself. This is the case that matters: it stops one organ,
        // or anything running an organ binary, from attributing a contribution to another.
        return claimedOrigin == caller;
    }

    // Everything else - tools, tests, future adapters that are not yet reserved identities - may
    // contribute under its own name, but may not borrow an organ's. Non-reserved origins stay open
    // deliberately: this closes impersonation, not authorship, and a general capability model is
    // still outstanding.
    return !reservedOrganIdentities().contains(claimedOrigin);
}

EventService::EventService(const QString &journalPath, QObject *parent)
    : QObject(parent)
    , m_journal(journalPath, QStringLiteral("cybou-eventd"))
    , m_offsetsPath(QFileInfo(journalPath).dir().filePath(
          QStringLiteral("consumer-offsets.json")))
    , m_checkpointPath(QFileInfo(journalPath).dir().filePath(
          QStringLiteral("verification-checkpoint.json")))
{
    m_offsetsReady = loadOffsets();
    loadCheckpoint();
    connect(
        &m_journal,
        &EventStore::accepted,
        this,
        [this](const CognitiveEnvelope &envelope, quint64 sequence) {
            Q_EMIT Accepted(
                EnvelopeCodec::encode(envelope),
                static_cast<qulonglong>(sequence));
        });
}

QString EventService::startupError() const
{
    return !m_journal.isOpen() ? m_journal.lastError() : m_offsetsError;
}

bool EventService::loadOffsets()
{
    m_offsetsError.clear();
    if (!QFile::exists(m_offsetsPath)) return true;
    QFile file(m_offsetsPath);
    if (!file.open(QIODevice::ReadOnly)) {
        m_offsetsError = QStringLiteral("cannot read consumer offsets: %1").arg(file.errorString());
        return false;
    }
    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()
        || document.object().value(QStringLiteral("version")).toInt() != 1
        || !document.object().value(QStringLiteral("offsets")).isObject()) {
        m_offsetsError = QStringLiteral("invalid consumer-offset state");
        return false;
    }
    const quint64 head = m_journal.count();
    QMap<QString, quint64> loaded;
    const QJsonObject offsets = document.object().value(QStringLiteral("offsets")).toObject();
    for (auto it = offsets.begin(); it != offsets.end(); ++it) {
        bool ok = false;
        const quint64 offset = it.value().toString().toULongLong(&ok);
        if (!validConsumerId(it.key()) || !ok || offset > head) {
            m_offsetsError = QStringLiteral("invalid consumer offset for %1").arg(it.key());
            return false;
        }
        loaded.insert(it.key(), offset);
    }
    m_offsets = loaded;
    return true;
}

bool EventService::saveOffsets(const QMap<QString, quint64> &offsets)
{
    QJsonObject encoded;
    for (auto it = offsets.cbegin(); it != offsets.cend(); ++it)
        encoded.insert(it.key(), QString::number(it.value()));
    QJsonObject root;
    root.insert(QStringLiteral("version"), 1);
    root.insert(QStringLiteral("offsets"), encoded);
    QSaveFile file(m_offsetsPath);
    if (!file.open(QIODevice::WriteOnly)
        || file.write(QJsonDocument(root).toJson(QJsonDocument::Compact)) < 0
        || !file.commit()) {
        m_offsetsError = QStringLiteral("cannot persist consumer offsets: %1")
                             .arg(file.errorString());
        return false;
    }
    m_offsetsError.clear();
    return true;
}

bool EventService::Ready() const
{
    return m_journal.isOpen();
}

int EventService::SchemaVersion() const
{
    return m_journal.databaseSchemaVersion();
}

QByteArray EventService::Submit(const QByteArray &encodedEnvelope)
{
    QString decodeError;
    const auto envelope =
        EnvelopeCodec::decode(encodedEnvelope, &decodeError);
    if (!envelope) {
        return submitReply(
            0,
            QStringLiteral("invalid Event1 proposal: %1").arg(decodeError));
    }

    if (!originIsAuthentic(envelope->originOrgan)) {
        return submitReply(
            0,
            QStringLiteral("origin %1 does not belong to the calling process")
                .arg(envelope->originOrgan));
    }

    // ADR-0028: submitting a contribution never authorizes an erasure.
    //
    // Erasure is a destructive storage operation, not a cognitive proposal, and the two must not
    // share a door. Refused here rather than deeper down because this is the door every organ
    // already has a key to - a proposal is not permission to execute, and if that rule lived
    // anywhere else it would eventually become an implementation detail of Submit().
    if (isErasureKind(envelope->kind)) {
        return submitReply(
            0,
            QStringLiteral("erasure is not a contribution; %1 must be requested explicitly")
                .arg(kindToString(envelope->kind)));
    }

    const quint64 sequence = m_journal.append(*envelope);
    return submitReply(
        sequence,
        sequence == 0 ? m_journal.lastError() : QString());
}

qulonglong EventService::Count() const
{
    return static_cast<qulonglong>(m_journal.count());
}

QByteArray EventService::Head() const
{
    return m_journal.head();
}

// A checkpoint that cannot be read is simply absent: verification then costs a full walk, which is
// correct, so there is nothing to fail about. It is never an error to lack an accelerator.
bool EventService::loadCheckpoint()
{
    m_checkpoint = {};
    QFile file(m_checkpointPath);
    if (!file.exists() || !file.open(QIODevice::ReadOnly)) {
        return false;
    }

    const QJsonObject root = QJsonDocument::fromJson(file.readAll()).object();
    if (root.value(QStringLiteral("version")).toInt() != 1) {
        return false;
    }

    bool ok = false;
    const quint64 sequence =
        root.value(QStringLiteral("sequence")).toString().toULongLong(&ok);
    const QByteArray hash = QByteArray::fromHex(
        root.value(QStringLiteral("hash")).toString().toLatin1());
    if (!ok || sequence == 0 || hash.isEmpty()) {
        return false;
    }

    m_checkpoint.sequence = sequence;
    m_checkpoint.hash = hash;
    m_checkpoint.verifiedAt = QDateTime::fromString(
        root.value(QStringLiteral("verifiedAt")).toString(), Qt::ISODateWithMs);
    return true;
}

void EventService::saveCheckpoint(const VerifiedCheckpoint &checkpoint)
{
    if (checkpoint.isEmpty()) {
        return;
    }

    QJsonObject root;
    root.insert(QStringLiteral("version"), 1);
    root.insert(QStringLiteral("sequence"), QString::number(checkpoint.sequence));
    root.insert(QStringLiteral("hash"), QString::fromLatin1(checkpoint.hash.toHex()));
    root.insert(
        QStringLiteral("verifiedAt"), checkpoint.verifiedAt.toString(Qt::ISODateWithMs));

    QSaveFile file(m_checkpointPath);
    if (file.open(QIODevice::WriteOnly)
        && file.write(QJsonDocument(root).toJson(QJsonDocument::Compact)) >= 0
        && file.commit()) {
        m_checkpoint = checkpoint;
    }
    // A checkpoint that fails to persist costs a full verification next time and nothing else, so
    // it is deliberately not an error the caller has to handle.
}

QByteArray EventService::VerifyIncremental()
{
    VerificationResult result = m_journal.verifyFrom(m_checkpoint);

    // A checkpoint that no longer describes this journal says nothing about the journal. Fall back
    // to the full walk rather than reporting damage that has not been established.
    if (result.status == VerificationStatus::CheckpointMismatch) {
        m_checkpoint = {};
        result = m_journal.verifyFrom({});
    }

    // Only advance the checkpoint on a chain that actually held. Advancing past a break would make
    // the next verification skip the very contribution that is wrong.
    if (result.intact()) {
        saveCheckpoint(m_journal.checkpointAtHead());
    }

    QCborMap reply;
    reply.insert(QStringLiteral("status"), verificationStatusToString(result.status));
    reply.insert(QStringLiteral("verifiedFrom"), QString::number(result.verifiedFrom));
    reply.insert(QStringLiteral("verifiedThrough"), QString::number(result.verifiedThrough));
    reply.insert(QStringLiteral("brokenAt"), QString::number(result.brokenAt));
    return reply.toCborValue().toCbor();
}

qulonglong EventService::Verify() const
{
    return static_cast<qulonglong>(m_journal.verify());
}

bool EventService::EnsureConsumer(const QString &consumerId, qulonglong initialOffset)
{
    if (!m_offsetsReady || !validConsumerId(consumerId)
        || initialOffset > m_journal.count()) return false;
    if (m_offsets.contains(consumerId)) return true;
    QMap<QString, quint64> candidate = m_offsets;
    candidate.insert(consumerId, initialOffset);
    if (!saveOffsets(candidate)) return false;
    m_offsets = candidate;
    return true;
}

bool EventService::AdvanceConsumer(const QString &consumerId, qulonglong offset)
{
    if (!m_offsetsReady || !m_offsets.contains(consumerId)
        || offset < m_offsets.value(consumerId) || offset > m_journal.count()) return false;
    if (offset == m_offsets.value(consumerId)) return true;
    QMap<QString, quint64> candidate = m_offsets;
    candidate[consumerId] = offset;
    if (!saveOffsets(candidate)) return false;
    m_offsets = candidate;
    return true;
}

QByteArray EventService::ConsumerBacklog(const QString &consumerId) const
{
    QCborMap result;
    result.insert(QStringLiteral("consumerId"), consumerId);
    const bool registered = validConsumerId(consumerId) && m_offsets.contains(consumerId);
    result.insert(QStringLiteral("registered"), registered);
    const quint64 head = m_journal.count();
    const quint64 offset = registered ? m_offsets.value(consumerId) : 0;
    result.insert(QStringLiteral("head"), QString::number(head));
    result.insert(QStringLiteral("offset"), QString::number(offset));
    quint64 backlog = registered ? head - offset : 0;
    // Consolidation must not count its own output as new input, so its backlog excludes
    // contributions carrying that capability scope. This used to decode one envelope per row of
    // backlog, which put an unbounded per-call cost on the process that owns the only write path -
    // the backlog grows with the biography, and any caller in the session could hold the writer
    // busy by asking repeatedly. One aggregate query answers the same question.
    if (registered && consumerId == QStringLiteral("lifecycle.consolidation")) {
        backlog = m_journal.countAfterExcludingCapability(offset, consumerId);
    }
    result.insert(QStringLiteral("backlog"), QString::number(backlog));
    return result.toCborValue().toCbor();
}

QByteArray EventService::Recent(int limit) const
{
    return EnvelopeCodec::encodeList(m_journal.recent(limit));
}

// Bounded by construction: a caller asking for more than kMaxReplayPage gets kMaxReplayPage. The
// cap is the difference between a paged protocol and Recent(0) wearing a cursor - without it a
// caller could still demand the whole biography in one reply and nothing would have improved.
QByteArray EventService::Replay(qulonglong afterSequence, int limit) const
{
    constexpr int kMaxReplayPage = 1000;
    const ContributionPage page = m_journal.after(
        static_cast<quint64>(afterSequence),
        limit <= 0 ? kMaxReplayPage : std::min(limit, kMaxReplayPage));

    QCborMap reply;
    reply.insert(QStringLiteral("ok"), page.ok);
    reply.insert(QStringLiteral("from"), QString::number(afterSequence));
    reply.insert(QStringLiteral("to"), QString::number(page.lastSequence));
    reply.insert(QStringLiteral("head"), QString::number(page.head));
    reply.insert(QStringLiteral("hasMore"), page.hasMore);
    reply.insert(
        QStringLiteral("envelopes"),
        QCborValue::fromVariant(QVariant(EnvelopeCodec::encodeList(page.envelopes))));
    return reply.toCborValue().toCbor();
}

QByteArray EventService::Episode(const QString &correlationId) const
{
    return EnvelopeCodec::encodeList(
        m_journal.episode(QUuid::fromString(correlationId)));
}

QByteArray EventService::AtSequence(qulonglong sequence) const
{
    const auto envelope = m_journal.atSequence(sequence);
    return envelope ? EnvelopeCodec::encode(*envelope) : QByteArray();
}

bool EventService::Contains(const QString &messageId) const
{
    return m_journal.contains(QUuid::fromString(messageId));
}

QByteArray EventService::Contribution(const QString &messageId) const
{
    const auto envelope =
        m_journal.contribution(QUuid::fromString(messageId));
    return envelope ? EnvelopeCodec::encode(*envelope) : QByteArray();
}

QByteArray EventService::EvidenceFor(const QString &messageId) const
{
    return EnvelopeCodec::encodeUuidList(
        m_journal.evidenceFor(QUuid::fromString(messageId)));
}

bool EventService::HasOutcomeFor(
    const QString &causeId,
    const QString &originOrgan) const
{
    return m_journal.hasOutcomeFor(
        QUuid::fromString(causeId),
        originOrgan);
}

} // namespace cybou
