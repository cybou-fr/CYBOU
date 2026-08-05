// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// РУС: ═══════════════════════════════════════════════════════════════════════════
// РУС: КОГНИТИВНЫЙ КОНВЕРТ — единый язык общения всех «органов» мозга Cybou.
// РУС:
// РУС: Архитектура Cybou Mind — это ансамбль равноправных органов (identityd,
// РУС: intentiond, predictord, selfd, workspaced, presenced). Ни один из них
// РУС: не является «центральным». Все они общаются между собой исключительно
// РУС: через типизированные вклады (contributions), упакованные в конверт.
// РУС:
// РУС: Конверт — это не сообщение в очереди. Это когнитивный акт, зафиксированный
// РУС: в Журнале. Каждый вклад несёт: кто его произвёл, что его вызвало, на каких
// РУС: данных он основан, и насколько орган уверен в своём выводе. Ни один вклад
// РУС: не может существовать без указания причины — кроме корневых наблюдений,
// РУС: которые по определению являются непосредственным восприятием реальности.
// РУС:
// РУС: Два инварианта системы типов (а не конвенции кода):
// РУС: 1. Каузальная цепочка: каждый вклад ссылается на причину и доказательства,
// РУС:    поэтому любое утверждение прослеживается до наблюдений.
// РУС: 2. Приватность «закрыта по умолчанию»: неизвестный класс = Local.
// РУС:    Строка — это комментарий, пока что-то не откажется действовать по ней.
// РУС: ═══════════════════════════════════════════════════════════════════════════
//
// The shared language of the Cybou Mind: typed cognitive contributions, never free text.
//
// docs/14-mind-architecture.md is the reference. Two rules are enforced by the types rather
// than by convention:
//
//   1. A contribution carries its causation and its evidence, so any claim can be traced back
//      to the observations it rests on.
//   2. PrivacyClass is a closed enum with a fail-closed default, because a string is a comment
//      until something refuses to act on it.

#pragma once

#include <QByteArray>
#include <QDateTime>
#include <QList>
#include <QString>
#include <QUuid>

namespace cybou {

// РУС: Вид когнитивного вклада — какой именно мыслительный акт совершает орган.
// РУС: Каждое значение — не просто метка, а указание на роль вклада в когнитивном цикле.
// РУС: Все значения объявлены сразу (даже неиспользуемые в альфе), чтобы проволочный
// РУС: формат (wire format) оставался стабильным при появлении новых органов.
/// What kind of contribution this is. The alpha uses a subset; the rest are declared now so
/// the wire format does not change when organs arrive (docs/14).
enum class ContributionKind : quint16 {
    // РУС: Наблюдение — первичный контакт с реальностью. Единственный вид,
    // РУС: которому не нужна причина (causationId): оно САМО является корнем цепочки.
    Observation = 1,
    // РУС: Пересмотр убеждения — орган обновляет ранее принятую гипотезу на основе
    // РУС: новых наблюдений. Старое убеждение не удаляется, а остаётся в Журнале.
    BeliefRevision,
    // РУС: Гипотеза — предположение, выдвинутое органом для объяснения наблюдений.
    // РУС: Может быть опровергнута или усилена последующими вкладами.
    Hypothesis,
    // РУС: Вызов из памяти — орган извлекает ранее записанный опыт из Журнала.
    MemoryRecall,
    // РУС: Сигнал потребности — орган сообщает о нехватке ресурса или внимания.
    NeedSignal,
    // РУС: Кандидат на внимание — workspaced использует это для формирования коалиций
    // РУС: вкладов по correlationId и вычисления салиентности (важности).
    AttentionCandidate,
    // РУС: Прогноз — predictord публикует предсказание с уверенностью (confidence).
    // РУС: Позднее Outcome покажет, сбылось ли оно, и калибрует точность предсказателя.
    Prediction,
    // РУС: Предложение плана — последовательность действий для достижения цели.
    PlanProposal,
    // РУС: Возражение — орган оспаривает предложение или гипотезу другого органа.
    Objection,
    // РУС: Решение — фиксация выбора из нескольких альтернатив.
    Decision,
    // РУС: Намерение — обязательство действовать. Намерение ЯВЛЯЕТСЯ вкладом в Журнал;
    // РУС: закрытие намерения — это Outcome, ссылающийся на него как на причину.
    Intention,
    // РУС: Результат — фиксация итога выполнения намерения или прогноза.
    // РУС: Замыкает каузальную петлю: Intention → действие → Outcome.
    Outcome,
    // РУС: Самооценка — selfd публикует модель собственного состояния системы.
    SelfAssessment,
    // РУС: Обучение — фиксация нового знания, извлечённого из опыта.
    Learning,
};

// РУС: Класс приватности — концентрические круги доверия.
// РУС: Порядок от самого закрытого к самому открытому — не случайность.
// РУС: Числовое значение Local=0 означает, что ЛЮБОЕ неизвестное или пустое
// РУС: значение трактуется как «не делиться ни с кем» (fail-closed).
// РУС: Это фундаментальный принцип: если система не понимает уровень
// РУС: приватности — она выбирает максимальную защиту, а не максимальную открытость.
/// Ordered from most restrictive to least. Local is the default on purpose: an absent or
/// unrecognised class must never mean "shareable".
enum class PrivacyClass : quint8 {
    // РУС: Local — данные не покидают этот процесс. Значение по умолчанию.
    Local = 0,
    // РУС: Node — данные видны всем органам на этом узле (машине).
    Node,
    // РУС: Household — данные доступны всем узлам в домашней сети Cybou.
    Household,
    // РУС: Public — данные могут быть показаны внешнему миру.
    Public,
};

// РУС: Наследование приватности: если вклад основан на доказательствах (evidence),
// РУС: его приватность не может быть МЕНЕЕ строгой, чем самое закрытое доказательство.
// РУС: Без этого правила «утечка через обобщение»: сводка локальных данных датчика
// РУС: стала бы публичной, просто пройдя через промежуточный орган.
/// A contribution's class is at least as restrictive as the most restrictive of its evidence.
/// Without this, privacy leaks through generalisation - a summary of local sensor data would
/// become publishable simply by passing through consolidation.
constexpr PrivacyClass mostRestrictive(PrivacyClass a, PrivacyClass b) noexcept
{
    return (static_cast<quint8>(a) < static_cast<quint8>(b)) ? a : b;
}

QString kindToString(ContributionKind kind);
QString privacyToString(PrivacyClass privacy);

/// Named kindToString/privacyToString rather than toString: QCOMPARE finds a toString
/// overload by ADL for its failure messages and requires char*, so a QString-returning
/// toString in this namespace breaks every test that compares these types.
///
/// Parse back from the journal. Unknown input yields Local for privacy - fail closed.
PrivacyClass privacyFromString(const QString &text);

// РУС: Структура конверта — минимальный набор полей, необходимый для того, чтобы
// РУС: любой орган мог: (а) идентифицировать вклад, (б) проследить его причину,
// РУС: (в) объединить его с другими вкладами в когнитивный эпизод, (г) оценить
// РУС: степень доверия и приватности.
/// The envelope every organ publishes and every organ can read.
struct CognitiveEnvelope {
    // РУС: Версия схемы — для обратной совместимости при эволюции формата.
    quint16 schemaVersion{1};

