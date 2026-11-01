<!--
Issue #219: Ruviform は Synvelis・AudioLens 両 Consumer の共通基盤。
公開 API / feature / behavior / dependency / CI 基盤の変更では、
両 Consumer を独立して impacted / no impact / unknown に分類する。
unknown を残す場合は未検証理由と解消先（Issue リンクまたは
consumer-verify workflow の dispatch 予定）を必須で記す。
レビュアーは merge 前に分類欄の記入と unknown 解消先を確認する。
-->

## Summary

<!-- 変更の要約と Issue リンク（Closes #NNN / Refs #NNN） -->

## Consumer 影響分類

<!-- 各行を impacted / no impact / unknown のいずれかにする。
     impacted: 対応 Issue/PR リンク必須。
     no impact: 根拠のみ記録（不要 Issue は立てない）。
     unknown: 未検証理由 + 解消先リンク必須。unknown 残置 = 互換性未確認。 -->

| Consumer | 分類 (impacted / no impact / unknown) | 根拠・対応 Issue/PR リンク |
|---|---|---|
| Synvelis | | |
| AudioLens | | |

### unknown の解消先

<!-- unknown を 1 行でも付けた場合: 未検証の理由と、解消を追跡する
     Issue / consumer-verify dispatch 予定 / 別 PR へのリンクを記す -->

## 検証証拠

<!-- 実行した検証を Consumer 別に記録する。実行しなかった lane は
     not run と明記する（not run と fail は区別する）。 -->

| 対象 | lane/test | OS | feature | ruviform revision | 結果 (pass/fail/not run) | 許容例外 | Rollback |
|---|---|---|---|---|---|---|---|
| Ruviform | | | | | | | |
| Synvelis | | | | | | | |
| AudioLens | | | | | | | |

## Test plan

<!-- 本 repo の検証手順（mise run verify-ci 等）。Consumer 側の検証は
     上の証拠表と consumer-verify workflow の結果リンクで示す -->
