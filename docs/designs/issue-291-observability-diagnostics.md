---
id: "DESIGN-034Unxw6Jlg98IypxJkHfQ"
type: "design"
status: "approved"
title: "Issue #291 OBS-09 Diagnostics: Evidence/Finding/Health/Report — 型で分ける事実と推定"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/291"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-291-observability-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-277-observability-core-source-contract.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/obs-05-observability-leaf-primitive-deps.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-291-observability-diagnostics.md"
    - "docs/designs/issue-277-observability-core-source-contract.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-291-observability-diagnostics-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  deferred-by: []
---

# Issue #291 OBS-09 Diagnostics: Evidence/Finding/Health/Report — 型で分ける事実と推定

## Context

`ruviform-observability` は OBS-02（#277）で source 登録/寿命契約、
OBS-05（#280）で `record`/`rt_bridge`、OBS-07（#289）で Collector
（`MetricsSnapshot`/`ObservationBatch`/`Measurement`/`MeasurementValidity`/
`CollectorHealth`）を実装済みである。Issue #291（OBS-09）はその上に
**診断層**を置く: 観測値と型付き状態から「検証可能な事実」と「推定」を
区別した Evidence/Finding/Health/Report を提供する。

設計上の中心問題は 4 つ: (1) Observed/Derived/Reported/Inferred と
確定/推定/評価不能を**型で混同不能**にする語彙設計、(2) Source
Generation・clock domain・Evidence 寿命を固定し Retired Reference を
別 Source へ再結合しないこと、(3) 閾値・Hysteresis・回復・Unknown・
stale・部分障害を明示的状態遷移として評価する `HealthEvaluator`、
(4) max findings/bytes/deadline・切り詰め・開示区分・未認可拒否を持つ
`DiagnosticReport` と、Logging OFF で動く bounded `FindingSink`。

## Scope

### In Scope

- `src/diagnostics/` モジュール群: `mod.rs`（module doc + re-export）、
  `evidence.rs`、`finding.rs`、`health.rs`、`report.rs`、`sink.rs`、
  `engine.rs`（`Diagnostics` 集約器）、`tests.rs`（crate 内 unit test）。
- lib.rs への `pub mod diagnostics` 登録と crate doc（モジュール一覧・
  RT 契約表）更新。`Cargo.toml` は変更なし（新規依存・feature 追加なし）。

### Out of Scope

- tracing adapter・Subscriber 接続（OBS-03 #278）、Host 認可機構
  （OBS-11 #293）、両 Consumer 検証（#281）、永続化、UI、ドメイン
  Error/Result・`DiagRegistry` の置き換え。

## Components and Boundaries

### evidence.rs — Evidence 語彙

```rust
pub enum EvidenceKind {          // #[non_exhaustive]
    Observed,    // collector/ドメインが直接観測した事実
    Derived,     // Observed 等からの決定的導出（basis 必須）
    Reported,    // 他者/外部が主張する値（裏付けなし・Observed に昇格しない）
    Inferred,    // 根拠からの推論（basis 必須・断定しない）
}

pub enum EvidenceTime {          // #[non_exhaustive]
    Monotonic(Instant),          // collector 観測の host monotonic 時刻
    Clock(ClockContext),         // ドメイン native clock stamp
    Unstamped,                   // 時刻主張なし
}

pub enum EvidencePayload {       // #[non_exhaustive] — 任意バイト列を運べない型
    None,
    Metric(MetricValue),         // 観測値そのもの（clone）
    Claim(&'static str),         // ドメイン宣言の静的 claim code
}

pub struct Evidence {
    kind: EvidenceKind,
    source: Option<SourceIdentity>,   // OBS-02 登録 identity（id+generation）
    subject: Option<&'static str>,    // 対象 metric key 等の静的 subject
    domain: &'static str,             // provenance ラベル（origin 相当）
    time: EvidenceTime,
    validity: MeasurementValidity,    // 既存 metrics 語彙を再利用
    confidentiality: Confidentiality, // 既存 source 語彙を再利用
    basis: Box<[EvidenceRef]>,        // Derived/Inferred は非空必須
    payload: EvidencePayload,
}
```

