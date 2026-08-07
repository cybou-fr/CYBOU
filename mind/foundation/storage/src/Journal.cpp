// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

// РУС: ═══════════════════════════════════════════════════════════════════════════
// РУС: Реализация Журнала — append-only хранилище с хеш-цепочкой SHA-256.
// РУС:
// РУС: Хранение — SQLite в режиме WAL (Write-Ahead Logging):
// РУС: - Читатели (presenced, workspaced) НИКОГДА не блокируют писателя (eventd).
// РУС: - Писатель никогда не блокирует читателей.
// РУС: - Это критично для когнитивной архитектуры: органы работают параллельно.
// РУС:
// РУС: Против таблицы НЕ выполняется ни один UPDATE или DELETE. Никогда.
// РУС: Хеш-цепочка делает внешнее изменение ОБНАРУЖИВАЕМЫМ, а не просто
// РУС: «нежелательным». verify() точно скажет, где цепочка сломалась.
// РУС: ═══════════════════════════════════════════════════════════════════════════

#include "cybou/storage/Journal.h"

#include <QCryptographicHash>
#include <QFileInfo>
#include <QDir>
#include <QSqlError>
#include <QSqlQuery>
#include <QUuid>

namespace cybou {

namespace {
// РУС: Уникальное имя соединения для каждого экземпляра Journal.
// РУС: Qt требует уникальные имена для QSqlDatabase::addDatabase(),
// РУС: иначе два экземпляра Journal будут конфликтовать.
QString defaultConnectionName()
{
    return QStringLiteral("cybou-journal-%1").arg(QUuid::createUuid().toString(QUuid::Id128));
}
}

Journal::Journal(const QString &path, const QString &connectionName)
    : m_connectionName(connectionName.isEmpty() ? defaultConnectionName() : connectionName)
{
    // РУС: Создаём директорию, если она не существует — Журнал должен быть доступен
    // РУС: с первого запуска, без внешней настройки.
    QDir().mkpath(QFileInfo(path).absolutePath());

    m_db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), m_connectionName);
    m_db.setDatabaseName(path);
    if (!m_db.open()) {
        m_lastError = m_db.lastError().text();
        return;
    }

    // WAL so a reader (presenced) never blocks the writer (eventd).
    // РУС: WAL (Write-Ahead Logging) — режим журналирования SQLite, при котором
    // РУС: читатели видят согласованный снимок данных, не блокируя запись.
    // РУС: synchronous=NORMAL — компромисс: защита от сбоев SQLite, но без
    // РУС: полного fsync на каждый коммит (что убило бы производительность).
    QSqlQuery pragma(m_db);
    pragma.exec(QStringLiteral("PRAGMA journal_mode=WAL"));
    pragma.exec(QStringLiteral("PRAGMA synchronous=NORMAL"));

    ensureSchema();
}

// РУС: Деструктор: закрываем соединение и удаляем его из реестра Qt.
// РУС: m_db = QSqlDatabase() — сбрасываем объект ДО removeDatabase(),
// РУС: иначе Qt предупредит о «database still in use».
Journal::~Journal()
{
    const QString name = m_connectionName;
    if (m_db.isOpen()) {
        m_db.close();
    }
    m_db = QSqlDatabase();
    QSqlDatabase::removeDatabase(name);
}

bool Journal::isOpen() const
{
    return m_db.isOpen();
}

QString Journal::lastError() const
{
    return m_lastError;
}

