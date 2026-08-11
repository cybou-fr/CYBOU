// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "PresenceService.h"

#include "cybou/fabric/FabricCodec.h"

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

const QStringList kProjectedCapabilities = {
    QStringLiteral("accepted-biography"), QStringLiteral("identity-continuity"),
    QStringLiteral("commitment-access"), QStringLiteral("prediction"),
    QStringLiteral("self-assessment"), QStringLiteral("attention-workspace"),
    QStringLiteral("consolidation"), QStringLiteral("presence-presentation"),
};

constexpr int kPresenceCommandTimeoutMs = 5000;

int presenceCommandTimeoutMs()
{
    bool ok = false;
    const int configured = qEnvironmentVariableIntValue(
        "CYBOU_PRESENCE_COMMAND_TIMEOUT_MS", &ok);
    return ok ? std::clamp(configured, 50, 60000) : kPresenceCommandTimeoutMs;
}

class CommandDeadline
{
public:
    CommandDeadline()
        : m_deadline(presenceCommandTimeoutMs())
    {
    }

    int remaining() const
    {
        return static_cast<int>(std::max<qint64>(0, m_deadline.remainingTime()));
    }

private:
    QDeadlineTimer m_deadline;
};

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
    for (const QString &capabilityId : kProjectedCapabilities) {
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
    const QVariantMap requirements{
        {QStringLiteral("activity"), QStringList{QStringLiteral("accepted-biography")}},
        {QStringLiteral("promise"), QStringList{QStringLiteral("accepted-biography"), QStringLiteral("commitment-access")}},
        {QStringLiteral("reflect"), QStringList{QStringLiteral("accepted-biography"), QStringLiteral("self-assessment")}},
        {QStringLiteral("fulfill"), QStringList{QStringLiteral("commitment-access")}},
        {QStringLiteral("abandon"), QStringList{QStringLiteral("commitment-access")}},
        {QStringLiteral("observe"), QStringList{QStringLiteral("prediction")}},
        {QStringLiteral("predict"), QStringList{QStringLiteral("prediction")}},
        {QStringLiteral("identity"), QStringList{QStringLiteral("identity-continuity")}},
        {QStringLiteral("attention"), QStringList{QStringLiteral("attention-workspace")}},
    };
    QVariantMap commands;
    for (auto it = requirements.cbegin(); it != requirements.cend(); ++it) {
        const QStringList required = it.value().toStringList();
        QStringList missing;
        for (const QString &capability : required)
            if (!isAvailable(snapshot, capability)) missing.append(capability);
        commands[it.key()] = QVariantMap{
            {QStringLiteral("available"), missing.isEmpty()},
            {QStringLiteral("requiredCapabilities"), required},
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

QString PresenceService::LastError() const
{
    if (!m_lastError.isEmpty()) {
        return m_lastError;
    }

    if (!m_events.lastError().isEmpty()) {
        return m_events.lastError();
    }
    if (!m_health.lastError().isEmpty()) return m_health.lastError();
    if (!m_identity.lastError().isEmpty()) {
        return m_identity.lastError();
    }
    if (!m_intentions.lastError().isEmpty()) {
        return m_intentions.lastError();
    }
    if (!m_predictor.lastError().isEmpty()) {
        return m_predictor.lastError();
    }
    if (!m_self.lastError().isEmpty()) {
        return m_self.lastError();
    }
    if (!m_workspace.lastError().isEmpty()) {
        return m_workspace.lastError();
    }
    return m_lifecycle.lastError();
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

QByteArray PresenceService::Activity(int limit) const
{
    QVariantList result;
    CommandDeadline deadline;

    int timeoutMs = deadline.remaining();
    const CapabilitySnapshot health = timeoutMs > 0
        ? m_health.snapshot(timeoutMs) : CapabilitySnapshot{};
    if (limit <= 0 || !isAvailable(health, QStringLiteral("accepted-biography"))) {
        return FabricCodec::encodeList(result);
    }

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return FabricCodec::encodeList(result);
    for (const CognitiveEnvelope &envelope :
         m_events.recent(limit, timeoutMs)) {
        QVariantMap map;
        map[QStringLiteral("when")] =
            envelope.wallTime.toLocalTime();
        map[QStringLiteral("organ")] =
            envelope.originOrgan;
        map[QStringLiteral("kind")] =
            kindToString(envelope.kind);
        map[QStringLiteral("thread")] =
            envelope.correlationId.toString(QUuid::WithoutBraces);
        result.append(map);
    }

    return FabricCodec::encodeList(result);
}

QByteArray PresenceService::DetailedObligations() const
{
    CommandDeadline deadline;
    int timeoutMs = deadline.remaining();
    const CapabilitySnapshot health = timeoutMs > 0
        ? m_health.snapshot(timeoutMs) : CapabilitySnapshot{};
    if (!isAvailable(health, QStringLiteral("commitment-access")))
        return FabricCodec::encodeList(QVariantList{});
    timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return FabricCodec::encodeList(QVariantList{});
    return FabricCodec::encodeList(
        m_intentions.open(timeoutMs));
}

bool PresenceService::appendUserObservation(
    const QString &event,
    const QVariantMap &details,
    QUuid *messageId,
    int timeoutMs)
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
        payload.insert(
            it.key(),
            QCborValue::fromVariant(it.value()));
    }
    observation.payloadCbor =
        payload.toCborValue().toCbor();

    if (m_events.append(observation, timeoutMs) == 0) {
        m_lastError = m_events.lastError();
        return false;
    }

    if (messageId) {
        *messageId = observation.messageId;
    }

    return true;
}

QString PresenceService::Promise(
    const QString &description)
{
    m_lastError.clear();
    const QString normalized = description.trimmed();
    CommandDeadline deadline;

    if (normalized.isEmpty()) {
        return {};
    }

    int timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return {};
    const CapabilitySnapshot capabilities = m_health.snapshot(timeoutMs);
    if (!isAvailable(capabilities, QStringLiteral("accepted-biography"))
        || !isAvailable(capabilities, QStringLiteral("commitment-access"))) return {};

    // Event1 is the required durability boundary for both records created by
    // Promise. Probe it before notifying auxiliary owners so an unavailable
    // journal consumes one bounded RPC budget instead of accumulating the
    // budgets of every step in this compound command.
    timeoutMs = deadline.remaining();
    if (timeoutMs == 0 || !m_events.isOpen(timeoutMs)) {
        m_lastError = m_events.lastError();
        return {};
    }

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return {};
    m_lifecycle.notifyUserActivity(QStringLiteral("presence.promise"), timeoutMs);

    QVariantMap details;
    details[QStringLiteral("description")] =
        normalized;

    QUuid requestId;
    timeoutMs = deadline.remaining();
    if (timeoutMs == 0 || !appendUserObservation(
            QStringLiteral("user-requested-intention"),
            details,
            &requestId,
            timeoutMs)) {
        return {};
    }

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return {};
    const QString intentionId = m_intentions.form(
        normalized,
        QStringLiteral("asked by the user"),
        requestId.toString(QUuid::WithoutBraces),
        timeoutMs);

    if (intentionId.isEmpty()) {
        m_lastError = m_intentions.lastError();
    }

    return intentionId;
}

bool PresenceService::Reflect()
{
    m_lastError.clear();
    CommandDeadline deadline;
    int timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return false;
    const CapabilitySnapshot capabilities = m_health.snapshot(timeoutMs);
    if (!isAvailable(capabilities, QStringLiteral("accepted-biography"))
        || !isAvailable(capabilities, QStringLiteral("self-assessment"))) return false;

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0 || !m_events.isOpen(timeoutMs)) {
        m_lastError = m_events.lastError();
        return false;
    }

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return false;
    m_lifecycle.notifyUserActivity(QStringLiteral("presence.reflect"), timeoutMs);

    QUuid requestId;
    timeoutMs = deadline.remaining();
    if (timeoutMs == 0 || !appendUserObservation(
            QStringLiteral("self-inspection-requested"),
            {},
            &requestId,
            timeoutMs)) {
        return false;
    }

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return false;
    const QVariantMap report = m_self.assess(
        requestId.toString(QUuid::WithoutBraces),
        timeoutMs);

    if (report.isEmpty()) {
        m_lastError = m_self.lastError();
        return false;
    }

    return true;
}

bool PresenceService::FulfillIndex(int index)
{
    m_lastError.clear();
    CommandDeadline deadline;
    int timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return false;
    const CapabilitySnapshot capabilities = m_health.snapshot(timeoutMs);
    if (!isAvailable(capabilities, QStringLiteral("commitment-access"))) return false;

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0 || !m_events.isOpen(timeoutMs)) {
        m_lastError = m_events.lastError();
        return false;
    }

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return false;
    m_lifecycle.notifyUserActivity(QStringLiteral("presence.fulfill"), timeoutMs);

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return false;
    const QVariantList open = m_intentions.open(timeoutMs);
    if (index < 0 || index >= open.size()) {
        return false;
    }

    const QString id =
        open.at(index)
            .toMap()
            .value(QStringLiteral("correlationId"))
            .toString();

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return false;
    const bool ok = m_intentions.close(
        id,
        0,
        QString(),
        timeoutMs);
    if (!ok) {
        m_lastError = m_intentions.lastError();
    }
    return ok;
}

