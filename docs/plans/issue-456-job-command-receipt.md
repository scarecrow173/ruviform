---
id: "PLAN-Hb2Yn9LcSt5RkVw3MxPq6D"
type: "plan"
status: "approved"
title: "Issue #456 — job 終端 receipt の実装計画"
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
    - "docs/designs/issue-456-job-command-receipt.md"
  depends-on:
    - "docs/specs/issue-456-job-command-receipt.md"
    - "docs/designs/issue-456-job-command-receipt.md"
---

# Issue #456 — 実装計画

## 1. スコープ

- 対象: `ruviform-ui`（runtime・wire 文書・receipt decode）、`ruviform-ui-gpui`（inbox の decode 委譲のみ）、`ruviform-ui-test`（Fake 側 parity）。
- 対象外: consumer（AudioLens / synvelis）のコード、新しい receipt status、render の Tabs/descriptor 周辺（並行 RF#451 の範囲）。

## 2. ファイル対応表

| ファイル | 変更内容 |
| --- | --- |
| `crates/ruviform-ui/src/runtime.rs` | `JobRecord.command_id` の追加、`collect_routed` の終端 receipt、`terminal_receipt` |
| `crates/ruviform-ui/src/ipc.rs` | `Intent` / `Event::Result` の文書（ack と終端 receipt の二段） |
| `crates/ruviform-ui/src/bridge.rs` | `call` / `onEvent` の文書（ack 解決と、終端 receipt の `onEvent` 配送） |
| `crates/ruviform-ui/src/behavior.rs` | `CommandReceipt::from_result` の追加と単体テスト |
| `crates/ruviform-ui/tests/runtime.rs` | GUI/CLI parity の期待列の更新、終端 receipt・取消・並行の検証 |
| `crates/ruviform-ui-gpui/src/render.rs` | `drain_inbox` の `Result` 分岐を `from_result` へ委譲（`ReceiptStatus` の import を削除） |
| `crates/ruviform-ui-test/fixtures/driver-lab/parity/selection-command.json` | `expect.button.jobReceipt`（ack / applied / rejected）の追加 |
| `crates/ruviform-ui-test/tests/driver_parity.rs` | Fake 側の receipt phase 検証 |
| `crates/ruviform-ui-gpui/tests/consumer_observation.rs` | GPUI 側の receipt phase 検証（`post_event` 経由） |

## 3. 作業順序

1. runtime の `command_id` 保持と終端 receipt の生成（headless の期待を先に書き、失敗を確認する）。
2. wire 文書（`ipc.rs`）を更新する。
3. `CommandReceipt::from_result` を追加し、GPUI の `drain_inbox` を委譲する。
4. Fake / GPUI の parity を fixture 経由で追加する。
5. 検証（後述）と文書チェーンの索引登録。

## 4. リスクと対策

| リスク | 対策 |
| --- | --- |
| 二回目の `result` を未知 id として扱う consumer が壊れる | Spec §4 に明記。ack は従来どおり `accepted` を維持 |
| `render.rs` の並行変更との競合 | 変更を `drain_inbox` の Result 分岐に限定し、他領域に触れない |
| 取消と終端の混同 | `job.cancel` の result は自分の id のみ。テストで `results_for("c")` が空であることを検証 |
| panic の stderr 出力 | 既存テスト（`worker_panic_maps_to_worker_failed`）と同じ扱いで許容 |

## 5. ロールバック

コミットを論理単位（runtime / decode 委譲 / parity・fixture / 文書）に分ける。問題があれば runtime のコミットを revert すると、ack のみの従来挙動に戻る。wire 形式の追加は既存 variant の再利用なので、データ移行は不要。

## 6. 検証マトリクス

| 項目 | コマンド（canonical） | 期待 |
| --- | --- | --- |
| 型検査 | `mise run check` | 成功 |
| lint | `mise run lint`、`mise run lint-ui-gpui` | `-D warnings` で成功 |
| 整形 | `mise run fmt-check` | 成功 |
| headless runtime | `mise run test`（workspace 内）、補助: `cargo test -p ruviform-ui` | 終端 receipt と parity 期待列が成功 |
| Fake parity | `mise run test-ui-parity` | `ui_dvr_03_parity_job_command_receipt_phases` が成功 |
| GPUI parity | `mise run test-ui-consumer-observation`、`mise run test-ui-gpui` | `observes_job_command_receipt_phases_through_seam` が成功 |
| driver 境界 | `mise run check-ui-driver-boundary` | verdict pass |
| driver 契約 | `mise run test-ui-driver-contract`、`mise run test-ui-component-view` | 成功 |
