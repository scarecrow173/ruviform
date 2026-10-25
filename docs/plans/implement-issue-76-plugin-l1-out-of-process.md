---
id: "PLAN-034LNXQmQvHjRBD57FCPBw"
type: "plan"
status: "in-progress"
title: "Issue #76 実施計画 — プラグイン L1 アウトプロセス分離（plugin-export 新設 + plugin-host L1 プロキシ）"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/76"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/plugin-l1-out-of-process.md"
  implemented-by:
    - "docs/tasks/l1-core-contracts-and-wire-codec.md"
    - "docs/tasks/plugin-export-crate-and-l1-runner.md"
    - "docs/tasks/l1-proxy-control-plane.md"
    - "docs/tasks/l1-shm-data-plane-and-rt-legs.md"
    - "docs/tasks/plugin-host-composite-and-l1-e2e.md"
    - "docs/tasks/plugin-l1-final-verification.md"
    - "docs/tasks/plugin-l1-closeout.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-74-plugin-host-l0.md"
    - "docs/plans/implement-issue-31-plugin-isolation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/adr/plugin-isolation-strategy.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/designs/plugin-host-l0.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/tasks/l1-core-contracts-and-wire-codec.md"
    - "docs/tasks/plugin-export-crate-and-l1-runner.md"
    - "docs/tasks/l1-proxy-control-plane.md"
    - "docs/tasks/l1-shm-data-plane-and-rt-legs.md"
    - "docs/tasks/plugin-host-composite-and-l1-e2e.md"
    - "docs/tasks/plugin-l1-final-verification.md"
    - "docs/tasks/plugin-l1-closeout.md"
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #76 実施計画 — プラグイン L1 アウトプロセス分離（plugin-export 新設 + plugin-host L1 プロキシ）

> status: in-progress — Refs #76。approved design `docs/designs/plugin-l1-out-of-process.md`（DESIGN-034LNAalREIHyUQNVXLLin）の正本どおり。Task 1 (l1-core-contracts-and-wire-codec) 完了（actions 1–4・`fmt-check`/`check`/`lint`/`test` 49 binaries・`test-plugin-core` 95・`lint-plugin-core`・`test-rt` 109 件全 exit 0・`cargo doc -p ruviform-plugin-host` 0 warnings・独立 review 2 件 + fix agent — P1 stale-handle generation 修正・P2 CLAP `STATE_FAILED` 往復・指摘全修正・全 mutation 検証済み）。Task 2 (plugin-export-crate-and-l1-runner) 完了（actions 5–8・`ruviform-plugin-export` 新設 + `run()` + `ruviform-plugin-l1-runner` bin + runner E2E 11・`fmt-check`/`check`/`lint`/`test` 55 suites・`test-rt` 全 exit 0・レビュー修正 — guarded `descriptor()` panic・`staging_out` 再ゼロ化・argv exit-2 pin・bounded inbound label・`wire::shm` 正本化・`shared_memory` 0.12.4 unsafe accessor の scoped 局所許可・全 mutation 検証済み）。Task 3–7 未着手 — 全体完了を意味しない。**7 buildable task** と **3 test-spec** を起票済み（task は Task 1–2 `done`・残り `todo`・test-spec は全て `proposed`）。GitHub merge は lead の authoritative `gh` のみ — 本計画は PR merged / issue closed の主張を含まない。

## Goal

spec `docs/specs/plugin-l1-out-of-process.md`（proposed・7 AC）と approved design `docs/designs/plugin-l1-out-of-process.md` の正本どおり、#74（L0・CLOSED）で確定した `PluginHandle` + `HostCommand`/`HostReply`/`HostError` の公開面を変えずに **opt-in の L1 アウトプロセス経路**を実装する: (1) プラグイン側契約の host→core 移設 + `LifecycleState::Failed`/`LifecycleEvent::Failed` + plugin-core `wire` codec、(2) `ruviform-plugin-export` クレート新設と薄いランナー `run(factories)`、(3) plugin-host の L1 プロキシ（stdio 制御面 + `shared_memory` データ面 + `ruviform-rt` SPSC の RT 脚 + クラッシュ→`Failed`+通知+pass-through）、(4) 統一ハンドル空間の `PluginHost` コンポジット。

## Non-Goals

