---
id: "TASK-Fe8Qa3WrNm7YtKc2XzLb4H"
type: "task"
status: "completed"
title: "Issue #456 — job 終端 receipt の実装タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/456"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/runtime.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/ipc.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/bridge.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
      - type: "file"
        path: "crates/ruviform-ui/tests/runtime.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/selection-command.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-456-job-command-receipt.md"
  depends-on:
    - "docs/plans/issue-456-job-command-receipt.md"
---

# Issue #456 — 実装タスク

## T1. runtime: command id ↔ job id の保持と終端 receipt

- [x] `JobRecord` に `command_id` を追加し、`spawn_job` で spawn 元 intent の id を保存する。
- [x] `collect_routed` の `Finished` で outcome を一度組み立て、終端 `job` event の後に `terminal_receipt` を owner window へ push する。
- [x] `terminal_receipt`: `Ok` → `applied`、`Err` → `ok: false`（`cancelled` / `worker_failed` 等の code を保持）。

## T2. wire 文書

- [x] `Intent` と `Event::Result` の文書を、ack と終端 receipt の二段に更新する。

## T3. receipt decode の一元化

- [x] `CommandReceipt::from_result` を追加し、単体テスト `result_events_decode_to_receipts` で境界を固定する。
- [x] `render.rs` の `drain_inbox` の `Result` 分岐を委譲し、不要になった `ReceiptStatus` の import を削除する。

## T4. テストと parity

- [x] `tests/runtime.rs`: GUI/CLI parity の期待列を `job/Finished` の後の `result` に更新する。
- [x] `tests/runtime.rs`: `job_command_receipt_follows_the_job_terminal`（applied / panic / cancel と ack の status なし）を追加する。
- [x] `tests/runtime.rs`: `concurrent_job_commands_receive_their_own_receipts`（command id ↔ job id）を追加する。
- [x] `selection-command.json`: `expect.button.jobReceipt` を追加する。
- [x] `driver_parity.rs`: Fake 側の `ui_dvr_03_parity_job_command_receipt_phases` を追加する。
- [x] `consumer_observation.rs`: GPUI 側の `observes_job_command_receipt_phases_through_seam` を追加する。
- [x] 受入の感度確認: 終端 receipt の生成を一時的に外すと、runtime・Fake・GPUI の新規テストがすべて失敗することを確認し、元に戻した。

## T5. 検証と文書の索引

- [x] `git fetch origin main && git merge origin/main` 後に検証を再実行する（`selection-command.json` の競合は RF#451 の tabs 節と結合して解消）。
- [x] 文書チェーン（spec → design → plan → task → test-spec）を作成し、各 README に索引を登録する。
