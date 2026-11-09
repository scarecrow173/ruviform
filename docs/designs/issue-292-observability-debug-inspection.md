---
id: "DESIGN-koNZAEijupozhje6vYRpDO"
type: "design"
status: "approved"
title: "Issue #292 OBS-10 Debug Inspection・Invariant・Session・Probe連携 — observability 内完結 slice"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/292"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-292-observability-debug-inspection.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-277-observability-core-source-contract.md"
    - "docs/designs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
    - "docs/designs/issue-291-observability-diagnostics.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/designs/issue-291-observability-diagnostics.md"
    - "docs/designs/issue-309-probe-time-context-and-alignment.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/obs-05-observability-leaf-primitive-deps.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-292-observability-debug-inspection.md"
    - "docs/designs/issue-291-observability-diagnostics.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-292-observability-debug-inspection-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/293"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/308"
    - "https://github.com/scarecrow173/ruviform/issues/309"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/293"
    - "https://github.com/scarecrow173/ruviform/issues/281"
  deferred-by: []
---

# Issue #292 OBS-10 Debug Inspection・Invariant・Session・Probe連携 — observability 内完結 slice

## Context

`ruviform-observability` は OBS-02（#277）source 契約、OBS-05（#280）
`record`/`rt_bridge`、OBS-07（#289）Collector、OBS-09（#291）
Diagnostics を実装済み。Issue #292（OBS-10）はその上に **追加コスト
のある debug 層**を置く: 有界 Inspection、不変条件の 3 値判定、明示的
Debug Session、開示認可付き Debug Report。#185 Generic Probe
（`ruviform-audio::typed::probe`）・#308 Generation-safe Rebind・
#309 Probe time alignment は別クレートに実装済みだが、
`ruviform-audio → ruviform-observability` の依存が既に存在するため
本 crate から audio 型を参照すると循環になる。したがって probe
参照は **port trait（`Box<dyn>` 注入）**に限定し、concrete adapter
は #293 / `ruviform-audio` 側の責務とする。

設計上の中心問題は 5 つ: (1) `InspectionBudget`/`InspectionRequest`/
`InspectionSnapshot` の有界・typed・非 transactional 契約、(2)
`InvariantOutcome` の Satisfied/Violated/Inconclusive を観測欠落・
世代不一致・未同期読取で偽装しない判定と OBS-09 への正直な接続、
(3) `Disabled→Prepared→Active→Draining→Stopped` の Session 状態機械と
capability 権限、(4) `SessionDrain` port + `EventRecordDrain` による
RT buffer 最終回収（quiescence 未確認で Buffer/Arc を解放しない）、
(5) Secrets/Media 非含有・未認可 External Export 拒否の
`DebugReport`。

## Scope

### In Scope

- `src/debug/` モジュール群: `mod.rs`（module doc + re-export）、
  `budget.rs`、`request.rs`、`snapshot.rs`、`invariant.rs`、
  `probe.rs`、`session.rs`、`report.rs`、`engine.rs`（`DebugInspector`
  集約器）、`tests.rs`（crate 内 unit test）。
- lib.rs への `pub mod debug` 登録と crate doc（モジュール一覧・RT
  契約表）更新。`Cargo.toml` は変更なし（新規依存・feature 追加なし）。

### Out of Scope

- concrete probe adapter・`debug-tools` feature・Host 認可機構・
  #185 Probe 本体・両 Consumer 検証・DSP 同値実機検証・
  Diagnostics resolve API（spec の Out of Scope と同一）。

## Components and Boundaries

### budget.rs — InspectionBudget

```rust
/// Non-RT inspection bounds. `0`/`None` semantics are per-field and
/// documented — same convention as diagnostics `ReportLimits`.
pub struct InspectionBudget {
    /// Max items collected per section; `0` disables item collection
    /// for every section (aggregate counts still reported).
    pub max_items: usize,
    /// Approximate byte budget across all sections (`size_of` +
    /// payload-length estimation, documented as approximate);
    /// `0` = unlimited.
    pub max_bytes: usize,
    /// Whole-inspect deadline checked between sections; `None` =
    /// unlimited. On expiry remaining sections report
    /// `SectionStatus::Unavailable { reason: DeadlineExceeded }`.
    pub timeout: Option<Duration>,
}
```

- `Default`: `max_items: 64`, `max_bytes: 65_536`, `timeout: None`
  （diagnostics `ReportLimits` の既定値と揃える）。
- 検証は `validate()` を設けず全値を受理する — `0`/`None` は
  「無制限/無効」の文書化済み意味であり、invalid 組合せは存在しない
  （`SinkPolicy::window == 0` のような曖昧ケースが無い）。

### request.rs — InspectionRequest / targets / detail

```rust
/// What to inspect — closed vocabulary bounding section kinds.
#[non_exhaustive]
pub enum InspectionTarget {
    /// Collector `MetricsSnapshot` measurements (per-series entries).
    Metrics,
    /// OBS-09 `DiagnosticReport` findings/health summary.
    Diagnostics,
    /// Registered `InvariantCheck`s evaluated this inspect.
    Invariants,
    /// Debug session state + drain accounting.
    Session,
    /// `ProbeSource` deck readings + generation/clock alignment.
    Probes,
}

/// How much detail entries carry.
#[non_exhaustive]
pub enum InspectionDetail {
    /// Aggregate counts only — per-item `entries` stay empty.
    Summary,
    /// Condensed per-item entries (identity/kind/validity/status;
    /// no payload values).
    Standard,
    /// Entries additionally carry typed payloads
    /// (`MetricValue`, probe status detail).
    Verbose,
}

pub struct InspectionRequest {
    /// Sections to produce — deduplicated and evaluated in enum
    /// declaration order regardless of request order (deterministic
    /// output). Empty = no sections.
    pub targets: Vec<InspectionTarget>,
    pub detail: InspectionDetail,
    /// Max `Confidentiality` the caller may see — items above this
    /// are withheld wholesale (same rule as diagnostics reports).
    pub disclosure: Confidentiality,
    pub budget: InspectionBudget,
}
```