- 自動再起動・supervisor ポリシー（#31 確定 — 再起動判断と状態復元は呼出し側/プラグイン責務）
- L2+ サンドボックス（seccomp・job object・権限分離等の OS 隔離強化）
- 規格アダプタ（`ruviform-plugin-clap`/`-vst3`/`-ofx`）の子プロセス組込み・実プラグインでの L1 E2E — runner/プロキシはモック fixture で検証可能な形に留める
- エンジン（`ruviform-audio`/`ruviform-video`）・利用側への L1 選択配線（consumer 側責務 — plugin-host の engine 消費自体が未実装）
- `InProcessHost`（L0）の挙動変更 — 既存 API・RT 経路は維持（契約移設は `pub use` 再エクスポートで既存パスを互換維持）
- L1 での GUI 実装（L1 インスタンスへの `OpenGui`/`CloseGui` は `Err(HostError::Unsupported)` の defined rejection — 子へ転送しない）
- `process` 経路の RT 化・video/NONRT プラグインの L1 経路・bundle 子（1 子 N インスタンス）・wire バージョン進化（`WIRE_VERSION=1` で十分）・stderr ログ配管・ドライバ共有リーパー/io 多重化（per-instance 2 スレッドのまま）

## Scope and Split Boundary

単一クリティカルパスの 1 計画（plugin-core 共有基盤 → export ランナー/host プロキシ制御面 → データ面 → コンポジットの直列依存 — 独立並行は主張しない）。**7 buildable task**（task 文書は起票済み・各 task が ordered substeps A1…An を持つ）。commit は論理ビルド可能単位で lead が管理 — 1 substep/1 commit 強制・red 専用 commit はしない。

- Task 1（core 基盤: 契約移設 + `Failed` + `wire`）は全後続の共有前提 — export・proxy 双方が wire/移設契約を使う
- Task 2（export ランナー）と Task 3（proxy 制御面）は Task 1 のみに依存し相互独立 — ただし単一レーン直列実行（Task 5 の結合 E2E が runner bin を使うため Task 5 は Task 2 にも依存）
- Task 4（shm データ面 + RT 脚）は Task 3 のプロキシ機構（spawn・スレッド・shm segment）を拡張、Task 5（`PluginHost` コンポジット）は Task 4 + Task 2 に依存
- Task 6（final verification）は Task 2–5 の全完了後に 1 回のみ — full gate の繰返し実行はしない
- テストは test-spec 3 件の契約を、各クレートの inline `#[cfg(test)] mod tests`（単体）と `tests/` 統合テスト（`CARGO_BIN_EXE_*` で実プロセス spawn の E2E）で実装する

## Files / Ownership

