---
id: "PLAN-034OnRCESpswfHetayj7CH"
type: "plan"
status: "approved"
title: "Issue #121 実施計画 — L1 diagnostics（observability crate・audio counters・stderr drain・FailureCause・pre-mint diagnostics）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-121-diagnostics.md"
  implemented-by:
    - "docs/tasks/create-ruviform-observability-crate.md"
    - "docs/tasks/add-failure-cause-and-spawn-failed.md"
    - "docs/tasks/add-cause-to-instance-failed.md"
    - "docs/tasks/wire-l1-audio-counters.md"
    - "docs/tasks/add-l1-diag-record-and-registry.md"
    - "docs/tasks/pipe-and-drain-l1-stderr.md"
    - "docs/tasks/wire-failure-cause-classification.md"
    - "docs/tasks/add-pre-mint-diagnostics.md"
    - "docs/tasks/add-l1-diagnostics-unit-tests.md"
    - "docs/tasks/add-l1-diagnostics-e2e.md"
    - "docs/tasks/verify-l1-video-diagnostics-parity.md"
    - "docs/tasks/l1-diagnostics-ledger-and-verify.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/impl/ir/rt-observability-poc.md"
    - "docs/designs/issue-117-bundle-child-process.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-121-diagnostics.md"
    - "docs/designs/issue-121-l1-diagnostics.md"
    - "docs/adr/issue-121-l1-diagnostics-policy.md"
    - "docs/discovery/issue-121-diagnostics.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-121-l1-audio-counters-tests.md"
    - "docs/test-specs/issue-121-l1-stderr-drain-tests.md"
    - "docs/test-specs/issue-121-l1-diagnostics-retention-tests.md"
    - "docs/test-specs/issue-121-l1-failure-cause-notification-tests.md"
    - "docs/test-specs/issue-121-l1-parity-nonregression-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #121 実施計画 — L1 diagnostics（observability crate・audio counters・stderr drain・FailureCause・pre-mint diagnostics） Implementation Plan

## Goal

`ruviform-observability` を新設し、L1 audio の RT-safe 計数・全 L1 子の piped stderr drain・`FailureCause` による失敗原因通知・pre-mint 診断レコードを、approved 設計どおりに実装する。

## Scope

単一の実装ストリームとして扱う — すべての変更が `ruviform-plugin-host` の L1 spawn/IO/収束経路とその公開面に集中し、同じ critical path（`l1/mod.rs` の `Shared`/`Inner`/`InstShared`・spawn 経路）を共有するため plan 分割はしない。設計上明確に独立しているのは `ruviform-observability` クレート本体と `FailureCause`/`HostError` 型追加であり、いずれも先に独立して着地可能だが、同じリリース意図の下流配線が本体であるため単一 plan の前段タスクとして扱う。

Non-goals（spec Out of Scope の再掲）: wire codec・`WIRE_VERSION`・`Hello.capabilities`・`WireError` tag の変更、`ruviform-foundation` 新設、`rt_bridge` 実装、構造化 logging、I/O 多重化（#120）、supervisor の原因分岐、`HostError::Ipc` への payload 追加、`BundleFailed` 相関通知 variant。

## Upstream Documents

- [spec](../specs/issue-121-diagnostics.md)（approved — R-B1..R-B8・AC1..AC7）
- [ADR](../adr/issue-121-l1-diagnostics-policy.md)（accepted — option A・source break 受容）

## Design Inputs

- [design](../designs/issue-121-l1-diagnostics.md)（approved — 承認前項目 1–5 の解決記録・Verification Notes）
- [discovery](../discovery/issue-121-diagnostics.md)（全コード観測・decision matrix）
- #104 PoC: `docs/impl/ir/rt-observability-poc.md`（counter pair・drop-on-full+count・`reserved: [u64; 2]` 規約・RT 経路 logging 不採用）
- #101 foundation: `docs/designs/foundation-observability-minimal-api.md`（依存方向 — plugin-host が observability に下向き依存、`ruviform-rt` への逆依存は追加しない）

## File Map

