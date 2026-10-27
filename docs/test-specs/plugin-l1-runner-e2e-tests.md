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
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# plugin L1 runner（ruviform-plugin-export）E2E テスト仕様

> status: approved — Issue #76 の実装（Task 2・Task 6 full gate 全 green・`l1_runner_e2e` 11 tests）で契約が実結果により確認済み。run 可能コマンドは task 側に定義する（本書は契約と oracle の正本）。

## Purpose

spec `docs/specs/plugin-l1-out-of-process.md`（proposed・AC6）・approved design `docs/designs/plugin-l1-out-of-process.md` に基づき、`ruviform-plugin-export` クレートの受入契約を固定する — (a) workspace member・公開 `run(factories)` エントリ・crate doc/README、(b) 薄いランナーの argv → `Hello` → shm attach → メッセージループ → `ErasedPlugin` dispatch 経路、(c) 内蔵モック `GainPlugin` を持つ参照 runner `ruviform-plugin-l1-runner` の結合（`CARGO_BIN_EXE_*` spawn → lifecycle → shm ブロック処理 gain 正当性 → `Shutdown` exit(0)）。本 spec は子プロセス側（export）の契約 — proxy 側の `Failed`/pass-through/RT 脚は `plugin-l1-proxy-contract-tests.md`、wire codec/`Failed` 遷移表は `plugin-l1-wire-codec-tests.md` が担当する。export が廃止・runner が再設計された場合に限り本 spec を retire する。

## Feature

`ruviform-plugin-export` クレート（workspace member — 総計 21）: 子プロセス側の薄いランタイムで、`pub fn run(Vec<Box<dyn PluginFactory>>) -> std::process::ExitCode` が唯一の公開エントリ。参照 runner `src/bin/ruviform-plugin-l1-runner.rs` は内蔵モック `GainPlugin` を持つ fixture — `CARGO_BIN_EXE_*` で実プロセス spawn し、最小 host 実装（test 側で codec/shm を組み立て）経由の E2E を検証する。

## Rules

- **workspace/公開面**: `crates/ruviform-plugin-export` が workspace member（`members` に追加・総計 21 クレート）・`publish.workspace = true` 継承（未 publish 継続）・`[lints] workspace`・crate README が設置される。`pub fn run(factories: Vec<Box<dyn PluginFactory>>) -> std::process::ExitCode` が唯一の公開エントリ — `Box<dyn PluginFactory>` でアダプタ非依存（静的 generic ではなく型消去済み trait object — core `erase`/`ErasedPlugin` と整合）
- **argv 規約**: `--ruviform-plugin-id <id>`（parent が `OutOfProcessSpec.descriptor.id` と照合する plugin id）・`--ruviform-shm-key <key>`（proxy 生成の named segment key）を受理 — 引数不足・未知引数は定義済み拒否（usage/`Err` で非ゼロ終了・ハングしない）
- **`Hello` 起動**: `Hello { version: WIRE_VERSION, capabilities }` が子→親への最初のフレーム（起動直後に送信）
- **shm attach + header 検証**: 引き渡された key で named segment を attach し `[header][in-slots][out-slots]` の header を検証（magic/バージョン/スロット形状 — 不一致は非ゼロ終了・定義済み拒否）。shm アクセスは内部 `shm` モジュールに閉じ込め（`shared_memory` の `as_slice`/`as_mut_slice` のみ・自前 `unsafe` ゼロ）
- **シングルスレッドメッセージループ**: stdin frame → `wire::Command` を `ErasedPlugin` へ dispatch → `Reply { seq, result }` を返す（`Reply.result` は `Result<wire::Reply, HostError>`）。`BlockIn { seq }` → `Lifecycle` が `Processing` を確認後、shm in-slot を `BufferView`・out-slot を `BufferViewMut` として `process_host` を呼出し `BlockDone { seq, result }` を返す。`Shutdown`/stdin EOF → lifecycle 正規化（`Deactivate`/`Destroy` 相当）→ exit(0)
- **`ErasedPlugin` dispatch**: `Scan` → `Scanned(Vec<wire::Descriptor>)`（factory scan → `From<PluginDescriptor>` で owned 変換）・`Initialize`/`Activate`/`StartProcessing`/`StopProcessing`/`Deactivate` → `Ok` または `Err`・`SaveState`/`RestoreState` → `State{bytes}`/`Ok`・`OpenGui`/`CloseGui` → `Opened{gui}`/`Ok`（実装が返すまま — L1 側では proxy が `Unsupported` に変換）。違法遷移は `Lifecycle` が `Err(HostError::Lifecycle(_))` を返す — wire `Reply` で送る（panic しない）
- **panic → `HostError::PluginFailed`**: `ErasedPlugin` dispatch（コマンド・`process_host`）の panic は catch して `Err(HostError::PluginFailed)` として `Reply`/`BlockDone` で返す — 子プロセスを殺さない（デフォルトの unwind を掴む — process abort ではなく `Err` に変換）
- **`process_host` 適合**: `Lifecycle` が `Processing` 以外で `BlockIn` → `Err(HostError::Lifecycle(_))`（`process_host` は L0 と同一の `Lifecycle` 判定を使う）。shm slot の `{frames, channels, event_count, flags, transport, events, samples}` を `BufferView`/`BufferViewMut` へ適合（planar `f32`・`WIRE_MAX_*` 上限内）
- **依存**: `ruviform-plugin-core`（path — wire/契約/`ErasedPlugin`）+ `shared_memory = "0.12"`（子側 shm attach — internal `shm` モジュールに閉じ込め・採用記録は design/plan「依存の確定」どおり）。runner 自身の unsafe はゼロ
- **参照 runner `ruviform-plugin-l1-runner`**: 内蔵モック `GainPlugin`（core `Plugin` 実装 — planar `f32` で `gain` を乗算・`PluginDescriptor` 保持・`Initialize`/`Activate`/`StartProcessing`/`SaveState`/`RestoreState`/`Deactivate`/`Destroy` の state 往復・param event 反映）を持ち `run` を呼ぶ fixture — `CARGO_BIN_EXE_ruviform-plugin-l1-runner` で spawn
- **`CARGO_BIN_EXE_*`**: 統合テスト（`tests/`）でのみ有効 — `env!("CARGO_BIN_EXE_ruviform-plugin-l1-runner")`/`option_env!` で crate 内に閉じる（cross-package の bin 解決に依存しない）

