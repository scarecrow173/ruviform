---
id: "TASK-IfiV9xO2raU6D60zy70KLs"
type: "task"
status: "in-progress"
title: "Issue #314 slice 1 — Golden Utility コアの実装タスク"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-314-golden-core.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/key.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/image.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/diff.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/store.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/report.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_lab.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/golden.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/golden/README.md"
      - type: "file"
        path: "crates/ruviform-ui-test/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-314-golden-core.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-314-golden-core.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-314-golden-core.md"
---

# Issue #314 slice 1 — Golden Utility コアの実装タスク

## T1: 依存追加と module 骨格

- 対象: `crates/ruviform-ui-test/Cargo.toml`、`src/lib.rs`、`src/golden.rs`。
- 作業: `png = "0.18"`・`base64 = "0.22"` を依存追加（理由コメント付き）。
  `golden` module と `GoldenVerdict`/`GoldenError` を定義し lib.rs から
  re-export。
- 完了条件: `cargo check -p ruviform-ui-test` が通り、`Cargo.lock` が
  更新される。

## T2: `golden/key.rs` — GoldenKey / EnvironmentMeta

- 対象: `src/golden/key.rs`。
- 作業: `GoldenKey`（story/theme/state/driver_class/os_class/gpu_class/
  scale_factor/font_class/revision）・`EnvironmentMeta`・`GpuMeta`・
  `CapturePath`（`#[non_exhaustive]` enum）。`canonical()`・
  `file_stem()`・serde round-trip。
- 完了条件: key 差異が canonical/stem 差異になる unit test が通る。

## T3: `golden/image.rs` — RgbaImage + codec

- 対象: `src/golden/image.rs`。
- 作業: `RgbaImage`（検証付き構築・pixel access）、`from_png`/`to_png`
  （normalize_to_color8 + RGBA8 正規化 + 次元上限）、`from_screenshot`
  （png/rgba-raw のみ受理）。
- 完了条件: PNG round-trip・Screenshot decode・拒否系の unit test が通る。

## T4: `golden/diff.rs` — DiffPolicy + diff

- 対象: `src/golden/diff.rs`。
- 作業: `ChannelTolerance`/`MaskRect`/`DiffPolicy`、純粋関数 `diff` が
  統計 + 差分画像（差分=赤・masked=減光）を返す。alpha=0 両側一致規則。
- 完了条件: 許容差境界・mask・統計・差分画像の unit test が通る。

## T5: `golden/store.rs` + `golden/report.rs` — store/evidence

- 対象: `src/golden/store.rs`、`src/golden/report.rs`、`src/driver_lab.rs`。
- 作業: `BaselineStore`（layout・sidecar 検証・`write_candidate`・
  `promote`・`compare`→`GoldenReport`）、`GoldenEvidence` JSONL
  （`RUVIFORM_GOLDEN_EVIDENCE`）。driver_lab の path 解決/append を
  `pub(crate)` helper 化して再利用。
- 完了条件: store round-trip・key 不一致拒否・verdict 写像・evidence
  append の unit/integration test が通る。

## T6: 統合 test・fixtures・文書

- 対象: `tests/golden.rs`、`fixtures/golden/README.md`、crate README、
  `mise.toml`（`test-ui-golden`）、`docs/development-environment.md`。
- 作業: 合成画像による end-to-end（candidate→promote→compare→evidence）
  を tempdir で検証。baseline 承認手順・semantic oracle 境界を文書化。
- 完了条件: `mise run test-ui-golden` が通る。

## T7: 検証と PR

- 作業: `docs/plans/issue-314-golden-core.md` の検証マトリクス 1〜7 を
  実行。review-protocol（独立レビュー相当の分離 pass）を実施し指摘を
  反映。`git fetch origin main && git merge origin/main` で最新化して
  再検証。
- 完了条件: 検証結果と NotRun/Unsupported（capture 経路が slice 3 対象外
  である旨を含む）を PR 本文に記載。PR は `Refs #314`（close しない）。