| Path | Responsibility | Notes |
| --- | --- | --- |
| `Cargo.toml`（workspace root） | `members` に `crates/ruviform-observability` を追加 + members 先頭の per-Issue rationale コメント | `publish.workspace = true` で `publish = false` を継承 |
| `Cargo.lock` | 新 member・新依存で再生成 | tracked — commit に含める（lockfile 規約） |
| `crates/README.md` | 新クレートの層・責務・依存方向を索引 + クレート数カウント更新 | 層構造は architecture baseline の DAG に整合 |
| `crates/ruviform-observability/Cargo.toml` | 新クレート定義 | 依存なし（最小） — `#104` counter pair 形状を参考に薄い API |
| `crates/ruviform-observability/README.md` | クレート責務・RT 制約・初消費者 L1 diagnostics の記述 | #101 trigger「最初の実需要」の証跡 |
| `crates/ruviform-observability/src/lib.rs` | 共有 counter pair（writer=`Relaxed` atomic 更新のみ・reader=非 RT snapshot）と最小公開 API | RT-suitable writer・生成/clone/drop は非 RT・`reserved: [u64; 2]` 規約は参照形 |
| `crates/ruviform-plugin-host/Cargo.toml` | `ruviform-observability` への `path` 依存 + **platform-gated FFI 依存**: unix `libc`（`poll`）、windows `windows-sys`（`Win32_System_Pipes`,`Win32_Foundation` — `PeekNamedPipe` は `Win32::System::Pipes` 所在） | 下向き依存のみ。`poll`/`PeekNamedPipe` は std に存在しない — 依存理由を PR/README に記録 |
| `crates/ruviform-plugin-core/src/boundary.rs` | `FailureCause`（`Copy`・`#[non_exhaustive]`・7 variant）と `HostError::SpawnFailed { cause, diagnostics_id }` を追加 | `HostError` が plugin-core 所在のため同所配置 |
| `crates/ruviform-plugin-core/src/wire.rs` | codec 互換テスト追加（`encode_host_error(SpawnFailed)` → `Err(UnsupportedVariant)`・`WIRE_VERSION` 不変・`FailureCause`/`HostNotification`/`ChildExit` は wire 対象外） | `encode_host_error` は wire.rs:1898・`_ => Err(UnsupportedVariant)` catch-all（:1947）が新 variant を自動的に local-only 扱い |
| `crates/ruviform-plugin-host/src/lib.rs` | `FailureCause` を re-export リスト（`HostError`/`ChildExit` と同列）に追加 | 公開 `InstanceFailed.cause` の型を downstream が命名可能にする |
| `crates/ruviform-plugin-host/src/l1/diag.rs` | 子スコープ診断レコード（64 KiB tail 優先 ring・`total_read_bytes`/`dropped_bytes`/flags・`Mutex` 保護 snapshot）・`DiagRegistry`（host retention 8 件・登録時連続採番 `diagnostics_id`・`evicted_watermark`/`issued_max`/`evicted_records`・found/evicted/not-found query）・drain thread 本体（案A） | 新規モジュール — `l1/mod.rs`（~5750 行）をさらに肥大化させない責務境界。**`#[path]` harness discipline**: `crate::` パス禁止（harness では `crate::` が test crate root を指すため — `tests/l1_proxy_e2e.rs:16-17` の graft）。参照は crate 名（`ruviform_plugin_core`/`ruviform_rt`/`ruviform_observability`/`libc`/`windows_sys`/`std`）か `self::`/`super::` で行う — whitelist ではなく「`crate::` を使わない」が不変条件。FFI はこのモジュール内 `#[expect(unsafe_code)]` + SAFETY コメントに閉じ込める |
| `crates/ruviform-plugin-host/src/l1/mod.rs` | `spawn_scanned` の `.stderr(Stdio::inherit())`（:1852）→ `piped`・`spawn_child` が `child.stderr.take()` で取得（~:1164）・`SpawnRx` に stderr+診断レコード handoff・spawn 系関数に `diag: &DiagRegistry` param・`Shared`/`Inner` に cause 記録・drain 起動（`Hello` 待ち前）・`STDERR_DRAIN_GRACE`(500ms) 定数・`push_result` で `results_dropped` 計装 | spawn-scoped owner 構造を維持 — `Child` 生成から drain 起動までの間に cancellable 処理を挟まない。失敗時のレコード登録は `Err` を返す関数内で行う（`Shared` drop 後も registry の `Arc` が所有） |
| `crates/ruviform-plugin-host/src/l1/data.rs` | `RtLeg::process` の計装点（`dropped_submits`・slot busy/`try_push` 失敗で block ごと最大 +1）・**`passthrough_submits` は `RtLeg::pass_through` 内部で計装**（`data.rs` 2 call site + `endpoint.rs:549` の arm-anomaly 経路を両方捕捉・warmup 込み）・`drain_to` で `results_failed_observed` 初回 pop 計装 | writer は Relaxed 更新のみ・RT 契約無退行 |
| `crates/ruviform-plugin-host/src/endpoint.rs` | `RtLeg::new` counter 引数対応 — 実サイト 10 箇所（`endpoint.rs:780,1083`・`rt_endpoint_arm_apply_no_alloc`:1295・production `mod.rs:1988`・in-crate test `mod.rs:3385,3784`・`data.rs:968`・harness `l1_proxy_e2e.rs:1185,1305,1497,1691`） | `leg.pass_through(outputs)`（:549）は `RtLeg::pass_through` 内部計装で自動的に捕捉。harness/test サイトは `InstShared` 無しで pair を fresh 生成する経路が必要 |
| `crates/ruviform-plugin-host/src/composite.rs` | `HostNotification::InstanceFailed` に `cause: FailureCause` field + variant レベル `#[non_exhaustive]`・通知生成/合成サイトで cause 供給・非 RT query 面（per-handle counters・stderr snapshot・diagnostics record query）・`PluginHost` が `Arc<DiagRegistry>` を保持 | 変更サイト: 生成 ~1165（cause 供給）・完全 pattern destructure ~2645（`..` 付与）。`:2983` の `InstanceFailed{handle: reported, ..}` は既に `..` 付きで無変更 — 構築×1 + destructure×1 + supervisor 合成×1 が実数 |
| `crates/ruviform-plugin-host/src/supervisor.rs` | `SupervisionEvent` 合成サイトで `cause` 供給（~1328）・watched 消費/Unsupervised pass-through は無変更 | `{handle, ..}` pattern（~933）は無影響 |
| `crates/ruviform-plugin-host/src/l1/video.rs` | video dedicated 子の spawn 経路（`VideoL1Instance::spawn` ~:150-165）にも同じ stderr drain + cause 通知経路を通す — `spawn_scanned`・notify 機構を共有するため主な配線は `l1/mod.rs` 側 | `src/video.rs`（`VideoPluginHost` facade）は spawn/IO を持たず変更不要。counter は video に載せない |
| `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs`・`src/bin/l1-test-video-worker.rs` | stderr 量 fixture knob を両 worker に同一 flag 名で追加: `--stderr-bytes=N`（起動時・Hello 前に N bytes 出力）+ `--heir-stderr-bytes=N --heir-stderr-delay-ms=M`（heir が遅延出力 — `unread_bytes_observed`/`drain_deadline_exceeded` 検証用） | 現行 fixture は usage 拒否の 1 行 `eprintln!` のみ — >64 KiB の制御出力が無いと T10/T11 の基準が実行不能。両 worker で flag 名を揃える（video parity） |
| `crates/ruviform-plugin-host/tests/*.rs`・`crates/ruviform-plugin-{clap,vst3}/tests/l1_real_adapter_e2e.rs` | `InstanceFailed { handle, exit }` 完全 pattern ×5 ファイル → `..` 付与、構築サイトへ `cause` 供給、spawn/Instantiate 経路の `Err(HostError::Ipc)`/`expect_err` assert ~12 件 → `matches!`/`SpawnFailed{cause,..}` 移行（`diagnostics_id` は実行時採番のため値 assert 不可）、新規 E2E | post-mint の waiter wakeup `Err(Ipc)`（例 `l1_proxy_e2e.rs:230/273/320`・`l1_video_e2e.rs:1251`）は変更しない — 区別が必要。l1_real 両ファイルは `#![cfg(feature="host")]` ゲート — `lint-l1-real-plugin` lane でのみ compile される |
| `docs/`（ledger・index） | #120 再トリガ記録（per-child 3 スレッド目）・#101「最初の実需要」記録・各 index 更新 | spec R-F3・AC7 |

