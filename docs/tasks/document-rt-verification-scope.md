---
id: "TASK-034JP1tARbXjwUcXVXxNMW"
type: "task"
status: "done"
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

- [x] README に保証範囲限定3点（CI 成功 ≠ 完全証明・API 単位保証は実装責任・OS ハード RT 保証なし）が明記されている
- [x] 静的検証不能制約がドキュメント契約 + コードレビュー観点として列挙されている
- [x] 全量検証（fmt-check/check/lint/test/test-rt/bench-smoke/prek）パス
- [x] doc 監査で本変更起因 findings 0

## Verification

- [x] 上記 Done When 全項目
- [x] Issue #22 Acceptance Criteria 全項目確認済み

## Completion Notes

- `README.md` RT 契約節に Issue #22 レーンの実装済み状態を追記: `assert_no_alloc` 測定窓（CLAP `process` 経路・VST3 `prepare_block`/`finish_block`/`IAudioProcessor::process`・audio-device コールバック）、CLAP `thread-check` defined rejection、機械検出対象の限定（alloc/dealloc + スレッド契約のみ）と静的制約のドキュメント契約 + レビュー責務。保証範囲限定3点（API 単位保証・CI 成功 ≠ 完全証明・OS ハード RT 保証なし）は既存記述で充足。
- `AGENTS.md` 暫定規約に「RT 検証レーン」条項を追加: `assert_no_alloc` 測定窓ルール（fixture/warm-up は窓外）、`alloc_counter` 併存ルール、thread-check 照会方式、静的検証不能制約の列挙と CI 非完全性の明記。
- `plugin-clap` lib.rs crate doc・`plugin-vst3` block.rs モジュール doc の「Issue #22 まで doc 契約」記述を実測済み記述に更新済み（T2）。
- 全量検証: `mise run fmt-check`/`check`/`lint`/`test`（30 バイナリ全パス）/`test-rt`（clap 7・vst3 4 の rt_ テスト含め全パス）/`bench-smoke`/`prek run --all-files` 全パス。
- Issue #22 受入基準確認: assert_no_alloc 型検出機構の CI レーン統合 ✓（test-rt レーンで `rt_*` テストが測定窓内実行）・thread-check 統合 ✓（init 時照会 + defined rejection + ABI 通し両経路テスト）・「CI 成功 ≠ 完全証明」「API 単位保証は実装責任」の明記 ✓（README・AGENTS.md・crate doc）。
