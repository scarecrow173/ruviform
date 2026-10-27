---
id: "SPEC-034MHDt79C7KINncq66sE3"
type: "spec"
status: "implemented"
title: "L1 規格アダプタの子プロセス組込み・実プラグイン E2E（CLAP/VST3）"
created: "2026-10-26"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/114"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/118"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
    - "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/ofx-host-video-contract-and-c-abi.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/specs/ofx-host-adapter.md"
    - "docs/specs/rt-verification-lane.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-114-real-adapter-l1-e2e.md"
    - "docs/discovery/plugin-l1-out-of-process.md"
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/discovery/engine-plugin-host-integration.md"
    - "docs/specs/engine-plugin-host-integration.md"
    - "docs/designs/engine-plugin-host-integration.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/plugin-isolation-strategy.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/adr/plugin-architecture-clap-vst3.md"
  derived-by:
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
    - "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
  verifies: []
  verified-by:
    - "docs/test-specs/adapter-host-bridge-contract-tests.md"
    - "docs/test-specs/real-adapter-l1-e2e-tests.md"
    - "docs/test-specs/l1-real-plugin-lane-tests.md"
  references: []
  defers: []
  deferred-by:
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/specs/engine-plugin-host-integration.md"
    - "docs/designs/engine-plugin-host-integration.md"
---

# L1 規格アダプタの子プロセス組込み・実プラグイン E2E（CLAP/VST3）

## What Should Be Built

Issue #114 — #76 で確立した L1 アウトプロセス経路（`run(factories)` runner + `PluginHost` コンポジット proxy）の検証がモックプラグイン範囲に留まる状態を解消し、**規格アダプタ経由の実プラグインバイナリを子プロセスで動かす経路**を実装・実証する。

1. **host 側規格 bridge の実装（CLAP + VST3）** — 実規格バイナリ（`.clap`/`.vst3` 形 shared object）の load → `PluginFactory`/`ErasedPlugin` bridge を adapter が供給する（bridge は子プロセス内で動作 — host 側は L1 wire のみを見る）
2. **実 CLAP・VST3 プラグイン L1 E2E** — host 側 CLAP/VST3 bridge を子プロセスに組込み、`PluginHost` 経由で spawn→handshake→lifecycle（initialize→activate→start/stop processing→deactivate）→shm block process（データ正当性）→state→destroy を両規格で実証
3. **runner バイナリのオーサリング面** — `ruviform_plugin_export::run(factories)` を規格アダプタの bridge factory 構成で exe 化する形（compile-time 固定 registration — ユーザー決定）
4. **実バイナリ依存 CI lane** — 実バイナリ依存 E2E の opt-in lane（専用 mise task + `#[ignore]` ゲート — ユーザー決定。別 workflow ファイルは作らない）

### スコープ確定（2026-10-26 ユーザー決定 — briefing 後の binding 判断）

