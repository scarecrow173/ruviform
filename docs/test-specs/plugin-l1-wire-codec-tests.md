---
id: "TSPEC-034LNXxXa2vencBWSvgemD"
type: "test-spec"
status: "approved"
title: "plugin L1 ワイヤ codec・ライフサイクル Failed 拡張 テスト仕様"
created: "2026-10-25"
updated: "2026-10-27"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/plugin-l1-out-of-process.md"
  derived-by: []
  verifies:
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/specs/issue-119-l1-wire-version-policy.md"
    - "docs/specs/issue-117-bundle-child-process.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# plugin L1 ワイヤ codec・ライフサイクル Failed 拡張 テスト仕様

> status: approved — Issue #76 の実装（Task 1–6・full gate 全 green）で契約が実結果により確認済み。run 可能コマンドは task 側に定義する（本書は契約と oracle の正本）。

## Purpose

spec `docs/specs/plugin-l1-out-of-process.md`（proposed）・approved design `docs/designs/plugin-l1-out-of-process.md` に基づき、`ruviform-plugin-core` の L1 基盤契約を固定する — (a) host→core へ移設されるプラグイン側契約群（`Plugin`/`PluginFactory`/`ErasedPlugin`/`erase`/`HostInput`/`HostOutput`/`HostError`/`GuiHandle`/`GuiParentToken`/`PluginHandle`）の互換性、(b) `LifecycleState::Failed`/`LifecycleEvent::Failed` の終端準状態遷移表、(c) `wire` モジュールの手書き LE codec（フレーム形式・6 メッセージ・`WIRE_VERSION`・`wire::Descriptor` 所有型・固定 wire レイアウト）。これらは L1 プロキシ（host）とランナー（export）が共有する唯一のプロトコル正本であり、将来の wire 変更・codec 最適化・契約の再移設で退行しないことを保証する。L1 プロキシの E2E 契約（コマンド parity・クラッシュ・pass-through・RT 脚）は `plugin-l1-proxy-contract-tests.md`、runner 側の結合は `plugin-l1-runner-e2e-tests.md` が担当する。ワイヤプロトコルが改訂（`WIRE_VERSION` 更新）された場合かつ同等の代替保証がレビュー済みの場合に限り本 spec を retire する。

## Feature

`ruviform-plugin-core` の L1 共有基盤 3 系: (a) plugin-side contract の所有移転（host が `pub use` で再エクスポートする互換維持付き）、(b) `Lifecycle` 状態機械への `Failed` 終端準状態追加（`LifecycleEvent::Failed` 報告遷移・`is_failed()`・`accepts_processing() == false`）、(c) `wire` モジュール — 親子プロセス間コマンド/ブロック doorbell のフレーム化 codec（`[u32 LE payload_len][u8 kind][payload]`・`WIRE_VERSION`（Issue #117 で v2 へ改訂 — `Command`/`Reply`/`BlockIn`/`BlockDone` payload は `[u64 seq][u32 instance][rest]` envelope・`Hello`/`Shutdown` は instance 非持ち。**但書 — Issue #118 で v3 へ再改訂**: video `Command` tag 12-22・`Reply` tag 4-7・`HostError` tag 10 `InvalidValue` の additive 追加で `WIRE_VERSION` 現行値は 3 — audio 側の envelope・scope 判定表・型 codec・tag 0-11/0-3 は不変で、本 spec の契約記述は v3 下でもそのまま成立する。video 面 codec の pin は `issue-118-wire-v3-codec-tests` が oracle）・`Hello`/`Command`/`Reply`/`BlockIn`/`BlockDone`/`Shutdown`）と `wire::Command`/`wire::Reply`/`wire::Descriptor` 型。

## Rules