bool PresenceService::AbandonIndex(int index)
{
    m_lastError.clear();
    CommandDeadline deadline;
    int timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return false;
    const CapabilitySnapshot capabilities = m_health.snapshot(timeoutMs);
    if (!isAvailable(capabilities, QStringLiteral("commitment-access"))) return false;

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0 || !m_events.isOpen(timeoutMs)) {
        m_lastError = m_events.lastError();
        return false;
    }

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return false;
    m_lifecycle.notifyUserActivity(QStringLiteral("presence.abandon"), timeoutMs);

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return false;
    const QVariantList open = m_intentions.open(timeoutMs);
    if (index < 0 || index >= open.size()) {
        return false;
    }

    const QString id =
        open.at(index)
            .toMap()
            .value(QStringLiteral("correlationId"))
            .toString();

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return false;
    const bool ok = m_intentions.close(
        id,
        1,
        QString(),
        timeoutMs);
    if (!ok) {
        m_lastError = m_intentions.lastError();
    }
    return ok;
}

bool PresenceService::Observe(
    const QString &subject,
    double value)
{
    m_lastError.clear();
    CommandDeadline deadline;
    int timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return false;
    const CapabilitySnapshot capabilities = m_health.snapshot(timeoutMs);
    if (!isAvailable(capabilities, QStringLiteral("prediction"))) return false;

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0 || !m_events.isOpen(timeoutMs)) {
        m_lastError = m_events.lastError();
        return false;
    }

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return false;
    m_lifecycle.notifyUserActivity(QStringLiteral("presence.observe"), timeoutMs);

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0) return false;
    const bool ok = m_predictor.observe(subject, value, timeoutMs);
    if (!ok) {
        m_lastError = m_predictor.lastError();
    }
    return ok;
}

