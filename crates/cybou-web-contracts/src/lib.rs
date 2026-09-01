// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Explicit versioned contract between Living Canvas and `cybou-web-gateway`.

use cybou_protocol::{CapabilityState, KnowledgeState, LocationRef, SchemaVersion};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// First web contract version. It is independent of internal D-Bus encoding versions.
pub const WEB_SCHEMA_V1: SchemaVersion = SchemaVersion(1);

/// How many provenance identifiers a disclosure projection carries.
///
/// Enough for someone cross-checking a delivery by hand, few enough that the surface stays a page
/// rather than a dump. The count it is a sample of is reported separately and is never truncated.
pub const DISCLOSURE_ITEM_SAMPLE: usize = 64;

/// Trust context established by the gateway, never by a frontend toggle.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionMode {
    /// Device-bound loopback session created by the desktop shell.
    LocalDesktop,
    /// Explicitly unauthenticated surface reachable by anyone who has the address.
    ///
    /// The name records the trust level, not the content: it promises no authentication was
    /// performed, and makes no claim that what it shows is non-personal. Whether a deployment
    /// points this mode at fixtures or at a live Mind is the owner's decision.
    PublicPreview,
    /// Authenticated browser session crossing the external network boundary.
    RemoteBrowser,
    /// Nothing is served until somebody signs in.
    ///
    /// The default, and what a deployment that can authenticate anybody should be. `PublicPreview`
    /// says a surface is deliberately open; this says the opposite, and says it before a reader has
    /// been shown anything rather than after. A visitor in this mode is served the sign-in surface
    /// and the routes that establish a session, and no projection of the person's Mind at all.
    SignInRequired,
}

/// Freshness carried with every owner projection.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Freshness {
    /// Projection is within the owner's declared freshness budget.
    Current,
    /// Projection is usable only as explicitly labelled stale context.
    Stale,
    /// Freshness could not be established.
    Unknown,
}

/// Authenticated session projection returned by `/api/v1/session`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// Opaque revocable session identifier.
    pub session_id: Uuid,
    /// Server-established trust context.
    pub mode: SessionMode,
    /// Stable named-consumer identifier used for context delivery policy.
    pub consumer_id: String,
    /// RFC 3339 expiry timestamp supplied by the gateway.
    pub expires_at: String,
}

/// One thing that was not supplied, as the person is shown it.
///
/// One thing the host concluded about itself, as a reader sees it.
///
/// Carries the readings behind it. A finding without them is indistinguishable from one a model
/// invented, and the whole reason this path is deterministic is so a person can check it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingProjection {
    /// Canonical identity of the causal `SystemInsight` (matches `ActionProposal.cause_id`).
    #[serde(default)]
    pub id: Option<Uuid>,
    /// The finding, in the frozen vocabulary.
    pub finding: String,
    /// Which thing it is about, for a finding about one named thing.
    ///
    /// `None` for a finding about the host itself. Carried separately from `means`, which describes
    /// the *kind*: two certificates close to expiry produce two findings whose `means` is the same
    /// sentence, and a reader looking at two identical rows cannot act on either.
    #[serde(default)]
    pub about: Option<String>,
    /// What it means, in the words a person would use.
    pub means: String,
    /// How well the evidence supports it: `weak`, `moderate` or `strong`.
    ///
    /// Named rather than numeric. A diagnosis reported as `0.81` invites comparison with another
    /// `0.79` as though the difference meant something; these three are distinguishable by what is
    /// actually behind them.
    pub strength: String,
    /// When the behaviour started, as far as the window can tell.
    pub since: String,
    /// The readings that led to it.
    pub readings: Vec<ReadingProjection>,
    /// What the host could offer to do about it, and what it decided about each.
    ///
    /// Empty is a real answer and a common one: a finding with no remedy produces no offer rather
    /// than a gesture.
    pub offers: Vec<OfferProjection>,
}

/// One reading behind a finding.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingProjection {
    /// The subject, in its frozen dotted name.
    pub subject: String,
    /// What was observed.
    pub observed: f64,
    /// What is ordinary for this host, once it has watched long enough to know.
    ///
    /// Absent on a host that has not. A categorical finding needs no baseline — a filesystem at 97%
    /// is a problem wherever it is — so the reading is the required half and this is the optional
    /// one. Sending  for a baseline nobody established would put a number on the page that
    /// says the observation is thirty-five spreads from normal, which is a fabricated claim about a
    /// host nobody has watched.
    #[serde(default)]
    pub ordinary: Option<f64>,
    /// How much this host ordinarily varies, when that is established.
    #[serde(default)]
    pub spread: Option<f64>,
}

/// Something the host could do, and what it decided about doing it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferProjection {
    /// The operation, in its frozen verb.
    pub operation: String,
    /// What it would act on.
    pub target: String,
    /// What being wrong about it costs: `low`, `medium`, `high` or `critical`.
    pub risk: String,
    /// Whether the system can undo it.
    ///
    /// Not whether it is safe. Restarting a service can be undone and still drops every connection
    /// it was holding; a package cache cannot be un-deleted and is among the safest things offered.
    pub reversible: bool,
    /// What the authorization gate decided: `granted`, `requires-confirmation` or `denied`.
    ///
    /// Nothing is `granted` on an installation nobody has configured, and nothing is carried out at
    /// all: there is no executor. The verdict is shown so a person can see what the gate would say
    /// before anything can act on it.
    pub verdict: String,
    /// Why, when the gate refused or wants asking.
    pub reason: String,
}

/// Where one watched subject is heading, and when it becomes a problem.
///
/// Separate from a finding. A finding is about now; this is about a rate, and the two answer
/// different questions — a disk at 71% produces no finding and may still be the most important thing
/// on the page.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionProjection {
    /// The subject, in its frozen dotted name.
    pub subject: String,
    /// Where it is heading: `rising`, `falling` or `flat`.
    pub trend: String,
    /// The most recent reading.
    pub current: f64,
    /// The value at which it becomes a problem.
    pub threshold: f64,
    /// When it arrives: `already`, `at-this-rate`, `not-at-this-rate` or `not-enough-history`.
    pub reaching: String,
    /// Seconds until it arrives, when it does.
    ///
    /// `None` for every other answer. Zero would be a time, and *not at this rate* is not a time —
    /// a surface that showed one number for both would render "arrives now" for a disk that is
    /// emptying.
    pub after_seconds: Option<i64>,
    /// Whether the arrival is further ahead than this window has watched.
    ///
    /// The most useful projection is usually the least certain: a young window saying a disk fills
    /// in three days is exactly what an operator needs and is an extrapolation. Both facts are the
    /// reader's.
    pub beyond_what_was_watched: bool,
    /// How long the window has actually watched, in seconds.
    pub watched_seconds: i64,
}

/// What the host currently makes of itself.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightProjection {
    /// Contract version.
    pub schema_version: SchemaVersion,
    /// Whether the telemetry organ could be read at all.
    pub knowledge: KnowledgeState,
    /// Whether it has watched long enough to have a notion of what is ordinary here.
    ///
    /// False for the first minutes after a restart. Its own field because *I have not watched long
    /// enough* and *nothing is wrong* are different answers, and a surface that could not tell them
    /// apart would show a confident all-clear built on four readings.
    pub watched_enough: bool,
    /// What needs attention.
    pub findings: Vec<FindingProjection>,
    /// The subjects that have no readings at all on this host.
    ///
    /// A kernel without pressure accounting, a host without swap. Named so an all-clear can be read
    /// against what was actually looked at, rather than as a statement about everything.
    pub unobserved: Vec<String>,
    /// Every watched thing and what is known about it, including what is not known.
    ///
    /// Carries the declared things that produced no reading. A surface built from the readings
    /// alone cannot tell a certificate nobody declared from one that was declared and never read,
    /// and those are opposites — the second is a watched thing an operator believes is covered.
    #[serde(default)]
    pub watched: Vec<WatchedProjection>,
    /// Where the watched subjects are heading.
    pub projections: Vec<ProjectionProjection>,
    /// The answer in prose, from the deterministic layer.
    ///
    /// Carried beside the structure rather than instead of it. The structure is what a surface
    /// draws; this is what the host would say if asked, and having both means the two can be
    /// compared.
    pub said: String,
}

/// One watched thing, as a reader receives it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchedProjection {
    /// What is watched, named by the thing it is about.
    pub subject: String,
    /// `observed`, `never-read`, `read-failed` or `stale`.
    ///
    /// The three unhappy ones are kept apart because they call for different actions: a path that
    /// does not exist, a file this process cannot open, and a sampler that has stopped.
    pub state: String,
    /// The instant the state refers to, if it refers to one.
    ///
    /// When it was read, when reading last failed, or when it was last read before going stale.
    /// Absent for a thing never read at all, because there is no instant to name.
    pub at: Option<String>,
    /// The value, for something actually observed.
    pub value: Option<f64>,
}

/// One earlier delivery to this consumer.
///
/// Counts and an instant, and deliberately not the items or the subjects. What a history answers is
/// *when did what I am given change, and by how much* — repeating every subject for every past
/// delivery would multiply the one thing the withholding rules exist to keep rare by the length of
/// the list.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryProjection {
    /// When it was recorded, RFC 3339.
    pub at: String,
    /// How many items crossed.
    pub supplied: u32,
    /// How many of them could be traced to a source.
    pub accounted_for: u32,
    /// How many distinct sources those came from.
    pub provenance_count: u32,
    /// How many items were held back.
    ///
    /// A count, not the reasons. The reasons for the delivery a person is looking at are beside it;
    /// a reason repeated for every past delivery is the same refusal restated sixteen times.
    pub withheld_count: u32,
}

/// A subject and a reason, never a value. Restating what was withheld in order to explain that it
/// was withheld would defeat the withholding, so this says what the item was *about* and why, and
/// stops there. Where even the subject would say too much, the subject is absent and the item is
/// still counted — an unnamed refusal is a smaller loss than a silent one.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WithheldProjection {
    /// What the item was about.
    pub subject: Option<String>,
    /// Why it was held back, in the frozen vocabulary of `WithheldBecause`.
    pub because: String,
}

