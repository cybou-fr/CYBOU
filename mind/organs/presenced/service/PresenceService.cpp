// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "PresenceService.h"

#include "cybou/events/EnvelopeCodec.h"
#include "cybou/fabric/FabricCodec.h"
#include "cybou/protocol/CapabilityRegistry.h"

#include <QCborMap>
#include <QCborValue>
#include <QDBusConnection>
#include <QDBusMessage>
#include <QDateTime>
#include <QDeadlineTimer>
#include <QThread>
#include <QTimer>

#include <algorithm>
#include <functional>
#include <limits>
#include <utility>

namespace cybou {

namespace {

// Projected in the registry's order, so a capability added there appears here without anyone
// having to remember this list exists.
const QStringList &projectedCapabilities()
{
    static const QStringList ids = CapabilityRegistry::capabilityIds();
    return ids;
}

constexpr int kPresenceCommandTimeoutMs = 5000;

int presenceCommandTimeoutMs()
{
    bool ok = false;
    const int configured = qEnvironmentVariableIntValue(
        "CYBOU_PRESENCE_COMMAND_TIMEOUT_MS", &ok);
    return ok ? std::clamp(configured, 50, 60000) : kPresenceCommandTimeoutMs;
}

bool isAvailable(const CapabilitySnapshot &snapshot, const QString &capabilityId)
{
    if (!snapshot.isValid()) return false;
    return std::none_of(
        snapshot.deficits.cbegin(), snapshot.deficits.cend(),
        [&capabilityId](const CapabilityDeficit &deficit) {
            return deficit.capabilityId == capabilityId;
        });
}

QVariantMap capabilityProjection(const CapabilitySnapshot &snapshot)
{
    QVariantMap projection;
    projection[QStringLiteral("aggregateState")] = snapshot.isValid()
        ? capabilityStateToString(snapshot.aggregateState) : QStringLiteral("unknown");
    projection[QStringLiteral("observedAt")] = snapshot.observedAt;
    QVariantMap states;
    QVariantMap details;
    for (const QString &capabilityId : projectedCapabilities()) {
        states[capabilityId] = snapshot.isValid()
            ? QStringLiteral("available") : QStringLiteral("unknown");
        details[capabilityId] = QVariantMap{
            {QStringLiteral("state"), states.value(capabilityId)},
            {QStringLiteral("available"), snapshot.isValid()},
            {QStringLiteral("causes"), QStringList{}},
            {QStringLiteral("impacts"), QStringList{}},
            {QStringLiteral("dependencies"), QStringList{}},
            {QStringLiteral("recoveryPolicies"), QStringList{}},
            {QStringLiteral("recoveryProgress"), snapshot.isValid()
                 ? QStringLiteral("ready") : QStringLiteral("unknown")},
            {QStringLiteral("lastVerifiedAt"), snapshot.isValid()
                 ? snapshot.observedAt : QDateTime{}},
        };
    }
    QVariantList deficits;
    for (const CapabilityDeficit &source : snapshot.deficits) {
        QVariantMap deficit;
        deficit[QStringLiteral("capabilityId")] = source.capabilityId;
        deficit[QStringLiteral("dependencyId")] = source.dependencyId;
        deficit[QStringLiteral("state")] = capabilityStateToString(source.state);
        deficit[QStringLiteral("cause")] = deficitCauseToString(source.cause);
        deficit[QStringLiteral("impact")] = source.impact;
        deficit[QStringLiteral("lastVerifiedAt")] = source.lastVerifiedAt;
        deficit[QStringLiteral("recoveryPolicy")] = recoveryPolicyToString(source.recoveryPolicy);
        deficit[QStringLiteral("errorReference")] = source.errorReference;
        deficits.append(deficit);
        const QString current = states.value(source.capabilityId).toString();
        const auto rank = [](const QString &state) {
            if (state == QStringLiteral("available")) return 0;
            if (state == QStringLiteral("recovering")) return 1;
            if (state == QStringLiteral("limited")) return 2;
            if (state == QStringLiteral("stale")) return 3;
            if (state == QStringLiteral("unknown")) return 4;
            return 5;
        };
        const QString candidate = capabilityStateToString(source.state);
        if (rank(candidate) > rank(current)) states[source.capabilityId] = candidate;
        QVariantMap detail = details.value(source.capabilityId).toMap();
        QStringList causes = detail.value(QStringLiteral("causes")).toStringList();
        QStringList impacts = detail.value(QStringLiteral("impacts")).toStringList();
        QStringList dependencies = detail.value(QStringLiteral("dependencies")).toStringList();
        QStringList policies = detail.value(QStringLiteral("recoveryPolicies")).toStringList();
        const QString cause = deficitCauseToString(source.cause);
        const QString policy = recoveryPolicyToString(source.recoveryPolicy);
        if (!causes.contains(cause)) causes.append(cause);
        if (!source.impact.isEmpty() && !impacts.contains(source.impact)) impacts.append(source.impact);
        if (!dependencies.contains(source.dependencyId)) dependencies.append(source.dependencyId);
        if (!policies.contains(policy)) policies.append(policy);
        const QDateTime previous = detail.value(QStringLiteral("lastVerifiedAt")).toDateTime();
        if (!previous.isValid() || source.lastVerifiedAt > previous)
            detail[QStringLiteral("lastVerifiedAt")] = source.lastVerifiedAt;
        detail[QStringLiteral("causes")] = causes;
        detail[QStringLiteral("impacts")] = impacts;
        detail[QStringLiteral("dependencies")] = dependencies;
        detail[QStringLiteral("recoveryPolicies")] = policies;
        details[source.capabilityId] = detail;
    }
    for (auto it = details.begin(); it != details.end(); ++it) {
        QVariantMap detail = it.value().toMap();
        const QString state = states.value(it.key()).toString();
        detail[QStringLiteral("state")] = state;
        detail[QStringLiteral("available")] = state == QStringLiteral("available");
        detail[QStringLiteral("recoveryProgress")] = state == QStringLiteral("available")
            ? QStringLiteral("ready") : state == QStringLiteral("recovering")
            ? QStringLiteral("verifying") : state == QStringLiteral("unknown")
            ? QStringLiteral("unknown") : QStringLiteral("waiting");
        it.value() = detail;
    }
    projection[QStringLiteral("states")] = states;
    projection[QStringLiteral("details")] = details;
    projection[QStringLiteral("deficits")] = deficits;
    return projection;
}

QVariantMap commandProjection(const CapabilitySnapshot &snapshot, bool lifecycleReady)
{
    QVariantMap commands;
    for (const CommandDeclaration &declaration : CapabilityRegistry::commands()) {
        QStringList missing;
        for (const QString &capability : declaration.requiredCapabilities)
            if (!isAvailable(snapshot, capability)) missing.append(capability);
        commands[declaration.commandId] = QVariantMap{
            {QStringLiteral("available"), missing.isEmpty()},
            {QStringLiteral("requiredCapabilities"), declaration.requiredCapabilities},
            {QStringLiteral("missingCapabilities"), missing},
        };
    }
    commands[QStringLiteral("interruptLifecycle")] = QVariantMap{
        {QStringLiteral("available"), lifecycleReady},
        {QStringLiteral("requiredCapabilities"), QStringList{}},
        {QStringLiteral("missingCapabilities"), lifecycleReady
             ? QStringList{} : QStringList{QStringLiteral("lifecycle-control")}},
    };
    return commands;
}

QVariantMap lifecycleProjection(const QVariantMap &state)
{
    QVariantMap projection;
    const QString mode = state.value(QStringLiteral("mode")).toString();
    const QString status = state.value(QStringLiteral("status")).toString();
    const QStringList required = state.value(QStringLiteral("requiredCapabilities")).toStringList();
    const QStringList optional = state.value(QStringLiteral("optionalCapabilities")).toStringList();
    const QStringList completed = state.value(QStringLiteral("completedWork")).toStringList();
    const QStringList missing = state.value(QStringLiteral("missingWork")).toStringList();
    const int total = required.size() + optional.size();
    const int resolved = completed.size() + missing.size();

    QString progressClass = QStringLiteral("inactive");
    if (mode == QStringLiteral("recovering")) progressClass = QStringLiteral("recovering");
    else if (mode == QStringLiteral("degraded")) progressClass = QStringLiteral("degraded");
    else if (status == QStringLiteral("active")) progressClass = QStringLiteral("running");
    else if (status == QStringLiteral("completed")) progressClass = QStringLiteral("complete");
    else if (status == QStringLiteral("failed") || status == QStringLiteral("interrupted"))
        progressClass = QStringLiteral("failed");

    QVariantList deficits;
    const QVariantMap causes = state.value(QStringLiteral("missingCauses")).toMap();
    for (const QString &capability : missing) {
        QVariantMap deficit;
        deficit[QStringLiteral("capability")] = capability;
        deficit[QStringLiteral("cause")] = causes.value(capability).toString();
        deficits.append(deficit);
    }

    const QDateTime requestedAt = state.value(QStringLiteral("requestedAt")).toDateTime();
    const qint64 ageSeconds = requestedAt.isValid()
        ? qMax<qint64>(0, requestedAt.secsTo(QDateTime::currentDateTimeUtc()))
        : -1;
    QString freshnessClass = QStringLiteral("unknown");
    if (ageSeconds >= 0 && ageSeconds < 300) freshnessClass = QStringLiteral("current");
    else if (ageSeconds >= 0 && ageSeconds < 3600) freshnessClass = QStringLiteral("aging");
    else if (ageSeconds >= 0) freshnessClass = QStringLiteral("stale");

    projection[QStringLiteral("mode")] = mode;
    projection[QStringLiteral("status")] = status;
    projection[QStringLiteral("progressClass")] = progressClass;
    projection[QStringLiteral("progressPercent")] = total > 0 ? (resolved * 100) / total : 0;
    projection[QStringLiteral("resolvedWork")] = resolved;
    projection[QStringLiteral("totalWork")] = total;
    projection[QStringLiteral("deficits")] = deficits;
    projection[QStringLiteral("freshnessClass")] = freshnessClass;
    projection[QStringLiteral("ageSeconds")] = ageSeconds;
    projection[QStringLiteral("requestedAt")] = requestedAt;
    return projection;
}

} // namespace

// One in-flight compound Snapshot. Lives as long as its outstanding calls, and replies exactly
// once - either when the last owner lands or when the guard timer fires.
struct SnapshotRequest {
    QDBusConnection connection;
    QDBusMessage message;
    QDeadlineTimer deadline;