| Path | Responsibility | Notes |
| --- | --- | --- |
| `crates/ruviform-plugin-core/src/plugin.rs`（新規） | `Plugin`/`PluginFactory`/`ErasedPlugin`/`erase`/`HostInput`/`HostOutput` — host `plugin.rs` からの移設先 | `crate::` 参照に調整。doc 互換維持 |
| `crates/ruviform-plugin-core/src/boundary.rs`（新規） | `PluginHandle`/`GuiHandle`/`GuiParentToken`/`HostError` — host `command.rs` の対象型の移設先 | `PluginHandle::new` は `#[doc(hidden)] pub` 化。`HostError` に `Failed`/`Ipc`/`DuplicatePluginId` variant + Display arm を追加（`#[non_exhaustive]` で non-breaking） |
| `crates/ruviform-plugin-core/src/lifecycle.rs` | `LifecycleState::Failed`（終端準状態・`Destroy` のみ脱出）・`LifecycleEvent::Failed`（報告遷移）・`is_failed()`・遷移表 const 拡張・doc 区別（terminal vs retryable） | `#[non_exhaustive]` で non-breaking |
| `crates/ruviform-plugin-core/src/wire.rs`（新規） | wire 型（`Message`/`Command`/`Reply`/`Descriptor`）・`WIRE_VERSION = 1`・`WIRE_MAX_FRAMES`/`WIRE_MAX_CHANNELS`/`WIRE_MAX_EVENTS`・手書き LE codec・固定 wire レイアウト + 単体テスト | serde 不採用・core は外部依存ゼロのまま |
| `crates/ruviform-plugin-core/src/lib.rs` | `mod plugin`/`mod boundary`/`pub mod wire` + ルート re-export + crate doc 更新 | `wire` は `pub mod`（export/host 双方が参照する共有面） |
| `crates/ruviform-plugin-host/src/plugin.rs`・`src/command.rs` | 移設型を `pub use` で再エクスポート — `ruviform_plugin_host::Plugin` 等の既存パスを無変更でコンパイル継続 | `HostCommand`/`HostReply` は host に残る（host 境界語彙 — wire はミラー型を別持ち） |
| `crates/ruviform-plugin-export/Cargo.toml`・`src/lib.rs`・`README.md` | 新設クレート — `pub fn run(Vec<Box<dyn PluginFactory>>) -> ExitCode`・argv 規約・メッセージループ・内部 `shm` モジュール | `publish.workspace = true` 継承・`[lints] workspace` |
| `crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-runner.rs` | 参照 runner — 内蔵モック `GainPlugin`（core `Plugin` 実装・state 往復・param event 反映） | fixture・`CARGO_BIN_EXE_*` で spawn 検証 |
| `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs` | runner E2E — test 側最小 host で spawn→`Hello`→`Scan`/lifecycle→shm ブロック処理（gain 正当性）→`Shutdown` exit(0) | `CARGO_BIN_EXE_*` は統合テスト（`tests/`）でのみ有効 |
| `crates/ruviform-plugin-host/src/l1/mod.rs`（新規・`pub(crate)`） | L1 インスタンス機構 — spawn・handshake・driver/reader スレッド・`try_wait` 監視・ローカル lifecycle ミラー・`Failed` 収束・通知キュー | 子 1 + スレッド 2 / インスタンス |
| `crates/ruviform-plugin-host/src/l1/shm.rs`（新規・`pub(crate)`） | `shared_memory` 閉じ込め — 一意 key 生成（pid+slot+counter）・segment 作成/attach・`[header][in-slots][out-slots]` レイアウト・slot view | 公開 API・feature 名に露出しない（採用条件） |
| `crates/ruviform-plugin-host/src/composite.rs`（新規） | `PluginHost`・`OutOfProcessSpec`・`HostNotification`・`ChildExit`・plugin_id→route 表・統一スロット表 | 公開 API（spec 選択面） |
| `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs` | wire 適合の最小 worker — `--crash-after-msgs=N`/`--hang-after-msgs=N` で確定的異常系 | host 側 fixture |
| `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` | proxy E2E — コマンド parity・process 経路・kill/crash→`Failed`+通知+拒否・pass-through | `CARGO_BIN_EXE_l1-test-worker`（+ 後段で runner） |
| `crates/ruviform-plugin-host/Cargo.toml` | `ruviform-rt`（path・Primitives — 許可範囲）+ `shared_memory = "0.12"` + `[[bin]] l1-test-worker` | 依存正当化は design「依存追加の判断」どおり |
| `crates/ruviform-plugin-export/Cargo.toml` | `ruviform-plugin-core`（path）+ `shared_memory = "0.12"` | 下記「依存の確定」参照 |
| `Cargo.toml`（root） | `members` += `crates/ruviform-plugin-export` + 先頭コメントのクレート列記同期 | |
| `crates/README.md` | 索引行追加（計 21 クレート）+ 層表の更新 | |
| `crates/ruviform-plugin-host/src/lib.rs`・`crates/ruviform-plugin-export/src/lib.rs` | crate doc — L1 経路・遅延契約・RT 契約・層責務 | `process` 等価の遅延 = `lookahead_blocks × block_frames / sample_rate` を明記 |

### 依存の確定（design の採否記録 → 本計画の配線）

- `ruviform-plugin-host` += `ruviform-rt`（path — Primitives 層・A.2 許可範囲）+ `shared_memory = "0.12"`（design 採用記録: MIT OR Apache-2.0・安全 slice API・依存木小・Windows+POSIX named object。停滞リスクは採用条件として internal `shm` モジュールへ隔離・公開 API/feature 名へ露出禁止）
- `ruviform-plugin-export` += `shared_memory = "0.12"` — **design の dep 表は export の workspace 依存を「`ruviform-plugin-core` のみ」と記録するが、子側の shm attach（`--ruviform-shm-key` で受けた named segment を開く）は同クレートなしに実現不能（自前 `unsafe`/OS 直は design が棄却）。本計画は同じ採用記録・同じ隔離条件で export にも追加する**（置換コスト封じ込めのため export 側も内部 `shm` モジュールに閉じ込める）。これは A.2 層ルール上の外部依存であり層順序には抵触しない — 実装上の整合解決として明示記録する
- `ruviform-plugin-core` は外部依存ゼロ維持 — wire codec は手書き LE（serde/postcard 不採用の design 判断どおり）
- `Cargo.lock` は追跡継続 — `shared_memory`（+ `cfg-if`/`libc`/`rand`/`win-sys` 系）の追加を記録

## Implementation Sequence（19 action — 将来 task 名のみ確定）

