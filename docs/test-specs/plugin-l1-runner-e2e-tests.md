---
id: "TSPEC-034LNXHUv8qjXSdWNUU2Ps"
type: "test-spec"
status: "approved"
title: "plugin L1 runner（ruviform-plugin-export）E2E テスト仕様"
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
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
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

# plugin L1 runner（ruviform-plugin-export）E2E テスト仕様

> status: approved — Issue #76 の実装（Task 2・Task 6 full gate 全 green・`l1_runner_e2e` 11 tests）で契約が実結果により確認済み。Issue #119 で shm header `wire_version` 拒否の pin を +1 追加（計 12 tests）。run 可能コマンドは task 側に定義する（本書は契約と oracle の正本）。

## Purpose

spec `docs/specs/plugin-l1-out-of-process.md`（proposed・AC6）・approved design `docs/designs/plugin-l1-out-of-process.md` に基づき、`ruviform-plugin-export` クレートの受入契約を固定する — (a) workspace member・公開 `run(factories)` エントリ・crate doc/README、(b) 薄いランナーの `<runner>` + spec.args → `Hello` → `CreateInstance`（registry insert + shm attach）→ メッセージループ → `ErasedPlugin` per-instance dispatch 経路（v2 — 子は `instances` registry で N instance を host。現行 wire v3 — Issue #118 で video 系統が別 surface として追加・audio runner の規約は不変）、(c) 内蔵モック `GainPlugin` を持つ参照 runner `ruviform-plugin-l1-runner` の結合（`CARGO_BIN_EXE_*` spawn → lifecycle → shm ブロック処理 gain 正当性 → `Shutdown` exit(0)）。本 spec は子プロセス側（export）の **audio** runner 契約 — proxy 側の `Failed`/pass-through/RT 脚は `plugin-l1-proxy-contract-tests.md`、wire codec/`Failed` 遷移表は `plugin-l1-wire-codec-tests.md` が担当する。export が廃止・runner が再設計された場合に限り本 spec を retire する。

## Feature

`ruviform-plugin-export` クレート（workspace member — 総計 21）: 子プロセス側の薄いランタイムで、`pub fn run(Vec<Box<dyn PluginFactory>>) -> std::process::ExitCode` が audio 側の公開エントリ（**但書 — Issue #118 で video 側公開エントリ `pub fn run_video(Vec<Box<dyn VideoPluginFactory>>) -> ExitCode` が追加済み**: 同一の framed pipe・`Hello`/`Scan`/`Shutdown` handshake・argv 拒否・exit 規約・panic 正規化を共有し、差分は video verb 系統（`CreateVideoInstance` + tag 12-22）・`BlockIn`/`BlockDone` doorbell 不在・per-instance `vshm` attach のみ — video runner の pin は `issue-118-video-e2e-tests` 系が担当）。参照 runner `src/bin/ruviform-plugin-l1-runner.rs` は内蔵モック `GainPlugin` を持つ fixture — `CARGO_BIN_EXE_*` で実プロセス spawn し、最小 host 実装（test 側で codec/shm を組み立て）経由の E2E を検証する（video 側の参照 runner `ruviform-plugin-l1-video-runner` は #118 で追加 — `InvertVideo`/`PanicVideo`/`OverLimitVideo`/`OverLimitInfoVideo` の 4 fixture）。

## Rules

