---
id: "SPEC-034MbMNCaEATFCOgSdtwkA"
type: "spec"
status: "proposed"
title: "エンジン/利用側へのプラグイン統合（plugin-host ノード化 + L1 選択配線）"
created: "2026-10-25"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/74"
    - "https://github.com/scarecrow173/ruviform/issues/114"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/engine-plugin-host-integration.md"
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/specs/ofx-host-adapter.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/engine-plugin-host-integration.md"
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/specs/plugin-host-l0.md"
    - "docs/designs/plugin-host-l0.md"
    - "docs/specs/audio-engine.md"
    - "docs/designs/audio-engine.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/plugin-isolation-strategy.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
  derived-by:
    - "docs/designs/engine-plugin-host-integration.md"
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
    - "docs/discovery/issue-114-real-adapter-l1-e2e.md"
    - "docs/specs/issue-114-real-adapter-l1-e2e.md"
    - "docs/discovery/issue-118-video-l1.md"
    - "docs/specs/issue-118-video-l1.md"
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-endpoint-contract-tests.md"
    - "docs/test-specs/plugin-graph-e2e-tests.md"
    - "docs/test-specs/plugin-rt-retirement-tests.md"
  references: []
  defers:
    - "docs/specs/issue-114-real-adapter-l1-e2e.md"
  deferred-by:
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
---

# エンジン/利用側へのプラグイン統合（plugin-host ノード化 + L1 選択配線）

## What Should Be Built

Issue #113 — #76 で host API として完成した L1 アウトプロセス経路（`PluginHost` コンポジット）を、audio エンジンの pull グラフから実際に消費できるようにする。`PluginHost` が `!Send`/`!Sync` で `AudioNode: Send`・コールバック `Send` と型レベルで両立しないため、**non-RT 制御側（`PluginHost` 保持: 登録・コマンド・ライフサイクル・通知・Destroy）と RT 処理側（インスタンス毎の `Send` な処理 endpoint をグラフノードが所有）に所有を分離**する（ADR `docs/adr/plugin-rt-ownership-split.md`）。

1. **プラグインの実グラフ消費** — `PluginHost`/`PluginHandle` 配下のインスタンスを `AudioNode`/`GraphPlan`/`Session` コールバック経路から per-block 駆動（example-only の host 呼出しではない）
2. **non-RT 構成面** — consumer-facing のエンジン構成 API が default L0 と opt-in `OutOfProcessSpec`（`register_out_of_process`・`lookahead_blocks`/`command_timeout`）を選択できる。`ruviform-audio` は plugin-host 型を露出しない（baseline 依存方向ルール）
3. **明示的 prepare + bounded 検証** — ノード投入前に instantiate→initialize→activate→start-processing を明示実行し、scratch/capacity（activated frame 上限・bus 形状）を bounded 検証
4. **error/fallback** — L1 クラッシュ → `Failed` + `HostNotification::InstanceFailed` + 定義済み pass-through（直前受信 or 無音）。遅延未着時も pass-through。自動再起動なし
5. **L1 遅延報告** — `latency_frames = lookahead_blocks × activated max_frames`（frames 整数・秒換算は `frames / sample_rate`）を構成面で報告
6. **退役の非 RT 保証** — node removal/plan-swap で退役した plugin リソースを RT コールバック経路上で drop/dealloc しない
7. **video NONRT 境界の明文化 + 延期** — video plugin（`VideoPluginInstance`/OFX NONRT ノード）の境界を本 Issue 内で明文化し、後続 Issue へ trigger 付き延期。L1 video wire は確立しない

## Why Now

#76 で L1 は host API として完成したが consumer から到達不能（design `docs/designs/plugin-l1-out-of-process.md` L268 deferred 行の trigger = 本 Issue）。プラグインを実グラフで駆動するには `!Send` ホストと `Send` グラフの所有分離を確定させる必要がある — この分離は #74/#76 の host 境界設計が未踏だった engine 統合面であり、先延ばしにすると L0/L1 双方が「使えるが接続できない」状態が続く。