| # | Action | Scope |
| --- | --- | --- |
| 1 | 契約移設 — `plugin.rs`/`boundary.rs` を plugin-core に新設（`Plugin`/`PluginFactory`/`ErasedPlugin`/`erase`/`HostInput`/`HostOutput`/`HostError`/`GuiHandle`/`GuiParentToken`/`PluginHandle`）・`PluginHandle::new` → `#[doc(hidden)] pub`・`HostError` += `Failed`/`Ipc`/`DuplicatePluginId`・host 側を `pub use` 再エクスポート化（`command.rs` は `HostCommand`/`HostReply` のみ保持）・既存テスト全パスで互換実証 | plugin-core + plugin-host |
| 2 | `LifecycleState::Failed`/`LifecycleEvent::Failed`/`is_failed()` + 遷移表拡張（`Created\|Initialized\|Activated\|Processing → Failed`・`(Failed, Destroy) → Destroyed`・その他 illegal）+ doc 区別 + `lifecycle_at` helper・遷移表テスト更新 | plugin-core |
| 3 | `wire` モジュール — メッセージ 6 種（`Hello`/`Command`/`Reply`/`BlockIn`/`BlockDone`/`Shutdown`）・`wire::Command`/`wire::Reply`/`wire::Descriptor`（enum は `#[non_exhaustive]`・`From<PluginDescriptor>` + フィールド等価比較）・`WIRE_VERSION`/`WIRE_MAX_*` 定数・LE codec（`[u32 LE payload_len][u8 kind][payload]`・`WireError` で malformed を定義済み拒否）・`ProcessConfig`/`ProcessResult`/`GuiHandle`/`GuiParentToken`/`PluginHandle`/`HostError`/`TransportInfo`/`Event`/`EventList`/`MediaTime` の固定 wire レイアウト（accessor encode・ctor decode）+ roundtrip/境界/malformed 単体テスト | plugin-core |
| 4 | narrow gate — `mise run check`/`lint`/`test` + `mise run test-plugin-core`/`lint-plugin-core` exit 0（移設後の無退行・`Failed` 遷移表・codec テスト green） | foundations |
| 5 | `crates/ruviform-plugin-export` 骨格 — Cargo.toml（plugin-core + `shared_memory`）・`publish.workspace`/`[lints] workspace`・lib.rs crate doc・crate README・root `Cargo.toml` members + コメント同期・`crates/README.md` 索引（計 21 クレート） | export 新設 |
| 6 | `run(factories) -> ExitCode` — argv（`--ruviform-plugin-id`・`--ruviform-shm-key`）→ `Hello` 送信 → shm attach + header 検証（内部 `shm` モジュール）→ シングルスレッドメッセージループ（`wire::Command` → `ErasedPlugin` dispatch → `Reply`・`BlockIn` → `Lifecycle` が `Processing` 確認後 `process_host` via slot `BufferView`/`BufferViewMut` view → `BlockDone`・`Shutdown`/stdin EOF → lifecycle 正規化 → exit(0)）・panic → `HostError::PluginFailed` 変換・runner 内 unsafe なし | export runner |
| 7 | `src/bin/ruviform-plugin-l1-runner` — 内蔵モック `GainPlugin`（`Plugin` 実装・state 往復・param event 反映）+ `run` 呼出し | export fixture |
| 8 | `tests/l1_runner_e2e.rs` — `CARGO_BIN_EXE_ruviform-plugin-l1-runner` spawn → `Hello` → `Scan`/lifecycle コマンド往復 → shm ブロック処理 gain 正当性 → `Shutdown` exit(0) + gate `mise run check`/`lint`/`test` | export E2E |
| 9 | plugin-host deps + `l1` 骨格 — `ruviform-rt` + `shared_memory = "0.12"` を Cargo.toml へ・`src/l1/mod.rs` + `src/l1/shm.rs`（`pub(crate)`・一意 key・segment 作成・header 検証 — 安全 API のみ） | host 制御面 |
| 10 | L1 インスタンス機構 — spawn（argv bootstrap）→ handshake（`Hello`→`Scan`→descriptor フィールド等価照合→`Instantiated`・失敗/`command_timeout` → `Ipc` + kill/reap）・driver スレッド（`Child` 単独所有・stdin・command mpsc・`try_wait` 周期 ~200µs `DRIVER_POLL_INTERVAL`・shutdown watchdog）・reader スレッド（stdout・`CommandReply`→waiter 起床・EOF `seen_eof`）・ローカル lifecycle ミラー・`Failed` 冪等収束（通知キュー + pending waiter へ `Ipc` 起床）・`Destroy` → `Shutdown` + reap | host 制御面 |
| 11 | `src/bin/l1-test-worker`（`--crash-after-msgs=N`/`--hang-after-msgs=N`）+ `tests/l1_proxy_e2e.rs` 制御面 E2E — scan→instantiate→lifecycle→state→destroy parity・`Lifecycle` エラー意味一致・kill/crash → `Failed` + `InstanceFailed` + 以後 `Failed`（`Destroy` は `Ok`）・自動再起動なし・GUI → `Unsupported` + gate `check`/`lint`/`test` | host 制御面 E2E |
| 12 | shm データ面 — slot レイアウト `{seq u64, frames u32, channels u32, event_count u32, flags, transport, result, events, samples f32[]}`（`WIRE_MAX_*` 上限・`Activate` 上限超 → `Prepare(UnsupportedConfig)`）・staging 事前確保（`Activate` で `max_frames × 総チャネル`・D = `lookahead_blocks + 1`・in/out 各）・doorbell fence（writer `compiler_fence(Release)`・reader `Acquire`・shm 共有制御状態なし・厳密交互制御） | host データ面 |
| 13 | RT 脚 + `process` 等価フロー — submit/result `ruviform_rt::channel`（`u64` seq）・RT 側は `try_push`/`try_pop` のみ・`process`（lifecycle mirror `Processing` const 検査 → `staging_in[seq%D]` 複写 + `try_push`（満杯→pass-through）→ `expected = seq - lookahead`（underflow=定義済み無音）→ result drain（stale 破棄・有界）→ `staging_out` 複写 + `last_output` 更新 else pass-through）・pass-through 規則（健全 underrun=直前受信 or 無音・Failed=無音・常に `Ok(CONTINUE)`） | host データ面 |
| 14 | RT レーン + データ面 E2E — `rt_l1_process_path_no_alloc`（assert_no_alloc 窓・`process` try 経路 0 alloc 実測）・`rt_l1_result_leg_try_ops` + `l1-test-worker` で process 経路 gain 正当性・定義遅延（先行 2 ブロック）・crash/hang 時 pass-through・ハングなし + gate `test`/`lint`/`test-rt`（実一致件数記録） | host データ面 E2E |
| 15 | `PluginHost` コンポジット（`src/composite.rs`）— `new`/`register_factory`/`register_out_of_process`/`execute`/`process`/`lifecycle_state`/`plugin_mut`（L0 のみ）/`try_recv_notification`/`#[doc(hidden)] kill_child`・`OutOfProcessSpec`/`HostNotification`/`ChildExit` 公開型・plugin_id→route 表・統一スロット表（generation 規則 L0 同一）・`DuplicatePluginId` 拒否・lib.rs re-export + crate doc（遅延契約明記） | composite |
| 16 | doc 整合 — `crates/ruviform-plugin-host/README.md`（L1・許可依存・RT 契約）・`crates/README.md` Host 層行・root `Cargo.toml` members コメント・plugin-export crate doc/README の最終一致 | docs |
| 17 | 公開面 E2E — `PluginHost` 経由 L0+L1 混在の scan→instantiate→lifecycle→process→state→destroy・runner bin を経路に使う結合・L0 無退行 + gate `fmt-check`/`check`/`lint`/`test`/`test-rt` | composite E2E |
| 18 | **Full Gate（1 回のみ・本計画最後段）** — `mise run fmt-check`・`check`・`lint`・`test`・`test-rt`・`bench-smoke`・`mise exec -- prek run --all-files`・doc audit — 全 log `docs/impl/exp/data/issue-76-final-gate-*-*.log` 保存・`test-rt` は実一致件数で証明（0 件一致スキップを成功と偽らない） | gate |
| 19 | canonical closeout — 実績 IR・coverage・task/plan status・test-spec を実結果で confirmed 更新・GitHub merge は authoritative `gh` のみ | closeout |

