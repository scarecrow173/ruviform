---
id: "TSPEC-034LNXHpBkMIVTg6nAQUVh"
type: "test-spec"
status: "approved"
title: "plugin L1 プロキシ契約（コマンド parity・クラッシュ Failed・pass-through・RT 脚）テスト仕様"
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
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# plugin L1 プロキシ契約（コマンド parity・クラッシュ Failed・pass-through・RT 脚）テスト仕様

> status: approved — Issue #76 の実装（Task 1–6・full gate 全 green・`l1_proxy_e2e` 42 tests + `rt_l1_*` 2 件実選択）で契約が実結果により確認済み。run 可能コマンドは task 側に定義する（本書は契約と oracle の正本）。

## Purpose

spec `docs/specs/plugin-l1-out-of-process.md`（proposed）・approved design `docs/designs/plugin-l1-out-of-process.md` に基づき、`ruviform-plugin-host` の L1 バックエンドと `PluginHost` コンポジットの受入契約を固定する — (a) `PluginHost` による opt-in L1 ルーティングと統一ハンドル空間・呼出し側の層非意識、(b) spawn→handshake→コマンド往復の L0 parity、(c) クラッシュ検出→`Failed` 遷移→`HostNotification::InstanceFailed`→定義済み拒否、(d) `process` 等価経路（shm スロット + `ruviform-rt` SPSC 脚 + pass-through + 遅延契約）、(e) RT 側読取りの no-block・no-alloc（`rt_` 命名テスト）、(f) `shared_memory` の隔離条件とプラットフォーム注記。L1 ワイヤ codec・`Failed` 遷移表自体は `plugin-l1-wire-codec-tests.md`、export 側 runner の結合は `plugin-l1-runner-e2e-tests.md` が担当する。L1 プロキシまたはコンポジットが置換・再設計された場合に限り本 spec を retire する。

## Feature

`ruviform-plugin-host` の L1 バックエンド: `PluginHost` コンポジット（`register_factory` L0・`register_out_of_process(OutOfProcessSpec)` L1・`execute`/`process`/`lifecycle_state`/`plugin_mut`/`try_recv_notification`/`kill_child`）・per-instance L1 プロキシ（`pub(crate)` — 1 子プロセス + `driver`/`reader` 2 スレッド）・stdio パイプ制御面 + `shared_memory` named segment データ面 + `ruviform-rt` SPSC の RT 脚。fixture は `src/bin/l1-test-worker`（wire 適合 worker・`--crash-after-msgs=N`/`--hang-after-msgs=N`）。

## Rules

