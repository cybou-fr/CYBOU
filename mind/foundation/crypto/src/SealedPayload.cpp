// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/crypto/SealedPayload.h"

#include <QCryptographicHash>

#include <sodium.h>

namespace cybou {

namespace {

// libsodium requires one initialisation before any other call, and it is safe to call repeatedly.
// Doing it lazily here rather than in main() means no organ can forget: a primitive that only works
// when someone remembered to set it up would fail in exactly the deployment nobody tested.
bool ensureInitialised()
{
    static const bool ready = sodium_init() >= 0;
    return ready;
}

const unsigned char *bytes(const QByteArray &value)
{
    return reinterpret_cast<const unsigned char *>(value.constData());
}

unsigned char *mutableBytes(QByteArray &value)
{
    return reinterpret_cast<unsigned char *>(value.data());
}

std::optional<SealedPayload> sealWith(const QByteArray &plaintext, const QByteArray &key)
{
    if (!ensureInitialised() || key.size() != kSealKeyBytes) {
        return std::nullopt;
    }

    SealedPayload sealed;
    sealed.nonce.resize(kSealNonceBytes);
    randombytes_buf(mutableBytes(sealed.nonce), kSealNonceBytes);

    sealed.ciphertext.resize(plaintext.size() + kSealTagBytes);
    unsigned long long written = 0;
    if (crypto_aead_xchacha20poly1305_ietf_encrypt(
            mutableBytes(sealed.ciphertext),
            &written,
            bytes(plaintext),
            static_cast<unsigned long long>(plaintext.size()),
            nullptr,
            0,
            nullptr,
            bytes(sealed.nonce),
            bytes(key))
        != 0) {
        return std::nullopt;
    }
    sealed.ciphertext.resize(static_cast<int>(written));
    return sealed;
}

std::optional<QByteArray> unsealWith(const SealedPayload &sealed, const QByteArray &key)
{
    if (!ensureInitialised() || key.size() != kSealKeyBytes || !sealed.isValid()) {
        return std::nullopt;
    }

    QByteArray plaintext;
    plaintext.resize(sealed.ciphertext.size() - kSealTagBytes);
    unsigned long long written = 0;
    if (crypto_aead_xchacha20poly1305_ietf_decrypt(
            mutableBytes(plaintext),
            &written,
            nullptr,
            bytes(sealed.ciphertext),
            static_cast<unsigned long long>(sealed.ciphertext.size()),
            nullptr,
            0,
            bytes(sealed.nonce),
            bytes(key))
        != 0) {
        // Authentication failed: wrong key, or the ciphertext was altered. Both answer the same way
        // on purpose - distinguishing them would tell an attacker which guess was closer.
        return std::nullopt;
    }
    plaintext.resize(static_cast<int>(written));
    return plaintext;
}

} // namespace

bool Seal::isAvailable()
{
    return ensureInitialised();
}

QByteArray Seal::generateKey()
{
    if (!ensureInitialised()) {
        return {};
    }
    QByteArray key;
    key.resize(kSealKeyBytes);
    randombytes_buf(mutableBytes(key), kSealKeyBytes);
    return key;
}

KeyDomain Seal::generateDomain(quint32 epoch)
{
    KeyDomain domain;
    domain.keyDomainId = QUuid::createUuid();
    domain.keyEpoch = epoch;
    return domain;
}

std::optional<SealedPayload> Seal::seal(const QByteArray &plaintext, const QByteArray &key)
{
    return sealWith(plaintext, key);
}

std::optional<QByteArray> Seal::unseal(const SealedPayload &sealed, const QByteArray &key)
{
    return unsealWith(sealed, key);
}

std::optional<SealedPayload> Seal::wrapKey(
    const QByteArray &dataKey, const QByteArray &keyEncryptionKey)
{
    return sealWith(dataKey, keyEncryptionKey);
}

std::optional<QByteArray> Seal::unwrapKey(
    const SealedPayload &wrapped, const QByteArray &keyEncryptionKey)
{
    return unsealWith(wrapped, keyEncryptionKey);
}

QByteArray Seal::sealedPayloadCommitment(const SealedPayload &sealed)
{
    QCryptographicHash hash(QCryptographicHash::Sha256);
    hash.addData(sealed.nonce);
    hash.addData(sealed.ciphertext);
    return hash.result();
}

} // namespace cybou
