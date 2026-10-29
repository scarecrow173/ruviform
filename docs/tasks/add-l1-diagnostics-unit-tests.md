---
id: "TASK-034OpsEs0zbbjn0dDTnIIj"
type: "task"
status: "done"
title: "T9. 単体テスト拡充（counter 9 ケース・ring 不変・分類写像・watermark）"
created: "2026-10-29"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/121"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/diag.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/data.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/endpoint.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-121-l1-diagnostics.md"
    - "docs/tasks/wire-l1-audio-counters.md"
    - "docs/tasks/wire-failure-cause-classification.md"
    - "docs/tasks/add-pre-mint-diagnostics.md"
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
    - "docs/test-specs/issue-121-l1-audio-counters-tests.md"
    - "docs/test-specs/issue-121-l1-stderr-drain-tests.md"
    - "docs/test-specs/issue-121-l1-diagnostics-retention-tests.md"
    - "docs/test-specs/issue-121-l1-failure-cause-notification-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# T9. 単体テスト拡充（counter 9 ケース・ring 不変・分類写像・watermark）

## Work

設計 Verification Notes の承認済み 9 ケースと診断機構の単体テストを追加する:

- **counter 9 ケース**（`l1/data.rs`/`endpoint.rs` のテスト module）: warmup pass-through +3・slot busy +1・同一 block 複数失敗で最大 +1・`RESULT_FAILED` 初回 pop +1・held 再取り出し +0・reader enqueue 失敗 +1・enqueue 成功 RT 未観測 +0・leg swap 維持・re-mint 0 開始。endpoint arm-anomaly `pass_through` 経路の計数も pin。
- **ring 不変条件**: `total_read = len + dropped`・tail 優先 eviction・`dropped_bytes` は「収集したが保持できなかった分」のみ（retention eviction・deadline 後未読は含まない）・snapshot が同一時点組み立て。
- **分類表写像**: 設計の意味ベース表の各行 → 正しい `FailureCause`（EOF→`Transport`・decode 不能→`Protocol`・非 `Hello` 先頭→`Handshake`・timeout→`Timeout`・exit→`ExitObserved`・reap error→`Internal`）・first-observed 競合で上書きされない・timeout が kill 前に記録される。
- **watermark**: `found/evicted/not-found` 3 値・`evicted_records` 計数・登録時連続採番・並行登録で欠番なし。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-121-l1-diagnostics.md`
- `docs/designs/issue-121-l1-diagnostics.md`（Verification Notes）

## Dependencies

- `docs/tasks/wire-l1-audio-counters.md`（counter 計装）
- `docs/tasks/wire-failure-cause-classification.md`（cause 分類）
- `docs/tasks/add-pre-mint-diagnostics.md`（retention/query）

## Affected Files

- `crates/ruviform-plugin-host/src/l1/{mod,data}.rs`・`src/l1/diag.rs`・`src/endpoint.rs` の `#[cfg(test)]` module

## Done When

- [x] 9 ケース全てが期待値を pin している
- [x] ring 不変条件・snapshot 同一時点・watermark 3 値が単体テストで pin されている
- [x] 分類表写像と first-observed 固定が単体テストで pin されている

## Verification

- [x] `mise run test` exits 0
- [x] `mise run test-rt` exits 0（RT 計装の no-alloc 無退行）
