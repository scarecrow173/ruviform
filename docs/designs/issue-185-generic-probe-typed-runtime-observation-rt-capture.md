---
id: "DESIGN-034QhWjbVG7vgYciUpfE2b"
type: "design"
status: "approved"
title: "Issue #185 Generic Probe: typed-runtime observation points, Off/Monitor/Capture execution, chunk-pool RT handoff, non-RT probe broker/collector"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/193"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-185-generic-probe-rt-capture.md"
    - "docs/adr/issue-185-generic-probe-contract-placement-and-rt-handoff-model.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md"
    - "docs/designs/issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md"
    - "docs/discovery/issue-185-generic-probe-observation-inventory.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/media-observation-non-destructive-pcm.md"
    - "docs/designs/foundation-observability-minimal-api.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-185-generic-probe-rt-capture.md"
    - "docs/adr/issue-185-generic-probe-contract-placement-and-rt-handoff-model.md"
    - "docs/discovery/issue-185-generic-probe-observation-inventory.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/104"
    - "https://github.com/scarecrow173/ruviform/issues/187"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/187"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/196"
  deferred-by: []
---

# Issue #185 Generic Probe: typed-runtime observation points, Off/Monitor/Capture execution, chunk-pool RT handoff, non-RT probe broker/collector

## Context

Issue #185 の最小 PoC を `ruviform-audio::typed` 上に構築する。前提は #195 の M2 契約（`PreparedPlan`/`ScheduledNode`/`RtRuntime`/`RtState`/snapshot publish/generation-checked commands/`Retirer`）と #184 の IR（`NodeId`/`PortId`/`PortCapability::ProbePoint`/`SignalDomain`/`PortFormat::Audio`）で確定済み。観測点の物理実体は arena `Slot` であり、tap 可能な位置は `RtRuntime::process` の schedule loop 内で 4 箇所に固定される。レビューで確定した主要制約:

- binding 解決規則（output port→slot index・effective input slot）の複製を作らない（compile 層で確定）。
- `Retirer` の leak-on-full を PCM 搬送に使わない — `chunks_out` の capacity = pool サイズによる構造的不満杯保証。
- generation-scoped command は activation で適用・lifetime command は毎 callback で適用、binding は `bound_generation` guard で誤 slot 観測を防ぐ。
- 絶対 `sample_position` は `RtState.frame_cursor`（callback ordinal の `seq` ではなく cumulative frames）。

## Scope

### In Scope

- `typed::probe` モジュール: 観測点参照・capability 列挙・`ProbeBroker`/`ProbeDeck`/`RtProbe`/`ProbeCollector`・record 型。
- `compile.rs` additive 拡張: `ScheduledNode::outputs` → `Vec<(PortId, usize)>`、`InputBinding::effective_slot`、`ScheduledNode::{latency_accum, own_latency}`。
- `runtime.rs` additive 拡張: `RtState::{frame_cursor, probes}`、probe command 2 系統 drain、4 tap 点、`GraphRuntime::published_plan`。
- `level.rs` additive 拡張: `ChannelLevel` の channel 単位算出関数（`LevelMeter` 挙動不変）。
- PoC テスト: `crates/ruviform-audio/tests/` の fixture を用いた spec AC-1..AC-10 相当の検証。

### Out of Scope

- TimedEvent/Control payload 観測、plugin 内部 stage、cross-session probe ID、永続ストレージ、Recording Writer 契約、旧 pull 型 graph、`ruviform-ui` 結合。

## Components and Boundaries

### `typed::probe` — 公開契約（全て `#[non_exhaustive]` enum / `pub struct`）