- test-spec（起票済み・proposed）: [plugin-l1-wire-codec-tests](../test-specs/plugin-l1-wire-codec-tests.md)・[plugin-l1-proxy-contract-tests](../test-specs/plugin-l1-proxy-contract-tests.md)・[plugin-l1-runner-e2e-tests](../test-specs/plugin-l1-runner-e2e-tests.md) — `verified-by` は実ファイルに更新済み
- **7 task 文書**（起票済み・todo・actions 1-4/5-8/9-11/12-14/15-17/18/19）: [l1-core-contracts-and-wire-codec](../tasks/l1-core-contracts-and-wire-codec.md)・[plugin-export-crate-and-l1-runner](../tasks/plugin-export-crate-and-l1-runner.md)・[l1-proxy-control-plane](../tasks/l1-proxy-control-plane.md)・[l1-shm-data-plane-and-rt-legs](../tasks/l1-shm-data-plane-and-rt-legs.md)・[plugin-host-composite-and-l1-e2e](../tasks/plugin-host-composite-and-l1-e2e.md)・[plugin-l1-final-verification](../tasks/plugin-l1-final-verification.md)・[plugin-l1-closeout](../tasks/plugin-l1-closeout.md)
- 各 task 間は狭い検証のみ（full suite 繰返しはしない — Full Gate は action 18 の一回）

