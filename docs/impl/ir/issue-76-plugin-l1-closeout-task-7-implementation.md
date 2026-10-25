---
id: "IMPL-034Map3xCZ4qfcYjTbEmx4"
type: "impl"
status: "completed"
title: "Issue #76 プラグイン L1 分離 closeout — Task 7 implementation"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-76-plugin-l1-closeout-task-7-implementation.md"
    modified:
      - type: "file"
        path: "docs/tasks/plugin-l1-closeout.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-wire-codec-tests.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-proxy-contract-tests.md"
      - type: "file"
        path: "docs/test-specs/plugin-l1-runner-e2e-tests.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/designs/plugin-l1-out-of-process.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/plugin-l1-closeout.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #76 プラグイン L1 分離 closeout — Task 7 implementation

## Summary

Issue #76 Task 7 完了 — canonical closeout（実績 IR・coverage
記述・task/plan status 更新・test-spec confirmed 更新・design
doc `compiler_fence` 文言同期）を immutable な実 log のレビュー
のみで実施（新規メトリクス再計算なし）。全 7 task の実装・gate・
独立レビュー修正が commit 済み証跡で確認できたため、task 文書を
`done`・plan `status` を `"completed"` へ遷移させ、`verified-by`
の 3 test-spec（`plugin-l1-wire-codec-tests`・
`plugin-l1-proxy-contract-tests`・`plugin-l1-runner-e2e-tests`）
を実結果で確認済み（schema 上の対応 status = `approved` — #99
先例）へ更新し、索引 4 件（tasks・impl/ir・plans・test-specs
README）を同期した。Task 4 deviation として記録済みの
`compiler_fence` → `std::sync::atomic::fence` 強化について、
正本 design doc の文言を実装側へ同期した（test-spec 内の同一
contract 記述も併せて同期）。本 task は doc-only — コード変更
なし。**GitHub PR merge は lead の authoritative `gh` で別途実施
— 本 IR および本 commit のいずれの文書も PR merged / issue
closed の主張を含まない。**

## Context

approved plan `docs/plans/implement-issue-76-plugin-l1-out-of-process.md`
（PLAN-034LNXQmQvHjRBD57FCPBw）の Task 7
（`docs/tasks/plugin-l1-closeout.md`・action 19）。upstream:
approved spec `docs/specs/plugin-l1-out-of-process.md`
（SPEC-034LNeyep4t7pIzFlpPyTK・7 AC）・approved design
`docs/designs/plugin-l1-out-of-process.md`
（DESIGN-034LNAalREIHyUQNVXLLin）・Task 1–5 の全 IR・Task 6
final gate の 9 本の実 log（commit `0b9d99e`）。本 task は実装
作業を持たず、branch `feature/issue-76-plugin-l1` 上の全完了
証跡を帳簿へ反映する最終段。

## Implementation

### Changes（doc-only — コード変更なし）

- `docs/impl/ir/issue-76-plugin-l1-closeout-task-7-implementation.md`
  （新規）: 本 IR — 全 issue スコープ・per-task commit chain・
  AC coverage 写像・gate 証跡・review 集計・cross-task
  follow-up 集約・spec/design relation 状態を集約記録。
- `docs/tasks/plugin-l1-closeout.md`: `status` → `done`・
  `owners` → `scarecrow173`・`implemented-by` → 本 IR・
  `changes` に本 task で触れたファイル列を記録・verification
  checkbox 2 件を実実行後に [x] 化。
- `docs/tasks/README.md`: `TASK-034LNYvBEBtmjJIUa6xJMi` 行の
  status を `done` へ同期。
- `docs/impl/ir/README.md`: 本 IR の行を追記。
- `docs/plans/implement-issue-76-plugin-l1-out-of-process.md`:
  frontmatter `status` → `"completed"`・冒頭 progress note を全
  7 task 完了（Task 6 commit `0b9d99e` 含む）へ更新 —
  merged 主張なし。
- `docs/plans/README.md`: `PLAN-034LNXQmQvHjRBD57FCPBw` 行の
  status を `completed` へ同期（index-metadata consistency）。