| 型 | 役割 |
| --- | --- |
| `ProbeId` | broker 生存期間内で単調採番される `u64` ラッパ（cross-session 安定性は範囲外） |
| `ProbePoint { node: NodeId, port: PortId, kind: TapKind }` | 論理観測点参照。`TapKind::{NodeInputPre, NodeOutputPost, GraphInput, GraphOutput}` |
| `PointAvailability` | `Configured`/`Inferred`/`Unavailable{reason}`/`Unsupported{reason}`/`Unknown{reason}` — 物理解決先行・宣言確認後置 |
| `ProbeMode` | `Off`/`Monitor`/`Capture(CaptureConfig)` — 実行状態。`CaptureConfig{max_frames: u64}` が期間予算 |
| `RecordMeta` | 全 record 共通封筒: `probe_id`/`node`/`port`/`kind`/`generation`/`sample_position`/`frames`/`channels`/`sample_rate`(宣言値)/`effective_latency` |
| `MetricRecord { meta, levels: [ChannelLevel; MAX_CH], channels: u8 }` | Peak/RMS snapshot |
| `CaptureChunk { meta, data: Vec<f32>, frames_valid }` | PCM block（planar ch-major、capacity = `max_frames × max_channels`） |
| `StatusRecord { meta, kind: StatusKind }` | `Bound`/`Unbound`/`Unobserved{reason}`/`CaptureStarted`/`CaptureStopped{reason}`/`Retired` — 未観測・不連続の区別は record として明示 |
| `ProbeEvent` | `Metric(MetricRecord)`/`Captured(Box<CaptureChunk>)`/`Status(StatusRecord)` — collector の drain 単位 |
| `ObservationReport { point, availability }` | capability 列挙の返却単位 |
| `ProbeError` | `UnknownPoint`/`NotObservable{availability}`/`BudgetExceeded`/`ChannelFull`/`UnknownProbe` — 非 RT API の定義済み失敗面 |

`Configured` は `PortCapability::ProbePoint` 宣言済み + 物理解決成功、`Inferred` は宣言なし + 物理解決成功、`Unavailable` は port 存在 + 解決失敗（未接続・`Activation::Disabled`・binding 不在）、`Unsupported` は非 `SignalDomain::Audio`、`Unknown` は node/port が spec/plan に存在しない。

### `ProbeBroker` — 非 RT 制御面（`&mut GraphRuntime` 連携）

- 保持: spec（`ValidatedGraph` 由来の `GraphSpec` clone）、probe 台帳 `BTreeMap<ProbeId, ProbeEntry>`、command `Producer`、`free` 用 chunk producer、deck stats の reader half。
- `capabilities() -> Vec<ObservationReport>`: spec の全 audio port + boundary を走査し plan binding で分類。決定的（BTreeMap/Vec 順で byte 等価）。
- `install(point, mode) -> Result<ProbeId>`: capability 解決 → `NotObservable`/`UnknownPoint` を早期 return。binding 解決結果 `{slot, kind, schedule_index, effective_latency, generation}` を `ProbeCommand::Install`（generation-scoped）で送る。
- `set_mode`/`capture`/`stop_capture`/`remove`: lifetime command（`SetMode`/`Remove`）を毎 callback drain 側へ。`SetMode` は mode 全体を運ぶ（冪等）。
- `on_publish()`: `GraphRuntime::published_plan()`（新規アクセッサ）から最新 plan を取得し、全 live probe を再解決 → `Rebind{probe_id, binding, mode, generation}` を送る。`Rebind` は完全状態を運び欠落 `SetMode` を自己修復する。
- `maintain()`: `free` channel の在庫を pool 目標数まで補充 + collector 側 recycle 待ち在庫管理。非 RT のみ・呼び出し周期は consumer 自由。
- `probe_status(id) -> Option<ProbeStatus>`: per-probe `Counter` reader + deck status mirror（applied generation/mode/loss/unbound_blocks）の合成読み出し。

### `ProbeDeck`/`RtProbe` — RT 適用面（`RtState.probes`）

- `ProbeDeck { probes: Vec<RtProbe>, free: Consumer<Box<CaptureChunk>>, chunks_out: Producer<Box<CaptureChunk>>, telemetry: Producer<ProbeEvent>, cmd: Consumer<ProbeCommand>, active_taps: u16, stats: ProbeDeckStats }`。
- `RtProbe { probe_id, point, binding: BoundSlot{slot, kind, bound_generation, effective_latency}, mode, working_chunk: Option<Box<CaptureChunk>>, capture_left: u64, lost_blocks: Counter, unbound_blocks: Counter, state_mirror: Arc<AtomicU64> }`。
- `active_taps` は command drain 後に `mode != Off` の総数を再計算し、0 なら全 tap 点を early-skip（OFF 時コスト = 整数比較 1 回）。
- tap 実行規則: `mode != Off` → `binding.bound_generation == plan.generation` 確認（不一致 → edge-triggered `Unbound` status + `unbound_blocks++`、slot には触れない）→ `try_borrow` で slot 共有借用 → `stamp() == seq` 確認（不一致 → `Unobserved` + 早期 return）→ `[..frames]` のみ観測。Off/失格判定では一切 slot を borrow しない。

### `ProbeCollector` — 非 RT 配送面

