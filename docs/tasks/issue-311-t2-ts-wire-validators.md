---
id: "TASK-jXU3ZGuh6DP2FfCSvkx5FC"
type: "task"
status: "done"
title: "T2 TS wire validators — ipcwire/bindingwire + conformance テスト（Issue #311）"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "ui/guitest/src/ipcwire.ts"
      - type: "file"
        path: "ui/guitest/src/bindingwire.ts"
      - type: "file"
        path: "ui/guitest/test/wire.test.ts"
      - type: "file"
        path: "ui/guitest/test/bindingwire.test.ts"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-311-ipc-conformance-replay.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-311-t1-fixture-corpus.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-311-ipc-conformance-replay.md"
  references: []
  defers: []
  deferred-by: []
---

# T2 TS wire validators + conformance テスト

## Work

- `src/ipcwire.ts`: `rvf.ipc.v1` の Intent parse（Rust `parse_intent` と同一判定: サイズ上限・`v`/`kind`/`op` 形・`id` 型）と Event envelope decode（`v`/`kind`/相関・optional `detail`/`details`）。`rvf://data` の `dataUrl` 構築往復・応答契約モデル（bodyLen ≤ min(len,total-offset,maxChunk)、`X-Rvf-Total`）。zod 等不使用・依存ゼロ。
- `src/bindingwire.ts`: `CommandIntent`/`CommandReceipt`/`StateProjection`/`ViewportQuery`/`QueryReply` validator。安定エラー語彙 `missing-field`/`bad-kind`/`bad-type`/`out-of-range`。
- `test/wire.test.ts`: 全 fixture case を TS 側で処理し `expect` と照合（同一 version/同一意味）。
- `test/bindingwire.test.ts`: fixture 構造健全性（`v`/`suite`/case 語彙、denylist field 非含有、id/seed 一意）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/tasks/issue-311-t1-fixture-corpus.md`
- `docs/designs/issue-311-ipc-conformance-replay.md` §2

## Verification

- `npm --prefix ui/guitest run typecheck`・`node --test`（mise タスク経由後は `mise run check-ui-guitest`/`test-ui-guitest`）。
