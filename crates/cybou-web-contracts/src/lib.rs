// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Explicit versioned contract between Living Canvas and `cybou-web-gateway`.

use cybou_protocol::{CapabilityState, KnowledgeState, SchemaVersion};
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

/// What a file held when it was read.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContentProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// The file that was read.
    pub path: String,
    /// Its text.
    pub text: String,
    /// How large the file is on disk.
    pub size_bytes: u64,
}

/// Request to end one of the caller's shells.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellCloseRequest {
    /// Which of the caller's shells to end.
    pub instance: u32,
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

#[cfg(test)]
mod tests {
    use super::{
        CommitmentsProjection, MindProjection, SessionMode, SessionProjection, SnapshotProjection,
        WEB_SCHEMA_V1,
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
}