- `targets` の重複は inspect 内で除去・宣言順ソートして決定的にする。
- `disclosure > config.max_inspection_disclosure` は
  `InspectionError::Unauthorized`（report と同じ capability モデル）。

### snapshot.rs — InspectionSnapshot / sections / consistency

```rust
pub const INSPECTION_SCHEMA_VERSION: u16 = 1;

/// Per-section delivery status — truncation/unavailability are typed,
/// never silent.
#[non_exhaustive]
pub enum SectionStatus {
    /// Section produced normally (possibly with truncated entries).
    Ok,
    /// Budget cut item collection short — see `InspectionTruncation`.
    Truncated,
    /// The section could not be produced — input absent, deadline
    /// exceeded, or probe source failure.
    Unavailable { reason: UnavailableReason },
    /// Withheld entirely — every item exceeded `request.disclosure`.
    Withheld,
}

#[non_exhaustive]
pub enum UnavailableReason {
    /// The caller did not supply the required input
    /// (e.g. `InspectionInputs::metrics` was `None`).
    InputNotProvided,
    /// `budget.timeout` elapsed before this section ran.
    DeadlineExceeded,
    /// A registered port (probe source / invariant check) failed or
    /// panicked — counted in `DebugHealth`, never fabricated.
    PortFailure,
}

/// Typed section payloads — closed vocabulary, no arbitrary state.
#[non_exhaustive]
pub enum InspectionSection {
    Metrics(MetricsInspection),
    Diagnostics(DiagnosticsInspection),
    Invariants(InvariantsInspection),
    Session(SessionInspection),
    Probes(ProbesInspection),
}
```

各 payload の形:

```rust
pub struct MetricsInspection {
    pub status: SectionStatus,
    pub generation: u64,               // MetricsSnapshot::generation
    pub taken_at: Instant,
    pub total_series: usize,
    /// validity 別集計 — Discontinuous/Stale/Partial/Unavailable を
    /// 潰さず計数する。
    pub validity_counts: ValidityCounts,
    pub entries: Vec<MetricEntry>,     // bounded by budget.max_items
}

pub struct MetricEntry {              // Standard 以上で収集
    pub identity: SourceIdentity,
    pub key: &'static str,
    pub kind: MetricKind,
    pub scope: ObservationScope,
    pub lifecycle: SourceLifecycle,
    pub validity: MeasurementValidity,
    pub issue: Option<MeasurementIssue>,
    pub observed_at: Instant,
    pub last_change_at: Instant,
    /// Verbose のみ Some — Standard/Summary では None。
    pub value: Option<MetricValue>,
}
```

```rust
pub struct DiagnosticsInspection {
    pub status: SectionStatus,
    pub source_generation: u64,        // DiagnosticReport::source_generation
    pub disclosure: Confidentiality,
    pub findings_total: usize,
    pub open_findings: usize,
    pub health: Vec<DomainHealth>,     // report の health をそのまま保持
    pub entries: Vec<FindingEntry>,    // bounded — code/severity/
                                       // certainty/status/domain のみ
}
```

```rust
pub struct InvariantsInspection {
    pub status: SectionStatus,
    pub checks_run: usize,
    pub summary: InvariantSummary,     // satisfied/violated/inconclusive
    pub results: Vec<InvariantResult>, // bounded
}
```

```rust
pub struct SessionInspection {
    pub status: SectionStatus,
    pub state: DebugSessionState,
    pub generation: u64,               // session instance generation
    pub label: Option<&'static str>,
    pub drains_attached: usize,
    pub drain: DrainAccounting,        // delivered/dropped/lag/pending
}
```

```rust
pub struct ProbesInspection {
    pub status: SectionStatus,
    pub decks: Vec<ProbeDeckView>,     // bounded by budget.max_items
}
pub struct ProbeDeckView {
    pub deck_generation: u64,
    pub clock: ProbeClockMark,
    pub aligned: bool,                 // 全 reading が deck gen に一致
    pub mismatched: usize,
    pub probes: Vec<ProbeEntry>,       // bounded
}
```

```rust
/// Generation alignment across inputs — worst-of-domain reporting.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConsistencyLevel {
    /// Every assessed domain agreed internally.
    Aligned,
    /// At least one domain spans multiple generations — declared,
    /// never hidden.
    Skewed,
    /// At least one domain's alignment cannot be established.
    Unknown,
}

pub struct InspectionConsistency {
    /// Worst level across assessed domains (Unknown > Skewed >
    /// Aligned in conservatism).
    pub level: ConsistencyLevel,
    /// Collector-domain generations seen (None fields = input absent).
    pub collector: CollectorGenerations,
    /// Probe decks assessed — empty when Probes target absent/failed.
    pub probe_decks: Vec<ProbeDeckAlignment>,
}

pub struct CollectorGenerations {
    pub snapshot: Option<u64>,
    pub batch: Option<u64>,
    pub diagnostics: Option<u64>,
    /// true iff every present pair within the collector generation
    /// domain is equal. Domains are never cross-compared numerically
    /// (collector poll gen vs plan gen are different domains).
    pub aligned: bool,
}

pub struct ProbeDeckAlignment {
    /// Opaque deck identity reported by the adapter (index order is
    /// NOT identity — mismatch detection uses generation+clock only).
    pub generation: u64,
    pub clock_known: bool,
    pub readings: usize,
    pub mismatched: usize,
}
```

