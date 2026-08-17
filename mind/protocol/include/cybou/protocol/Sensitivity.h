// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include <QList>
#include <QString>
#include <QtGlobal>

namespace cybou {

/// Who may be shown a contribution, from ADR-0018's amendment.
///
/// A separate axis from `PrivacyClass` and from `RetentionClass`, and the reason is that
/// `PrivacyClass` cannot answer this. Its ordering is a replication scope -- *where* a contribution
/// may exist -- and `Local` is its default, so the overwhelming majority of ordinary contributions
/// sit at its most restricted value. A scope that restricted by default carries no signal about
/// whether the content is dangerous.
///
/// ADR-0033's A9 is the case that forces the split: secrets must never enter an opaque training
/// path, and until a payload can be *typed* as a credential, that gate is satisfiable by a refusal
/// for any reason at all.
enum class SensitivityClass : quint8 {
    Ordinary = 0, ///< no particular exposure concern
    Personal,     ///< about the person, and theirs to release
    Sensitive,    ///< harmful if disclosed, even to a trusted consumer
    Secret,       ///< disclosure is the harm
    Credential,   ///< confers access; never a deliberate training target
};

QString sensitivityToString(SensitivityClass sensitivity);

/// Whether a classification may ever be used as a supervised training target.
///
/// ADR-0033's A9 as a predicate rather than a hope. Deliberately a closed rule over the type
/// instead of a policy flag: a flag can be cleared by whoever is doing the training.
constexpr bool mayBeTrainingTarget(SensitivityClass sensitivity) noexcept
{
    return sensitivity != SensitivityClass::Secret
        && sensitivity != SensitivityClass::Credential;
}

/// The more sensitive of two classifications.
constexpr SensitivityClass mostSensitive(SensitivityClass a, SensitivityClass b) noexcept
{
    return static_cast<quint8>(a) > static_cast<quint8>(b) ? a : b;
}

/// What a derived contribution must declare, given what it was derived from.
///
/// Sensitivity propagates the way privacy does and for the same reason: a conclusion that restated
/// its evidence at a weaker classification would launder it. The caller compares this against what
/// an envelope declares and refuses a mismatch rather than silently correcting one, so the
/// declaration stays a contract instead of a suggestion.
SensitivityClass derivedSensitivity(
    SensitivityClass declared,
    const QList<SensitivityClass> &evidence);

/// How an unclassified contribution is read.
///
/// `Personal`, not `Ordinary`. Every row written before this axis existed carries no classification,
/// and reading absence as harmless would make the entire history look safe on the day the point of
/// the axis is to notice what is not. A default that errs downward is the one failure this cannot
/// afford.
constexpr SensitivityClass kUnclassifiedSensitivity = SensitivityClass::Personal;

} // namespace cybou
