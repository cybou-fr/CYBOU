// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include <QByteArray>
#include <QUuid>

#include <optional>

namespace cybou {

/// Sizes are fixed by the primitive rather than chosen here, and named so call sites do not carry
/// magic numbers that would silently disagree with the algorithm if it were ever changed.
inline constexpr int kSealKeyBytes = 32;   // XChaCha20-Poly1305 key
inline constexpr int kSealNonceBytes = 24; // XChaCha20 extended nonce
inline constexpr int kSealTagBytes = 16;   // Poly1305 authentication tag

/// A payload as the Journal will store it once ADR-0028's sensitive path is wired in.
///
/// The nonce is kept beside the ciphertext because it is not a secret and is required to decrypt.
/// The tag is inside `ciphertext`, where the AEAD's combined mode puts it: separating them would
/// invite a call site to verify one without the other.
struct SealedPayload {
    QByteArray nonce;
    QByteArray ciphertext;

    bool isValid() const
    {
        return nonce.size() == kSealNonceBytes && ciphertext.size() > kSealTagBytes;
    }
};

/// A per-contribution data key, and the domain it is wrapped under.
///
/// `keyDomainId` is a UUID and `keyEpoch` an integer **on purpose**. ADR-0028 forbids naming a key
/// domain after what it protects: a domain called `medical` or `location` would leak the category
/// of the erased thing through metadata that survives erasure, which for many subjects is most of
/// what there was to hide.
struct KeyDomain {
    QUuid keyDomainId;
    quint32 keyEpoch{0};

    bool isValid() const { return !keyDomainId.isNull(); }
};

/// Randomized authenticated encryption, and the commitment the Journal makes to its output.
///
/// Every operation here is a precondition for erasure rather than a feature in its own right: the
/// point is that destroying a key leaves a commitment nobody can test a guess against. Encrypting
/// the same plaintext twice must therefore produce different ciphertext, which is what the random
/// nonce buys and what `sealedPayloadCommitment` inherits.
class Seal
{
public:
    /// Whether the primitive is usable at all. False means no sensitive payload may be written -
    /// never that it may be written in the clear.
    static bool isAvailable();

    static QByteArray generateKey();
    static KeyDomain generateDomain(quint32 epoch = 1);

    static std::optional<SealedPayload> seal(const QByteArray &plaintext, const QByteArray &key);
    static std::optional<QByteArray> unseal(const SealedPayload &sealed, const QByteArray &key);

    /// Wrap a data key under a key-encrypting key, and unwrap it again.
    ///
    /// Erasure destroys a DEK and every wrapping of it. Wrapping is the same AEAD, so a wrapped key
    /// is as opaque as a sealed payload and a backup holding one is useless without the KEK.
    static std::optional<SealedPayload> wrapKey(
        const QByteArray &dataKey, const QByteArray &keyEncryptionKey);
    static std::optional<QByteArray> unwrapKey(
        const SealedPayload &wrapped, const QByteArray &keyEncryptionKey);

    /// What a v3 row commits to for a sensitive payload: `SHA256(nonce ‖ ciphertext ‖ tag)`.
    ///
    /// Deliberately not a digest of the plaintext. A digest of a low-entropy plaintext - a
    /// diagnosis, a boolean, one of a handful of known values - is a permanent oracle: anyone
    /// holding it can confirm a guess long after the payload is gone. This commits to bytes that
    /// depend on randomness the guesser does not have.
    static QByteArray sealedPayloadCommitment(const SealedPayload &sealed);
};

} // namespace cybou
