---
id: "PLAN-Ws6Tz1RbMn4QcKe9YhGx3Dp"
type: "plan"
status: "in-progress"
title: "Issue #443 — Tile 列区間と count 契約の実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/443"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-443-tile-column-contract.md"
      - type: "file"
        path: "docs/test-specs/issue-443-tile-column-contract.md"
    modified:
      - type: "file"
        path: "crates/ruviform-viz/src/spectrogram.rs"
      - type: "file"
        path: "crates/ruviform-viz/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-viz/README.md"
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-443-tile-column-contract.md"
    - "docs/designs/issue-443-tile-column-contract.md"
  implemented-by: []
  depends-on: []
  blocks: []
---

# Issue #443 — Tile 列区間と count 契約の実装計画

## スコープ分割

単一 PR（`feature/443-tile-column-contract`）で完結させる。分割しない理由: 変更は `ruviform-viz` の 1 モジュールとその export・doc に閉じ、`ruviform-ui` 側は doc 1 行である。

## ファイルマップ

| ファイル | 変更内容 |
| --- | --- |
| `crates/ruviform-viz/src/spectrogram.rs` | `TileColumn`、`Tile::columns`、`column_offsets`（単一の列区間規則）、`tile` の列区間を共有関数へ置換、単体テスト 4 件 |
| `crates/ruviform-viz/src/lib.rs` | `TileColumn` の re-export |
| `crates/ruviform-viz/README.md` | `Tile::columns` の説明 |
| `crates/ruviform-ui/src/view.rs` | `SpectrogramColumn::count` の doc 修正（型・validation 不変） |
| `docs/specs`・`docs/designs`・`docs/plans`・`docs/tasks`・`docs/test-specs` | `issue-443-tile-column-contract` の文書 chain |

## 依存グラフ

1. spec（完了）→ design（完了）→ plan（本書）→ task・test-spec
2. `column_offsets` → `Tile::columns` → `tile` の置換（同じ式を使うことで既存テストが回帰検出になる）
3. `lib.rs` export → テスト（外部から `TileColumn` を参照）
4. `ruviform-ui` doc 修正は独立

## 作業順序

1. 既存テストが green であることを確認（baseline）。
2. テストを先に書く（`columns_partition_frames_for_every_span`、`columns_cover_frames_when_cols_exceed_span`、`tile_columns_split_indivisible_span_and_mark_nan_gap`、`column_count_counts_finite_cells`）。
3. `column_offsets` と `Tile::columns` を実装し、`tile` を置換する。
4. `lib.rs` export、README、`ruviform-ui` の doc。
5. 検証（後述）と、merge 後 HEAD での最終検証。
6. review-protocol（独立サブエージェントによる diff レビュー）。
7. PR 作成（日本語、`Refs #443`、Issue は close しない）。

## リスク

| リスク | 影響 | 対策 |
| --- | --- | --- |
| `tile` の置換で列区間がずれる | 既存の Spectrogram 出力が変わる | 既存テスト（aggregate / clamp / stft）を無変更で pass させる。列区間の全組合せテストを追加 |
| `count` の意味変更で consumer が誤読する | AudioLens の写し替えが sample 数を前提にすると誤る | spec / design / `ruviform-ui` doc に「有限セル数」と明記。AudioLens 側は follow-up として PR 本文に記載 |
| `cols > span` で列が重なる | consumer が重なりを分割と誤認する | D3 と doc に明記。テストで重なりを固定 |
| 空 frame 範囲で `tile` が範囲外参照する（既存） | パニックの可能性 | 本 PR の範囲外として報告。挙動は変えない |
| `RUSTFLAGS` / Windows 環境で gpui 系の検証が実行できない | UI 系の検証が NotRun | NotRun と理由を PR 本文に明記 |

## ロールバック

変更は `ruviform-viz` の追加 API（`TileColumn` / `Tile::columns`）と `tile` 内部の共有関数化のみ。問題があれば `tile` の式を元の 3 行へ戻し、追加 API を削除すれば、既存の公開 API は変わらない。`ruviform-ui` は doc のみのため戻しは容易。

## 検証マトリクス

| 検証 | 対象 | 実行 | 期待 |
| --- | --- | --- | --- |
| 単体（viz） | 列区間分割・cols > span・NaN 列・有限セル数 | `mise exec -- cargo test -p ruviform-viz --lib` | 39 件 pass（新規 4 件を含む） |
| `mise run test` | ワークスペース全体のテスト | `mise run test` | pass |
| `mise run lint` | clippy `-D warnings` | `mise run lint` | warning なし |
| `mise run fmt-check` | rustfmt | `mise run fmt-check` | 差分なし |
| `mise run check` | 型検査 | `mise run check` | pass |
| `mise run check-ui-driver-boundary` | 共通層に GPUI 型を出さない | `mise run check-ui-driver-boundary` | pass |
| `mise run test-ui-driver-contract` / `test-ui-parity` / `test-ui-component-view` | `ruviform-ui` の契約（doc のみ変更の確認） | 各 mise タスク | pass |
| `mise run test-ui-test-msrv` | MSRV 1.89 | `mise run test-ui-test-msrv` | pass（実行可否は環境依存） |
| `mise run test-ui-gpui` / `lint-ui-gpui` / `check-ui-gpui` | GPUI consumer（`ruviform-ui` 依存） | 各 mise タスク | 環境で実行できない場合は NotRun と理由を記載 |

検証は merge 後 HEAD で実施する。
