// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#include "cybou/presence/Presence.h"

#include <QCborMap>
#include <QCborValue>
#include <QDir>

namespace cybou {

Presence::Presence(const QString &dataDir, QObject *parent)
    : QObject(parent)
    , m_dataDir(dataDir)
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

} // namespace cybou
