// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
#include "LifecycleService.h"
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
namespace cybou {
namespace {
LifecycleMode modeFrom(const QString &s) { for (int i=1;i<=7;++i) { auto m=static_cast<LifecycleMode>(i); if (lifecycleModeToString(m)==s) return m; } return static_cast<LifecycleMode>(0); }
LifecycleRunStatus statusFrom(const QString &s) { for (int i=1;i<=5;++i) { auto v=static_cast<LifecycleRunStatus>(i); if (lifecycleRunStatusToString(v)==s) return v; } return static_cast<LifecycleRunStatus>(0); }
void failpoint(const char *name) { if (qEnvironmentVariable("CYBOU_LIFECYCLE_FAILPOINT") == QLatin1String(name)) qFatal("lifecycled fault injection: %s", name); }
}
LifecycleService::LifecycleService(const QString &path, QObject *parent):QObject(parent),m_path(path) { m_ready=load(); }
bool LifecycleService::load() {
    QFile f(m_path); if (!f.exists()) return save();
    if (!f.open(QIODevice::ReadOnly)) { m_error="cannot read lifecycle state"; return false; }
    auto o=QJsonDocument::fromJson(f.readAll()).object();
    const int version=o["version"].toInt(0);
    if(version<0||version>1){m_error="unsupported lifecycle state version";return false;}
    const bool migrate=version==0;
    if(migrate){const QString backup=m_path+".pre-v1";if(!QFile::exists(backup)&&!QFile::copy(m_path,backup)){m_error="cannot back up legacy lifecycle state";return false;}}
    m_mode=modeFrom(o["mode"].toString());
    if (static_cast<int>(m_mode)==0) { m_error="invalid lifecycle mode"; return false; }
    const QByteArray run=QByteArray::fromBase64(o["run"].toString().toLatin1());
    if (!run.isEmpty()) { QString e; m_run=decodeLifecycleRun(run,&e); if(!e.isEmpty()){m_error=e;return false;} m_hasRun=true; }
    if (m_hasRun && m_run.status==LifecycleRunStatus::Active) m_mode=LifecycleMode::Recovering;
    return migrate||m_mode==LifecycleMode::Recovering ? save() : true;
}
bool LifecycleService::save() {
    QDir().mkpath(QFileInfo(m_path).absolutePath()); QJsonObject o;
    o["version"]=1; o["mode"]=lifecycleModeToString(m_mode);
    o["run"]=m_hasRun?QString::fromLatin1(encodeLifecycleRun(m_run).toBase64()):QString();
    QSaveFile f(m_path); if(!f.open(QIODevice::WriteOnly)){m_error="cannot write lifecycle state";return false;}
    f.write(QJsonDocument(o).toJson(QJsonDocument::Compact)); if(!f.commit()){m_error="cannot commit lifecycle state";return false;} Q_EMIT Changed(); return true;
}
QByteArray LifecycleService::State() const { QVariantMap m; m["mode"]=lifecycleModeToString(m_mode); m["hasRun"]=m_hasRun; if(m_hasRun){m["runId"]=m_run.runId.toString(QUuid::WithoutBraces);m["kind"]=m_run.kind;m["policyId"]=m_run.policyId;m["requestedAt"]=m_run.requestedAt;m["status"]=lifecycleRunStatusToString(m_run.status);m["inputHighWaterMark"]=m_run.inputHighWaterMark;m["requiredCapabilities"]=m_run.requiredCapabilities;m["optionalCapabilities"]=m_run.optionalCapabilities;m["completedWork"]=m_run.completedWork;m["missingWork"]=m_run.missingWork;QVariantMap refs;for(auto it=m_run.workContributions.cbegin();it!=m_run.workContributions.cend();++it)refs[it.key()]=it.value().toString(QUuid::WithoutBraces);m["workContributions"]=refs;QVariantMap causes;for(auto it=m_run.missingCauses.cbegin();it!=m_run.missingCauses.cend();++it)causes[it.key()]=it.value();m["missingCauses"]=causes;m["terminalCause"]=m_run.terminalCause;m["terminalContributionId"]=m_run.terminalContributionId.toString(QUuid::WithoutBraces);} return FabricCodec::encodeMap(m); }
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
bool LifecycleService::FinishRun(const QString &status,const QString &cause){auto s=statusFrom(status);if(!m_hasRun||m_run.status!=LifecycleRunStatus::Active||(s!=LifecycleRunStatus::Completed&&s!=LifecycleRunStatus::Interrupted&&s!=LifecycleRunStatus::Failed)||cause.trimmed().isEmpty()){m_error="invalid lifecycle terminal transition";return false;}const auto oldRun=m_run;const auto oldMode=m_mode;if(s==LifecycleRunStatus::Completed){for(const auto &capability:m_run.requiredCapabilities)if(!m_run.completedWork.contains(capability)){m_error="required lifecycle work is incomplete";return false;}for(const auto &capability:m_run.optionalCapabilities)if(!m_run.completedWork.contains(capability)&&!m_run.missingWork.contains(capability)){m_error="optional lifecycle work is unresolved";return false;}if(!commitTerminalContribution(cause))return false;}m_run.status=s;m_run.terminalCause=cause;m_mode=(s==LifecycleRunStatus::Completed?(m_run.missingWork.isEmpty()?LifecycleMode::Awake:LifecycleMode::Degraded):LifecycleMode::Recovering);if(!save()){m_run=oldRun;m_mode=oldMode;return false;}return true;}
}
