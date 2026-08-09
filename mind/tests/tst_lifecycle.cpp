// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
#include "LifecycleService.h"
#include "cybou/fabric/FabricCodec.h"
#include <QTemporaryDir>
#include <QTest>
using namespace cybou;
class TestLifecycle:public QObject{Q_OBJECT private Q_SLOTS:
void persistsAndRecoversActiveRun(){QTemporaryDir root;auto path=root.filePath("lifecycle/state.json");QUuid id;
 {LifecycleService s(path);QVERIFY(s.isReady());QVERIFY(s.Transition("idle"));LifecycleRun r;r.runId=QUuid::createUuid();id=r.runId;r.kind="consolidation";r.policyId="test";r.requestedAt=QDateTime::currentDateTimeUtc();QVERIFY(s.BeginRun(encodeLifecycleRun(r)));}
 {LifecycleService s(path);QVERIFY(s.isReady());QString e;auto state=FabricCodec::decodeMap(s.State(),&e);QVERIFY(e.isEmpty());QCOMPARE(state["mode"].toString(),QString("recovering"));QCOMPARE(state["runId"].toString(),id.toString(QUuid::WithoutBraces));QCOMPARE(state["status"].toString(),QString("active"));}}
void rejectsIllegalAndCorruptState(){QTemporaryDir root;auto path=root.filePath("state.json");LifecycleService s(path);QVERIFY(!s.Transition("consolidating"));QFile f(path);QVERIFY(f.open(QIODevice::WriteOnly|QIODevice::Truncate));f.write("{\"mode\":\"bogus\"}");f.close();LifecycleService broken(path);QVERIFY(!broken.isReady());}
};QTEST_MAIN(TestLifecycle)
#include "tst_lifecycle.moc"
