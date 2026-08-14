// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/crypto/SealedPayload.h"

#include <QString>
#include <QUuid>

#include <optional>

namespace cybou {

/// Where per-contribution data keys live, and where they stop living.
///
/// ADR-0028 makes erasure a three-step protocol whose middle step - destroying a key - happens
/// outside any database transaction. That step must therefore be **idempotent**: after a crash it
/// is simply run again, and running it twice has to mean the same as running it once. A destroy
/// that failed when the key was already gone would turn recovery into a second failure mode.
///
/// Keys are stored wrapped, one file per contribution, so destroying one reaches exactly one
/// payload and leaves every other key untouched.
class KeyStore
{
public:
    explicit KeyStore(QString root);

    bool isUsable() const { return m_usable; }
    QString lastError() const { return m_lastError; }

    /// Create and store a wrapped data key for a contribution. Returns the unwrapped key, which the
    /// caller uses immediately and does not keep.
    std::optional<QByteArray> createKeyFor(
        const QUuid &contributionId, const QByteArray &keyEncryptionKey);

    /// The data key for a contribution, or nothing if it never existed or has been destroyed.
    ///
    /// Those two are deliberately one answer here. A caller that could tell them apart would learn
    /// whether a record was ever sensitive, which is a fact about content that erasure is meant to
    /// remove.
    std::optional<QByteArray> keyFor(
        const QUuid &contributionId, const QByteArray &keyEncryptionKey) const;

    /// Destroy a contribution's key. **Idempotent**: destroying an absent key succeeds.
    bool destroyKeyFor(const QUuid &contributionId);

    bool hasKeyFor(const QUuid &contributionId) const;

private:
    QString pathFor(const QUuid &contributionId) const;

    QString m_root;
    bool m_usable{false};
    mutable QString m_lastError;
};

} // namespace cybou