## Implementation Sequence

- [x] T1. `ruviform-observability` クレート新設 — `Cargo.toml`・`lib.rs`（counter pair: `Arc<AtomicU64>` writer + 非 RT snapshot reader — **複数 counter の逐次 read は非整合 snapshot であることを API doc に明記**）・README・`crates/README.md` 索引・workspace `members` 登録・単体テスト（writer 更新→reader 観測・Relaxed・clone 共有）
- [x] T2. `FailureCause` + `HostError::SpawnFailed` — `boundary.rs` に `FailureCause`（`Copy`・`#[non_exhaustive]`・7 variant + doc に分類表）と `HostError::SpawnFailed { cause: FailureCause, diagnostics_id: u64 }` を追加、in-crate exhaustive match の修正、`wire.rs` に codec 互換テスト（`encode_host_error(SpawnFailed)` → `Err(UnsupportedVariant)`・`WIRE_VERSION` 不変）、`mise exec -- cargo doc -p ruviform-plugin-core` 警告なし
- [x] T3. `HostNotification::InstanceFailed` に `cause` + variant レベル `#[non_exhaustive]` — composite.rs の型変更、全構築サイトで cause 供給（この時点では分類配線前なので暫定値 `Internal` でよい — T7 で正しい分類に置換）、完全 pattern ×5 ファイル（`l1_real_adapter_e2e`×2・`l1_video_e2e`・`l1_supervisor_e2e`・`l1_proxy_e2e` — `plugin_graph_e2e.rs:428` は既に `..` 使用で無変更）に `..` 付与、supervisor 合成サイト更新。**compile 確認は `mise run check` + `mise run lint` + `mise run lint-l1-real-plugin`** — l1_real 両ファイルは `#![cfg(feature="host")]` で既定 target 外のため lane 必須（crate 境界 compile の承認条件）
- [x] T4. audio counters — **`ruviform_observability::{Counter, CounterReader}` pair 型を必須使用**（裸 `Arc<AtomicU64>` 直置きは path dep 死蔵・R-F1 違反）。4 pair（`dropped_submits`/`passthrough_submits`/`results_failed_observed`/`results_dropped`）は **`InstKind::Audio` arm が所有**（`InstShared` 本体 field 不可 — `InstKind::Video` エントリも持つため video に counter が乗る = R-B6 非混入 pin 違反）。計装は設計表どおり: slot busy/`try_push` 失敗は block ごと最大 +1・**`passthrough_submits` は `RtLeg::pass_through` 内部で計装**（`data.rs` の 2 call site と `endpoint.rs:549` の arm-anomaly 経路を両方捕捉・warmup 込み・「出力が実際に成立した地点」）・`drain_to` 初回 pop・reader `push_result` enqueue 失敗。`RtLeg::new` 実サイト 10 箇所（`endpoint.rs:780,1083`・`rt_endpoint_arm_apply_no_alloc`:1295・`mod.rs:1988,3385,3784`・`data.rs:968`・harness `l1_proxy_e2e.rs:1185,1305,1497,1691`）を counter 引数に対応（harness/test は fresh pair 生成経路が必要）。composite の非 RT query 面（handle→`CounterReader`、destroy 後 not-found）、re-mint/leg swap 寿命の確認
- [x] T5. `l1::diag` — 子スコープ診断レコード型（`Mutex` 保護 64 KiB tail 優先 ring・`total_read_bytes`/`dropped_bytes`・flags）・`StderrSnapshot`（`bytes: Vec<u8>` を **bytes のまま**返す — String 変換しない・同一時点組み立て・`total_read = len + dropped` 不変条件）・`DiagRegistry`（host retention 8 件・上限超過は最古 evict + **`evicted_records: u64` 計数**・登録時連続採番 `diagnostics_id: u64`・`evicted_watermark`/`issued_max`・`DiagnosticQuery::{Found, Evicted, NotFound}`）の型と単体テスト。**stderr は他 sink へ暗黙 echo しない**（設計 — retention/query のみ）。命名は既存 `arm_retire_overflows`/`DropCollector::overflows` 系の `_overflows`/`_dropped` 語彙と整合確認（spec R-V7）。`#[path]` harness discipline: `crate::` パス禁止
- [x] T6. stderr piped + drain — **`spawn_scanned`** の `.stderr(Stdio::inherit())`（mod.rs:1852）→ `Stdio::piped()`、`spawn_child` が `child.stderr.take()` で取得（stdout と同列）、`SpawnRx` 拡張（stderr+record handoff — payload は新しい record 型）、`Hello` 待ち前に drain thread 開始、案A 実装（POSIX `libc::poll` / Windows `windows-sys` `PeekNamedPipe`+短 sleep — **両者は `l1/diag.rs` 内の `#[expect(unsafe_code)]` + SAFETY コメントで局所化**。platform-gated dep は plugin-host `Cargo.toml` に追加）、`STDERR_DRAIN_GRACE`=500ms 子終了観測起算・延長なし・EOF 即終了・終了時 read fd close・JoinHandle detach、`unread_bytes_observed`/`drain_deadline_exceeded`/`eof` 記録、drain 初期化失敗時は既存 abort 経路で rollback（partial child を残さない）。**fixture knob**: `l1-test-worker.rs`/`l1-test-video-worker.rs` に `--stderr-bytes=N` + `--heir-stderr-bytes=N --heir-stderr-delay-ms=M` を同一 flag 名で追加
- [x] T7. cause 分類配線 — `Inner` に minted instance ごとの `Option<FailureCause>` + 通知状態を保持、first-observed 固定（同一 `inner` lock 内・上書き禁止）、分類表の全観測点へ配線（reader: `Hello`/EOF/decode、demux: child-scope/instance-scope violation、driver: stdin write・command timeout・watchdog kill 前記録、`try_wait`/reap: `ExitObserved`/`Internal`、spawn: `Spawn`/`Handshake`/`Timeout`）、通知へ `cause` を伝搬（`{None,None}` status でも fixed cause で送る）、T3 の暫定値を実分類に置換。**変更面の注意**: `converge_child`/`converge_instance` の cause 引数 or `Inner` 記録で ~33 call site、notify channel payload（`Receiver<ChildExit>` → cause 同送）+ `Shared`/`InstShared` literal ~13 site — 全て compile-caught だが task-doc で site 表を持つこと
- [x] T8. pre-mint 診断 — `DiagRegistry` を `PluginHost` が `Arc` で保持し spawn 系関数へ `diag: &DiagRegistry` 引数で渡す（record 生成は spawn 経路内・**失敗確定時は `Err` を返す関数自身が登録** — `Shared` drop 後も registry の `Arc` がレコードを所有）。失敗レコードに `phase`（pre-spawn 検証 / OS spawn / drain 初期化 / mint・handshake）を記録、`Err(HostError::SpawnFailed { cause, diagnostics_id })` 返却、cleanup 失敗は `cleanup_error` 副次診断（主 cause 不変）、minted 失敗は child finalize で retention 登録 + handle index。**`SpawnFailed` 境界の pin**: 定義済み拒否は variant を保持し `SpawnFailed` で包まない・レコードも登録しない — `Err(HostError::InstanceLimit)`（mod.rs の unflattened match arm :2005/:2123・doc :2433 の契約）と spec-validation `Err(Ipc)`（事前検証）はそのまま。live 子への pooled `create_instance` `Reply::Err` → `SpawnFailed{cause: Handshake}` + **試行レコード**（その時点の子 stderr の frozen snapshot copy — live 子の active レコードは mutate 継続・分離）。**assert 移行**: spawn/Instantiate 経路の `Err(HostError::Ipc)`/`expect_err` ~12 件を `matches!`/`SpawnFailed{cause,..}` に（ID は実行時採番で値 assert 不可 — `cause` を oracle に）。post-mint waiter の `Err(Ipc)` は変更しない。query 面を composite に公開
- [x] T9. 単体テスト拡充 — 設計 Verification Notes の 9 ケース（warmup +3・slot busy +1・block 複数失敗で最大 +1・初回 pop +1・held 再取り出し +0・enqueue 失敗 +1・enqueue 成功未観測 +0・leg swap 維持・re-mint 0）・ring 不変条件・分類表写像・first-observed 競合・watermark found/evicted/not-found・`evicted_records` 計数
- [x] T10. E2E — underrun 発生→計数観測（in-place + endpoint）、stderr 捕捉（`Hello` 前出力を含む — `--stderr-bytes` fixture）・pipe capacity 超過で drain 継続・tail 保持 + `dropped_bytes` 精確・deadline/unread flags（`--heir-stderr-*`）、heir grandchild で bounded teardown（OS 別期待値）、EOF→`Transport`・timeout→`Timeout`・違反→`Protocol` 通知 payload・`ExitObserved` は `--leave-heir-after-msgs` fixture で deterministic に（plain crash は EOF/exit 競合で lock 直列化順 — 断言しない）、bundled fan-out 全 instance、supervisor watched 消費/Unsupervised pass-through、pre-mint 各 `phase` + rollback（partial child 残存なし）、`Err(InstanceLimit)`/`Err(Ipc)` パススルー無退行
- [x] T11. video parity — `l1_video_e2e` で video 子 stderr 配管・render timeout→`Timeout`・protocol 違反→`Protocol` が同一契約で動くこと、audio counter が video route に混入しないこと、video worker の `--stderr-bytes` で同条件検証
- [x] T12. docs/検証 — `docs/designs/plugin-l1-out-of-process.md` 等の trigger ledger に #120（per-child 3 スレッド目）・#101（observability 最初の実需要）記録、`crates/README.md`/index 更新（クレート数カウント）、`mise run fmt-check`/`check`/`lint`/`test`/`test-rt` + `mise run lint-l1-real-plugin` + `mise run test-l1-real-plugin` 全パス、`doc_status.js audit` blocking 0、`git diff --check`、依存追加理由を PR/README に記録（`libc`/`windows-sys` platform-gated）

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | `InstanceFailed { handle, exit }` 完全 pattern の source break — workspace 内限定だが見落としがあると compile 失敗（l1_real 両ファイルは `#![cfg(feature="host")]` で既定 gate 外） | med | low | 5 ファイルを洗い出し済み（`composite.rs`×3・`supervisor.rs`・test×5）。T3 で `check`+`lint`+**`lint-l1-real-plugin`** を全 lane にかけて網羅性を機械確認 |
| 2 | drain thread の Windows 残余ブロック（`PeekNamedPipe` 非保証）で teardown が deadline を超過 | low | med | 合意済み受容リスク — `drain_deadline_exceeded` flag で観測可能化、E2E は OS 別期待値で記述 |
| 3 | RT 計装が alloc/lock を混入させ `rt_l1_*` テストを退行させる | low | high | writer は `fetch_add(Relaxed)` のみ・`InstShared` 既存 `Arc` 共有で alloc なし — AC2 で既存 assert_no_alloc 窓を無退行確認 |
| 4 | retention 8 件が高頻度 spawn 失敗で早期 eviction → 診断 query が evicted を返す | med | low | 仕様どおり（bounded retention）— `evicted`/`not-found` 区別で caller が検出可能、設計契約どおり guaranteed delivery を要求しない |
| 5 | 外部依存の追加 | none→low | low | `ruviform-observability` は依存なし。**`libc`/`windows-sys` のみ platform-gated で plugin-host に追加**（`poll`/`PeekNamedPipe` は std 不在 — design 承認済み案A の必要 dep）。`unsafe` は `l1::diag` 内 `#[expect(unsafe_code)]`+SAFETY コメントに閉じ込め、依存理由を PR/README に記録 |
| 6 | `kill` 失敗 + 子生存で rollback `wait()` がブロックする理論経路（既存契約の residual） | low | med | 現行契約をそのまま引き継ぐ設計承認済み — 本 Issue で新規導入しない |
| 7 | `Err(HostError::Ipc)` assert ~12 件の `matches!` 移行で oracle が弱まる（`cause` を assert しないまま variant のみ確認） | med | low | T8 の移行基準を明記 — `SpawnFailed{cause,..}` で `cause` を oracle に。post-mint waiter の `Err(Ipc)` 識別子付きで区別 |

