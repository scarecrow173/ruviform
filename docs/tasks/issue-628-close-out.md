---
id: "TASK-034Y3hoX8z7w5ICx2KWLaP"
type: "task"
status: "done"
title: "Issue #628 close-out タスク — 残存小項目統合"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "docs/specs/issue-628-close-out.md"
    - "docs/plans/issue-628-close-out.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-628-close-out.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-628-close-out.md"
    - "docs/designs/issue-628-close-out.md"
    - "docs/plans/issue-628-close-out.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-628-close-out.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-628-close-out.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-628-close-out.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 close-out タスク — 残存小項目統合

## Work

`crates/ruviform-audio-test` の close-out slice（Issue
`scarecrow173/ruviform#628`）: RunMetadata JSON serialization、
`StreamFault::Cancellation`、docs 陳腐化修正、AC 棚卸し表。
製品 crate の src 変更ゼロが境界条件。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/issue-628-close-out.md`
- `docs/designs/issue-628-close-out.md`
- `docs/plans/issue-628-close-out.md`
- `docs/test-specs/issue-628-close-out.md`

## Dependencies

- 前提 merge 済み: PR `scarecrow173/ruviform#638`（1st slice）、
  `scarecrow173/ruviform#659`（2nd slice）、`scarecrow173/ruviform#664`
  （3rd slice）。consumer oracle は両 repo merge 済み。

## Affected Files

- `crates/ruviform-audio-test/Cargo.toml` — serde/serde_json 追加・
  description 更新
- `crates/ruviform-audio-test/src/{metadata,clock,backend,lib}.rs`、
  `src/backend/stream.rs` — serde derive・JSON API・Cancellation fault
- `crates/ruviform-audio-test/tests/{metadata,backend_faults}.rs` —
  新規 test
- `crates/ruviform-audio-test/README.md`、`crates/README.md` —
  陳腐記述更新
- `docs/tasks/issue-628-{deterministic-test-runtime,scripted-backend,
  pipeline-wiring}.md` — status 修正のみ
- `evidence/rf628/` — AC 棚卸し表・検証記録
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/
  issue-628-close-out.*` — 本 chain
- `Cargo.lock`・`docs/*/README.md`（管理 index）— 再生成のみ

## Task List

- [x] T1: `Cargo.toml` に serde/serde_json 追加 + description 更新
- [x] T2: `clock.rs` serde derive、`metadata.rs` serde derive + JSON
      API + stale doc 更新
- [x] T3: `backend.rs` `StreamFault::Cancellation` + `stream.rs` pump
      経路 + `lib.rs` doc 更新
- [x] T4: `tests/metadata.rs` JSON tests、`tests/backend_faults.rs`
      cancellation tests
- [x] T5: crate README・`crates/README.md`・`docs/tasks/` 3 件 status
      修正
- [x] T6: `evidence/rf628/` AC 棚卸し表 + 検証記録
- [x] T7: 検証実行（fmt-check/check/lint/test/test-rt/prek）
- [x] T8: impl/ir + impl/exp 記録、review-protocol レビュー
- [x] T9: docs index 再生成・origin/main merge・commit/push/PR

## Done When

- [x] `RunMetadata` JSON round-trip・wire 語彙 pin・save/load test が
  pass する。
- [x] `StreamFault::Cancellation` の Expected/Observed 正負テストが
  pass する（output/input・mid-stream inject）。
- [x] `docs/tasks/` 3 件が `done`、`crates/README.md`・crate
  `Cargo.toml` description・crate README が現状に揃う。
- [x] `evidence/rf628/` に Issue 全 AC × verdict 棚卸し表がある。
- [x] 指定検証コマンド全て pass、docs index 再生成済み、製品 crate
  src 差分なし。

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check ruviform-audio-test`
- [x] `mise run lint ruviform-audio-test`
- [x] `mise run test ruviform-audio-test`
- [x] `mise run test-rt`（既存 `rt_` 非回帰確認）
- [x] `mise exec -- prek run --all-files`
- [x] docs index 再生成の in-sync 確認
