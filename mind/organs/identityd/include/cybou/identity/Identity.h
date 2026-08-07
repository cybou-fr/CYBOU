// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// РУС: ═══════════════════════════════════════════════════════════════════════════
// РУС: Орган идентичности — непрерывность субъекта через перезагрузки и
// РУС: архитектурные изменения.
// РУС:
// РУС: Идентичность — это НЕ база данных и НЕ UUID. Это утверждение, что субъект,
// РУС: который существовал вчера, — тот же самый, что существует сегодня. Журнал
// РУС: хранит биографию, а этот орган хранит указатель на владельца биографии.
// РУС:
// РУС: Орган не хранит воспоминаний и не принимает решений. Он отвечает на один
// РУС: вопрос: «Я всё ещё тот же?» — и записывает ответ в журнал, чтобы само
// РУС: утверждение имело доказательство.
// РУС:
// РУС: Состояние хранится в отдельном JSON-файле рядом с журналом. При каждом
// РУС: запуске (beginSession) записывается вклад в журнал — «рождение», «продолжение
// РУС: сессии» или «миграция архитектуры». Если файл повреждён, орган отказывается
// РУС: работать, а НЕ создаёт новую идентичность — потеря идентичности означает
// РУС: потерю непрерывности, а это единственное, что этот орган существует защищать.
// РУС: ═══════════════════════════════════════════════════════════════════════════
//
// Continuity of the subject.
//
// docs/14-mind-architecture.md: identity is not the database and not the id. It is the fact
// that the same subject persists across reboots and across architectural change, carrying its
// biography with it.
//
// This organ holds no memories and makes no decisions. It answers one question - "am I still
// the same?" - and records the answer in the journal so the claim itself has evidence.

#pragma once

#include "cybou/events/EventStore.h"

#include <QDateTime>
#include <QString>
#include <QUuid>

namespace cybou {

// РУС: Снимок идентичности: UUID субъекта, момент рождения, счётчик сессий,
// РУС: версия архитектуры. Сериализуется в JSON и читается обратно при каждом
// РУС: запуске. Минимальное состояние — всё остальное живёт в журнале.
struct IdentityState {
    // РУС: Уникальный идентификатор субъекта. Создаётся один раз при рождении.
    QUuid identityId;
    /// When this identity first existed. Never rewritten.
    // РУС: Момент рождения — устанавливается единожды и никогда не перезаписывается.
    QDateTime origin;
    /// How many times the system has come up as this identity.
    // РУС: Счётчик сессий — монотонно растёт при каждом запуске.
    quint64 sessionCount{0};
    /// The architecture that last wrote this state, so a migration can be detected.
    // РУС: Версия архитектуры — позволяет обнаружить миграцию при следующем запуске.
    QString architectureVersion;

    // РУС: Состояние валидно, только если есть и UUID, и дата рождения.
    bool isValid() const { return !identityId.isNull() && origin.isValid(); }

    /// How long this identity has existed, in days. What "I have been here since" means.
    // РУС: Возраст в днях — сколько субъект существует с момента рождения.
    qint64 ageInDays() const;
};

// РУС: Класс Identity — орган идентичности. Владеет путём к файлу состояния и
// РУС: указателем на журнал. Не управляет временем жизни журнала (журнал принадлежит
// РУС: Presence). Все записи помечаются originOrgan = "identityd".
class Identity
{
public:
    /// `statePath` is a small file beside the journal. The journal is the biography; this is
    /// only the pointer that says whose biography it is.
    // РУС: statePath — маленький файл рядом с журналом. Журнал — биография, а файл —
    // РУС: только указатель на то, чья это биография.
    Identity(const QString &statePath, EventStore *journal);

    /// Loads existing state, or creates it on first run. Increments the session counter and
    /// writes one contribution to the journal either way, because "I woke up" is an event and
    /// "I was born" is a different event.
    ///
    /// Returns false only when the state cannot be persisted - continuity that is not written
    /// down is not continuity.
    // РУС: beginSession() — главный метод жизненного цикла:
    // РУС:   • Файла нет → рождение: создаёт UUID + origin, пишет Observation «identity created».
    // РУС:   • Файл есть, архитектура та же → продолжение: увеличивает счётчик, пишет
    // РУС:     Observation «session N began».
    // РУС:   • Файл есть, архитектура другая → миграция: пишет SelfAssessment «architecture
    // РУС:     changed from X to Y, identity preserved» — ритуал непрерывности из docs/14.
    // РУС:   Возвращает false, если состояние не удалось сохранить — непрерывность, которая
    // РУС:   не записана, не является непрерывностью.
    bool beginSession();

    IdentityState state() const { return m_state; }

    /// True when this run created the identity rather than continuing one.
    bool wasBorn() const { return m_born; }

    QString lastError() const { return m_lastError; }

private:
    // РУС: load() — читает JSON-файл состояния. Если файл повреждён, возвращает false
    // РУС: и устанавливает ошибку. Повреждённое состояние НЕ заменяется молча —
    // РУС: новая идентичность стёрла бы утверждение о непрерывности.
    bool load();
    // РУС: save() — атомарная запись через QSaveFile. Половина записанного файла после
    // РУС: сбоя питания хуже, чем отсутствие файла.
    bool save() const;
    // РУС: record() — создаёт CognitiveEnvelope с originOrgan="identityd",
    // РУС: correlationId=identityId (вся жизнь — один эпизод для идентичности),
    // РУС: privacy=Node (идентичность реплицируется — это то, что делает узел тем же).
    void record(ContributionKind kind, const QString &summary);

    QString m_statePath;
    EventStore *m_events;
    IdentityState m_state;
    // РУС: m_born — true, если текущий запуск создал идентичность, а не продолжил.
    bool m_born{false};
    QString m_lastError;
};

} // namespace cybou