- **`PluginHost` 選択面**: `register_factory`（L0 ルート）・`register_out_of_process(OutOfProcessSpec) -> Result<(), HostError>`（L1 ルート — plugin_id → L1 route を登録時に宣言）・`Instantiate{plugin_id}` の形は不変で呼出し側は層を意識しない。統一ハンドル空間 — `PluginHandle` の slot/generation 規則は L0 と同一（generation で破棄済みスロット検出）。同一 plugin_id の L0/L1 重複登録 → `Err(HostError::DuplicatePluginId)`
- **`OutOfProcessSpec`**: `descriptor: PluginDescriptor`（'static — `Scan`/`Instantiate` の公開面を担う）・`program: OsString`・`args: Vec<OsString>`・`lookahead_blocks: u32`（既定 **2**）・`command_timeout: Duration`（既定 **10s** — Reply 待ちの defined timeout）
- **公開通知面**: `HostNotification::InstanceFailed { handle: PluginHandle, exit: ChildExit }`（`#[non_exhaustive]`）・`ChildExit { code: Option<i32>, signal: Option<i32> }`（`signal` は unix のみ）・`try_recv_notification() -> Option<HostNotification>`（std mpsc キュー・非 RT）
- **handshake**: spawn → argv `--ruviform-plugin-id <id>`/`--ruviform-shm-key <key>` → 子 `Hello` → 親 `Scan` → 子 `Scanned`（`wire::Descriptor` を `OutOfProcessSpec.descriptor` とフィールド等価照合 — 不一致 → kill + `HostError::Ipc`）→ `Instantiated{handle}`。いずれかの失敗・`command_timeout` → `Err(HostError::Ipc)` + child kill/reap
- **コマンド parity（spec AC1）**: L1 バックエンドが L0 と同一コマンド系列（scan → instantiate → initialize → activate → start/stop processing → deactivate → save/restore state → destroy）を子プロセス側で実行し同一リプライ/エラー意味で返す — 違法遷移は子側 `Lifecycle` が判定し `Err(HostError::Lifecycle(_))` が wire で返る（L0 と同一の意味）
- **クラッシュ契約（spec AC4）**: (i) reader が stdout EOF を検出（`seen_eof`）・(ii) driver の `try_wait` が `Some(ExitStatus)`（reap 兼務・~200µs `DRIVER_POLL_INTERVAL`）— 先に発火した方が冪等に `Failed` 遷移 + `HostNotification::InstanceFailed` を通知キューへ + pending コマンド waiter を `HostError::Ipc` で起床。ライブ中の child exit は exit 0 を含め `Failed`（自発終了もプロトコル違反）・`Shutdown` 送信後の exit のみ clean teardown・自動再起動はしない
- **`Failed` 後の定義済み拒否**: コマンド（`Destroy` 除く）→ `Err(HostError::Failed)`・`Destroy` → `Ok`（必要なら kill・reap・スロット解放）・`process` 等価 → `Ok(CONTINUE)` + 無音。`Failed` 以後に新たな wire メッセージは送らない（子は死んでいるか殺される前提）
- **process 等価経路（spec AC2）**: RT `process(handle, inputs, outputs, events, transport)` — (1) ローカル lifecycle ミラーが `Processing` か const 検査 → (2) 入力を `staging_in[seq % D]` へ複写・`submit.try_push(seq)`（満杯なら投入放棄 → pass-through）→ (3) `expected = seq - lookahead`（underflow = 定義済み無音）→ (4) result ch を `expected` まで drain（`< expected` は stale 破棄・有界回数）→ (5) 一致 seq なら `staging_out` を `outputs` へ複写し `last_output` 更新、なければ pass-through
- **pass-through（spec AC5）**: 健全インスタンスのアンダーラン → 直前に受信した出力ブロック（受信歴なし → 無音）・`Failed` → 無音。いずれも `Ok(ProcessResult::CONTINUE)` — RT 面の定義済み拒否は pass-through 出力そのものでハングしない。submit/result チャネル満杯・応答遅延・無応答はエラーでなくフォールバック
- **遅延契約**: `lookahead_blocks`（既定 2）× `block_frames / sample_rate`（2 × 512f @ 48 kHz ≈ 21.3 ms）— API doc・crate doc に明記。ブロック面に wall-clock タイムアウトは置かない（underrun が機構）
- **shm データ面**: インスタンス毎に named segment 1 本（proxy が一意 key `pid + slot + counter` で生成・argv `--ruviform-shm-key` で子へ引き渡し・所有権は proxy・Destroy/drop で unlink）。`[header][in-slots][out-slots]`・slot `{seq u64, frames u32, channels u32, event_count u32, flags, transport wire, result wire, events wire, samples f32[]}`・`WIRE_MAX_FRAMES = 8192`/`WIRE_MAX_CHANNELS = 16`/`WIRE_MAX_EVENTS = 1024`（slot ≈ 550 KB・D=3 ≈ 3.3 MB/インスタンス）。`Activate` の `ProcessConfig` が上限超 → `Err(HostError::Prepare(PrepareError::UnsupportedConfig))`
- **shm 制御状態なし**: head/tail・ロック・atomic 配置を shm に持たない — スロット選択は doorbell `seq`（`seq % D`）・in/out は厳密交互制御（producer は `done seq-D` 受領後にのみ再利用）・doorbell バイト列が release/acquire エッジ（writer `atomic::fence(Release)` 前・reader `atomic::fence(Acquire)` 後 — 実装は `std::sync::atomic::fence` の hardware fence・design 初出文言の `compiler_fence` から Task 4 で強化済み）・shm アクセスは `as_slice`/`as_mut_slice` のみ（自前 `unsafe` ゼロ）
- **RT 契約（spec AC3）**: submit/result 両脚は `ruviform_rt::channel`（`u64` seq）— RT 側は `try_push`/`try_pop` + 事前確保 staging への bounded memcpy のみ（pipe/shm/ブロッキング read は driver/reader 側）。`rt_l1_process_path_no_alloc`（assert_no_alloc 窓で `process` try 経路 0 alloc）・`rt_l1_result_leg_try_ops` を持ち `mise run test-rt` で選択可能。非 RT テスト名に `rt_` を含めない
- **拒否面一覧**: 違法 lifecycle 遷移 → `Err(HostError::Lifecycle(_))`（子側 `Lifecycle` 判定）・`Failed` インスタンスへのコマンド（`Destroy` 除く）→ `Err(HostError::Failed)`・`Failed` への `Destroy` → `Ok`・`Failed` への `process` → `Ok(CONTINUE)` + 無音・spawn/handshake/version/transport 失敗・コマンド応答タイムアウト → `Err(HostError::Ipc)`（タイムアウトは併せて `Failed` 遷移）・L1 への `OpenGui`/`CloseGui` → `Err(HostError::Unsupported)`（proxy が子へ送らず拒否）・`Activate` 上限超 → `Prepare(UnsupportedConfig)`・重複 plugin_id → `DuplicatePluginId`
- **`shared_memory` 隔離**: `shared_memory` の利用は internal `shm` モジュールに閉じ込め・公開 API・feature 名に露出しない（design 採用条件・置換コスト ~200 LOC）
- **`kill_child`**: `#[doc(hidden)] pub fn kill_child(PluginHandle)` — 検証支援（E2E で子を強制終了しクラッシュ経路を駆動）
- **`plugin_mut`**: `PluginHost::plugin_mut(handle)` は L0 スロットのみを返す（L1 は `ErasedPlugin` を持たない — `None`）
- **スレッド/プロセス所有**: per L1 インスタンス = 子 1 + スレッド 2（`driver`: `Child` 単独所有・stdin・shm in-slot・command mpsc・submit `Consumer`・`try_wait`・shutdown watchdog `child.kill()`。`reader`: stdout・shm out-slot・result `Producer`・`CommandReply`→waiter 起床・`BlockDone`→staging 複写+push・EOF `seen_eof`）