- コンストラクタで不変条件を強制: `observed(source, ..)` は
  `source: SourceIdentity` 必須・basis 空。`reported(..)` は basis 空。
  `derived(basis, ..)`/`inferred(basis, ..)` は basis 非空を要求し、
  空なら `Err(EvidenceError::MissingBasis)`（`#[non_exhaustive]`）。
- `Evidence::from_measurement(&Measurement) -> Evidence`: `kind=Observed`、
  `source=m.identity`、`time=Monotonic(m.observed_at)`、
  `validity=m.validity`、`confidentiality=m.descriptor.semantics
  .confidentiality`、`subject=Some(m.descriptor.key)`、
  `payload=Metric(m.value.clone())`（`None` のとき `None`）。
  `MeasurementValidity` → `Evidence.validity` は同一語彙をそのまま
  保持する（写像による情報落としをしない）。
- `EvidenceRef(u64)`: engine の evidence store が採番する不変参照。
  id は単調増加・再利用しない。`EvidenceEntry { reference,
  evidence, superseded }` は snapshot/report 時点の表示用ビューで、
  `superseded` は「記録時の `source` identity が最新 snapshot に存在
  しない（retire/Removed/世代交代）」ことを示す bool。

### finding.rs — Finding 語彙

```rust
pub struct DiagnosticCode {          // Copy
    pub name: &'static str,           // ドメイン所有の安定 code
    pub schema_version: u16,          // code の payload/意味 version
}

pub enum FindingSeverity { Info, Warning, Critical }        // #[non_exhaustive]
pub enum FindingCertainty { Confirmed, Inferred, NotEvaluable } // #[non_exhaustive]
pub enum FindingStatus { Open, Resolved }                   // #[non_exhaustive]

pub struct DiagnosticFinding {
    pub code: DiagnosticCode,
    pub domain: &'static str,
    pub source: Option<SourceIdentity>,
    pub severity: FindingSeverity,
    pub certainty: FindingCertainty,
    pub status: FindingStatus,
    pub summary: String,               // 非 RT のみ。任意 bytes/path/token を
                                       // 運ぶ専用フィールドは存在しない
    pub evidence_refs: Box<[EvidenceRef]>,
    pub first_seen_at: Instant,
    pub last_seen_at: Instant,
}
```

- `DiagnosticCode` は中央 enum に統合しない（ドメイン `Error` との
  二重管理禁止・コード再利用による互換性破壊を避ける）。`Ord` を derive
  し report の決定的ソートに使う。
- コンストラクタ `confirmed/inferred/not_evaluable` は certainty と
  status=Open・時刻を初期化する。`Inferred` は basis（evidence_refs）
  非空を要求 — 根拠なき推定を型で防ぐ。`NotEvaluable` は評価不能の
  理由を summary に持つ。
- severity 順位は `Info < Warning < Critical`（内部 rank fn。
  `#[non_exhaustive]` のため match は wildcard 必須）。

### health.rs — HealthEvaluator

```rust
pub enum HealthState { Healthy, Degraded, Unhealthy, Unknown } // #[non_exhaustive]
// 順位: Healthy < Unknown < Degraded < Unhealthy（Unknown は悪化でも
// 回復でもない第 4 状態として明示的に扱う）

pub enum SignalKind { GaugeValue, CounterDelta, CounterRate }  // #[non_exhaustive]
pub enum SignalDirection { Above, Below }                      // #[non_exhaustive]

pub struct Hysteresis { pub enter_after: u32, pub recover_after: u32 }
// 連続評価カウント。両方 >= 1 必須（0 は RuleError）。

pub struct HealthRule {
    pub code: DiagnosticCode,          // 遷移時 finding の code
    pub metric_key: &'static str,      // MetricDescriptor.key に bind
    pub signal: SignalKind,
    pub direction: SignalDirection,
    pub degrade_at: Option<f64>,       // この値以上/以下で最低 Degraded
    pub unhealthy_at: Option<f64>,     // より厳しい側で Unhealthy
    pub reduced_validity_is: HealthState, // Stale/Partial/Discontinuous 時
    pub missing_is: HealthState,          // measurement 自体が無い時
    pub valueless_is: HealthState,        // value=None (Unavailable等) 時
    pub hysteresis: Hysteresis,
}
```

- 評価順序（固定・文書化）: (1) measurement 不在 → `missing_is`、
  (2) `value=None` → `valueless_is`、(3) `validity != Valid` →
  `reduced_validity_is`、(4) signal 抽出不能（delta/rate が None 等）
  → `Unknown`、(5) threshold 比較 → Healthy/Degraded/Unhealthy。
