---
id: "TASK-034JNwUPMqjwynDi5XzU09"
type: "task"
status: "done"
title: "ドキュメント同期と Issue #58 完了検証（AGENTS.md 型列挙・全量検証・監査）"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/58"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-58-time-type-extension.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-58-time-type-extension.md"
    - "migrate-timecode-to-framerate.md"
    - "add-time-precision-tests.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-58-time-type-extension.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/time-type-extension-contracts.md"
  references: []
  defers: []
  deferred-by: []
---

# ドキュメント同期と Issue #58 完了検証（AGENTS.md 型列挙・全量検証・監査）

## Work

plan `PLAN-034JNstCXHhzzwOvo9iiCv` の Phase T4。T1–T3 のコード変更とドキュメント群の整合を取り、Issue #58 の完了条件を全量検証で確定させる。

- [x] `AGENTS.md` の `ruviform-media-core` 型列挙（「メディアデータモデル（`TimeBase`・`MediaTime`・…）」等の記述箇所）に `FrameRate` を追記 — 実際の記述を確認した上で最小限の同期に留める
- [x] `time.rs` / `timecode.rs` / `lib.rs` の doc コメントが最終コードと一致（`FrameRate` の説明・`frame_duration()` の逆数関係・新シグネチャの `# Errors` 記述）
- [x] plan 内チェックリストの全項目を完了マーク（上流文書の行動項目箇条書きは upstream 所有のため編集しない）
- [x] 全量検証: `mise run fmt-check`・`check`・`lint`・`test`・`test-rt` を実施し、結果を記録（`bench` は整数演算のみの変更で計量対象外・スキップ判断を plan に記録済み）
- [x] `mise exec -- prek run --all-files` でコミットフック全量を通す
- [x] ドキュメント監査 `audit_docs.js --type all --json` で 0 findings を確認（relation 双方向整合・README index 整合を含む）
- [x] spec `../specs/time-type-extension-framerate-timecode-rounding.md` の `status` を `implemented` に更新（spec の実装完了ステータス遷移 — graph の implemented 判定条件）
- [x] PR 準備: 対象ファイルのみ個別ステージ（`git add -A` 禁止・`apm.lock.yaml` 除外）、コミット、PR 作成。Issue #58 との紐付け（`Closes #58`）を本文に記載

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-58-time-type-extension.md](../plans/implement-issue-58-time-type-extension.md) — Phase T4 の正本
- [../specs/time-type-extension-framerate-timecode-rounding.md](../specs/time-type-extension-framerate-timecode-rounding.md) — `status: implemented` への遷移対象
- [../../AGENTS.md](../../AGENTS.md) — `FrameRate` 型列挙の同期対象

## Dependencies

- 前提: `migrate-timecode-to-framerate.md`・`add-time-precision-tests.md`（コード変更が前提）
- 外部クレート依存・`unsafe` なし

## Affected Files

- `AGENTS.md` — 型列挙への `FrameRate` 追記（必要な場合のみ）
- `docs/specs/time-type-extension-framerate-timecode-rounding.md` — status 遷移
- `docs/plans/implement-issue-58-time-type-extension.md` — チェックリスト完了・status 更新
- `docs/tasks/*.md` — 本タスク含む status 更新

## Done When

- [x] `AGENTS.md` と crate-level doc が実装済み API と一致する
- [x] `mise run fmt-check && mise run check && mise run lint && mise run test && mise run test-rt` がすべて実施・合格し、`prek run --all-files` が通る
- [x] ドキュメント監査が 0 findings
- [x] spec status が `implemented` に更新され、plan/task 群のステータスが最終状態に整合する
- [x] PR が作成されている（merge 判定は PR 上で実施）

## Verification

- [x] `mise run fmt-check && mise run check && mise run lint && mise run test && mise run test-rt` — 全合格
- [x] `mise exec -- prek run --all-files` — 合格
- [x] `node .agents/skills/doc-driven-dev-graph/scripts/audit_docs.js --type all --json` — findings 0
- [x] `git status` で `apm.lock.yaml` が未ステージのまま残っていることを確認