## Examples

- **コマンド parity**: `PluginHost` に `OutOfProcessSpec`（`program` = `l1-test-worker`）を登録 → `Instantiate` → `Initialize` → `Activate` → `StartProcessing` → `SaveState` → `RestoreState` → `StopProcessing` → `Deactivate` → `Destroy` が L0 と同一の `HostReply`/`HostError` 意味で返る（`Err(HostError::Lifecycle(_))` の違法遷移を含む）
- **kill→Failed**: `kill_child`（または子の自発 kill/`--crash-after-msgs`）→ `try_wait`/EOF で `Failed` → `try_recv_notification()` が `InstanceFailed { handle, exit }` を返す → 以後 `Initialize`/`Activate`/`SaveState` → `Err(HostError::Failed)`・`Destroy` → `Ok`・`process` → `Ok(CONTINUE)` + 無音・自動再起動なし（新規 spawn が観測されない）
- **descriptor 不一致**: `OutOfProcessSpec.descriptor` と子が `Scanned` で報告した `wire::Descriptor` が不一致 → handshake 失敗 → `Err(HostError::Ipc)` + child kill/reap
- **version 不一致**: `Hello.version != 1` → handshake 失敗 → `Err(HostError::Ipc)`
- **応答なし pass-through**: `--hang-after-msgs` worker で process 経路が応答なし → underrun → `Ok(CONTINUE)` + 直前受信 or 無音・ハングしない
- **GUI defined rejection**: L1 インスタンスへの `OpenGui`/`CloseGui` → `Err(HostError::Unsupported)`（子へ転送しない）
- **`Activate` 上限超**: `max_frames > WIRE_MAX_FRAMES`（または channels/events 超過）→ `Err(HostError::Prepare(PrepareError::UnsupportedConfig))`
- **重複登録**: L0 `register_factory` と L1 `register_out_of_process` で同一 plugin_id → `Err(HostError::DuplicatePluginId)`
- **`rt_l1_process_path_no_alloc`**: assert_no_alloc 窓で `process` の try 経路（staging 複写 + try_push + drain + staging_out 複写/pass-through）が 0 alloc・`rt_l1_result_leg_try_ops` が try_pop/try_push のみを使う
- **runner 経由結合**: `ruviform-plugin-l1-runner`（export fixture）を `program` に持つ `OutOfProcessSpec` で spawn → lifecycle → shm ブロック処理で gain 正当性（`l1-test-worker` だけでなく実 runner での結合 — Task 5）

## Guarantees

- `PluginHost` のコマンド系列・リプライ・`HostError` 意味は L0 と同一（呼出し側は層を意識しない）— `Instantiate{plugin_id}` の形は不変
- `Failed` 遷移は冪等 — EOF・`try_wait` のどちらが先でも同一の `Failed` + 通知 + `Ipc` 起床に収束・二重遷移/二重通知しない
- `process` 等価経路はハングしない — underrun/クラッシュ/無応答で `Ok(CONTINUE)` + pass-through（直前受信 or 無音）
- RT 側は `try_push`/`try_pop` + staging bounded memcpy のみ — `rt_l1_process_path_no_alloc` で 0 alloc 実測・`mise run test-rt` で選択可能
- `shared_memory` は internal `shm` モジュールに閉じ込め — 公開 API・feature 名に露出しない・自前 `unsafe` ゼロ（`as_slice`/`as_mut_slice` のみ）
- `Failed` 後のコマンド/`process` 等価は定義済み拒否（`Failed`/`Ok(Continue)`+無音/`Ok`）— 新たな wire メッセージは送らない
- `kill_child` は `#[doc(hidden)]` の検証支援 — 公開 API としての安定経路ではない