    CapabilitySnapshot health;
    QVariantMap self;
    QVariantMap lifecycle;
    QVariantMap scheduling;
    QVariantMap identity;
    QVariantMap moment;
    QVariantList intentions;
    QVariantList calibrations;
    QVariantList coalitions;
    QString attention;
    qulonglong contributions{0};

    int pending{0};
    bool replied{false};
    bool budgetExpired{false};

    SnapshotRequest(QDBusConnection bus, QDBusMessage request, int budgetMs)
        : connection(std::move(bus))
        , message(std::move(request))
        , deadline(budgetMs)
    {
    }

    int remaining() const
    {
        return static_cast<int>(std::max<qint64>(0, deadline.remainingTime()));
    }
};

namespace {

// Transport policy for the read-only projection.
//
// The default policy retries with backoff and latches a circuit open for five seconds after three
// infrastructure failures. Both are wrong for this path, in ways that only show up as a UI that
// lies:
//
// - Retries cannot help inside a deadline the guard is about to cut off anyway. They only spend
//   budget that later owners in the same gather still need.
// - A latched circuit outlives the request that opened it. One transient stall would blank that
//   owner's section of every subsequent projection for five seconds, and because the projection
//   reports typed defaults for anything it could not read, the UI would present that as "nothing
//   there" rather than "not asked". P6.7 bounds degradation per request; a circuit that persists
//   across requests quietly converts a transient fault into a sticky one.
//
// One attempt, no latching. Resilience on this path is the shared deadline and the typed empty
// projection, not retry. The mutation paths keep the default policy, where retry semantics and
// idempotency are actually reasoned about.
RpcRetryPolicy projectionPolicy()
{
    RpcRetryPolicy policy;
    policy.maximumAttempts = 1;
    policy.circuitFailureThreshold = std::numeric_limits<int>::max();
    policy.circuitOpenMs = 0;
    return policy;
}

} // namespace

// One in-flight compound mutation. Carries the delayed reply and the shared budget across the
// continuation chain; `lastError` is published through LastError() when the reply is sent.
struct CommandRequest {
    QDBusConnection connection;
    QDBusMessage message;
    QDeadlineTimer deadline;
    QString lastError;
    bool replied{false};