/// What this consumer was last supplied, and what was kept from them.
///
/// The point of the surface is the difference between the two. Every system that assembles context
/// silently is doing this much bookkeeping internally; what it does not do is let the person it is
/// about read it (ADR-0030 B1, B6).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
// Four flags, and each answers a question the others cannot. Whether the delivery crossed a
// boundary, whether the consumer keeps it, whether the subjects are named to them, and whether a
// delivery happened at all are independent facts; folding any pair into one value would report
// them as the same fact and lose exactly the distinction this surface exists to make.
#[allow(clippy::struct_excessive_bools)]
pub struct DisclosureProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// The consumer this delivery went to.
    pub consumer_id: String,
    /// Whether the delivery crossed a boundary this system does not control.
    pub external_boundary: bool,
    /// Whether the consumer keeps what it was given.
    ///
    /// False for every consumer today, because no consumer learns from what it is shown. It is
    /// reported rather than assumed so that the first consumer which does retain something is
    /// visible here on the day it appears, instead of being discovered later.
    pub retains: bool,
    /// How many items were supplied.
    pub supplied: u32,
    /// How many of those named at least one contribution they were derived from.
    ///
    /// Deliberately separate from `supplied`. A projection that lost track of where one of its rows
    /// came from must say it supplied five things and can account for four — a delivery that
    /// silently reported four would be claiming provenance it does not have.
    pub accounted_for: u32,
    /// How many distinct contributions those items were derived from in total.
    ///
    /// A different scale from `supplied`, and routinely much larger: one belief can cite hundreds
    /// of contributions. It is carried as its own number because the length of `items` is a sample
    /// and answers a different question.
    pub provenance_count: u32,
    /// A bounded sample of the contributions those items were derived from.
    ///
    /// At most [`DISCLOSURE_ITEM_SAMPLE`] entries. The full set reached three thousand on the
    /// first live deployment of this surface, which is a hundred kilobytes served to anyone who
    /// asks and unreadable by the person it is for. `provenance_count` carries the true total, so
    /// the sample is never mistaken for it.
    pub items: Vec<Uuid>,
    /// What was held back, and why.
    pub withheld: Vec<WithheldProjection>,
    /// Whether the subjects of the withheld items are named to this consumer.
    ///
    /// False for a consumer whose trust is `Public`. The subject of a withheld item is the least
    /// that still lets a person ask "why was that held back?" — but to a stranger it is the thing
    /// that was held back. A concept refused for exposure is refused by its label, and a surface
    /// that published the label to explain the refusal would perform the disclosure it exists to
    /// report.
    ///
    /// The count and the reason are still given, because how much was refused and on what grounds
    /// are facts about the system rather than about the person. The flag exists so a reader can
    /// tell "no subject could be named" from "you are not the person this record is about".
    pub subjects_visible: bool,
    /// What this consumer was supplied before now, newest first.
    ///
    /// A person could see what they were supplied and not what they were supplied last week, which
    /// makes the surface a status light rather than a record. Only *changes* are recorded — a
    /// reader receiving the same projection every few seconds produces no new entry — so this is a
    /// list of the times what they were being given actually became something else.
    ///
    /// Bounded, and short. The durable record is the `ContextDisclosed` contribution in the
    /// Journal; this is a window onto its recent end, and a full list here would be the Journal
    /// again, in memory, unbounded.
    #[serde(default)]
    pub history: Vec<DeliveryProjection>,
    /// Whether the history above is everything that was ever supplied to this consumer.
    ///
    /// **False, always, in this build**, and carried rather than assumed. The list is held in the
    /// gateway process: it is bounded, and it starts empty when the gateway starts. A person
    /// reading three entries has been shown three changes since this process began, not three
    /// deliveries in the life of their machine — and a surface that let them believe otherwise
    /// would be answering *what was I supplied* with a fraction of the answer and no hedge.
    ///
    /// The complete record is the `ContextDisclosed` contributions in the Journal. Making this true
    /// means reading it back at start, which is worth doing and is not done.
    #[serde(default)]
    pub history_complete: bool,
    /// The earliest instant this history could possibly cover, RFC 3339.
    ///
    /// When the gateway started. What it bounds is coverage, not content: nothing was necessarily
    /// supplied then, and nothing before it can appear here.
    #[serde(default)]
    pub history_covers_since: Option<String>,
    /// Whether this consumer has been supplied anything at all yet.
    ///
    /// An empty delivery and no delivery are different facts, and the first reading of this route
    /// on a fresh gateway is the second one.
    pub delivered: bool,
}

/// Minimal capability row used by deterministic frontend fixtures.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityProjection {
    /// Stable capability identifier.
    pub id: String,
    /// Owner-projected state.
    pub state: CapabilityState,
    /// Whether the state itself is known.
    pub knowledge: KnowledgeState,
    /// Freshness of the projection.
    pub freshness: Freshness,
    /// Optional non-authoritative explanation.
    pub reason: Option<String>,
}

/// Atomic read model returned by `/api/v1/snapshot`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// Monotonic opaque projection revision.
    pub projection_version: u64,
    /// Cursor from which event resumption may be attempted.
    pub cursor: String,
    /// RFC 3339 observation timestamp.
    pub observed_at: String,
    /// Whether the aggregate projection is fresh.
    pub freshness: Freshness,
    /// Whether the aggregate projection is known; distinguishes known-empty from unavailable.
    pub knowledge: KnowledgeState,
    /// Current capability rows; an empty list is meaningful only when the projection is known.
    pub capabilities: Vec<CapabilityProjection>,
}

/// One owner's contribution to the Mind panel.
///
/// Every section carries its own [`KnowledgeState`] because the owners are separate processes and
/// fail separately: a Journal the gateway could not reach must not render as a Journal with no
/// contributions. `Unknown` means the owner was not reached, and the payload fields are then
/// absent rather than zero.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityProjection {
    /// Whether Identity1 answered.
    pub knowledge: KnowledgeState,
    /// Stable subject identifier.
    pub identity_id: Option<String>,
    /// RFC 3339 instant the subject was first created.
    pub origin: Option<String>,
    /// Number of sessions since origin.
    pub session_count: Option<u64>,
    /// Whole days since origin.
    pub age_in_days: Option<i64>,
    /// Architecture version the subject was created under.
    pub architecture_version: Option<String>,
}

/// One contribution as the Journal recorded it.
///
/// Metadata only. Payloads are sealed in the Journal and stay sealed here: what a reader learns is
/// that an organ recorded something of a given kind at a given moment, which is what a biography
/// looks like from outside.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContributionProjection {
    /// Stable contribution identity.
    pub message_id: String,
    /// Frozen contribution kind in its own spelling, or an explicit unknown for a kind this
    /// contract version cannot name.
    pub kind: String,
    /// Organ that recorded it.
    pub origin_organ: String,
    /// RFC 3339 instant the organ recorded.
    pub recorded_at: String,
}

/// What the canonical Journal holds, as reported by Event1.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalProjection {
    /// Whether Event1 answered.
    pub knowledge: KnowledgeState,
    /// Total accepted contributions.
    pub contribution_count: Option<u64>,
    /// Current erasure epoch.
    pub erasure_epoch: Option<u64>,
    /// The most recent contributions, newest first. Empty is meaningful only when `knowledge`
    /// is known.
    pub recent: Vec<ContributionProjection>,
    /// What verification has established about the chain, in the owner's own terms: `verified`,
    /// `broken at N`, `verified through N of M`, or unknown when no pass has run.
    pub integrity: Option<String>,
}

/// One open commitment as Intention1 holds it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitmentProjection {
    /// Intention identity.
    pub id: String,
    /// What was promised.
    pub description: String,
    /// Condition under which it became active.
    pub trigger: String,
    /// RFC 3339 formation instant.
    pub formed: String,
}

/// Open obligations, as reported by Intention1.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitmentsProjection {
    /// Whether Intention1 answered. An empty list is meaningful only when this is `Known`.
    pub knowledge: KnowledgeState,
    /// Number of open obligations.
    pub open_count: Option<u32>,
    /// The open obligations themselves.
    pub open: Vec<CommitmentProjection>,
}

/// Sleep/wake state, as reported by Lifecycle1.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleProjection {
    /// Whether Lifecycle1 answered.
    pub knowledge: KnowledgeState,
    /// Current mode, in the owner's own spelling.
    pub mode: Option<String>,
    /// RFC 3339 instant of the last observed user activity.
    pub last_user_activity_at: Option<String>,
}

/// How the system assesses itself, as reported by Self1.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfProjection {
    /// Whether Self1 answered.
    pub knowledge: KnowledgeState,
    /// The narration Self1 produced from its own report.
    ///
    /// Composed by the owner, not by the gateway and not by the page: these are the system's
    /// words about itself, and rewording them here would make them someone else's.
    pub narration: Option<String>,
    /// Whole days since origin, as the report measured them.
    pub age_in_days: Option<i64>,
    /// Sessions recorded.
    pub sessions: Option<u64>,
    /// Obligations still open at the moment of assessment.
    pub open_intentions: Option<u32>,
    /// Predictions that have been settled against an outcome.
    pub settled_predictions: Option<u32>,
}

/// What currently holds attention, as reported by Workspace1.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionProjection {
    /// Whether Workspace1 answered.
    pub knowledge: KnowledgeState,
    /// Correlation identity of the winning coalition, absent when nothing holds focus.
    pub focus: Option<String>,
    /// Salience of the winning coalition.
    pub salience: Option<f64>,
    /// Organs participating in it.
    pub organs: Vec<String>,
}

/// One belief as Epistemic1 holds it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeliefProjection {
    /// What the belief is about.
    pub subject: String,
    /// What is asserted about it.
    pub value: String,
    /// Confidence in the assertion.
    pub confidence: f64,
    /// Epistemic validity in the owner's own spelling: observed, stale, disputed, superseded or
    /// unknown. A belief and its validity are separate facts, and the second is the one that says
    /// whether the first may still be relied on.
    pub status: String,
    /// RFC 3339 instant the belief was last corroborated.
    pub last_corroborated_at: String,
}

/// What the system currently believes, as reported by Epistemic1.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeliefsProjection {
    /// Whether Epistemic1 answered. An empty list is meaningful only when this is known.
    pub knowledge: KnowledgeState,
    /// The beliefs themselves.
    pub beliefs: Vec<BeliefProjection>,
}

/// What the system perceives of the machine it runs on, as reported by Perception1.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerceptionProjection {
    /// Whether Perception1 answered.
    pub knowledge: KnowledgeState,
    /// Outcome of the last acquisition in the owner's own spelling.
    pub status: Option<String>,
    /// RFC 3339 instant of that acquisition.
    pub acquired_at: Option<String>,
    /// Which source was read.
    pub source_id: Option<String>,
}

/// One activated concept as Context1 holds it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptProjection {
    /// The concept.
    pub label: String,
    /// Activation weight.
    pub salience: f64,
    /// Why it was activated — the answer to "why was this retrieved?".
    pub activation_reason: String,
    /// RFC 3339 instant it was last activated.
    pub last_activated_at: String,
}

/// The associative context, as reported by Context1.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextProjection {
    /// Whether Context1 answered. An empty list is meaningful only when this is known.
    pub knowledge: KnowledgeState,
    /// Currently activated concepts.
    pub concepts: Vec<ConceptProjection>,
}

/// What Mind actually holds right now, returned by `/api/v1/mind`.
///
/// Only owners that hold real state appear here. Nothing in this projection is composed by the
/// gateway: each section is what one owner answered, or an explicit unknown.
// Salience is a float, so this projection compares by value rather than by identity.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MindProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// RFC 3339 instant the gateway assembled this read.
    pub observed_at: String,
    /// Subject continuity.
    pub identity: IdentityProjection,
    /// Canonical Journal.
    pub journal: JournalProjection,
    /// Open obligations.
    pub commitments: CommitmentsProjection,
    /// Sleep/wake state.
    pub lifecycle: LifecycleProjection,
    /// Self-assessment.
    pub self_model: SelfProjection,
    /// Global workspace attention.
    pub attention: AttentionProjection,
    /// Beliefs and their validity.
    pub beliefs: BeliefsProjection,
    /// Perception of the host.
    pub perception: PerceptionProjection,
    /// Associative context.
    pub context: ContextProjection,
}

/// Typed request payload to execute a bounded Shell capability.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellExecRequest {
    /// Full command line string.
    pub command: String,
    /// Which of the caller's shells this command is for.
    ///
    /// A Shell card is not a singleton — two of them are two places a person is standing, and a
    /// `cd` in one must not move the other. The card carries the instance it belongs to and sends
    /// it here, because the gateway cannot otherwise tell two cards in one session apart.
    ///
    /// Defaults to zero so a request written before this field existed still names a shell rather
    /// than being refused.
    #[serde(default)]
    pub instance: u32,
}

