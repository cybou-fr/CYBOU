// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// РУС: ═══════════════════════════════════════════════════════════════════════════
// РУС: Орган намерений — стоящие цели, которые переживают процесс, их
// РУС: создавший.
// РУС:
// РУС: Ключевой принцип: намерения НЕ хранятся отдельно. Намерение ЯВЛЯЕТСЯ
// РУС: вкладом (Intention) в журнале, а его закрытие ЯВЛЯЕТСЯ
// РУС: результатом (Outcome), который указывает на намерение как причину.
// РУС:
// РУС: Открытые намерения вычисляются каждый раз свёрткой журнала:
// РУС: (все Intention) минус (все, чей Outcome именует их как причину).
// РУС: Нет дублирования — список не может разойтись с биографией,
// РУС: и на вопрос «почему ты думал, что мне это должен?» можно ответить,
// РУС: прочитав цепочку.
// РУС: ═══════════════════════════════════════════════════════════════════════════
//
// Standing goals that outlive the process that formed them.
//
// There is no separate state file here, on purpose. An intention *is* a contribution in the
// journal, and closing one *is* an Outcome that names it as its cause. Open intentions are
// therefore derived, not stored twice - so the list can never drift from the biography, and
// "why did you think you owed me this?" is answerable by reading the chain.

#pragma once

#include "cybou/storage/Journal.h"

#include <QString>

namespace cybou {

// РУС: Одно намерение: обязательство системы перед пользователем.
struct Intention {
    // РУС: id = messageId вклада Intention в журнале.
    QUuid id;
    // РУС: Что система обещала сделать.
    QString description;
    /// What has to happen for this to be satisfiable - free text in the alpha, a condition
    /// later. Recorded so the reason survives even when the intention does not.
    // РУС: trigger — условие, при котором намерение может быть выполнено.
    // РУС: Записывается, чтобы причина пережила само намерение.
    QString trigger;
    // РУС: Момент формирования — для отображения возраста обязательства.
    QDateTime formed;
};

// РУС: Исход намерения: выполнено, оставлено или устарело. Различение важно
// РУС: для калибровки предсказаний и для честности самооценки.
enum class Resolution : quint8 {
    // РУС: Намерение выполнено.
    Fulfilled,
    // РУС: Намерение оставлено сознательно.
    Abandoned,
    /// The reason it existed no longer applies. Not a failure, and worth distinguishing from
    /// abandonment when accuracy is measured later.
    // РУС: Причина намерения отпала. Не провал, и важно отличать от оставления
    // РУС: при измерении точности.
    Obsolete,
};

// РУС: Класс Intentions — орган намерений. Нет собственного файла состояния —
// РУС: всё состояние выводится из журнала при каждом запросе.
class Intentions
{
public:
    // РУС: Единственная зависимость — журнал. Намерения живут в нём.
    explicit Intentions(Journal *journal);

    /// Forms an intention and returns its id, or a null uuid on failure.
    // РУС: form() — создаёт намерение и возвращает его ID. Намерение является
    // РУС: своей собственной причиной (causationId = messageId) — оно не выведено
    // РУС: из другого, а сформировано. Это корень каузальной цепочки.
    QUuid form(const QString &description, const QString &trigger = QString());

    /// Records that an intention ended, and why. The Outcome names the intention as its
    /// cause, which is what removes it from the open list.
    // РУС: close() — закрывает намерение. Создаёт Outcome, указывающий на намерение
    // РУС: как причину (causationId = intentionId). Именно это исключает его из
    // РУС: списка открытых.
    bool close(const QUuid &intentionId, Resolution resolution, const QString &note = QString());

    /// Intentions with no Outcome naming them. This is the answer to "what do I still owe?"
    /// after a reboot, and it is computed from the journal every time rather than cached.
    // РУС: open() — вычисляет открытые намерения свёрткой журнала каждый раз,
    // РУС: без кеширования. Просто, пока журнал невелик. Ответ на «что я
    // РУС: всё ещё должен?» после перезагрузки.
    QList<Intention> open() const;

    QString lastError() const { return m_lastError; }

private:
    Journal *m_journal;
    QString m_lastError;
};

QString resolutionToString(Resolution r);

} // namespace cybou