- `docs/test-specs/plugin-l1-wire-codec-tests.md`・
  `docs/test-specs/plugin-l1-proxy-contract-tests.md`・
  `docs/test-specs/plugin-l1-runner-e2e-tests.md`: `status`
  `proposed` → `approved`・body `> status:` 注記を実結果確認済み
  へ更新 — task 文書の「`verified-by` の 3 test-spec を実結果で
  confirmed に更新」の実施（test-spec の valid status 集合は
  `draft`/`proposed`/`approved`/`deprecated`/`superseded` で
  `confirmed` 値は存在しない — ratified-by-actual-results に
  対応する schema 値 `approved` へ写像。#99 先例どおり）。
- `docs/test-specs/plugin-l1-proxy-contract-tests.md`: 併せて
  shm 制御状態なし記述の `compiler_fence(Release/Acquire)` を
  `atomic::fence` へ同期（下記 design doc 同期と同一 deviation
  の反映 — ratify される contract 記述が実装と一致するため）。
- `docs/test-specs/README.md`: 3 test-spec 行の status を
  `approved` へ同期。
- `docs/designs/plugin-l1-out-of-process.md`: §共有メモリデータ面
  の doorbell fence 記述を `compiler_fence` →
  `std::sync::atomic::fence` へ同期 + 強化理由の注記（Task 4
  deviation「実装側を強化・正本 doc は closeout で同期」の
  確定実行 — Task 4/Task 5 IR follow-up の引継ぎ項目を解消）。

### Whole-issue scope（7 task 集約）

| Task | Task 文書 | Actions | Commit | IR |
| --- | --- | --- | --- | --- |
| 前提 | —（discovery/spec/design/plan/tasks 文書パッケージ） | — | `3085c4d` | — |
| 1 | `l1-core-contracts-and-wire-codec.md` | A1–A4 | `8766fd2` | `issue-76-l1-core-wire-task-1-implementation.md` |
| 2 | `plugin-export-crate-and-l1-runner.md` | A5–A8 | `18530b1` | `issue-76-plugin-export-task-2-implementation.md` |
| 3 | `l1-proxy-control-plane.md` | A9–A11 | `c9d8b38` | `issue-76-l1-proxy-control-plane-task-3-implementation.md` |
| 4 | `l1-shm-data-plane-and-rt-legs.md` | A12–A14 | `72bf874` | `issue-76-l1-shm-data-plane-task-4-implementation.md` |
| 5 | `plugin-host-composite-and-l1-e2e.md` | A15–A17 | `2f5a648` | `issue-76-plugin-host-composite-task-5-implementation.md` |
| 6 | `plugin-l1-final-verification.md` | A18 | `0b9d99e` | —（gate log + task 文書のみ・IR なし） |
| 7 | `plugin-l1-closeout.md` | A19 | 本 docs commit | 本 IR |

scope 全体: plugin-core への契約移設（`Plugin`/`PluginFactory`/
`ErasedPlugin`/`erase`/`HostInput`/`HostOutput`/`PluginHandle`/
`GuiHandle`/`GuiParentToken`/`HostError` — host 側は `pub use`
再エクスポートで既存パス無変更）+ `LifecycleState::Failed`/
`LifecycleEvent::Failed`（終端準状態・`Destroy` のみ脱出）+
手書き LE `wire` codec（`WIRE_VERSION = 1`・`WIRE_MAX_*` 上限・
serde 不採用・core は外部依存ゼロ維持）→ `ruviform-plugin-export`
新設（`run(factories) -> ExitCode`・argv 規約・内部 `shm`
モジュール・参照 runner `ruviform-plugin-l1-runner` + 内蔵
`GainPlugin` fixture）→ plugin-host `pub(crate) l1` 制御面
（spawn + `Hello`/`Scan` handshake + driver/reader 2 スレッド +
2 経路冪等 `Failed` 収束 + 通知キュー + `l1::shm`）→
`l1::data` データ面（`BLOCK_SEQ_FLAG` doorbell・`InStage`/
`OutStage` atomic staging・`ruviform-rt` SPSC try-only RT 脚・
pass-through・`l1::shm::View` per-thread attach）→
`PluginHost` コンポジット公開面（`OutOfProcessSpec`/
`HostNotification::InstanceFailed`/`ChildExit`・登録時 routing
`register_factory` L0 / `register_out_of_process` L1 +
`DuplicatePluginId`・統一 `PluginHandle` 空間 — stale handle は
cross-route しない）→ full workspace gate → 本 closeout。