## Non-goals

- 自動再起動・supervisor（#31 確定 — design deferred）・bundle 子・共有リーパー/io 多重化
- L1 GUI（`OpenGui`/`CloseGui` は `Unsupported` で拒否 — design deferred）
- 実プラグイン（CLAP/VST3/OFX）の L1 E2E — fixture（`l1-test-worker`・`ruviform-plugin-l1-runner`）で検証
- `process` 経路の RT 化・video/NONRT プラグインの L1 経路
- wire codec 自体の契約（`plugin-l1-wire-codec-tests.md` が担当）・runner 側の結合（`plugin-l1-runner-e2e-tests.md` が担当）
- stderr ログ配管・shm 通知機構の多重化（doorbell はパイプ多重化で十分）
- macOS/Linux でのローカル実証 — Windows ローカルのみ実証・3 OS matrix は CI `verify` ジョブに委譲（design 採用条件・platform notes 下記）

## Platform notes

- **検証範囲**: ローカル検証は **Windows のみ実証**（本計画の作業環境）。macOS/Linux のコンパイル・E2E は CI `verify` ジョブ（ubuntu-latest/windows-latest/macos-latest・`workflow_dispatch`・`verify-ci` 経由 `cargo test --workspace`）に委譲 — design の `shared_memory` 採用条件「3 OS でのコンパイル/E2E」を満たす。計画書・task は未実行の 3 OS 実証を主張しない
- **shm 命名**: `shared_memory` の named segment はプラットフォーム差を持つ（Windows: `Global\` 名前空間オブジェクト・POSIX: `/dev/shm` ファイル）。key は proxy が `pid + slot + counter` で生成する一意名 — `OpenShmem`/`ShmemConf::os_id` 形状の差を internal `shm` モジュールで吸収し、key 生成・unlink 規則を crate 内に閉じ込める
- **shm cleanup**: 所有権は proxy — `Destroy`（`Shutdown` + reap）・drop で unlink。子の attach は `os_id` のみを使う（unlink 責務は持たない）。異常終了で残った segment は `Create`/`Open` の冪等性ではなく unlink 前の key 一意性（counter）で衝突回避する
- **`ChildExit.signal`**: unix のみ `Some`（Windows は `None`）— `try_wait` の `ExitStatus` から `signal()`/`code()` で取得
- **`l1-test-worker` / `ruviform-plugin-l1-runner`**: `CARGO_BIN_EXE_*` は統合テスト（`tests/`）でのみ有効 — crate 内に閉じる（cross-package の bin 解決に依存しない）

## Risk

- クラッシュ検出の冪等収束の誤り（二重 `Failed` 遷移・通知漏れ・pending waiter の永久待機）→ E2E で kill/crash 両経路・`Ipc` 起床・通知一回性を固定
- voluntary exit 0 を clean と誤区別（`Shutdown` 前の exit 0 は `Failed` であるべき）→ ライブ中 exit は code 不問 `Failed` を固定
- shm データ面のメモリ順序・slot 再利用規約の誤り（fence 欠落・`seq % D` 衝突・制御状態を shm に混入）→ doorbell fence・厳密交互制御・staging 複写の構造を固定
- pass-through 規則の誤り（クラッシュでエラーを返す・直前受信でなく無音を返す・ハング）→ `Ok(CONTINUE)` + 直前受信 or 無音を固定
- `shared_memory` の公開 API 露出・自前 `unsafe` 混入（採用条件の破綻）→ internal `shm` モジュール隔離を構造で固定
- RT 脚への alloc/blocking 流出（`process` で pipe/shm 直接アクセス・待機）→ `assert_no_alloc` 窓の `rt_l1_process_path_no_alloc` + `rt_` 命名で固定
- Windows/POSIX の shm 名・unlink 規則差（platform notes どおり CI に委譲 — ローカルは Windows 実証のみ）
- `command_timeout`/`Instantiate` 失敗での child leak（kill/reap 漏れ）→ handshake 失敗時の kill/reap を固定