## Task Breakdown

task-doc への分割単位は Implementation Sequence の T1–T12 とする。依存関係:

| Task | Depends on | Blocks |
| --- | --- | --- |
| T1 observability crate | — | T4 |
| T2 FailureCause/HostError | — | T3, T7 |
| T3 InstanceFailed `cause` + 呼出サイト | T2 | T7 |
| T4 audio counters | T1 | T9, T10 |
| T5 `l1::diag` record/ring/retention | — | T6, T8 |
| T6 stderr piped + drain | T5 | T8, T11 |
| T7 cause 分類配線 | T2, T3 | T8, T9, T10, T11 |
| T8 pre-mint 診断 | T5, T6, T7 | T9, T10, T11 |
| T9 単体テスト拡充 | T4, T7, T8 | T12 |
| T10 E2E（audio/bundled/supervisor/pre-mint） | T4, T8 | T12 |
| T11 video parity | T6, T7, T8 | T12 |
| T12 docs/最終検証 | T9, T10, T11 | — |

独立して先行可能: T1・T2・T5。critical path: T5→T6→T7→T8→T10→T12（T7 は T2→T3 の型変更にも依存）。

## Verification

| Step | Verification command or criteria | Pass condition |
| --- | --- | --- |
| T1 | `mise run check` + `mise exec -- cargo test -p ruviform-observability` | counter pair 単体テストがグリーン |
| T2 | `mise run check` + `mise exec -- cargo doc -p ruviform-plugin-core` + `mise exec -- cargo test -p ruviform-plugin-core` | 警告なし・codec テスト: `encode_host_error(SpawnFailed)` → `Err(UnsupportedVariant)`・`WIRE_VERSION` 不変 |
| T3 | `mise run check` + `mise run lint` + **`mise run lint-l1-real-plugin`** | workspace 全 target + `host` feature lane で compile 通過（完全 pattern/構築サイトの網羅修正を機械確認 — 通知互換の承認条件。l1_real 両ファイルは既定 gate 外のため lane 必須） |
| T4 | `mise run test`（既存 rt_ を含む） | `rt_l1_*` assert_no_alloc 窓が無退行（AC2）・counter 単体テスト・endpoint arm-anomaly 経路も計数 |
| T5 | `mise exec -- cargo test -p ruviform-plugin-host` diag 単体 | ring 不変条件 `total_read = len + dropped`・watermark 3 値識別・`evicted_records` 計数 |
| T6 | `mise exec -- cargo test -p ruviform-plugin-host` + `l1_proxy_e2e` | stderr 捕捉（`--stderr-bytes`）・飽和で drain 継続・bounded teardown（OS 別） |
| T7 | `mise exec -- cargo test -p ruviform-plugin-host` 分類単体 + `l1_proxy_e2e` | 分類表各行 → 正しい `FailureCause`・first-observed 固定 |
| T8 | `mise exec -- cargo test -p ruviform-plugin-host` + `l1_proxy_e2e` pre-mint | `phase`/`cause` 区別・rollback で partial child なし・ID query 3 値・`Err(InstanceLimit)`/`Err(Ipc)` パススルー無退行 |
| T9 | `mise run test` | 9 ケース全て期待値 |
| T10 | `mise run test` | 全 E2E グリーン・通知 exactly-once 無退行 |
| T11 | `mise exec -- cargo test --test l1_video_e2e` | video 子 stderr/cause parity・counter 非混入 |
| T12 | `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` + `mise run lint-l1-real-plugin` + `mise run test-l1-real-plugin` + audit | 全パス・`blocking 0`・`git diff --check` クリーン |