## Test Oracles（design/test-spec の正本どおり・新規代替プロトコル禁止）

- フレーム: `[u32 LE payload_len][u8 kind][payload]` — `Hello` が唯一 `WIRE_VERSION = 1` を運ぶ・不一致は handshake 失敗 → kill + `HostError::Ipc`・残りの wire メッセージにバージョンは載せない
- コマンド往復: `HostCommand` 10 動詞（`Scan`/`Initialize`/`Activate`/`StartProcessing`/`StopProcessing`/`Deactivate`/`SaveState`/`RestoreState`/`OpenGui`/`CloseGui`）が `wire::Command` へ 1:1 写像（handle 非修飾 — 子は singleton）。`Instantiate` ↔ spawn + handshake、`Destroy` ↔ `Shutdown` + reap。違法遷移は子側 `Lifecycle` が判定し `Err(HostError::Lifecycle(_))` が wire で返る — L0 と同一のリプライ/エラー意味
- `Failed` 契約: `Created|Initialized|Activated|Processing → Failed`・`(Failed, Destroy) → Destroyed`・`accepts_processing() == false`・`is_failed() == true`。`Failed` インスタンスへのコマンド（`Destroy` 除く）→ `Err(HostError::Failed)`・`Destroy` → `Ok`・`process` 等価 → `Ok(CONTINUE)` + 無音。ライブ中の child exit は exit 0 を含め `Failed`・`Shutdown` 後の exit のみ clean teardown・自動再起動なし
- pass-through 契約: 健全 underrun → 直前に受信した出力ブロック（受信歴なし → 無音）・`Failed` → 無音・常に `Ok(ProcessResult::CONTINUE)` — RT 面の定義済み拒否は pass-through 出力そのもの
- 遅延契約: `lookahead_blocks`（既定 **2**・`OutOfProcessSpec.lookahead_blocks`）× `block_frames / sample_rate`（2 × 512f @ 48 kHz ≈ 21.3 ms）— API doc・crate doc に明記。ブロック面に wall-clock タイムアウトは置かない（underrun が機構）
- shm データ面: インスタンス毎に named segment 1 本・`[header][in-slots][out-slots]`・`WIRE_MAX_FRAMES = 8192`・`WIRE_MAX_CHANNELS = 16`・`WIRE_MAX_EVENTS = 1024`（slot ≈ 550 KB・D=3 で ≈ 3.3 MB/インスタンス）・shm に共有制御状態（head/tail・ロック・atomic 配置）を持たない・doorbell `seq` がスロット選択を担う（`seq % D`）・doorbell バイト列が release/acquire エッジ（`compiler_fence`）・`as_slice`/`as_mut_slice` の安全 API のみ（自前 `unsafe` ゼロ）
- クラッシュ検出: (i) reader stdout EOF（`seen_eof`）(ii) driver `try_wait` → `Some(ExitStatus)`（reap 兼務）— 先に発火した方が冪等に `Failed` 遷移 + `HostNotification::InstanceFailed { handle, exit: ChildExit { code, signal } }`（std mpsc → `try_recv_notification`）+ pending waiter へ `HostError::Ipc` 起床
- RT 契約: RT 側は `try_push`/`try_pop`（`u64` seq）+ 事前確保 staging への bounded memcpy のみ — pipe/shm/ブロッキング read は driver/reader 側。`rt_l1_process_path_no_alloc` で assert_no_alloc 窓内 0 alloc を実測
- handshake: spawn → argv `--ruviform-plugin-id <id>`/`--ruviform-shm-key <key>` → `Hello` → `Scan` → `Scanned`（`wire::Descriptor` を `OutOfProcessSpec.descriptor` とフィールド等価照合・不一致 → kill + `Ipc`）→ `Instantiated{handle}`。`HostReply::Scanned` は registration 供給の `'static` 値を返す（owned→'static 変換なし・`Box::leak` なし）
- codec: `wire::Descriptor` は所有型（`String`/`Vec<String>`）— `PluginDescriptor` の `&'static` 問題を解消。`TransportInfo`/`Event`/`EventList`/`MediaTime` は固定 wire レイアウト（accessor encode・ctor decode — `repr(Rust)` memcpy 横断なし）
- 拒否面: `Failed` インスタンスへの `process` 等価/コマンド・GUI → `Unsupported`・`Activate` 上限超 → `Prepare(UnsupportedConfig)`・重複 plugin_id → `DuplicatePluginId`・spawn/handshake/timeout → `Ipc`