- **workspace/公開面**: `crates/ruviform-plugin-export` が workspace member（`members` に追加・総計 21 クレート）・`publish.workspace = true` 継承（未 publish 継続）・`[lints] workspace`・crate README が設置される。`pub fn run(factories: Vec<Box<dyn PluginFactory>>) -> std::process::ExitCode` が audio 側公開エントリ（**#118 で `run_video` が parallel な video 側公開エントリとして追加 — 本 spec の pin 対象は `run` のまま**）— `Box<dyn PluginFactory>` でアダプタ非依存（静的 generic ではなく型消去済み trait object — core `erase`/`ErasedPlugin` と整合）
- **argv 規約（v2 確立・v3 でも不変）**: `<runner>` + `spec.args` のみ — `--ruviform-plugin-id`/`--ruviform-shm-key` は除去・id/key は `CreateInstance`（video は `CreateVideoInstance`）payload で受け渡し。未知引数・`--ruviform-*` 受信は定義済み拒否（usage/`Err` で非ゼロ終了・ハングしない）— `run_video` も同一規約
- **`Hello` 起動**: `Hello { version: WIRE_VERSION, capabilities }` が子→親への最初のフレーム（起動直後に送信）
- **`CreateInstance` + shm attach/header 検証 + registry**: `CreateInstance{i, plugin_id, shm_key, in_depth, out_depth}` → `plugin_id ∈ factories`（不在→`Err(UnknownPluginId)`）→ `registry.len() < WIRE_MAX_INSTANCES`（超過→`Err(InstanceLimit)` — capacity は dup より先に判定: 満杯 registry は全合法 handle を既登録として保持するため dup 先行では `InstanceLimit` が到達不能・`export/lib.rs` `create_instance` の実装順が正本）→ `i` 未登録（dup→`Err(Ipc)`）→ `shm_key` で named segment attach + `[header][in-slots][out-slots]` header 検証 + 宣言 depth 照合（失敗→`Err(Ipc)`）→ `instances: HashMap<u32, InstanceEntry>` registry へ insert（子は `instances` registry で N instance を host する — singleton ではない）。shm アクセスは内部 `shm` モジュールに閉じ込め（`shared_memory` の `as_slice`/`as_mut_slice` のみ・自前 `unsafe` ゼロ）
- **シングルスレッドメッセージループ（instance 解決 dispatch）**: stdin frame → `wire::Command{seq, instance, cmd}` を解決（`Scan` → factories・`CreateInstance`/`DestroyInstance` → registry 操作・他 instance 動詞 → `instances[i]`（未登録→`Reply{i,seq,Err(InvalidHandle)}` flagged・子は継続））→ `Reply { seq, instance, result }` を返す（`Reply.result` は `Result<wire::Reply, HostError>`・echo は verbatim）。`BlockIn { instance, seq }` → `Lifecycle` が `Processing` を確認後、shm in-slot を `BufferView`・out-slot を `BufferViewMut` として `process_host` を呼出し `BlockDone { instance, seq, result }` を返す。`Shutdown`/stdin EOF → 全 entry の lifecycle 正規化（`Deactivate`/`Destroy` 相当）→ exit(0)
- **`ErasedPlugin` dispatch**: `Scan` → `Scanned(Vec<wire::Descriptor>)`（factory scan → `From<PluginDescriptor>` で owned 変換）・`Initialize`/`Activate`/`StartProcessing`/`StopProcessing`/`Deactivate` → `Ok` または `Err`・`SaveState`/`RestoreState` → `State{bytes}`/`Ok`・`OpenGui`/`CloseGui` → `Opened{gui}`/`Ok`（実装が返すまま — L1 側では proxy が `Unsupported` に変換）。違法遷移は `Lifecycle` が `Err(HostError::Lifecycle(_))` を返す — wire `Reply` で送る（panic しない）
- **panic → `HostError::PluginFailed`**: `ErasedPlugin` dispatch（コマンド・`process_host`）の panic は catch して `Err(HostError::PluginFailed)` として `Reply`/`BlockDone` で返す — 子プロセスを殺さない（デフォルトの unwind を掴む — process abort ではなく `Err` に変換）
- **`process_host` 適合**: `Lifecycle` が `Processing` 以外で `BlockIn` → `Err(HostError::Lifecycle(_))`（`process_host` は L0 と同一の `Lifecycle` 判定を使う）。shm slot の `{frames, channels, event_count, flags, transport, events, samples}` を `BufferView`/`BufferViewMut` へ適合（planar `f32`・`WIRE_MAX_*` 上限内）
- **依存**: `ruviform-plugin-core`（path — wire/契約/`ErasedPlugin`）+ `shared_memory = "0.12"`（子側 shm attach — internal `shm` モジュールに閉じ込め・採用記録は design/plan「依存の確定」どおり）。runner 自身の unsafe はゼロ
- **参照 runner `ruviform-plugin-l1-runner`**: 内蔵モック `GainPlugin`（core `Plugin` 実装 — planar `f32` で `gain` を乗算・`PluginDescriptor` 保持・`Initialize`/`Activate`/`StartProcessing`/`SaveState`/`RestoreState`/`Deactivate`/`Destroy` の state 往復・param event 反映）を持ち `run` を呼ぶ fixture — `CARGO_BIN_EXE_ruviform-plugin-l1-runner` で spawn
- **`CARGO_BIN_EXE_*`**: 統合テスト（`tests/`）でのみ有効 — `env!("CARGO_BIN_EXE_ruviform-plugin-l1-runner")`/`option_env!` で crate 内に閉じる（cross-package の bin 解決に依存しない）