- threshold 意味: `Above` は `signal >= threshold` が bad、
  `Below` は `signal <= threshold` が bad。`unhealthy_at` は
  `degrade_at` より悪い側にあること（Above: `unhealthy_at >=
  degrade_at`、Below: `unhealthy_at <= degrade_at`）を検証。
- **hysteresis（連続カウント式）**: rule ごとに `current`・
  `run_level`（現在の非 Healthy streak の最悪 level）・`run_count`・
  `ok_count` を保持。raw verdict `L`:
  - `L == Healthy`: `ok_count += 1`。`ok_count >= recover_after` かつ
    `current != Healthy` → `current = Healthy`（transition 記録）。
    それ以外は現状維持（部分的改善では降格しない）。
  - `L != Healthy`: `ok_count = 0`。`L == run_level` なら
    `run_count += 1`、異なれば `run_level = L, run_count = 1`
    （level 変化で streak リセット — 交互信号は確定させない）。
    `run_count >= enter_after` なら `current = max(current, run_level)`
    （Unknown < Degraded < Unhealthy の順位で escalate のみ。
    Degraded→Unknown のような「見えなくなった」は現状維持 =
    悪い状態を Unknown で帳消しにしない）。
- **generation リセット**: rule 状態は最後に観測した
  `SourceIdentity` を保持し、id または generation が変われば
  hysteresis 状態（current/run/ok count）を初期化してから評価する
  （再登録を前世代の streak と混同しない）。
- `HealthEvaluator`: 1 ドメイン分の rule 集合を保持
  （`add_rule`/`remove_rule`/`clear` は非 RT）。`evaluate(
  &mut self, measurements: &[Measurement], now: Instant) ->
  DomainHealth` が rule 評価・遷移検出・finding upsert を行う。
- `DomainHealth { domain, state, rules: Vec<RuleHealth> }`。
  `state` は rule 状態の決定的 rollup:
  - 既知（非 Unknown）が 0 件 → `Unknown`
  - 既知が全て Unhealthy → `Unhealthy`
  - 既知に Unhealthy が一部のみ → `Degraded`（部分障害）
  - 既知に Degraded あり → `Degraded`
  - 既知が全て Healthy だが Unknown が混在 → `Degraded`
    （「見えている範囲は正常」だが宣言済み観測面の一部が不明 —
    Healthy と断定しない）
  - 全て Healthy → `Healthy`
- `RuleHealth { code, metric_key, source, state, input,
  last_transition_at }`。`input` は `RuleInput::{Signal(f64),
  ReducedValidity(MeasurementValidity), Missing, Valueless,
  Unevaluable}` の観測事実を保持し、rollup だけでなく「なぜその
  state か」を説明可能にする。

### report.rs — Snapshot / Report / 開示

```rust
pub enum SnapshotConsistency {   // #[non_exhaustive]
    /// 各 series/atomic の独立読取の集約。単一時点 transactional
    /// snapshot ではない。
    IndependentReads,
}

pub struct DiagnosticSnapshot {
    pub generation: u64,               // MetricsSnapshot の世代
    pub taken_at: Instant,
    pub consistency: SnapshotConsistency,
    pub domains: Vec<DomainHealth>,
    pub findings: Vec<DiagnosticFinding>,   // 有界・Open+Resolved
    pub evidence: Vec<EvidenceEntry>,       // 有界・superseded flag 付き
    pub missing_rules: usize,               // 入力無しで評価された rule 数
}

pub struct ReportLimits {              // 全て 0 許容（0=空/無制限は
    pub max_findings: usize,           //   項目ごとに文書化）
    pub max_bytes: usize,
    pub deadline: Option<Duration>,
}

pub struct DiagnosticsConfig {
    pub max_evidence: usize,           // 0 = evidence 保持無効
    pub max_findings: usize,           // 0 = finding 保持無効
    pub report: ReportLimits,
    pub max_export_disclosure: Confidentiality, // engine の export 上限
    pub sink_policy: SinkPolicy,
}
// validate(): sink_policy.window == 0 → DiagnosticsConfigError::
// ZeroSinkWindow（唯一の invalid 組合せ）。他は 0=disabled/empty として受理。

pub struct ReportRequest { pub disclosure: Confidentiality }
pub enum ReportError {               // #[non_exhaustive]
    Unauthorized { requested: Confidentiality, permitted: Confidentiality },
}

pub struct ReportTruncation {
    pub findings_dropped: usize,
    pub evidence_dropped: usize,
    pub health_dropped: usize,
    pub withheld_confidential: usize,   // 開示区分超過で除外した件数
    pub unresolved_evidence_refs: usize,// report に残らなかった参照数
    pub bytes_estimated: usize,
    pub deadline_exceeded: bool,
}

pub struct DiagnosticReport {
    pub schema_version: u16,            // REPORT_SCHEMA_VERSION = 1
    pub generated_at: Instant,
    pub source_generation: u64,
    pub disclosure: Confidentiality,    // 実際に適用した上限
    pub findings: Vec<DiagnosticFinding>,
    pub evidence: Vec<EvidenceEntry>,
    pub health: Vec<DomainHealth>,
    pub truncation: ReportTruncation,
    pub notes: Vec<&'static str>,       // 静的な制限事項（非 transactional 等）
}
```