```rust
pub struct InspectionTruncation {
    /// Items dropped by `budget.max_items`/`max_bytes`/`deadline`.
    pub items_dropped: usize,
    /// Items withheld because `confidentiality > request.disclosure`.
    pub withheld_confidential: usize,
    /// Approximate bytes accounted for included items.
    pub bytes_estimated: usize,
    /// `true` when `budget.timeout` stopped collection early.
    pub deadline_exceeded: bool,
    /// Sections reported `Unavailable` — count for quick scanning.
    pub sections_unavailable: usize,
}

pub struct InspectionSnapshot {
    pub schema_version: u16,           // INSPECTION_SCHEMA_VERSION
    pub taken_at: Instant,
    pub request_summary: RequestEcho,  // targets/detail/disclosure/budget
    pub consistency: InspectionConsistency,
    pub sections: Vec<InspectionSection>, // request order → decl order
    pub truncation: InspectionTruncation,
}
```

- `RequestEcho` は `targets: Vec<InspectionTarget>`（重複除去済み）・
  `detail`・`disclosure`・`budget` を保持し、snapshot が「何を・どの
  予算で」答えたかを自己記述する。
- `consistency` は inspect が収集した生の generation stamp を保持し、
  判定ロジックは section 生成と独立に計算する（section が
  `Unavailable` でも他入力の整合は報告する）。

### invariant.rs — InvariantOutcome / check port

```rust
/// Three-valued invariant verdict — Inconclusive is never collapsed
/// into Satisfied or Violated.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvariantOutcome {
    /// The declared invariant was verified against observations.
    Satisfied,
    /// The declared invariant was observed violated.
    Violated,
    /// The invariant could not be evaluated — missing observation,
    /// generation mismatch, unsynchronized read, or check decline.
    Inconclusive,
}

/// Why an invariant could not be decided — typed, never a guessed
/// cause.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InconclusiveReason {
    /// Required observation absent (no measurement/probe reading).
    MissingObservation,
    /// Generation/source-identity mismatch — e.g. probe index or
    /// source identity differs from the inspected generation.
    GenerationMismatch,
    /// Inputs were not read atomically; a concurrent mutation may
    /// invalidate the verdict.
    UnsynchronizedRead,
    /// The check itself reported it cannot evaluate.
    CheckDeclined,
    /// The check panicked — isolated via `catch_unwind`, counted in
    /// `DebugHealth::invariant_panics_total`.
    CheckPanicked,
}

/// What a check returns — outcome plus optional static detail and
/// evidence basis for diagnostics publishing.
pub struct InvariantVerdict {
    pub outcome: InvariantOutcome,
    /// Domain-declared attention level for Violated (ignored for
    /// other outcomes). Default `Warning` when the check does not
    /// declare — the verdict builder requires it explicitly for
    /// Violated so severity is never silently defaulted.
    pub severity: Option<FindingSeverity>,
    /// Human summary — `String`, non-RT construction only. No
    /// dedicated fields for paths/bytes/tokens exist.
    pub summary: String,
    /// EvidenceRefs the verdict rests on — non-empty makes the
    /// published evidence `Derived`; empty publishes `Reported`.
    pub basis: Box<[EvidenceRef]>,
    /// Typed reason when `outcome == Inconclusive`.
    pub inconclusive: Option<InconclusiveReason>,
}
```

- `InvariantVerdict` は private field + コンストラクタ
  （`satisfied()`/`violated(severity, summary)`/`inconclusive(reason,
  summary)`）で不変条件を強制する: `Violated` は `severity: Some`、
  `Inconclusive` は `reason: Some` を要求し、他の組合せは
  `InvariantError` で拒否する。`basis` は任意（Derived/Reported の
  選択に使う）。

```rust
/// Non-RT invariant check port — implemented by domain owners and
/// injected via `Box<dyn>`. Called once per inspect that includes
/// `InspectionTarget::Invariants`, in registration order.
pub trait InvariantCheck: Send {
    /// Stable domain-owned code — reuse `DiagnosticCode`
    /// (name + schema_version), never a central enum.
    fn code(&self) -> DiagnosticCode;
    /// Evaluates the invariant against `ctx`. Must be non-RT,
    /// bounded, and must not block; panic is isolated by the caller.
    fn evaluate(&mut self, ctx: &InspectionContext<'_>) -> InvariantVerdict;
}
```

```rust
/// Read-only view handed to checks — assembled per inspect before
/// any check runs (probe readings are collected first so checks can
/// cite them).
pub struct InspectionContext<'a> {
    pub now: Instant,
    pub metrics: Option<&'a MetricsSnapshot>,
    pub batch: Option<&'a ObservationBatch>,
    pub diagnostics: Option<&'a DiagnosticReport>,
    pub probes: &'a [ProbeDeckSnapshot],
    pub session: &'a SessionView,
}
```

```rust
pub struct InvariantResult {
    pub code: DiagnosticCode,
    pub outcome: InvariantOutcome,
    pub inconclusive: Option<InconclusiveReason>,
    pub severity: Option<FindingSeverity>,
    pub summary: String,
    pub basis: Box<[EvidenceRef]>,
    pub evaluated_at: Instant,
}
```

### probe.rs — ProbeSource port（#185 語彙の observability 側写像）

```rust
/// Mirror of `ruviform_audio::typed::probe::ProbeMode` — kept local
/// because observability cannot depend on ruviform-audio.
#[non_exhaustive]
pub enum ProbeMode { Off, Monitor, Capture }

/// Mirror of `PointAvailability` (Configured/Inferred/Unavailable/
/// Unsupported/Unknown).
#[non_exhaustive]
pub enum ProbeAvailability { /* same 5 variants */ }

/// Mirror of `StatusKind` — the non-data lifecycle/loss events a
/// probe reports. Variant fields mirror the audio-side payload.
#[non_exhaustive]
pub enum ProbeStatusKind { /* Installed/ModeChanged{to}/
    CaptureStarted{budget_frames}/CaptureStopped{reason}/Unbound/
    Unobserved{reason}/LostBlocks/LostRecords/Removed/RebindDropped */
}
```