## Examples

- **正常系 E2E**: 最小 host 実装が `ruviform-plugin-l1-runner` を spawn（`<runner>` + spec.args）→ `Hello` → `Scan`（`Scanned` の `wire::Descriptor` が `GainPlugin` descriptor と等価）→ `CreateInstance{i, "gain", shm_key, 3, 3}`（registry insert + shm attach）→ `Initialize`/`Activate`/`StartProcessing` → shm in-slot に既知入力を書き `BlockIn` → `BlockDone` → out-slot が gain 乗算後の値を持つ（gain 正当性）→ `SaveState`/`RestoreState` の往復 → `StopProcessing`/`Deactivate` → `DestroyInstance` → `Shutdown` で exit(0)
- **argv 異常**: `--ruviform-plugin-id`/`--ruviform-shm-key` を受け渡した場合（v2 では除去）・未知引数 → usage/`Err` で非ゼロ終了・`Hello` を送らずハングしない
- **registry/定義済み拒否**: 未登録 `i` 宛 `BlockIn`・instance 動詞 → `Reply{i,seq,Err(InvalidHandle)}` flagged・子は継続。`CreateInstance` で `plugin_id` 不在 → `Err(UnknownPluginId)`・`len ≥ 64` → `Err(InstanceLimit)`・`i` dup → `Err(Ipc)`・shm attach/header/depth 不整合 → `Err(Ipc)` — いずれも defined rejection で子は生存
- **shm header 不一致**: `CreateInstance` の `shm_key` が存在しない・header magic/`wire_version`/形状が不正・宣言 depth 不整合 → attach/header 検証で `Err(Ipc)`（定義済み拒否 — `wire_version` 不一致は `Hello.version` と同一の厳密一致契約、Issue #119）
- **panic → `PluginFailed`**: `process_host`（または `Initialize` 等）を panic する `ErasedPlugin` → catch され `Reply`/`BlockDone` が `Err(HostError::PluginFailed)` — 子は終了せず後続コマンドを受理し続ける（プロセス死亡ではない）
- **非 Processing で `BlockIn`**: `StartProcessing` 前に `BlockIn` → `Reply`/`BlockDone` が `Err(HostError::Lifecycle(_))`（L0 と同一意味）
- **version 不一致**: `Hello.version != WIRE_VERSION` → 親（test 側最小 host）が handshake 失敗として kill（proxy contract spec と整合 — runner 自体は version を送るだけ）

## Guarantees

