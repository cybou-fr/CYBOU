// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
// РУС: Реализация органа идентичности. См. Identity.h для архитектурного обзора.

#include "cybou/identity/Identity.h"

#include <QCborMap>
#include <QCborValue>
#include <QDir>
#include <QElapsedTimer>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSaveFile>

namespace cybou {

namespace {
// РУС: Версия архитектуры вшита в бинарник. При смене этой строки beginSession()
// РУС: обнаружит миграцию и запишет факт в журнал.
constexpr auto kArchitectureVersion = "presence-0.1";

// РУС: Монотонные миллисекунды для поля monotonicTime в CognitiveEnvelope.
// РУС: wallTime может прыгнуть (синхронизация NTP); monotonicTime — никогда.
quint64 monotonicMs()
{
    static QElapsedTimer timer;
    if (!timer.isValid()) {
        timer.start();
    }
    return static_cast<quint64>(timer.elapsed());
}
}

qint64 IdentityState::ageInDays() const
{
    return origin.isValid() ? origin.daysTo(QDateTime::currentDateTimeUtc()) : 0;
}

Identity::Identity(const QString &statePath, Journal *journal)
    : m_statePath(statePath)
    , m_journal(journal)
{
}

// РУС: Загрузка состояния из JSON-файла. Важно: повреждённый файл не заменяется
// РУС: молча. Если файл нечитаем, но существует — это ошибка, а не рождение.
bool Identity::load()
{
    QFile f(m_statePath);
    if (!f.exists()) {
        return false;
    }
    if (!f.open(QIODevice::ReadOnly)) {
        m_lastError = f.errorString();
        return false;
    }

    const QJsonObject o = QJsonDocument::fromJson(f.readAll()).object();
    IdentityState s;
    s.identityId = QUuid::fromString(o.value(QStringLiteral("identityId")).toString());
    s.origin = QDateTime::fromString(o.value(QStringLiteral("origin")).toString(),
                                     Qt::ISODateWithMs);
    s.sessionCount = static_cast<quint64>(
        o.value(QStringLiteral("sessionCount")).toInteger(0));
    s.architectureVersion = o.value(QStringLiteral("architectureVersion")).toString();

    if (!s.isValid()) {
        // Corrupt state is not silently replaced: a new identity would erase the claim that
        // this system is the same one, which is the single thing this organ exists to keep.
        // РУС: Повреждённое состояние НЕ перезаписывается молча: новая идентичность
        // РУС: уничтожила бы утверждение о непрерывности — то единственное,
        // РУС: ради чего этот орган существует.
        m_lastError = QStringLiteral("identity state is present but unreadable");
        return false;
    }
    m_state = s;
    return true;
}

// РУС: Атомарное сохранение через QSaveFile — гарантия, что полузаписанный файл
// РУС: не появится после сбоя питания. Либо старый, либо новый — никогда половина.
bool Identity::save() const
{
    QDir().mkpath(QFileInfo(m_statePath).absolutePath());

    QJsonObject o;
    o[QStringLiteral("identityId")] = m_state.identityId.toString(QUuid::WithoutBraces);
    o[QStringLiteral("origin")] = m_state.origin.toString(Qt::ISODateWithMs);
    o[QStringLiteral("sessionCount")] = static_cast<qint64>(m_state.sessionCount);
    o[QStringLiteral("architectureVersion")] = m_state.architectureVersion;

    // QSaveFile: a half-written identity after a power cut would be worse than none.
    // РУС: QSaveFile: пишет во временный файл, затем атомарный rename (commit).
    QSaveFile f(m_statePath);
    if (!f.open(QIODevice::WriteOnly)) {
        return false;
    }
    f.write(QJsonDocument(o).toJson(QJsonDocument::Indented));
    return f.commit();
}

// РУС: Запись вклада в журнал от имени органа идентичности.
// РУС: correlationId = identityId — вся жизнь субъекта образует один эпизод.
// РУС: confidence = 1.0 — факты об идентичности не вероятностны.
// РУС: privacy = Node — идентичность должна реплицироваться между узлами.
void Identity::record(ContributionKind kind, const QString &summary)
{
    if (!m_journal) {
        return;
    }
    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = m_state.identityId; // the whole life is one episode for identity
    // РУС: correlationId = identityId: все записи об идентичности образуют
    // РУС: одну коалицию в рабочем пространстве — «вся жизнь как один эпизод».
    e.originOrgan = QStringLiteral("identityd");
    e.kind = kind;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.monotonicTime = monotonicMs();
    e.confidence = 1.0;
    e.privacy = PrivacyClass::Node; // identity replicates; it is what makes a node the same one
    // РУС: privacy = Node: идентичность реплицируется — она делает узел тем же самым.

    QCborMap payload;
    payload[QStringLiteral("summary")] = summary;
    payload[QStringLiteral("session")] = static_cast<qint64>(m_state.sessionCount);
    payload[QStringLiteral("architecture")] = m_state.architectureVersion;
    e.payloadCbor = payload.toCborValue().toCbor();

    // An Observation needs no cause; a later Learning about migration would.
    // РУС: Observation не требует причины (causationId) — это точка входа мира в журнал.
    m_journal->append(e);
}

// РУС: Главный метод жизненного цикла — три ветки:
// РУС:   1. Рождение: файла нет, ошибок нет → создаём UUID + origin.
// РУС:   2. Продолжение: файл есть, архитектура та же → просто новая сессия.
// РУС:   3. Миграция: файл есть, архитектура другая → ритуал непрерывности из docs/14.
// РУС: Особый случай: файл существует, но нечитаем → отказ. Тихое создание новой
// РУС: идентичности поверх повреждённой — недопустимо.
bool Identity::beginSession()
{
    const bool existed = load();

    if (!existed && !m_lastError.isEmpty()) {
        // Unreadable rather than absent: refuse instead of quietly starting a new life.
        // РУС: Файл существует, но нечитаем — отказ, а не новая жизнь.
        return false;
    }

    if (!existed) {
        // РУС: Рождение: файла не было, ошибок нет — создаём новый субъект.
        m_state.identityId = QUuid::createUuid();
        m_state.origin = QDateTime::currentDateTimeUtc();
        m_state.sessionCount = 0;
        m_born = true;
    }

    const QString previousArchitecture = m_state.architectureVersion;
    m_state.architectureVersion = QString::fromLatin1(kArchitectureVersion);
    m_state.sessionCount += 1;

    if (!save()) {
        // РУС: Непрерывность, которая не записана на диск, не является непрерывностью.
        m_lastError = QStringLiteral("could not persist identity state");
        return false;
    }

    if (m_born) {
        // РУС: Ветка 1 — рождение: первая запись в биографии.
        record(ContributionKind::Observation,
               QStringLiteral("identity created"));
    } else if (!previousArchitecture.isEmpty()
               && previousArchitecture != m_state.architectureVersion) {
        // The architecture changed underneath the same identity. docs/14 calls this the
        // continuity ritual; recording it is the part that makes the claim checkable later.
        // РУС: Ветка 3 — миграция архитектуры: ритуал непрерывности.
        // РУС: Запись SelfAssessment делает утверждение проверяемым позже.
        record(ContributionKind::Observation,
               QStringLiteral("architecture changed from %1 to %2, identity preserved")
                   .arg(previousArchitecture, m_state.architectureVersion));
    } else {
        // РУС: Ветка 2 — обычное продолжение сессии.
        record(ContributionKind::Observation,
               QStringLiteral("session %1 began").arg(m_state.sessionCount));
    }

    return true;
}

} // namespace cybou