    // РУС: Уникальный идентификатор этого вклада. Именно через messageId на вклад
    // РУС: ссылаются causationId и evidence других конвертов.
    QUuid messageId;
    // РУС: Идентификатор когнитивного эпизода. Все вклады с одним correlationId
    // РУС: принадлежат одной «нити размышления» — сборке, расследованию, перезагрузке.
    // РУС: Workspace группирует вклады в коалиции по этому полю и вычисляет салиентность.
    /// Binds a whole cognitive episode - a build, a reboot, an investigation.
    QUuid correlationId;
    // РУС: Какой именно вклад НЕПОСРЕДСТВЕННО породил этот. Null только для корневых
    // РУС: наблюдений (Observation). Без causationId каузальный граф обрастает сиротами
    // РУС: и объяснение невозможно проследить до источника.
    /// The contribution that directly produced this one. Null for a root observation.
    QUuid causationId;

    // РУС: Какой орган и на каком узле произвёл этот вклад.
    QString originOrgan;
    QString originNode;

    // РУС: Вид когнитивного акта (см. ContributionKind выше).
    ContributionKind kind{ContributionKind::Observation};

    // РУС: Три временны́х метки, каждая решает свою задачу:
    // РУС: wallTime — когда это произошло по настенным часам (для человека).
    QDateTime wallTime;
    // РУС: monotonicTime — миллисекунды с загрузки; настенное время может прыгать
    // РУС: (NTP, спячка), монотонное — никогда. Гарантирует локальный порядок.
    /// Monotonic milliseconds since boot: wall time can jump, this cannot.
    quint64 monotonicTime{0};
    // РУС: logicalClock — логические часы Лэмпорта. Восстанавливают порядок между
    // РУС: узлами без необходимости синхронизировать физические часы.
    /// Restores order across nodes without trusting synchronised clocks.
    quint64 logicalClock{0};

    // РУС: Степень уверенности органа в этом вкладе. 1.0 для прямого наблюдения,
    // РУС: меньше — для выводов. predictord использует это для калибровки.
    /// How sure the originating organ is. 1.0 for a direct observation.
    double confidence{1.0};

    // РУС: Список messageId вкладов, на которых основан этот вывод.
    // РУС: Обеспечивает прослеживаемость: от любого утверждения можно дойти
    // РУС: до исходных наблюдений, пройдя по цепочке evidence.
    QList<QUuid> evidence;
    // РУС: Полезная нагрузка в формате CBOR — содержимое зависит от вида вклада.
    QByteArray payloadCbor;

    // РУС: Класс приватности. По умолчанию Local — fail-closed.
    PrivacyClass privacy{PrivacyClass::Local};
    // РУС: Область полномочий — какие возможности нужны для обработки этого вклада.
    QString capabilityScope;

    // РУС: Валидация конверта перед записью в Журнал. Журнал — единственное место,
    // РУС: где ошибочный вклад стал бы ВЕЧНЫМ, поэтому валидация строга:
    // РУС: - messageId не может быть пустым (иначе вклад не адресуем)
    // РУС: - originOrgan обязателен (анонимных органов не бывает)
    // РУС: - wallTime обязателен (когда произошло)
    // РУС: - не-Observation обязан иметь causationId или evidence (каузальная цепочка)
    // РУС: - confidence в диапазоне [0.0, 1.0]
    /// True when the envelope can be written to the journal and read back meaningfully.
    bool isValid() const;

    // РУС: Вычисляет эффективную приватность с учётом доказательств. Если этот вклад
    // РУС: основан на Local-данных, он сам не может быть Public — иначе утечка.
    // РУС: Вызывающий код передаёт классы приватности вкладов из списка evidence.
    /// Privacy derived from evidence, per the rule above. Callers pass the classes of the
    /// contributions named in `evidence`.
    PrivacyClass derivedPrivacy(const QList<PrivacyClass> &evidencePrivacy) const;
};

} // namespace cybou