ワイヤ互換性の承認条件（設計 Verification Notes）: T2 で `wire.rs` の codec テストにより `SpawnFailed` が `encode_host_error` で `UnsupportedVariant` になること・`WIRE_VERSION` 不変を確認。`FailureCause`/`HostNotification`/`ChildExit` は wire 型ではないため serialize 対象外 — codec テストは `HostError` 系の網羅で担保する。

## Rollback Strategy

- **新クレート**: `Cargo.toml` members・`crates/ruviform-observability/`・plugin-host の dep 行を含む commit を `git revert` — 他のタスクに先行するため依存タスク未着手なら単独 revert 可能。
- **API 追加**（`FailureCause`・`HostError::SpawnFailed`・`InstanceFailed.cause`）: 全て workspace 内公開面 — revert commit で完全に除去できる。外部 publish/永続化形式は存在しない（`publish = false`）。
- **stderr piped**: `Stdio::inherit` への差し戻しは `spawn_scanned` の 1 箇所（mod.rs:1852）— ただし drain/diag は additive なので revert は T6 以降の commit 単位で行う。platform-gated `libc`/`windows-sys` dep 行も同 commit で除去。
- **Trigger**: lint/test 失敗・RT 退行・wire 互換性違反が修正不能と判明した場合。**Procedure**: 該当 task の commit 単位で `git revert`。**Verification**: revert 後に `mise run check`/`test`/`test-rt` が元どおりグリーン。