/// How many directory entries one listing carries.
///
/// A bound, because the sandbox root is a directory somebody else can fill. What is cut is always
/// reported as cut — a listing that quietly stopped would be a smaller directory, not a partial
/// answer.
pub const FILE_LISTING_MAX_ENTRIES: usize = 512;

/// How many bytes of a file one read carries.
pub const FILE_READ_MAX_BYTES: usize = 256 * 1024;

/// How many UTF-8 bytes one bounded sandbox write accepts.
pub const FILE_WRITE_MAX_BYTES: usize = 256 * 1024;

/// The terminal wire, shared with the owner that allocates the pseudoterminal.
///
/// Re-exported rather than restated. A browser and a host that disagreed about what a resize frame
/// is would disagree silently, in the one direction where the disagreement is a program drawing
/// into the wrong window.
pub use cybou_protocol::terminal::{
    FromGateway as TerminalFromGateway, FromOwner as TerminalFromOwner,
    MAX_COLUMNS as TERMINAL_MAX_COLUMNS, MAX_FRAME_BYTES as TERMINAL_MAX_FRAME_BYTES,
    MAX_ROWS as TERMINAL_MAX_ROWS, Refusal as TerminalRefusal,
    window_is_possible as terminal_window_is_possible,
};

/// How many bytes one file transfer carries, in either direction.
///
/// Larger than the text read and write bounds, because those exist to keep a projection small
/// enough to hold in a panel and this one exists to move a file. It is still a bound: the gateway
/// buffers a transfer whole, so an unbounded one is a way for a seat to spend the host's memory.
pub const FILE_TRANSFER_MAX_BYTES: usize = 8 * 1024 * 1024;

/// A file placed into the sandbox by whoever holds the seat.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileUploadRequest {
    /// Where to place it, interpreted inside the sandbox root and never outside it.
    pub path: String,
    /// The bytes, base64 with padding.
    ///
    /// Base64 rather than a raw body because every other route on this surface carries its path in
    /// a JSON body, and a raw body would have to carry that path in a header or a query string
    /// instead. A file name is the kind of thing that ends up in an access log.
    pub content_base64: String,
}

/// What the sandbox holds after a file was placed in it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileUploadProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// Owner-issued authority-domain reference for the file that was created.
    pub location: LocationRef,
    /// The path it was created at, as the sandbox resolved it.
    pub path: String,
    /// Lowercase SHA-256 of the bytes the sandbox read back after writing.
    pub content_sha256: String,
    /// How large the file is on disk.
    pub size_bytes: u64,
}

/// A person's answer to a proposal that was waiting on one.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmActionRequest {
    /// Which proposal is being answered.
    pub proposal_id: Uuid,
    /// The decision the person was shown when they answered.
    ///
    /// Not the decision they want; the decision they saw. Action1 refuses if it is no longer the
    /// one it holds.
    pub decision_id: Uuid,
}

/// Which path in the sandbox a request is about.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePathRequest {
    /// The path, interpreted inside the sandbox root and never outside it.
    pub path: String,
}

/// One entry in a directory, as the sandbox established it.
///
/// Name, kind and size, and nothing else. There is deliberately no mode and no owner: the sandbox
/// does not read them, and a surface that showed them would be showing a constant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryEntryProjection {
    /// Entry name, without any path.
    pub name: String,
    /// Whether the entry is a directory.
    pub is_dir: bool,
    /// Size in bytes for files. Zero for directories, whose size the sandbox does not establish.
    pub size_bytes: u64,
}

/// What a directory held when it was read.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryListingProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// The directory that was read, as the sandbox resolved it.
    pub path: String,
    /// Its entries, directories first, then by name.
    pub entries: Vec<DirectoryEntryProjection>,
    /// How many entries the directory actually held.
    ///
    /// Separate from the length of `entries` so a listing that hit
    /// [`FILE_LISTING_MAX_ENTRIES`] says so rather than presenting a bounded answer as a complete
    /// one. Partial is not empty truth.
    pub total_entries: u32,
    /// Whether entries were left out to stay inside the bound.
    pub truncated: bool,
}

/// Directory listing issued by the authenticated user's filesystem owner.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostDirectoryListingProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// Owner-issued authority-domain reference for the directory actually read.
    pub location: LocationRef,
    /// Entries returned within the ordinary listing bound.
    pub entries: Vec<DirectoryEntryProjection>,
    /// Total entries established by the owner.
    pub total_entries: u32,
    /// Whether the bounded response omitted entries.
    pub truncated: bool,
}

/// What a file held when it was read.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContentProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// The file that was read.
    pub path: String,
    /// Owner-issued authority-domain reference for the file that was actually read.
    ///
    /// The browser must carry this value into another panel rather than deriving authority from
    /// the spelling of [`Self::path`].
    pub location: LocationRef,
    /// Its text.
    pub text: String,
    /// How large the file is on disk.
    pub size_bytes: u64,
    /// Lowercase SHA-256 of the exact bytes returned in [`Self::text`].
    pub content_sha256: String,
}

/// Conditional write of a previously read file.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileWriteRequest {
    /// Owner-issued location returned by the read being edited.
    pub location: LocationRef,
    /// SHA-256 observed when the editor buffer was opened or last saved.
    pub expected_sha256: String,
    /// Complete replacement UTF-8 content.
    pub text: String,
}

/// Verified result of one conditional file write.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileWriteProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// Location that was written and then re-read.
    pub location: LocationRef,
    /// SHA-256 established by the post-write re-read.
    pub content_sha256: String,
    /// Verified byte size after the write.
    pub size_bytes: u64,
}

/// Request to create a new file with exclusive creation semantics (`O_CREAT | O_EXCL`).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCreateRequest {
    /// Relative path requested inside the authenticated seat's jail.
    pub path: String,
    /// Initial UTF-8 text content.
    pub text: String,
}

/// Request to write a file in the user's home authority domain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostFileWriteRequest {
    /// Target path inside user's home.
    pub path: String,
    /// Expected SHA-256 before modification, if provided.
    pub expected_sha256: Option<String>,
    /// Replacement UTF-8 text content.
    pub text: String,
}

/// Request to create a file in the user's home authority domain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostFileCreateRequest {
    /// Target path inside user's home.
    pub path: String,
    /// Initial UTF-8 text content.
    pub text: String,
    /// Whether creation must be exclusive.
    pub exclusive: bool,
}

/// Request to create a directory in the user's home authority domain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostDirectoryCreateRequest {
    /// Target path inside user's home.
    pub path: String,
    /// Whether parents should be created recursively.
    pub recursive: bool,
}

/// Request to rename or move a path in the user's home authority domain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostPathRenameRequest {
    /// Source path inside user's home.
    pub from_path: String,
    /// Destination path inside user's home.
    pub to_path: String,
}

/// Request to delete a path in the user's home authority domain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostPathDeleteRequest {
    /// Target path inside user's home.
    pub path: String,
    /// Whether directories should be removed recursively.
    pub recursive: bool,
}

/// Request to copy a path in the user's home authority domain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostPathCopyRequest {
    /// Source path inside user's home.
    pub from_path: String,
    /// Destination path inside user's home.
    pub to_path: String,
}

/// Category of a desktop workspace location.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LocationCategory {
    /// Real user home filesystem.
    Home,
    /// Autonomous Agent isolated workspace.
    AgentWorkspace,
    /// Bounded preview / demo sandbox.
    Sandbox,
    /// Historical read-only snapshot.
    Backup,
    /// Governed system configuration (/etc, etc.).
    System,
}

/// Request to end one of the caller's shells.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellCloseRequest {
    /// Which of the caller's shells to end.
    pub instance: u32,
}

/// User-scoped draft descriptor persisted on gateway/server for safe recovery.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDraftProjection {
    /// Unique draft identifier.
    pub draft_id: String,
    /// Title or label of the draft.
    pub title: String,
    /// UTF-8 draft text content.
    pub content: String,
    /// Base filesystem location if this draft was opened from an existing file.
    pub base_location: Option<LocationRef>,
    /// Observed SHA-256 at the moment the draft was derived.
    pub base_sha256: Option<String>,
    /// UTC timestamp of last server-side update.
    pub updated_at_utc: String,
}

/// Collection of user drafts returned by the gateway.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDraftListProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// List of user-scoped drafts.
    pub drafts: Vec<UserDraftProjection>,
}

/// One person's saved desktop arrangement.
///
/// The layout itself is a string and stays one all the way through the gateway. Its schema belongs
/// to the frontend that writes it; parsing it here would be a second implementation of that schema,
/// and it would be wrong the first time a card gained a field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopLayoutProjection {
    /// Contract schema version.
    pub schema_version: SchemaVersion,
    /// The arrangement this seat last saved, or `None` if it has never saved one. `None` is not an
    /// empty desktop: it means the browser should keep whatever it already had.
    pub layout: Option<String>,
    /// When it was saved.
    pub updated_at_utc: Option<String>,
}

/// Request to replace this seat's desktop arrangement.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopLayoutSaveRequest {
    /// The arrangement, as the browser wrote it.
    pub layout: String,
}

/// Request to save or update a user draft.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDraftSaveRequest {
    /// Unique draft identifier.
    pub draft_id: String,
    /// Title or label of the draft.
    pub title: String,
    /// UTF-8 draft text content.
    pub content: String,
    /// Base filesystem location if this draft was opened from an existing file.
    pub base_location: Option<LocationRef>,
    /// Observed SHA-256 at the moment the draft was derived.
    pub base_sha256: Option<String>,
}

/// Request to delete a user draft.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDraftDeleteRequest {
    /// Unique draft identifier to remove.
    pub draft_id: String,
}

/// Typed response from executing a bounded Shell capability.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellExecResponse {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// Standard execution exit code (0 for success).
    pub exit_code: i32,
    /// Standard output text.
    pub stdout: String,
    /// Standard error text.
    pub stderr: String,
    /// Sandbox working directory after command.
    pub cwd: String,
}

/// One criticism check result behind an action proposal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CriticismCheckProjection {
    /// Evaluation rule identifier.
    pub rule_id: String,
    /// Human-readable rule description.
    pub description: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Diagnostic objection or note if check failed.
    pub objection: Option<String>,
}

/// Durable boundary record before an action mutation touches the host.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionStartedProjection {
    /// Stable execution identity.
    pub attempt_id: Uuid,
    /// Proposal identity.
    pub proposal_id: Uuid,
    /// Authorized operation name.
    pub operation: String,
    /// Target resource affected.
    pub target_resource: String,
    /// RFC 3339 start timestamp.
    pub started_at: String,
}

/// Report from the executor following execution attempt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionAttemptProjection {
    /// Execution identity.
    pub attempt_id: Uuid,
    /// Proposal identity.
    pub proposal_id: Uuid,
    /// Operation name.
    pub operation: String,
    /// Target resource.
    pub target_resource: String,
    /// Status report: completed, failed, refused, did-not-finish.
    pub report: String,
    /// Reason if failed or refused.
    pub reason: Option<String>,
    /// RFC 3339 completion instant.
    pub ended_at: Option<String>,
}

