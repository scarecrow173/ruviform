---
id: "SPEC-Rk5Wq2TnBx8LmYc3VdHa9E"
type: "spec"
status: "approved"
title: "Issue #456 — job を伴う command の receipt を job 終端まで route する"
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
  implements: []
  implemented-by: []
  refines:
    - "docs/specs/issue-448-table-selection-command.md"
  depends-on:
    - "docs/specs/issue-448-table-selection-command.md"
  blocks: []
---

# Issue #456 — job を伴う command の receipt を job 終端まで route する

## 1. 背景

`job` を spawn する command（`CallOutcome::Job`）は、ack の `result` を一つ返した後、job が終端しても command id に対する `result` を返さない。`job` 終端は `job` event（`job: u64`）でしか流れず、command id との対応は runtime 内部にも残らない。その結果、job を伴う command の receipt は `accepted` で止まり、`applied` / `rejected` に進めない。consumer（AudioLens slice7）は `Tick` の `job` event で Run tab を再射影することで回避している。本 Issue はその恒久解消である。

## 2. 目的

job を spawn する command について、job の終端結果を command の receipt として返す。receipt と job 終端の対応付け（command id ↔ job id）は runtime の責務とする。

## 3. 受入条件

1. job を spawn する command は、ack の `result`（`ok`、`data = {job, label}`、`status` なし = `accepted`）の後、同じ command id に対する終端 receipt を **ちょうど一回** 受ける。
2. 終端 receipt の対応は次のとおり。
   - `Finished` → `applied`（`ok: true`、`data.status = "applied"`、`data.job`）
   - `Failed` → `rejected`（`ok: false`、job の `error`）
   - `Cancelled` → `rejected`（`ok: false`、`error.code = "cancelled"`）
   - worker panic → `rejected`（`ok: false`、`error.code = "worker_failed"`）
3. command id ↔ job id の対応は runtime が保持し、window owner 経由の routing（`RoutedEvent.window`）は job event と同じ window に届く。
4. `job.cancel` は独立した command である。その `result` は取消要求の ack（`cancelling: true`）だけを表し、job を spawn した command の receipt の代わりにはならない。
5. receipt の status 語彙は既存契約（`accepted` / `queued` / `applied` / `rejected` / `unsupported`）だけを使う。新しい語彙は導入しない。
6. headless（`ruviform-ui` の runtime テスト）で、job を伴う command が `accepted` の後に終端 receipt（applied / rejected）を受けることを検証する。
7. Fake と GPUI が同一 fixture（`selection-command.json`）で同じ receipt phase（`accepted` → `applied` / `rejected`）を観測することを検証する。

## 4. 契約変更

- `rvf.ipc.v1` の `result` は「1 intent につき 1 回」から「job を spawn する intent は ack 1 回と終端 receipt 1 回」に拡張される。新しい event variant は追加しない（既存の `result` を再利用する）。
- 既存 consumer は ack の `accepted` だけを扱うことで従来どおり動作する。同一 id の二回目の `result` を未知 id として扱う consumer は影響を受けるため、PR 本文で明記する。
- webview 側の `window.__rvf`: `call` は ack で解決したままとし、終端 receipt は `onEvent` に `result` として届く。`bridge.rs` の文書をこの振る舞いに合わせる（JS コードは変更しない）。
- AudioLens / synvelis 側のコードは変更しない。

## 5. 非スコープ

- consumer 側の `job` event 再射影の撤去（consumer 側の判断）。
- job の progress を receipt に反映すること。
- 新しい receipt status（例: `cancelled`）の導入。
- `command_index` の掃除方針（`applied` 後も index を残す既存挙動）の変更。
