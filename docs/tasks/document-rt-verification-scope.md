---
id: "TASK-034JP1tARbXjwUcXVXxNMW"
type: "task"
status: "todo"
title: "RT 検証基盤の検証文書整備と Issue #22 完了確認"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-22-rt-verification.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-22-rt-verification.md"
    - "apply-assert-no-alloc-to-plugin-boundary.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-22-rt-verification.md"
    - "apply-assert-no-alloc-to-plugin-boundary.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/rt-verification-contracts.md"
  references: []
  defers: []
  deferred-by: []
---

# RT 検証基盤の検証文書整備と Issue #22 完了確認

## Work

1. `README.md` に RT 検証の保証範囲限定を明記: 「CI の `rt_*` テスト成功は RT 安全性の完全証明ではない」「保証は API 単位で定義され実装責任である」「OS 全体のハードリアルタイム保証は行わない」
2. 静的に検証できない制約（ブロッキング操作・ファイル/ネットワーク I/O・無制限待機/リトライ・実行時間無界・ロック保持時間）を「ドキュメント契約 + コードレビュー観点」として列挙 — 本設計書または AGENTS.md の暫定規約を確定版に合わせて更新
3. `assert_no_alloc` と `alloc_counter` の二系統併存ルール（新規は `assert_no_alloc`・ruviform-rt の既存 `alloc_counter` は維持）を文書化
4. 上流文書（spec・ADR・design）の Verification チェックボックスを実測結果で更新し、タスクを done に
5. `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`・`bench-smoke`・`prek run --all-files` 全パス・doc 監査で本チェーン起因 findings 0
6. Issue #22 の Acceptance Criteria 全項目を確認して完了記録

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-22-rt-verification.md](../plans/implement-issue-22-rt-verification.md)
- [../specs/rt-verification-lane.md](../specs/rt-verification-lane.md)
- [../adr/rt-assert-no-alloc-thread-check.md](../adr/rt-assert-no-alloc-thread-check.md)
- [../test-specs/rt-verification-contracts.md](../test-specs/rt-verification-contracts.md)
- [apply-assert-no-alloc-to-plugin-boundary.md](apply-assert-no-alloc-to-plugin-boundary.md)

## Dependencies

- 先行タスク: [apply-assert-no-alloc-to-plugin-boundary.md](apply-assert-no-alloc-to-plugin-boundary.md)

## Affected Files

- `README.md`（保証範囲限定の明記）
- `AGENTS.md` または設計文書（ドキュメント契約の列挙 — 既存暫定規約との整合）
- `docs/specs/rt-verification-lane.md`・`docs/adr/rt-assert-no-alloc-thread-check.md`・`docs/designs/rt-verification-lane-design.md`（チェックボックス）
- `docs/tasks/README.md`（ステータス同期）

## Done When

- [ ] README に保証範囲限定3点（CI 成功 ≠ 完全証明・API 単位保証は実装責任・OS ハード RT 保証なし）が明記されている
- [ ] 静的検証不能制約がドキュメント契約 + コードレビュー観点として列挙されている
- [ ] 全量検証（fmt-check/check/lint/test/test-rt/bench-smoke/prek）パス
- [ ] doc 監査で本変更起因 findings 0

## Verification

- [ ] 上記 Done When 全項目
- [ ] Issue #22 Acceptance Criteria 全項目確認済み
