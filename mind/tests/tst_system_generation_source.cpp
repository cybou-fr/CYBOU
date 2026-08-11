// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
//
// The first perception source.
//
// The property under test is not "it reads a symlink". It is that a source which cannot be read
// produces a typed failure rather than an observation, because an adapter reporting its own failure
// as a value about the world is exactly what ADR-0027's separation of producer and source exists to
// prevent.

#include "cybou/perception/SystemGenerationSource.h"

#include <QDir>
#include <QFile>
#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

class TestSystemGenerationSource : public QObject
{
    Q_OBJECT

private:
    QDateTime m_now = QDateTime(QDate(2026, 8, 11), QTime(9, 0), Qt::UTC);

private Q_SLOTS:
    void readsTheBuildIdentityOfTheActiveSystem()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString store = dir.filePath(QStringLiteral("abc123-nixos-system-host-26.05"));
        QVERIFY(QDir().mkpath(store));
        const QString link = dir.filePath(QStringLiteral("current-system"));
        QVERIFY(QFile::link(store, link));

        const AcquisitionResult result = SystemGenerationSource(link).acquire(m_now);

        QVERIFY2(result.acquired(), qPrintable(result.detail));
        QVERIFY(result.observation.isValid());
        QCOMPARE(result.observation.sourceId, QStringLiteral("nixos.system"));
        QCOMPARE(result.observation.subject, QStringLiteral("current-system"));
        QCOMPARE(
            result.observation.value.toString(),
            QStringLiteral("abc123-nixos-system-host-26.05"));

        // Acquisition time is a fact about the world and must be what the caller observed, not when
        // anything later happened to accept it.
        QCOMPARE(result.observation.acquiredAt, m_now);
        QVERIFY(result.observation.freshnessUntil > m_now);
        QVERIFY(result.observation.isFreshAt(m_now));

        // Provenance has to be enough to challenge the value, so it names both ends of the read.
        QVERIFY(result.observation.provenance.contains(link));
        QVERIFY(result.observation.provenance.contains(store));
    }

    // The value must change when the system does, or nothing downstream could ever supersede an
    // earlier observation and the contradiction machinery would have nothing to work on.
    void aDifferentSystemIsADifferentValue()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString link = dir.filePath(QStringLiteral("current-system"));

        const QString first = dir.filePath(QStringLiteral("aaa-nixos-system-host-26.05"));
        QVERIFY(QDir().mkpath(first));
        QVERIFY(QFile::link(first, link));
        const AcquisitionResult before = SystemGenerationSource(link).acquire(m_now);
        QVERIFY(before.acquired());

        const QString second = dir.filePath(QStringLiteral("bbb-nixos-system-host-26.05"));
        QVERIFY(QDir().mkpath(second));
        QVERIFY(QFile::remove(link));
        QVERIFY(QFile::link(second, link));
        const AcquisitionResult after =
            SystemGenerationSource(link).acquire(m_now.addSecs(60));
        QVERIFY(after.acquired());

        QVERIFY(before.observation.value != after.observation.value);

        // Same source and subject, different acquisition: two observations, not one. The identity
        // must distinguish them or the second could never be recorded.
        QVERIFY(
            observationMessageId(
                before.observation.sourceId,
                before.observation.subject,
                before.observation.acquiredAt)
            != observationMessageId(
                after.observation.sourceId,
                after.observation.subject,
                after.observation.acquiredAt));
    }

    // Reading the same unchanged system twice at one instant is one acquisition. This is what makes
    // an adapter restart or a retry a durable no-op rather than a second contribution.
    void repeatedAcquisitionAtOneInstantIsOneObservation()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString store = dir.filePath(QStringLiteral("ccc-nixos-system-host-26.05"));
        QVERIFY(QDir().mkpath(store));
        const QString link = dir.filePath(QStringLiteral("current-system"));
        QVERIFY(QFile::link(store, link));

        const SystemGenerationSource source(link);
        const AcquisitionResult first = source.acquire(m_now);
        const AcquisitionResult second = source.acquire(m_now);
        QVERIFY(first.acquired());
        QVERIFY(second.acquired());

        QCOMPARE(
            observationMessageId(
                first.observation.sourceId,
                first.observation.subject,
                first.observation.acquiredAt),
            observationMessageId(
                second.observation.sourceId,
                second.observation.subject,
                second.observation.acquiredAt));
    }

    // An absent source is an ordinary answer, not an error, and never an observation.
    void absentSourceIsTypedNotObserved()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        const AcquisitionResult result =
            SystemGenerationSource(dir.filePath(QStringLiteral("nothing-here"))).acquire(m_now);

        QCOMPARE(result.status, AcquisitionStatus::SourceUnavailable);
        QVERIFY(!result.acquired());
        QVERIFY(!result.detail.isEmpty());
        // The failure must not have been smuggled in as a value.
        QVERIFY(!result.observation.isValid());
        QVERIFY(result.observation.value.isNull() || result.observation.value.isUndefined());
    }

    // Present but not what this source is defined in terms of. Distinct from absent: something is
    // there and it is wrong, which someone may want to act on differently.
    void presentButWrongShapeIsUnavailableNotObserved()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString path = dir.filePath(QStringLiteral("current-system"));
        QFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.write("not a symlink");
        file.close();

        const AcquisitionResult result = SystemGenerationSource(path).acquire(m_now);

        QCOMPARE(result.status, AcquisitionStatus::SourceUnavailable);
        QVERIFY(result.detail.contains(QStringLiteral("not a symbolic link")));
        QVERIFY(!result.observation.isValid());
    }

    // A dangling symlink still resolves to a target, so the build identity is still readable. The
    // system it names may be gone, but what was observed is what the link said - deciding whether
    // that is still true belongs to the epistemic projection, not to the adapter.
    void danglingLinkStillReportsWhatItSaid()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString link = dir.filePath(QStringLiteral("current-system"));
        QVERIFY(QFile::link(dir.filePath(QStringLiteral("ddd-nixos-system-gone")), link));

        const AcquisitionResult result = SystemGenerationSource(link).acquire(m_now);

        QVERIFY2(result.acquired(), qPrintable(result.detail));
        QCOMPARE(result.observation.value.toString(), QStringLiteral("ddd-nixos-system-gone"));
    }

    void freshnessHorizonIsDeclaredByTheSource()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString store = dir.filePath(QStringLiteral("eee-nixos-system-host-26.05"));
        QVERIFY(QDir().mkpath(store));
        const QString link = dir.filePath(QStringLiteral("current-system"));
        QVERIFY(QFile::link(store, link));

        const AcquisitionResult result = SystemGenerationSource(link, 60).acquire(m_now);
        QVERIFY(result.acquired());

        QCOMPARE(result.observation.freshnessUntil, m_now.addSecs(60));
        QVERIFY(result.observation.isFreshAt(m_now.addSecs(59)));
        QVERIFY(!result.observation.isFreshAt(m_now.addSecs(60)));

        // A nonsensical horizon is replaced by the default rather than producing an observation
        // that was never current.
        const AcquisitionResult defaulted = SystemGenerationSource(link, 0).acquire(m_now);
        QVERIFY(defaulted.acquired());
        QVERIFY(defaulted.observation.freshnessUntil > m_now);
    }
};

QTEST_MAIN(TestSystemGenerationSource)
#include "tst_system_generation_source.moc"
