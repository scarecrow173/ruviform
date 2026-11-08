---
id: "DESIGN-Tm7Xp4QvKn1ZrWd6JbGe8F"
type: "design"
status: "approved"
title: "Issue #456 — job 終端 receipt の routing 設計"
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
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-456-job-command-receipt.md"
---

# Issue #456 — 設計

## 1. 設計判断の要旨

job を spawn した command の id を runtime が job の登録と一緒に保持し、job が終端した時点で同じ id に対する `result` を一つ追加で流す。receipt の decode は `ruviform-ui` の一箇所（`CommandReceipt::from_result`）に置き、GPUI の inbox と Fake 側の parity テストが同じ実装を使う。

## 2. 責務境界

| 層 | 責務 | 変更 |
| --- | --- | --- |
| `runtime.rs`（UiRuntime） | command id ↔ job id の対応保持、終端 receipt の生成と routing | `JobRecord.command_id` を追加し、`collect_routed` の `Finished` で `terminal_receipt` を emit |
| `ipc.rs`（wire 契約） | `result` の意味（ack と終端 receipt）の文書化 | 文書のみ。variant は追加しない |
| `bridge.rs`（webview 契約） | `call` は ack で解決、終端 receipt は `onEvent` に届く | 文書のみ。JS は変更しない（既存の未対応 id の fall-through で届く） |
| `behavior.rs`（receipt 契約） | `Result` → `CommandReceipt` の decode（唯一の実装） | `CommandReceipt::from_result` を追加 |
| `render.rs`（GPUI inbox） | inbox の `Result` を view へ適用 | `drain_inbox` の decode を `from_result` へ委譲（分岐を削除） |

## 3. runtime の処理

- `spawn_job` は `JobRecord { cancel, owner, command_id }` を保持する。`command_id` は spawn 元 intent の id。
- `collect_routed` の `Finished` は、job の登録を取り除き、outcome を一度だけ組み立てる。
  - `JobResult::Ok(v)` → `Ok(v)`、`JobResult::Err(e)` → `Err(e)`、`JobResult::Panic` → `Err(worker_failed)`。
  - 終端 `job` event を先に push し、その後に `terminal_receipt(command_id, job, &outcome)` を push する。どちらも job の `owner` window に routing する。
- `terminal_receipt` は `Ok` → `result_ok(id, {status: "applied", job})`、`Err` → `result_err(id, e)` を返す。`Err` の `cancelled` は `e.code` にそのまま残る。

`job.cancel` は通常の runtime op として扱い、その `result` は自分の id にだけ届く。取消された job の receipt は spawn 元 command の id で届く。

## 4. receipt の decode（`CommandReceipt::from_result`）

- `ok == false` → `Rejected`、detail は `error.message`。
- `ok == true` で `data.status` なし → `Accepted`（ack）。
- `ok == true` で `data.status` あり → 語彙に一致すればその値、不一致なら `Rejected`（detail に理由）。
- `data.revision` は revision として読む（従来どおり）。

従来の `drain_inbox` と挙動は同一であり、テスト（`result_events_decode_to_receipts`）で境界を固定する。

## 5. 代替案と却下理由

- **新しい event variant（`Receipt`）を追加する**: 既存 consumer の `result` 解析（status の decode）を二重化し、全 consumer に新 variant の対応を強いるため却下。
- **GPUI 側で `job` event を command id に紐付ける**: command id ↔ job id の対応は runtime の責務（Issue の指定）であり、view 側に持つと Fake 側と乖離するため却下。
- **ack を job 終端まで遅延する**: `accepted` の意味（gateway が受理した）が失われ、長時間 job で ack が届かなくなるため却下。

## 6. 境界条件

- **window 破棄後の終端**: owner window が消えていても終端 receipt は生成される。配送層が破棄済み window への event を安全に捨てる（既存の job event と同じ扱い）。
- **`cancel_window_jobs`**: 取消された job は `rejected`（`cancelled`）で終端する。window close と job cancel は別である（Issue #181 の方針を維持）。
- **同一 command id の再利用**: id の一意性は client の責務。runtime は id ごとの二重 receipt を抑止しない（既存の `dispatch_op` と同じ扱い）。
- **`command_index`**: `ViewHost` の `command_index` は `rejected` / `unsupported` で掃除され、`applied` では残る。この既存挙動は変更しない（Spec の非スコープ）。