- **規格スコープ = CLAP + VST3 両方** — discovery 軸 3 の「CLAP 先行・VST3/OFX は条件付き/後続」lean を上書き。host-side bridge・実バイナリ E2E を両規格で本 Issue に含める。OFX は video・NONRT・`VideoPluginInstance` 構造上 L1 に載らないため **Issue [#118](https://github.com/scarecrow173/ruviform/issues/118) で追跡**（discovery 軸 4 (a) 相当・trigger 付き延期）
- **runner オーサリング = compile-time 固定** — `run(factories)` の static-registration 形を維持。argv/`--plugin-path` による実行時動的ロードは rejected alternative（discovery 軸 1 (a) 採用・(b)(c) 棄却）
- **CI lane = 専用 mise task + `#[ignore]` ゲート** — 実バイナリ lane は専用 task で opt-in 実行。別 workflow ファイルは作らない（discovery 軸 5 の (b) 系 opt-in 形で確定 — 詳細メカニズムは design）

## Why Now

#76 で L1 は host API として完成し（design L272 deferred 行の trigger = 本 Issue）、#113 で engine 側の endpoint・graph E2E も確立した。残る未検証面は「実プラグイン」そのもの — 現行検証は `GainPlugin`/`l1-test-worker`/`UnderwritePlugin`/`PanicPlugin` のモックに限定され、**規格 ABI を経由した実バイナリが L1 wire に乗る経路は未証明**のまま。adapter 側に host 方向の bridge が存在しない以上、この経路を開くのは本 Issue の配線であり、後続の engine 統合・実利用の前提になる。

## Users and Value

- **プラグイン利用者（host 側）** — 実 CLAP/VST3 プラグインを L1 分離経路で使えることが実証され、未検証バイナリのクラッシュ隔離が実バイナリで確認できる
- **規格アダプタ実装者** — host 方向 adapter（binary → `ErasedPlugin` bridge）と runner オーサリング形が確立し、規格毎の追加が同型になる
- **検証・CI 運用者** — 実バイナリ依存の検証 lane が方針として定義され、mock-only 検証との棲み分けが明確になる

## Scope

### In Scope

- runner バイナリのオーサリング面 — 規格アダプタが供給する `PluginFactory`/`ErasedPlugin` bridge を `run(factories)` で exe 化する **compile-time 固定** の runner bin（規格毎）。`run` の argv 規約（`--ruviform-*` bootstrap flag・未知引数の defined rejection）は不変に維持し、プラグインバイナリ指定は runner 内の compile-time 確定解決（実行時引数注入は行わない）
- host 側 CLAP bridge の実装 — 実 CLAP バイナリ（workspace 生成の `ruviform_plugin_clap` cdylib）を子プロセスで load・instantiate し `PluginFactory`/`ErasedPlugin` に bridge する部品（`clack-host` — dev-dep → production dep 昇格は依存追加規約の理由記録に従う）
- host 側 VST3 bridge の実装 — 実 VST3 バイナリ（workspace 生成の `ruviform_plugin_vst3` cdylib）を子プロセスで load・instantiate し `PluginFactory`/`ErasedPlugin` に bridge する部品（`vst3` crate の raw COM bindings 直駆動 — host 側 loader API を持たないため adapter が最小 host を実装する。dlopen は `libloading`）
- 実 CLAP・VST3 プラグインの L1 E2E — `PluginHost`/`OutOfProcessSpec` 経由で spawn→`Hello`/`Scan` handshake（descriptor 整合）→lifecycle コマンド往復→shm block process（gain 等のデータ正当性）→state 動詞の defined 応答往復→`Shutdown`/Destroy で exit(0) を両規格で実証。crash/異常系は既存契約の維持を確認（`Failed` + once 通知 + pass-through・自動再起動なし）
- OFX 境界の明文化 — `VideoPluginInstance`（全操作 NONRT・video wire 非確立）が `run(factories)`/audio shm 面に載らない構造的境界を本 Issue 産物で明文化し、Issue [#118](https://github.com/scarecrow173/ruviform/issues/118) で追跡することと trigger（NONRT video port の分離要求）を記録
- 実バイナリ依存 CI lane の設計と実装 — 専用 `mise.toml` task（実バイナリ・runner の build + `#[ignore]` ゲート済みテストの `--ignored` 実行）と、必要に応じた lint 用 task の追加・`.github/workflows/ci.yml`（既存 workflow_dispatch ファイル）内への job 追加 or 手動 lane のみの判断記録。別 workflow ファイルは作らない
- crate doc・README・索引（`crates/README.md`）の更新

### Out of Scope

- L1 video wire の確立（`VideoPluginInstance`/`render` を wire/shm に載せる拡張 — #113 design 決定 `k` で非確立・deferred 継続）
- OFX プラグインの L1 E2E・OFX host-side の `ErasedPlugin` bridge（video・NONRT 境界により L1 非対応 — Issue [#118](https://github.com/scarecrow173/ruviform/issues/118) で追跡）
- 実行時プラグインバイナリ指定（argv `--plugin-path` 等による loader 型 runner・`run` argv 規約拡張・`OutOfProcessSpec.args` 経由のプラグイン path 受け渡し — ユーザー決定で compile-time 固定を採用。rejected alternative として design に記録）
- third-party/外部取得プラグインバイナリでの E2E（本 Issue の実バイナリは workspace 生成物/fixture — 外部取得が要る場合は lane 設計の前提が変わるため別論点）
- `ruviform-plugin-clap`/`-vst3`/`-ofx` の既存 adapter 内部実装の変更（host 方向 bridge の新設は別部品として追加 — 既存の plugin-side adapter・OFX host adapter の公開契約を破壊しない）
- L1 wire/host/runner 基盤の変更（wire プロトコル・shm レイアウト・`L1Instance`/`RtLeg` は #76 確定済みのまま — 変更が要る場合は design で根拠を記録）
- engine/利用側への接続（#113 で確立済み — 本 Issue は host/export 側の runner+adapter+E2E に留まる）
- L1 GUI（`OpenGui`/`CloseGui` のプロセス越え配線 — 継続 deferred）
- 自動再起動・supervisor ポリシー（#31 確定判断のまま）
- `#[non_exhaustive]` 方針・`unsafe` 原則禁止の緩和（foreign ABI load が `unsafe`/`libloading` を要する場合は局所許可 + SAFETY コメントの scoped 許可で処理 — 既存先例と同型）

## Requirements

- **実バイナリ経路の実証（CLAP + VST3）**: E2E は真の規格バイナリ（workspace 生成の `.clap`/`.vst3` 形 shared object — compile-time typed entry のみに留まらない file-load/dlopen 経路）を両規格で対象とし、`PluginHost` 経由の spawn→lifecycle→process→destroy がデータ正当性（モックでない規格 ABI 経由の処理出力）を持つこと
- **wire/host/runner 契約維持**: wire プロトコル・shm レイアウト・`argv` 規約（`--ruviform-shm-key`/`--ruviform-plugin-id`）・`OutOfProcessSpec` 面・`Failed`+通知+pass-through・自動再起動なしの契約を変えない。runner は compile-time 固定 registration（`run` argv 規約の拡張・`spec.args` 経由のプラグイン path 受け渡しは行わない — ユーザー決定）
- **descriptor 整合**: `OutOfProcessSpec.descriptor` と子の `Scanned` 報告の field-equality が実プラグイン経由でも維持される（adapter が実バイナリ metadata から `PluginDescriptor` を合成する経路の存在）
- **異常系契約維持**: 実プラグインの panic → `Err(HostError::PluginFailed)` で子存続・子プロセス強制終了 → `Failed` + `HostNotification::InstanceFailed` once + 以後定義済み拒否・pass-through（直前受信 or 無音）・応答なしハングなし。規格バイナリ不在・load 失敗・instantiate 失敗は defined rejection（spawn `Err(HostError::Ipc)` 系）で収束しハングしない
- **RT 契約維持**: L1 の RT 側読取り経路（`RtLeg`/endpoint）に変更を加えない — `rt_` 命名テストが `mise run test-rt` でグリーン。子内の foreign ABI オブジェクトに `Send` は要求しない（`create_sendable` は L1 経路で不要求）
- **層・依存規約**: adapter bridge の配置は baseline A.2 許可依存内（adapter crate → `ruviform-plugin-export`/`-host`/`-core`）。`clack-host` の dev-dep → production dep 昇格・`libloading` 等新規依存はメンテナンス状況・ライセンス・依存木サイズの理由記録に従う（採否記録が要る場合は design/plan フェーズの責務）
- **`unsafe` 方針**: foreign ABI load/instance 生成が要する `unsafe`/`libloading`/COM vtable 呼出しは局所許可（`#[expect(unsafe_code)]` + SAFETY コメント）で封じ込め、既存 scoped 許可の先例（OFX `ffi/loader.rs` 等）を超えない
- **CI lane**: 実バイナリ依存 E2E は専用 `mise.toml` task + `#[ignore]` ゲートの opt-in 形とし（ユーザー決定）、実バイナリが存在しない環境で lane が誤検出・silent skip しない設計（欠落時は skip でなく fail）。別 workflow ファイルは作らない。`workflow_dispatch` 運用は維持
- 新規公開 enum は `#[non_exhaustive]`。`rt_` 命名の RT 契約テスト規約を維持（非 RT テスト名に `rt_` を含めない）

## Acceptance Criteria

- [ ] AC1（runner オーサリング面確定）: `run(factories)` を規格アダプタ供給の実プラグイン bridge factory 構成で exe 化する **compile-time 固定** runner bin が規格毎（CLAP・VST3）に存在する — `run` argv 規約（bootstrap flag・未知引数 reject）との整合が保たれ、実行時プラグイン指定（argv/`spec.args`）を持たない
- [ ] AC2（実 CLAP L1 E2E）: `PluginHost`/`OutOfProcessSpec` 経由で実 CLAP バイナリを載せた子プロセスが spawn→`Hello`/`Scan` handshake→Instantiate→lifecycle 往復→shm block process→state 動詞→Destroy で完走し、規格 ABI 経由の実処理出力（gain 等）のデータ正当性が検証される — `l1_proxy_e2e` 同型の device-independent な host 側 E2E として
- [ ] AC3（実 VST3 L1 E2E）: 同構成で実 VST3 バイナリを載せた子プロセスが同じ経路を完走し、規格 ABI（`GetPluginFactory`→`IPluginFactory`→`IComponent`/`IAudioProcessor` COM 経路）の実処理出力のデータ正当性が検証される
- [ ] AC4（異常系維持）: 実 adapter 構成の子が既存契約を維持 — 子強制終了 → `Failed` + `InstanceFailed` once + pass-through・自動再起動なし・応答なしハングなしが観測できる（既存 `l1-test-worker`/`l1_runner_e2e` の異常系と同契約）。規格バイナリ不在時は spawn 系の defined rejection で収束しハング・silent skip しない
- [ ] AC5（OFX 境界明文化）: `VideoPluginInstance`（NONRT・video wire 非確立）が `run(factories)`/audio shm 面に載らない構造的境界が本 Issue 産物で明文化され、Issue #118 で追跡することと延期 trigger（NONRT video port の分離要求）が記録される
- [ ] AC6（CI lane）: 専用 `mise.toml` task が実バイナリ・runner の build と `#[ignore]` ゲート済みテストの `--ignored` 実行を担い、`.github/workflows/ci.yml` への反映（既存ファイル内 job 追加 or 手動 lane のみ）の判断記録が存在する — 別 workflow ファイルなし・実バイナリ欠落時は fail であって silent skip しない設計
- [ ] AC7（契約維持）: `run`/`OutOfProcessSpec`/wire/`RtLeg` の公開契約に互換破壊がなく、新規公開 enum は `#[non_exhaustive]`、`unsafe` は scoped 許可の範囲内
- [ ] AC8（quality gate）: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` パス + opt-in lane task パス・doc 監査で本チェーン起因 findings 0

## Deferred Design Concerns

- video/NONRT プラグイン（OFX `VideoPluginInstance`）の L1 経路 | video port は全操作 NONRT・wire/shm は audio 専用（#113 design 決定 `k`） — **Issue [#118](https://github.com/scarecrow173/ruviform/issues/118) で追跡（ユーザー決定）** | NONRT video port の分離要求（L1 design L271 と整合） | video plugin は L1 不可
- 実行時プラグインバイナリ指定（loader 型 runner・argv/`spec.args` 経由 path 受け渡し） | ユーザー決定で compile-time 固定 registration 採用・`run` argv 規約不変 | 実行時にプラグインバイナリを差し替える要求（third-party スキャン等） | runner は組込み fixture のみを serve し続ける
- third-party/外部取得プラグインバイナリ E2E | 本 Issue の実バイナリは workspace 生成物/fixture | 外部バイナリ取得・固定の要求発生 | fixture のカバレッジが自前 plugin に留まる
- `.clap`/`.vst3` 規格準拠配布形（bundle・拡張子改名・ディレクトリ構造）の再現 | raw cdylib dlopen で E2E は成立する | 規格準拠ホストとしての発見経路検証要求 | bundle 経路は未検証のまま
- L1 GUI（`OpenGui`/`CloseGui` プロセス越え） | 規格・プラットフォーム固有課題・L1 design 継続 deferred | L1 インスタンスへの GUI 要求 | `Unsupported` 拒否のまま
- runner_program 等 `target/<profile>/` path 解決の強化（Issue #123） | #114 E2E ハーネスも同一解決に依存 | #123 が解決形を決定 | 現行の `current_exe` 相対解決に依存し続ける

## Sources and Relations

- Issue [#114](https://github.com/scarecrow173/ruviform/issues/114)（本 spec の対象 Issue）・Issue [#76](https://github.com/scarecrow173/ruviform/issues/76)（L1 実装・CLOSED）・Issue [#113](https://github.com/scarecrow173/ruviform/issues/113)（engine 統合・CLOSED）・Issue [#118](https://github.com/scarecrow173/ruviform/issues/118)（OFX/video 側の追跡先）
- docs/discovery/issue-114-real-adapter-l1-e2e.md（本 spec の discovery — コード観測・選択肢・実バイナリ在否・adapter 現状）
- docs/designs/issue-114-real-adapter-l1-e2e.md（本 spec の design — bridge 配置・runner/fixture/lane 構成の確定）
- docs/specs/plugin-l1-out-of-process.md / docs/designs/plugin-l1-out-of-process.md（L1 契約・runner 形態確定・deferred 行の trigger — 本 Issue が解決する deferred 項）
- docs/specs/engine-plugin-host-integration.md / docs/designs/engine-plugin-host-integration.md / docs/discovery/engine-plugin-host-integration.md（#113 確立の endpoint/graph 面・video 境界決定 `k`・#114 参照 deferred）
- docs/adr/plugin-isolation-strategy.md / docs/designs/plugin-isolation.md（L0/L1 階層意味・規格アダプタ層の独立性）
- docs/adr/plugin-architecture-clap-vst3.md（ホスト・制作双方の規格アダプタ方針）
- docs/adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md（clack/vst3 binding 方式 — `clack-host` の host 側利用はこの ADR の binding 採否とは別の依存方向論点）
- docs/adr/ofx-host-video-contract-and-c-abi.md / docs/specs/ofx-host-adapter.md（OFX host adapter の video・NONRT 境界 — L1 前提としない）
- docs/adr/realtime-contract-and-verification-policy.md（RT 契約・`rt_` lane）
- docs/designs/ruviform-architecture-design-baseline.md（adapter 層・許可依存）
