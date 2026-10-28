---
id: "TSPEC-034MLRBDZVBiSYnXZ5dTrr"
type: "test-spec"
status: "approved"
title: "実規格アダプタ L1 E2E（spawn/handshake/lifecycle/shm block/state defined 応答/destroy + 異常系継承）テスト仕様"
created: "2026-10-26"
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
    - "docs/test-specs/adapter-host-bridge-contract-tests.md"
    - "docs/test-specs/l1-real-plugin-lane-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-114-real-adapter-l1-e2e.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 実規格アダプタ L1 E2E（spawn/handshake/lifecycle/shm block/state defined 応答/destroy + 異常系継承）テスト仕様

> status: approved — Issue #114 の実装（Task 1–5・Full Gate 全 green・`mise run test-l1-real-plugin` の `--ignored` 段で CLAP 3 + VST3 3 E2E 実 pass — 実 fixture cdylib を dlopen する実子プロセス）で契約が実結果により確認済み。Refs #114。`docs/specs/issue-114-real-adapter-l1-e2e.md`（AC1–AC4・AC7）を verify。配置: `crates/ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs`・`crates/ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs` — `#![cfg(feature = "host")]` + 各テスト `#[ignore = "..."]` ゲート（opt-in — `mise run test-l1-real-plugin` が唯一の実行入口）。既存 `l1_proxy_e2e.rs`（`ruviform-plugin-host` crate 側・mock worker）の E2E 形を踏襲し、`run`/`OutOfProcessSpec`/`PluginHost`/wire/`RtLeg` の公開契約は**変更しない**（AC7 — 本 spec は契約維持側の実証を担う）。

## Purpose

実 `.clap`/`.vst3` cdylib を **compile-time 固定 runner**（`required-features = ["host"]`）経由で L1 子プロセスに載せ、`PluginHost`/`OutOfProcessSpec` の公開 API 経由で、L1 の全動詞経路（spawn→handshake→scan→instantiate→lifecycle→shm block processing→state verb→destroy）が既存 L1 契約どおり動くことを実規格バイナリで実証する。目的は (a) mock-only で覆えなかった「実規格 plugin への L1 適用」を実コードで検証する、(b) handshake field-equality が「子は実 scan で合成・親は const literal」という**独立 2 系の値**で実照合されることで、bridge の metadata 写像を E2E 層でも検証する、(c) `Failed`/`InstanceFailed`/pass-through/自動再起動なし の既存異常系契約が実規格バイナリでも維持されることを実証する、(d) wire/export/host/core の公開契約が**本変更で不変**であることを確認する（AC7 の E2E 側証拠）。

## Feature

実規格バイナリ（CLAP/VST3 各 1 fixture・workspace 生成 cdylib）を compile-time 固定 runner 経由で L1 子プロセスに載せ、`PluginHost` 経由で全動詞を駆動する E2E — **規格別 runner・`spec.args` 空・`CARGO_BIN_EXE_*` 解決・fixture 既知値 const pin・既定 lookahead/timeout**。

## Rules