## Users and Value

- **再生ホスト/エンジン利用者** — plugin を audio graph へ実接続でき、L0（in-process・低遅延）と L1（out-of-process・隔離）をインスタンス単位で opt-in 選択できる
- **未検証プラグインの利用側** — L1 の `Failed` + pass-through がグラフ経路でも維持され、クラッシュでも音切れせず継続する
- **RT パス実装者** — 所有分離が型で表れ、RT 側に non-RT 制御状態が漏れないことが `Send` で保証される

## Scope

### In Scope

- プラグインインスタンスの実グラフ消費 — L0（in-process）と L1（`register_out_of_process` 経由）双方のノード化と per-block `process` 駆動
- non-RT 制御 / RT 処理の所有分離 — `PluginHost` は non-RT 専有、RT 側は `Send` endpoint のみ（`Send` 証拠を型で保持）
- non-RT 構成面 — default L0 vs opt-in `OutOfProcessSpec` 選択（`lookahead_blocks`/`command_timeout` を含む）、`DuplicatePluginId` 等の defined rejection 維持
- 明示的 prepare — ノード構築/投入前の lifecycle シーケンス実行と bounded scratch/capacity 検証
- error/fallback — L1 `Failed` + once 通知 + pass-through・遅延未着 pass-through・自動再起動なし
- L1 遅延報告 — `latency_frames = lookahead_blocks × activated max_frames`（frames 整数）を構成面で露出
- 退役保証 — node removal/plan-swap の退役 plugin リソースが RT コールバック上で drop/dealloc しない（`ruviform-rt` の機構または non-RT 最終参照保持）
- mock/device-independent 検証 — `rt_` RT 契約テスト + L0/L1 graph E2E（`l1-test-worker`/参照 runner・mock device）
- video NONRT 境界の明文化 — video plugin 操作が NONRT であり RT パスに置かないこと・L1 video wire 非確立の明記
- crate doc・README・索引の更新

### Out of Scope

- 実規格アダプタ（CLAP/VST3/OFX）経由の実プラグイン動作 — Issue #114（real-adapter work）
- L1 video wire・video plugin の RT/アウトプロセス経路 — deferred（trigger: NONRT video port の分離要求）
- 自動再起動・supervisor ポリシー（#31 確定: 再起動判断は呼出し側/プラグイン責務）
- `ErasedPlugin`/`Plugin` への全域 `Send` bound — opt-in capability のみ（全域化は ADR で rejected）
- `ruviform-audio` → plugin-host 依存・plugin-host → engine 依存（baseline 禁止・相互に型非露出）
- `PluginHost`/`InProcessHost`/`L1Instance` の host 側 API 互換破壊（追加は `#[non_exhaustive]` 方針で）
- プラグイン GUI 配線
- plugin チェーンの複雑なルーティング/自動接続（v1 グラフは順序リストのまま）

## Requirements