- `ruviform-plugin-export` は workspace member・`run` が audio 側公開エントリ（`run_video` は #118 で追加の video 側 parallel エントリ — 本 spec の対象外）— `Box<dyn PluginFactory>` でアダプタ非依存
- `<runner>` + spec.args → `Hello` → `CreateInstance`（registry insert + shm attach/header 検証）→ instance 解決 dispatch のメッセージループ経路が定義済み — argv/header/registry 異常で defined rejection・ハングしない
- `wire::Command` が `ErasedPlugin` per-instance dispatch へ写像（`instances` registry）— `Reply.result` が `Result<wire::Reply, HostError>`・違法遷移は `Err(Lifecycle(_))` を wire で返す・echo は verbatim・`Shutdown`/EOF で全 entry 正規化 → exit(0)（panic しない）
- プラグイン panic は `Err(HostError::PluginFailed)` として `Reply`/`BlockDone` で返る — 子プロセスは生存し後続を受理（process abort ではない）
- `BlockIn` は `Lifecycle` が `Processing` のときのみ `process_host` — shm in/out slot を `BufferView`/`BufferViewMut` に適合し `BlockDone` を返す
- shm アクセス・`shared_memory` は内部 `shm` モジュールに閉じ込め — `as_slice`/`as_mut_slice` のみ・自前 `unsafe` ゼロ
- `ruviform-plugin-l1-runner`（`GainPlugin` fixture）は `CARGO_BIN_EXE_*` で spawn 可能 — `tests/` 内に閉じる

## Non-goals

- proxy/host 側の `Failed` 遷移・pass-through・RT 脚（`plugin-l1-proxy-contract-tests.md` が担当）・wire codec/`Failed` 遷移表自体（`plugin-l1-wire-codec-tests.md` が担当）
- 実プラグイン（CLAP/VST3/OFX）の L1 組込み — `GainPlugin` モックで検証（adapter の `run` 呼出しは adapter 側 Issue）
- bundle 子の host 側機構（pool/isolation surface・demux・`Failed` 粒度 — `issue-117-bundle-failed-granularity-tests`・`issue-117-bundle-e2e-tests` が対象。runner 自体は v2 で `instances` registry を持つ singleton 非該当）・複数 wire バージョン共存/交渉（単一最新バージョン方針は Issue #119 で確定 — `docs/specs/issue-119-l1-wire-version-policy.md`・現行値は v3）・stderr 配管・自動再起動
- video runner（`run_video`・`ruviform-plugin-l1-video-runner`・`vshm` attach/検証・video verb dispatch）は Issue #118 で実装済みだが pin は #118 test-spec が担当 — 本 spec は audio `run` 経路の oracle のまま
- shm スロットレイアウトの所有（host 側 `l1/shm.rs` が作成 — runner は attach/検証のみ）・`shared_memory` の採否判断（design 記録どおり）
- macOS/Linux でのローカル実証 — Windows ローカルのみ・3 OS は CI `verify` ジョブ（platform notes 下記）

## Platform notes

- **検証範囲**: ローカルは Windows のみ実証 — macOS/Linux の runner E2E は CI `verify` ジョブ（3 OS matrix・`cargo test --workspace`）に委譲（`plugin-l1-proxy-contract-tests.md` の platform notes と同一の採用条件）
- **shm attach**: 子側は `os_id`（named segment key）のみを使い attach — 作成/unlink は持たない（所有権は proxy）。`shared_memory` の Windows/POSIX 差は内部 `shm` モジュールで吸収
- **`CARGO_BIN_EXE_*`**: 統合テスト（`tests/`）でのみ有効 — `env!`/`option_env!` で crate 内に閉じる

## Risk

- `run` の公開形（`ExitCode`/`Box<dyn>` ではなく generic）や argv 規約の誤定 → spawn 不能/形状不一致 — fixture E2E で固定
- `Hello` を送らない・shm header 未検証・ハングする argv 異常 → 起動経路の定義済み拒否を固定
- panic が子プロセスを殺す（abort）または panic で wire 破損 → catch → `Err(PluginFailed)` + 子生存を固定
- `Lifecycle` 判定の非一致（非 `Processing` で `process_host` 受理・違法遷移を panic）→ `Err(Lifecycle(_))` を wire 返却で固定
- `shared_memory` の unsafe 混入・公開 API 露出 → 内部 `shm` モジュール隔離を構造で固定
- `CARGO_BIN_EXE_*` を crate 外で解決する前提に依存（cross-package bin は未提供）→ `tests/` 内 `env!`/`option_env!` で固定
- descriptor の `&'static` 問題の再発（`Scanned` を owned にできない）→ `wire::Descriptor`（String/Vec）+ `From` で固定