- **runner オーサリング（AC1）**: `ruviform-plugin-clap-l1-runner`/`ruviform-plugin-vst3-l1-runner`（`[[bin]] required-features = ["host"]`）は compile-time 固定の fixture factories を `ruviform_plugin_export::run(factories)` に渡す — argv に plugin path/loader 引数を追加しない（v2 では `run` の argv 規約は `<runner>` + spec.args のみ — `--ruviform-shm-key`/`--ruviform-plugin-id` は除去・id/key は `CreateInstance` payload で受け渡し・`--ruviform-*` 受信は usage reject → exit 2）。fixture は `RUVIFORM_L1_REAL_PLUGIN_DIR`（任意 shell-only override）→ unset 時 `current_exe` sibling の `{DLL_PREFIX}ruviform_plugin_<std>{DLL_SUFFIX}` で解決 — 規格 bundle レイアウト・拡張子改名・任意パス指定はしない（deferred）
- **spawn/handshake（AC2/AC3）**: `PluginHost::register_out_of_process(OutOfProcessSpec::new(EXPECTED, runner))`（`spec.args` 空・既定 `lookahead_blocks=2`/`command_timeout=10s`）→ `execute(Scan)` → `Instantiate` → `Initialize` → `Activate`（`ProcessConfig::new(rate, 1, frames)`）→ `StartProcessing` → N block `host.process` → `SaveState`/`RestoreState` → `StopProcessing` → `Deactivate` → `Destroy` → 子 exit(0)。v2 では `Instantiate` は spawn + `Hello`（version == `wire::WIRE_VERSION`（=2））→ `Scan`（`instance: WIRE_INSTANCE_NONE`・child-scope）→ `CreateInstance{i, plugin_id, shm_key, in_depth, out_depth}` 経由 — いずれか 1 descriptor が `Descriptor::matches(&spec.descriptor)` で field-equality（id/name/vendor/version/features 順序保持 — wire.rs:407）を照合 — **子は bridge が実バイナリ scan から合成・親は test が fixture 既知値で pin**（同一 scan API を両側で呼ばない — 同一関数の同値では検証力を持たない。VST3 `id` は `vst3::uid(...)` bytes から derive — `uid()` は vendored 決定的 const fn で scan 経路ではない・literal は regression pin のみ）
- **既知値 const pin（親側 — test binary は dlopen しない）**: CLAP = `PluginDescriptor { id: "org.ruviform.clap-gain-poc", name: "Ruviform Gain PoC", vendor: "Ruviform", version: "0.1.0", features: &["audio-effect", "stereo"] }`（plugin.rs:197-203）。VST3 = `PluginDescriptor { id: <cid hex>, name: "Gain", vendor: "Ruviform", version: "0.0.0", features: &["Fx"] }` — `id` は component CID の raw 16byte 小文字 hex・**platform 依存** — **expected id は test が `vst3::uid(0x5A1B2C3D, 0x4E5F6A7B, 0x8C9D0E1F, 0x2A3B4C5D)` raw bytes から実行時に derive**（`vst3` は adapter crate の既存 prod dep で test target から参照可・`GainComponent::CID` は `pub(crate)` で tests/ から不可のため `uid(...)` 呼出しで等価 derive・`PluginDescriptor.id` の `&'static str` 化は `String::leak`）・`#[cfg]`-conditional literal は regression pin のみ: Windows `"3d2c1b5a5f4e7b6a8c9d0e1f2a3b4c5d"`・非 Windows `"5a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d"`（`uid()` byte order 由来・component.rs の `GainComponent::CID`）
- **runner/fixture 解決（テスト側）**: runner は `env!("CARGO_BIN_EXE_<runner>")`（same-crate・`#[cfg(feature = "host")]` 内のみ有効 — `--features host` 未指定では `CARGO_BIN_EXE_*` が定義されないため cfg で封じる）・fixture は `current_exe` → `deps/` → `target/<profile>/` または `deps/` sibling の `{DLL_PREFIX}{name}{DLL_SUFFIX}` を `assert!(path.is_file())` で pin（`runner_program()` 同型 — 欠落は fail・silent skip なし）
- **processing データ正当性**: N block `host.process` で delivered block == gain × `lookahead_blocks`（既定 2 = 2 × activated max_frames latency）前の入力 — L1 lookahead/pass-through 意味不変（underrun/結果未着 → 直前受信 else 無音・常に `Ok(CONTINUE)`）。`EventKind::ParamValue{param_id: ParamId(0), value: ParamValue::new(v)}` 込みで規格別期待係数を pin — **CLAP normalized v → plain `2v`**（fixture GAIN plain `[0.0, 2.0]`）・**VST3 normalized v → plain `v`**（plain `[0.0, 1.0]`）
- **state/GUI 動詞 = defined 応答**: `SaveState`/`RestoreState`/`OpenGui`/`CloseGui` コマンドは wire に乗り `Err(HostError::Unsupported)` defined 応答で収束 — bridge は規格 state/GUI 動詞を子プロセス内で**一度も呼ばない**（CLAP `state` ext 未解決・VST3 state verb は `kNotImplemented`/`kResultOk` no-op stub — 呼出し禁止・偽の round-trip 観測を禁止）
- **異常系継承（AC4）**: `PluginHost::kill_child`（composite.rs:810）→ `Failed` 収束 + `HostNotification::InstanceFailed` once per handle（notification を handle 毎に 1 回のみ受信 — 二重 emit なし。**但書**: `Bundled` route（`isolation: Bundled`・default）の子 kill は影響を受けた **各 live instance** へ 1 つずつ `InstanceFailed` が fan-out する N 通知列 — `Dedicated`/単一 instance では 1 通知で退化。粒度正本は `docs/adr/issue-117-bundle-child-failed-granularity.md`）+ 以後の動詞は defined 拒否（`Err(HostError::Failed)` 系）+ pass-through 意味不変 + **自動再起動なし** + 応答なしハングなし（`command_timeout` 10s bound）。binary 不在/dlopen/entry/scan 失敗 → 子 `eprintln!` + `exit(1)` **pre-Hello** → 親 spawn `Err(HostError::Ipc)`（`Failed` 未到達・notification なし — pre-Hello のため handshake 未成立）
- **契約（AC7）**: `run`/`OutOfProcessSpec`/wire/`RtLeg`/`PluginHost`/`plugin-core` の公開契約は #114 の本変更で一切変更しない（#114 時点の記録）。**但書**: Issue #117 の `WIRE_VERSION` 1→2 bump で argv 規約自体が v2 へ変わる（`--ruviform-*` 除去・id/key は `CreateInstance` payload — 本 spec の「不変」は #114 の変更範囲に対する記述であり、#117 の wholesale bump では argv 規約が更新対象）
- **`#[ignore]` + `#![cfg]` ゲート**: 全テスト `#[ignore = "..."]`（理由: fixture cdylib + runner + feature `host` を要求 — `cargo test` 既定経路では実行されない）・ファイル先頭 `#![cfg(feature = "host")]`（feature off で test ファイル自体が空 → `cargo test --workspace` 既定経路に成果物ゼロ・default gate 不変グリーン）

