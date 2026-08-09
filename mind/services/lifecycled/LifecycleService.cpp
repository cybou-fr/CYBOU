// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
#include "LifecycleService.h"
#include "cybou/fabric/FabricCodec.h"
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
    f.write(QJsonDocument(o).toJson(QJsonDocument::Compact)); if(!f.commit()){m_error="cannot commit lifecycle state";return false;} return true;
}
QByteArray LifecycleService::State() const { QVariantMap m; m["mode"]=lifecycleModeToString(m_mode); m["hasRun"]=m_hasRun; if(m_hasRun){m["runId"]=m_run.runId.toString(QUuid::WithoutBraces);m["status"]=lifecycleRunStatusToString(m_run.status);} return FabricCodec::encodeMap(m); }
bool LifecycleService::Transition(const QString &mode) { auto next=modeFrom(mode); if(static_cast<int>(next)==0||!canTransition(m_mode,next)){m_error="illegal lifecycle transition";return false;} auto old=m_mode;m_mode=next;if(!save()){m_mode=old;return false;}m_error.clear();return true; }
bool LifecycleService::BeginRun(const QByteArray &encoded) { QString e; auto run=decodeLifecycleRun(encoded,&e); if(!e.isEmpty()||run.status!=LifecycleRunStatus::Requested||m_hasRun&&!m_run.isTerminal()||m_mode!=LifecycleMode::Idle){m_error=e.isEmpty()?"cannot begin lifecycle run":e;return false;} m_run=run;m_run.status=LifecycleRunStatus::Active;m_hasRun=true;m_mode=LifecycleMode::Consolidating;return save(); }
QString LifecycleService::RequestRun(const QString &kind,const QString &policyId,qulonglong mark,const QStringList &required,const QStringList &optional){LifecycleRun run;run.runId=QUuid::createUuid();run.kind=kind;run.policyId=policyId;run.requestedAt=QDateTime::currentDateTimeUtc();run.inputHighWaterMark=mark;run.requiredCapabilities=required;run.optionalCapabilities=optional;return BeginRun(encodeLifecycleRun(run))?run.runId.toString(QUuid::WithoutBraces):QString();}
bool LifecycleService::FinishRun(const QString &status,const QString &cause){auto s=statusFrom(status);if(!m_hasRun||m_run.status!=LifecycleRunStatus::Active||(s!=LifecycleRunStatus::Completed&&s!=LifecycleRunStatus::Interrupted&&s!=LifecycleRunStatus::Failed)||cause.trimmed().isEmpty()){m_error="invalid lifecycle terminal transition";return false;}m_run.status=s;m_run.terminalCause=cause;m_mode=(s==LifecycleRunStatus::Completed)?LifecycleMode::Awake:LifecycleMode::Recovering;return save();}
}
