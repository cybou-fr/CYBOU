// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
#include "cybou/protocol/Lifecycle.h"

#include <QCborArray>
#include <QCborMap>
#include <QCborValue>
#include <QSet>

namespace cybou {
namespace {
void setError(QString *error, const QString &text) { if (error) *error = text; }
bool uniqueNonEmpty(const QStringList &items) {
    QSet<QString> seen;
    for (const QString &item : items) {
        const QString value = item.trimmed();
        if (value.isEmpty() || seen.contains(value)) return false;
        seen.insert(value);
    }
    return true;
}
QCborArray strings(const QStringList &items) { QCborArray out; for (const auto &v : items) out.append(v); return out; }
QStringList stringList(const QCborValue &value) { QStringList out; for (const auto &v : value.toArray()) out.append(v.toString()); return out; }
}

QString lifecycleModeToString(LifecycleMode mode) {
    switch (mode) { case LifecycleMode::Awake: return "awake"; case LifecycleMode::Idle: return "idle";
    case LifecycleMode::Consolidating: return "consolidating"; case LifecycleMode::Maintenance: return "maintenance";
    case LifecycleMode::Recovering: return "recovering"; case LifecycleMode::Degraded: return "degraded";
    case LifecycleMode::Suspended: return "suspended"; } return "unknown";
}
QString lifecycleRunStatusToString(LifecycleRunStatus status) {
    switch (status) { case LifecycleRunStatus::Requested: return "requested"; case LifecycleRunStatus::Active: return "active";
    case LifecycleRunStatus::Completed: return "completed"; case LifecycleRunStatus::Interrupted: return "interrupted";
    case LifecycleRunStatus::Failed: return "failed"; } return "unknown";
}

bool canTransition(LifecycleMode from, LifecycleMode to) noexcept {
    if (from == to) return false;
    if (to == LifecycleMode::Degraded || to == LifecycleMode::Recovering) return true;
    switch (from) {
    case LifecycleMode::Awake: return to == LifecycleMode::Idle || to == LifecycleMode::Maintenance || to == LifecycleMode::Suspended;
    case LifecycleMode::Idle: return to == LifecycleMode::Awake || to == LifecycleMode::Consolidating || to == LifecycleMode::Maintenance || to == LifecycleMode::Suspended;
    case LifecycleMode::Consolidating: return to == LifecycleMode::Awake || to == LifecycleMode::Idle;
    case LifecycleMode::Maintenance: return to == LifecycleMode::Awake || to == LifecycleMode::Suspended;
    case LifecycleMode::Recovering: return to == LifecycleMode::Awake || to == LifecycleMode::Suspended;
    case LifecycleMode::Degraded: return to == LifecycleMode::Recovering || to == LifecycleMode::Awake || to == LifecycleMode::Suspended;
    case LifecycleMode::Suspended: return to == LifecycleMode::Recovering || to == LifecycleMode::Awake;
    } return false;
}

bool LifecycleRun::isTerminal() const noexcept { return status == LifecycleRunStatus::Completed || status == LifecycleRunStatus::Interrupted || status == LifecycleRunStatus::Failed; }
bool LifecycleRun::isValid() const {
    if (schemaVersion != kLifecycleSchemaVersion || runId.isNull() || kind.trimmed().isEmpty() || policyId.trimmed().isEmpty() || !requestedAt.isValid()) return false;
    switch (status) {
    case LifecycleRunStatus::Requested:
    case LifecycleRunStatus::Active:
    case LifecycleRunStatus::Completed:
    case LifecycleRunStatus::Interrupted:
    case LifecycleRunStatus::Failed:
        break;
    default:
        return false;
    }
    if (!uniqueNonEmpty(requiredCapabilities) || !uniqueNonEmpty(optionalCapabilities) || !uniqueNonEmpty(completedWork) || !uniqueNonEmpty(missingWork)) return false;
    QSet<QString> required(requiredCapabilities.begin(), requiredCapabilities.end());
    for (const auto &v : optionalCapabilities) if (required.contains(v)) return false;
    QSet<QString> requested=required; requested.unite(QSet<QString>(optionalCapabilities.begin(),optionalCapabilities.end()));
    QSet<QString> completed(completedWork.begin(),completedWork.end()); QSet<QString> missing(missingWork.begin(),missingWork.end());
    for(const auto &v:completed)if(!requested.contains(v))return false;
    for(const auto &v:missing)if(!requested.contains(v)||completed.contains(v))return false;
    if (missingCauses.size() != missing.size()) return false;
    for (auto it = missingCauses.cbegin(); it != missingCauses.cend(); ++it)
        if (!missing.contains(it.key()) || it.value().trimmed().isEmpty()) return false;
    QSet<QUuid> contributionIds;
    for (auto it = workContributions.cbegin(); it != workContributions.cend(); ++it) {
        if (!completed.contains(it.key()) || it.value().isNull()
            || contributionIds.contains(it.value())) return false;
        contributionIds.insert(it.value());
    }
    if (isTerminal() != !terminalCause.trimmed().isEmpty()) return false;
    if (!terminalContributionId.isNull() && !isTerminal()) return false;
    if(status==LifecycleRunStatus::Completed){for(const auto &v:required)if(!completed.contains(v)||missing.contains(v))return false;for(const auto &v:optionalCapabilities)if(!completed.contains(v)&&!missing.contains(v))return false;}
    return true;
}

QByteArray encodeLifecycleRun(const LifecycleRun &run) {
    QCborMap m;
    m.insert(QStringLiteral("schemaVersion"), run.schemaVersion);
    m.insert(QStringLiteral("runId"), run.runId.toString(QUuid::WithoutBraces));
    m.insert(QStringLiteral("kind"), run.kind);
    m.insert(QStringLiteral("policyId"), run.policyId);
    m.insert(QStringLiteral("requestedAt"), run.requestedAt.toUTC().toString(Qt::ISODateWithMs));
    m.insert(QStringLiteral("inputHighWaterMark"), static_cast<qint64>(run.inputHighWaterMark));
    m.insert(QStringLiteral("requiredCapabilities"), strings(run.requiredCapabilities));
    m.insert(QStringLiteral("optionalCapabilities"), strings(run.optionalCapabilities));
    m.insert(QStringLiteral("status"), static_cast<qint64>(run.status));
    m.insert(QStringLiteral("completedWork"), strings(run.completedWork));
    QCborMap contributions;
    for (auto it = run.workContributions.cbegin(); it != run.workContributions.cend(); ++it)
        contributions.insert(it.key(), it.value().toString(QUuid::WithoutBraces));
    m.insert(QStringLiteral("workContributions"), contributions);
    m.insert(QStringLiteral("missingWork"), strings(run.missingWork));
    QCborMap missingCauses;
    for (auto it = run.missingCauses.cbegin(); it != run.missingCauses.cend(); ++it)
        missingCauses.insert(it.key(), it.value());
    m.insert(QStringLiteral("missingCauses"), missingCauses);
    m.insert(QStringLiteral("terminalCause"), run.terminalCause);
    m.insert(QStringLiteral("terminalContributionId"), run.terminalContributionId.toString(QUuid::WithoutBraces));
    return m.toCborValue().toCbor();
}

LifecycleRun decodeLifecycleRun(const QByteArray &encoded, QString *error) {
    if (error) error->clear(); LifecycleRun run; const QCborValue value = QCborValue::fromCbor(encoded);
    if (!value.isMap()) { setError(error, "lifecycle payload is not a CBOR map"); return {}; }
    const QCborMap m = value.toMap(); run.schemaVersion = m.value("schemaVersion").toInteger(); run.runId = QUuid(m.value("runId").toString());
    run.kind = m.value("kind").toString(); run.policyId = m.value("policyId").toString(); run.requestedAt = QDateTime::fromString(m.value("requestedAt").toString(), Qt::ISODateWithMs);
    run.inputHighWaterMark = m.value("inputHighWaterMark").toInteger(); run.requiredCapabilities = stringList(m.value("requiredCapabilities"));
    run.optionalCapabilities = stringList(m.value("optionalCapabilities")); run.status = static_cast<LifecycleRunStatus>(m.value("status").toInteger());
    run.completedWork = stringList(m.value("completedWork"));
    const QCborMap contributions = m.value("workContributions").toMap();
    for (auto it = contributions.cbegin(); it != contributions.cend(); ++it)
        run.workContributions.insert(it.key().toString(), QUuid(it.value().toString()));
    run.missingWork = stringList(m.value("missingWork"));
    const QCborMap missingCauses = m.value("missingCauses").toMap();
    for (auto it = missingCauses.cbegin(); it != missingCauses.cend(); ++it)
        run.missingCauses.insert(it.key().toString(), it.value().toString());
    run.terminalCause = m.value("terminalCause").toString();
    run.terminalContributionId = QUuid(m.value("terminalContributionId").toString());
    if (!run.isValid()) { setError(error, "invalid lifecycle run"); return {}; } return run;
}
} // namespace cybou