```rust
/// Clock identity of a probe cursor series — reuses the shared
/// media-core vocabulary (`ClockDomainId` + `ClockEpochId`).
/// `domain == ClockDomainId::UNKNOWN` mirrors `ProbeClockMark::
/// UNKNOWN` fail-closed semantics: UNKNOWN never compares equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProbeClockMark {
    pub domain: ClockDomainId,
    pub epoch: ClockEpochId,
}
```

```rust
/// One probe's non-RT state as of the port call — Copy, no-alloc,
/// carries generation + clock mark + bounded counters. Point
/// identity stays opaque (`node`/`port` as u64 pair) because
/// `NodeId`/`PortId` are domain types observability cannot name.
#[derive(Clone, Copy, Debug)]
pub struct ProbeReading {
    /// Domain probe id (opaque u64 — `ProbeId` equivalent).
    pub probe: u64,
    /// Observed point — opaque `(node, port)` ids.
    pub node: u64,
    pub port: u64,
    pub mode: ProbeMode,
    pub availability: ProbeAvailability,
    /// Plan generation the binding resolved against.
    pub generation: u64,
    /// Cursor-series clock identity at read time.
    pub clock: ProbeClockMark,
    /// Latest non-data status event, if any.
    pub last_status: Option<ProbeStatusKind>,
    /// Cumulative telemetry records accepted.
    pub recorded: u64,
    /// Cumulative telemetry records dropped (channel full).
    pub dropped: u64,
    /// Cumulative PCM blocks lost (pool exhausted).
    pub lost_blocks: u64,
}

/// One probe deck (broker pair) snapshot.
#[derive(Clone, Debug)]
pub struct ProbeDeckSnapshot {
    /// Plan generation the deck currently binds against.
    pub generation: u64,
    /// Deck-level clock mark (all probes on one cursor share it).
    pub clock: ProbeClockMark,
    /// Per-probe readings — bounded by the adapter; the inspector
    /// applies `budget.max_items` on top.
    pub probes: Vec<ProbeReading>,
}

/// Non-RT probe port — implemented by a domain adapter (e.g. an
/// `ruviform-audio` adapter mapping `ProbeDeck` state; that adapter
/// is #293's responsibility). Called once per inspect that includes
/// `InspectionTarget::Probes`.
pub trait ProbeSource: Send {
    /// Returns the current deck snapshots. Must be non-RT and
    /// bounded; failure is reported by returning `Err` or panicking
    /// (isolated → `SectionStatus::Unavailable{PortFailure}`).
    fn decks(&mut self) -> Vec<ProbeDeckSnapshot>;
}
```

- `ProbeReading`/`ProbeDeckSnapshot` は inspect 内で `Copy`/clone
  される値型。`ProbeStatusKind` は `Copy`（payload は u64/bool/enum
  のみ — String を持たない）。
- generation 整合: `reading.generation != deck.generation` →
  `ProbeDeckView.mismatched += 1`・`aligned = false` →
  `ConsistencyLevel::Skewed`（clock `UNKNOWN` → `Unknown`）。
  probe index/source identity の不一致は check 側が
  `InconclusiveReason::GenerationMismatch` で表現する。

### session.rs — DebugSession 状態機械 + drain port

```rust
/// Session lifecycle — one-directional except Stopped→Prepared
/// (re-prepare = new session generation).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DebugSessionState {
    /// No session — the default. Inspection of existing observability
    /// data still works; only session-gated capture is absent.
    Disabled,
    /// Permit consumed, RT targets pre-registered, not yet active.
    Prepared,
    /// Debug capture may run; drains accept records.
    Active,
    /// Stop requested — no new work; collecting final RT records.
    Draining,
    /// Terminal for this generation — buffers released after
    /// confirmed quiescence (or explicit abandon).
    Stopped,
}
```

```rust
/// Opaque capability — minted only by `DebugAuthority::permit()`.
/// `prepare`/`request_stop` consume it by value.
#[derive(Debug)]
pub struct SessionPermit { /* private */ }

/// Host-side authority marker — constructing one is the explicit
/// act of assuming session-control responsibility (real ACL/audit
/// is OBS-11 #293).
#[derive(Debug, Default)]
pub struct DebugAuthority { /* private — may carry issuance counters */ }

impl DebugAuthority {
    /// Issues a permit. Returns `None` when the authority's issuance
    /// policy denies (e.g. revoked) — modeled as Option, not error.
    pub fn permit(&self) -> Option<SessionPermit>;
}
```

```rust
pub struct SessionRequest {
    /// Domain-owned static label (e.g. `"synvelis.signal-flow"`).
    pub label: &'static str,
    /// Bound on records retained from drains — overflow is counted,
    /// never silently dropped. `0` = collect nothing (drains still
    /// accounted).
    pub max_records: usize,
    /// Bound on a single `poll_drain` call's delivered records —
    /// protects non-RT callers from unbounded work.
    pub drain_batch: usize,
}

