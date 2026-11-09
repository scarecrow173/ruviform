---
id: "PLAN-84yUP5PkLYsA3yfDgOShzQ"
type: "plan"
status: "proposed"
title: "Issue #426 GPUI lab binding 実装計画"
created: "2026-11-08"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-426-gpui-lab-binding.md"
    - "docs/designs/issue-426-gpui-lab-binding.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-426-gpui-lab-binding.md"
  implemented-by:
    - "docs/tasks/issue-426-gpui-lab-binding.md"
---

# Issue #426 実装計画

## 段階

| 段 | 内容 | 検証 |
| --- | --- | --- |
| P1 | `driver_lab.rs` の mise-task `package`+`filter` 検査 | `test-ui-driver-contract`（正例・負例） |
| P2 | GPUI binding test（`lab_binding.rs`・`render.rs`） | `test-ui-gpui` |
| P3 | Story matrix（fake）・共通 op 順序の決定化 | `test-ui-component-view` |
| P4 | manifest（cases/stories）更新 | `test-ui-driver-contract` |
| P5 | Consumer 検査の実行・記録（変更なし） | consumer `check-cli-headless` / `check-headless-deps` |
| P6 | Golden 境界の確認・NotRun 記録 | IR |
| P7 | fmt / lint / boundary / 全体 test・merge 後 HEAD で再検証 | `fmt-check` `lint` `lint-ui-gpui` `check-ui-driver-boundary` `test` |
| P8 | 共有 story fixture（`lab_story.rs`）新設・Fake/GPUI adapter の同一 oracle walk | `test-ui-component-view` `test-ui-scenario-contract-fake` `test-ui-scenario-contract-gpui` |
| P9 | GPUI story matrix（`tests/lab_story.rs`）+ story-keyed golden link（Unsupported 正直記録） | `test-ui-gpui` |
| P10 | `golden_capture.rs` story-keyed 実窓 capture（ignored・desktop）+ tick guard 修正 | `--features gpui --test golden_capture -- --ignored`（ローカル Windows で実測） |
| P11 | stories.json GPUI binding を pass に更新（honest note 付き） | `test-ui-driver-contract` |

## 依存

- P2 は P1 の filter 検査に依存する（pass の根拠を機械検査してから binding を pass にする）。
- P4 は P2・P3 の test 名確定後。
- P5 は独立（consumer は read-only）。

## リスク

| リスク | 対策 |
| --- | --- |
| gpui test platform の window activation が `RvfView::new` では購読されない | cancel 検証は `RvfView::mounted` 経路で行う |
| gpui test platform の scale が 2.0 固定 | scale 軸は `ResolvedTheme::with_scale` の ScaleContext で与え tokens 不変を検査（Fake/GPUI 同一解釈）。window pixel の scale 1.0 は real-os/Visual lane の残件として明記し pass の対象外とする |
| `HashMap` 反復順による op 順序の不安定 | 削除キーを sort（`ruviform-ui/src/view.rs`） |
| consumer 検査が ruviform の pinned revision で評価される | IR に明記。consumer 変更は範囲外 |

## ロールバック

各段は独立した test 追加と manifest の status 変更。問題があれば該当 binding を
`not_run`（owner と rerun 必須）へ戻す。共通層の変更は sort 1 行のみで、revert は単独で可能。