- **契約移設の互換性**: `Plugin`/`PluginFactory`/`ErasedPlugin`/`erase`/`HostInput`/`HostOutput`/`HostError`/`GuiHandle`/`GuiParentToken`/`PluginHandle` は `ruviform-plugin-core` が所有し、`ruviform_plugin_host::*` の既存公開パス（`Plugin`・`PluginFactory`・`ErasedPlugin`・`erase`・`HostInput`・`HostOutput`・`HostError`・`GuiHandle`・`GuiParentToken`・`PluginHandle`）は `pub use` 再エクスポートで無変更コンパイル継続 — L0 公開面・挙動は変えない。`PluginHandle::new` は `#[doc(hidden)] pub`（host が mint するが型は共有語彙）。`HostCommand`/`HostReply` は host 側に残る（wire は handle 非修飾のミラー型を別持ち）
- **`HostError` 拡張**: `Failed`（`Failed` インスタンスへのコマンド拒否）・`Ipc`（spawn/handshake/バージョン/transport 失敗・コマンド応答タイムアウト）・`DuplicatePluginId`（L0/L1 同一 plugin_id 重複登録）の 3 variant を追加 — `#[non_exhaustive]` で non-breaking・Display arm を持つ
- **`LifecycleState::Failed`**: 終端準状態 — `(Failed, Destroy) → Destroyed` のみ合法（その他の `Failed` からの遷移は `IllegalTransition`）。`accepts_processing()` は `false`・`is_failed()` は `true` を返す。`Created|Initialized|Activated|Processing → Failed` は `LifecycleEvent::Failed` で到達
- **`LifecycleEvent::Failed`**: 報告遷移（`InitFailed`/`ActivateFailed` 系の命名規約）— host が子プロセス死亡・応答喪失・プロトコル喪失など実行コンテキストの終端失敗を注入する。retry 可能な `InitFailed`/`ActivateFailed` とは意味が異なり terminal である旨を doc が区別する。遷移表は const のまま・`#[non_exhaustive]` で非破壊追加
- **wire フレーム**: `[u32 LE payload_len][u8 kind][payload]` — `payload_len` は payload バイト長（kind・len フィールド自体を含まない）。`Hello { version: u16, capabilities: u32 }` のみが `WIRE_VERSION` を運ぶ（本 spec 確定時の値は 2 — **Issue #118 で現行値は 3**。厳密一致・交渉なしの規則は不変）— 不一致は handshake 失敗（kill + `HostError::Ipc`）
- **wire メッセージ 6 種**: `Hello`（子→親・最初のフレーム）・`Command { seq: u64, instance: u32, cmd: wire::Command }`（親→子・相関 seq + instance envelope）・`Reply { seq: u64, instance: u32, result: Result<wire::Reply, HostError> }`（子→親・`HostError` を直接 encode）・`BlockIn { instance: u32, seq: u64 }`（親→子 doorbell・in-slot `seq % D`）・`BlockDone { instance: u32, seq: u64, result: ProcessResult }`（子→親 doorbell・out-slot `seq % D`）・`Shutdown`（親→子・`HostCommand::Destroy` の wire 意味）。`Command`/`Reply`/`BlockIn`/`BlockDone` payload は `[u64 seq][u32 instance][rest]` envelope・`Hello`/`Shutdown` は instance 非持ち（Issue #117 — 子は N instance を host する bundle 形態。`instance` scope 判定表・`WIRE_MAX_INSTANCES=64`/`WIRE_INSTANCE_NONE=u32::MAX` の詳細は `issue-117-wire-v2-codec-tests` が oracle）
- **`wire::Command`**（`HostCommand` の handle 非修飾ミラー）: `Scan`/`Initialize`/`Activate { config: ProcessConfig }`/`StartProcessing`/`StopProcessing`/`Deactivate`/`SaveState`/`RestoreState { bytes: Vec<u8> }`/`OpenGui { parent: GuiParentToken }`/`CloseGui`/`CreateInstance { plugin_id, shm_key, in_depth, out_depth }`（tag 10）・`DestroyInstance`（tag 11）。v2 では `Instantiate`/`Destroy` が `CreateInstance`/`DestroyInstance` wire 動詞へ写像される（`Instantiate` ↔ spawn + `Scan` + `CreateInstance`・`Destroy` ↔ `DestroyInstance` + live=0 で `Shutdown` + reap）。**但書（v3・Issue #118）**: video 動詞は別系統で追加済み — `CreateVideoInstance`/`VideoInitialize`/`VideoActivate`/`VideoStartProcessing`/`VideoRender`/`VideoStopProcessing`/`VideoDeactivate`/`VideoGetParams`/`VideoGetParamInfo`/`VideoGetParamValue`/`VideoSetParamValue`（tag 12-22）・`Rendered`/`Params`/`ParamInfo`/`ParamValue`（`Reply` tag 4-7）・`HostError::InvalidValue`（tag 10）。audio verb/tag 集合は不変・video codec の pin は `issue-118-wire-v3-codec-tests` が担当
- **`wire::Reply`**: `Ok`/`Scanned(Vec<Descriptor>)`/`State { bytes: Vec<u8> }`/`Opened { gui: GuiHandle }`
- **`wire::Descriptor`**: `PluginDescriptor` の所有 wire 形 — `id`/`name`/`vendor`/`version: String`・`features: Vec<String>`・`From<PluginDescriptor>` とフィールド等価比較を提供（`&'static` → owned 変換を wire 側で完結させ、公開 `HostReply::Scanned` は registration 供給の 'static 値を返す設計と整合 — `Box::leak` なし）
- **`#[non_exhaustive]` 方針**: `wire::Command`/`wire::Reply`/`Message`（enum）は enum レベル `#[non_exhaustive]`（公開 enum 規約）。`wire::Descriptor` は struct のため対象外。wire プロトコル自体は `WIRE_VERSION` handshake で厳格一致 — フィールド追加はバージョン更新とセット
- **固定 wire レイアウト**: `ProcessConfig`/`ProcessResult`/`GuiHandle`/`GuiParentToken`/`PluginHandle`/`HostError` は codec が直接 encode/decode。`TransportInfo`/`Event`/`EventList`/`MediaTime` は repr 固定の wire レイアウト — accessor 経由で encode・ctor で decode（`repr(Rust)` の memcpy 横断はしない）
- **codec の定義済み拒否**: malformed frame（`payload_len` と実バイトの不一致・truncated・未知 kind・上限超過）は panic せず `Err` で拒否 — 部分消費・部分書込みなし。untrusted データの検証は wire 層に置く（codec 境界が untrusted 入力の検証責務を持つ）
- **依存維持**: plugin-core は外部依存ゼロ — codec は手書き LE（serde/postcard は core に入れない — design 棄却）