QByteArray PresenceService::Predict(
    const QString &subject)
{
    m_lastError.clear();
    CommandDeadline deadline;
    int timeoutMs = deadline.remaining();
    if (timeoutMs == 0)
        return FabricCodec::encodeMap(QVariantMap{});
    const CapabilitySnapshot capabilities = m_health.snapshot(timeoutMs);
    if (!isAvailable(capabilities, QStringLiteral("prediction")))
        return FabricCodec::encodeMap(QVariantMap{});

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0)
        return FabricCodec::encodeMap(QVariantMap{});
    m_lifecycle.notifyUserActivity(QStringLiteral("presence.predict"), timeoutMs);

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0)
        return FabricCodec::encodeMap(QVariantMap{});
    const QVariantMap prediction =
        m_predictor.predict(subject, {}, timeoutMs);

    if (prediction.isEmpty()) {
        m_lastError = m_predictor.lastError();
    }

    return FabricCodec::encodeMap(prediction);
}

bool PresenceService::InterruptLifecycle(const QString &cause)
{
    m_lastError.clear();
    CommandDeadline deadline;
    bool delayOk = false;
    const int delayMs = qEnvironmentVariableIntValue(
        "CYBOU_PRESENCE_INTERRUPT_DELAY_MS", &delayOk);
    if (delayOk && delayMs > 0) {
        QThread::msleep(static_cast<unsigned long>(delayMs));
    }

    int timeoutMs = deadline.remaining();
    if (timeoutMs == 0) {
        m_lastError = QStringLiteral("lifecycle interruption deadline exceeded");
        return false;
    }
    const QString reason = cause.trimmed().isEmpty()
        ? QStringLiteral("interrupted by user")
        : cause.trimmed();
    const QVariantMap state = m_lifecycle.state(timeoutMs);
    if (state.isEmpty() && !m_lifecycle.lastError().isEmpty()) {
        m_lastError = m_lifecycle.lastError();
        return false;
    }
    if (state.value(QStringLiteral("status")).toString() != QStringLiteral("active")) {
        m_lastError = QStringLiteral("no active lifecycle run to interrupt");
        return false;
    }

    timeoutMs = deadline.remaining();
    if (timeoutMs == 0) {
        m_lastError = QStringLiteral("lifecycle interruption deadline exceeded");
        return false;
    }
    if (!m_lifecycle.finishRun(QStringLiteral("interrupted"), reason, timeoutMs)) {
        m_lastError = m_lifecycle.lastError();
        return false;
    }
    return true;
}

} // namespace cybou