## Open Questions / Gaps

- query 面の正確な公開名（`l1_counters(handle)`・`l1_stderr(handle)`・`l1_diagnostic(id)` 案）は T4/T8 の実装時に既存 `PluginHost` メソッド規約に合わせて確定 — 設計上の契約（粒度・意味論）は確定済み。
- drain 起床周期の具体値（設計は 5–10 ms 想定）は T6 で定数として確定 — `DRIVER_POLL_INTERVAL`（200 µs）と `STDERR_DRAIN_GRACE`（500 ms）の中間で選ぶ。
- `l1::diag` の構造（単一モジュール内の型配置・drain との所有境界）は T5/T6 で実装形に合わせて決定 — 責務分離の意図は設計どおり維持。

## Self-Review

- [x] Every spec requirement maps to a task — R-B1→T4/T9、R-B2→T4（rt_ 無退行）・T12、R-B3→T5/T6/T10/T11、R-B4→T2/T3/T7/T8/T10、R-B5→T2/T12 codec 検証、R-B6→T11、R-B7→T10 無退行、R-B8→T10 supervisor。R-F1→T1、R-F2→T4 query 面、R-F3→T6/T12 ledger、R-F4→T5、R-F5→T3、R-F6→T7。AC1→T4/T10、AC2→T4/T12、AC3→T6/T10、AC4→T7/T8/T10、AC5→T2/T12、AC6→T10/T11、AC7→T12。
- [x] No placeholders remain — 全計装点・定数・API 形は設計の確定値を引用、未確定は Open Questions に明示。
- [x] File paths, relations, status values, and terminology are consistent — 修正サイトは設計の同定結果どおり、`FailureCause`/counter 名/flags は設計語彙。