#[non_exhaustive]
pub enum SessionError {
    /// No `DebugAuthority` was installed on the inspector.
    NoAuthority,
    /// `DebugAuthority::permit()` returned `None`.
    PermitDenied,
    /// Transition not allowed from the current state
    /// (carries expected/actual like `LifecycleError`).
    InvalidTransition { expected: DebugSessionState, actual: DebugSessionState },
    /// A non-Stopped session already exists — prepare is rejected
    /// rather than queued (the 切替 contract: wait for Stopped).
    SessionBusy { state: DebugSessionState },
    /// `activate` requires at least one attached drain.
    NoDrains,
    /// `drain_batch == 0` or `max_records == 0` where required.
    InvalidRequest,
}
```

```rust
/// One drained debug record — `EventRecord` (fixed-size, cannot
/// carry media/secrets) + positional loss.
#[derive(Clone, Copy, Debug)]
pub struct SessionRecord {
    pub record: EventRecord,
    pub dropped_before: u64,
}

/// Per-call drain accounting.
#[derive(Clone, Copy, Debug, Default)]
pub struct DrainReport {
    /// Records delivered to the session this call.
    pub delivered: usize,
    /// Cumulative dropped on this endpoint (bridge counter).
    pub dropped_total: u64,
    /// Records still in flight after this call (recorded - drained).
    pub lag: u64,
}

/// Non-RT drain endpoint — implemented by domain adapters and by
/// the provided `EventRecordDrain`. The session retains endpoints
/// until quiescence is confirmed; it never drops a sink while the
/// writer might still produce.
pub trait SessionDrain: Send {
    /// `true` once the RT writer has left the RT path — the
    /// precondition for final drain (mirrors the `rt_bridge`
    /// writer-quiescence contract).
    fn writer_quiesced(&self) -> bool;
    /// Bounded final drain; delivers each pending record to `emit`.
    /// Called only after `writer_quiesced()` returns `true`.
    fn drain_final(&mut self, emit: &mut dyn FnMut(SessionRecord)) -> DrainReport;
}
```

```rust
/// Concrete `SessionDrain` over `rt_bridge::RecordSink<EventRecord>`
/// — the in-crate adapter proving reuse of the OBS-05 bridge rather
/// than a second record path. Quiescence is reported through a
/// shared `Arc<AtomicBool>` the domain sets when its recorder leaves
/// the RT path (same shared-cell precedent as `SourceState`).
pub struct EventRecordDrain {
    sink: RecordSink<EventRecord>,
    quiesced: Arc<AtomicBool>,
}

impl EventRecordDrain {
    /// `quiesced` is shared with the domain: the domain stores `true`
    /// (Relaxed) once the `RtRecorder` has been moved out of / dropped
    /// from the RT path.
    pub fn new(sink: RecordSink<EventRecord>, quiesced: Arc<AtomicBool>) -> Self;
    /// The shared flag — for tests and adapter composition.
    pub fn quiesced_flag(&self) -> Arc<AtomicBool>;
}
```

Session 内部状態（`engine.rs` 内の private struct）:

```rust
struct SessionSlot {
    state: DebugSessionState,
    generation: u64,             // increments per prepare
    label: Option<&'static str>,
    drains: Vec<Box<dyn SessionDrain>>,
    records: VecDeque<SessionRecord>,  // bounded by max_records
    drain_overflow: u64,               // records beyond cap
    prepared_at/activated_at/drain_started_at/stopped_at: Option<Instant>,
    abandoned: bool,
    drain_pending_quiescence: usize,   // last poll_drain snapshot
    drain_pending_records: u64,
}
```

遷移規則（`DebugInspector` の `&mut self` メソッド）:

- `prepare_session(permit, req) -> Result<(), SessionError>`:
  state ∈ {Disabled, Stopped} 必須（それ以外 `SessionBusy`）。
  `generation += 1`、`records`/`drains` を初期化、label/max_records/
  drain_batch を保持 → `Prepared`。
- `attach_drain(&mut self, drain: Box<dyn SessionDrain>)`:
  state ∈ {Prepared, Active} のみ。`Draining`/`Stopped` は
  `InvalidTransition`。
- `activate_session() -> Result<(), SessionError>`:
  `Prepared` → `Active`。`drains.is_empty()` → `NoDrains`。
- `request_stop(permit) -> Result<(), SessionError>`:
  `Prepared | Active` → `Draining`（`drain_started_at = now`）。
  `Draining`/`Stopped`/`Disabled` は `InvalidTransition`。
  stop も permit を消費する（開始/停止とも権限ある Host のみ）。
- `poll_drain(&mut self, now) -> SessionDrainStatus`:
  `Draining` のみ有効。他 state は `InvalidTransition` 相当を
  `SessionDrainStatus::NotDraining{state}` で返す（poll は
  「まだdrain中か」の問合せなので error ではなく status が誠実）。
  各 endpoint について `writer_quiesced()` が false なら pending
  quiescence に計上し `drain_final` を呼ばない（契約上
  quiescence 前の final drain は禁止）。true なら `drain_final`
  を `drain_batch` 上限の emit で呼び、`SessionRecord` を
  `max_records` cap の `VecDeque` へ（超過は `drain_overflow`）。
  全 endpoint が quiesced かつ `lag == 0` なら `Stopped` —
  この時点で初めて `drains` を drop して ring/Arc を解放する
  （non-RT、quiescence 確認後のみ）。`drain_timeout`（config、
  `Option<Duration>`）超過でも **強制 Stopped しない** —
  `SessionDrainStatus::Draining{pending_quiescence,
  pending_records, deadline_exceeded: true}` を返し続け、
  Buffer を保持したまま「完了していない」ことを正直に報告する。
- `abandon_session(permit) -> Result<(), SessionError>`:
  `Prepared | Active | Draining` → `Stopped`（`abandoned = true`、
  `drains` を drop — quiescence 未確認のまま解放する明示的経路。
  `DebugHealth::sessions_abandoned_total` 計数、report/inspection
  に `abandoned` を表示）。`Disabled`/`Stopped` は
  `InvalidTransition`。

`SessionDrainStatus`:

```rust
#[non_exhaustive]
pub enum SessionDrainStatus {
    /// Still draining — counts of endpoints awaiting quiescence and
    /// records still in flight.
    Draining { pending_quiescence: usize, pending_records: u64, deadline_exceeded: bool },
    /// All endpoints quiesced and drained — session is Stopped and
    /// buffers were released.
    Stopped { clean: bool, records_collected: usize, records_overflowed: u64 },
    /// `poll_drain` called outside Draining.
    NotDraining { state: DebugSessionState },
}
```

- 「切替」: single-slot session のため `prepare` は非 Stopped で
  `SessionBusy`。切替は `request_stop` → `poll_drain` で `Stopped`
  を待ってから `prepare`（新 generation）。暗黙 queue は作らない。

### report.rs — DebugReport / export 認可

```rust
pub const DEBUG_REPORT_SCHEMA_VERSION: u16 = 1;

