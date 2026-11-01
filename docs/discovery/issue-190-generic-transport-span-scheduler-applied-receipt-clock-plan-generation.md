---
id: "DISC-034Qg9A5RpGIykEqJYBwFA"
type: "discovery"
status: "resolved"
title: "Issue #190 Generic Transport Span Scheduler / Applied Receipt / Clock-Plan Generation contract"
created: "2026-11-01"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/synvelis/issues/20"
    - "https://github.com/scarecrow173/synvelis/issues/22"
    - "https://github.com/scarecrow173/synvelis/blob/main/docs/designs/time-transport-midi-automation-plugin-integration-architecture.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/specs/issue-189-rational-beat-variable-ppqn-m3.md"
    - "docs/designs/issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/187"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/186"
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/free-audio/clap/blob/main/include/clap/events.h"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
  deferred-by: []
---

# Issue #190 Generic Transport Span Scheduler / Applied Receipt / Clock-Plan Generation contract

## Summary

Ruviform に汎用の Clock Epoch / Engine Frame / Transport・Plan Generation を
区別する契約、Play/Pause/Stop/Seek/SetLoop の非RT Validate→RT有界
Apply→Applied Receipt、および Transport Context 不連続で Audio Block を
半開 Span に分割する Scheduler を設計・PoC する。現行コードには
`TransportStatus`/`TransportInfo`/`EventList`/`ProcessConfig` はあるが
Epoch/Generation・Span Scheduler・Receipt は存在しない。
仮説（高信頼）: 正本契約は `ruviform-media-core`、plugin wire 変換は
`ruviform-plugin-core` の薄い bridge に置き、min/max Block 制約は非RT
Preflight の Exact-first（不整合は明示 Reject / 明示選択時のみ有界
Degraded）で解く —— 最終決定は ADR。

## Exploration Goal

Issue #190 の問い: Synvelis v0.5 §14（spec 正本）が要求する「論理
Transport Span ≠ 物理 Plugin Process Call」「Canonical Engine Frame 正本・
各 Origin 相対 Offset」「Command→Applied Receipt」「Epoch/Generation 区別」
を、現行 `ProcessConfig.min_frames/max_frames`（L0/L1 差あり）と
`EventList.frame_offset`（process 開始相対）・`TransportInfo` 契約を壊さず
に実現する API・crate 配置・Fault/Budget 方針を決められるか。

## Key Issues and Assumptions

実コード監査で確定した事実（仮説ではない）:

- `EventList.frame_offset` は **process 呼出し開始相対**。`EventList::new`
  は `offset < block_frames`（0-frame は offset==0 flush）と非減少順序を
  boundary で検証する（`plugin-core/src/event.rs`）。Wire 契約は維持する。
- L0 gate（`plugin-host/src/endpoint.rs` `L0Endpoint::process`）は
  `activated.min_frames() ≤ frames ≤ activated.max_frames()` を lock 内で
  Reject。L1 gate は `frames ≤ armed_frame_capacity` のみで **min は
  child-side contract へ委譲**（armed `Staging` が `min_frames` を持たない）。
  `process_audio` bound-clip は `channels` 等号 + `frames ≤ bound.max_frames`。
- `TransportStatus` は `Stopped/Playing/Paused` のみ。`TransportInfo` は
  status + `MediaTime` position + `MediaRange` loop + `MusicalTimeInfo`。
  Seek/Loop Wrap/Device Restart の不連続・Generation を表す型は無い。
- #189 で `BeatPosition`（正規化有理数）・validated `Ppqn`・`TickPosition`・
  Beat 基準 `TempoMap`（RT immutable・exact rational・単回丸め）が確立済み。
  固定 960 前提・`960/3840` リテラル再導入は禁止。
- `ruviform-rt` は**無依存**（`MasterClock` は `(u32,u32)` timebase 保持のみ。
  media-core 非依存は明示コメント）。`ruviform-graph` は `audio-core` のみ依存。
- 層規約（baseline A.2）: engines → 拡張層（plugin-core 等）の直接依存は
  禁止。engine/composition が scheduler を消費するには canonical 契約を
  foundation 層に置く必要がある。`media-core` ↔ `exec` 相互依存禁止。