## Examples

- Given CLAP fixture cdylib + runner が `target/<profile>/` に存在、When `mise run test-l1-real-plugin`（`cargo test ... --test l1_real_adapter_e2e -- --ignored`）、Then `PluginHost` 経由で `Scan`（`Scanned` == EXPECTED 含む）→`Instantiate`→`Initialize`→`Activate`→`StartProcessing`→N block `process`（delivered == `0.25×2=0.5` gain × 2 blocks 前 input）→`SaveState`/`RestoreState`→`Err(Unsupported)`→`StopProcessing`→`Deactivate`→`Destroy`→exit(0) が全て `Ok`/defined 応答で往復する
- Given 同 E2E を VST3 fixture で実行、Then identical series — `id` は `vst3::uid(...)` bytes derive の cid hex（regression pin literal: Win `"3d2c1b5a5f4e7b6a8c9d0e1f2a3b4c5d"`/他 `"5a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d"`）・gain は `0.25×1=0.25`（plain `[0.0,1.0]`）— **規格別期待係数を test が pin**
- Given `ParamValue{param_id: ParamId(0), value: ParamValue::new(0.25)}` event を processing 中の 1 block に積む、When `host.process`、Then delivered block が gain 0.5（CLAP）/0.25（VST3）で返る — `ParamValue` が規格 event → 中立 `EventKind` → `ProcessData.inputParameterChanges`（VST3）/`EventBuffer` 内 `ParamValueEvent`（CLAP）の写像で plugin 側 GAIN に届く
- Given `PluginHost` で kill test、When `host.kill_child(handle)`、Then `wait_host_failed` で `Failed` 収束 → `wait_host_notification` で `InstanceFailed` を handle 毎に 1 回受信（`ChildExit` に exit code/signal — `Bundled` route の kill は影響各 live instance 分の N 通知列・`Dedicated`/単一では 1 通知に退化）→ 以後 `process`/動詞は `Err(HostError::Failed)` 系 defined 拒否・pass-through 意味不変・自動再起動なし
- Given `spec.program` を存在しない runner path（`"nonexistent-runner"`）にして `PluginHost` で spawn、Then `Err(HostError::Ipc)` で収束 — `Failed` 収束・notification・ハングは起きない（pre-Hello exit(1) 規約）
- Given `--features host` なしで `cargo test --workspace`、Then `l1_real_adapter_e2e.rs` は `#![cfg]` で空・runner bin は `required-features` でビルド対象外・default gate に一切差分なし（`#[ignore]` も合わせて既定経路で実行されない）

