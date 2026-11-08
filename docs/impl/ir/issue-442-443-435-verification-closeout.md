---
id: "IMPL-034UcjJXdshRRIGjbzDcS9"
type: "impl"
status: "completed"
title: "Issue #442/#443/#435 検証クローズアウト — 既実装 3 Issue の検証証拠と文書整合"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-442-observer-drawn-spectrogram-label-table.md"
    - "docs/tasks/issue-443-tile-column-contract.md"
    - "docs/tasks/issue-435-label-table-kinds.md"
    - "https://github.com/scarecrow173/ruviform/issues/442"
    - "https://github.com/scarecrow173/ruviform/issues/443"
    - "https://github.com/scarecrow173/ruviform/issues/435"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-442-443-435-verification-closeout.md"
    modified:
      - type: "file"
        path: "docs/specs/issue-435-label-table-kinds.md"
      - type: "file"
        path: "docs/specs/issue-442-observer-drawn-spectrogram-label-table.md"
      - type: "file"
        path: "docs/specs/issue-443-tile-column-contract.md"
      - type: "file"
        path: "docs/designs/issue-435-label-table-kinds.md"
      - type: "file"
        path: "docs/designs/issue-442-observer-drawn-spectrogram-label-table.md"
      - type: "file"
        path: "docs/designs/issue-443-tile-column-contract.md"
      - type: "file"
        path: "docs/plans/issue-435-label-table-kinds.md"
      - type: "file"
        path: "docs/plans/issue-442-observer-drawn-spectrogram-label-table.md"
      - type: "file"
        path: "docs/plans/issue-443-tile-column-contract.md"
      - type: "file"
        path: "docs/tasks/issue-435-label-table-kinds.md"
      - type: "file"
        path: "docs/tasks/issue-442-observer-drawn-spectrogram-label-table.md"
      - type: "file"
        path: "docs/tasks/issue-443-tile-column-contract.md"
      - type: "file"
        path: "docs/test-specs/issue-435-label-table-kinds.md"
      - type: "file"
        path: "docs/test-specs/issue-442-observer-drawn-spectrogram-label-table.md"
      - type: "file"
        path: "docs/test-specs/issue-443-tile-column-contract.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-442-observer-drawn-spectrogram-label-table.md"
    - "docs/tasks/issue-443-tile-column-contract.md"
    - "docs/tasks/issue-435-label-table-kinds.md"
  implemented-by: []
  depends-on: []
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #442/#443/#435 検証クローズアウト — 既実装 3 Issue の検証証拠と文書整合

## Summary

main に統合済みの 3 Issue（#442 GpuiObserver drawn plan への Spectrogram / Label / Table 追加、#443 Tile→SpectrogramColumn 上流契約、#435 Label/Table ViewSpec kind）について、最新 main で検証 lane を再実行し、受入条件への対応表とあわせて docs チェーンの stale な status・checkbox・ID・relation を実態に合わせて修正した記録。コード変更は行っていない（`crates/**` 無変更）。

## Context

- 対象 Issue: https://github.com/scarecrow173/ruviform/issues/442 、/443 、/435
- 実装 PR: #442 = PR #444（`3121f8e6`）、#443 = PR #445（`ccee0cb1`）+ #446（`d3be1ad9`、空 frame panic 修正）、#435 = PR #440（`627e4bc2`）。いずれも main マージ済み。
- 文書 chain: `docs/specs|designs|plans|tasks|test-specs/issue-{435,442,443}-*.md`
- 作業ブランチ: `chore/verify-442-443-435`（`origin/main` = `4e81b055` 起点、docs のみの差分）

## Implementation

### Changes

コード変更なし。`docs/**` のみを更新した。

- spec 3 件: status を `implemented` に更新（#435 / #442 / #443）。
- design 3 件: status を `approved` に更新。
- plan 3 件: status を `completed` に更新し、`relations.verified-by` で対応 test-spec へリンク。#443 の ID を `PLAN-` + 22 文字 Base62 の形式へ修正。
- task 3 件: status を `done` に更新し、検証・レビューの checkbox を実績に合わせて更新。#435 T10（platform a11y）と T12（大量行仮想化）は Unsupported / 対象外として未チェックのまま維持。#443 の AudioLens 式複製 follow-up と空 frame panic follow-up は後続変更で完了した旨を記録。`count <= rows` validation の要否は未対応の follow-up として明記。
- test-spec 3 件: status を `approved` に更新し、`relations.verifies` で spec / design へリンク。ID を `TSPEC-` 形式へ修正（#442 は接尾辞 22 文字へ補正）。
- managed index 5 件（`docs/{specs,designs,plans,tasks,test-specs}/README.md`）を `doc_maintenance.js apply` で再生成。
- 本 Implementation Record と `docs/impl/ir/README.md` の index 行を追加。

### Decisions During Implementation

