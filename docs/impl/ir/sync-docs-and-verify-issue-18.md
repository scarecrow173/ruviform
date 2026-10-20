---
id: "IMPL-034J5L5fAAqFUmUkYGAf1t"
type: "impl"
status: "completed"
title: "Sync docs and verify Issue 18"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/18"
  changes:
    added: []
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/sync-docs-and-verify-issue-18.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/implement-issue-18-data-model.md"
    - "../../designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/media-data-model-api.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Sync docs and verify Issue 18

## Summary

task `TASK-034J5KRwAtAqF1xu0moYuS`（plan `PLAN-034J5K603qXDNmT82wI9v7` Phase D）として、Issue #18 着地に伴う `Cargo.toml` / `README.md` / `AGENTS.md` の記述同期、ワークスペース総合検証スイート、独立レビューサイクル（A・B → Validator → Judge → 修正 wave）を実施し、PR #39 を main へマージした（merge commit `d041ba9`、feature commit `bc81838`、Closes #18）。

## Context

- 実施タスク: [../../tasks/sync-docs-and-verify-issue-18.md](../../tasks/sync-docs-and-verify-issue-18.md)
- 上流: [plan](../../plans/implement-issue-18-data-model.md)、[spec](../../specs/implement-media-data-model-api.md)（受け入れ基準の突合対象）、[test spec](../../test-specs/media-data-model-api.md)
- 前提: タスク 1〜3 の実装完了（media/audio/video-core のデータモデル API）
- 制約: `docs/` 配下（歴史的記録）は変更対象外。検証入口は `mise run <task>` のみ

## Implementation

### Changes

- `Cargo.toml`（L5–6 コメント）: 「クレート間の path 依存は API 未確定のため未設定」→ API 確定に伴い段階追加・Issue #18 で audio-core・video-core → media-core を設定済み、と記録（値行不変）
- `README.md`: 冒頭説明（Phase 1 基盤クレート群のうち 3 クレートはデータモデル API 実装済み）・現状節・未確定リスト（公開 API の限定化）・RT 契約段落（API 単位 RT ラベル付与済み・ランタイム API は未着手）・`crates/` 説明を実態へ同期
- `AGENTS.md`（ワークスペース構成段落）: 3 クレート実装済み・path 依存設定済み・残りスケルトン（rt は `RtContext` のみ）へ更新し、「公開 API 未確定」を 3 クレート以外に限定

### Decisions During Implementation

- **docs 更新は事実突合ベース**: docs 整合レポート（`tmp/issue-18/docs-report.md`）で実クレート状態（`Cargo.toml` L11 の path 依存・`**RT contract:**` ラベルの実在）を grep 確認してから文言を確定
- **レビュー修正は blocking のみマージ前必須**: Judge 判定で A1・A2・A3/C3 を blocking、C1+C2・C4・C5 を非 blocking とし、7 件すべてを修正 wave で処理（C5 は文書化済み判断のため変更不要 ruling）
- **コーディネータ rulings**: `MediaDuration::to_seconds_f64` 追加（A4 仕様曖昧の決着）/ exhaustive 列挙型維持（C5）/ `AudioBuffer::new` は `-> Self` + 文書化 panic（形状エラーは `Result` コンストラクタ側）/ timecode は media-core 範囲内・デバイス境界 trait は別 Issue / `MediaDuration` 命名

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows x64・Git Bash。検証スイート実績（`tmp/issue-18/verify-report.md` + 修正 wave `tmp/issue-18/fix-report.md`）:

| Check | コマンド | 結果 |
| --- | --- | --- |
| fmt-check | `mise run fmt-check` | PASS（差分なし） |
| check | `mise run check` | PASS・0 warnings |
| lint | `mise run lint` | PASS・全 6 クレート 0 warnings |
| test | `mise run test` / `mise exec -- cargo test --workspace` | PASS・最終 80 単体テスト + 50 doctest = 130 合格・0 失敗（初回計測は 75 + 49 = 124） |
| test-rt | `mise run test-rt` | PASS・意図した 10 テストのみ選択（初回は `view_rejects_short_backing` 誤選択 1 件 → 改名で解消。`convert_exact` doctest 名の誤検出余地は substring フィルタの既知制約） |
| bench | `mise run bench` | PASS（ベンチマーク未実装・実質 no-op） |
| publish/dep graph | `mise exec -- cargo metadata --format-version 1 --no-deps` | 全 6 パッケージ `publish` が `[]`（公開禁止）・依存辺は audio-core→media-core・video-core→media-core のみ。media-core 依存ゼロ・audio↔video なし・cycle なし |
| prek | `mise exec -- prek run --all-files` | 全フック Passed・ファイル変更不要 |
| git status | `git status --short` | 意図したファイルのみ（`tmp/` は `.gitignore` で非追跡） |

レビューサイクル:

- Reviewer A（spec 適合）: A1 P2（約 17 アクセサのテストカバレッジ欠落）・A2/A3/A4 P3。Reviewer B（健全性）: C1/C2 P2（`isize::MAX` 容量 panic）・C3/C4/C5 P3
- Validator: 8 件 CONFIRMED・A4 UNVERIFIED（仕様曖昧 → coordinator ruling で `to_seconds_f64` 追加）
- Judge: A1・A2・A3/C3 を blocking（契約明示違反・安価な修正）と判定 → 修正 wave 7 件適用後に **ACCEPT WITH NON-BLOCKING NOTES**
- 修正 wave の内訳: カバレッジ追加 3 テストファイル、`view_rejects_undersized_backing` 改名、`version()` RT ラベル 3 箇所、`isize::MAX` 境界 2 系統、`Plane::new`/`PlaneMut::new` の stride 不変条件、`MediaDuration::to_seconds_f64`

## Risks

- `mise run test-rt` の substring フィルタは精度限界あり（`rt_` を内包する非 RT テスト名・`convert_exact` doctest 名が誤検出余地）。フィルタ改善は別検討事項として ledger に記録
- `cargo metadata` の `publish` 値は `[]`（禁止）であり `null` ではない。チェックリスト文言どおり `null` を grep すると誤判定になる点を verify-report の attention item として記録済み
- C5（exhaustive 列挙型）・残る非 blocking 注記は公開時の再評価事項として ledger に留保

## Follow-ups

- 本遡及 doc チェーン（spec → plan → test-spec → 4 task → 4 IR）は Issue #18 完了後の記録整備であり、今後の実装ストリームは同一チェーンを事前作成で運用する
- `test-rt` フィルタの厳密化（プレフィックスマッチまたはレーン分離）は RT 検証 Issue 側で検討