## Required mise tasks

- 既存タスクのみで完結（新規 mise タスクなし）: `fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`・`test-plugin-core`/`lint-plugin-core`（core 集中検証の既存レーン）・`mise exec -- prek run --all-files`・`mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json`
- L1 は feature gate ではなく常時コード — `cargo test --workspace` が E2E 統合テスト（`tests/`・`CARGO_BIN_EXE_*`）を含めて実行するため別レーンは不要。`rt_l1_*` は `mise run test-rt`（`rt_` 名フィルタ）で選択される
- CI: `.github/workflows/ci.yml` の `verify` ジョブ（ubuntu/windows/macos 3 OS matrix・`workflow_dispatch`）が `verify-ci` 経由で L1 E2E を含む `cargo test --workspace` を実行 — design の採用条件「3 OS でのコンパイル/E2E」を plan の検証条件として引き継ぐ（ローカル実証は Windows のみ・macOS/Linux は CI 実行時のみ記録 — 計画書は未実行の主張をしない）

## Risks / Readiness

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | `shared_memory` の実効 API/MSRV（0.12.x・2022-03 停滞）が宣言と異なる（`ShmemConf`/`os_id`/`as_mut_slice` の形状・Drop 時 unlink 規則） | med | med | Task 3 冒頭で docs.rs/cargo doc で実 API を確認 — 相違なら blocker 記録（fake success なし）。採用条件どおり internal `shm` モジュールに隔離（置換 ~200 LOC） |
| 2 | 子側 shm attach の依存配線誤認（export が shared_memory 無しで attach 不可能） | low | high | plan「依存の確定」で export への `shared_memory` 追加を明示 — Task 2 の dep 追加で実証 |
| 3 | `Failed` 遷移・クラッシュ検出の実装誤り（`try_wait` と EOF の冪等収束・voluntary exit 0 を `Failed` と誤区別） | med | high | test-spec で冪等収束・exit code 不問・`Shutdown` 後のみ clean teardown・pending waiter `Ipc` 起床を固定。E2E で `kill`/クラッシュ両経路を検証 |
| 4 | shm データ面のメモリ順序・スロット再利用規約の誤り（`seq % D`・`done seq-D` 後のみ再使用・fence 欠落で stale 読み） | med | high | doorbell 前 Release/受信後 Acquire の `compiler_fence`・厳密交互制御を test-spec で固定・3 OS matrix で検証（CI） |
| 5 | RT 脚の alloc/blocking 混入（`process` 経路で pipe write・shm write・待機を RT スレッドへ流出） | med | high | `assert_no_alloc` 窓の `rt_l1_process_path_no_alloc` で機械検出 + 構造レビュー（RT 側は try_push/try_pop + bounded memcpy のみ） |
| 6 | 契約移設の互換破壊（既存 `ruviform_plugin_host::*` パス・列挙参照が変わる） | low | high | `pub use` 再エクスポートで無変更コンパイル継続 — Task 1 gate で既存テスト全パス + `mise run check`/`lint` を担保 |
| 7 | Windows/POSIX の shm 名・unlink 規則差（named object の名前空間・Destroy 時 cleanup）で POSIX が未検証のまま | med | med | test-spec の platform notes に固定 — ローカルは Windows のみ実証・3 OS matrix は CI `verify` ジョブに委譲（design 採用条件） |
| 8 | `command_timeout` 中に子がハングし pending waiter が永久待機 | low | med | タイムアウト → `Ipc` + `Failed` 遷移・driver watchdog が猶予超過で `child.kill()` — ハングしないことを E2E で固定 |

## Rollback

- 新規クレート・新規ファイル・enum variant 追加が主体 — 履歴保持の corrective/revert commit で task 単位に戻す（reset/force なし）。`Cargo.lock` は `git checkout -- Cargo.lock` で戻し再生成
- `LifecycleState::Failed`/`LifecycleEvent::Failed`・`HostError` 新 variant は `#[non_exhaustive]` への追加 — revert しても downstream match の破壊にならない
- 契約移設は `pub use` 再エクスポートで完全互換 — revert は移設戻しで完結（公開パス不変のまま）
- `ruviform-plugin-export` は未 publish（`publish = false` 継承）・`shared_memory` は内部モジュール隔離のため外部依存の除去も局所完結・DB/persistent data migration なし

