// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#include "cybou/context/ConsumerRegistry.h"
#include "cybou/ipc/CallerIdentity.h"

#include <QFileInfo>
#include <QTest>

using namespace cybou;

class TestConsumerRegistry : public QObject
{
    Q_OBJECT

private slots:
    /// The ceiling comes from a verified name; anything else is the least privilege there is.
    void anUnknownConsumerGetsTheLeastPrivilege();

    /// A caller may ask for less than its ceiling, never for more.
    void aRequestAboveTheCeilingIsNotPermitted();

    /// A binary outside the installed directory is not a Mind binary, whatever it is called.
    void aBinaryNamedLikeAnOrganIsNotAnOrgan();
};

void TestConsumerRegistry::anUnknownConsumerGetsTheLeastPrivilege()
{
    // Nothing resolved at all: the only safe reading of "I could not tell who that was".
    QCOMPARE(ConsumerRegistry::ceilingFor(QString()), ConsumerTrust::Untrusted);
    QCOMPARE(ConsumerRegistry::ceilingFor(QStringLiteral("totally-safe")),
             ConsumerTrust::Untrusted);

    // A name that merely looks official buys nothing either.
    QCOMPARE(ConsumerRegistry::ceilingFor(QStringLiteral("cybou-contextd")),
             ConsumerTrust::Untrusted);

    // A real organ is bounded, which is what makes the refusals above meaningful.
    QCOMPARE(ConsumerRegistry::ceilingFor(QStringLiteral("predictord")), ConsumerTrust::Bounded);

    // And nothing is granted Full: the level exists for a consumer that does not exist yet, and
    // granting it in advance would hand it to whatever later carried that name.
    for (const QString &name : {QStringLiteral("predictord"), QStringLiteral("contextd"),
                                QStringLiteral("inspector"), QStringLiteral("presenced")}) {
        QVERIFY2(ConsumerRegistry::ceilingFor(name) != ConsumerTrust::Full, qPrintable(name));
    }
}

void TestConsumerRegistry::aRequestAboveTheCeilingIsNotPermitted()
{
    QVERIFY(ConsumerRegistry::permitsRequest(ConsumerTrust::Bounded, ConsumerTrust::Untrusted));
    QVERIFY(ConsumerRegistry::permitsRequest(ConsumerTrust::Bounded, ConsumerTrust::Bounded));
    QVERIFY(!ConsumerRegistry::permitsRequest(ConsumerTrust::Bounded, ConsumerTrust::Full));

    QVERIFY(!ConsumerRegistry::permitsRequest(ConsumerTrust::Untrusted, ConsumerTrust::Bounded));
    QVERIFY(!ConsumerRegistry::permitsRequest(ConsumerTrust::Untrusted, ConsumerTrust::Full));

    // Asking for less than permitted is always fine: a consumer narrowing its own request is the
    // one direction that needs no defending.
    QVERIFY(ConsumerRegistry::permitsRequest(ConsumerTrust::Full, ConsumerTrust::Untrusted));
    QVERIFY(ConsumerRegistry::permitsRequest(ConsumerTrust::Full, ConsumerTrust::Full));
}

void TestConsumerRegistry::aBinaryNamedLikeAnOrganIsNotAnOrgan()
{
    // The test binary itself resolves, which proves the directory rule is doing real work rather
    // than rejecting everything.
    const QString trusted = trustedBinaryDirectory();
    QVERIFY2(!trusted.isEmpty(), "the running executable must be resolvable");

    // Anyone can create this path. The name is right and the directory is not.
    QCOMPARE(mindBinaryNameForExecutable(QStringLiteral("/tmp/cybou-contextd")), QString());
    QCOMPARE(mindBinaryNameForExecutable(QStringLiteral("/home/someone/bin/cybou-epistemicd")),
             QString());
    QCOMPARE(mindBinaryNameForExecutable(QString()), QString());

    // In the trusted directory, the decoration the Nix wrapper adds is undone rather than
    // rejected -- otherwise this check would pass every test and fail every installation.
    QCOMPARE(mindBinaryNameForExecutable(trusted + QStringLiteral("/cybou-contextd")),
             QStringLiteral("contextd"));
    QCOMPARE(mindBinaryNameForExecutable(trusted + QStringLiteral("/.cybou-contextd-wrapped")),
             QStringLiteral("contextd"));

    // A file in the trusted directory that is not a Mind binary is still not one.
    QCOMPARE(mindBinaryNameForExecutable(trusted + QStringLiteral("/something-else")), QString());
}

QTEST_MAIN(TestConsumerRegistry)
#include "tst_consumer_registry.moc"