/// Bounds for report generation — same 0/None convention.
pub struct ExportLimits {
    /// Max section entries carried; `0` = section summaries only.
    pub max_items: usize,
    /// Approximate byte budget; `0` = unlimited.
    pub max_bytes: usize,
    /// Generation deadline; `None` = unlimited.
    pub deadline: Option<Duration>,
}

pub struct DebugReportRequest {
    /// Max confidentiality the caller may receive.
    pub disclosure: Confidentiality,
    /// `true` = external export (file/agent hand-off); requires
    /// `DebugConfig::allow_external_export`.
    pub external: bool,
}

#[non_exhaustive]
pub enum ExportError {
    /// Disclosure above `max_export_disclosure`, or `external`
    /// without `allow_external_export`.
    Unauthorized { requested: Confidentiality, permitted: Confidentiality, external: bool },
}

pub struct ExportTruncation {
    pub items_dropped: usize,
    pub withheld_confidential: usize,
    pub bytes_estimated: usize,
    pub deadline_exceeded: bool,
}

pub struct DebugReport {
    pub schema_version: u16,
    pub generated_at: Instant,
    /// Echo of the inspection the report summarizes (or `None` when
    /// generated session-only).
    pub inspection: Option<InspectionEcho>,
    pub session: Option<SessionReportView>,
    /// Bounded drained-record *summaries* — event id/version/source/
    /// generation/clock per record, capped by `max_items`. Raw
    /// `u64` payload values are NOT included (payload semantics are
    /// domain-owned; the report carries metadata only).
    pub records: Vec<RecordSummary>,
    pub truncation: ExportTruncation,
    pub notes: Vec<&'static str>,
}
```

- `report(&mut self, request, snapshot: Option<&InspectionSnapshot>,
  now) -> Result<DebugReport, ExportError>`:
  `request.disclosure > config.max_export_disclosure` または
  `request.external && !config.allow_external_export` →
  `Unauthorized`（`reports_rejected_total` 計数）。許可時は
  `confidentiality > request.disclosure` の項目を項目ごと除外し
  `withheld_confidential` に計数（diagnostics report と同じ
  非流出規則）。`max_items`/`max_bytes`/`deadline` で
  `ExportTruncation` に構造化記録。
- `DebugReport`/`InspectionSnapshot`/`SessionRecord` が運ぶ型は
  全て closed vocabulary（`EventRecord` metadata・`MetricValue`・
  `DiagnosticFinding` 要約・`&'static str`）であり、Secrets/
  Media/Path/Credential/Prompt/stderr を運ぶフィールドは存在
  しない — 型レベルで非含有を保証する。

### engine.rs — DebugInspector / DebugConfig / DebugHealth

```rust
pub struct DebugConfig {
    /// Highest confidentiality inspection/report requests may ask
    /// for — capability ceiling (OBS-11 owns real ACL).
    pub max_inspection_disclosure: Confidentiality,
    /// Same ceiling for `DebugReport` exports.
    pub max_export_disclosure: Confidentiality,
    /// Whether `DebugReportRequest.external` may be granted.
    pub allow_external_export: bool,
    /// Session drain deadline — `None` = never force; Draining
    /// persists until quiescence or explicit abandon.
    pub drain_timeout: Option<Duration>,
    /// Default `InspectionBudget` applied when a request does not
    /// override — request always carries its own budget, so this is
    /// the *documented fallback* used by `inspect_default` helpers.
    /// Kept for host convenience; `inspect` uses request.budget.
    pub default_budget: InspectionBudget,
}
```

```rust
pub struct DebugInspector {
    config: DebugConfig,
    authority: Option<DebugAuthority>,
    session: SessionSlot,
    invariant_checks: Vec<Box<dyn InvariantCheck>>,
    probe_sources: Vec<Box<dyn ProbeSource>>,
    health: DebugHealth,
}
```

- `new(config)` — authority なし（session API は `NoAuthority`）。
  `with_authority(config, authority)` — session 有効化。
- `add_invariant_check(Box<dyn InvariantCheck>)` /
  `add_probe_source(Box<dyn ProbeSource>)` — 登録順を保持
  （決定的評価順）。
- `inspect(&mut self, request: &InspectionRequest,
  inputs: &InspectionInputs<'_>, now: Instant)
  -> Result<InspectionSnapshot, InspectionError>`:
  1. `request.disclosure > max_inspection_disclosure` →
     `Unauthorized`。
  2. `budget.timeout` の起点を `now` とし、section 生成の合間に
     `Instant::now()` で経過検査（deadline 超過以降の section は
     `Unavailable{DeadlineExceeded}`）。
  3. target を宣言順に処理。Probes は先に全 `ProbeSource::decks()`
     を呼んで `Vec<ProbeDeckSnapshot>` を集め（失敗は
     `Unavailable{PortFailure}` + `probe_source_failures_total`）、
     `InspectionContext` を組み立ててから Invariants を評価する
     （check が probe reading を参照できる順序）。
  4. 各 section の item 収集は `budget.max_items`・残り
     `max_bytes`（概算 byte 会計）・deadline で打ち切り、
     `InspectionTruncation` に記録。`confidentiality >
     request.disclosure` の item は除外し `withheld_confidential`。
  5. `consistency` を全収集入力の generation stamp から計算
     （section 生成と独立）。
  6. `DebugHealth.inspections_total += 1`、truncation/deadline
     発生時は対応カウンタを計数。
