---
id: "TASK-Ze4Rp7MvBkWc2QnYt9LdXh"
type: "task"
status: "in-progress"
title: "Issue #434 — Spectrogram ViewSpec kind の実装タスク"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/434"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-434-spectrogram-kind.md"
      - type: "file"
        path: "docs/designs/issue-434-spectrogram-kind.md"
      - type: "file"
        path: "docs/plans/issue-434-spectrogram-kind.md"
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
    - "docs/plans/issue-434-spectrogram-kind.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# Issue #434 — 実装タスク

## 完了

- [x] T1 文書 chain（spec / design / plan / task / test-spec）を `docs/` に作成。
- [x] T2 `SpectrogramColumn` / `SpectrogramSpec` / `MAX_SPECTROGRAM_CELLS` と `ComponentSpec::Spectrogram`（`key` / `kind` / validation）。
- [x] T3 契約の単体テスト `spectrogram_keeps_gap_cells_and_rejects_unbounded_payloads`（NaN セル・count==0 列の保持、行数不一致・上限超過・不正な magnitude_range の拒否）。
- [x] T4 Fake 側 parity（`parity_spectrogram_payload_retained_fake`）と fixture の Spectrogram child・`expect.spectrogram`。
- [x] T5 GPUI 描画: `spectrogram_element`（canvas）と純関数 `spectrogram_cell_plan`。
- [x] T6 GPUI 側テスト: `spectrogram_plan_skips_gaps_and_clamps` と fixture 一致の拡張（`parity_new_kinds_plans_match_shared_fixture`）。
- [x] T7 `test-ui-parity` の説明と開発環境表の更新。
- [x] T8 headless 検証（`test-ui-parity` 4 件、`test-ui-component-view` 17 件、`ruviform-ui` lib 64 件）pass。
- [x] T9 GPUI 検証（`test-ui-gpui` 47 件 + `test-ui-consumer-observation` 4 件）pass。

## 残作業

- [ ] T10 `lint` / `lint-ui-gpui` / `fmt-check` / `check-ui-driver-boundary` / `test-ui-test-msrv` を merge 後 HEAD で実行し結果を記録する。
- [ ] T11 review-protocol（独立サブエージェントによる diff レビュー）の実施と指摘への対応。
- [ ] T12 commit・push・PR 作成（日本語、`Refs #434`、Issue は close しない）と head SHA の報告。

## 未達（follow-up へ分離）

- [ ] F1 AudioLens 側の Spectrogram 射影（audio-lens#166 の残課題）。ruviform 側の kind 追加完了後に AudioLens 側で行う。
- [ ] F2 `ruviform-viz` の `Tile` から `SpectrogramColumn` への consumer 側写し替え。
- [ ] F3 専用 theme token による Spectrogram 色（現状は `viz.waveform` の alpha）。
- [ ] F4 JSON 上の非有限値表現（D5）。