    CommandRequest(QDBusConnection bus, QDBusMessage request, int budgetMs)
        : connection(std::move(bus))
        , message(std::move(request))
        , deadline(budgetMs)
    {
    }

    int remaining() const
    {
        return static_cast<int>(std::max<qint64>(0, deadline.remainingTime()));
    }
};

PresenceService::PresenceService(QObject *parent)
    : QObject(parent)
    , m_healthRpc(kHealthEndpoint, projectionPolicy())
    , m_selfRpc(kSelfEndpoint, projectionPolicy())
    , m_lifecycleRpc(kLifecycleEndpoint, projectionPolicy())
    , m_intentionRpc(kIntentionEndpoint, projectionPolicy())
    , m_workspaceRpc(kWorkspaceEndpoint, projectionPolicy())
    , m_identityRpc(kIdentityEndpoint, projectionPolicy())
    , m_predictorRpc(kPredictorEndpoint, projectionPolicy())
    , m_eventRpc(kEventEndpoint, projectionPolicy())
{
    connect(
        &m_workspace,
        &WorkspaceClient::changed,
        this,
        [this]() {
            Q_EMIT Changed();
        });
    connect(&m_lifecycle, &LifecycleClient::changed, this, [this]() { Q_EMIT Changed(); });
    connect(&m_health, &HealthClient::changed, this, [this]() { Q_EMIT Changed(); });
}

// presenced loads no persistent state at startup, so there is no window in which it is running but
// not yet able to accept a call. Readiness is therefore genuinely constant, unlike Health() below;
// it is stated here as a property of this organ rather than left to look like an oversight.
bool PresenceService::Ready() const
{
    return true;
}

// Health1 derives the `presence-presentation` capability from this answer. presenced owns nothing
// durable, so its health is entirely a statement about whether it can still project its owners:
// answering a constant "healthy" would mean that capability could never enter a deficit no matter
// how much of the projection had gone missing.
//
// The answer describes the last projection actually attempted, not a fresh probe. Probing here
// would issue downstream calls from inside the health refresh that healthd runs against presenced,
// which is the accumulation P6.7 exists to prevent.
QString PresenceService::Health() const
{
    // Both failure outcomes report "degraded" rather than "unavailable": presenced answered the
    // call, so it is reachable and still serving the ungated part of the projection. Which of the
    // two occurred is a diagnostic question, and LastError() carries the originating client error.
    switch (m_lastProjection) {
    case ProjectionOutcome::NotAttempted:
    case ProjectionOutcome::Complete:
        return QStringLiteral("healthy");
    case ProjectionOutcome::CapabilitiesUnavailable:
    case ProjectionOutcome::BudgetExhausted:
        return QStringLiteral("degraded");
    }
    return QStringLiteral("degraded");
}

// Every command now records why it failed on its own request and publishes that when it replies, so
// this is a single value rather than a search.
//
// The chain this replaces walked each synchronous client's lastError() in turn. Those clients no
// longer carry the commands, so the fallbacks could only ever return state left over from an
// unrelated earlier call - a stale error attributed to whichever command asked next.
QString PresenceService::LastError() const
{
    return m_lastError;
}

QVariantMap PresenceService::healthMap(const CapabilitySnapshot &snapshot) const
{
    QVariantMap map;
    for (const ComponentHealthRecord &component : snapshot.components)
        map[component.componentId] = componentHealthToString(component.state);
    map[QStringLiteral("presenced")] = Health();
    map[QStringLiteral("healthd")] = snapshot.isValid()
        ? QStringLiteral("healthy") : QStringLiteral("unavailable");
    return map;
}

QVariantMap PresenceService::assembleSnapshot(const SnapshotRequest &request) const
{
    QVariantMap map;
    const QVariantMap capability = capabilityProjection(request.health);

    QStringList obligations;
    for (const QVariant &entry : request.intentions) {
        obligations.append(entry.toMap().value(QStringLiteral("description")).toString());
    }

    map[QStringLiteral("runtimeReachable")] = true;
    map[QStringLiteral("awake")] = true;
    map[QStringLiteral("aggregateCapabilityState")] =
        capability.value(QStringLiteral("aggregateState"));
    map[QStringLiteral("capabilityStates")] = capability.value(QStringLiteral("states"));
    map[QStringLiteral("capabilityDetails")] = capability.value(QStringLiteral("details"));
    map[QStringLiteral("capabilityDeficits")] = capability.value(QStringLiteral("deficits"));
    map[QStringLiteral("capabilityObservedAt")] = capability.value(QStringLiteral("observedAt"));
    map[QStringLiteral("commandAvailability")] =
        commandProjection(request.health, !request.lifecycle.isEmpty());
    map[QStringLiteral("lifecycleState")] = request.lifecycle;
    map[QStringLiteral("lifecycleMode")] = request.lifecycle.value(QStringLiteral("mode"));
    map[QStringLiteral("lifecycleStatus")] = request.lifecycle.value(QStringLiteral("status"));
    map[QStringLiteral("lifecycleProjection")] = lifecycleProjection(request.lifecycle);
    map[QStringLiteral("lifecycleScheduling")] = request.scheduling;
    map[QStringLiteral("narration")] = request.self.value(QStringLiteral("narration")).toString();
    map[QStringLiteral("obligations")] = obligations;
    map[QStringLiteral("attention")] = request.attention;
    map[QStringLiteral("contributions")] = request.contributions;
    map[QStringLiteral("stats")] = request.self;
    map[QStringLiteral("identityState")] = request.identity;
    map[QStringLiteral("calibrations")] = request.calibrations;
    map[QStringLiteral("coalitions")] = request.coalitions;
    map[QStringLiteral("moment")] = request.moment;

    // Settle the outcome before healthMap(), which embeds this organ's own Health() answer in the
    // projection: deciding afterwards would publish a snapshot claiming presenced was healthy
    // during the very collection that ran out of budget.
    //
    // Expiry is only reported when the projection was otherwise complete. A missing Health1
    // snapshot is the more specific fact and must not be overwritten by the budget outcome, since
    // skipping gated reads is exactly what makes the remaining budget look healthy.
    m_lastProjection = request.health.isValid()
        ? ProjectionOutcome::Complete
        : ProjectionOutcome::CapabilitiesUnavailable;
    if (m_lastProjection == ProjectionOutcome::Complete && request.budgetExpired) {
        m_lastProjection = ProjectionOutcome::BudgetExhausted;
    }

    map[QStringLiteral("organHealth")] = healthMap(request.health);

    return map;
}

void PresenceService::finishSnapshot(const std::shared_ptr<SnapshotRequest> &request) const
{
    if (request->replied) {
        return;
    }
    request->replied = true;

    const QVariantMap map = assembleSnapshot(*request);
    request->connection.send(
        request->message.createReply(QVariant(FabricCodec::encodeMap(map))));
}

// Every read below is independent: none of them feeds another, they only share the gate that
// Health1 already answered. Issuing them together turns the cost of a projection from the sum of
// the owner latencies into the largest one, and because nothing blocks, presenced keeps answering
// other callers while they are in flight.
void PresenceService::gatherSnapshot(const std::shared_ptr<SnapshotRequest> &request) const
{
    const CapabilitySnapshot &health = request->health;
    const int budget = request->remaining();

    // The guard is what preserves the P6.7 contract under retries. Each call is given the remaining
    // budget as its transport timeout, but the retry policy can schedule another attempt after a
    // failure, which would otherwise push the total past the deadline. When the guard fires, the
    // projection is assembled from whatever landed - structurally complete, with typed defaults for
    // the rest - and later replies find `replied` already set.
    QTimer *guard = new QTimer(const_cast<PresenceService *>(this));
    guard->setSingleShot(true);
    guard->setInterval(budget);
    connect(guard, &QTimer::timeout, this, [this, request, guard]() {
        guard->deleteLater();
        if (!request->replied) {
            request->budgetExpired = true;
            finishSnapshot(request);
        }
    });
    guard->start();

    // Hold one reference for the issuing loop itself. AsyncRpcClient can complete synchronously -
    // an open circuit rejects without ever reaching the bus - so without this the first such
    // completion would drive `pending` to zero while later reads are still being issued, and the
    // request would reply with a projection missing every section after that point.
    request->pending = 1;

    const auto arrived = [this, request, guard]() {
        if (--request->pending > 0 || request->replied) {
            return;
        }
        guard->stop();
        guard->deleteLater();
        request->budgetExpired = request->remaining() == 0;
        finishSnapshot(request);
    };

    // Decode only on success. A failed call leaves the field at its typed default, which is exactly
    // what a section that could not be measured should project.
    const auto readBytes = [](const RpcResult &result) {
        return result.succeeded() && !result.reply.arguments().isEmpty()
            ? result.reply.arguments().first().toByteArray()
            : QByteArray();
    };

    const auto issue = [&](AsyncRpcClient &client,
                           const QString &method,
                           bool gated,
                           std::function<void(const RpcResult &)> handle) {
        if (!gated) {
            return;
        }
        ++request->pending;
        client.call(
            method,
            {},
            RpcOperationSemantics::ReadOnly,
            [handle = std::move(handle), arrived](const RpcResult &result) {
                handle(result);
                arrived();
            },
            request->remaining());
    };

    issue(
        m_selfRpc,
        QStringLiteral("Measure"),
        isAvailable(health, QStringLiteral("self-assessment")),
        [request, readBytes](const RpcResult &result) {
            QString error;
            request->self = FabricCodec::decodeMap(readBytes(result), &error);
        });

    // Lifecycle state is deliberately ungated: lifecycle mode is orthogonal to capability health,
    // and the UI distinguishes an unavailable coordinator from an idle one.
    issue(
        m_lifecycleRpc,
        QStringLiteral("State"),
        true,
        [request, readBytes](const RpcResult &result) {
            QString error;
            request->lifecycle = FabricCodec::decodeMap(readBytes(result), &error);
        });

    issue(
        m_lifecycleRpc,
        QStringLiteral("EvaluateScheduling"),
        true,
        [request, readBytes](const RpcResult &result) {
            QString error;
            request->scheduling = FabricCodec::decodeMap(readBytes(result), &error);
        });

    issue(
        m_intentionRpc,
        QStringLiteral("Open"),
        isAvailable(health, QStringLiteral("commitment-access")),
        [request, readBytes](const RpcResult &result) {
            QString error;
            request->intentions = FabricCodec::decodeList(readBytes(result), &error);
        });

    issue(
        m_workspaceRpc,
        QStringLiteral("Attention"),
        isAvailable(health, QStringLiteral("attention-workspace")),
        [request](const RpcResult &result) {
            if (result.succeeded() && !result.reply.arguments().isEmpty()) {
                request->attention = result.reply.arguments().first().toString();
            }
        });

    issue(
        m_workspaceRpc,
        QStringLiteral("Coalitions"),
        isAvailable(health, QStringLiteral("attention-workspace")),
        [request, readBytes](const RpcResult &result) {
            QString error;
            request->coalitions = FabricCodec::decodeList(readBytes(result), &error);
        });

    issue(
        m_workspaceRpc,
        QStringLiteral("Moment"),
        isAvailable(health, QStringLiteral("attention-workspace")),
        [request, readBytes](const RpcResult &result) {
            QString error;
            request->moment = FabricCodec::decodeMap(readBytes(result), &error);
        });

    issue(
        m_eventRpc,
        QStringLiteral("Count"),
        isAvailable(health, QStringLiteral("accepted-biography")),
        [request](const RpcResult &result) {
            if (result.succeeded() && !result.reply.arguments().isEmpty()) {
                request->contributions = result.reply.arguments().first().toULongLong();
            }
        });

    issue(
        m_identityRpc,
        QStringLiteral("State"),
        isAvailable(health, QStringLiteral("identity-continuity")),
        [request, readBytes](const RpcResult &result) {
            QString error;
            request->identity = FabricCodec::decodeMap(readBytes(result), &error);
        });

    issue(
        m_predictorRpc,
        QStringLiteral("Calibrations"),
        isAvailable(health, QStringLiteral("prediction")),
        [request, readBytes](const RpcResult &result) {
            QString error;
            request->calibrations = FabricCodec::decodeList(readBytes(result), &error);
        });

    // Release the issuing reference. If Health1 was unavailable every read above was gated off, or
    // all of them completed synchronously, this is what replies.
    arrived();
}

// Snapshot answers through a delayed reply. The method returns immediately and presenced continues
// serving its event loop while the owners answer, instead of holding the thread inside a chain of
// blocking calls where a single slow owner stalls every other caller.
QByteArray PresenceService::Snapshot() const
{
    setDelayedReply(true);

    auto request = std::make_shared<SnapshotRequest>(
        connection(), message(), presenceCommandTimeoutMs());

    // Health1 is the one genuinely sequential step: its answer decides which of the reads below are
    // legitimate to make at all. Everything after it goes out together.
    m_healthRpc.call(
        QStringLiteral("Snapshot"),
        {},
        RpcOperationSemantics::ReadOnly,
        [this, request](const RpcResult &result) {
            if (result.succeeded() && !result.reply.arguments().isEmpty()) {
                QString error;
                request->health = decodeCapabilitySnapshot(
                    result.reply.arguments().first().toByteArray(), &error);
            }
            gatherSnapshot(request);
        },
        request->remaining());

    return {};
}

// Activity and DetailedObligations are two-step gated reads rather than compound gathers, but they
// are still Presence surface: leaving them on the blocking client would mean a slow Event1 or
// Intention1 could hold presenced exactly as the projection used to. Same shape as the mutations -
// gate first, then read - only without a durable contribution.
void PresenceService::gatedRead(
    const std::shared_ptr<CommandRequest> &request,
    const QStringList &requiredCapabilities,
    AsyncRpcClient &client,
    const QString &method,
    const QVariantList &arguments,
    std::function<QVariant(const RpcResult &)> project,
    const QVariant &failureValue) const
{
    m_healthRpc.call(
        QStringLiteral("Snapshot"),
        {},
        RpcOperationSemantics::ReadOnly,
        [this, request, requiredCapabilities, &client, method, arguments,
         project = std::move(project), failureValue](const RpcResult &healthResult) {
            CapabilitySnapshot capabilities;
            if (healthResult.succeeded() && !healthResult.reply.arguments().isEmpty()) {
                QString error;
                capabilities = decodeCapabilitySnapshot(
                    healthResult.reply.arguments().first().toByteArray(), &error);
            }
            const bool gated = std::any_of(
                requiredCapabilities.cbegin(),
                requiredCapabilities.cend(),
                [&capabilities](const QString &id) { return !isAvailable(capabilities, id); });
            if (gated || request->remaining() == 0) {
                replyOnce(request, failureValue);
                return;
            }
            client.call(
                method,
                arguments,
                RpcOperationSemantics::ReadOnly,
                [this, request, project](const RpcResult &result) {
                    replyOnce(request, project(result));
                },
                request->remaining());
        },
        request->remaining());
}

QByteArray PresenceService::Activity(int limit) const
{
    auto request = beginRequest();
    const QVariant empty = QVariant(FabricCodec::encodeList(QVariantList{}));

    if (limit <= 0) {
        replyOnce(request, empty);
        return {};
    }

    gatedRead(
        request,
        CapabilityRegistry::requiredCapabilitiesFor(QStringLiteral("activity")),
        m_eventRpc,
        QStringLiteral("Recent"),
        {limit},
        [](const RpcResult &result) {
            QVariantList activity;
            if (result.succeeded() && !result.reply.arguments().isEmpty()) {
                QString error;
                for (const CognitiveEnvelope &envelope : EnvelopeCodec::decodeList(
                         result.reply.arguments().first().toByteArray(), &error)) {
                    QVariantMap entry;
                    entry[QStringLiteral("when")] = envelope.wallTime.toLocalTime();
                    entry[QStringLiteral("organ")] = envelope.originOrgan;
                    entry[QStringLiteral("kind")] = kindToString(envelope.kind);
                    entry[QStringLiteral("thread")] =
                        envelope.correlationId.toString(QUuid::WithoutBraces);
                    activity.append(entry);
                }
            }
            return QVariant(FabricCodec::encodeList(activity));
        },
        empty);

    return {};
}

QByteArray PresenceService::DetailedObligations() const
{
    auto request = beginRequest();
    const QVariant empty = QVariant(FabricCodec::encodeList(QVariantList{}));

    gatedRead(
        request,
        CapabilityRegistry::requiredCapabilitiesFor(QStringLiteral("fulfill")),
        m_intentionRpc,
        QStringLiteral("Open"),
        {},
        [](const RpcResult &result) {
            QString error;
            const QVariantList open = result.succeeded()
                    && !result.reply.arguments().isEmpty()
                ? FabricCodec::decodeList(result.reply.arguments().first().toByteArray(), &error)
                : QVariantList{};
            return QVariant(FabricCodec::encodeList(open));
        },
        empty);

    return {};
}

namespace {

CognitiveEnvelope userObservation(const QString &event, const QVariantMap &details)
{
    CognitiveEnvelope observation;
    observation.messageId = QUuid::createUuid();
    observation.correlationId = observation.messageId;
    observation.originOrgan = QStringLiteral("presenced");
    observation.kind = ContributionKind::Observation;
    observation.wallTime = QDateTime::currentDateTimeUtc();
    observation.confidence = 1.0;
    observation.privacy = PrivacyClass::Node;

    QCborMap payload;
    payload.insert(QStringLiteral("event"), event);
    for (auto it = details.cbegin(); it != details.cend(); ++it) {
        payload.insert(it.key(), QCborValue::fromVariant(it.value()));
    }
    observation.payloadCbor = payload.toCborValue().toCbor();
    return observation;
}

// eventd answers Submit with a CBOR {sequence, error} map rather than a bare value, so acceptance
// has to be read out of the payload instead of inferred from the call succeeding.
bool submitAccepted(const RpcResult &result, QString *error)
{
    if (!result.succeeded() || result.reply.arguments().isEmpty()) {
        *error = rpcOutcomeToString(result.outcome) + QStringLiteral(": ") + result.errorMessage;
        return false;
    }
    const QCborValue value =
        QCborValue::fromCbor(result.reply.arguments().first().toByteArray());
    if (!value.isMap()) {
        *error = QStringLiteral("eventd returned an invalid Submit reply");
        return false;
    }
    bool ok = false;
    const quint64 sequence =
        value.toMap().value(QStringLiteral("sequence")).toString().toULongLong(&ok);
    if (!ok || sequence == 0) {
        const QString reported = value.toMap().value(QStringLiteral("error")).toString();
        *error = reported.isEmpty() ? QStringLiteral("eventd rejected the contribution") : reported;
        return false;
    }
    return true;
}

} // namespace

std::shared_ptr<CommandRequest> PresenceService::beginRequest() const
{
    m_lastError.clear();
    setDelayedReply(true);
    return std::make_shared<CommandRequest>(
        connection(), message(), presenceCommandTimeoutMs());
}

void PresenceService::replyOnce(
    const std::shared_ptr<CommandRequest> &request,
    const QVariant &value) const
{
    if (request->replied) {
        return;
    }
    request->replied = true;
    if (!request->lastError.isEmpty()) {
        m_lastError = request->lastError;
    }
    request->connection.send(request->message.createReply(value));
}

void PresenceService::beginUserCommand(
    const std::shared_ptr<CommandRequest> &request,
    const QStringList &requiredCapabilities,
    const QString &activityCause,
    const QString &observationEvent,
    const QVariantMap &observationDetails,
    const QVariant &failureValue,
    std::function<void(const QUuid &causeId)> committed) const
{
    const auto fail = [this, request, failureValue](const QString &reason) {
        request->lastError = reason;
        replyOnce(request, failureValue);
    };

    if (request->remaining() == 0) {
        fail(QStringLiteral("the command budget expired before it started"));
        return;
    }

    m_healthRpc.call(
        QStringLiteral("Snapshot"),
        {},
        RpcOperationSemantics::ReadOnly,
        [this, request, requiredCapabilities, activityCause, observationEvent, observationDetails,
         failureValue, committed = std::move(committed), fail](const RpcResult &healthResult) {
            CapabilitySnapshot capabilities;
            if (healthResult.succeeded() && !healthResult.reply.arguments().isEmpty()) {
                QString error;
                capabilities = decodeCapabilitySnapshot(
                    healthResult.reply.arguments().first().toByteArray(), &error);
            }
            for (const QString &capabilityId : requiredCapabilities) {
                if (!isAvailable(capabilities, capabilityId)) {
                    fail(QStringLiteral("%1 is unavailable").arg(capabilityId));
                    return;
                }
            }

            // Event1 is the durability boundary for everything this command produces. Probe it
            // before notifying auxiliary owners, so an unavailable Journal costs one bounded step
            // rather than the budget of every step that would have followed it.
            if (request->remaining() == 0) {
                fail(QStringLiteral("the command budget expired before the Event1 preflight"));
                return;
            }
            m_eventRpc.call(
                QStringLiteral("Ready"),
                {},
                RpcOperationSemantics::ReadOnly,
                [this, request, activityCause, observationEvent, observationDetails,
                 committed = std::move(committed), fail](const RpcResult &readyResult) {
                    const bool journalOpen = readyResult.succeeded()
                        && !readyResult.reply.arguments().isEmpty()
                        && readyResult.reply.arguments().first().toBool();
                    if (!journalOpen) {
                        fail(QStringLiteral("the cognitive journal is unavailable"));
                        return;
                    }

                    // Lifecycle activity is advisory: the user acted, and an unreachable
                    // coordinator must not veto a command the capability graph already allowed.
                    // Its reply is awaited only to keep the ordering explicit.
                    m_lifecycleRpc.call(
                        QStringLiteral("NotifyUserActivity"),
                        {activityCause},
                        RpcOperationSemantics::IdempotentMutation,
                        [this, request, observationEvent, observationDetails,
                         committed = std::move(committed), fail](const RpcResult &) {
                            if (request->remaining() == 0) {
                                fail(QStringLiteral(
                                    "the command budget expired before the durable observation"));
                                return;
                            }

                            const CognitiveEnvelope observation =
                                userObservation(observationEvent, observationDetails);
                            const QUuid causeId = observation.messageId;

                            // Submitting the same envelope twice would create a second
                            // contribution, so this step is non-idempotent and must not be retried
                            // on an unknown outcome.
                            m_eventRpc.call(
                                QStringLiteral("Submit"),
                                QVariantList{
                                    QVariant(EnvelopeCodec::encode(observation))},
                                RpcOperationSemantics::NonIdempotentMutation,
                                [causeId, committed = std::move(committed), fail](
                                    const RpcResult &submitResult) {
                                    QString error;
                                    if (!submitAccepted(submitResult, &error)) {
                                        fail(error);
                                        return;
                                    }
                                    committed(causeId);
                                },
                                request->remaining());
                        },
                        request->remaining());
                },
                request->remaining());
        },
        request->remaining());
}

QString PresenceService::Promise(const QString &description)
{
    auto request = beginRequest();
    const QString normalized = description.trimmed();
    if (normalized.isEmpty()) {
        replyOnce(request, QVariant(QString()));
        return {};
    }

    beginUserCommand(
        request,
        CapabilityRegistry::requiredCapabilitiesFor(QStringLiteral("promise")),
        QStringLiteral("presence.promise"),
        QStringLiteral("user-requested-intention"),
        {{QStringLiteral("description"), normalized}},
        QVariant(QString()),
        [this, request, normalized](const QUuid &causeId) {
            m_intentionRpc.call(
                QStringLiteral("Form"),
                {normalized,
                 QStringLiteral("asked by the user"),
                 causeId.toString(QUuid::WithoutBraces)},
                RpcOperationSemantics::NonIdempotentMutation,
                [this, request](const RpcResult &result) {
                    const QString intentionId = result.succeeded()
                            && !result.reply.arguments().isEmpty()
                        ? result.reply.arguments().first().toString()
                        : QString();
                    if (intentionId.isEmpty()) {
                        request->lastError = rpcOutcomeToString(result.outcome)
                            + QStringLiteral(": ") + result.errorMessage;
                    }
                    replyOnce(request, QVariant(intentionId));
                },
                request->remaining());
        });

    return {};
}

bool PresenceService::Reflect()
{
    auto request = beginRequest();

    beginUserCommand(
        request,
        CapabilityRegistry::requiredCapabilitiesFor(QStringLiteral("reflect")),
        QStringLiteral("presence.reflect"),
        QStringLiteral("self-inspection-requested"),
        {},
        QVariant(false),
        [this, request](const QUuid &causeId) {
            m_selfRpc.call(
                QStringLiteral("Assess"),
                {causeId.toString(QUuid::WithoutBraces)},
                RpcOperationSemantics::NonIdempotentMutation,
                [this, request](const RpcResult &result) {
                    QString error;
                    const QVariantMap report = result.succeeded()
                            && !result.reply.arguments().isEmpty()
                        ? FabricCodec::decodeMap(
                              result.reply.arguments().first().toByteArray(), &error)
                        : QVariantMap{};
                    if (report.isEmpty()) {
                        request->lastError = rpcOutcomeToString(result.outcome)
                            + QStringLiteral(": ") + result.errorMessage;
                    }
                    replyOnce(request, QVariant(!report.isEmpty()));
                },
                request->remaining());
        });

    return false;
}

// Fulfil and abandon differ only in the resolution they record, so they share one continuation.
// The open list is read after the durable Observation is accepted, because the index the user acted
// on refers to what Presence was showing, and re-reading it is what turns that index into an id.
void PresenceService::closeIntentionAtIndex(
    const std::shared_ptr<CommandRequest> &request,
    int index,
    int resolution,
    const QString &commandId,
    const QString &activityCause,
    const QString &observationEvent) const
{
    beginUserCommand(
        request,
        // Fulfil and abandon declare the same requirement, so the caller passes which one it is.
        CapabilityRegistry::requiredCapabilitiesFor(commandId),
        activityCause,
        observationEvent,
        {{QStringLiteral("index"), index}},
        QVariant(false),
        [this, request, index, resolution](const QUuid &) {
            m_intentionRpc.call(
                QStringLiteral("Open"),
                {},
                RpcOperationSemantics::ReadOnly,
                [this, request, index, resolution](const RpcResult &openResult) {
                    QString error;
                    const QVariantList open = openResult.succeeded()
                            && !openResult.reply.arguments().isEmpty()
                        ? FabricCodec::decodeList(
                              openResult.reply.arguments().first().toByteArray(), &error)
                        : QVariantList{};
                    if (index < 0 || index >= open.size()) {
                        request->lastError =
                            QStringLiteral("no open commitment at the requested position");
                        replyOnce(request, QVariant(false));
                        return;
                    }

                    const QString id = open.at(index)
                                           .toMap()
                                           .value(QStringLiteral("correlationId"))
                                           .toString();
                    m_intentionRpc.call(
                        QStringLiteral("Close"),
                        {id, resolution, QString()},
                        RpcOperationSemantics::IdempotentMutation,
                        [this, request](const RpcResult &closeResult) {
                            const bool ok = closeResult.succeeded()
                                && !closeResult.reply.arguments().isEmpty()
                                && closeResult.reply.arguments().first().toBool();
                            if (!ok) {
                                request->lastError = rpcOutcomeToString(closeResult.outcome)
                                    + QStringLiteral(": ") + closeResult.errorMessage;
                            }
                            replyOnce(request, QVariant(ok));
                        },
                        request->remaining());
                },
                request->remaining());
        });
}

bool PresenceService::FulfillIndex(int index)
{
    closeIntentionAtIndex(
        beginRequest(),
        index,
        0,
        QStringLiteral("fulfill"),
        QStringLiteral("presence.fulfill"),
        QStringLiteral("user-fulfilled-intention"));
    return false;
}

bool PresenceService::AbandonIndex(int index)
{
    closeIntentionAtIndex(
        beginRequest(),
        index,
        1,
        QStringLiteral("abandon"),
        QStringLiteral("presence.abandon"),
        QStringLiteral("user-abandoned-intention"));
    return false;
}

bool PresenceService::Observe(const QString &subject, double value)
{
    auto request = beginRequest();

    beginUserCommand(
        request,
        CapabilityRegistry::requiredCapabilitiesFor(QStringLiteral("observe")),
        QStringLiteral("presence.observe"),
        QStringLiteral("user-recorded-observation"),
        {{QStringLiteral("subject"), subject}, {QStringLiteral("value"), value}},
        QVariant(false),
        [this, request, subject, value](const QUuid &) {
            m_predictorRpc.call(
                QStringLiteral("Observe"),
                {subject, value},
                RpcOperationSemantics::NonIdempotentMutation,
                [this, request](const RpcResult &result) {
                    const bool ok = result.succeeded() && !result.reply.arguments().isEmpty()
                        && result.reply.arguments().first().toBool();
                    if (!ok) {
                        request->lastError = rpcOutcomeToString(result.outcome)
                            + QStringLiteral(": ") + result.errorMessage;
                    }
                    replyOnce(request, QVariant(ok));
                },
                request->remaining());
        });

    return false;
}

// Predict reads; it does not contribute to biography. It therefore skips the Event1 preflight and
// the durable Observation, and keeps only the capability gate and the activity notification.
QByteArray PresenceService::Predict(const QString &subject)
{
    auto request = beginRequest();
    const QVariant empty = QVariant(FabricCodec::encodeMap(QVariantMap{}));

    m_healthRpc.call(
        QStringLiteral("Snapshot"),
        {},
        RpcOperationSemantics::ReadOnly,
        [this, request, subject, empty](const RpcResult &healthResult) {
            CapabilitySnapshot capabilities;
            if (healthResult.succeeded() && !healthResult.reply.arguments().isEmpty()) {
                QString error;
                capabilities = decodeCapabilitySnapshot(
                    healthResult.reply.arguments().first().toByteArray(), &error);
            }
            const QStringList required =
                CapabilityRegistry::requiredCapabilitiesFor(QStringLiteral("predict"));
            if (std::any_of(
                    required.cbegin(), required.cend(), [&capabilities](const QString &id) {
                        return !isAvailable(capabilities, id);
                    })) {
                request->lastError = QStringLiteral("prediction is unavailable");
                replyOnce(request, empty);
                return;
            }

            m_lifecycleRpc.call(
                QStringLiteral("NotifyUserActivity"),
                {QStringLiteral("presence.predict")},
                RpcOperationSemantics::IdempotentMutation,
                [this, request, subject, empty](const RpcResult &) {
                    m_predictorRpc.call(
                        QStringLiteral("Predict"),
                        {subject, QString()},
                        RpcOperationSemantics::ReadOnly,
                        [this, request, empty](const RpcResult &result) {
                            QString error;
                            const QVariantMap prediction = result.succeeded()
                                    && !result.reply.arguments().isEmpty()
                                ? FabricCodec::decodeMap(
                                      result.reply.arguments().first().toByteArray(), &error)
                                : QVariantMap{};
                            if (prediction.isEmpty()) {
                                request->lastError = rpcOutcomeToString(result.outcome)
                                    + QStringLiteral(": ") + result.errorMessage;
                            }
                            replyOnce(
                                request, QVariant(FabricCodec::encodeMap(prediction)));
                        },
                        request->remaining());
                },
                request->remaining());
        },
        request->remaining());

    return {};
}

// InterruptLifecycle is not a user contribution: it terminates an existing run rather than adding
// to biography, so it has no Event1 preflight and no Observation. FinishRun stays non-idempotent -
// the shell relies on a timeout surfacing as unknown-outcome rather than as a failure, because a
// reply that never arrived does not mean the run was not finished.
bool PresenceService::InterruptLifecycle(const QString &cause)
{
    auto request = beginRequest();

    bool delayOk = false;
    const int delayMs =
        qEnvironmentVariableIntValue("CYBOU_PRESENCE_INTERRUPT_DELAY_MS", &delayOk);
    if (delayOk && delayMs > 0) {
        QThread::msleep(static_cast<unsigned long>(delayMs));
    }

    const QString reason = cause.trimmed().isEmpty()
        ? QStringLiteral("interrupted by user")
        : cause.trimmed();

    if (request->remaining() == 0) {
        request->lastError = QStringLiteral("lifecycle interruption deadline exceeded");
        replyOnce(request, QVariant(false));
        return false;
    }

    m_lifecycleRpc.call(
        QStringLiteral("State"),
        {},
        RpcOperationSemantics::ReadOnly,
        [this, request, reason](const RpcResult &stateResult) {
            QString error;
            const QVariantMap state = stateResult.succeeded()
                    && !stateResult.reply.arguments().isEmpty()
                ? FabricCodec::decodeMap(
                      stateResult.reply.arguments().first().toByteArray(), &error)
                : QVariantMap{};
            if (!stateResult.succeeded()) {
                request->lastError = rpcOutcomeToString(stateResult.outcome)
                    + QStringLiteral(": ") + stateResult.errorMessage;
                replyOnce(request, QVariant(false));
                return;
            }
            if (state.value(QStringLiteral("status")).toString()
                != QStringLiteral("active")) {
                request->lastError = QStringLiteral("no active lifecycle run to interrupt");
                replyOnce(request, QVariant(false));
                return;
            }
            if (request->remaining() == 0) {
                request->lastError = QStringLiteral("lifecycle interruption deadline exceeded");
                replyOnce(request, QVariant(false));
                return;
            }

            m_lifecycleRpc.call(
                QStringLiteral("FinishRun"),
                {QStringLiteral("interrupted"), reason},
                RpcOperationSemantics::NonIdempotentMutation,
                [this, request](const RpcResult &finishResult) {
                    const bool ok = finishResult.succeeded()
                        && !finishResult.reply.arguments().isEmpty()
                        && finishResult.reply.arguments().first().toBool();
                    if (!ok) {
                        request->lastError = rpcOutcomeToString(finishResult.outcome)
                            + QStringLiteral(": ") + finishResult.errorMessage;
                    }
                    replyOnce(request, QVariant(ok));
                },
                request->remaining());
        },
        request->remaining());

    return false;
}

} // namespace cybou