- `drain(&mut Vec<ProbeEvent>) -> usize`: `chunks_out` と `telemetry` を drain し (generation, sample_position) で整列可能な event 列を返す。pull ベース・呼び出し側が頻度を制御（購読解除 = drain 停止で完結）。
- `recycle(chunk)`: drain 済み chunk を `free` channel へ try_push（満杯なら drop — dealloc は非 RT なので安全）。
- `status_mirror()`: per-probe stats snapshot。

### `ChannelLevel::measure(&[f32]) -> ChannelLevel` — audio-analysis 抽出

`LevelMeter::process` の per-channel 計算を関数化し `LevelMeter` もそれを呼ぶ。probe は slot の channel plane `[..frames]` を直接計測（`PlanarBufferView` 組立て不要）。

### `compile.rs` の binding 規則集約（fix at the right boundary）

- `ScheduledNode::outputs` → `Vec<(PortId, usize)>`（`plan.inputs`/`outputs` と同型）。runtime の `mark_written` ループは `(_, slot)` を読む。
- `InputBinding::effective_slot: Option<usize>` — `scratch ?? (edges.len()==1 → edges[0].slot) ?? None` を compile 時に確定。`NodeIo::input`・materialize・probe resolver が同じ規則を読む（現状 io.rs:114-117 と compile 内の重複を解消）。
- `ScheduledNode::{latency_accum, own_latency}` — `apply_pdc` の `accum`/`own_latency` を正本化（現状 `total_latency` のみ残り per-node は破棄される）。
- `edge.delay_frames` は既存フィールドをそのまま metadata に利用。

### `runtime.rs` の RT 拡張

- `RtState` 追加: `frame_cursor: u64`（process 冒頭で block 先頭位置として読み、終端で `+= frames`）、`probes: ProbeDeck`、probe command `Consumer`。
- `activate_generation` 内で `commands` と同じ drain 位置に probe **generation-scoped** command drain を追加（`Install`/`Rebind` は `cmd.generation == plan.generation` のみ適用・stale は `probe_stats.stale_commands++`）。
- `process()` の activation 直後・event drain 前に **lifetime** command（`SetMode`/`Remove`）を毎 callback drain（staleness 判定なし・probe_id 宛て・plan 内容を含まない）。
- tap 点: (1) graph input = activation/event drain 後・schedule loop 前に `plan.inputs` 全 slot、(2) node input = `materialize` 直後・processor 前、`(3)` node output = `mark_written` 直後、(4) graph output = schedule loop 後・return 前に `plan.outputs` 全 slot。全て `st` RefMut と `&self.arena` の disjoint borrow で成立。
- `GraphRuntime::published_plan: Option<Arc<PreparedPlan>>` — publish 成功時に retain、非 RT のみの `pub fn published_plan(&self) -> Option<&PreparedPlan>` で broker が参照。

## Data and Control Flow

### チャンネル構成（全て既存 `ruviform_rt::spsc::channel`）

| チャンネル | 方向 | 型 | capacity | 満杯時 |
| --- | --- | --- | --- | --- |
| `probe_cmd` | non-RT→RT | `ProbeCommand` | `max_probes*2` | `ChannelFull` を broker へ返す |
| `free` | non-RT→RT | `Box<CaptureChunk>` | `pool_chunks` | broker 補充時 drop+dealloc（非 RT 安全） |
| `chunks_out` | RT→non-RT | `Box<CaptureChunk>` | `pool_chunks` | 構造的に満杯にならない（在飛 ≤ pool）。防御的失敗は working slot 再利用 + fault |
| `telemetry` | RT→non-RT | `ProbeEvent`(Metric/Status のみ・POD 相当) | 明示値（例 512） | drop-newest + per-probe `lost_records++` |

### 主要 flow

1. **capability 列挙**: `ProbeBroker::capabilities()` — spec port × plan binding で `ObservationReport` 列を生成（決定的）。
2. **install**: broker が `point` を `capabilities` と同規則で解決 → `ProbeCommand::Install{generation=plan.generation, binding, mode}` → 次 activation で適用 → `Bound` status（binding 有効）または binding 不在なら `Unbound`。
3. **Monitor block**: tap → stamp 合格 → `MetricRecord` を telemetry push。失敗時 `lost_records++`。
4. **Capture block**: tap → stamp 合格 → `free.try_pop` で chunk 取得（失敗 → `lost_blocks++`）→ `[..frames]` copy + meta 書込 → `chunks_out` push → `capture_left` 減算・0 で `CaptureStopped{BudgetExhausted}` + mode=Monitor。
5. **rebind**: `on_publish` → `Rebind{new_generation, binding, mode}` → activation で原子的適用。activation 後に未 rebind の probe は `Unbound`（観測スキップ）。
6. **remove**: lifetime command → RT で probe を deck から除去 → working chunk は `chunks_out` へ orphan push（collector 側で drop）→ `Retired` status。

