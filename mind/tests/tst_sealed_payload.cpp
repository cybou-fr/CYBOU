// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
//
// The storage primitive erasure rests on.
//
// ADR-0028's whole erasure guarantee reduces to one property: after a key is destroyed, what
// survives must not let anyone test a hypothesis about what was erased. That is a claim about this
// file and nothing else, which is why it is proven here before any state machine is built on top -
// a crash-safe protocol over a primitive that leaks would pass its own tests and fail the thing it
// exists for.

#include "cybou/crypto/SealedPayload.h"

#include <QSet>
#include <QTest>

using namespace cybou;

class TestSealedPayload : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void initTestCase()
    {
        QVERIFY2(Seal::isAvailable(), "the AEAD primitive must be usable");
    }

    void aSealedPayloadRoundTrips()
    {
        const QByteArray key = Seal::generateKey();
        QCOMPARE(key.size(), kSealKeyBytes);

        const QByteArray plaintext = QByteArrayLiteral("diagnosis: none of your business");
        const auto sealed = Seal::seal(plaintext, key);
        QVERIFY(sealed.has_value());
        QVERIFY(sealed->isValid());
        QVERIFY2(!sealed->ciphertext.contains(plaintext), "the plaintext must not survive in it");

        const auto opened = Seal::unseal(*sealed, key);
        QVERIFY(opened.has_value());
        QCOMPARE(*opened, plaintext);
    }

    // E3, first half: the same plaintext sealed twice yields different ciphertext, and therefore a
    // different commitment. Without this every other erasure property is decoration - identical
    // plaintext would produce an identical commitment, which is a plaintext digest wearing a hat.
    void theSamePlaintextSealsDifferentlyEveryTime()
    {
        const QByteArray key = Seal::generateKey();
        const QByteArray plaintext = QByteArrayLiteral("true");

        QSet<QByteArray> commitments;
        QSet<QByteArray> nonces;
        for (int i = 0; i < 32; ++i) {
            const auto sealed = Seal::seal(plaintext, key);
            QVERIFY(sealed.has_value());
            commitments.insert(Seal::sealedPayloadCommitment(*sealed));
            nonces.insert(sealed->nonce);
        }

        // A one-byte plaintext is the worst case on purpose: it is exactly the shape - a boolean, a
        // small enum, a diagnosis from a short list - whose digest would otherwise be trivially
        // reversible by enumeration.
        QCOMPARE(commitments.size(), 32);
        QCOMPARE(nonces.size(), 32);
    }

    // E3, second half: knowing the plaintext, the key, and the algorithm is still not enough to
    // reproduce a surviving commitment, because the nonce is not derivable from any of them.
    //
    // This is the property that makes a retained commitment safe to keep forever. A guesser holding
    // the erased record's commitment and a list of candidate values cannot test a single one.
    void aGuessedPlaintextCannotReproduceASurvivingCommitment()
    {
        const QByteArray key = Seal::generateKey();
        const QByteArray secret = QByteArrayLiteral("positive");

        const auto sealed = Seal::seal(secret, key);
        QVERIFY(sealed.has_value());
        const QByteArray surviving = Seal::sealedPayloadCommitment(*sealed);

        // The attacker's whole advantage, granted: the exact plaintext and the key itself.
        for (const QByteArray &guess :
             {QByteArrayLiteral("positive"), QByteArrayLiteral("negative"),
              QByteArrayLiteral("unknown")}) {
            for (int attempt = 0; attempt < 16; ++attempt) {
                const auto resealed = Seal::seal(guess, key);
                QVERIFY(resealed.has_value());
                QVERIFY2(
                    Seal::sealedPayloadCommitment(*resealed) != surviving,
                    "a commitment must not be reproducible from a guessed plaintext");
            }
        }
    }

    // And once the key is gone the ciphertext is inert, which is what "erasure destroys the key"
    // has to mean. A wrong key answers exactly as a corrupted ciphertext does: telling those apart
    // would tell an attacker which guess was closer.
    void aDestroyedKeyLeavesInertCiphertext()
    {
        const QByteArray key = Seal::generateKey();
        const auto sealed = Seal::seal(QByteArrayLiteral("secret"), key);
        QVERIFY(sealed.has_value());

        QVERIFY(!Seal::unseal(*sealed, Seal::generateKey()).has_value());

        SealedPayload tampered = *sealed;
        tampered.ciphertext[0] = static_cast<char>(tampered.ciphertext[0] ^ 0x01);
        QVERIFY(!Seal::unseal(tampered, key).has_value());
    }

    // Key wrapping is the same primitive, so a wrapped key in a backup is as opaque as the payload
    // it protects. That is what makes a restored backup decrypt only the records whose keys
    // survived.
    void aWrappedKeyIsUselessWithoutTheKeyEncryptingKey()
    {
        const QByteArray dataKey = Seal::generateKey();
        const QByteArray kek = Seal::generateKey();

        const auto wrapped = Seal::wrapKey(dataKey, kek);
        QVERIFY(wrapped.has_value());
        QVERIFY2(!wrapped->ciphertext.contains(dataKey), "the data key must not survive in it");

        const auto unwrapped = Seal::unwrapKey(*wrapped, kek);
        QVERIFY(unwrapped.has_value());
        QCOMPARE(*unwrapped, dataKey);

        QVERIFY(!Seal::unwrapKey(*wrapped, Seal::generateKey()).has_value());
    }

    // Key domains are opaque identifiers, never names. A domain called "medical" or "location"
    // would leak the category of the forgotten thing through metadata that survives erasure, which
    // for many subjects is most of what there was to hide.
    void aKeyDomainCarriesNoMeaning()
    {
        const KeyDomain first = Seal::generateDomain();
        const KeyDomain second = Seal::generateDomain(4);

        QVERIFY(first.isValid());
        QVERIFY(second.isValid());
        QVERIFY(first.keyDomainId != second.keyDomainId);
        QCOMPARE(second.keyEpoch, 4u);
    }
};

QTEST_MAIN(TestSealedPayload)
#include "tst_sealed_payload.moc"
