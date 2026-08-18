// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/protocol/Sensitivity.h"

#include <QList>
#include <QString>
#include <QUuid>

namespace cybou {

/// What a person was doing with an utterance, from ADR-0031.
///
/// A closed, versioned vocabulary rather than free text. The whole point of the meaning boundary is
/// that the result stays inspectable after the parser that produced it is gone, and prose cannot be
/// inspected -- it can only be re-read by something that might interpret it differently.
///
/// Deliberately small. Enough for ordinary interaction, without pretending to encode every human
/// speech act on the first attempt.
enum class ActKind : quint8 {
    Ask = 0,   ///< a question about state
    Inform,    ///< a report from the person
    Request,   ///< asking for something to happen; still only interpreted intent
    Correct,   ///< superseding an earlier interpretation
    Confirm,
    Reject,
};

QString actKindToString(ActKind kind);

/// Whether an act, if honoured, would change something.
///
/// ADR-0031's C2 turns on this: a mutating act may not proceed on an unresolved referent, while
/// asking a question about an ambiguous thing is merely a question that needs clarifying.
constexpr bool isMutating(ActKind kind) noexcept
{
    return kind == ActKind::Request;
}

/// One candidate for what a reference pointed at, with the evidence that suggested it.
struct ReferenceCandidate {
    QString entityId;
    double score{0.0};
    QList<QUuid> evidence;
};

/// What a reference resolved to, or did not.
///
/// Candidates are kept even when resolution succeeds. A resolution that discarded the alternatives
/// would be indistinguishable from one that never had any, and the difference is exactly what a
/// person needs in order to correct it.
class ReferenceResolution
{
public:
    QString surfaceForm;
    QList<ReferenceCandidate> candidates;

    /// The chosen entity, or empty when the reference is unresolved.
    ///
    /// Ranking is not resolution. ADR-0031 rejects "highest score wins" because uncertainty about
    /// which server was meant is not permission to act on the wrong one.
    QString resolved() const { return m_resolved; }
    bool isResolved() const { return !m_resolved.isEmpty(); }

    /// Resolve only when one candidate is unambiguously ahead.
    ///
    /// `margin` is the gap a leader must have over the runner-up. A single candidate resolves; a
    /// tie or a near-tie does not, and the caller is expected to ask rather than guess.
    void resolveIfUnambiguous(double margin);

    /// Resolve to a specific entity because the person said so. A correction is evidence, not a
    /// guess, so it does not have to clear the margin.
    bool resolveByPerson(const QString &entityId);

private:
    QString m_resolved;
};

/// A typed act, ready to cross into Mind.
///
/// `CognitiveAct` is a protocol object, not a prompt string. It carries provenance to the
/// expression it came from, so an interpretation never loses the thing it interpreted.
struct CognitiveAct {
    ActKind kind{ActKind::Ask};

    /// What the act is about, once references are resolved.
    QString target;
    QString property;

    /// The expression this was derived from. An interpretation with no source is an assertion the
    /// parser made on its own authority.
    QUuid utterance;

    QList<ReferenceResolution> references;

    /// Sensitivity inherited from whatever the interpretation drew on.
    SensitivityClass sensitivity{kUnclassifiedSensitivity};

    /// Whether every reference this act depends on is resolved.
    bool isFullyResolved() const;

    /// Whether this act may proceed as it stands.
    ///
    /// A mutating act with an unresolved reference may not. This is C2, expressed where it cannot
    /// be forgotten by a caller who only checked `isResolved()` on one reference.
    bool mayProceed() const { return !isMutating(kind) || isFullyResolved(); }

    bool isValid() const { return !utterance.isNull(); }
};

} // namespace cybou