/// Observed relief and re-observation verdict after an action.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionOutcomeProjection {
    /// Outcome identity.
    pub outcome_id: Uuid,
    /// Proposal identity.
    pub proposal_id: Uuid,
    /// Relief verdict: relieved, still-present, worse, not-established.
    pub relief: String,
    /// Agreement between executor claim and telemetry observation: agree, disagree, not-comparable.
    pub agreement: String,
    /// Disagreement detail if any.
    pub disagreement: Option<String>,
    /// Observed metric reading before the action.
    pub observation_before: Option<f64>,
    /// Observed metric reading after the action.
    pub observation_after: Option<f64>,
    /// RFC 3339 conclusion instant.
    pub concluded_at: String,
}

/// One proposal after Action1 has evaluated, decided, executed, and re-observed it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionRecordProjection {
    /// Proposal identity.
    pub proposal_id: Uuid,
    /// Identity of the decision this record currently carries.
    ///
    /// On the wire so a person answering a confirmation can say which decision they were shown.
    /// A proposal re-decided between being drawn and being clicked is a different prompt, and
    /// without this the answer to one question could authorize another.
    pub decision_id: Uuid,
    /// Associated telemetry insight/cause identity, if any.
    pub cause_id: Option<Uuid>,
    /// Proposer description.
    pub proposer: String,
    /// High-level communicative intent.
    pub intent: String,
    /// Operation verb (e.g. "service.restart").
    pub operation: String,
    /// Target resource (e.g. "systemd:demo-api.service").
    pub target_resource: String,
    /// Risk level: low, medium, high, critical.
    pub risk_level: String,
    /// Whether the action is reversible.
    pub reversible: bool,
    /// RFC 3339 proposal instant.
    pub proposed_at: String,
    /// Criticism checks evaluated.
    pub checks: Vec<CriticismCheckProjection>,
    /// Policy authorization verdict: granted, requires-confirmation, denied.
    pub verdict: String,
    /// Verdict reason or explanation.
    pub verdict_reason: Option<String>,
    /// Durable execution started boundary, if claimed.
    pub execution_started: Option<ExecutionStartedProjection>,
    /// Execution attempt report, if completed.
    pub attempt: Option<ExecutionAttemptProjection>,
    /// Independent outcome observation, if concluded.
    pub outcome: Option<ActionOutcomeProjection>,
}

pub use cybou_protocol::agent::{
    AgentOffersResponse, AgentTaskView, OfferedModelView, OfferedProfileView,
};
pub use cybou_protocol::notification::{
    NotificationAction, NotificationActionKind, NotificationCategory, NotificationItem,
    NotificationSeverity,
};
pub use cybou_protocol::operation::{
    OperationKind, OperationLogEntry, OperationProgress, OperationRecord, OperationState,
};

/// List of active and historical server operations.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationsListProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// Count of currently running or queued operations.
    pub active_count: usize,
    /// Operations list, latest first.
    pub operations: Vec<OperationRecord>,
}

/// Execution logs for a specific operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationLogsProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// Operation ID.
    pub operation_id: uuid::Uuid,
    /// Ordered log entries.
    pub logs: Vec<OperationLogEntry>,
}

/// Request to cancel a running operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationCancelRequest {
    /// Target operation identifier.
    pub operation_id: uuid::Uuid,
    /// Optional cancellation reason.
    pub reason: Option<String>,
}

/// List of notifications grouped with summary counters.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationsListProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// Number of unread notifications.
    pub unread_count: usize,
    /// Number of high-priority attention notifications requiring user action.
    pub attention_count: usize,
    /// Notification items list, latest first.
    pub notifications: Vec<NotificationItem>,
}

/// Request to dismiss or mark notifications as read.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDismissRequest {
    /// Specific notification ID to dismiss, or `None` if `dismiss_all` is true.
    pub notification_id: Option<uuid::Uuid>,
    /// Whether to dismiss all notifications.
    #[serde(default)]
    pub dismiss_all: bool,
}

/// Request to trigger an interactive notification action.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationActionRequest {
    /// Target notification ID.
    pub notification_id: uuid::Uuid,
    /// Action button ID that was clicked.
    pub action_id: String,
}

// -------------------------------------------------------------------------------------------------
// System Substrate (Services, Processes, System Monitor, System Logs)
// -------------------------------------------------------------------------------------------------

pub use cybou_protocol::system::{
    CpuCoreStat, DiskPartitionInfo, LogsUnavailable, NetworkInterfaceInfo, ProcessRecord,
    ProcessSignal, ServiceAction, ServiceRecord, ServiceState, ServiceUnitType, SystemLogEntry,
};

/// Projection listing system service daemons.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServicesListProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Total count of actively running services.
    pub active_count: usize,
    /// Total count of failed services requiring attention.
    pub failed_count: usize,
    /// System service units list.
    pub services: Vec<ServiceRecord>,
}

/// Request to perform a state transition action on a system service.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceActionRequest {
    /// Service unit name (e.g. `cybou-web-gateway.service`).
    pub name: String,
    /// Desired action.
    pub action: ServiceAction,
}

/// Projection listing operating system processes.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessesListProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Total active process count.
    pub total_count: usize,
    /// Number of process records included in this bounded response.
    pub showing_count: usize,
    /// Whether additional observed processes were omitted from the response.
    pub truncated: bool,
    /// Aggregate CPU utilization percentage across all processes.
    pub total_cpu_percent: f32,
    /// Total resident memory in bytes consumed by listed processes.
    pub total_memory_bytes: u64,
    /// Running process records.
    pub processes: Vec<ProcessRecord>,
}

/// Request to send a signal to an operating system process.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSignalRequest {
    /// Target Process ID.
    pub pid: u32,
    /// Signal to deliver.
    pub signal: ProcessSignal,
}

/// Real-time hardware telemetry and system resource metrics projection.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMonitorProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Machine hostname.
    pub hostname: String,
    /// Operating system release identifier.
    pub os_release: String,
    /// System uptime in seconds.
    pub uptime_seconds: u64,
    /// System load averages (1 min, 5 min, 15 min).
    pub load_avg: [f32; 3],
    /// Overall CPU utilization percentage (0.0 - 100.0%).
    pub total_cpu_percent: f32,
    /// Individual CPU core utilization metrics.
    pub cores: Vec<CpuCoreStat>,
    /// Total physical RAM in bytes.
    pub memory_total_bytes: u64,
    /// Used physical RAM in bytes.
    pub memory_used_bytes: u64,
    /// Free/available physical RAM in bytes.
    pub memory_free_bytes: u64,
    /// Total swap capacity in bytes.
    pub swap_total_bytes: u64,
    /// Used swap in bytes.
    pub swap_used_bytes: u64,
    /// Mounted filesystem disk partitions.
    pub disk_partitions: Vec<DiskPartitionInfo>,
    /// Network interface I/O counters.
    pub network_interfaces: Vec<NetworkInterfaceInfo>,
}

/// System log feed projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemLogsProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Log records in chronological order.
    pub logs: Vec<SystemLogEntry>,
    /// Why the feed is empty, when the reason is not that the query matched nothing.
    ///
    /// `None` means the journal was read. An empty `logs` beside `None` is an honest "no entries
    /// matched"; an empty `logs` beside `Some` is a machine this reader cannot hear.
    pub unavailable: Option<LogsUnavailable>,
    /// Whether this reader can see the whole system journal, or only its own account's.
    ///
    /// `journalctl` does not fail for a process outside the `systemd-journal` group: it quietly
    /// narrows to that account's own entries. A feed that did not say so would answer "what is
    /// this host doing" with one service's half of it and look complete.
    pub system_journal_readable: bool,
}

/// Query parameters for searching system logs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemLogsQueryRequest {
    /// Optional unit filter (e.g. `cybou-agentd.service`).
    pub unit: Option<String>,
    /// Optional severity filter (e.g. `err`, `warning`).
    pub severity: Option<String>,
    /// Search substring or pattern.
    pub search: Option<String>,
    /// Maximum log lines to return (capped at 500).
    pub limit: Option<usize>,
}

// -------------------------------------------------------------------------------------------------
// Storage, Network, Packages & Governed Updates Substrate (Milestone 4)
// -------------------------------------------------------------------------------------------------

pub use cybou_protocol::system::{
    BtrfsSubvolumeRecord, NetworkConnectionKind, NetworkConnectionRecord, PackageActionKind,
    PackageRecord, PackageStatus, SnapshotRecord, SystemUpdatesSummary,
};

/// Projection listing Btrfs subvolumes and snapshots.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Whether a storage owner established the complete projection.
    pub state: SystemSurfaceState,
    /// Btrfs subvolumes.
    pub subvolumes: Vec<BtrfsSubvolumeRecord>,
    /// Snapshots list.
    pub snapshots: Vec<SnapshotRecord>,
    /// Total pool storage capacity in bytes.
    pub total_space_bytes: u64,
    /// Unallocated free pool storage capacity in bytes.
    pub free_space_bytes: u64,
}

/// Request to create a point-in-time filesystem snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSnapshotRequest {
    /// Subvolume to snapshot (e.g. `@home` or `@root`).
    pub subvolume_path: String,
    /// User label for the snapshot.
    pub name: String,
    /// Whether the snapshot is immutable/read-only.
    pub readonly: bool,
}

/// Request to restore a filesystem snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreSnapshotRequest {
    /// Target snapshot ID to restore.
    pub snapshot_id: String,
}

/// Projection listing network interfaces, Wi-Fi, and VPN tunnels.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Whether a host network reader established this list.
    pub state: SystemSurfaceState,
    /// Active and configured network connections.
    pub connections: Vec<NetworkConnectionRecord>,
}

/// Request to connect or disconnect a network profile.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConnectRequest {
    /// Target connection ID.
    pub connection_id: String,
    /// True to bring up / connect, false to bring down / disconnect.
    pub activate: bool,
}

/// Projection listing software packages.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackagesProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Whether a package database reader established these values.
    pub state: SystemSurfaceState,
    /// Total installed packages count.
    pub installed_count: usize,
    /// Total upgradable packages count.
    pub upgradable_count: usize,
    /// Packages list matching query.
    pub packages: Vec<PackageRecord>,
}

/// Governed package operation request (install/upgrade/remove).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageActionRequest {
    /// Target package name.
    pub name: String,
    /// Action to execute.
    pub action: PackageActionKind,
}

/// System-wide software update status projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemUpdatesProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Whether an update provider established this summary.
    pub state: SystemSurfaceState,
    /// Update status summary.
    pub summary: SystemUpdatesSummary,
}

/// Request to apply pending system updates.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyUpdatesRequest {
    /// Optional list of specific package names to update, or None for all.
    pub package_names: Option<Vec<String>>,
}

// -------------------------------------------------------------------------------------------------
// Users, Security & Backup Substrate (Milestone 5)
// -------------------------------------------------------------------------------------------------

pub use cybou_protocol::system::{
    BackupArchiveRecord, BackupRepositoryRecord, BackupScheduleRecord, SecurityAuditEntry,
    SecurityPolicyRecord, SshKeyRecord, UserAccountRecord,
};