## Examples

- **正常系 E2E**: 最小 host 実装が `ruviform-plugin-l1-runner` を spawn（argv に `--ruviform-plugin-id`/`--ruviform-shm-key`）→ `Hello` → `Scan`（`Scanned` の `wire::Descriptor` が `GainPlugin` descriptor と等価）→ `Initialize`/`Activate`/`StartProcessing` → shm in-slot に既知入力を書き `BlockIn` → `BlockDone` → out-slot が gain 乗算後の値を持つ（gain 正当性）→ `SaveState`/`RestoreState` の往復 → `StopProcessing`/`Deactivate` → `Shutdown` で exit(0)
- **argv 異常**: `--ruviform-plugin-id`/`--ruviform-shm-key` なし・未知引数 → usage/`Err` で非ゼロ終了・`Hello` を送らずハングしない
- **shm header 不一致**: 引き渡した key が存在しない・header magic/`wire_version`/形状が不正 → attach/header 検証で非ゼロ終了（定義済み拒否 — `wire_version` 不一致は `Hello.version` と同一の厳密一致契約、Issue #119）
- **panic → `PluginFailed`**: `process_host`（または `Initialize` 等）を panic する `ErasedPlugin` → catch され `Reply`/`BlockDone` が `Err(HostError::PluginFailed)` — 子は終了せず後続コマンドを受理し続ける（プロセス死亡ではない）
- **非 Processing で `BlockIn`**: `StartProcessing` 前に `BlockIn` → `Reply`/`BlockDone` が `Err(HostError::Lifecycle(_))`（L0 と同一意味）
- **version 不一致**: `Hello.version != WIRE_VERSION` → 親（test 側最小 host）が handshake 失敗として kill（proxy contract spec と整合 — runner 自体は version を送るだけ）

## Guarantees

- `ruviform-plugin-export` は workspace member・`run` が唯一の公開エントリ — `Box<dyn PluginFactory>` でアダプタ非依存
- argv → `Hello` → shm attach/header 検証 → メッセージループの経路が定義済み — 引数/header 異常で非ゼロ終了・ハングしない
- `wire::Command` が `ErasedPlugin` dispatch へ写像 — `Reply.result` が `Result<wire::Reply, HostError>`・違法遷移は `Err(Lifecycle(_))` を wire で返す（panic しない）
- プラグイン panic は `Err(HostError::PluginFailed)` として `Reply`/`BlockDone` で返る — 子プロセスは生存し後続を受理（process abort ではない）
- `BlockIn` は `Lifecycle` が `Processing` のときのみ `process_host` — shm in/out slot を `BufferView`/`BufferViewMut` に適合し `BlockDone` を返す
- shm アクセス・`shared_memory` は内部 `shm` モジュールに閉じ込め — `as_slice`/`as_mut_slice` のみ・自前 `unsafe` ゼロ
- `ruviform-plugin-l1-runner`（`GainPlugin` fixture）は `CARGO_BIN_EXE_*` で spawn 可能 — `tests/` 内に閉じる

## Non-goals

- proxy/host 側の `Failed` 遷移・pass-through・RT 脚（`plugin-l1-proxy-contract-tests.md` が担当）・wire codec/`Failed` 遷移表自体（`plugin-l1-wire-codec-tests.md` が担当）
- 実プラグイン（CLAP/VST3/OFX）の L1 組込み — `GainPlugin` モックで検証（adapter の `run` 呼出しは adapter 側 Issue）
- bundle 子（1 子 N インスタンス — runner は singleton）・複数 wire バージョン共存/交渉（単一最新バージョン方針は Issue #119 で確定 — `docs/specs/issue-119-l1-wire-version-policy.md`）・stderr 配管・自動再起動
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