- **開示モデル**: `Confidentiality`（Public < Internal < Confidential、
  内部 rank fn）を「開示区分」として再利用する。`report(request, now)`
  は `request.disclosure > config.max_export_disclosure` なら
  `Err(ReportError::Unauthorized)`（capability パラメータによる
  未認可拒否 — OBS-11 の実認可を待たない明示的モデル化）。
  許可された場合、`confidentiality > request.disclosure` の
  evidence/finding は report から**除外**し `withheld_confidential`
  に計数する（値を伏せるのではなく項目ごと出さない = 非流出）。
- **切り詰め**: findings は severity 降順の stable sort
  （同 severity は記録順維持）→ `max_findings` で打ち切り。
  evidence/health も `max_bytes`（項目ごとの概算 byte:
  `size_of` + `summary.len()` + refs 等の単純和）を超えた時点で打ち切り。
  `deadline` は各 item 追加前に `now`（caller 供給 Instant）との
  elapsed を検査し、超過で `deadline_exceeded=true` を立てて打ち切る。
  打ち切りは必ず `ReportTruncation` に構造化して記録する（黙って
  切らない）。
- `notes` は `&'static str` の静的注記のみ（件数等の動的値は
  `ReportTruncation` の数値フィールドに置く — String を増やさない）。

### sink.rs — FindingSink / DiagnosticsEvent / SinkPolicy

```rust
pub trait FindingSink: Send {           // object-safe・Sync 不要
    fn emit(&mut self, event: DiagnosticsEvent<'_>);
}

pub enum DiagnosticsEvent<'a> {         // #[non_exhaustive]
    /// ドメイン rollup state の遷移（重要状態変化のみ）。
    HealthTransition {
        domain: &'static str,
        from: HealthState,
        to: HealthState,
        at: Instant,
    },
}

pub struct SinkPolicy { pub max_events: u32, pub window: Duration }
// leaky bucket: window 内に max_events 件まで emit、超過は
// DiagnosticsHealth.sink_suppressed_total に計数して捨てる。
// window=0 は config validate で拒否。
```

- 実 tracing adapter は OBS-03 deferred — 本 Issue は port と抑制
  機構のみ提供し、`tracing` 型への依存を持たない（Logging OFF 契約）。
- sink 呼出は evaluate（非 RT）からのみ。sink の panic は呼出側で
  `catch_unwind`? — いや、panic は bug として伝播させるのではなく
  **隔離**する: emit を `std::panic::catch_unwind(AssertUnwindSafe)`
  で包み、panic は `sink_panics_total` に計数して評価を継続する
  （観測機構が診断対象を道連れにしない）。

### engine.rs — Diagnostics 集約器

```rust
pub struct Diagnostics {
    config: DiagnosticsConfig,
    evaluators: BTreeMap<&'static str, HealthEvaluator>,
    evidence: VecDeque<EvidenceEntry-store>,   // 有界 FIFO + next_ref
    findings: VecDeque<DiagnosticFinding>,    // 有界 FIFO + open upsert index
    sink: Option<Box<dyn FindingSink>>,
    sink_window_start: Instant,
    sink_window_count: u32,
    health: DiagnosticsHealth,                // 固定カウンタ群
    last_domain_states: BTreeMap<&'static str, HealthState>, // rollup 前回値
}
```

