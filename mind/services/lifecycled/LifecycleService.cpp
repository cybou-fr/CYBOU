// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
#include "LifecycleService.h"
#include "LifecycleSchedulingPolicy.h"
#include "cybou/fabric/FabricCodec.h"
#include "cybou/fabric/OrganBus.h"
#include "cybou/fabric/RpcClient.h"
#include <QCborMap>
#include <QCborArray>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSaveFile>
#include <QtGlobal>
namespace cybou {
namespace {
const QString kConsolidationConsumer = QStringLiteral("lifecycle.consolidation");
const QUuid kScheduledRunNamespace(
    QStringLiteral("ea65af07-61cc-5d75-9178-e849fddc9238"));
const QUuid kLifecycleTerminalNamespace(
    QStringLiteral("156fb70e-a26c-56f1-9540-ad81cc3a3629"));
QUuid scheduledRunId(const QUuid &capabilitySnapshotId, const QUuid &homeostasisSnapshotId)
{
    return QUuid::createUuidV5(
        kScheduledRunNamespace,
        QStringLiteral("event-backlog-v1:%1:%2")
            .arg(capabilitySnapshotId.toString(QUuid::WithoutBraces),
                 homeostasisSnapshotId.toString(QUuid::WithoutBraces))
            .toUtf8());
}
QUuid terminalContributionId(const QUuid &runId)
{
    return QUuid::createUuidV5(
        kLifecycleTerminalNamespace,
        QStringLiteral("lifecycle:%1:completed")
            .arg(runId.toString(QUuid::WithoutBraces)).toUtf8());
}
QByteArray schedulerReply(
    const QString &outcome,
    const QString &runId,
    const QString &reason)
{
    return FabricCodec::encodeMap({
        {QStringLiteral("outcome"), outcome},
        {QStringLiteral("runId"), runId},
        {QStringLiteral("reason"), reason},
    });
}
LifecycleMode modeFrom(const QString &s) { for (int i=1;i<=7;++i) { auto m=static_cast<LifecycleMode>(i); if (lifecycleModeToString(m)==s) return m; } return static_cast<LifecycleMode>(0); }
LifecycleRunStatus statusFrom(const QString &s) { for (int i=1;i<=5;++i) { auto v=static_cast<LifecycleRunStatus>(i); if (lifecycleRunStatusToString(v)==s) return v; } return static_cast<LifecycleRunStatus>(0); }
void failpoint(const char *name) { if (qEnvironmentVariable("CYBOU_LIFECYCLE_FAILPOINT") == QLatin1String(name)) qFatal("lifecycled fault injection: %s", name); }
int activityCooldownMs() { bool ok=false; const int value=qEnvironmentVariableIntValue("CYBOU_LIFECYCLE_ACTIVITY_COOLDOWN_MS",&ok); return ok?qBound(0,value,3600000):60000; }
int ownerTimeoutMs() { bool ok=false; const int value=qEnvironmentVariableIntValue("CYBOU_LIFECYCLE_OWNER_TIMEOUT_MS",&ok); return ok?qBound(50,value,60000):5000; }
}
LifecycleService::LifecycleService(const QString &path, QObject *parent):QObject(parent),m_path(path)
{
    m_ready = load();
    if (!m_ready || !m_events.isOpen()
        || !m_events.ensureConsumer(kConsolidationConsumer, 0)) return;
    if (m_hasRun && m_run.status == LifecycleRunStatus::Completed)
        m_events.advanceConsumer(kConsolidationConsumer, m_run.inputHighWaterMark);
    if (!qEnvironmentVariableIsSet("CYBOU_LIFECYCLE_DISABLE_AUTO_SCHEDULING")) {
        m_schedulerDebounce.setSingleShot(true);
        m_schedulerDebounce.setInterval(100);
        connect(&m_health, &HealthClient::changed, &m_schedulerDebounce,
                qOverload<>(&QTimer::start));
        connect(&m_schedulerDebounce, &QTimer::timeout, this,
                [this]() { RunSchedulingCycle(); });
        m_schedulerTimer.setInterval(30000);
        connect(&m_schedulerTimer, &QTimer::timeout, this,
                [this]() { RunSchedulingCycle(); });
        m_schedulerTimer.start();
    }
}
bool LifecycleService::load() {
    QFile f(m_path); if (!f.exists()) return save();
    if (!f.open(QIODevice::ReadOnly)) { m_error="cannot read lifecycle state"; return false; }
    auto o=QJsonDocument::fromJson(f.readAll()).object();
    const int version=o["version"].toInt(0);
    if(version<0||version>2){m_error="unsupported lifecycle state version";return false;}
    const bool migrate=version<2;
    if(migrate){const QString backup=m_path+".pre-v1";if(!QFile::exists(backup)&&!QFile::copy(m_path,backup)){m_error="cannot back up legacy lifecycle state";return false;}}
    m_mode=modeFrom(o["mode"].toString());
    if (static_cast<int>(m_mode)==0) { m_error="invalid lifecycle mode"; return false; }
    const QByteArray run=QByteArray::fromBase64(o["run"].toString().toLatin1());
    if (!run.isEmpty()) { QString e; m_run=decodeLifecycleRun(run,&e); if(!e.isEmpty()){m_error=e;return false;} m_hasRun=true; }
    m_lastUserActivityAt=QDateTime::fromString(o["lastUserActivityAt"].toString(),Qt::ISODateWithMs).toUTC();
    m_schedulerCooldownUntil=QDateTime::fromString(o["schedulerCooldownUntil"].toString(),Qt::ISODateWithMs).toUTC();
    if (m_hasRun && m_run.status==LifecycleRunStatus::Active) m_mode=LifecycleMode::Recovering;
    return migrate||m_mode==LifecycleMode::Recovering ? save() : true;
}
bool LifecycleService::save() {
    QDir().mkpath(QFileInfo(m_path).absolutePath()); QJsonObject o;
    o["version"]=2; o["mode"]=lifecycleModeToString(m_mode);
    o["run"]=m_hasRun?QString::fromLatin1(encodeLifecycleRun(m_run).toBase64()):QString();
    o["lastUserActivityAt"]=m_lastUserActivityAt.isValid()?m_lastUserActivityAt.toUTC().toString(Qt::ISODateWithMs):QString();
    o["schedulerCooldownUntil"]=m_schedulerCooldownUntil.isValid()?m_schedulerCooldownUntil.toUTC().toString(Qt::ISODateWithMs):QString();
    QSaveFile f(m_path); if(!f.open(QIODevice::WriteOnly)){m_error="cannot write lifecycle state";return false;}
    f.write(QJsonDocument(o).toJson(QJsonDocument::Compact)); if(!f.commit()){m_error="cannot commit lifecycle state";return false;} Q_EMIT Changed(); return true;
}
QByteArray LifecycleService::State() const { QVariantMap m; m["mode"]=lifecycleModeToString(m_mode); m["hasRun"]=m_hasRun;m["lastUserActivityAt"]=m_lastUserActivityAt;m["schedulerCooldownUntil"]=m_schedulerCooldownUntil;m["schedulerCooldownActive"]=m_schedulerCooldownUntil.isValid()&&m_schedulerCooldownUntil>QDateTime::currentDateTimeUtc(); if(m_hasRun){m["runId"]=m_run.runId.toString(QUuid::WithoutBraces);m["kind"]=m_run.kind;m["policyId"]=m_run.policyId;m["requestedAt"]=m_run.requestedAt;m["status"]=lifecycleRunStatusToString(m_run.status);m["inputHighWaterMark"]=m_run.inputHighWaterMark;m["requiredCapabilities"]=m_run.requiredCapabilities;m["optionalCapabilities"]=m_run.optionalCapabilities;m["completedWork"]=m_run.completedWork;m["missingWork"]=m_run.missingWork;QVariantMap refs;for(auto it=m_run.workContributions.cbegin();it!=m_run.workContributions.cend();++it)refs[it.key()]=it.value().toString(QUuid::WithoutBraces);m["workContributions"]=refs;QVariantMap causes;for(auto it=m_run.missingCauses.cbegin();it!=m_run.missingCauses.cend();++it)causes[it.key()]=it.value();m["missingCauses"]=causes;m["terminalCause"]=m_run.terminalCause;m["terminalContributionId"]=m_run.terminalContributionId.toString(QUuid::WithoutBraces);} return FabricCodec::encodeMap(m); }
QByteArray LifecycleService::EvaluateScheduling() const
{
    const SchedulingEvaluation evaluation = LifecycleSchedulingPolicy::evaluate(
        m_mode, m_hasRun && !m_run.isTerminal(), m_health.snapshot(), m_health.measurements(), false, QDateTime::currentDateTimeUtc(), m_schedulerCooldownUntil);
    return FabricCodec::encodeMap(evaluation.toMap());
}
QString LifecycleService::ExecuteSchedulingDecision(
    const QString &capabilitySnapshotId,
    const QString &homeostasisSnapshotId)
{
    const QUuid capabilityId(capabilitySnapshotId);
    const QUuid homeostasisId(homeostasisSnapshotId);
    if (capabilityId.isNull() || homeostasisId.isNull()) {
        m_error = QStringLiteral("invalid scheduling evidence identity");
        return {};
    }
    const QUuid runId = scheduledRunId(capabilityId, homeostasisId);
    if (m_hasRun && m_run.runId == runId)
        return runId.toString(QUuid::WithoutBraces);
    if (m_events.contains(terminalContributionId(runId)))
        return runId.toString(QUuid::WithoutBraces);

    const CapabilitySnapshot capabilities = m_health.snapshot();
    const HomeostasisSnapshot homeostasis = m_health.measurements();
    if (capabilities.snapshotId != capabilityId || homeostasis.snapshotId != homeostasisId) {
        // Refusing to start a run on evidence that has been replaced is correct: the decision was
        // reasoned about one snapshot and healthd has since produced another. But it is a race, not
        // a fault - healthd refreshes on a 30 s timer and on every bus owner change, so a refresh
        // landing between the caller's snapshot and this one is ordinary. Marking it retryable is
        // what separates "ask again" from "something is broken"; reporting it as a failure made an
        // ordinary race look like a defect and produced an intermittent test failure with no
        // information attached to it.
        m_error = QStringLiteral("scheduling evidence was superseded by a newer health snapshot");
        m_lastSchedulingRaceLost = true;
        return {};
    }
    m_lastSchedulingRaceLost = false;
    const SchedulingEvaluation evaluation = LifecycleSchedulingPolicy::evaluate(
        m_mode, m_hasRun && !m_run.isTerminal(), capabilities, homeostasis, false, QDateTime::currentDateTimeUtc(), m_schedulerCooldownUntil);
    if (evaluation.decision != SchedulingDecision::Run) {
        m_error = evaluation.reason;
        return {};
    }
    if (!m_events.isOpen() || m_events.count() == 0) {
        m_error = QStringLiteral("Event1 is unavailable or empty");
        return {};
    }

    LifecycleRun run;
    run.runId = runId;
    run.kind = QStringLiteral("consolidation");
    run.policyId = QStringLiteral("event-backlog-v1:%1")
                       .arg(homeostasisId.toString(QUuid::WithoutBraces));
    run.requestedAt = QDateTime::currentDateTimeUtc();
    run.inputHighWaterMark = m_events.count();
    run.requiredCapabilities = evaluation.eligibleWorkers;
    run.optionalCapabilities = evaluation.missingWorkers.keys();
    if (!BeginRun(encodeLifecycleRun(run))) return {};
    m_error.clear();
    return runId.toString(QUuid::WithoutBraces);
}
QByteArray LifecycleService::continueScheduledRun()
{
    const QString runId = m_run.runId.toString(QUuid::WithoutBraces);
    if (m_mode == LifecycleMode::Recovering && !ResumeRun())
        return schedulerReply(QStringLiteral("failed"), runId, m_error);
    if (m_mode != LifecycleMode::Consolidating)
        return schedulerReply(
            QStringLiteral("deferred"), runId,
            QStringLiteral("scheduled run is not dispatchable in the current mode"));
    if (!startScheduledDispatch())
        return schedulerReply(QStringLiteral("failed"), runId, m_error);
    return schedulerReply(QStringLiteral("started"), runId, QString());
}

bool LifecycleService::startScheduledDispatch()
{
    if (!m_hasRun || m_run.status != LifecycleRunStatus::Active
        || m_mode != LifecycleMode::Consolidating
        || !m_run.policyId.startsWith(QStringLiteral("event-backlog-v1:"))) {
        m_error = QStringLiteral("no dispatchable scheduled lifecycle run");
        return false;
    }
    if (m_scheduledDispatchInFlight) return true;
    dispatchNextScheduledOwner();
    return true;
}

void LifecycleService::dispatchNextScheduledOwner()
{
    if (!m_hasRun || m_run.status != LifecycleRunStatus::Active
        || m_mode != LifecycleMode::Consolidating) {
        m_scheduledDispatchInFlight = false;
        m_scheduledOwner.reset();
        return;
    }
    const QStringList requested = m_run.requiredCapabilities + m_run.optionalCapabilities;
    QString capability;
    for (const QString &candidate : requested) {
        if (!m_run.completedWork.contains(candidate) && !m_run.missingWork.contains(candidate)) {
            capability = candidate;
            break;
        }
    }
    if (capability.isEmpty()) {
        m_scheduledDispatchInFlight = false;
        m_scheduledOwner.reset();
        FinishRun(QStringLiteral("completed"), QStringLiteral("event backlog scheduling policy"));
        return;
    }
    const BusEndpoint *endpoint = capability == QStringLiteral("predictor") ? &kPredictorEndpoint
        : capability == QStringLiteral("workspace") ? &kWorkspaceEndpoint : nullptr;
    if (!endpoint) {
        if (m_run.optionalCapabilities.contains(capability)) {
            if (MarkMissing(capability, QStringLiteral("unsupported optional capability")))
                dispatchNextScheduledOwner();
        } else {
            FinishRun(QStringLiteral("failed"),
                      QStringLiteral("unsupported required capability: %1").arg(capability));
        }
        return;
    }
    const QUuid runId = m_run.runId;
    const qulonglong mark = m_run.inputHighWaterMark;
    const QString key = WorkOperationKey(capability);
    m_scheduledDispatchInFlight = true;
    m_scheduledOwner = std::make_unique<AsyncRpcClient>(*endpoint, RpcRetryPolicy{}, this);
    m_scheduledOwner->call(
        QStringLiteral("Consolidate"),
        {runId.toString(QUuid::WithoutBraces), key, QVariant::fromValue<qulonglong>(mark)},
        RpcOperationSemantics::IdempotentMutation,
        [this, runId, capability, key, mark](const RpcResult &result) {
            handleScheduledOwnerResult(runId, capability, key, mark, result);
        },
        ownerTimeoutMs());
}

void LifecycleService::handleScheduledOwnerResult(
    const QUuid &runId, const QString &capability, const QString &operationKey,
    qulonglong mark, const RpcResult &result)
{
    m_scheduledDispatchInFlight = false;
    if (auto *owner = m_scheduledOwner.release()) owner->deleteLater();
    if (!m_hasRun || m_run.runId != runId || m_run.status != LifecycleRunStatus::Active
        || m_mode != LifecycleMode::Consolidating) return;
    QString codecError;
    const QByteArray encoded = result.reply.arguments().value(0).toByteArray();
    const QVariantMap receipt = result.succeeded()
        ? FabricCodec::decodeMap(encoded, &codecError) : QVariantMap{};
    const QUuid contributionId(receipt.value(QStringLiteral("contributionId")).toString());
    const bool valid = result.succeeded() && codecError.isEmpty()
        && receipt.value(QStringLiteral("accepted")).toBool()
        && receipt.value(QStringLiteral("owner")).toString() == capability
        && receipt.value(QStringLiteral("operationKey")).toString() == operationKey
        && receipt.value(QStringLiteral("inputHighWaterMark")).toULongLong() == mark
        && !contributionId.isNull();
    if (valid && acceptOwnerResult(capability, operationKey, mark, contributionId)) {
        dispatchNextScheduledOwner();
        return;
    }
    const QString cause = result.errorMessage.isEmpty()
        ? QStringLiteral("owner rejected scheduled work") : result.errorMessage;
    if (m_run.optionalCapabilities.contains(capability)) {
        if (MarkMissing(capability, cause)) dispatchNextScheduledOwner();
    } else {
        FinishRun(QStringLiteral("failed"), cause);
    }
}

QByteArray LifecycleService::RunSchedulingCycle()
{
    if (m_hasRun && m_run.status == LifecycleRunStatus::Active
        && m_run.policyId.startsWith(QStringLiteral("event-backlog-v1:")))
        return continueScheduledRun();

    const CapabilitySnapshot capabilities = m_health.snapshot();
    const HomeostasisSnapshot homeostasis = m_health.measurements();
    const SchedulingEvaluation evaluation = LifecycleSchedulingPolicy::evaluate(
        m_mode, m_hasRun && !m_run.isTerminal(), capabilities, homeostasis, false, QDateTime::currentDateTimeUtc(), m_schedulerCooldownUntil);
    if (evaluation.decision != SchedulingDecision::Run)
        return schedulerReply(
            evaluation.decision == SchedulingDecision::Block
                ? QStringLiteral("blocked") : QStringLiteral("deferred"),
            QString(), evaluation.reason);

    const QString runId = ExecuteSchedulingDecision(
        capabilities.snapshotId.toString(QUuid::WithoutBraces),
        homeostasis.snapshotId.toString(QUuid::WithoutBraces));
    // A lost race is deferred, not failed. "deferred" already means the run did not start and a
    // later attempt may succeed, which is exactly the situation; "failed" means the scheduler could
    // not do its job.
    if (runId.isEmpty())
        return schedulerReply(
            m_lastSchedulingRaceLost ? QStringLiteral("deferred") : QStringLiteral("failed"),
            QString(),
            m_error);
    failpoint("after-scheduled-execute");
    return continueScheduledRun();
}
bool LifecycleService::NotifyUserActivity(const QString &cause)
{
    const QString reason=cause.trimmed();
    if(reason.isEmpty()){m_error="user activity cause is empty";return false;}
    const auto oldRun=m_run;const auto oldMode=m_mode;const auto oldActivity=m_lastUserActivityAt;const auto oldCooldown=m_schedulerCooldownUntil;
    m_lastUserActivityAt=QDateTime::currentDateTimeUtc();
    m_schedulerCooldownUntil=m_lastUserActivityAt.addMSecs(activityCooldownMs());
    const bool automatic=m_hasRun&&m_run.status==LifecycleRunStatus::Active&&m_run.policyId.startsWith(QStringLiteral("event-backlog-v1:"));
    if(automatic){m_run.status=LifecycleRunStatus::Interrupted;m_run.terminalCause=reason;m_mode=LifecycleMode::Awake;}
    else if(m_mode==LifecycleMode::Idle)m_mode=LifecycleMode::Awake;
    if(!save()){m_run=oldRun;m_mode=oldMode;m_lastUserActivityAt=oldActivity;m_schedulerCooldownUntil=oldCooldown;return false;}
    m_error.clear();return true;
}
bool LifecycleService::Transition(const QString &mode) { auto next=modeFrom(mode); if(static_cast<int>(next)==0||!canTransition(m_mode,next)){m_error="illegal lifecycle transition";return false;} auto old=m_mode;m_mode=next;if(!save()){m_mode=old;return false;}m_error.clear();return true; }
bool LifecycleService::BeginRun(const QByteArray &encoded) { QString e; auto run=decodeLifecycleRun(encoded,&e); if(!e.isEmpty()||run.status!=LifecycleRunStatus::Requested||m_hasRun&&!m_run.isTerminal()||m_mode!=LifecycleMode::Idle){m_error=e.isEmpty()?"cannot begin lifecycle run":e;return false;}const auto oldRun=m_run;const auto oldMode=m_mode;const bool oldHasRun=m_hasRun;m_run=run;m_run.status=LifecycleRunStatus::Active;m_hasRun=true;m_mode=LifecycleMode::Consolidating;if(!save()){m_run=oldRun;m_mode=oldMode;m_hasRun=oldHasRun;return false;}return true; }
QString LifecycleService::RequestRun(const QString &kind,const QString &policyId,qulonglong mark,const QStringList &required,const QStringList &optional){LifecycleRun run;run.runId=QUuid::createUuid();run.kind=kind;run.policyId=policyId;run.requestedAt=QDateTime::currentDateTimeUtc();run.inputHighWaterMark=mark;run.requiredCapabilities=required;run.optionalCapabilities=optional;return BeginRun(encodeLifecycleRun(run))?run.runId.toString(QUuid::WithoutBraces):QString();}
QString LifecycleService::RequestRunAtCurrentHead(const QString &kind,const QString &policyId,const QStringList &required,const QStringList &optional){if(!m_events.isOpen()){m_error="Event1 is unavailable";return {};}const quint64 mark=m_events.count();if(mark==0){m_error="cannot consolidate an empty Event1 biography";return {};}return RequestRun(kind,policyId,mark,required,optional);}
bool LifecycleService::requestedCapability(const QString &capability) const { return m_run.requiredCapabilities.contains(capability)||m_run.optionalCapabilities.contains(capability); }
QString LifecycleService::WorkOperationKey(const QString &capability) const { if(!m_hasRun||m_run.status!=LifecycleRunStatus::Active||!requestedCapability(capability))return {};return QStringLiteral("%1:%2:%3").arg(m_run.runId.toString(QUuid::WithoutBraces),capability,QString::number(m_run.inputHighWaterMark)); }
bool LifecycleService::AcknowledgeWork(const QString &capability,const QString &operationKey,qulonglong mark){if(!m_hasRun||m_run.status!=LifecycleRunStatus::Active||!requestedCapability(capability)||mark!=m_run.inputHighWaterMark||operationKey!=WorkOperationKey(capability)){m_error="invalid lifecycle work acknowledgement";return false;}if(m_run.missingWork.contains(capability)){m_error="capability already marked missing";return false;}if(m_run.completedWork.contains(capability)){m_error.clear();return true;}const auto oldRun=m_run;m_run.completedWork.append(capability);if(!save()){m_run=oldRun;return false;}m_error.clear();return true;}
bool LifecycleService::acceptOwnerResult(const QString &capability,const QString &operationKey,qulonglong mark,const QUuid &contributionId){if(!m_hasRun||m_run.status!=LifecycleRunStatus::Active||!requestedCapability(capability)||mark!=m_run.inputHighWaterMark||operationKey!=WorkOperationKey(capability)||contributionId.isNull()||!m_events.contains(contributionId)){m_error="invalid or unaccepted owner result";return false;}if(m_run.missingWork.contains(capability)){m_error="capability already marked missing";return false;}if(m_run.completedWork.contains(capability)){if(m_run.workContributions.value(capability)==contributionId){m_error.clear();return true;}m_error="owner contribution changed for completed work";return false;}const auto oldRun=m_run;m_run.completedWork.append(capability);m_run.workContributions.insert(capability,contributionId);if(!save()){m_run=oldRun;return false;}m_error.clear();return true;}
bool LifecycleService::MarkMissing(const QString &capability,const QString &cause){const QString reason=cause.trimmed();if(!m_hasRun||m_run.status!=LifecycleRunStatus::Active||!m_run.optionalCapabilities.contains(capability)||reason.isEmpty()||m_run.completedWork.contains(capability)){m_error="invalid missing capability";return false;}if(m_run.missingWork.contains(capability)&&!m_run.missingCauses.value(capability).isEmpty()){if(m_run.missingCauses.value(capability)==reason){m_error.clear();return true;}m_error="missing capability cause changed";return false;}const auto oldRun=m_run;if(!m_run.missingWork.contains(capability))m_run.missingWork.append(capability);m_run.missingCauses.insert(capability,reason);if(!save()){m_run=oldRun;return false;}m_error.clear();return true;}
bool LifecycleService::Dispatch()
{
    if (!m_hasRun || m_run.status != LifecycleRunStatus::Active
        || m_mode != LifecycleMode::Consolidating) {
        m_error = "no dispatchable lifecycle run";
        return false;
    }
    const QStringList requested = m_run.requiredCapabilities + m_run.optionalCapabilities;
    for (const auto &capability : requested) {
        if (m_run.completedWork.contains(capability)
            || m_run.missingWork.contains(capability)) continue;
        const BusEndpoint *endpoint = nullptr;
        if (capability == QStringLiteral("predictor")) endpoint = &kPredictorEndpoint;
        else if (capability == QStringLiteral("workspace")) endpoint = &kWorkspaceEndpoint;
        if (!endpoint) {
            if (m_run.optionalCapabilities.contains(capability)) {
                if (!MarkMissing(capability, QStringLiteral("unsupported optional capability"))) return false;
                continue;
            }
            m_error = QStringLiteral("unsupported required capability: %1").arg(capability);
            return false;
        }
        const QString key = WorkOperationKey(capability);
        RpcClient owner(*endpoint);
        const QByteArray encoded = owner.callBytes(
            QStringLiteral("Consolidate"),
            {m_run.runId.toString(QUuid::WithoutBraces), key,
             QVariant::fromValue<qulonglong>(m_run.inputHighWaterMark)});
        QString codecError;
        const QVariantMap receipt = FabricCodec::decodeMap(encoded, &codecError);
        const QUuid contributionId(receipt.value(QStringLiteral("contributionId")).toString());
        const bool valid = codecError.isEmpty()
            && receipt.value(QStringLiteral("accepted")).toBool()
            && receipt.value(QStringLiteral("owner")).toString() == capability
            && receipt.value(QStringLiteral("operationKey")).toString() == key
            && receipt.value(QStringLiteral("inputHighWaterMark")).toULongLong()
                == m_run.inputHighWaterMark
            && !contributionId.isNull();
        if (!valid) {
            if (m_run.optionalCapabilities.contains(capability)) {
                if (!MarkMissing(
                        capability,
                        owner.lastError().isEmpty()
                            ? QStringLiteral("optional owner rejected work")
                            : owner.lastError())) return false;
                continue;
            }
            m_error = owner.lastError().isEmpty()
                ? QStringLiteral("required owner rejected work: %1").arg(capability)
                : owner.lastError();
            return false;
        }
        failpoint("after-owner-commit");
        if (!acceptOwnerResult(capability, key, m_run.inputHighWaterMark, contributionId)) return false;
    }
    m_error.clear();
    return true;
}
bool LifecycleService::ResumeRun(){if(!m_hasRun||m_run.status!=LifecycleRunStatus::Active||m_mode!=LifecycleMode::Recovering){m_error="no recoverable lifecycle run";return false;}const auto oldMode=m_mode;m_mode=LifecycleMode::Consolidating;if(!save()){m_mode=oldMode;return false;}m_error.clear();return true;}
bool LifecycleService::commitTerminalContribution(const QString &cause){if(m_run.workContributions.size()!=m_run.completedWork.size()||m_run.workContributions.isEmpty()){m_error="completed work lacks durable Event1 contributions";return false;}QList<QUuid> ids=m_run.workContributions.values();for(const auto &id:ids)if(!m_events.contains(id)){m_error="owner contribution disappeared from Event1";return false;}const QUuid terminalId=QUuid::createUuidV5(QUuid(QStringLiteral("156fb70e-a26c-56f1-9540-ad81cc3a3629")),QStringLiteral("lifecycle:%1:completed").arg(m_run.runId.toString(QUuid::WithoutBraces)).toUtf8());if(!m_events.contains(terminalId)){CognitiveEnvelope terminal;terminal.messageId=terminalId;terminal.correlationId=m_run.runId;terminal.causationId=ids.takeFirst();terminal.evidence=ids;terminal.originOrgan=QStringLiteral("lifecycled");terminal.originNode=QStringLiteral("local");terminal.kind=ContributionKind::Outcome;terminal.wallTime=QDateTime::currentDateTimeUtc();terminal.privacy=PrivacyClass::Local;terminal.capabilityScope=QStringLiteral("lifecycle.consolidation");QCborMap payload;payload[QStringLiteral("runId")]=m_run.runId.toString(QUuid::WithoutBraces);payload[QStringLiteral("status")]=QStringLiteral("completed");payload[QStringLiteral("cause")]=cause;payload[QStringLiteral("inputHighWaterMark")]=static_cast<qint64>(m_run.inputHighWaterMark);payload[QStringLiteral("completedCapabilities")]=QCborArray::fromStringList(m_run.completedWork);payload[QStringLiteral("missingCapabilities")]=QCborArray::fromStringList(m_run.missingWork);QCborMap missingCauses;for(auto it=m_run.missingCauses.cbegin();it!=m_run.missingCauses.cend();++it)missingCauses.insert(it.key(),it.value());payload[QStringLiteral("missingCauses")]=missingCauses;terminal.payloadCbor=payload.toCborValue().toCbor();if(m_events.append(terminal)==0){m_error=m_events.lastError();return false;}}failpoint("after-terminal-commit");m_run.terminalContributionId=terminalId;return true;}
bool LifecycleService::FinishRun(const QString &status,const QString &cause){auto s=statusFrom(status);if(!m_hasRun||m_run.status!=LifecycleRunStatus::Active||(s!=LifecycleRunStatus::Completed&&s!=LifecycleRunStatus::Interrupted&&s!=LifecycleRunStatus::Failed)||cause.trimmed().isEmpty()){m_error="invalid lifecycle terminal transition";return false;}const auto oldRun=m_run;const auto oldMode=m_mode;if(s==LifecycleRunStatus::Completed){for(const auto &capability:m_run.requiredCapabilities)if(!m_run.completedWork.contains(capability)){m_error="required lifecycle work is incomplete";return false;}for(const auto &capability:m_run.optionalCapabilities)if(!m_run.completedWork.contains(capability)&&!m_run.missingWork.contains(capability)){m_error="optional lifecycle work is unresolved";return false;}if(!commitTerminalContribution(cause))return false;}m_run.status=s;m_run.terminalCause=cause;m_mode=(s==LifecycleRunStatus::Completed?(m_run.missingWork.isEmpty()?LifecycleMode::Awake:LifecycleMode::Degraded):LifecycleMode::Recovering);if(!save()){m_run=oldRun;m_mode=oldMode;return false;}if(s==LifecycleRunStatus::Completed){failpoint("after-terminal-state-commit");m_events.advanceConsumer(kConsolidationConsumer,m_run.inputHighWaterMark);}return true;}
}
