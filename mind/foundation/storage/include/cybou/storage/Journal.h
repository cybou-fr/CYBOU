// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/protocol/CognitiveEnvelope.h"

#include <QObject>
#include <QSqlDatabase>
#include <QSqlQuery>
#include <QString>

#include <optional>

namespace cybou {

inline constexpr int kCurrentDatabaseSchemaVersion = 2;
inline constexpr int kLegacyJournalHashVersion = 1;
inline constexpr int kCurrentJournalHashVersion = 2;

class Journal : public QObject
{
    Q_OBJECT

public:
    explicit Journal(
        const QString &path,
        const QString &connectionName = QString(),
        QObject *parent = nullptr);
    ~Journal() override;

    Journal(const Journal &) = delete;
    Journal &operator=(const Journal &) = delete;

    bool isOpen() const;
    QString lastError() const;
    int databaseSchemaVersion() const;

    /// Persist one contribution. A non-zero sequence means the transaction committed.
    quint64 append(const CognitiveEnvelope &envelope);

    quint64 count() const;
    QByteArray head() const;
    quint64 verify() const;

    QList<CognitiveEnvelope> recent(int limit = 50) const;
    QList<CognitiveEnvelope> episode(const QUuid &correlationId) const;

    bool contains(const QUuid &messageId) const;
    std::optional<CognitiveEnvelope> contribution(const QUuid &messageId) const;
    QList<QUuid> evidenceFor(const QUuid &messageId) const;
    bool hasOutcomeFor(const QUuid &causeId, const QString &originOrgan = QString()) const;

Q_SIGNALS:
    /// The local in-process precursor of eventd's accepted-contribution stream.
    ///
    /// Emitted synchronously only after COMMIT succeeds. Validation failures and rolled-back
    /// writes never appear on this stream.
    void accepted(const CognitiveEnvelope &envelope, quint64 sequence);

private:
    bool ensureSchema();
    bool createSchemaV2();
    bool migrateV1ToV2();
    bool ensureV2Indexes();
    bool createMigrationBackup();

    bool beginImmediate();
    bool commitTransaction();
    void rollbackTransaction();
    bool execSql(const QString &sql);

    int userVersion() const;
    bool tableExists(const QString &table) const;
    bool columnExists(const QString &table, const QString &column) const;

    QByteArray rowHashV1(
        quint64 seq, const CognitiveEnvelope &envelope, const QByteArray &previousHash) const;
    QByteArray rowHashV2(
        quint64 seq, const CognitiveEnvelope &envelope, const QByteArray &previousHash) const;

    CognitiveEnvelope envelopeFromQuery(const QSqlQuery &query, int offset) const;
    std::optional<CognitiveEnvelope> readOne(QSqlQuery &query) const;

    QSqlDatabase m_db;
    QString m_connectionName;
    QString m_path;
    QString m_lastError;
    bool m_ready{false};
};

} // namespace cybou
