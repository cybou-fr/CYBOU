// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

// РУС: ═══════════════════════════════════════════════════════════════════════════
// РУС: Реализация протокольного слоя — сериализация, валидация, разрешение приватности.
// РУС:
// РУС: Этот файл не содержит бизнес-логики органов. Он обеспечивает:
// РУС: 1. Преобразование перечислений в строки и обратно (для Журнала и отладки).
// РУС: 2. Валидацию конверта перед записью — единственный рубеж между
// РУС:    «мыслью» органа и её увековечиванием в хеш-цепочке Журнала.
// РУС: 3. Вычисление унаследованной приватности — защита от утечки через обобщение.
// РУС: ═══════════════════════════════════════════════════════════════════════════

#include "cybou/protocol/CognitiveEnvelope.h"

namespace cybou {

// РУС: Функция названа kindToString, а не toString, из-за ADL-конфликта:
// РУС: QCOMPARE ищет toString() через ADL для сообщений об ошибках и требует
// РУС: char*. Если бы в этом namespace была функция toString, возвращающая QString,
// РУС: все тесты, сравнивающие эти типы, перестали бы компилироваться.
QString kindToString(ContributionKind kind)
{
    switch (kind) {
    case ContributionKind::Observation:        return QStringLiteral("observation");
    case ContributionKind::BeliefRevision:     return QStringLiteral("belief-revision");
    case ContributionKind::Hypothesis:         return QStringLiteral("hypothesis");
    case ContributionKind::MemoryRecall:       return QStringLiteral("memory-recall");
    case ContributionKind::NeedSignal:         return QStringLiteral("need-signal");
    case ContributionKind::AttentionCandidate: return QStringLiteral("attention-candidate");
    case ContributionKind::Prediction:         return QStringLiteral("prediction");
    case ContributionKind::PlanProposal:       return QStringLiteral("plan-proposal");
    case ContributionKind::Objection:          return QStringLiteral("objection");
    case ContributionKind::Decision:           return QStringLiteral("decision");
    case ContributionKind::Intention:          return QStringLiteral("intention");
    case ContributionKind::Outcome:            return QStringLiteral("outcome");
    case ContributionKind::SelfAssessment:     return QStringLiteral("self-assessment");
    case ContributionKind::Learning:           return QStringLiteral("learning");
    }
    // РУС: Неизвестный вид — безопасное значение, не вызывающее паники.
    return QStringLiteral("unknown");
}

QString privacyToString(PrivacyClass privacy)
{
    switch (privacy) {
    case PrivacyClass::Local:     return QStringLiteral("local");
    case PrivacyClass::Node:      return QStringLiteral("node");
    case PrivacyClass::Household: return QStringLiteral("household");
    case PrivacyClass::Public:    return QStringLiteral("public");
    }
    // РУС: Fail-closed: неизвестное числовое значение → «local» (максимальная защита).
    return QStringLiteral("local");
}

// РУС: Обратное преобразование из строки Журнала в перечисление.
// РУС: Ключевой момент: всё, что не распознано явно, возвращается как Local.
// РУС: Это и есть «fail-closed» — приватность закрыта по умолчанию.
// РУС: Даже если в будущем появятся новые уровни, старый код НЕ откроет данные
// РУС: случайно — он просто увидит их как Local.
PrivacyClass privacyFromString(const QString &text)
{
    if (text == QLatin1String("public")) {
        return PrivacyClass::Public;
    }
    if (text == QLatin1String("household")) {
        return PrivacyClass::Household;
    }
    if (text == QLatin1String("node")) {
        return PrivacyClass::Node;
    }
    // Everything else, including an empty or unrecognised value: fail closed.
    // РУС: Пустая строка, опечатка, будущий неизвестный уровень → Local.
    return PrivacyClass::Local;
}

// РУС: Валидация — последний рубеж перед вечностью.
// РУС: Журнал — append-only с хеш-цепочкой; записанное нельзя удалить или изменить.
// РУС: Поэтому isValid() проверяет не формат, а семантическую целостность:
// РУС: можно ли ОСМЫСЛЕННО записать этот конверт и потом прочитать обратно.
bool CognitiveEnvelope::isValid() const
{
    // РУС: Минимальная идентификация: кто, когда, и уникальный ID.
    if (messageId.isNull() || originOrgan.isEmpty() || !wallTime.isValid()) {
        return false;
    }
    // A contribution that is not a root observation must say what caused it. Without this the
    // causal graph silently grows orphans, and an explanation cannot be traced.
    // РУС: Каузальная целостность: если это не наблюдение (не корень), должна быть
    // РУС: либо прямая причина (causationId), либо список доказательств (evidence).
    // РУС: Без этого граф причинности обрастает «сиротами» — утверждениями,
    // РУС: которые невозможно проследить до реальности.
    if (kind != ContributionKind::Observation && causationId.isNull() && evidence.isEmpty()) {
        return false;
    }
    // РУС: Уверенность — число от 0.0 (полное незнание) до 1.0 (прямое наблюдение).
    return confidence >= 0.0 && confidence <= 1.0;
}

// РУС: Вычисление эффективной приватности — свёртка (fold) по всем доказательствам.
// РУС: Начинаем с собственного класса приватности вклада и «ужесточаем» его
// РУС: до самого закрытого из доказательств. Это предотвращает «утечку через
// РУС: обобщение»: если орган делает публичный вывод из локальных данных датчика,
// РУС: derivedPrivacy() вернёт Local, а не Public.
PrivacyClass CognitiveEnvelope::derivedPrivacy(const QList<PrivacyClass> &evidencePrivacy) const
{
    // РУС: Начальное значение — собственный класс приватности конверта.
    PrivacyClass result = privacy;
    // РУС: Проходим по всем доказательствам и выбираем наиболее строгий класс.
    for (const PrivacyClass p : evidencePrivacy) {
        result = mostRestrictive(result, p);
    }
    return result;
}

} // namespace cybou
