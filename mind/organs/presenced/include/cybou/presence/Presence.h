// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// What the panel is allowed to see.
//
// This is the only class the surface talks to, and that is deliberate: it is the single place
// where "may this be shown?" is decided. Everything it exposes is read back out of the journal
// or computed by an organ - there is no field here that the panel could fill in itself, and no
// path by which something unmeasured reaches a pixel.
//
// It owns the organs rather than borrowing them, because a half-assembled Presence that still
// renders would be exactly the fake affordance ADR-0003 forbids. If it cannot open its journal
// it says so and shows nothing.

#pragma once

#include "cybou/self/SelfModel.h"
#include "cybou/workspace/Workspace.h"

#include <QObject>

#include <memory>

namespace cybou {

/// One line for the panel's activity list: what happened, when, and who said it.
struct Moment {
    QDateTime when;
    QString organ;
    /// Human-readable kind, already translated - the panel does not interpret enums.
    QString kind;
    /// The concern this belongs to, so the panel can group without knowing what a coalition is.
    QUuid thread;
};

class Presence : public QObject
{
    Q_OBJECT

    Q_PROPERTY(bool awake READ isAwake NOTIFY changed)
    Q_PROPERTY(QString narration READ narration NOTIFY changed)
    Q_PROPERTY(QStringList obligations READ obligations NOTIFY changed)
    Q_PROPERTY(QString attention READ attention NOTIFY changed)
    Q_PROPERTY(int contributions READ contributions NOTIFY changed)

public:
    /// dataDir is where the journal and identity live. Creating a Presence does not wake it.
    explicit Presence(const QString &dataDir, QObject *parent = nullptr);

    /// The constructor QML uses: the journal goes where the user's data goes. Kept separate
    /// from the one above so tests can never touch the real journal by accident.
    explicit Presence(QObject *parent = nullptr);
    ~Presence() override;

    /// Opens the journal, begins a session, and restores the moment. Returns false if any of
    /// that failed, in which case the object stays asleep and reports nothing.
    bool wake();

    bool isAwake() const { return m_awake; }

    /// Sentences built only from measured values. Empty while asleep.
    QString narration() const;

    /// Open intentions, oldest first, as text the panel can list directly.
    QStringList obligations() const;

    /// What the system is attending to, or an empty string when nothing is in the moment.
    /// Never invents a topic to fill the space.
    QString attention() const;

    int contributions() const;

    /// The recent activity list, newest first.
    QList<Moment> recent(int limit = 12) const;

    /// The same list in the shape QML can consume: maps with when/organ/kind/thread. A separate
    /// method rather than a metatype conversion, so the field names the panel binds to are
    /// visible here and cannot drift silently.
    Q_INVOKABLE QVariantList activity(int limit = 12) const;

    /// The panel may state an intention on the user's behalf - this is the one thing it can
    /// write. Returns a null uuid while asleep.
    Q_INVOKABLE QUuid promise(const QString &description);

    /// Records a self-assessment. Called when the panel is opened: being looked at is an event
    /// worth remembering, and it is what keeps the narration current.
    Q_INVOKABLE bool reflect();

    QString lastError() const { return m_lastError; }

Q_SIGNALS:
    /// Anything the panel displays may have changed.
    void changed();

private:
    QString m_dataDir;
    QString m_lastError;
    bool m_awake{false};

    std::unique_ptr<Journal> m_journal;
    std::unique_ptr<Identity> m_identity;
    std::unique_ptr<Intentions> m_intentions;
    std::unique_ptr<Predictor> m_predictor;
    std::unique_ptr<SelfModel> m_self;
    std::unique_ptr<Workspace> m_workspace;
};

} // namespace cybou

Q_DECLARE_METATYPE(cybou::Moment)