- TA-008（Plan/Transport/Target/Instance Generation を跨ぐ Event 誤配送
  なし）・TA-009（可変PPQN/Tempo/Loop→Frame/Event 精度・順序）が本 Issue の
  上位 Gate。

## Alternatives and Comparison

### 配置（crate placement）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| canonical 契約+scheduler を `ruviform-media-core` | transport/time 正本・全上位層が依存可（engine/host/#186-#188/#195） | 「値型クレート」に scheduler 機械を足す性格変化 | **有力** |
| 全部 `ruviform-plugin-core` | EventList/TransportInfo/ProcessConfig が揃う | engine→拡張層依存禁止で generic consumer が使えない | 不採用方向 |
| 新規 `ruviform-transport`（media-core のみ依存） | 専用クレート・層が明確 | EventList/TransportInfo/ProcessConfig を参照不可（plugin wire 変換は別 crate 必要）+ 29→30 crate | 有力候補2 |
| 新規 crate（media-core+plugin-core 依存） | 全語彙利用可 | 拡張層順位 = plugin-core 以上になり engine 利用不可 | 不採用方向 |
| `ruviform-rt` | RT 基盤の意味論 | 無依存のため TempoMap 不可・media-core 依存追加は層規約違反 | 不採用 |

plugin wire 変換（span window → `(frames, EventList, TransportInfo)` 物理
call）は wire 型の所有元 `plugin-core` に置くのが層規約上自然。

### min_frames 不整合の解消（ADR 対象）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| ① Preflight Exact-first: capability 未確認 Plugin は「対応済み」と扱わず、適法分割を構成できない command/plan は非RT Reject | 正確・偽装なし・TT-026 一致 | 能力が狭い環境では機能制限 | **有力** |
| ② CLAP `CLAP_EVENT_TRANSPORT` 等 block 内更新を使う | 分割不要で正確 | 実 Plugin 対応・adapter 実装・順序が未検証。Unknown capability を対応済みと誤認できない | 将来の capability 拡張枠として設計に残す（PoC は Unknown→不使用） |
| ③ 明示選択 Live Degraded: 適用を次の適法境界へ延期/量子化し Receipt に actual frame+degradation を記録 | Live 用途で継続動作 | 適用時点がずれる・ユーザー明示が前提 | ①の補完として有力（Offline Exact では拒否） |

### Event occurrence モデル

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| plan が position 基準 event list を持ち、RT scheduler が loop 反復ごとに occurrence を materialize | 無限 loop に対応・source event 一意・occurrence 区別が自然 | span 毎の有界走査と budget が必要 | **有力** |
| compile 時に engine-frame occurrence へ全展開 | RT が単純 | 無限 loop を展開不可 | 不採用 |

## Tentative Conclusions and Hypotheses

**Confirmed（実コード監査済み）:**

- L0/L1 の min_frames gate 差は実在（L0 lock 内 check、L1 は child contract
  委譲で armed ring には min 無し）。`process_audio` bound-clip は
  `channels` 等号と `frames ≤ bound.max_frames` のみ。
- `EventList` の ordering/offset/zero-frame flush 契約は変更不要 —
  scheduler は **u32 offset 列** を正しく生産すれば wire 契約を維持できる。
- `TransportStatus`/`TransportInfo`/`MusicalTimeInfo` ABI は既存 consumer が
  依存しており、本契約は**追加型**として導入し既存型を破壊しない。

**Hypothesis（ADR/設計で確定）:**

- canonical 型 `EngineFrame`/`ClockEpochId`/`TransportGeneration`/
  `PlanGeneration`/`TempoMapRevision`/`LoopOccurrence` は `media-core` に
  置き、`TransportInfo` への snapshot 変換は plugin 側 bridge が行う。
- span scheduler は immutable Prepared Plan（loop region・tempo map・position
  event list・budget）を RT cursor が半開 span に分割。 discontinuity =
  seek/status 変更/loop wrap/plan revision 切替。loop wrap は Clock Epoch
  断絶でも新 TransportGeneration でもなく `LoopOccurrence` 増分。
- Physical call chunking は logical span を capability（min/max・in-block
  transport update 対応）で判定。Exact-first Preflight、未確認能力は
  Reject、Live のみ明示選択で「次の適法境界への延期」を Degraded とし
  actual frame を Receipt に記録。