## Implementation Handoff

- Recommended mode: inline execution — T5–T8 は `l1/mod.rs` 同一ファイルへの密な配線変更が連鎖し、並列委譲では同一箇所の競合が発生しやすい。T1・T2・T5 の先行独立タスクのみ委譲可能（ユーザー承認時のみ）。
- Independent work streams: T1（新クレート）・T2（plugin-core 型追加）・T5（`l1::diag` 新規モジュール）は相互独立 — 他は `l1/mod.rs`/`data.rs`/`composite.rs` の共有 file map を持つため逐次。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 設計上の全判断（counter 意味論・stderr 保持/容量・drain OS 別保証・diagnostics ID・rollback・FailureCause 境界・通知互換方針）は approved 設計で確定済み — 本 plan はその実装順序と検証割当のみを担う。レビューの焦点は「タスク粒度と検証割当が抜けを持たないか」。
- 前提: `HostError`/`FailureCause` は plugin-core `boundary.rs` 配置（`encode_host_error` の `_` catch-all が local-only を保証 — T2 で機械確認）。`l1::diag` 新規モジュールは `l1/mod.rs` 肥大化抑制の責務境界。
- 残存リスク: drain の Windows 残余ブロック・retention 早期 eviction は設計上受容済み — flag/query 3 値で観測可能化する実装を T6/T8 で確認。