- **所有分離**: `PluginHost`（`register_*`・`execute`・`lifecycle_state`・`try_recv_notification`・`kill_child`・`Destroy`）は non-RT 制御側のみ。RT 側はインスタンス毎の `Send` endpoint をノードが所有し、`PluginHost`/`L1Instance`/`ErasedPlugin` の `!Send` 値が `GraphPlan`/`RtPlan`/コールバックに入らないことを型（`Send`）で示す
- **port 経由の engine 接続**: engine が消費するのは下位 port（`Processor`/`RtProcessor`/`AudioNode` 契約）であり、`PluginHost`/`PluginHandle`/`OutOfProcessSpec` 等の plugin 型を `ruviform-audio` 公開 API に露出しない（baseline「engines → 拡張層の直接依存は禁止」）
- **明示的 prepare**: ノードの RT 利用前に instantiate→initialize→activate（`ProcessConfig`）→start-processing を明示実行。scratch/staging/channel capacity は構築時に bounded 確保し、activated frame 上限超の block は defined rejection
- **RT 契約維持**: ノードの `pull`/endpoint の `process` は no-alloc・no-block・try ベース。control→RT の publish（arm/差替）は lock-free SPSC/try-read 系に限定
- **defined failure**: L1 クラッシュ → `Failed` 遷移 + `HostNotification::InstanceFailed` を once で通知 + 以後 pass-through（直前受信 else 無音）。遅延未着・slot busy も pass-through。`Failed` 以後のコマンドは定義済み拒否（L1 design の error/rejection 表と整合）
- **L0 fallback**: L0 経路で RT 側が処理不能な場合（未 `Processing`・contract 外呼出し・endpoint 未到達）の defined 挙動を明示（lifecycle gate 拒否または defined 出力 — panic しない）
- **退役の非 RT**: node removal/plan-swap で退役した plugin リソース（endpoint・staging・SPSC endpoint・plugin object）の Drop/dealloc を RT コールバック経路上で実行しない
- **latency 報告**: L1 選択時の遅延 = `latency_frames = lookahead_blocks × activated max_frames` frames（秒換算は `frames / sample_rate`）を構成面から照会可能（既定 2 blocks × 512 ≈ 21.3ms @ 48kHz）
- **video 境界**: video plugin（`VideoPluginInstance` 全操作 NONRT・OFX NONRT L0）を RT パスに接続する経路は持たず、NONRT 境界と延期（trigger: NONRT video port の分離要求）を明文化
- `rt_` 命名の RT 契約テストを持ち `mise run test-rt` で選択可能。非 RT テスト名に `rt_` を含めない
- 新規公開 enum は `#[non_exhaustive]`。`unsafe` は原則禁止（局所許可は現行の scoped 許可を超えない）

## Acceptance Criteria