- Event は canonical `EngineFrame` 正本 + `source_sequence`（同 frame 因果
  順）+ `LoopOccurrence` 区別 + plan/transport generation tag を持ち、
  generation 不一致 event は delivery せず Defined Fault。
- Budget（max spans/callback・events/span・pending applies）は非RT plan
  validation で強制し、RT 超過は Defined Fault（静かな成功禁止）。

## Open Questions

- Degraded 延期の上界（max deferral・drift）と Receipt/UI への表現 —
  design で bounded 値を確定。
- `TransportGeneration` を loop wrap で bump するか（現方針: bump せず
  occurrence で区別 — 設計で再確認）。
- Offline exact render での driver/apply 形態（本 PoC は Mock live driver。
  offline は policy=Exact として Reject 経路を共有）。
- CLAP in-block transport event の adapter 実証（#196 範囲。本 PoC は
  capability=Unknown→使用しない経路のみ）。

## Promotion Candidates

- [x] spec-doc needed: 受入条件 R1〜 + TT-025〜030/TT-006〜010/TT-012 相当の
  受入を Generic Contract として定義。
- [x] adr-doc needed: epoch/generation・half-open span・offset 単一定義・
  min_frames 方針（Exact-first + 明示 Degraded、in-block event は検証時
  のみ）・crate 配置の最終決定。

## Sources and Relations

- 要件正本: [ruviform#190](https://github.com/scarecrow173/ruviform/issues/190)、
  spec 正本 [Synvelis v0.5](https://github.com/scarecrow173/synvelis/blob/main/docs/designs/time-transport-midi-automation-plugin-integration-architecture.md)（§3/§4/§5/§14）。
- 上位: [ruviform#192](https://github.com/scarecrow173/ruviform/issues/192) /
  `docs/designs/synvelis-driven-target-architecture-and-migration.md`（§5.4）/
  `docs/adr/synvelis-driven-generic-execution-boundaries.md`。
- 前提: `docs/specs/issue-189-rational-beat-variable-ppqn-m3.md`（#189）、
  `docs/designs/issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md`（#184）、
  `docs/discovery/issue-194-m0-baseline-audit.md`（M0 実測 baseline）。
- 監査対象コード: `crates/ruviform-plugin-core/src/{processor,event,transport}.rs`、
  `crates/ruviform-plugin-host/src/endpoint.rs`、
  `crates/ruviform-media-core/src/{music,transport,time,error}.rs`。

## Evidence

### 2026-11-02: 現行コード監査（HEAD a62df24）

- `endpoint.rs:407-410` L0 gate: `frames < activated.min_frames() ||
  frames > activated.max_frames() || channels != bound.channels` →
  `Err(HostError::Prepare(PrepareError::UnsupportedConfig))`。
- `endpoint.rs` L1 gate（`L1Endpoint::process`）: `frames >
  leg.armed_frame_capacity() || channels != bound.channels` のみ。
  コメント: "min_frames is deliberately absent — the defined-rejection spec
  delegates it to the child-side plugin contract (the armed `Staging`
  carries no `min_frames`)"。
- `process_audio` bound-clip: `channels != bound.channels as usize ||
  frames > bound.max_frames as usize` → UnsupportedConfig。
- `event.rs`: `EventList::new` は `offset < max(block_frames,1)` +
  非減少順序を検証。`sort_events_by_frame_offset` は stable かつ
  allocation-free merge sort。
- `music.rs`: `Ppqn::new(1..=u16::MAX)`、`BeatPosition{num:i64,den:u32}`、
  `TempoMap::{beat_to_media,media_to_beat,media_to_tick,tick_to_media,
  musical_position,time_info}` は全て RT-suitable（allocation-free exact
  rational + binary search）。
- `transport.rs`（plugin-core）: `TransportInfo{status,position,loop_range,
  musical,reserved}` `Copy` 24B+。`TransportSnapshot`（media-core）も
  `Copy` snapshot。
- `ruviform-rt`: `[dependencies]` なし（`MasterClock` doc: "does not depend
  on ruviform-media-core's time types"）。`ruviform-graph` deps:
  `ruviform-audio-core` のみ（README: rt/engine/plugin 依存は層規約禁止）。
