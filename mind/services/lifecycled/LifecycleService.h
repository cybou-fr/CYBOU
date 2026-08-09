// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
#pragma once
#include "cybou/ipc/EventClient.h"
#include "cybou/protocol/Lifecycle.h"
#include <QObject>
namespace cybou {
class LifecycleService : public QObject {
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.cybou.Mind.Lifecycle1")
public:
    explicit LifecycleService(const QString &path, QObject *parent = nullptr);
    bool isReady() const { return m_ready; }
    QString startupError() const { return m_error; }
public Q_SLOTS:
    bool Ready() const { return m_ready; }
    QString Health() const { return m_ready ? QStringLiteral("healthy") : QStringLiteral("unavailable"); }
    QString LastError() const { return m_error; }
    QByteArray State() const;
    bool Transition(const QString &mode);
    bool BeginRun(const QByteArray &encoded);
    QString RequestRun(const QString &kind, const QString &policyId,
                       qulonglong inputHighWaterMark,
                       const QStringList &requiredCapabilities,
                       const QStringList &optionalCapabilities);
    QString WorkOperationKey(const QString &capability) const;
    bool AcknowledgeWork(const QString &capability, const QString &operationKey,
                         qulonglong inputHighWaterMark);
    bool MarkMissing(const QString &capability, const QString &cause);
    bool Dispatch();
    bool ResumeRun();
    bool FinishRun(const QString &status, const QString &cause);
private:
    bool load(); bool save();
    bool requestedCapability(const QString &capability) const;
    bool acceptOwnerResult(const QString &capability, const QString &operationKey,
                           qulonglong inputHighWaterMark, const QUuid &contributionId);
    bool commitTerminalContribution(const QString &cause);
    QString m_path; LifecycleMode m_mode{LifecycleMode::Awake}; LifecycleRun m_run;
    bool m_hasRun{false}; bool m_ready{false}; QString m_error;
    EventClient m_events;
};
}