- `publish_invariants(&mut self, diagnostics: &mut Diagnostics,
  snapshot: &InspectionSnapshot, now: Instant)
  -> InvariantPublishReport`:
  `snapshot.sections` の `Invariants` section を走査し、各
  `InvariantResult` につき (a) `Evidence` を構築 — `basis` 非空は
  `Evidence::derived`（`Err(MissingBasis)` にはならない）、空は
  `Evidence::reported`。`EvidenceMeta` は `domain`=check code の
  name ではなく `"debug.invariant"` 固定ラベル、`subject=Some(
  code.name)`、`time=Monotonic(now)`、`validity=Valid`（評価は
  実施済みの事実）、`confidentiality=Internal`、`payload=
  Claim(code.name)` — `record_evidence` で記録。(b) `Violated` →
  `DiagnosticFinding::new(code, "debug.invariant", None,
  severity.unwrap_or(Warning), Confirmed, Internal, summary,
  basis_refs, now)`、`Inconclusive` → 同 `NotEvaluable`/`Info`。
  `Satisfied` は finding を生成しない。`record_finding` の
  `(domain, code)` upsert で繰返しは merge される。
  `RetentionDisabled`（max_evidence/max_findings == 0）は
  `skipped` 計数で握り潰さず報告する（`InvariantPublishReport
  { evidence_recorded, findings_recorded, skipped_retention,
  skipped_no_section }`）。
  **既知制約**: `Satisfied` 後の finding Resolved 化は現行
  `Diagnostics` 公開 API に経路がなく、本 slice では Open finding
  が retention FIFO で bounded に残る。文書化して #293/将来の
  diagnostics API へ defer する。
- `report(&mut self, request: &DebugReportRequest,
  snapshot: Option<&InspectionSnapshot>, now: Instant)
  -> Result<DebugReport, ExportError>` — report.rs の契約。
- `session_state()`/`session_view()`/`drain_status()` 等の
  read-only accessor。
- `health() -> DebugHealth` — 固定カウンタ clone、source 非登録。

```rust
/// Fixed-size self-observation — never registered as a source.
#[derive(Debug, Default, Clone)]
pub struct DebugHealth {
    pub inspections_total: u64,
    pub inspection_errors_total: u64,
    pub items_truncated_total: u64,
    pub sections_unavailable_total: u64,
    pub withheld_confidential_total: u64,
    pub probe_source_failures_total: u64,
    pub invariant_checks_total: u64,
    pub invariant_violated_total: u64,
    pub invariant_inconclusive_total: u64,
    pub invariant_panics_total: u64,
    pub sessions_prepared_total: u64,
    pub sessions_activated_total: u64,
    pub sessions_stopped_total: u64,
    pub sessions_abandoned_total: u64,
    pub drain_records_delivered_total: u64,
    pub drain_records_overflowed_total: u64,
    pub reports_generated_total: u64,
    pub reports_rejected_total: u64,
    pub findings_published_total: u64,
    pub evidence_published_total: u64,
}
```

### lib.rs / Cargo.toml

- `pub mod debug;` 登録。crate doc のモジュール一覧に debug を
  追加し、RT 契約表に「全 debug 操作は非 RT only（alloc/lock）。
  RT 隣接値型（`SessionRecord`/`DrainedRecord`/`ProbeReading`）
  のみ `Copy` で RT-suitable な構築・読み取りを持つ」を明記。
- `Cargo.toml` 変更なし。`debug-tools` feature は #275 の
  「実装時にコンパイル検証」方針と OBS-07/09 の無条件 module
  先例に倣い deferred（canonical `cargo test --workspace` が
  default feature で検証できることを優先）。

## Data and Control Flow

- Setup（非 RT）: `DebugInspector::with_authority(config, auth)` →
  `add_probe_source(adapter)` / `add_invariant_check(domain impl)`。
- Inspect: `inspector.inspect(&request, &inputs, now)` →
  disclosure 認可 → probe decks 収集 → `InspectionContext` 組立 →
  section 生成（budget/deadline/disclosure 適用）→ consistency
  計算 → `InspectionSnapshot`。
- Diagnostics 接続: `inspector.publish_invariants(&mut diag,
  &snapshot, now)` → Evidence/Finding を `Diagnostics` へ記録
  → `diag.report(&req, now)` で確認可能。
- Session: `authority.permit()` → `prepare_session(permit, req)`
  → `attach_drain(Box::new(EventRecordDrain::new(sink, flag)))` →
  `activate_session()` → （domain が RT recorder を動かし records
  を `try_record`）→ `request_stop(permit)` → `poll_drain(now)`
  繰返し → 全 quiesced + lag 0 で `Stopped`（sink/ring 解放）。
  未確認なら `Draining` 継続 or `abandon_session(permit)`。
- Report: `inspector.report(&req, Some(&snapshot), now)` →
  認可 → bounded `DebugReport`。

## Non-Functional Constraints

- 依存: std + 既存 `ruviform-media-core`（`ClockDomainId`/
  `ClockEpochId`）+ 既存 crate 内型（`source`/`metrics`/`collector`/
  `record`/`rt_bridge`/`diagnostics`）。新規外部依存なし・
  `unsafe` なし・`logging` feature 非依存。
