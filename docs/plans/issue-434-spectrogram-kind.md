---
id: "PLAN-Hd5Ym2QwNtRx8LcGv3KpJe"
type: "plan"
status: "in-progress"
title: "Issue #434 — Spectrogram ViewSpec kind の実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/434"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-434-spectrogram-kind.md"
      - type: "file"
        path: "docs/test-specs/issue-434-spectrogram-kind.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/new-kinds.json"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-434-spectrogram-kind.md"
    - "docs/designs/issue-434-spectrogram-kind.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
---

# Issue #434 — 実装計画

## 1. ファイル対応表

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-ui/src/view.rs` | `SpectrogramColumn` / `SpectrogramSpec` / `MAX_SPECTROGRAM_CELLS`、`ComponentSpec::Spectrogram`、`key`/`kind` の arm、`check_spectrogram`、単体テスト 1 件 |
| `crates/ruviform-ui-gpui/src/render.rs` | `spectrogram_element`、純関数 `spectrogram_cell_plan`、gpui-test 2 件（fixture 一致 + gap/clamp） |
| `crates/ruviform-ui-test/fixtures/driver-lab/parity/new-kinds.json` | Spectrogram child、`mountedKeys` 追加、`expect.spectrogram`（`declaredColumns` / `declaredCells` / `gapColumns` / `drawnCells`） |
| `crates/ruviform-ui-test/tests/driver_parity.rs` | Fake 側 `parity_spectrogram_payload_retained_fake` |
| `mise.toml` | `test-ui-parity` の description を Spectrogram を含む表現へ更新（タスク構成は不変） |
| `docs/development-environment.md` | `test-ui-parity` の表行を更新 |
| `docs/{specs,designs,plans,tasks,test-specs}/issue-434-*.md` | 文書 chain |

`ruviform-ui-theme`、`ruviform-viz`、`ruviform-plugin-*`、`ruviform-ui-webview`、`Cargo.toml` / `Cargo.lock` は変更しない（依存追加なし）。

## 2. 順序付き手順

1. #434 本文・既存 `ComponentSpec`（Waveform/Meter/TimeAxis）・`ruviform-viz` の `Tile` / `TileRequest`・parity fixture を確認する。完了。
2. 文書 chain（spec → design → plan → task → test-spec）を作る。完了（本ドキュメント群）。
3. 契約: `SpectrogramSpec` と validation を追加し、単体テストで NaN gap / count 0 の保持と上限・不正入力の拒否を検査する。完了。
4. Fake: parity fixture に Spectrogram を追加し、`driver_parity` で mount key・shape・gap 列を検査する。完了。
5. GPUI: `spectrogram_cell_plan` 純関数と `spectrogram_element` を追加し、gpui-test で fixture の `drawnCells` と gap/clamp を検査する。完了。
6. 検証: `fmt-check` / `lint` / `lint-ui-gpui` / `check-ui-driver-boundary` / `test-ui-parity` / `test-ui-component-view` / `test-ui-gpui` / `test-ui-consumer-observation` / `test-ui-headless` 系を merge 後 HEAD で実行する。
7. review-protocol（独立サブエージェントによる diff レビュー）を実施し、指摘を検証してから反映する。
8. commit（Conventional Commits）、push、PR（日本語、`Refs #434`、Issue は close しない）。

## 3. 依存関係

- 手順 3 の契約は手順 4・5 の前提。
- 手順 5 の fixture 一致検査は手順 4 の fixture 追加に依存する。
- 手順 6 は手順 3〜5 の完了後。最終検証は merge 後 HEAD で実施する。

## 4. リスク登録

| リスク | 影響 | 対策 |
| --- | --- | --- |
| `ruviform-ui-gpui` の `view.rs` / `render.rs` / `lib.rs` を RF435 が並行編集する | マージ競合 | 差分を Spectrogram 関係の追加だけに限定し、`origin/main` 取り込み後に rebase で解消する |
| JSON で NaN を表現できない | fixture で NaN gap を検査できない | fixture は `count == 0`、NaN は Rust 単体テストで検査。D5 として記録 |
| `ComponentSpec` への variant 追加が exhaustive match の破壊的変更になる | 下流の match が壊れる | `ComponentSpec` は `#[non_exhaustive]` のため破壊的変更にならない。`other =>` arm を維持 |
| GPUI の gpui-test ビルドが重い | 検証時間 | `target/` の既存キャッシュを利用。実機描画の確認は manual 扱い |

## 5. ロールバック

- 変更は `ruviform-ui` の variant 追加と GPUI の arm 追加に限られる。問題があれば `Spectrogram` 関連コードと fixture の追加分を revert すれば既存 kind は元の状態に戻る（既存 kind の挙動は変更していない）。

## 6. 検証マトリクス

| 検証 | タスク | 対象 |
| --- | --- | --- |
| 型・lint | `mise run check` / `mise run lint` / `mise run lint-ui-gpui` | workspace / GPUI feature |
| 形式 | `mise run fmt-check` | workspace |
| 境界 | `mise run check-ui-driver-boundary` | 共通層に GPUI 型が出ないこと |
| headless 契約 | `mise run test-ui-parity` / `mise run test-ui-component-view` / `cargo test -p ruviform-ui --lib`（`mise exec -- cargo test -p ruviform-ui --lib`） | Fake projection・gap 保持・既存 kind |
| GPUI | `mise run test-ui-gpui` / `mise run test-ui-consumer-observation` | plan・fixture 一致・gap/clamp・描画経路 |
| MSRV | `mise run test-ui-test-msrv` | MSRV 1.89 での検証（実行可否は最終報告で明記） |
