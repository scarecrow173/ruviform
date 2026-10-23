---
id: "TASK-034KPgQj9u5utlkxH3JGqv"
type: "task"
status: "done"
title: "edit 評価射影 + テスト・文書クローズ"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-edit/src/eval.rs"
      - type: "file"
        path: "crates/ruviform-edit/README.md"
    modified:
      - type: "file"
        path: "crates/ruviform-edit/src/lib.rs"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "docs/plans/implement-issue-96-edit-model.md"
      - type: "file"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-96-edit-model.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-96-edit-model.md"
    - "docs/tasks/edit-crate-and-model.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# edit 評価射影 + テスト・文書クローズ

## Work

`Timeline::evaluate` の評価射影（`Evaluation`/`ActiveClip`）を実装し、
test-spec 全ケースのテスト・crate README・`crates/README.md` 索引行・
検証列・doc 監査で Issue #96 の doc チェーンを閉じる（plan 手順 3–5）。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-issue-96-edit-model.md
- docs/tasks/edit-crate-and-model.md

## Dependencies

- docs/tasks/edit-crate-and-model.md（骨格 + データモデル型・mutation）

## Affected Files

- crates/ruviform-edit/src/eval.rs（`Evaluation`/`ActiveClip`/`evaluate`）
- crates/ruviform-edit/src/lib.rs（re-export 追加）
- crates/ruviform-edit/README.md（crate README 新設）
- crates/README.md（索引行追加 — engine と同順位の利用側層）
- docs/*（チェーン frontmatter: task status・plan Verification チェック）

## Done When

- [x] `evaluate` が design の算術経路（`at` 1 回変換・半開包含・後開始/
  同開始後挿入優先・`source_time` 写像・到達可能残区間・`TimeOverflow`）
  で `Evaluation`/`ActiveClip` を返す
- [x] test-spec 全ケースパス
- [x] crate README + `crates/README.md` 索引行・検証列全パス・
  doc 監査チェーン起因 findings 0

## Verification

- [x] `mise run fmt-check`/`check`/`lint`/`test` パス
- [x] `doc_status.js audit` findings 0・依存が `ruviform-media-core` のみ

## Completion Notes

- `eval.rs`: `Evaluation`/`ActiveClip`（pub フィールド plain data・
  不変条件なし — `exhaustive_structs` は lib.rs のクレート単位 allow で
  `ClipSpec` と同規約）と `impl Timeline::evaluate` を実装。
  `at` → `convert(base, NearestEven)` 1 回 → enabled トラックごとに
  半開区間包含の最後のクリップを勝者とし（後開始・同開始後挿入優先が
  stable sort 順から導出）、`offset = at - range.start()` →
  `source_time = source_range.start().checked_add(offset)` → 残区間
  `[source_time, min(out, in + range.duration()))`（`end()` の `out` は
  mutation 時 `checked_end` と同値を再現）を `ActiveClip.source_range`
  に格納。`Evaluation.at` は timeline base 正規化済み値を格納。
  O(tracks × clips) 線形走査・`Vec::with_capacity(tracks.len())`
- `lib.rs`: `mod eval` + `pub use eval::{ActiveClip, Evaluation}` を追加
- テスト: 空タイムライン・ギャップ半開境界・オーバーラップ（後開始
  優先・同開始後挿入優先・非包含の後開始クリップ不選出）・disabled・
  `source_time` 写像（non-zero `in`・残区間 `min` clamp・等長時 `out` 到達・
  source base 保持）・cross-base `at`・`at` 変換 `TimeOverflow`・
  複数トラック列挙順・per-track 独立ギャップ・remove 後の非活性・
  決定性・foreign-base 格納後評価 — eval.rs テストモジュール 14 件 +
  `evaluate` doctest。`rt_` テストは非 RT クレートのため設けない
- crate README（責務・層/許可依存・RT 契約・関連）新設。
  `crates/README.md` に「利用側層（engine と同順位・engine ではない）」
  節を追加し索引行を登録（計18→19クレート）
- 検証列（`mise run fmt`/`check`/`lint`/`test`）全パス・
  `doc_status.js audit` findings 0。依存は `ruviform-media-core` のみ
  （Cargo.toml 確認済み）