### Acceptance criteria coverage（spec 7 AC ↔ actions/test-specs/log）

| AC | spec 受入基準 | 実施 actions / 証跡 | 検証 |
| --- | --- | --- | --- |
| AC1 | L1 が L0 と同一コマンド系列（scan→instantiate→initialize→activate→start/stop→deactivate→save/restore→destroy）を子プロセス側で実行し同一リプライ/エラー意味（違法遷移は定義済み `Lifecycle` エラー） | actions 9–11（proxy 制御面）・15–17（composite 統一面）・tspec `plugin-l1-proxy-contract-tests` | `l1_proxy_e2e` 42 tests（scan→instantiate→lifecycle→state→destroy parity・`Lifecycle` エラー意味一致・composite 経由の同一リプライ意味）・runner E2E 11（子側コマンド応答）— commit `c9d8b38`/`2f5a648`・final gate `test` log |
| AC2 | `process` 等価経路でブロック送信 → 定義遅延後に `try` ベース読取りで処理済み出力・テストプラグイン（gain）でデータ正当性 | actions 12–14（shm データ面 + RT 脚）・6–8（runner 側 block 処理）・tspec proxy/runner | `l1_proxy_e2e` process 経路 gain 正当性・定義遅延（先行 `lookahead_blocks` = 2 ブロック ≈ 21.3 ms @ 512f/48 kHz）・`l1_runner_e2e` shm ブロック処理 gain — commit `72bf874`/`18530b1` |
| AC3 | RT 側読取り経路が no-block・no-alloc・`rt_` 命名テストが `mise run test-rt` で green | action 14（RT 脚 + assert_no_alloc 測定）・18（full gate 再確認） | `rt_l1_process_path_no_alloc`（assert_no_alloc 窓内 0 alloc 実測）+ `rt_l1_result_leg_try_ops` — `test-rt` 実一致 **111 件**・`rt_l1_*` 2 件実選択 2 passed（final gate log — 0 件スキップでないことを件数で証明） |
| AC4 | 子プロセス kill が wait/exit status で検出・`Failed` 遷移・ホスト通知・以後のコマンド/`process` 等価が定義済み拒否・自動再起動なし | actions 10–11（2 経路冪等収束 + 通知キュー）・15・17（公開通知型 + E2E） | `l1_proxy_e2e` + `composite_e2e` — kill → `InstanceFailed` 通知 exactly once・`Failed` 後コマンド `Err(Failed)`（`Destroy` のみ `Ok`）・`process` → `Ok(CONTINUE)` + 無音・exit 0 自発終了も `Failed`・再起動なし — commit `c9d8b38`/`2f5a648` |
| AC5 | クラッシュ/応答なし状態で `process` 等価が pass-through（直前既受信 or 無音）を返しハングしない | actions 13–14（pass-through 規則 + E2E） | `process_underrun_passes_through_defined_output` 等 — 健全 underrun=直前受信 or 無音・`Failed`=無音・常に `Ok(CONTINUE)`・クラッシュ/ハング時 pass-through・ハングなし — commit `72bf874` |
| AC6 | `ruviform-plugin-export` が workspace メンバー・薄いランナーエントリ・spawn→コマンド応答→process→終了の結合をテスト/モックで検証 | actions 5–8（export 新設 + runner bin + E2E） | workspace member（root `Cargo.toml` + `crates/README.md` 索引）・`run(Vec<Box<dyn PluginFactory>>) -> ExitCode`・`ruviform-plugin-l1-runner` bin・`l1_runner_e2e` 11 tests（`CARGO_BIN_EXE_*` spawn→`Hello`→lifecycle→shm ブロック gain→`Shutdown` exit(0)）— commit `18530b1` |
| AC7 | `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` パス・doc 監査で本チェーン起因 findings 0 | actions 18–19 | Task 6 full gate 全 exit 0（9 log `docs/impl/exp/data/issue-76-final-gate-*-20261025.log`）+ 本 task の doc audit blocking 0・findings [] — commit `0b9d99e` + 本 commit |