- #435 の optional hardening（`view.rs` の `columns.is_empty()` 拒否・列見出し重複検証）は実施しない。現行の mount 時検証は行数・有限数・行列上限をカバーしており、空 columns / 重複見出しの拒否は既存契約の拡張であり、本タスクの「検証・文書整合」スコープを超えるため follow-up 候補として記録に留めた。
- `mise run test-viz` は `mise.toml` に存在しないため NotRun。`ruviform-viz` のテスト（#443 の `Tile::columns` 契約テストを含む）は workspace の `mise run test` で実行される。
- consumer 側（audio-lens）の受入照合は、consumer repo の pin 更新を伴う実 test 実行ではなく、audio-lens main のソース確認で代替した（#442: `gpui_parity.rs` に Spectrogram gap 列・NaN セル・Table 欠損セル・Label text の照合あり、#443: `view/screen.rs` が `tile.columns()` を使用し式複製なし、#435: `run_screen.rs` / `job_list` / `analysis_screen.rs` が `label()` / `table()` helper 経由で新 kind を生成）。audio-lens 側の実 test lane 実行は consumer repo の作業として残る。
- docs audit は repository 全体で多数の既存 finding（無関係な invalid ID・status・relation 不整合）を報告する。本タスクでは対象 3 chain のみを修復し、リポジトリ全体の audit が clean であるとは主張しない。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

実行環境: Windows 11 x64 / Git Bash、mise 管理 toolchain（`mise.toml` の pin、Rust 1.99.0 / MSRV lane は rust 1.89.0）。`chore/verify-442-443-435` ブランチ（docs のみの差分、コードは `origin/main` `4e81b055` と同一）で実施。すべて `mise run` 経由で実行し exit code 0 を確認した。

| タスク | 結果 |
| --- | --- |
| `mise run fmt-check` | pass（exit 0） |
| `mise run check` | pass（exit 0） |
| `mise run lint` | pass（exit 0、`-D warnings`） |
| `mise run test` | pass（exit 0、workspace 3088 件・失敗 0） |
| `mise run test-ui-consumer-observation` | pass（exit 0、11 件） |
| `mise run test-ui-parity` | pass（exit 0、11 件） |
| `mise run test-ui-driver-contract` | pass（exit 0、7 件） |
| `mise run check-ui-driver-boundary` | pass（exit 0） |
| `mise run test-ui-gpui` | pass（exit 0、72 件 = unit 61 + consumer_observation 11） |
| `mise run check-ui-gpui` | pass（exit 0） |
| `mise run lint-ui-gpui` | pass（exit 0、`-D warnings`） |
| `mise run test-ui-component-view` | pass（exit 0、17 件） |
| `mise run test-ui-test-msrv`（rust 1.89.0） | pass（exit 0、121 件） |
| `mise run check-ui-headless` | pass（exit 0、`ui/core` tsc --noEmit） |
| `mise run test-ui-headless` | pass（exit 0、`ui/core` node --test 67 件） |
| `mise run test-viz` | NotRun（`mise.toml` に task 未定義。`ruviform-viz` は `mise run test` でカバー） |

### 受入条件との対応

- #442: `DrawnPlan::{Spectrogram, Label, Table}` と `DrawnCell` は `observe.rs` の公開 DTO として存在し、`collect_drawn` は paint と同一の `spectrogram_cell_plan` / `table_plan` / `spec.text` を観測する。`consumer_observation.rs` の `observes_drawn_spectrogram_cells_through_seam` / `observes_drawn_label_and_table_plans_through_seam` が gap 列（`count == 0`）・NaN セルの除外、Label text、Table header/rows、`None` セルの位置保持を fixture 期待値と照合する。`test-ui-consumer-observation` / `test-ui-gpui` / `test-ui-parity` が pass。
- #443: `Tile::columns()` / `TileColumn` / `column_offsets` が `ruviform-viz` の単一実装として存在し、`Spectrogram::tile` と共有される。`columns_partition_frames_for_every_span`（隙間・重なりなし）、`columns_cover_frames_when_cols_exceed_span`（`cols > span`）、`tile_columns_split_indivisible_span_and_mark_nan_gap`（割り切れない span・NaN 列 `count == 0`）、`column_count_counts_finite_cells` が `mise run test` で pass。consumer（audio-lens `view/screen.rs`）は `tile.columns()` をそのまま使用し式複製なし。
- #435: `ComponentSpec::{Label, Table}`・`LabelSpec` / `TableSpec` / `CellValue`・contract 側 a11y 木導出が存在する。`driver-lab/parity/label-table.json` fixture に対し Fake / GPUI の projection と a11y 木が一致し、行列順序・欠損セル位置保持の単体テストが `test-ui-parity` / `test-ui-component-view` / `test-ui-gpui` で pass。platform a11y API 接続は Unsupported（別 Gate、#313 系）、大量行仮想化は対象外。

## Risks

- `drawn` の観測値は spec からの純粋計算であり、実 pixel 出力ではない（設計上の限界。実 OS window での描画確認は manual verification）。
- #435 の platform a11y（OS accessibility tree への接続）は Unsupported のまま。drawn grid と contract a11y 木の一致が代替証拠。
- audio-lens 側の実 test lane 実行は consumer repo の pin 更新後の作業であり、本記録はソース確認による代替証拠に留まる。
- repository 全体の doc audit には対象外の既存 finding が多数残る。

## Follow-ups

- #435: GPUI platform a11y API 接続（#313 系の別 Gate）と大量行仮想化・スクロールは Issue スコープ外として継続。
- #435: `view.rs` の `columns.is_empty()` 拒否・列見出し重複検証の hardening は未実施。必要なら別 Issue で対応。
- #443: `ruviform-ui` の `SpectrogramColumn::count <= rows` validation の要否検討（未対応。follow-up Issue 候補）。
- audio-lens: ruviform pin 更新後に consumer 側 test lane で drawn parity / `Tile::columns` / Label・Table kind の実観測を実行する。