/// Projection listing configured user accounts and authorized SSH keys.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsersSettingsProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Whether an NSS/account reader established these lists.
    pub state: SystemSurfaceState,
    /// Configured user accounts.
    pub users: Vec<UserAccountRecord>,
    /// Authorized SSH public keys for active user.
    pub ssh_keys: Vec<SshKeyRecord>,
}

/// Request to create a new user account.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    /// Username (e.g. `alice`).
    pub username: String,
    /// Display full name (e.g. `Alice Walker`).
    pub full_name: String,
    /// Whether user has administrative rights.
    pub is_admin: bool,
}

/// Request to add an authorized SSH public key.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddSshKeyRequest {
    /// Key label/comment.
    pub name: String,
    /// Public key contents (`ssh-ed25519 AAAAC3...`).
    pub public_key: String,
}

/// Request to delete an authorized SSH public key.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSshKeyRequest {
    /// Target key ID.
    pub key_id: String,
}

/// Projection describing sandbox confinement policies and security audit events.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SystemSurfaceState {
    /// A reader/provider established the returned state.
    Known,
    /// No reader could establish the host state.
    Unknown,
    /// The operator has not configured a provider for this surface.
    NotConfigured,
}

/// Projection describing sandbox confinement policies and security audit events.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecuritySettingsProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Whether a host reader established this projection.
    pub state: SystemSurfaceState,
    /// Confinement policy rules.
    pub policy: Option<SecurityPolicyRecord>,
    /// Recent security audit events.
    pub audit_log: Vec<SecurityAuditEntry>,
}

/// Request to update security confinement policies.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(
    clippy::struct_excessive_bools,
    reason = "one field per kernel confinement mechanism"
)]
pub struct UpdateSecurityPolicyRequest {
    /// Linux Landlock filesystem sandbox status.
    pub landlock_enabled: bool,
    /// Bubblewrap unprivileged user namespace isolation status.
    pub bubblewrap_enabled: bool,
    /// `AppArmor` LSM enforcement status.
    pub apparmor_enforcing: bool,
    /// Strict Seccomp BPF syscall filter status.
    pub seccomp_strict: bool,
    /// Strict network egress firewall enforcement.
    pub egress_firewall_strict: bool,
}

/// Projection detailing Borg/Btrfs backup repository, snapshots timeline, and schedule.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSettingsProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Whether an operator configured a backup provider.
    pub state: SystemSurfaceState,
    /// Target backup repository metadata.
    pub repository: Option<BackupRepositoryRecord>,
    /// Historical snapshot archives in repository.
    pub archives: Vec<BackupArchiveRecord>,
    /// Retention and automation schedule.
    pub schedule: Option<BackupScheduleRecord>,
}

/// Request to trigger an immediate backup snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerBackupRequest {
    /// Optional label for the archive snapshot.
    pub name: Option<String>,
}

/// Request to restore a backup archive.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreArchiveRequest {
    /// Target archive ID.
    pub archive_id: String,
    /// Optional target extraction path (or None to restore home).
    pub target_path: Option<String>,
}

/// Request to update automated backup schedule and retention policy.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBackupScheduleRequest {
    /// Enable/disable automated backups.
    pub enabled: bool,
    /// Frequency (`hourly`, `daily`, `weekly`).
    pub frequency: String,
    /// Daily retention count.
    pub retention_daily: u32,
    /// Weekly retention count.
    pub retention_weekly: u32,
    /// Monthly retention count.
    pub retention_monthly: u32,
}

pub use cybou_protocol::personal::{
    CalendarEventRecord, ContactRecord, MailAccountRecord, MailFolderKind, MailMessageRecord,
    NoteRecord,
};

/// Projection for personal email accounts and messages.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Configured email accounts.
    pub accounts: Vec<MailAccountRecord>,
    /// Listed messages in the active view.
    pub messages: Vec<MailMessageRecord>,
    /// Active account ID.
    pub active_account_id: String,
    /// Active folder view.
    pub active_folder: MailFolderKind,
}

/// Request to compose and dispatch an outbound email.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMailRequest {
    /// Sender account ID.
    pub account_id: String,
    /// Recipient addresses.
    pub to: Vec<String>,
    /// Subject line.
    pub subject: String,
    /// Plaintext/markdown body.
    pub body: String,
    /// Optional cognitive subject reference.
    pub referenced_subject: Option<cybou_protocol::SubjectRef>,
}

/// Projection for personal calendar schedule and events.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Scheduled calendar events.
    pub events: Vec<CalendarEventRecord>,
}

/// Request to create a new calendar schedule event.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCalendarEventRequest {
    /// Event title.
    pub title: String,
    /// Event description.
    pub description: String,
    /// ISO 8601 start time.
    pub start_time: String,
    /// ISO 8601 end time.
    pub end_time: String,
    /// Whether this event is all-day.
    pub is_all_day: bool,
    /// Optional location.
    pub location: Option<String>,
    /// Invited attendees.
    pub attendees: Vec<String>,
    /// Category color identifier.
    pub color_category: String,
    /// Optional cognitive subject reference.
    pub referenced_subject: Option<cybou_protocol::SubjectRef>,
}

/// Projection for personal knowledge notes.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotesProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Saved personal notes.
    pub notes: Vec<NoteRecord>,
}

/// Request to create a new personal note.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteRequest {
    /// Note title.
    pub title: String,
    /// Markdown body content.
    pub content_markdown: String,
    /// Descriptive tags.
    pub tags: Vec<String>,
    /// Pin at top.
    pub is_pinned: bool,
    /// Optional cognitive subject reference.
    pub referenced_subject: Option<cybou_protocol::SubjectRef>,
}

/// Request to update an existing personal note.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteRequest {
    /// Note identifier.
    pub id: String,
    /// Updated title.
    pub title: String,
    /// Updated Markdown content.
    pub content_markdown: String,
    /// Updated tags.
    pub tags: Vec<String>,
    /// Updated pin status.
    pub is_pinned: bool,
}

/// Projection for personal address book contacts.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactsProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Address book contacts.
    pub contacts: Vec<ContactRecord>,
}

/// Request to create a new contact entry.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateContactRequest {
    /// Full contact name.
    pub name: String,
    /// Primary email address.
    pub email: String,
    /// Professional role or title.
    pub role: String,
    /// Organization or company.
    pub organization: String,
    /// Optional phone number.
    pub phone: Option<String>,
    /// Descriptive tags.
    pub tags: Vec<String>,
    /// Freeform notes.
    pub notes: String,
    /// Optional cognitive subject reference.
    pub referenced_subject: Option<cybou_protocol::SubjectRef>,
}

pub use cybou_protocol::cognitive::{
    CognitiveEdgeRecord, CognitiveEdgeType, CognitiveGraphRecord, CognitiveNodeRecord,
    CognitiveNodeType, EventJournalEntry,
};

/// Projection for the deep unified Cognitive Graph and causal relations.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveGraphProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Graph nodes and edges.
    pub graph: CognitiveGraphRecord,
    /// Currently focused or rooted node ID, if any.
    pub focus_node_id: Option<String>,
}

/// Projection for the canonical Event1 chronological journal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventJournalProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Chronological list of journal entries.
    pub entries: Vec<EventJournalEntry>,
    /// Total count of matching records.
    pub total_count: usize,
}

/// Request to query subgraphs, causal paths, and semantic relations in the Cognitive Graph.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveQueryRequest {
    /// Search query string.
    pub query: String,
    /// Filter by node types if specified.
    pub node_types: Option<Vec<String>>,
    /// Root node ID to traverse from.
    pub focus_id: Option<String>,
    /// Traversal max depth.
    pub max_depth: Option<u32>,
}

pub use cybou_protocol::agent::{CapsuleAction, CapsuleTelemetryRecord};

/// Request to perform a lifecycle or boundary control action on an active capsule.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapsuleControlRequest {
    /// Desired action.
    pub action: CapsuleAction,
}

/// Real-time telemetry projection for an individual agent capsule.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapsuleTelemetryProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Live telemetry readings.
    pub telemetry: CapsuleTelemetryRecord,
}

pub use cybou_protocol::meaning::{
    CognitiveAct, CognitiveActKind, MeaningInterpretation, Qualification, ReferenceCandidate,
    ReferenceResolution, ResponsePlan,
};

/// Request to interpret a natural language query into a typed cognitive act without LLM guessing.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeaningInterpretRequest {
    /// The natural language utterance or command.
    pub utterance: String,
    /// Desired response realization language ("en", "ru", "de", "fr").
    pub language: Option<String>,
}

/// Structured interpretation and planned realization output from the Meaning1 engine.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeaningInterpretProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Typed semantic interpretation of the utterance.
    pub interpretation: MeaningInterpretation,
    /// Abstract plan with epistemic qualifications before prose generation.
    pub response_plan: Option<ResponsePlan>,
    /// Realized natural language response in the requested tongue.
    pub realization: Option<String>,
}

/// Projection of dialogue memory state and referents.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogueMemoryProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Current conversation turn.
    pub current_turn: u64,
    /// Actively remembered candidate entity labels.
    pub remembered_referents: Vec<String>,
    /// Turn retention bound.
    pub turns_bound: u64,
}

pub use cybou_protocol::governance::{ActorKind, TaskScope, ToolCallProposal, ToolCallVerdict};
pub use cybou_protocol::learning::{
    ArtifactStatus, LearnedArtifactLineage, LearningCandidate, LearningLayer, PromotionGate,
};
pub use cybou_protocol::promotion::{DemonstratedOutcome, Promoted, PromotionRefused};

/// Projection for active and historical lifelong learning candidates.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningCandidatesProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Active learning candidates.
    pub candidates: Vec<LearningCandidate>,
    /// Total count of candidate records.
    pub total_count: usize,
}

/// Projection for promoted learned artifacts and skill lineages.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearnedArtifactsProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Promoted learned artifacts.
    pub artifacts: Vec<LearnedArtifactLineage>,
    /// Total count of durable artifacts.
    pub total_count: usize,
}

/// Request to propose a new candidate learning proposition derived from episodes.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposeLearningCandidateRequest {
    /// Target learning layer.
    pub layer: LearningLayer,
    /// Proposed generalization or behavioral rule.
    pub generalization: String,
    /// Target applicability scope.
    pub scope: String,
    /// Source evidence message IDs.
    pub source_evidence: Vec<Uuid>,
    /// Outcome evidence message IDs.
    pub outcome_evidence: Vec<Uuid>,
}

/// Detailed result of evaluating a candidate against promotion criteria.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateEvaluationProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Evaluated candidate identifier.
    pub candidate_id: Uuid,
    /// Promotion approval record if passed.
    pub promoted: Option<Promoted>,
    /// Reason for refusal if failed.
    pub refused: Option<PromotionRefused>,
    /// Durable artifact created if promoted.
    pub artifact: Option<LearnedArtifactLineage>,
}

/// Request to revoke or deprecate a promoted learned artifact.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeArtifactRequest {
    /// Target artifact identifier.
    pub artifact_id: Uuid,
    /// Reason for revocation.
    pub reason: String,
}

/// Projection for active task-scoped capability and tool grants.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceScopesProjection {
    /// Web contract schema version.
    pub schema_version: SchemaVersion,
    /// Active task scopes.
    pub scopes: Vec<TaskScope>,
}

#[cfg(test)]
mod tests {
    use super::{
        CommitmentsProjection, FileContentProjection, LocationRef, MindProjection, SessionMode,
        SessionProjection, SnapshotProjection, WEB_SCHEMA_V1,
    };

