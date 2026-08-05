// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// РУС: ═══════════════════════════════════════════════════════════════════════════
// РУС: ЖУРНАЛ — биография системы. Только дописывание, хеш-цепочка для целостности.
// РУС:
// РУС: Это не база данных и не лог. Это автобиография когнитивной системы.
// РУС: Каждая «мысль» (CognitiveEnvelope) каждого органа записывается сюда
// РУС: и НИКОГДА не удаляется и не изменяется.
// РУС:
// РУС: Почему append-only?
// РУС: Потому что ошибки — часть опыта. Если predictord предсказал X и ошибся,
// РУС: ошибочный прогноз остаётся в Журнале. Исправление — это НОВЫЙ вклад
// РУС: (BeliefRevision), ссылающийся на старый через causationId. Так система
// РУС: учится на своих ошибках, а не прячет их.
// РУС:
// РУС: Хеш-цепочка (SHA-256):
// РУС: Каждая строка содержит хеш предыдущей. Если кто-то изменит любую строку
// РУС: в прошлом — цепочка ломается от этой точки и далее, и verify() покажет
// РУС: точный номер первой испорченной записи. Это делает append-only
// РУС: ПРИНУДИТЕЛЬНЫМ, а не «договорённостью».
// РУС:
// РУС: Органы-равноправные участники: ни один орган не «владеет» Журналом.
// РУС: Все органы пишут сюда свои вклады и все могут их читать.
// РУС: ═══════════════════════════════════════════════════════════════════════════
//
// The biography. Append-only, and the append-only part is enforced rather than promised:
// every row carries the hash of the previous one, so a rewrite anywhere in the past breaks
// the chain from that point forward and `verify()` says where.
//
// docs/14-mind-architecture.md: old events are never corrected. A weakened hypothesis is a
// later event, so the mistake stays visible.

#pragma once

#include "cybou/protocol/CognitiveEnvelope.h"

#include <QSqlDatabase>
#include <QString>

namespace cybou {

// РУС: Класс Journal — единственный интерфейс органов к биографии системы.
// РУС: Не копируемый (удалены copy-конструктор и оператор присваивания),
// РУС: потому что два объекта Journal на одну БД — это гарантированный конфликт
// РУС: хеш-цепочки.
class Journal
{
public:
    // РУС: Открывает (или создаёт) SQLite-файл по указанному пути.
    // РУС: Схема создаётся автоматически при первом открытии.
    /// `path` is the SQLite file. Opening creates the schema if absent.
    explicit Journal(const QString &path, const QString &connectionName = QString());
    ~Journal();

    Journal(const Journal &) = delete;
    Journal &operator=(const Journal &) = delete;

    bool isOpen() const;
    QString lastError() const;

    // РУС: Дописывает один вклад в конец Журнала. Возвращает порядковый номер (seq)
    // РУС: или 0 при неудаче. Невалидные конверты ОТКЛОНЯЮТСЯ — Журнал является
    // РУС: единственным местом, где ошибочный вклад стал бы вечным.
    // РУС: Внутри: берёт head() (хеш последней строки), вычисляет новый хеш,
    // РУС: и вставляет строку с prev_hash → hash, продолжая цепочку.
    /// Appends one contribution and returns its sequence number, or 0 on failure.
    /// Invalid envelopes are refused: the journal is the one place where a malformed
    /// contribution would be permanent.
    quint64 append(const CognitiveEnvelope &envelope);

    // РУС: Сколько вкладов записано в Журнале.
    /// Number of contributions recorded.
    quint64 count() const;

    // РУС: Хеш последней записи — «голова» цепочки. Следующий append()
    // РУС: возьмёт это значение как prev_hash для новой строки.
    /// The hash of the most recent row - the head of the chain.
    QByteArray head() const;

    // РУС: Проверка целостности — проходит всю цепочку от первой строки до последней.
    // РУС: Для каждой строки пересчитывает хеш и сравнивает с сохранённым.
    // РУС: Возвращает 0, если цепочка цела. Иначе — номер первой повреждённой строки.
    // РУС: Это позволяет обнаружить подделку, даже если она произошла вне процесса
    // РУС: (например, через прямое редактирование SQLite-файла).
    /// Walks the whole chain. Returns 0 when intact, otherwise the sequence number of the
    /// first row whose stored hash does not match its recomputed one.
    quint64 verify() const;

    // РУС: Последние вклады, от новых к старым. presenced использует это
    // РУС: для отображения текущего состояния сознания на панели.
    /// Reads back contributions, newest first. `limit` of 0 means all.
    QList<CognitiveEnvelope> recent(int limit = 50) const;

    // РУС: Все вклады одного когнитивного эпизода, от старых к новым.
    // РУС: Эпизод определяется correlationId — все вклады с одним correlationId
    // РУС: образуют «нить размышления» (сборка, расследование, перезагрузка).
    // РУС: Порядок хронологический, чтобы цепочку рассуждений можно было
    // РУС: «проиграть» заново в том порядке, как она возникла.
    /// Everything belonging to one cognitive episode, oldest first, so a chain of reasoning
    /// can be replayed in the order it happened.
    QList<CognitiveEnvelope> episode(const QUuid &correlationId) const;

private:
    // РУС: Вычисляет SHA-256 хеш строки: H(prev || seq || messageId || ... || payload).
    // РУС: prev — хеш предыдущей строки, что и образует цепочку.
    QByteArray rowHash(quint64 seq, const CognitiveEnvelope &e, const QByteArray &prev) const;
    // РУС: Создаёт таблицу contribution, если она ещё не существует.
    bool ensureSchema();

    QSqlDatabase m_db;
    QString m_connectionName;
    QString m_lastError;
};

} // namespace cybou
