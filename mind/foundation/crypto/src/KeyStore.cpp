// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/crypto/KeyStore.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QSaveFile>

namespace cybou {

namespace {

// A wrapped key is a nonce and a ciphertext. Stored as one file with the nonce first, because two
// files could disagree after a crash and a half-written key is indistinguishable from a destroyed
// one - which is exactly the confusion erasure must not have.
QByteArray encodeWrapped(const SealedPayload &wrapped)
{
    return wrapped.nonce + wrapped.ciphertext;
}

std::optional<SealedPayload> decodeWrapped(const QByteArray &stored)
{
    if (stored.size() <= kSealNonceBytes + kSealTagBytes) {
        return std::nullopt;
    }
    SealedPayload wrapped;
    wrapped.nonce = stored.left(kSealNonceBytes);
    wrapped.ciphertext = stored.mid(kSealNonceBytes);
    return wrapped;
}

} // namespace

KeyStore::KeyStore(QString root)
    : m_root(std::move(root))
{
    if (!Seal::isAvailable()) {
        m_lastError = QStringLiteral("the sealing primitive is unavailable");
        return;
    }
    if (!QDir().mkpath(m_root)) {
        m_lastError = QStringLiteral("cannot create the key store at %1").arg(m_root);
        return;
    }

    // Owner-only, and enforced rather than assumed. A key store readable by anything else on the
    // machine would make erasure a gesture.
    QFile::setPermissions(m_root, QFile::ReadOwner | QFile::WriteOwner | QFile::ExeOwner);
    m_usable = true;
}

QString KeyStore::pathFor(const QUuid &contributionId) const
{
    return QDir(m_root).filePath(contributionId.toString(QUuid::WithoutBraces) + ".key");
}

std::optional<QByteArray> KeyStore::createKeyFor(
    const QUuid &contributionId, const QByteArray &keyEncryptionKey)
{
    if (!m_usable || contributionId.isNull()) {
        return std::nullopt;
    }

    const QByteArray dataKey = Seal::generateKey();
    const auto wrapped = Seal::wrapKey(dataKey, keyEncryptionKey);
    if (!wrapped.has_value()) {
        m_lastError = QStringLiteral("could not wrap a data key");
        return std::nullopt;
    }

    QSaveFile file(pathFor(contributionId));
    if (!file.open(QIODevice::WriteOnly)) {
        m_lastError = QStringLiteral("could not write a wrapped key");
        return std::nullopt;
    }
    file.setPermissions(QFile::ReadOwner | QFile::WriteOwner);
    if (file.write(encodeWrapped(*wrapped)) < 0 || !file.commit()) {
        m_lastError = QStringLiteral("could not commit a wrapped key");
        return std::nullopt;
    }
    return dataKey;
}

std::optional<QByteArray> KeyStore::keyFor(
    const QUuid &contributionId, const QByteArray &keyEncryptionKey) const
{
    if (!m_usable) {
        return std::nullopt;
    }

    QFile file(pathFor(contributionId));
    if (!file.exists() || !file.open(QIODevice::ReadOnly)) {
        return std::nullopt;
    }
    const auto wrapped = decodeWrapped(file.readAll());
    if (!wrapped.has_value()) {
        return std::nullopt;
    }
    return Seal::unwrapKey(*wrapped, keyEncryptionKey);
}

bool KeyStore::destroyKeyFor(const QUuid &contributionId)
{
    if (!m_usable) {
        return false;
    }

    const QString path = pathFor(contributionId);
    if (!QFile::exists(path)) {
        // Already gone, which is success. Erasure's middle step runs outside any transaction and is
        // re-run after a crash; if destroying an absent key were an error, recovery would report a
        // failure for having already succeeded.
        return true;
    }
    if (!QFile::remove(path)) {
        m_lastError = QStringLiteral("could not destroy the key for %1")
                          .arg(contributionId.toString(QUuid::WithoutBraces));
        return false;
    }
    return true;
}

bool KeyStore::hasKeyFor(const QUuid &contributionId) const
{
    return m_usable && QFile::exists(pathFor(contributionId));
}

} // namespace cybou
