---
id: "TASK-Gy5KbQe2WnRt8LxVc3PmHo"
type: "task"
status: "done"
title: "Issue #435 — Label/Table ViewSpec kind の実装タスク"
created: "2026-11-08"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/435"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/label-table.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-435-label-table-kinds.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# Issue #435 — 実装タスク

## 完了

- [x] T1 `LabelSpec` / `TableSpec` / `CellValue` / `A11yNode` と `ComponentSpec::{Label, Table}`、`A11yRole` 5 variant を追加。
- [x] T2 Table の検証（行長・非有限数・行列上限）と `ViewHost::a11y_node`。
- [x] T3 contract unit test 4 件（順序・欠損保持、検証、Label 既定 role、mount/reconcile）。
- [x] T4 parity fixture `driver-lab/parity/label-table.json`。
- [x] T5 Fake 側 parity テスト 3 件（mount/reconcile、a11y 木一致、payload 保持）。
- [x] T6 GPUI の `table_plan` と `label_element` / `table_element`、`element_for` の arm。
- [x] T7 GPUI parity テスト 3 件（plan と fixture の一致、drawn grid = a11y grid、順序・欠損の単体、RvfView 描画）。
- [x] T8 `cases.json` の UI-DVR-02 に Label/Table の fake・gpui binding を追加（実行して pass を確認したもののみ）。
- [x] T9 `cargo fmt` と、§5 の検証のうち merge 前 HEAD で実行できるもの。

## 部分達成・未達

- [ ] T10 GPUI の platform a11y API への接続（OS の accessibility tree）。renderer は a11y を描画に使っておらず、a11y の実 OS 検証は別 Gate。現状は drawn grid と contract の a11y 木の一致で代替。follow-up Issue を推奨。
- [x] T11 AudioLens 側 Job/Run/Analysis の ViewSpec 化。consumer 側の作業で、本 kind の merge 後に別 PR で行う。（2026-11-09 に audio-lens main で確認: `view/run_screen.rs` の状態 Label・stage Table、`job_list` の一列 Table、`analysis_screen.rs` の events/alignments Table が `ruviform_ui::view` の `label()`/`table()` helper 経由で `ComponentSpec::Label`/`Table` を生成している）
- [ ] T12 大量行の仮想化・スクロール。上限は固定値で、超過は mount 時に拒否する。

## 最終確認

- [x] T13 merge 後 HEAD で検証マトリクス（plan §5）を全件実行し、結果を PR 本文に記録する。（PR #440 で merge 後 HEAD `a777f14b` の結果を記録済み。2026-11-09 に `chore/verify-442-443-435`（= `origin/main` `4e81b055` + docs のみの差分）で再実行し全件 pass。`docs/impl/ir/issue-442-443-435-verification-closeout.md` 参照）
- [x] T14 review-protocol（独立サブエージェントによる diff レビュー）を実施し、blocker がないことを確認する（PR #440 本文に記録済み。判定 ACCEPT WITH NON-BLOCKING NOTES、P0/P1 なし）。