    const SESSION_FIXTURE: &str = include_str!("../../../fixtures/web/v1/session-local.json");
    const SNAPSHOT_FIXTURE: &str = include_str!("../../../fixtures/web/v1/snapshot-nominal.json");
    const SESSION_SCHEMA: &str = include_str!("../../../schemas/web/v1/session.schema.json");
    const SNAPSHOT_SCHEMA: &str = include_str!("../../../schemas/web/v1/snapshot.schema.json");
    const MIND_FIXTURE: &str = include_str!("../../../fixtures/web/v1/mind-nominal.json");
    const MIND_SCHEMA: &str = include_str!("../../../schemas/web/v1/mind.schema.json");

    #[test]
    fn local_session_fixture_is_explicitly_local() {
        let projection: SessionProjection =
            serde_json::from_str(SESSION_FIXTURE).expect("valid local session fixture");
        assert_eq!(projection.schema_version, WEB_SCHEMA_V1);
        assert_eq!(projection.mode, SessionMode::LocalDesktop);
        assert!(!projection.consumer_id.is_empty());
    }

    #[test]
    fn nominal_snapshot_round_trips_without_losing_state() {
        let projection: SnapshotProjection =
            serde_json::from_str(SNAPSHOT_FIXTURE).expect("valid nominal snapshot fixture");
        let encoded = serde_json::to_string(&projection).expect("serialize nominal snapshot");
        let decoded: SnapshotProjection =
            serde_json::from_str(&encoded).expect("round-trip nominal snapshot");
        assert_eq!(decoded, projection);
        assert_eq!(projection.schema_version, WEB_SCHEMA_V1);
        assert!(!projection.cursor.is_empty());
        assert_eq!(projection.knowledge, cybou_protocol::KnowledgeState::Known);
    }

    #[test]
    fn mind_fixture_round_trips_and_keeps_unknown_distinct_from_empty() {
        let projection: MindProjection =
            serde_json::from_str(MIND_FIXTURE).expect("valid nominal mind fixture");
        let encoded = serde_json::to_string(&projection).expect("serialize mind projection");
        let decoded: MindProjection =
            serde_json::from_str(&encoded).expect("round-trip mind projection");
        assert_eq!(decoded, projection);
        assert_eq!(projection.schema_version, WEB_SCHEMA_V1);

        // A section the gateway could not read must not be readable as a section holding nothing.
        let unreached = CommitmentsProjection {
            knowledge: cybou_protocol::KnowledgeState::Unknown,
            open_count: None,
            open: Vec::new(),
        };
        let known_empty = CommitmentsProjection {
            knowledge: cybou_protocol::KnowledgeState::Known,
            open_count: Some(0),
            open: Vec::new(),
        };
        assert_ne!(unreached, known_empty);
    }

    #[test]
    fn checked_in_json_schemas_are_v1_and_closed() {
        for raw in [SESSION_SCHEMA, SNAPSHOT_SCHEMA, MIND_SCHEMA] {
            let schema: serde_json::Value = serde_json::from_str(raw).expect("valid JSON schema");
            assert_eq!(
                schema["$schema"],
                "https://json-schema.org/draft/2020-12/schema"
            );
            assert_eq!(schema["additionalProperties"], false);
            assert_eq!(schema["properties"]["schemaVersion"]["const"], 1);
        }
    }

    #[test]
    fn file_content_keeps_the_owner_issued_authority_domain() {
        let projection = FileContentProjection {
            schema_version: WEB_SCHEMA_V1,
            path: "/etc/example.conf".to_string(),
            location: LocationRef::SafeShellJail {
                session_id: "seat-1".to_string(),
                path: "/etc/example.conf".to_string(),
            },
            text: "demo".to_string(),
            size_bytes: 4,
            content_sha256: "2a97516c354b68848cdbd8f54a226a0a848a850a1c904e3cacd9d91f2571a4bf"
                .to_string(),
        };

        let encoded = serde_json::to_string(&projection).expect("encode file projection");
        let decoded: FileContentProjection =
            serde_json::from_str(&encoded).expect("decode file projection");
        assert_eq!(decoded, projection);
        assert!(matches!(
            decoded.location,
            LocationRef::SafeShellJail { .. }
        ));
    }