## Guarantees

- 実規格バイナリ（workspace 生成・外部取得なし）が L1 子プロセスで spawn/lifecycle/shm block/state verb/destroy の全経路を defined 応答で往復する — mock では代替不能な「実規格 plugin への L1 適用」を E2E で実証
- handshake field-equality が「子 = 実 scan 合成・親 = const literal」という独立 2 系の値照合で駆動される — bridge の metadata 写像が E2E 層でも検証される
- `Failed`/`InstanceFailed`/pass-through/自動再起動なし/`Err(Ipc)` spawn 拒否/`Err(Unsupported)` state/GUI 応答の**既存異常系契約が実規格バイナリでも維持**される
- wire/export/host/core の公開契約は**本変更で不変**（AC7 — 全動詞・reply 型・lifecycle 遷移は既存どおり消費・変更なし）
- runner/fixture 欠落は `assert!`/spawn `Err` で **fail** — env-gated silent skip なし・`cargo test` 既定経路不変グリーン・`#[ignore]` opt-in lane のみが実行者
- lookahead/pass-through 意味は不変（delivered == gain × lookahead 前 input・underrun は直前受信 else 無音・常に `Ok(CONTINUE)`）

## Non-goals

- in-process bridge 層の独立写像検証は [adapter-host-bridge-contract-tests](adapter-host-bridge-contract-tests.md) が担う — 本 spec は `PluginHost`/`OutOfProcessSpec` 経由の L1 E2E 層のみ
- mise task・CI job・workflow の配置/構成/欠落挙動は [l1-real-plugin-lane-tests](l1-real-plugin-lane-tests.md) が担う
- third-party/外部取得プラグインバイナリ・`.clap`/`.vst3` bundle レイアウト・任意パス指定（`RUVIFORM_L1_REAL_PLUGIN_DIR` は shell/lane の fixture override のみ — 任意プラグイン指定ではない）は対象外（deferred）
- CLAP `state` ext・VST3 `IBStream`/`setComponentState`・GUI・VST3 拡張面（`IConnectionPoint`・note/event bus）は対象外 — state/GUI 動詞は `Err(Unsupported)` defined 応答の pin のみ
- OFX host bridge・video wire・`VideoPluginInstance` の L1 載せは対象外（Issue #118 追跡）
- `Failed`/`InstanceFailed`/`restart`/`pass-through`/`kill_child` の**契約自体の検証**は #113 の既存 E2E（`l1_proxy_e2e.rs`/`plugin_graph_e2e.rs`）が担う — 本 spec は実規格バイナリでの継承確認のみ（重複検証をしない）
- RT 制約・`Send` 契約・`create_sendable` の検証は対象外（bridge unit test spec が default `None` の pin のみ行う）

## Risk

- `#[ignore]` opt-in lane は default `cargo test` で実行されないため、bridge/E2E 欠陥が lane 実行時まで検出不能になり得る — `l1-real-verify` CI job（workflow_dispatch）の手動 dispatch と runbook の `mise run test-l1-real-plugin` 明記で緩和（opt-in の性質上不可避 — spec AC6 整合）
- 実規格バイナリの foreign crash（SEGV/abort — `catch_unwind` 不能）は既存 `Failed` 契約に吸収されるが、子側 crash そのものが L1 分離の検証対象 — `kill_child`/`Err(Ipc)` で継承を実証
- descriptor `id` の platform-order hex を単一 literal で pin すると platform 毎に handshake field-equality が偽 fail — `#[cfg]`-conditional 両 literal で防ぐ
- `spec.args` を誤用して plugin path を argv で渡すと `run` の未知引数 usage reject（exit 2）で spawn が失敗する — `spec.args` 空を pin して規約不変を保つ
- `Err(Unsupported)` の pin を実 state 実装と混同すると（VST3 state stub が `kResultOk` no-op を返すため）偽の state round-trip を主張する — bridge は規格 state 動詞を呼ばないを Rules で固定