## Non-Functional Constraints

- **RT 契約**: tap 経路は `try_borrow`/`try_pop`/`try_push`/整数比較/PCM copy/Peak-RMS 加算のみ。alloc 0・destructor 0・`#[forbid(unsafe_code)]`。
- **bounded**: `max_probes`（例 64）、`pool_chunks`（broker 設定・例 16）、`telemetry` capacity、`CaptureConfig::max_frames` 全て定数上限。chunk メモリ = `pool × max_frames × max_channels × 4B` の明示予算。
- **決定性**: capability 列挙・`ProbeId` 採番（単調）・slot 解決・loss 計数は全て deterministic。
- **OFF 境界**: `active_taps == 0` で tap loop early-skip、`mode==Off` で slot borrow 0。ON/OFF 負荷差は計測して記録する。
- **観測の正直性**: stale slot は `stamp` 監査で排除、stale binding は `bound_generation` guard で排除、delay warm-up 中の合成 zero は engine 正直な信号として観測（discontinuity は rebind status で別途記録）。

## Risks and Trade-offs

- **`chunks_out` 構造的不満杯保証**は「chunk が working slot 外で重複 push されない」実装不変条件に依存 → `#[debug_assert]`/単体テストで担保。
- **`bound_generation` guard の防御深度**: activation drain で gen-scoped command が正しく適用されれば不一致は観測されないが、broker 未同期・command 溢れ時に必須の安全弁として残す。
- **telemetry drop-newest** は monitor 流量が capture を枯渇させないための分離設計。metric の連続性は per-probe `lost_records` で説明可能。
- **`published_plan` の追加**は `GraphRuntime` の非 RT 公開面を 1 メソッド増やす — additive・`Arc` clone のみ・RT 経路に触れない。
- **既知の先行問題（本 Issue では修正しない・別途報告）**: `input_writer`→`process` の plan-read race（境界 slot 解決が publish と競合）、processor が何も書かなくても `mark_written` される不変条件の限界。probe は同じ規則を正直に継承する。

## Deferred Design Concerns

| 論点 | 本設計での扱い | 移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| TimedEvent/Control payload | `Unsupported` 分類 + `TapKind`/`ProbeEvent` の variant 拡張余地 | M3 (#188/#190) | event を audio PCM として誤観測 |
| cross-session probe ID | broker 生存期間内の `ProbeId` のみ | consumer 側 Semantic ID 設計 | UI 再構成で ID 対応が崩れる |
| Recording Writer 共通 primitive | drop/overwrite 許容は観測専用・別契約 | #187 / Synvelis #12 | lossless 契約への緩い保証の混入 |
| fan-out subscription | 単一 `ProbeCollector` pull のみ | 上位層の配信設計 Issue | 複数 consumer 要求時に API 追加 |
| plugin 内部 stage | 公開契約として保証しない | plugin-host 内部 tap 契約 Issue | 内部観測要求を曖昧に残す |

## Verification Notes

- [ ] capability 列挙が AC-1 の 5 分類を mock graph で分離（fixture に `ProbePoint` 宣言 port を含める）
- [ ] `mode == Off` で slot borrow 0・ON/OFF bit 一致
- [ ] 同一 block pre/post Peak/RMS が `ChannelLevel` 契約と一致
- [ ] Capture chunk が全メタデータ封筒を持ち `BudgetExhausted`→Monitor 復帰・`recycle` で pool 復帰
- [ ] free 枯渇・telemetry 満杯・途中停止・generation 更新・遅延 drain で RT 非ブロック + stall 回復後 capture 再開（leak なし）
- [ ] rebind 後に新 plan の正しい slot を観測（wrong-slot 安全性）
- [ ] 非 RT 制御 API の全失敗面が定義済み `ProbeError`
- [ ] `rt_probe_*` で `assert_no_alloc` 窓内 alloc 0（全 mode）
- [ ] `cargo tree` で依存層方向・`mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`verify-ci` 全 green
