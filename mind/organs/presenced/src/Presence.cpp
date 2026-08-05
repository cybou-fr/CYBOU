// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#include "cybou/presence/Presence.h"

#include <QCborMap>
#include <QCborValue>
#include <QDir>
#include <QStandardPaths>

namespace cybou {

Presence::Presence(const QString &dataDir, QObject *parent)
    : QObject(parent)
    , m_dataDir(dataDir)
{
}

Presence::Presence(QObject *parent)
    : Presence(QDir(QStandardPaths::writableLocation(QStandardPaths::AppDataLocation))
                   .filePath(QStringLiteral("cybou")),
               parent)
{
}

Presence::~Presence() = default;

bool Presence::wake()
{
    if (m_awake) {
        return true;
    }

    if (!QDir().mkpath(m_dataDir)) {
        m_lastError = QStringLiteral("cannot create %1").arg(m_dataDir);
        return false;
    }

    auto journal = std::make_unique<Journal>(QDir(m_dataDir).filePath(QStringLiteral("journal.db")));
    if (!journal->isOpen()) {
        m_lastError = journal->lastError();
        return false;
    }

    auto identity = std::make_unique<Identity>(
        QDir(m_dataDir).filePath(QStringLiteral("identity.json")), journal.get());
    if (!identity->beginSession()) {
        // Without continuity there is no one for the panel to be a presence *of*.
        m_lastError = identity->lastError();
        return false;
    }

    m_journal = std::move(journal);
    m_identity = std::move(identity);
    m_intentions = std::make_unique<Intentions>(m_journal.get());
    m_predictor = std::make_unique<Predictor>(m_journal.get());
    m_self = std::make_unique<SelfModel>(m_journal.get(), m_identity.get(), m_intentions.get(),
                                         m_predictor.get());
    m_workspace = std::make_unique<Workspace>(m_journal.get());
    m_workspace->rehydrate();

    // A shift of attention is the one thing the panel should react to without being asked.
    connect(m_workspace.get(), &Workspace::focusChanged, this, &Presence::changed);

    m_awake = true;
    Q_EMIT changed();
    return true;
}

QString Presence::narration() const
{
    if (!m_awake) {
        return {};
    }
    return m_self->narrate(m_self->measure());
}

QStringList Presence::obligations() const
{
    QStringList result;
    if (!m_awake) {
        return result;
    }
    const auto open = m_intentions->open(); // oldest first
    result.reserve(open.size());
    for (const Intention &i : open) {
        result.append(i.description);
    }
    return result;
}

QString Presence::attention() const
{
    if (!m_awake) {
        return {};
    }

    const Coalition f = m_workspace->focus();
    if (!f.isValid()) {
        // Nothing is going on. Saying so is better than filling the space with a generality.
        return {};
    }

    const CognitiveEnvelope &latest = f.members.last();
    const QStringList voices = f.organs();
    if (voices.size() > 1) {
        return QObject::tr("%1, with %n organ(s) involved", nullptr, voices.size())
            .arg(kindToString(latest.kind));
    }
    return QObject::tr("%1, from %2").arg(kindToString(latest.kind), latest.originOrgan);
}

int Presence::contributions() const
{
    return m_awake ? static_cast<int>(m_journal->count()) : 0;
}

QList<Moment> Presence::recent(int limit) const
{
    QList<Moment> result;
    if (!m_awake || limit <= 0) {
        return result;
    }

    const auto envelopes = m_journal->recent(limit); // newest first
    result.reserve(envelopes.size());
    for (const auto &e : envelopes) {
        Moment m;
        m.when = e.wallTime;
        m.organ = e.originOrgan;
        m.kind = kindToString(e.kind);
        m.thread = e.correlationId;
        result.append(m);
    }
    return result;
}

QVariantList Presence::activity(int limit) const
{
    QVariantList result;
    for (const Moment &m : recent(limit)) {
        QVariantMap entry;
        entry[QStringLiteral("when")] = m.when.toLocalTime();
        entry[QStringLiteral("organ")] = m.organ;
        entry[QStringLiteral("kind")] = m.kind;
        entry[QStringLiteral("thread")] = m.thread.toString(QUuid::WithoutBraces);
        result.append(entry);
    }
    return result;
}

QUuid Presence::promise(const QString &description)
{
    if (!m_awake) {
        return {};
    }
    const QUuid id = m_intentions->form(description, QStringLiteral("asked by the user"));
    if (!id.isNull()) {
        Q_EMIT changed();
    }
    return id;
}

bool Presence::reflect()
{
    if (!m_awake) {
        return false;
    }
    const SelfReport r = m_self->assess();
    if (!r.isValid()) {
        m_lastError = m_self->lastError();
        return false;
    }
    Q_EMIT changed();
    return true;
}

bool Presence::fulfillIndex(int index)
{
    if (!m_awake || !m_intentions) {
        return false;
    }
    const auto open = m_intentions->open();
    if (index < 0 || index >= open.size()) {
        return false;
    }
    bool ok = m_intentions->close(open.at(index).id, Resolution::Fulfilled);
    if (ok) {
        Q_EMIT changed();
    }
    return ok;
}

bool Presence::abandonIndex(int index)
{
    if (!m_awake || !m_intentions) {
        return false;
    }
    const auto open = m_intentions->open();
    if (index < 0 || index >= open.size()) {
        return false;
    }
    bool ok = m_intentions->close(open.at(index).id, Resolution::Abandoned);
    if (ok) {
        Q_EMIT changed();
    }
    return ok;
}

QVariantList Presence::detailedObligations() const
{
    QVariantList list;
    if (!m_awake || !m_intentions) {
        return list;
    }
    for (const Intention &i : m_intentions->open()) {
        QVariantMap map;
        map[QStringLiteral("id")] = i.id.toString(QUuid::WithoutBraces);
        map[QStringLiteral("description")] = i.description;
        map[QStringLiteral("trigger")] = i.trigger;
        map[QStringLiteral("formed")] = i.formed.toLocalTime();
        list.append(map);
    }
    return list;
}

bool Presence::observe(const QString &subject, double value)
{
    if (!m_awake || !m_predictor) {
        return false;
    }
    bool ok = m_predictor->observe(subject, value);
    if (ok) {
        Q_EMIT changed();
    }
    return ok;
}

QVariantMap Presence::stats() const
{
    QVariantMap map;
    if (!m_awake || !m_self) {
        return map;
    }
    const SelfReport r = m_self->measure();
    map[QStringLiteral("ageInDays")] = r.ageInDays;
    map[QStringLiteral("sessions")] = r.sessions;
    map[QStringLiteral("openIntentions")] = r.openIntentions;
    map[QStringLiteral("oldestObligationDays")] = r.oldestObligationDays;
    map[QStringLiteral("settledPredictions")] = r.settledPredictions;
    map[QStringLiteral("contributions")] = r.contributions;
    map[QStringLiteral("journalIntact")] = r.journalIntact;
    map[QStringLiteral("firstBrokenAt")] = r.firstBrokenAt;
    return map;
}

} // namespace cybou
