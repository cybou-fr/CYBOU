// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/events/EventStore.h"

#include <QSqlDatabase>
#include <QSqlQuery>
#include <QString>

#include <optional>

namespace cybou {

inline constexpr int kCurrentDatabaseSchemaVersion = 2;
inline constexpr int kLegacyJournalHashVersion = 1;
inline constexpr int kCurrentJournalHashVersion = 2;

/// SQLite `synchronous` level at or above which a returned COMMIT has reached storage. Below this,
/// Event1 would publish acceptance for a commit that a power loss can still discard.
inline constexpr int kRequiredSynchronousLevel = 2; // FULL

/// Low-level SQLite implementation.
///
/// Production organs do not depend on this class after M3. cybou-eventd owns the production
/// instance; tests may still instantiate Journal directly behind the EventStore contract.
class Journal : public EventStore
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

    bool isOpen() const override;
    QString lastError() const override;
    int databaseSchemaVersion() const override;

    quint64 append(const CognitiveEnvelope &envelope) override;

    quint64 count() const override;
    QByteArray head() const override;
    quint64 verify() const override;

    QList<CognitiveEnvelope> recent(int limit = 50) const override;
    QList<CognitiveEnvelope> episode(const QUuid &correlationId) const override;
    std::optional<CognitiveEnvelope> atSequence(quint64 sequence) const override;

    bool contains(const QUuid &messageId) const override;
    std::optional<CognitiveEnvelope> contribution(const QUuid &messageId) const override;
    QList<QUuid> evidenceFor(const QUuid &messageId) const override;
    bool hasOutcomeFor(
        const QUuid &causeId,
        const QString &originOrgan = QString()) const override;

private:
    bool ensureDurability();
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