## Examples

- **`Failed` 遷移表**: `lifecycle_at(Processing).transition(Failed) → Ok(Failed)`・`(Failed, Destroy) → Ok(Destroyed)`・`(Failed, Initialize|Activate|StartProcessing|Process|StopProcessing|Deactivate|SaveState 相当の各 event) → IllegalTransition`・`accepts_processing() == false`・`is_failed() == true`
- **wire roundtrip**: `Command{seq: 7, cmd: Activate{config}}` → encode → decode で同一 seq/config・`Reply{seq: 7, result: Ok(State{bytes})}` で bytes 往復・`Descriptor` が `PluginDescriptor`（'static）→ `wire::Descriptor`（owned）でフィールド等価
- **malformed frame**: `payload_len` 超過の切詰バッファ → `Err`・未知 `kind` byte → `Err`・`payload_len` が `WIRE_MAX_*` 上限を超える宣言 → `Err`（確保しない）
- **`Hello` バージョン**: `version != WIRE_VERSION`（本 spec 確定時 `!= 2` — 現行 `!= 3`）は handshake 失敗 — `wire::Hello` の `version` フィールドが `WIRE_VERSION` 以外で codec 層の decode は受理するが handshake 意味（プロキシ側）では kill + `Ipc`（ proxy contract spec が固定）
- **移設互換**: `ruviform_plugin_host::Plugin`/`PluginFactory`/`ErasedPlugin`/`erase`/`HostInput`/`HostOutput`/`HostError`/`GuiHandle`/`GuiParentToken`/`PluginHandle` の既存 use が変わらずコンパイル — `InProcessHost` の既存テストが全パス