### Per-task gate evidence（immutable log・再計算なし）

- Task 1: `fmt-check`・`check`・`lint`・`test`（**49 test
  binaries**）・`test-plugin-core`（**95**）・`lint-plugin-core`・
  `test-rt`（`rt_` 一致 **109 件**）・`cargo doc -p
  ruviform-plugin-host` 0 warnings — 全 exit 0。
- Task 2: `fmt-check`・`check`・`lint`・`test`（**55 suites** —
  runner E2E **11** 含む）・`test-rt` — 全 exit 0。
- Task 3: `fmt-check`・`check`・`lint`・`test`（**55 suites** —
  proxy E2E **20**）・`test-rt` — 全 exit 0。
- Task 4: `fmt-check`・`check`・`lint`・`test`（proxy E2E
  **34**）・`test-rt`（`rt_l1_*` **2 件実一致**）— 全 exit 0。
- Task 5: `fmt-check`・`check`・`lint`・`test`（`composite_e2e`
  **8** 新規・`l1_proxy_e2e` 計 **42**）・`test-rt`（`rt_l1_*`
  2 件実一致）— 全 exit 0。
- Task 6: full gate **9 log 全 exit 0** — `fmt-check`・`check`・
  `lint`・`test`（**55 test binaries・1021 passed・0 failed** —
  L1 E2E 統合テスト含む: `l1_runner_e2e` 11・`l1_proxy_e2e` 42）・
  `test-rt`（**実一致 111 tests・0 failed** — `rt_l1_process_
  path_no_alloc`・`rt_l1_result_leg_try_ops` が `l1_proxy_e2e`
  で実選択・2 passed）・`bench-smoke`・`prek --all-files`（7
  hooks passed・1 skipped=Windows 非対象）・doc audit（**317
  documents・`blocking: 0`・`findings: []`**）+ 補助
  `cargo tree`（host 直接外部依存 `shared_memory` のみ +
  `ruviform-rt` — export も `shared_memory` のみ・外部依存
  クリープなし）— log は `docs/impl/exp/data/
  issue-76-final-gate-*-20261025.log`（commit `0b9d99e`）。

### Independent review findings totals

各 task ともレビューを受け、全 CONFIRMED finding は修正済み・
修正後に該当 gate 再実行で再 green を確認（全 mutation 検証済み
と各 IR に記録）。

