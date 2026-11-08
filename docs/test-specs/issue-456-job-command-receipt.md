---
id: "TSPEC-Vc4Dk1PhGj6WsQy9NrTm5A"
type: "test-spec"
status: "approved"
title: "Issue #456 — job 終端 receipt の検証契約"
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
        path: "crates/ruviform-ui/tests/runtime.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/selection-command.json"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  verifies:
    - "docs/specs/issue-456-job-command-receipt.md"
    - "docs/designs/issue-456-job-command-receipt.md"
  depends-on:
    - "docs/specs/issue-456-job-command-receipt.md"
    - "docs/designs/issue-456-job-command-receipt.md"
  blocks: []
  supersedes: []
---

# Issue #456 — 検証契約

## 1. 保証すること

| ID | 保証 | 検証箇所 |
| --- | --- | --- |
| T-456-01 | job を伴う command は ack（`ok`、`status` なし）を返し、その後 **同じ id** の終端 receipt を一つだけ返す | `runtime.rs::job_command_receipt_follows_the_job_terminal` |
| T-456-02 | 終端 `Finished` の receipt は `applied`（`data.job` が ack の job と一致） | 同上 |
| T-456-03 | 終端 receipt は終端 `job` event より後に出る | 同上（position の比較） |
| T-456-04 | worker panic の receipt は `ok: false`、`worker_failed` | 同上 |
| T-456-05 | cancel の receipt は `ok: false`、`cancelled`。`job.cancel` の result は自分の id のみ | 同上 |
| T-456-06 | 並行する job command は、それぞれ自分の job の applied receipt を受ける（command id ↔ job id） | `runtime.rs::concurrent_job_commands_receive_their_own_receipts` |
| T-456-07 | GUI と CLI の経路は同じ event 列（終端 receipt を含む） | `runtime.rs::gui_and_cli_paths_produce_identical_event_shapes` |
| T-456-08 | `Result` → `CommandReceipt` の decode（ack / applied / failed / 未知 status / 非 result） | `behavior.rs::result_events_decode_to_receipts` |
| T-456-09 | Fake 側で、ack → `accepted`、終端 → `applied` / `rejected` の phase 列 | `driver_parity.rs::ui_dvr_03_parity_job_command_receipt_phases` |
| T-456-10 | GPUI 側で、同じ phase 列が inbox（`post_event`）経由で観測される | `consumer_observation.rs::observes_job_command_receipt_phases_through_seam` |

## 2. 検証しないこと

- 実 OS window 上での描画と入力（手動検証の範囲）。
- consumer（AudioLens / synvelis）側の再射影の挙動。
- 同一 command id の再利用時の扱い（client の責務。Design §6）。

## 3. 感度

T-456-01〜10 のうち runtime 由来のものは、終端 receipt の生成を外すと失敗することを確認済み（Task T4）。