- `new(config) -> Result<Self, DiagnosticsConfigError>`
- `record_evidence(&mut self, ev: Evidence) -> Result<EvidenceRef,
  EvidenceError>`: `max_evidence==0` → `Err(EvidenceError::
  RetentionDisabled)`。FIFO evict・`evidence_evicted_total` 計数。
- `record_finding(&mut self, f: DiagnosticFinding) -> Result<
  FindingId?` — finding の upsert は `(domain, code)` を key に
  Open finding を探して `last_seen_at`/severity/certainty/
  evidence_refs/summary を更新（resolved でない限り重複蓄積しない）。
  `max_findings==0` → `Err(FindingError::RetentionDisabled)`。
  戻り値は `()`（参照 id は finding に不要 — finding は pull データ）。
- `add_health_rule(domain, rule) -> Result<(), RuleError>`:
  domain の evaluator を get-or-create して rule を追加。
  `remove_health_rule(domain, code)`/`remove_domain(domain)`。
- `set_sink(&mut self, sink: Box<dyn FindingSink>)`。
- `evaluate(&mut self, snapshot: &MetricsSnapshot, now: Instant)
  -> DiagnosticSnapshot`: 全 evaluator を `snapshot.measurements()` で
  評価 → `DomainHealth` 集約 → domain state 前回値との差分で
  `DiagnosticsEvent::HealthTransition` を sink policy 経由で emit
  → rule 遷移に伴う finding upsert（code=rule.code、severity=
  Degraded→Warning/Unhealthy→Critical/Unknown→Info、
  certainty=Confirmed（threshold 違反は実測+宣言規則）または
  Unknown→NotEvaluable、回復で status=Resolved）→ evidence の
  superseded 再計算（`source` identity が snapshot に無い →
  `superseded=true`）→ `DiagnosticSnapshot` 構築。
- `report(&mut self, request: &ReportRequest, now: Instant)
  -> Result<DiagnosticReport, ReportError>`: 最新の内部状態
  （最後の evaluate 結果 or 現在の evidence/findings/evaluators）から
  開示フィルタ + 切り詰めを適用して生成。`reports_generated_total`/
  `reports_rejected_total` 計数。
- `health(&self) -> DiagnosticsHealth`: 固定カウンタの clone
  （evidence/findings retained・evicted・evaluations・sink
  emitted/suppressed/panics・reports generated/rejected・
  domain_transitions_total）。**source に登録しない**（非再帰）。

### lib.rs / Cargo.toml

- `pub mod diagnostics;` 登録。crate doc のモジュール一覧に
  diagnostics を追加し、RT 契約表に「全 diagnostics 操作は非 RT
  only（alloc/lock）」を明記。
- `Cargo.toml` 変更なし（std + 既存 `ruviform-media-core` の
  `ClockContext`/`ClockDomainId` 経由の型のみ使用。新規 feature も
  不要 — Logging OFF が default）。

## Data and Control Flow

- Domain（非 RT）: `collector.handle().snapshot()` または batch から
  `MetricsSnapshot` を取得 → 注目 measurement に対し
  `Evidence::from_measurement`/`derived`/`reported`/`inferred` を構築し
  `diagnostics.record_evidence` で記録（`EvidenceRef` が返る）。
- Rule 宣言: `diagnostics.add_health_rule("my.domain", HealthRule{..})`
  を setup 時に 1 回。`evaluate` ごとに再宣言は不要（hysteresis 状態を
  保持するため）。
- Evaluate: `diagnostics.evaluate(&snapshot, Instant::now())` →
  rule 評価（measurement 不在/stale/valueless/threshold/hysteresis）
  → domain rollup → transition 検出 → sink emit（抑制上限付き）+
  finding upsert → `DiagnosticSnapshot` 返却。
- Report: `diagnostics.report(&ReportRequest{ disclosure }, now)` →
  Unauthorized なら `Err`、許可なら開示フィルタ + truncation を適用した
  `DiagnosticReport`。
- Source 世代: evidence の `source` は記録時の `SourceIdentity`
  （id+generation）。evaluate 時に最新 snapshot と照合し、不一致
  （retire/Removed/再登録）は `superseded=true` — 別 Source への
  再結合は起きない（ref id は単調・evidence は immutable）。