- Task 1: 独立 review 2 件 + fix agent — **P1** stale-handle
  generation（`InProcessHost` の slot 再利用で generation が
  進まない pre-existing #74 由来 defect — `generations:
  Vec<u16>` 新設で修正・regression test 追加）・**P2** CLAP
  `STATE_FAILED` 往復 — 指摘全修正。
- Task 2: レビュー修正 — guarded `descriptor()` panic・
  `staging_out` 再ゼロ化・argv exit-2 pin・bounded inbound
  label・`wire::shm` 正本化・`shared_memory` 0.12.4 unsafe
  accessor の scoped 局所許可（`rtrb` 先例の SAFETY 手続き
  どおり）— 全修正。
- Task 3: レビュー修正 — **P1** `Shared.child` 共有 + converge
  側 force-kill・**P2** driver-only `shutdown_initiated`・
  `try_wait` Err kill+reap・reader detach・shm key retry・
  driver write 前 terminal check — 全修正。
- Task 4: レビュー修正 — **P1** `read_out_slot` hostile dims
  publish withhold・**P2** `stage_in` stale plane zero-fill・
  `Submit{seq,generation}` stale-submit ゲート・
  `lookahead_blocks == 0` → `Err(Ipc)`・全 cross-process
  エッジ `compiler_fence`→`atomic::fence` 強化（deviation —
  本 task で正本 doc へ同期済み）。
- Task 5: レビュー修正 — **P2** reply-kind pairing
  `expected_reply_kind`/`host_reply` fail-closed・**P2**
  advisory `stop_processing` mirror lag `err_committed_state`
  + worker `--fail-stop-processing`・deviation 記録 —
  `register_factory` infallible 維持（first-claimant
  shadowing）・`runner_program()` `current_exe`→
  `target/<profile>` 解決・`Box<L1Instance>`
  large_enum_variant。
- Task 6/7: コードレビュー対象の実装変更なし（gate 記録・帳簿のみ）。

### Spec/design/test-spec relation state

- `docs/specs/plugin-l1-out-of-process.md` — `status: "proposed"`
  のまま維持。task 文書の closeout 列挙 scope（実績 IR・
  coverage・task/plan status・3 test-spec confirmed）に spec
  status 遷移は含まれず、#98/#99 先例でも spec は `proposed`
  のまま closeout — status 遷移判断は本 task の権限外として現
  状態を記録（follow-up へ記載）。
- `docs/designs/plugin-l1-out-of-process.md` — `status:
  "approved"`。本 task で fence 文言のみ実装へ同期（下記
  Decisions 参照 — 契約内容の変更ではなく記述の正確化）。
- 3 test-spec — plan/task/spec/design の `verified-by`/
  `verifies` 先・本 task で `approved` へ遷移（上述）。相互
  relation（spec `verified-by` → 3 tspec・design `verified-by`
  → 3 tspec・plan `verified-by` → 3 tspec・tspec `verifies` →
  spec + design）は起票時点で完備済み・変更不要を確認。
- plan `implemented-by`/`derived-by` → 7 task・task 群
  `implements` → plan・task `implemented-by` → 各 IR — 相互
  link 完備済み（Task 7 task 文書の `implemented-by` に本 IR
  を今回追記）。

### Decisions During Implementation

- 実績記述は immutable な実 log（commit history・各 task IR・
  final gate log 9 本）のレビューのみで構成 — 新規メトリクス
  再計算・再検証は行わない（task 文書の expected 契約どおり）。
- plan `status` の `"completed"` 遷移は全 7 task の done が
  commit 済み証跡で確認できた時点でのみ実施 — 未来の完了を
  先取りしない。
- test-spec の "confirmed" は schema 値 `approved` へ写像 —
  test-spec の valid status に `confirmed` は存在せず、実結果で
  ratify された契約文書に対応するのは `approved`（#99 先例）。
- **design doc fence 文言同期** — Task 4 IR の deviation 記録
  「正本 doc の更新は closeout task の canonical doc 同期で
  扱う」と Task 5 IR follow-up「design doc の `compiler_fence`
  文言同期を含む」の確定実行として、design L166 の
  `compiler_fence(Ordering::Release/Acquire)` を
  `std::sync::atomic::fence` へ同期（実装側を弱めず文書側を
  正確化する方向 — compiler fence は同一 process 内の
  compiler 並べ替え規制に留まり cross-process 可視性を保証
  しない・実 fence + pipe syscall が publication boundary）。
  test-spec proxy-contract の同一 contract 記述（shm 制御状態
  なし項）も併せて同期 — ratify 対象の記述を実装と一致させる
  ため。plan 文書内の `compiler_fence` 言及（A12/Test
  Oracles/Risks/Review Handoff）は計画時点の記録として保持
  （status note に deviation を記録済み・歴史文書は書換えない）。
- **GitHub merge は lead の authoritative `gh` のみ** — 文書
  チェックリストで代行せず、本 IR・task・plan のいずれにも PR
  merged / issue closed の主張を書かない。
- `docs/plans/README.md` の status 列も同期 — doc audit の
  index-metadata consistency が plan status を index と照合
  するため。

## Related Experiments

### Adopted

なし — approved plan の確定 closeout 手順のみ。

### Rejected

なし。

## Validation

- `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js
  audit --json` — exit 0・`blocking: 0`・`findings: []`（本
  commit 全編集後に実実行）。
- `rg -q '^status: "completed"$'
  docs/plans/implement-issue-76-plugin-l1-out-of-process.md` —
  exit 0（plan status 遷移後に実実行）。

## Non-Goals / Deferred

- spec `docs/specs/plugin-l1-out-of-process.md` の status 遷移
  （`proposed` → 上位 status）— closeout scope 外・lead が別途
  裁定（#98/#99 先例）。
- plan 文書内 `compiler_fence` 言及の書換え — 計画時点の記録と
  して保持（deviation は status note + 本 IR + design doc 同期
  で帳簿化済み）。
- GitHub PR merge・issue close — 本 bookkeeping の範囲外。lead
  が `gh` で別途実施する。
- コード変更全般 — 本 task は doc-only（follow-up のコード系
  項目は下記 Follow-ups に集約）。

## Risks

- 残存リスクなし — doc-only 変更・全 gate green の証跡は
  branch 上に commit 済み。
- merged 主張がないため、docs と実 GitHub 状態の乖離リスクは
  ない（merge 実施は lead の責務として明確に分離）。
- spec が `proposed` のまま残るのは記録済みの状態 — 実装完了と
  の status 不整合は本 task の権限外として follow-up 記録。

## Follow-ups

cross-task で引き継がれた engineering follow-up の集約（各 task
IR から転記・closeout 時点の状態付き）:

- **underrun metrics**（Task 4 → Task 5 → 継続 open）—
  `last_output`/silence pass-through・dropped submit・
  `RESULT_FAILED` の発生回数に観測手段なし（design が指標を
  定義しない）。`HostNotification` 面ができたため metrics
  surface の置き場候補として composite を再評価可 — 後続
  Issue 管轄。
- **`View::attach` の header 再検証**（Task 4 documented
  decision → Task 5 継続 → open）— `View` は owner の
  `Segment::create` が `verify_header` 済み前提で header を
  再検証しない設計。第三者 attach 経路が現れた場合に再評価。
- **frame-I/O helper dedup ×4**（Task 2 → 3 → 4 → 5 → open）—
  `read_message`/`write_message` が `l1/mod.rs`・
  `l1-test-worker.rs`・`l1_proxy_e2e`・export
  `read_frame`/`send` の 4 箇所に同形状で散在。wire 層への
  共通 helper 格上げ or test-only dedup — Task 6/7 は doc/
  gate のみのため未着手・後続 Issue で評価。
- **u16 generation wrap parity**（Task 5 → open）—
  `PluginHost::mint` は L0 規則の parity 実装で wrap 限界も
  L0 と同一（parity by design — 2^16 周期の slot 再利用で
  stale handle が理論上 alias し得る既知限界）。実害が観測
  された場合に L0 と一緒に評価。
- **`runner_program()` path fragility**（Task 5 → open）—
  `current_exe` からの `target/<profile>` 解決は workspace
  test entry（`cargo test --workspace` が全 member bin を同一
  profile dir に build）依存。`CARGO_BIN_EXE_*` の
  cross-package 代替 or cargo metadata ベースの解決を評価。
- **`compiler_fence`→`atomic::fence` design-doc 文言同期**
  （Task 4 → **本 task で解消**）— design L166 + test-spec
  proxy-contract の fence 記述を `std::sync::atomic::fence`
  に同期済み。
- **その他 task 由来の残件**（各 IR より — closeout で状態
  確認のみ）: `WireError`→`HostError::Ipc` への protocol/
  transport loss 畳込み（design どおり・通知 payload に原因
  詳細を持たせるかは将来の通知型拡張で再評価可）・`stderr`
  piping（`Stdio::inherit` のまま — deferred design concern）・
  `Descriptor::matches` の features 順序感度（現行は pairwise
  等価 — 集合等価が必要になった場合に緩和検討）・`gui_open`
  write-only field（L0 `host.rs` の内部 bookkeeping として
  残存 — L1 は GUI defined rejection のため使用経路なし）・
  `ruviform-audio-core` cargo doc 警告 2 件（本 issue 非起因
  の pre-existing issue・別途 triage）。
- lead による authoritative `gh` での #76 PR 実 merge — 本 IR
  は merged 主張を含まず、merge 完了の記録も本 bookkeeping
  の範囲外。
- spec `docs/specs/plugin-l1-out-of-process.md` の status 遷移
  判断は lead が別途裁定。
