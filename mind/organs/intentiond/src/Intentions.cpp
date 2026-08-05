// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
// РУС: Реализация органа намерений. См. Intentions.h для архитектурного обзора.

#include "cybou/intentions/Intentions.h"

#include <QCborMap>
#include <QCborValue>
#include <QSet>

namespace cybou {

// РУС: Преобразование исхода в строку для payload. Важно для читаемости журнала.
QString resolutionToString(Resolution r)
{
    switch (r) {
    case Resolution::Fulfilled: return QStringLiteral("fulfilled");
    case Resolution::Abandoned: return QStringLiteral("abandoned");
    case Resolution::Obsolete:  return QStringLiteral("obsolete");
    }
    // РУС: По умолчанию — "abandoned". Безопасный вариант: лучше сказать «оставлено»,
    // РУС: чем молча считать выполненным.
    return QStringLiteral("abandoned");
}

Intentions::Intentions(Journal *journal)
    : m_journal(journal)
{
}

// РУС: form() — создание намерения. Ключевой момент:
// РУС:   correlationId = messageId — намерение открывает собственный эпизод.
// РУС:   causationId = messageId — намерение является корнем: оно не выведено,
// РУС:   а сформировано. Это своё собственное основание для существования.
QUuid Intentions::form(const QString &description, const QString &trigger)
{
    if (!m_journal || description.isEmpty()) {
        m_lastError = QStringLiteral("an intention needs a journal and a description");
        return {};
    }

    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    // Its own episode: everything that later happens because of this intention hangs off it.
    // РУС: Собственный эпизод: всё, что происходит из-за этого намерения,
    // РУС: объединяется в одну коалицию в рабочем пространстве.
    e.correlationId = e.messageId;
    e.originOrgan = QStringLiteral("intentiond");
    e.kind = ContributionKind::Intention;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.confidence = 1.0;
    e.privacy = PrivacyClass::Node;
    // An Intention is a root: it is formed, not derived, so it needs no causation. The type
    // check in CognitiveEnvelope::isValid would otherwise reject it, which is why evidence
    // carries the reason instead when there is one.
    // РУС: Intention — корень: оно сформировано, а не выведено. isValid()
    // РУС: требует причину для не-Observation, поэтому оно указывает на себя.

    QCborMap payload;
    payload[QStringLiteral("description")] = description;
    payload[QStringLiteral("trigger")] = trigger;
    e.payloadCbor = payload.toCborValue().toCbor();

    // Intention is not an Observation, so isValid() demands a cause or evidence. It names
    // itself: an intention is its own reason for existing until something supersedes it.
    // РУС: Намерение именует себя как причину — «я существую, потому что я
    // РУС: было сформировано», пока что-то не заменит его.
    e.causationId = e.messageId;

    if (m_journal->append(e) == 0) {
        m_lastError = m_journal->lastError();
        return {};
    }
    return e.messageId;
}

// РУС: close() — закрытие намерения. Создаёт Outcome:
// РУС:   correlationId = intentionId — остаётся в эпизоде намерения.
// РУС:   causationId = intentionId — именно это делает намерение закрытым:
// РУС:   open() ищет Outcome, у которых causationId = messageId намерения.
// РУС:   evidence = {intentionId} — связь для проверки цепочки.
bool Intentions::close(const QUuid &intentionId, Resolution resolution, const QString &note)
{
    if (!m_journal || intentionId.isNull()) {
        m_lastError = QStringLiteral("closing needs a journal and an intention");
        return false;
    }

    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = intentionId; // stays inside the intention's episode
    // РУС: Остаётся в эпизоде намерения — одна коалиция.
    e.causationId = intentionId;   // this is what removes it from the open list
    // РУС: Причина = намерение. Это то, что open() ищет для исключения.
    e.originOrgan = QStringLiteral("intentiond");
    e.kind = ContributionKind::Outcome;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.confidence = 1.0;
    e.privacy = PrivacyClass::Node;
    e.evidence = {intentionId};

    QCborMap payload;
    payload[QStringLiteral("resolution")] = resolutionToString(resolution);
    payload[QStringLiteral("note")] = note;
    e.payloadCbor = payload.toCborValue().toCbor();

    if (m_journal->append(e) == 0) {
        m_lastError = m_journal->lastError();
        return false;
    }
    return true;
}

// РУС: open() — свёртка журнала для вычисления открытых намерений.
// РУС: Алгоритм:
// РУС:   1. Проход 1: собрать множество closed — causationId всех Outcome.
// РУС:   2. Проход 2: собрать Intention, чей messageId не в closed.
// РУС:   3. Перевернуть: самое старое обязательство — сверху.
// РУС: O(N) по журналу — первое место для оптимизации (проекционная таблица),
// РУС: когда журнал вырастет.
QList<Intention> Intentions::open() const
{
    QList<Intention> result;
    if (!m_journal) {
        return result;
    }

    // Read the whole biography once and fold it: intentions formed, minus intentions whose
    // outcome names them. Cheap while the journal is small; when it stops being small this is
    // the first place a projection table earns its keep.
    // РУС: Читаем всю биографию за один проход и сворачиваем.
    const auto all = m_journal->recent(0);

    // РУС: Проход 1: собираем множество закрытых — causationId всех Outcome.
    QSet<QUuid> closed;
    for (const auto &e : all) {
        if (e.kind == ContributionKind::Outcome && !e.causationId.isNull()) {
            closed.insert(e.causationId);
        }
    }

    // РУС: Проход 2: собираем Intention, которых нет в множестве закрытых.
    for (const auto &e : all) {
        if (e.kind != ContributionKind::Intention || closed.contains(e.messageId)) {
            continue;
        }
        const QCborMap payload = QCborValue::fromCbor(e.payloadCbor).toMap();
        Intention i;
        i.id = e.messageId;
        i.description = payload[QStringLiteral("description")].toString();
        i.trigger = payload[QStringLiteral("trigger")].toString();
        i.formed = e.wallTime;
        result.append(i);
    }

    // recent() is newest first; an obligation list reads better oldest first - the thing you
    // have owed longest is the thing you should see at the top.
    // РУС: recent() возвращает новые сначала; обязательства читаются лучше
    // РУС: старые сначала — то, что должен дольше всего, должно быть сверху.
    std::reverse(result.begin(), result.end());
    return result;
}

} // namespace cybou