    #[test]
    fn operations_and_notifications_round_trip() {
        use super::{
            NotificationAction, NotificationActionKind, NotificationCategory, NotificationItem,
            NotificationSeverity, NotificationsListProjection, OperationKind, OperationProgress,
            OperationRecord, OperationState, OperationsListProjection,
        };
        use time::OffsetDateTime;
        use uuid::Uuid;

        let op_id = Uuid::new_v4();
        let ops = OperationsListProjection {
            schema_version: WEB_SCHEMA_V1,
            active_count: 1,
            operations: vec![OperationRecord {
                id: op_id,
                kind: OperationKind::PackageInstall,
                state: OperationState::Running,
                label: "Installing ripgrep".to_string(),
                initiator: cybou_protocol::action::Proposer::Mind,
                subject: Some(cybou_protocol::SubjectRef::Package {
                    name: "ripgrep".to_string(),
                    installed_version: None,
                }),
                progress: OperationProgress {
                    percent: Some(45.0),
                    step: "Unpacking binaries".to_string(),
                    total_steps: Some(4),
                    current_step: Some(2),
                    detail: Some("4.2 MB / 8.5 MB".to_string()),
                },
                cancellable: true,
                started_at: OffsetDateTime::UNIX_EPOCH,
                updated_at: OffsetDateTime::UNIX_EPOCH,
                finished_at: None,
            }],
        };

        let encoded_ops = serde_json::to_string(&ops).expect("serialize ops");
        let decoded_ops: OperationsListProjection =
            serde_json::from_str(&encoded_ops).expect("deserialize ops");
        assert_eq!(decoded_ops, ops);

        let notif_id = Uuid::new_v4();
        let notifs = NotificationsListProjection {
            schema_version: WEB_SCHEMA_V1,
            unread_count: 1,
            attention_count: 1,
            notifications: vec![NotificationItem {
                id: notif_id,
                category: NotificationCategory::Attention,
                severity: NotificationSeverity::Warning,
                title: "Package Installation Proposal".to_string(),
                body: "Agent asked to install build-essential".to_string(),
                subject: None,
                created_at: OffsetDateTime::UNIX_EPOCH,
                read: false,
                dismissed: false,
                actions: vec![NotificationAction {
                    id: "approve".to_string(),
                    label: "Approve".to_string(),
                    kind: NotificationActionKind::ApproveProposal {
                        proposal_id: Uuid::nil(),
                    },
                    primary: true,
                }],
            }],
        };

        let encoded_notifs = serde_json::to_string(&notifs).expect("serialize notifs");
        let decoded_notifs: NotificationsListProjection =
            serde_json::from_str(&encoded_notifs).expect("deserialize notifs");
        assert_eq!(decoded_notifs, notifs);
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one round trip per contract, in one place"
    )]
    fn system_contracts_round_trip() {
        use super::{
            BackupArchiveRecord, BackupRepositoryRecord, BackupScheduleRecord,
            BackupSettingsProjection, BtrfsSubvolumeRecord, CpuCoreStat, DiskPartitionInfo,
            NetworkConnectionKind, NetworkConnectionRecord, NetworkInterfaceInfo,
            NetworkProjection, PackageRecord, PackageStatus, PackagesProjection, ProcessRecord,
            ProcessesListProjection, SecurityAuditEntry, SecurityPolicyRecord,
            SecuritySettingsProjection, ServiceRecord, ServiceState, ServiceUnitType,
            ServicesListProjection, SnapshotRecord, SshKeyRecord, StorageProjection,
            SystemLogEntry, SystemLogsProjection, SystemMonitorProjection, SystemSurfaceState,
            UserAccountRecord, UsersSettingsProjection,
        };

        let services = ServicesListProjection {
            schema_version: WEB_SCHEMA_V1,
            active_count: 1,
            failed_count: 0,
            services: vec![ServiceRecord {
                name: "cybou-web-gateway.service".to_string(),
                description: "CYBOU Web Gateway Daemon".to_string(),
                state: ServiceState::Active,
                substate: "running".to_string(),
                enabled: true,
                main_pid: Some(1024),
                memory_bytes: Some(38_000_000),
                unit_type: ServiceUnitType::Service,
            }],
        };
        let encoded_svc = serde_json::to_string(&services).expect("serialize services");
        let decoded_svc: ServicesListProjection =
            serde_json::from_str(&encoded_svc).expect("deserialize services");
        assert_eq!(decoded_svc, services);

        let procs = ProcessesListProjection {
            schema_version: WEB_SCHEMA_V1,
            total_count: 1,
            showing_count: 1,
            truncated: false,
            total_cpu_percent: 1.2,
            total_memory_bytes: 38_000_000,
            processes: vec![ProcessRecord {
                pid: 1024,
                ppid: 1,
                name: "cybou-web-gateway".to_string(),
                cmdline: "/usr/bin/cybou-web-gateway".to_string(),
                user: "cybou".to_string(),
                cpu_percent: 1.2,
                memory_bytes: 38_000_000,
                memory_percent: 0.5,
                state: "running".to_string(),
                threads: 4,
            }],
        };
        let encoded_procs = serde_json::to_string(&procs).expect("serialize procs");
        let decoded_procs: ProcessesListProjection =
            serde_json::from_str(&encoded_procs).expect("deserialize procs");
        assert_eq!(decoded_procs, procs);

        let logs = SystemLogsProjection {
            schema_version: WEB_SCHEMA_V1,
            logs: vec![SystemLogEntry {
                timestamp: "2026-08-28T22:00:00Z".to_string(),
                unit: Some("cybou-web-gateway.service".to_string()),
                severity: "info".to_string(),
                message: "started gateway".to_string(),
                pid: Some(1024),
            }],
            unavailable: None,
            system_journal_readable: true,
        };
        let encoded_logs = serde_json::to_string(&logs).expect("serialize logs");
        let decoded_logs: SystemLogsProjection =
            serde_json::from_str(&encoded_logs).expect("deserialize logs");
        assert_eq!(decoded_logs, logs);

        let monitor = SystemMonitorProjection {
            schema_version: WEB_SCHEMA_V1,
            hostname: "cybou-host".to_string(),
            os_release: "Linux 6.6-cybou".to_string(),
            uptime_seconds: 86400,
            load_avg: [0.15, 0.22, 0.18],
            total_cpu_percent: 12.5,
            cores: vec![CpuCoreStat {
                core_id: 0,
                usage_percent: 12.5,
            }],
            memory_total_bytes: 16_000_000_000,
            memory_used_bytes: 4_000_000_000,
            memory_free_bytes: 12_000_000_000,
            swap_total_bytes: 4_000_000_000,
            swap_used_bytes: 0,
            disk_partitions: vec![DiskPartitionInfo {
                mount_point: "/".to_string(),
                device: "/dev/nvme0n1p2".to_string(),
                fs_type: "btrfs".to_string(),
                total_bytes: 512_000_000_000,
                used_bytes: 42_000_000_000,
                available_bytes: 470_000_000_000,
            }],
            network_interfaces: vec![NetworkInterfaceInfo {
                name: "eth0".to_string(),
                rx_bytes: 10_000_000,
                tx_bytes: 5_000_000,
                is_up: true,
            }],
        };
        let encoded_mon = serde_json::to_string(&monitor).expect("serialize monitor");
        let decoded_mon: SystemMonitorProjection =
            serde_json::from_str(&encoded_mon).expect("deserialize monitor");
        assert_eq!(decoded_mon, monitor);

        let storage = StorageProjection {
            schema_version: WEB_SCHEMA_V1,
            state: SystemSurfaceState::Known,
            subvolumes: vec![BtrfsSubvolumeRecord {
                id: 256,
                path: "@home".to_string(),
                parent_uuid: None,
                is_snapshot: false,
                readonly: false,
            }],
            snapshots: vec![SnapshotRecord {
                id: "snap-01".to_string(),
                subvolume_path: "@home".to_string(),
                name: "pre-upgrade-backup".to_string(),
                timestamp: "2026-08-28T22:00:00Z".to_string(),
                size_bytes: 1_200_000_000,
                readonly: true,
            }],
            total_space_bytes: 1_000_000_000_000,
            free_space_bytes: 750_000_000_000,
        };
        let encoded_storage = serde_json::to_string(&storage).expect("serialize storage");
        let decoded_storage: StorageProjection =
            serde_json::from_str(&encoded_storage).expect("deserialize storage");
        assert_eq!(decoded_storage, storage);

        let network = NetworkProjection {
            schema_version: WEB_SCHEMA_V1,
            state: SystemSurfaceState::Known,
            connections: vec![NetworkConnectionRecord {
                id: "conn-eth0".to_string(),
                name: "eth0".to_string(),
                kind: NetworkConnectionKind::Ethernet,
                is_active: true,
                ip_address: Some("192.168.1.50/24".to_string()),
                gateway: Some("192.168.1.1".to_string()),
                dns: vec!["1.1.1.1".to_string()],
                rx_bytes: 50_000_000,
                tx_bytes: 25_000_000,
            }],
        };
        let encoded_net = serde_json::to_string(&network).expect("serialize network");
        let decoded_net: NetworkProjection =
            serde_json::from_str(&encoded_net).expect("deserialize network");
        assert_eq!(decoded_net, network);

        let packages = PackagesProjection {
            schema_version: WEB_SCHEMA_V1,
            state: SystemSurfaceState::Known,
            installed_count: 1,
            upgradable_count: 0,
            packages: vec![PackageRecord {
                name: "ripgrep".to_string(),
                installed_version: Some("14.1.0".to_string()),
                candidate_version: Some("14.1.0".to_string()),
                description: "fast search tool".to_string(),
                architecture: "x86_64".to_string(),
                repository: "cybou-main".to_string(),
                status: PackageStatus::Installed,
                download_size_bytes: Some(4_500_000),
            }],
        };
        let encoded_pkg = serde_json::to_string(&packages).expect("serialize pkg");
        let decoded_pkg: PackagesProjection =
            serde_json::from_str(&encoded_pkg).expect("deserialize pkg");
        assert_eq!(decoded_pkg, packages);

        let users = UsersSettingsProjection {
            schema_version: WEB_SCHEMA_V1,
            state: SystemSurfaceState::Known,
            users: vec![UserAccountRecord {
                uid: 1000,
                username: "cybou".to_string(),
                full_name: "CYBOU Operator".to_string(),
                home_dir: "/home/cybou".to_string(),
                shell: "/bin/bash".to_string(),
                groups: vec!["wheel".to_string(), "sudo".to_string()],
                is_admin: true,
                is_locked: false,
            }],
            ssh_keys: vec![SshKeyRecord {
                id: "ssh-key-01".to_string(),
                name: "Workstation ED25519".to_string(),
                fingerprint: "SHA256:abc123xyz...".to_string(),
                key_type: "ssh-ed25519".to_string(),
                public_key: "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI...".to_string(),
                created_at: "2026-08-28T22:00:00Z".to_string(),
            }],
        };
        let encoded_users = serde_json::to_string(&users).expect("serialize users");
        let decoded_users: UsersSettingsProjection =
            serde_json::from_str(&encoded_users).expect("deserialize users");
        assert_eq!(decoded_users, users);

        let security = SecuritySettingsProjection {
            schema_version: WEB_SCHEMA_V1,
            state: SystemSurfaceState::Known,
            policy: Some(SecurityPolicyRecord {
                landlock_enabled: true,
                bubblewrap_enabled: true,
                apparmor_enforcing: true,
                seccomp_strict: true,
                egress_firewall_strict: true,
            }),
            audit_log: vec![SecurityAuditEntry {
                timestamp: "2026-08-28T22:30:00Z".to_string(),
                severity: "info".to_string(),
                category: "sandbox".to_string(),
                message: "Landlock rules applied".to_string(),
            }],
        };
        let encoded_security = serde_json::to_string(&security).expect("serialize security");
        let decoded_security: SecuritySettingsProjection =
            serde_json::from_str(&encoded_security).expect("deserialize security");
        assert_eq!(decoded_security, security);

        let backup = BackupSettingsProjection {
            schema_version: WEB_SCHEMA_V1,
            state: SystemSurfaceState::Known,
            repository: Some(BackupRepositoryRecord {
                id: "repo-01".to_string(),
                name: "CYBOU Local Vault".to_string(),
                destination: "/var/backups/cybou.borg".to_string(),
                encryption: "repokey-blake2-chacha20-poly1305".to_string(),
                last_backup_time: Some("2026-08-28T20:00:00Z".to_string()),
                total_archives: 12,
                total_size_bytes: 45_000_000_000,
            }),
            archives: vec![BackupArchiveRecord {
                id: "arch-01".to_string(),
                name: "nightly-2026-08-28".to_string(),
                timestamp: "2026-08-28T20:00:00Z".to_string(),
                size_bytes: 3_200_000_000,
                duration_seconds: 42,
            }],
            schedule: Some(BackupScheduleRecord {
                enabled: true,
                frequency: "daily".to_string(),
                retention_daily: 7,
                retention_weekly: 4,
                retention_monthly: 12,
            }),
        };
        let encoded_backup = serde_json::to_string(&backup).expect("serialize backup");
        let decoded_backup: BackupSettingsProjection =
            serde_json::from_str(&encoded_backup).expect("deserialize backup");
        assert_eq!(decoded_backup, backup);
    }

    #[test]
    fn personal_pack_contracts_round_trip() {
        use super::{
            CalendarEventRecord, CalendarProjection, ContactRecord, ContactsProjection,
            MailAccountRecord, MailFolderKind, MailMessageRecord, MailProjection, NoteRecord,
            NotesProjection,
        };

        let mail = MailProjection {
            schema_version: WEB_SCHEMA_V1,
            accounts: vec![MailAccountRecord {
                id: "acc-01".to_string(),
                name: "Work IMAP".to_string(),
                email: "operator@cybou.local".to_string(),
                server: "mail.cybou.local".to_string(),
                unread_count: 3,
            }],
            messages: vec![MailMessageRecord {
                id: "msg-01".to_string(),
                account_id: "acc-01".to_string(),
                folder: MailFolderKind::Inbox,
                from: "security@cybou.local".to_string(),
                to: vec!["operator@cybou.local".to_string()],
                subject: "Weekly Landlock Policy Audit".to_string(),
                preview: "Audit completed with 0 violations".to_string(),
                body: "# Weekly Security Audit\nAll agent capsules executed under Landlock v3."
                    .to_string(),
                timestamp: "2026-08-28T22:00:00Z".to_string(),
                is_unread: true,
                is_starred: true,
                referenced_subject: None,
            }],
            active_account_id: "acc-01".to_string(),
            active_folder: MailFolderKind::Inbox,
        };
        let encoded_mail = serde_json::to_string(&mail).expect("serialize mail");
        let decoded_mail: MailProjection =
            serde_json::from_str(&encoded_mail).expect("deserialize mail");
        assert_eq!(decoded_mail, mail);

        let calendar = CalendarProjection {
            schema_version: WEB_SCHEMA_V1,
            events: vec![CalendarEventRecord {
                id: "evt-01".to_string(),
                title: "Autonomous Agent Review".to_string(),
                description: "Review OpenCode refactoring pull request".to_string(),
                start_time: "2026-08-29T10:00:00Z".to_string(),
                end_time: "2026-08-29T11:00:00Z".to_string(),
                is_all_day: false,
                location: Some("Studio A".to_string()),
                attendees: vec!["cybou-operator".to_string()],
                color_category: "indigo".to_string(),
                referenced_subject: None,
            }],
        };
        let encoded_cal = serde_json::to_string(&calendar).expect("serialize cal");
        let decoded_cal: CalendarProjection =
            serde_json::from_str(&encoded_cal).expect("deserialize cal");
        assert_eq!(decoded_cal, calendar);

        let notes = NotesProjection {
            schema_version: WEB_SCHEMA_V1,
            notes: vec![NoteRecord {
                id: "note-01".to_string(),
                title: "CYBOU Spatial Architecture".to_string(),
                content_markdown:
                    "# Living Canvas\nInfinite 2D spatial canvas with reactive cards.".to_string(),
                tags: vec!["architecture".to_string(), "design".to_string()],
                updated_at: "2026-08-28T23:00:00Z".to_string(),
                is_pinned: true,
                referenced_subject: None,
            }],
        };
        let encoded_notes = serde_json::to_string(&notes).expect("serialize notes");
        let decoded_notes: NotesProjection =
            serde_json::from_str(&encoded_notes).expect("deserialize notes");
        assert_eq!(decoded_notes, notes);

        let contacts = ContactsProjection {
            schema_version: WEB_SCHEMA_V1,
            contacts: vec![ContactRecord {
                id: "cnt-01".to_string(),
                name: "Dr. Elena Rostova".to_string(),
                email: "elena.rostova@cybou.net".to_string(),
                role: "Cognitive Systems Architect".to_string(),
                organization: "DeepMind / CYBOU Labs".to_string(),
                phone: Some("+1-555-0199".to_string()),
                tags: vec!["core-team".to_string(), "research".to_string()],
                notes: "Focusing on Action1 causality and continuous epistemic models".to_string(),
                referenced_subject: None,
            }],
        };
        let encoded_contacts = serde_json::to_string(&contacts).expect("serialize contacts");
        let decoded_contacts: ContactsProjection =
            serde_json::from_str(&encoded_contacts).expect("deserialize contacts");
        assert_eq!(decoded_contacts, contacts);
    }

    #[test]
    fn cognitive_graph_and_journal_contracts_round_trip() {
        use super::{
            CognitiveEdgeRecord, CognitiveEdgeType, CognitiveGraphProjection, CognitiveGraphRecord,
            CognitiveNodeRecord, CognitiveNodeType, EventJournalEntry, EventJournalProjection,
        };
        use cybou_protocol::epistemic::EpistemicStatus;
        use std::collections::HashMap;

        let graph = CognitiveGraphProjection {
            schema_version: WEB_SCHEMA_V1,
            graph: CognitiveGraphRecord {
                nodes: vec![
                    CognitiveNodeRecord {
                        id: "node:agent:opencode-main".to_string(),
                        label: "OpenCode Agent Capsule".to_string(),
                        node_type: CognitiveNodeType::Agent {
                            name: "opencode-main".to_string(),
                            model: "claude-3-5-sonnet".to_string(),
                            state: "active".to_string(),
                        },
                        epistemic_status: EpistemicStatus::Observed,
                        confidence: 0.98,
                        subject: None,
                        created_at: "2026-08-28T20:00:00Z".to_string(),
                        updated_at: "2026-08-28T23:00:00Z".to_string(),
                        metadata: HashMap::new(),
                    },
                    CognitiveNodeRecord {
                        id: "node:service:cybou-web-gateway".to_string(),
                        label: "CYBOU Web Gateway".to_string(),
                        node_type: CognitiveNodeType::Service {
                            name: "cybou-web-gateway.service".to_string(),
                            state: "running".to_string(),
                        },
                        epistemic_status: EpistemicStatus::Observed,
                        confidence: 1.0,
                        subject: None,
                        created_at: "2026-08-28T18:00:00Z".to_string(),
                        updated_at: "2026-08-28T23:00:00Z".to_string(),
                        metadata: HashMap::new(),
                    },
                ],
                edges: vec![CognitiveEdgeRecord {
                    id: "edge-01".to_string(),
                    source_id: "node:agent:opencode-main".to_string(),
                    target_id: "node:service:cybou-web-gateway".to_string(),
                    edge_type: CognitiveEdgeType::Observes,
                    weight: 0.95,
                    description: "Agent observes gateway REST endpoints and presence stream"
                        .to_string(),
                }],
            },
            focus_node_id: Some("node:agent:opencode-main".to_string()),
        };
        let encoded_graph = serde_json::to_string(&graph).expect("serialize graph");
        let decoded_graph: CognitiveGraphProjection =
            serde_json::from_str(&encoded_graph).expect("deserialize graph");
        assert_eq!(decoded_graph, graph);

        let journal = EventJournalProjection {
            schema_version: WEB_SCHEMA_V1,
            entries: vec![EventJournalEntry {
                event_id: "evt-jnl-001".to_string(),
                causation_id: None,
                correlation_id: "corr-001".to_string(),
                origin_organ: "actiond".to_string(),
                event_type: "ActionDispatched".to_string(),
                summary: "Applied Landlock sandbox confinement policy".to_string(),
                payload_preview: "{\"status\":\"enforced\"}".to_string(),
                timestamp: "2026-08-28T22:30:00Z".to_string(),
                subject: None,
                epistemic_status: EpistemicStatus::Observed,
            }],
            total_count: 1,
        };
        let encoded_jnl = serde_json::to_string(&journal).expect("serialize journal");
        let decoded_jnl: EventJournalProjection =
            serde_json::from_str(&encoded_jnl).expect("deserialize journal");
        assert_eq!(decoded_jnl, journal);
    }

    #[test]
    fn capsule_control_and_telemetry_contracts_round_trip() {
        use super::{
            CapsuleAction, CapsuleControlRequest, CapsuleTelemetryProjection,
            CapsuleTelemetryRecord, WEB_SCHEMA_V1,
        };
        use cybou_protocol::agent::{AgentMetric, Standing};
        use time::OffsetDateTime;
        use uuid::Uuid;

        let req = CapsuleControlRequest {
            action: CapsuleAction::Quarantine,
        };
        let encoded_req = serde_json::to_string(&req).expect("serialize req");
        let decoded_req: CapsuleControlRequest =
            serde_json::from_str(&encoded_req).expect("deserialize req");
        assert_eq!(decoded_req, req);

        let cap_id = Uuid::new_v4();
        let observed_at = OffsetDateTime::now_utc();
        let telemetry = CapsuleTelemetryProjection {
            schema_version: WEB_SCHEMA_V1,
            telemetry: CapsuleTelemetryRecord {
                capsule_id: cap_id,
                standing: Standing::Running,
                pids_count: AgentMetric::known(4, observed_at),
                pids_current: AgentMetric::known(4, observed_at),
                pids_max: AgentMetric::known(512, observed_at),
                memory_used_mib: AgentMetric::known(128, observed_at),
                memory_max_mib: AgentMetric::known(512, observed_at),
                cpu_usage_pct: AgentMetric::known(12.5, observed_at),
                cpu_usage_usec: AgentMetric::known(84_000, observed_at),
                egress_requests_count: AgentMetric::known(42, observed_at),
                egress_denied_count: AgentMetric::known(0, observed_at),
                files_modified_count: AgentMetric::known(7, observed_at),
                tokens_in: AgentMetric::known(1540, observed_at),
                tokens_out: AgentMetric::known(420, observed_at),
                active_tool: AgentMetric::known("edit_file".to_string(), observed_at),
                recent_activity: AgentMetric::known(
                    vec!["Opened workspace AST".to_string()],
                    observed_at,
                ),
            },
        };
        let encoded_tel = serde_json::to_string(&telemetry).expect("serialize telemetry");
        let decoded_tel: CapsuleTelemetryProjection =
            serde_json::from_str(&encoded_tel).expect("deserialize telemetry");
        assert_eq!(decoded_tel, telemetry);
    }

    #[test]
    fn meaning_contracts_round_trip() {
        use super::{
            CognitiveAct, CognitiveActKind, MeaningInterpretProjection, MeaningInterpretRequest,
            MeaningInterpretation, Qualification, ResponsePlan, WEB_SCHEMA_V1,
        };
        use time::OffsetDateTime;
        use uuid::Uuid;

        let req = MeaningInterpretRequest {
            utterance: "explain why cybou-web-gateway restarted".into(),
            language: Some("en".into()),
        };
        let enc_req = serde_json::to_string(&req).expect("serialize req");
        let dec_req: MeaningInterpretRequest =
            serde_json::from_str(&enc_req).expect("deserialize req");
        assert_eq!(dec_req, req);

        let proj = MeaningInterpretProjection {
            schema_version: WEB_SCHEMA_V1,
            interpretation: MeaningInterpretation {
                utterance: "explain why cybou-web-gateway restarted".into(),
                primary_act: CognitiveAct {
                    act_id: Uuid::new_v4(),
                    kind: CognitiveActKind::Explain,
                    subject: "cybou-web-gateway".into(),
                    parameters: vec![("reason".into(), "restart".into())],
                    source: "person".into(),
                    evidence: Vec::new(),
                },
                references: Vec::new(),
                confidence: 0.95,
                ambiguous: false,
                derived_at: OffsetDateTime::now_utc(),
            },
            response_plan: Some(ResponsePlan {
                plan_id: Uuid::new_v4(),
                intent: "explain_restart".into(),
                key_points: vec![
                    "Service cybou-web-gateway restarted due to SIGHUP configuration reload".into(),
                ],
                referenced_evidence: Vec::new(),
                qualifications: vec![Qualification::Unverified],
            }),
            realization: Some(
                "The service cybou-web-gateway restarted due to SIGHUP reload.".into(),
            ),
        };

        let enc_proj = serde_json::to_string(&proj).expect("serialize proj");
        let dec_proj: MeaningInterpretProjection =
            serde_json::from_str(&enc_proj).expect("deserialize proj");
        assert_eq!(dec_proj, proj);
    }

    #[test]
    fn learning_and_governance_contracts_round_trip() {
        use super::{
            ActorKind, ArtifactStatus, CandidateEvaluationProjection, GovernanceScopesProjection,
            LearnedArtifactLineage, LearnedArtifactsProjection, LearningCandidate,
            LearningCandidatesProjection, LearningLayer, Promoted, ProposeLearningCandidateRequest,
            TaskScope, WEB_SCHEMA_V1,
        };
        use time::OffsetDateTime;
        use uuid::Uuid;

        let now = OffsetDateTime::now_utc();
        let cand_id = Uuid::new_v4();
        let ev1 = Uuid::new_v4();
        let ev2 = Uuid::new_v4();

        let propose = ProposeLearningCandidateRequest {
            layer: LearningLayer::Procedural,
            generalization: "restart nginx on connection refused".into(),
            scope: "service.nginx".into(),
            source_evidence: vec![ev1],
            outcome_evidence: vec![ev2],
        };
        let enc_prop = serde_json::to_string(&propose).expect("serialize propose");
        let dec_prop: ProposeLearningCandidateRequest =
            serde_json::from_str(&enc_prop).expect("deserialize propose");
        assert_eq!(dec_prop, propose);

        let candidates = LearningCandidatesProjection {
            schema_version: WEB_SCHEMA_V1,
            candidates: vec![LearningCandidate {
                candidate_id: cand_id,
                layer: LearningLayer::Procedural,
                source_evidence: vec![ev1],
                outcome_evidence: vec![ev2],
                generalization: "restart nginx on connection refused".into(),
                scope: "service.nginx".into(),
                derivation_version: 1,
                created_at: now,
            }],
            total_count: 1,
        };
        let enc_cand = serde_json::to_string(&candidates).expect("serialize candidates");
        let dec_cand: LearningCandidatesProjection =
            serde_json::from_str(&enc_cand).expect("deserialize candidates");
        assert_eq!(dec_cand, candidates);

        let artifact_id = Uuid::new_v4();
        let artifacts = LearnedArtifactsProjection {
            schema_version: WEB_SCHEMA_V1,
            artifacts: vec![LearnedArtifactLineage {
                artifact_id,
                layer: LearningLayer::Procedural,
                status: ArtifactStatus::Promoted,
                contributing_candidates: vec![cand_id],
                source_evidence: vec![ev1, ev2],
                promoted_at: Some(now),
                erasure_epoch: 1,
            }],
            total_count: 1,
        };
        let enc_art = serde_json::to_string(&artifacts).expect("serialize artifacts");
        let dec_art: LearnedArtifactsProjection =
            serde_json::from_str(&enc_art).expect("deserialize artifacts");
        assert_eq!(dec_art, artifacts);

        let eval_proj = CandidateEvaluationProjection {
            schema_version: WEB_SCHEMA_V1,
            candidate_id: cand_id,
            promoted: Some(Promoted {
                candidate_id: cand_id,
                layer: LearningLayer::Procedural,
                independent_episodes: 3,
                success_rate: 1.0,
            }),
            refused: None,
            artifact: Some(LearnedArtifactLineage {
                artifact_id,
                layer: LearningLayer::Procedural,
                status: ArtifactStatus::Promoted,
                contributing_candidates: vec![cand_id],
                source_evidence: vec![ev1, ev2],
                promoted_at: Some(now),
                erasure_epoch: 1,
            }),
        };
        let enc_eval = serde_json::to_string(&eval_proj).expect("serialize eval_proj");
        let dec_eval: CandidateEvaluationProjection =
            serde_json::from_str(&enc_eval).expect("deserialize eval_proj");
        assert_eq!(dec_eval, eval_proj);

        let scopes = GovernanceScopesProjection {
            schema_version: WEB_SCHEMA_V1,
            scopes: vec![TaskScope {
                actor_id: Uuid::new_v4(),
                kind: ActorKind::Worker,
                intention_id: Some(Uuid::new_v4()),
                capabilities: vec!["fs.read".into()],
                tool_grants: vec!["git.status".into()],
                network_destinations: vec!["api.github.com".into()],
                ttl_seconds: 3600,
                max_compute_ms: 60000,
                delegation_permitted: false,
                granted_at: now,
            }],
        };
        let enc_scopes = serde_json::to_string(&scopes).expect("serialize scopes");
        let dec_scopes: GovernanceScopesProjection =
            serde_json::from_str(&enc_scopes).expect("deserialize scopes");
        assert_eq!(dec_scopes, scopes);
    }
}