// РУС: Схема таблицы contribution — зеркало структуры CognitiveEnvelope.
// РУС: Два ключевых столбца для хеш-цепочки:
// РУС:   prev_hash — хеш предыдущей строки (NULL для первой строки)
// РУС:   hash — SHA-256 хеш текущей строки, включающий prev_hash
// РУС: Против этой таблицы НИКОГДА не выполняется UPDATE или DELETE.
// РУС: Индекс idx_correlation ускоряет episode() — запрос всех вкладов эпизода.
bool Journal::ensureSchema()
{
    QSqlQuery q(m_db);
    // No UPDATE or DELETE is ever issued against this table. The hash chain makes a rewrite
    // outside the process detectable rather than merely discouraged.
    const bool ok = q.exec(QStringLiteral(R"SQL(
        CREATE TABLE IF NOT EXISTS contribution (
            seq            INTEGER PRIMARY KEY AUTOINCREMENT,
            message_id     TEXT    NOT NULL UNIQUE,
            correlation_id TEXT,
            causation_id   TEXT,
            origin_organ   TEXT    NOT NULL,
            origin_node    TEXT,
            kind           INTEGER NOT NULL,
            wall_time      TEXT    NOT NULL,
            monotonic_time INTEGER NOT NULL,
            logical_clock  INTEGER NOT NULL,
            confidence     REAL    NOT NULL,
            evidence       TEXT,
            payload        BLOB,
            privacy        INTEGER NOT NULL,
            capability     TEXT,
            prev_hash      BLOB,
            hash           BLOB    NOT NULL
        )
    )SQL"));

    if (!ok) {
        m_lastError = q.lastError().text();
        return false;
    }
    // РУС: Индекс по correlation_id — для быстрого извлечения эпизодов.
    q.exec(QStringLiteral("CREATE INDEX IF NOT EXISTS idx_correlation "
                          "ON contribution(correlation_id)"));
    return true;
}

// РУС: Вычисление хеша строки — сердце механизма целостности.
// РУС: Формула: SHA-256(prev_hash || seq || messageId || correlationId || causationId
// РУС:                   || originOrgan || kind || wallTime || logicalClock || payload)
// РУС:
// РУС: prev — хеш предыдущей строки. Для первой строки prev пуст.
// РУС: Именно включение prev в хеш создаёт ЦЕПОЧКУ: изменение любой строки
// РУС: в прошлом меняет её хеш, что ломает prev_hash следующей строки, и так далее
// РУС: до конца. verify() обнаружит разрыв.
// РУС:
// РУС: Не все поля конверта входят в хеш — только те, что определяют
// РУС: семантическую идентичность вклада. Это осознанный выбор.
QByteArray Journal::rowHash(quint64 seq, const CognitiveEnvelope &e, const QByteArray &prev) const
{
    QCryptographicHash h(QCryptographicHash::Sha256);
    // РУС: Начинаем с хеша предыдущей строки — это и есть «звено цепочки».
    h.addData(prev);
    h.addData(QByteArray::number(static_cast<qulonglong>(seq)));
    h.addData(e.messageId.toByteArray());
    h.addData(e.correlationId.toByteArray());
    h.addData(e.causationId.toByteArray());
    h.addData(e.originOrgan.toUtf8());
    h.addData(QByteArray::number(static_cast<int>(e.kind)));
    h.addData(e.wallTime.toString(Qt::ISODateWithMs).toUtf8());
    h.addData(QByteArray::number(static_cast<qulonglong>(e.logicalClock)));
    h.addData(e.payloadCbor);
    return h.result();
}

// РУС: append() — единственный способ добавить вклад в Журнал.
// РУС: Это последний рубеж валидации: isValid() проверяет конверт перед записью.
// РУС: Если конверт невалиден — отказ, а не запись с пометкой «ошибка».
// РУС: Потому что в append-only хранилище с хеш-цепочкой невозможно потом
// РУС: «тихо удалить» испорченную запись.
quint64 Journal::append(const CognitiveEnvelope &e)
{
    if (!m_db.isOpen()) {
        m_lastError = QStringLiteral("journal is not open");
        return 0;
    }
    if (!e.isValid()) {
        // Refused rather than stored: this is the one place a malformed contribution would
        // become permanent.
        // РУС: Отказ, а не хранение: Журнал — единственное место, где ошибочный
        // РУС: вклад стал бы ВЕЧНЫМ и сломал бы целостность хеш-цепочки.
        m_lastError = QStringLiteral("refusing to append an invalid envelope");
        return 0;
    }

    // РУС: Берём хеш последней строки — «голову» цепочки.
    // РУС: Для первой записи в пустом Журнале prev будет пустым QByteArray.
    const QByteArray prev = head();
    // РУС: Вычисляем следующий порядковый номер.
    const quint64 seq = count() + 1;

    // РУС: Сериализуем список evidence UUID в строку через запятую.
    QStringList evidenceIds;
    evidenceIds.reserve(e.evidence.size());
    for (const QUuid &id : e.evidence) {
        evidenceIds << id.toString(QUuid::WithoutBraces);
    }

    QSqlQuery q(m_db);
    q.prepare(QStringLiteral(
        "INSERT INTO contribution (message_id, correlation_id, causation_id, origin_organ, "
        "origin_node, kind, wall_time, monotonic_time, logical_clock, confidence, evidence, "
        "payload, privacy, capability, prev_hash, hash) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"));

    q.addBindValue(e.messageId.toString(QUuid::WithoutBraces));
    // РУС: null UUID → SQL NULL, чтобы корневые наблюдения не имели фиктивных ссылок.
    q.addBindValue(e.correlationId.isNull() ? QVariant()
                                            : e.correlationId.toString(QUuid::WithoutBraces));
    q.addBindValue(e.causationId.isNull() ? QVariant()
                                          : e.causationId.toString(QUuid::WithoutBraces));
    q.addBindValue(e.originOrgan);
    q.addBindValue(e.originNode);
    q.addBindValue(static_cast<int>(e.kind));
    q.addBindValue(e.wallTime.toString(Qt::ISODateWithMs));
    q.addBindValue(static_cast<qulonglong>(e.monotonicTime));
    q.addBindValue(static_cast<qulonglong>(e.logicalClock));
    q.addBindValue(e.confidence);
    q.addBindValue(evidenceIds.join(QLatin1Char(',')));
    q.addBindValue(e.payloadCbor);
    q.addBindValue(static_cast<int>(e.privacy));
    q.addBindValue(e.capabilityScope);
    // РУС: Записываем prev_hash и вычисленный hash — продолжаем цепочку.
    q.addBindValue(prev);
    q.addBindValue(rowHash(seq, e, prev));

    if (!q.exec()) {
        m_lastError = q.lastError().text();
        return 0;
    }
    // РУС: Возвращаем порядковый номер записи — подтверждение успешного дописывания.
    return seq;
}

quint64 Journal::count() const
{
    QSqlQuery q(m_db);
    if (q.exec(QStringLiteral("SELECT COUNT(*) FROM contribution")) && q.next()) {
        return q.value(0).toULongLong();
    }
    return 0;
}

// РУС: head() — хеш последней строки. Это «голова» цепочки, от которой
// РУС: следующий append() начнёт наращивать. ORDER BY seq DESC LIMIT 1
// РУС: гарантирует получение именно последней записи.
QByteArray Journal::head() const
{
    QSqlQuery q(m_db);
    if (q.exec(QStringLiteral("SELECT hash FROM contribution ORDER BY seq DESC LIMIT 1"))
        && q.next()) {
        return q.value(0).toByteArray();
    }
    // РУС: Пустой Журнал — пустой хеш. Первая запись будет иметь prev_hash = {}.
    return {};
}

// РУС: verify() — проход по всей хеш-цепочке от первой строки до последней.
// РУС: Алгоритм:
// РУС:   1. Читаем все строки в порядке seq (хронологическом).
// РУС:   2. Для каждой строки проверяем ДВА условия:
// РУС:      а) prev_hash этой строки == hash предыдущей строки (цепочка не разорвана)
// РУС:      б) пересчитанный rowHash() == сохранённый hash (данные не изменены)
// РУС:   3. Если оба условия выполнены — переходим к следующей строке.
// РУС:   4. Если нет — возвращаем seq первой сломанной строки.
// РУС:   5. Если дошли до конца — возвращаем 0 (всё в порядке).
// РУС:
// РУС: Это позволяет обнаружить ЛЮБОЕ изменение данных, сделанное вне процесса
// РУС: (например, через sqlite3 CLI или поврежение файла).
quint64 Journal::verify() const
{
    QSqlQuery q(m_db);
    if (!q.exec(QStringLiteral(
            "SELECT seq, message_id, correlation_id, causation_id, origin_organ, kind, "
            "wall_time, logical_clock, payload, prev_hash, hash FROM contribution ORDER BY seq"))) {
        // РУС: Не удалось выполнить запрос — считаем первую строку подозрительной.
        return 1;
    }

    // РУС: expectedPrev — хеш, который мы ОЖИДАЕМ увидеть в prev_hash следующей строки.
    // РУС: Для первой строки он пуст (у первой записи нет предшественника).
    QByteArray expectedPrev;
    while (q.next()) {
        const quint64 seq = q.value(0).toULongLong();

        // РУС: Восстанавливаем конверт из БД для пересчёта хеша.
        CognitiveEnvelope e;
        e.messageId = QUuid::fromString(q.value(1).toString());
        e.correlationId = QUuid::fromString(q.value(2).toString());
        e.causationId = QUuid::fromString(q.value(3).toString());
        e.originOrgan = q.value(4).toString();
        e.kind = static_cast<ContributionKind>(q.value(5).toInt());
        e.wallTime = QDateTime::fromString(q.value(6).toString(), Qt::ISODateWithMs);
        e.logicalClock = q.value(7).toULongLong();
        e.payloadCbor = q.value(8).toByteArray();

        const QByteArray storedPrev = q.value(9).toByteArray();
        const QByteArray storedHash = q.value(10).toByteArray();

        // РУС: Двойная проверка:
        // РУС: 1. storedPrev == expectedPrev — цепочка не разорвана
        // РУС: 2. rowHash(seq, e, storedPrev) == storedHash — данные не изменены
        if (storedPrev != expectedPrev || rowHash(seq, e, storedPrev) != storedHash) {
            return seq;
        }
        // РУС: Продвигаем ожидаемый хеш для следующей строки.
        expectedPrev = storedHash;
    }
    // РУС: Вся цепочка цела.
    return 0;
}

// РУС: recent() — последние N вкладов, от новых к старым (ORDER BY seq DESC).
// РУС: Используется presenced для отображения текущего «потока сознания» на панели.
// РУС: limit=0 означает «все записи» (без LIMIT в SQL).
QList<CognitiveEnvelope> Journal::recent(int limit) const
{
    QList<CognitiveEnvelope> out;
    QSqlQuery q(m_db);
    const QString sql = QStringLiteral(
        "SELECT message_id, correlation_id, causation_id, origin_organ, origin_node, kind, "
        "wall_time, monotonic_time, logical_clock, confidence, payload, privacy "
        "FROM contribution ORDER BY seq DESC%1");
    q.prepare(sql.arg(limit > 0 ? QStringLiteral(" LIMIT %1").arg(limit) : QString()));
    if (!q.exec()) {
        return out;
    }
    while (q.next()) {
        CognitiveEnvelope e;
        e.messageId = QUuid::fromString(q.value(0).toString());
        e.correlationId = QUuid::fromString(q.value(1).toString());
        e.causationId = QUuid::fromString(q.value(2).toString());
        e.originOrgan = q.value(3).toString();
        e.originNode = q.value(4).toString();
        e.kind = static_cast<ContributionKind>(q.value(5).toInt());
        e.wallTime = QDateTime::fromString(q.value(6).toString(), Qt::ISODateWithMs);
        e.monotonicTime = q.value(7).toULongLong();
        e.logicalClock = q.value(8).toULongLong();
        e.confidence = q.value(9).toDouble();
        e.payloadCbor = q.value(10).toByteArray();
        e.privacy = static_cast<PrivacyClass>(q.value(11).toInt());
        out.append(e);
    }
    return out;
}

// РУС: episode() — все вклады одного когнитивного эпизода, в хронологическом порядке.
// РУС: Эпизод — это «нить размышления», объединённая одним correlationId.
// РУС: Примеры эпизодов: сборка проекта, расследование аномалии, перезагрузка.
// РУС: ORDER BY seq (не DESC!) — чтобы цепочку можно было «проиграть» заново
// РУС: в порядке возникновения: наблюдение → гипотеза → проверка → результат.
// РУС: Используется workspaced для формирования коалиций и вычисления салиентности.
// РУС: Индекс idx_correlation обеспечивает быстрый поиск без полного сканирования.
QList<CognitiveEnvelope> Journal::episode(const QUuid &correlationId) const
{
    QList<CognitiveEnvelope> out;
    QSqlQuery q(m_db);
    q.prepare(QStringLiteral(
        "SELECT message_id, causation_id, origin_organ, kind, wall_time, confidence "
        "FROM contribution WHERE correlation_id = ? ORDER BY seq"));
    q.addBindValue(correlationId.toString(QUuid::WithoutBraces));
    if (!q.exec()) {
        return out;
    }
    while (q.next()) {
        CognitiveEnvelope e;
        e.messageId = QUuid::fromString(q.value(0).toString());
        // РУС: correlationId не нужно читать из БД — он одинаков для всех строк выборки.
        e.correlationId = correlationId;
        e.causationId = QUuid::fromString(q.value(1).toString());
        e.originOrgan = q.value(2).toString();
        e.kind = static_cast<ContributionKind>(q.value(3).toInt());
        e.wallTime = QDateTime::fromString(q.value(4).toString(), Qt::ISODateWithMs);
        e.confidence = q.value(5).toDouble();
        out.append(e);
    }
    return out;
}

} // namespace cybou
