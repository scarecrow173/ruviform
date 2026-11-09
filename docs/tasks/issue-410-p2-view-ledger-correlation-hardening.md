---
id: "TASK-034UwdhT5Gt2dptCoVPLfF"
type: "task"
status: "done"
title: "Issue #410 P2 — view ledger correlation hardening"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/410"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-410-p2-view-ledger-correlation-hardening.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/behavior.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-410-p2-view-ledger-correlation-hardening.md"
  implemented-by:
    - "docs/impl/ir/issue-410-p2-view-ledger-correlation-hardening.md"
  depends-on:
    - "docs/plans/issue-410-p2-view-ledger-correlation-hardening.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-410-p2-view-ledger-correlation-hardening.md"
    - "docs/designs/issue-410-p2-view-ledger-correlation-hardening.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-410-p2-view-ledger-correlation-hardening.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #410 P2 — view ledger correlation hardening

## Work

`crates/ruviform-ui/src/view.rs` と `behavior.rs` に対し、
Issue #410 P2 slice（projection correlation floor + slice 1 受入
条件の検証固定）を実装する:

1. `crate::behavior::revision_advances`（`pub(crate)`）を追加し、
   `CommandBinding::on_projection` の revision 比較を集約。
2. `ViewHost::projection_floors: HashMap<ControlId, Revision>` を
   追加し、`apply_projection` で floor 未 advance の projection を
   stale drop、受理時に floor を更新。`unmount`/`detach`/`update`
   では clear しない。
3. `apply_projection` の doc comment に correlation 契約
   （revision 単調性スコープ・canonical entry point・
   delivery path の義務）を記述。
4. epoch 境界（detach→attach・unmount→mount）の順序逆転・
   remount 誤適用を固定する回帰 unit test を追加。
5. slice 1 対応 test の棚卸しと受入条件との対応記録。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-410-p2-view-ledger-correlation-hardening.md`
- `docs/specs/issue-410-p2-view-ledger-correlation-hardening.md`
- `docs/designs/issue-410-p2-view-ledger-correlation-hardening.md`
- `docs/test-specs/issue-410-p2-view-ledger-correlation-hardening.md`

## Dependencies

- なし（blocked by 無し）。slice 1（PR #502）は main 統合済み。

## Affected Files

- `crates/ruviform-ui/src/view.rs`
- `crates/ruviform-ui/src/behavior.rs`

## Done When

- [x] `projection_floors` が `unmount`/`detach`/`update` を跨いで
  維持され、floor 未 advance の projection が stale drop される
- [x] `revision_advances` が `on_projection` と同一規則で共有
  されている
- [x] remount・detach/attach の順序逆転・`deliver_event` 経路の
  floor 強制を固定する unit test が pass する
- [x] slice 1 の受入条件 5 行に対応する既存 test が pass し続ける
- [x] wraparound/reentrant borrow 契約が design の Review Record
  に記録されている

## Verification

- [x] `mise run fmt-check` exits 0
- [x] `mise run check` exits 0
- [x] `mise run lint` exits 0
- [x] `mise run test` exits 0
- [x] `mise run test-ui-*`（存在するもの全て）exits 0