## Non-Functional Constraints

- 依存: std + 既存 `ruviform-media-core`（`ClockContext` 経由）+
  既存 crate 内型（`source`/`metrics`/`collector`/`record`）。
  新規外部依存なし・`unsafe` なし・`logging` feature 非依存。
- RT 契約: diagnostics の全公開操作は非 RT only（alloc/lock/vector
  操作）。RT writer 経路に触れない。crate doc の RT 契約表に明記。
- 上限: `max_evidence`/`max_findings`/`max_batch`（report）/
  `SinkPolicy` window・`Hysteresis` count。全て設定駆動・超過は
  evict/計数/truncation として型付きで報告（黙殺しない）。
- `#[non_exhaustive]` は全公開 enum に付与。`DiagnosticCode.name` と
  `domain`/`subject`/`origin` 系は `&'static str` で cardinality を
  静的に制限。`summary` は `String`（非 RT）だが path/bytes/token を
  運ぶ専用フィールドは存在しない。

## Risks and Trade-offs

- `Evidence.validity` に `MeasurementValidity` を再利用した: evidence
  は measurement 以外（Reported/Inferred）もあり得るが、validity の
  4+2 分類（Valid/Stale/Partial/Discontinuous/Unavailable/
  Unsupported）は evidence の「欠落/stale 等」を過不足なく表せ、
  並行 enum を新設するより単一語彙の方が一貫する。Reported/Inferred
  evidence が measurement を持たない場合は `Unavailable`/`Valid` を
  ドメインが正直に宣言する。
- hysteresis を「連続カウント式」にした: threshold band 式
  （enter/exit 閾値差）もあり得るが、count 式は決定的・テスト容易で
  「enter_after 回連続で悪化→遷移」「recover_after 回連続で正常→回復」
  を明示できる。閾値自体の hysteresis（帯域）は domain が
  degrade_at/unhealthy_at の間隔で表現可能。
- rollup で「既知全て Healthy + Unknown 混在 → Degraded」とした:
  Unknown を無視して Healthy と言い切るより保守的。domain が optional
  source を健康評価に含めたくない場合は rule を登録しない（rule が
  無ければ評価対象外 = Unknown にすらならない）ことで制御する設計。
- `report(&mut self)`: `reports_generated_total` 計数のため
  `&mut self`（非 RT のみなので問題なし）。`evaluate` も `&mut self`
  （hysteresis/evidence/findings/sink 状態を更新）。
- sink panic の `catch_unwind`: 観測 sink の panic で診断全体を
  道連れにしないための隔離。`AssertUnwindSafe` は sink が
  `&mut self` 経由で外部状態を触り得るため必要（unwind safety は
  保証しないが panic を計数して継続するのが目的）。
- finding upsert の `(domain, code)` key: 同一 code の再発を Open の
  まま `last_seen_at` 更新で表す。Resolved finding は upsert 対象外
  （新規 Open として再作成）。`max_findings` ring の FIFO evict で
  無制限蓄積を防ぐ。

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- `FindingSink` → `tracing` Event adapter・rate limit 連携 | OBS-03
  （#278）| #278 実装時 | tracing 依存を持ち込むと Logging OFF 契約が
  崩れる。
- Host 認可機構（capability 発行/失効/監査） | OBS-11（#293）|
  #293 実装時 | 本 Issue は `max_export_disclosure` パラメータの
  モデル化まで。
- 両 Consumer での Evidence/Health ユースケース受入 | #281 |
  cross-repo gate | worker 環境は単一 repo。
- Evidence の永続化・downsample・外部 export 形式 | 実需要 |
  有界 in-process 保持で現行要件を満たす。
- f64 以外の signal 型・複合 rule（AND/OR）・rule 動的 reload |
  実需要 | 未定義の意味論を仮固定しない。

## Verification Notes

- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全 pass。
- [ ] golden: Evidence 4 種の区別・basis 必須性・Finding certainty・
  Health 遷移（閾値/hysteresis/回復/Unknown/stale/部分障害）・
  Snapshot/Report truncation・開示拒否・sink 抑制。
- [ ] `logging` feature OFF での build+test。
- [ ] 既存 observability 試験が無改変で pass（回帰なし）。