- [ ] AC1（実グラフ消費）: `PluginHost` 経由のプラグインインスタンスが `Graph`/`GraphPlan`/`Session`（mock `OutputStream` 駆動コールバック）の内部から per-block で `process` され、テストプラグインで出力正当性（例: gain）が検証される — example-only の host 直叩きではない
- [ ] AC2（所有分離）: `PluginHost` 本体は non-RT 側に残り、グラフ内ノードは `Send` endpoint のみを保持 — ノード/`GraphPlan`/`RtPlan`/コールバック capture が `Send` であることをコンパイル時に示し、`PluginHost`/`L1Instance`/`ErasedPlugin` の参照が RT 側に存在しない
- [ ] AC3（構成面）: non-RT 構成 API が default L0 と opt-in `OutOfProcessSpec`（`lookahead_blocks`/`command_timeout` を含む）を選択でき、`DuplicatePluginId`・`lookahead_blocks=0` 等の defined rejection が維持される。`ruviform-audio` の公開 API・Cargo.toml に plugin-host 型/依存が露出しない（DAG 監査）
- [ ] AC4（明示的 prepare + bounded）: ノード投入前に instantiate→activate→start-processing が実行済みであること、activated capacity 超のブロック/未 prepare 状態が defined rejection（panic/UB/alloc なし）であること
- [ ] AC5（error/fallback）: L1 子強制終了 → `Failed` + `InstanceFailed` 通知 once + 以後 pass-through（直前受信 or 無音）・自動再起動なし。遅延未着/slot busy も pass-through でハングしない。`Failed` 以後コマンドは定義済み拒否。**但書（Issue #117・wire v2）**: per-handle once semantics は維持するが、bundled route（`OutOfProcessIsolation::Bundled`・既定）の子強制終了は影響を受けた**各** live instance へ `Failed` + `InstanceFailed` を 1 つずつ fan-out する N 通知列となる（`Dedicated`/単一 instance では 1 通知に退化 — 粒度正本は `docs/adr/issue-117-bundle-child-failed-granularity.md`）
- [ ] AC6（latency 報告）: L1 選択ノードの報告遅延が `latency_frames = lookahead_blocks × activated max_frames`（frames 整数。秒換算は `frames / sample_rate`）と一致することを構成面から照会して観測できる
- [ ] AC7（RT alloc）: ノード `pull`（L0/L1 endpoint `process` 経路）が `assert_no_alloc` 窓内 0 alloc で `rt_` 命名テストが `mise run test-rt` でグリーン
- [ ] AC8（退役）: node removal/plan-swap で退役した plugin リソースの Drop/dealloc が RT コールバック経路上で発生しない（retire/drain 観測 or non-RT 最終参照の証明）
- [ ] AC9（E2E）: L0 と L1 の graph E2E が mock device / device-independent に成立（L1 は `l1-test-worker` または参照 runner で spawn→lifecycle→process→destroy）
- [ ] AC10（video 境界）: video plugin（`VideoPluginInstance`/OFX NONRT）の NONRT 境界が本 Issue 産物で明文化され、延期 trigger（NONRT video port の分離要求）が記録され、L1 video wire が確立されていない
- [ ] AC11（quality gate）: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` パス・doc 監査で本チェーン起因 findings 0

## Deferred Design Concerns

- video plugin の RT/アウトプロセス経路・L1 video wire | 本 Issue は audio `process` 経路のみ・video port は全操作 NONRT | NONRT video port の分離要求（L1 design L271 と整合） | video plugin は L1/RT 経路を使えない
- OFX NONRT ノードの engine/video 側接続形 | 本 Issue は境界明文化まで | 第 2 ターゲット video engine 統合 | video plugin が engine から到達不能のまま
- 実規格アダプタ・実プラグイン E2E（Issue #114） | runner 形態・実バイナリ依存の別 Issue 級 | 実プラグインでの統合需要 | 検証が mock/test-plugin 範囲に留まる
- graph-attached handle への non-RT verb（state/GUI）の到達経路 | endpoint 分離の形が前提 | 実用上の verb 要求 | graph 内 plugin への一部制御が不可のまま
- plugin chain の routing/自動接続・複数 bus | v1 グラフは順序リストのまま | 実プラグイン構成の多様化 | 手動配線のみ

## Sources and Relations

- Issue [#113](https://github.com/scarecrow173/ruviform/issues/113)（本 spec の対象 Issue）・Issue [#76](https://github.com/scarecrow173/ruviform/issues/76)（L1 実装・CLOSED）・Issue [#74](https://github.com/scarecrow173/ruviform/issues/74)（L0 実装・CLOSED）・Issue [#114](https://github.com/scarecrow173/ruviform/issues/114)（real-adapter 後続・範囲外）
- docs/discovery/engine-plugin-host-integration.md（本 spec の discovery — コード観測・選択肢・所有分離の型強制）
- docs/adr/plugin-rt-ownership-split.md（所有分離のアーキテクチャ決定・proposed — 本 spec の構造前提）
- docs/specs/plugin-l1-out-of-process.md / docs/designs/plugin-l1-out-of-process.md（L1 契約・deferred 行の trigger — 本 Issue が解決する deferred 項）
- docs/specs/plugin-host-l0.md / docs/designs/plugin-host-l0.md（L0 ハンドル/コマンド境界）
- docs/specs/audio-engine.md / docs/designs/audio-engine.md（`AudioNode`/`Graph`/`GraphPlan`/`Session` の engine 面）
- docs/adr/audio-engine-snapshot-swap.md（plan-swap + non-RT retirement 期待）
- docs/adr/plugin-isolation-strategy.md / docs/designs/plugin-isolation.md（L0/L1 階層意味）
- docs/designs/ruviform-architecture-design-baseline.md（依存方向ルール — engines → 拡張層禁止・`Processor` port 経由駆動・A.2）
- docs/adr/realtime-contract-and-verification-policy.md / rt-foundation-primitives-implementation.md（RT 契約・`retire_channel`/`spsc` 機構）
- docs/adr/ofx-host-video-contract-and-c-abi.md（video NONRT 境界の先例）