- RT 契約: debug の公開操作は全て非 RT only（alloc/lock/vector
  操作）。RT writer 経路に触れない。RT 隣接値型
  （`SessionRecord`/`DrainedRecord`/`ProbeReading`/
  `ProbeClockMark`/`ProbeStatusKind`）は `Copy` で no-alloc を
  `rt_` レーンで試験する。crate doc の RT 契約表に明記。
- 上限: `InspectionBudget`（items/bytes/timeout）・
  `SessionRequest`（max_records/drain_batch）・
  `ExportLimits`（items/bytes/deadline）・登録数は呼出側契約。
  全て設定駆動・超過は evict/計数/truncation として型付き報告
  （黙殺しない）。
- `#[non_exhaustive]` は全公開 enum に付与。`InvariantCheck::
  code`/`SessionRequest::label`/section `key`/`domain` 系は
  `&'static str` で cardinality を静的に制限。`summary`/`String`
  系は非 RT 構築のみで、path/bytes/token を運ぶ専用フィールドは
  存在しない。

## Risks and Trade-offs

- **feature gate を付けない判断**: #275 は `debug-tools` feature を
  挙げるが、OBS-07/09 が feature なしで着地した先例と、canonical
  `cargo test --workspace` が default feature のみで全 module を
  検証する運用を優先した。feature 分離が必要になれば #293 で
  `#[cfg(feature)]` を足すだけで移行可能（公開 API の形は
  変わらない）。
- **`publish_invariants` の finding resolve 制約**: `Diagnostics`
  の `record_finding` は Open upsert のみで、外部発 finding を
  Resolved にする公開経路がない。`Satisfied` を finding にしない
  （Evidence のみ）ことで「回復を Open finding で偽装しない」を
  優先し、Open 残留は retention bounded の既知制約として deferred
  記録した。`resolve_finding` の公開化は diagnostics module の
  API 拡張であり本 slice の編集範囲外。
- **`InvariantVerdict.basis` を `EvidenceRef` にした**: check が
  自分で `record_evidence` した ref を cite する設計。basis が
  diagnostics 未登録の ref を指す可能性は残るが、Evidence の
  basis は「説明可能な根拠参照」であり、存在検証は
  `unresolved_evidence_refs` が report 側で既に計数する
  （diagnostics の既存機構を再利用）。
- **`ProbeReading` の point identity を opaque u64 にした**:
  `NodeId`/`PortId` は domain 型で observability から名指せない。
  `(node, port)` の u64 pair は adapter が写像する opaque 値で、
  意味解釈は domain/consumer 側。本 slice の inspection は点の
  *存在と状態*を見るので十分。
- **`SessionDrain::drain_final` が `&mut dyn FnMut` を取る**:
  object-safe を保ちつつ `RecordSink::drain_final(impl FnMut)` へ
  委譲できる形。`EventRecordDrain` が唯一の concrete 実装で、
  他 domain は独自 adapter を書く（port の意味を保つ）。
- **`poll_drain` が `SessionDrainStatus` を返す（Result でない）**:
  Draining 中の繰返し問合せが通常経路であり、`NotDraining` は
  「まだ drain 中でない」という状態情報であって失敗ではない。
  遷移違反（Prepared での activate 等）は `SessionError` のまま。
- **`Arc<AtomicBool>` quiescence flag**: `SourceState`（Arc<
  AtomicU8>+Arc<AtomicBool>）と同じ共有 cell 先例。domain が
  recorder を RT path から外した時点で `store(true, Relaxed)`
  する契約を doc で固定し、`EventRecordDrain::writer_quiesced`
  が読む。flag を立て忘れた domain は session が永久 Draining
  になるが、それは「quiescence 未確認」の正直な報告であり
  `abandon` が明示的脱出経路。

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- concrete probe adapter（`ruviform-audio::typed::probe` →
  `ProbeSource`/`InvariantCheck`）・`debug-tools` feature 分離
  | OBS-11（#293）/`ruviform-audio` | #293 実装時 | 依存循環の
  ため本 crate には置けない。
- `Diagnostics` finding resolve 公開 API | `diagnostics/` API
  拡張 | 別 Issue/改訂 | 外部発 finding の Open→Resolved 経路が
  ない。Open 残留は retention bounded の既知制約。
- Host 認可機構（capability 発行・失効・監査） | OBS-11（#293）|
  #293 実装時 | 本 Issue は `DebugAuthority`/`SessionPermit`/
  `allow_external_export` のモデル化まで。
- `FindingSink` → tracing adapter・RT Record → tracing 変換 |
  OBS-03（#278）| #278 実装時 | tracing 依存を本 module に
  持ち込まない。
- 両 Consumer での Inspection/Report 受入・Synvelis Debug 能力
  連携 | #281/#219・synvelis#81 | cross-repo gate | worker 環境は
  単一 repo。
- ON/OFF DSP 結果同値・RT callback alloc/lock/IO 非呼出しの実機
  検証 | domain/consumer 側 + RT verification lane | #293/#281 |
  debug module は RT 呼出 API を持たない構造保証まで。
- PCM/Signal Capture backend・Deterministic Replay・Crash
  Handler | Issue 非対象 | 実需要 | lossy 診断契約を lossless
  Recording 品質契約と混同しない。

## Verification Notes

- 正規検証: `mise run fmt-check` / `mise run check` /
  `mise run lint` / `mise run test` / `mise run test-rt`。
- golden 試験は `src/debug/tests.rs` に集約し、`cargo test
  --workspace`（default features）で実行される。
- `rt_` プレフィックス試験で `SessionRecord`/`ProbeReading` 等の
  `Copy` 値型の no-alloc を `assert_no_alloc` で検証する
  （`test-rt` レーン対象）。