## Guarantees

- wire codec は正規入力で可逆（encode → decode が同値）・malformed 入力で panic せず `Err` — codec 層が untrusted 入力の検証責務を持つ
- `wire::Command`/`wire::Reply`/`wire::Descriptor` は core 公開型 — `#[non_exhaustive]`（enum）・`WIRE_VERSION` は handshake でのみ運用・フィールド追加はバージョン更新とセット
- `Failed` は終端準状態 — `Destroy` 以外の全遷移は `IllegalTransition`・`accepts_processing() == false`・`is_failed() == true` で機械的に区別可能
- 移設型は `ruviform_plugin_host::*` の既存パスでコンパイル継続 — L0 公開面・挙動は不変（`InProcessHost` テスト全パスで実証）
- plugin-core は外部依存ゼロ — codec は手書き LE・serde/postcard は依存木に入らない
- `HostError` の `Failed`/`Ipc`/`DuplicatePluginId` は `#[non_exhaustive]` で non-breaking・Display で観測可能

## Non-goals

- L1 プロキシの E2E 契約（コマンド parity・クラッシュ→`Failed`+通知・pass-through・RT 脚）— `plugin-l1-proxy-contract-tests.md` が担当
- runner（export）の argv/shm attach/メッセージループの結合 — `plugin-l1-runner-e2e-tests.md` が担当
- shm スロットレイアウト・doorbell fence・`WIRE_MAX_*` 適用の process 経路（slot 配置は proxy 側 — codec は型 codec のみ）
- 複数バージョン共存・交渉機構の検証（単一最新バージョン方針は Issue #119 で確定 — `docs/specs/issue-119-l1-wire-version-policy.md`。本 spec の固定対象は現行 `WIRE_VERSION` の契約のみ）
- bundle 子の v2 増分契約 — `instance` envelope 詳細・`WIRE_MAX_INSTANCES`/`WIRE_INSTANCE_NONE`・新動詞 payload bounds・scope 判定表・`HostError::InstanceLimit` wire tag は `issue-117-wire-v2-codec-tests` が oracle（v2 で改訂済み — 本 spec は引き継ぎ frame/kind/型契約を v2 で保持する baseline）
- video L1 面の契約 — video verb codec（`Command` tag 12-22・`Reply` tag 4-7）・`VideoFrameDesc`/`PlainParamInfoOwned` codec・`HostError::InvalidValue`（tag 10）・`WIRE_VIDEO_MAX_*`・`wire::vshm` 定盤は Issue #118 の test-spec（`issue-118-wire-v3-codec-tests` 他）が oracle — 本 spec は audio 側 baseline のまま（`WIRE_VERSION` 現行値 3）
- host/runner の demux・registry・converge 意味論（`issue-117-bundle-failed-granularity-tests`・`issue-117-bundle-e2e-tests` が対象）
- 実プラグイン（CLAP/VST3/OFX）の L1 組込み

## Risk

- codec の encode/decode 非対称・境界条件（0/上限/切詰）での panic や部分消費 → wire 層が壊れプロセス間通信が破損 — roundtrip + malformed ケースで固定
- `&'static` 問題の再発（`PluginDescriptor` を owned にできず leak や unsafe 借用を誘発）→ `wire::Descriptor` 所有型 + `From` + フィールド等価比較で固定
- `Failed` の遷移表誤り（非 `Destroy` event を受理・`accepts_processing`/`is_failed` の誤報）→ 遷移表テストで固定 — L1 の `Failed` 契約の基盤
- 移設の互換破壊（既存 `ruviform_plugin_host::*` パスが変わり consumer を壊す）→ `pub use` 再エクスポートのコンパイル継続を固定 — L0 無退行の保証
- serde/postcard が core の依存木に流入し全 adapter に波及 → core 外部依存ゼロを `cargo tree`/Cargo.toml で固定
- `WIRE_VERSION` の取り違え（メッセージ毎に運ぶ等の設計逸脱）→ `Hello` のみが version を運ぶ形状を固定