## Verification

- per-task narrow checks のみ — 各 task の `## Verification` が run 可能コマンドを持つ（`mise run check`/`lint`/`test`/`test-rt`・core 集中は `test-plugin-core`/`lint-plugin-core`）
- Full Gate は action 18 の一回のみ — 全 log 保存・`test-rt` 実一致件数で証明
- spec AC1–AC7 ↔ actions/test-specs の写像: AC1（コマンド parity）→ actions 9–11・15–17（tspec proxy）・AC2（process 等価+遅延+gain）→ actions 12–14・6–8（tspec proxy/runner）・AC3（RT try 経路）→ action 14・18（tspec proxy）・AC4（kill→Failed+通知+拒否）→ actions 10–11・15・17（tspec proxy）・AC5（pass-through）→ actions 13–14（tspec proxy）・AC6（export クレート+runner）→ actions 5–8（tspec runner）・AC7（gate+監査）→ actions 18–19
- doc 監査チェーン起因 findings 0（`mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json`）
- test-spec link は `verified-by` に更新済み — **実検証は未実施**（PLANNED・未実行）

## Open Questions / Gaps

- なし — design が discovery の Open Questions 全項目を確定済み（shm 機構・wire 形式・`Failed` 表現・runner 形態・L0/L1 選択面・遅延/pass-through・fixture）。`shared_memory` を export にも配線する点は「依存の確定」で明示済み（design dep 表の workspace 依存記述との整合解決）。codec 関数署名の細部（`encode`/`decode`/エラー型名）は design が固定しない実装詳細 — 本計画は `wire::encode`/`wire::decode`/`WireError` の最小形を task 文書でピン止めする

## Self-Review

- [x] Every spec requirement maps to a task（7 AC → 7 task + 3 test-spec の写像を確認済み）
- [x] No placeholders remain（ファイル名・依存指定・定数・コマンド名は確定値のみ — codec 関数名は task でピン止め）
- [x] File paths, relations, status values, and terminology are consistent（`PluginHost`/`OutOfProcessSpec`/`HostNotification::InstanceFailed`/`ChildExit`/`LifecycleState::Failed`/`LifecycleEvent::Failed`/`HostError::{Failed,Ipc,DuplicatePluginId}`/`wire::Command`/`wire::Reply`/`wire::Descriptor`/`WIRE_VERSION`/`WIRE_MAX_*`/`rt_l1_*` — design の正本どおり）

## Implementation Handoff

- Recommended mode: delegated 実行 + lead の task 完了ごとの実レビュー（ユーザ承認済みを前提 — 環境依存の skill ID は本計画に埋め込まず、実環境で発見される capability に routing する）
- Independent work streams: Task 2（export）と Task 3（proxy 制御面）は Task 1 のみに依存し論理上独立だが、単一クリティカルパスで直列実行（E2E 結合が Task 5 で runner を使う）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- レビュー焦点: `wire` codec が design のフレーム/型定義と逐語一致しているか（`[u32 LE payload_len][u8 kind][payload]`・6 メッセージ・`WIRE_VERSION = 1`・`&'static` 問題の解消 — `wire::Descriptor` 所有型 + registration 供給 'static）
- `LifecycleState::Failed`/`LifecycleEvent::Failed` の遷移表が design どおり terminal かつ `(Failed, Destroy) → Destroyed` のみ合法か — retryable `InitFailed`/`ActivateFailed` との doc 区別
- `PluginHost` の統一ハンドル空間・登録時ルーティング・`DuplicatePluginId` 拒否が「呼出し側は層を意識しない」契約を壊していないか（`InProcessHost` は無変更のまま）
- クラッシュ収束が 2 経路（EOF・`try_wait`）で冪等か — voluntary exit 0 を `Failed` とみなす・`Shutdown` 後 exit のみ clean・pending waiter の `Ipc` 起床・通知キューへの `InstanceFailed`
- pass-through 規則（健全 underrun=直前受信 or 無音・Failed=無音・`Ok(CONTINUE)`）と遅延契約（`lookahead_blocks × block_frames / sample_rate`）の doc/実装一致
- `shared_memory` が internal `shm` モジュールに閉じ込められ公開 API/feature 名に露出していないか（採用条件）・自前 `unsafe` ゼロか（`as_slice`/`as_mut_slice` のみ + `compiler_fence`）
- RT 側が `try_push`/`try_pop` + staging bounded memcpy のみで pipe/shm/ブロッキングが driver/reader に留まるか（`rt_l1_process_path_no_alloc` の assert_no_alloc 窓）
