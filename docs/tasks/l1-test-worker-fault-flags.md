---
id: "TASK-034NOXyYOTNwDoH2c7VhrV"
type: "task"
status: "done"
title: "l1-test-worker fault flags（--fail-restore・--state-unsupported・--fail-initialize・--fail-start-processing）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/116"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-116-supervisor.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-116-supervisor.md"
  blocks:
    - "docs/tasks/l1-supervisor-e2e.md"
    - "docs/tasks/l1-supervisor-final-verification.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/l1-supervisor-core.md"
    - "docs/tasks/l1-wire-v2-codec-cutover.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-116-supervisor.md"
    - "docs/designs/issue-116-supervisor.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-116-supervisor-e2e-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# l1-test-worker fault flags（--fail-restore・--state-unsupported・--fail-initialize・--fail-start-processing）

## Work

`l1-test-worker` に supervisor E2E が必要とする失敗分類注入 flag を 4 本追加する（design Verification Notes の **fixture gap** — 現行 `RestoreState` は常に `Ok`（seed blob `0x11,0x22,0x33,0x44` を clone するだけ・`src/bin/l1-test-worker.rs` の `RestoreState` arm）で `Err(Unsupported)`/`Err(PluginFailed)` を生起不能・lifecycle hook 失敗の注入経路も `--fail-stop-processing` しかない）。命名・形状規約は `--fail-stop-processing` に揃える。

1. `Flags` struct・`parse_flags`・argv doc table に以下を追加（全て child-scope — 全 instance 共通・既存 flag と同じ粒度。instance 単位制御は現行 argv モデル上の構造制約で対象外）:
   - `--fail-restore` — `RestoreState` が `Err(HostError::PluginFailed)` で応答（**不 commit** — `self.state` blob を変更しない。supervisor の保守的分類「checkpoint を捨てない attempt 失敗」の注入）
   - `--state-unsupported` — `RestoreState` が `Err(HostError::Unsupported)` で応答（**不 commit** — state 非対応 plugin の宣言。`--fail-restore` 併用時は `Unsupported` を先に評価して determinism を確保する順序を明記）
   - `--fail-initialize` — `Initialize` が `Err(HostError::PluginFailed)` で応答（**不 commit** — `Created` 残留 → `Destroy` 合法の cleanup 経路注入）
   - `--fail-start-processing` — `StartProcessing` が `Err(HostError::PluginFailed)` で応答（**不 commit** — `Activated` 残留 → unwind `Deactivate`→`Destroy` 経路注入）
2. `WorkerEntry::dispatch`/`dispatch` の signature を flag 群へ拡張 — 現行は `fail_stop_processing: bool` 単独引数。5 bool の位置引数は誤配線リスクのため `&Flags`（または verb 用小 struct）を渡す形へ整理（bin 内部の整合 change — argv/外部契約は不変）
3. argv doc table（bin 頭部の flag 表）へ 4 行追加 — 「何に `Err` を返すか・commit するかどうか」を既存行と同じ形状で記述
4. **注入形状の規則**: 新 flag は全て**不 commit `Err`**（hook 失敗が transition 前に観測される形状）。commit-then-advisory（transition commit 後に `Err` 応答）は `--fail-stop-processing` のみ — 実 runner の `stop_processing` が transition を先に適用する形状に倣った旗（worker 既存コメントの正本規則）
5. 既存 `--crash-after-msgs=N` は変更不要 — message counter は handshake を含む（Scan=1・`CreateInstance`=2）。N=2 で mid-`CreateInstance` 死、N=3 で `Initialize` 中死（attempt 中の子死亡）を注入可能 — T3 の注入表で使う

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-116-supervisor.md](../plans/implement-issue-116-supervisor.md)（T1 — fixture）
- [../designs/issue-116-supervisor.md](../designs/issue-116-supervisor.md) Verification Notes（fixture gap 行 — `--fail-restore`/`--state-unsupported` 相当を実装 task に含める明記）・失敗分類表（`RestoreState` `Err(Unsupported)` skip・`Err(PluginFailed)` 系 budget burn・lifecycle `Err` cleanup）
- [../specs/issue-116-supervisor.md](../specs/issue-116-supervisor.md) AC4/AC7・R-V2/R-V3（分類注入の検証要求）
- [issue-116-supervisor-e2e-tests.md](../test-specs/issue-116-supervisor-e2e-tests.md)（flag の注入効果 oracle — T3 で接続）

## Dependencies

- 先行タスク: なし（plan のみ — T2 `l1-supervisor-core` とは独立・並行可。fixture flag は supervisor API に非依存）
- 後続タスク: [l1-supervisor-e2e.md](l1-supervisor-e2e.md) — T3 の restore 分類・lifecycle 失敗注入の前提

## Affected Files

- `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs` — `Flags`/`parse_flags`/argv doc table/`WorkerEntry::dispatch`/`dispatch` signature・serve loop の flag 伝搬

## Done When

- [x] `--fail-restore`・`--state-unsupported`・`--fail-initialize`・`--fail-start-processing` が parse され argv doc table に記録されている（実績: `db80ead` — `src/bin/l1-test-worker.rs` L29-32 doc table・L172-179 parse・`Flags` struct field）
- [x] `RestoreState` → `Err(PluginFailed)`（`--fail-restore`）/`Err(Unsupported)`（`--state-unsupported`・併用時はこちら優先の定義済み順序）で state blob 不変更・`Initialize`/`StartProcessing` → `Err(PluginFailed)` で lifecycle 不 commit（`Created`/`Activated` 残留）（実績: T3 E2E で全系発火 — `checkpoint_unsupported_skips_without_attempt`/`checkpoint_pluginfailed_burns_attempts_to_gaveup`/`fail_initialize_attempt_cleans_created`/`fail_start_processing_attempt_unwinds_activated` 全 pass）
- [x] flag 未指定時の挙動は現行と同値（既存 `l1_proxy_e2e` 全 pass で非退行 — `--fail-stop-processing` 経路も不変）（実績: T4 gate で `l1_proxy_e2e` 100/100 無 diff green）
- [x] dispatch signature が flag 群を安全に運ぶ（bool 位置引数 5 連になっていない — `&Flags` or 小 struct）（実績: `Flags` struct 経由 — `flags.hang_after_msgs`/`fail_stop_processing` 等 field アクセス形）

## Verification

- [x] `cargo test -p ruviform-plugin-host --test l1_proxy_e2e` が全 pass（flag 未指定系の非退行）（実績: T4 gate 100/100 pass）
- [x] `mise run fmt-check`・`mise run check`・`mise run lint`・`mise run test`・`mise run test-rt` が全 pass（実績: T4 Full Gate 全 exit 0 — `test` 60 binaries・1294 passed・`test-rt` 137 一致 0 failed）
- [x] flag 付き spawn で `RestoreState`/`Initialize`/`StartProcessing` が対応する `Err` を返すことを dispatch 経路または直接 spawn smoke で確認（E2E 接続自体は T3 の範囲）（実績: T3 `l1_supervisor_e2e` 22/22 で flag 経由の `Err` 分類を実子経路で検証）
