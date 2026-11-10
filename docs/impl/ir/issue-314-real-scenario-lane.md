---
id: "IMPL-034VgClFxuPB48DOb3hBDv"
type: "impl"
status: "completed"
title: "Issue #314 slice 4 follow-up — real-scenario lane and story matrix IR"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
    - "docs/specs/issue-314-real-scenario-lane.md"
    - "docs/designs/issue-314-real-scenario-lane.md"
    - "docs/plans/issue-314-real-scenario-lane.md"
    - "docs/tasks/issue-314-real-scenario-lane.md"
    - "docs/test-specs/issue-314-real-scenario-lane.md"
    - "docs/impl/ir/issue-314-real-window-adapter.md"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-314-real-scenario-lane.md"
      - type: "file"
        path: "docs/specs/issue-314-real-scenario-lane.md"
      - type: "file"
        path: "docs/designs/issue-314-real-scenario-lane.md"
      - type: "file"
        path: "docs/plans/issue-314-real-scenario-lane.md"
      - type: "file"
        path: "docs/tasks/issue-314-real-scenario-lane.md"
      - type: "file"
        path: "docs/test-specs/issue-314-real-scenario-lane.md"
      - type: "file"
        path: "evidence/issue-314-real-scenario/driver-lab-evidence.jsonl"
      - type: "file"
        path: "evidence/issue-314-real-scenario/golden-evidence.jsonl"
      - type: "directory"
        path: "evidence/issue-314-real-scenario/png"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/real_window.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/stories.json"
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
  implements:
    - "docs/tasks/issue-314-real-scenario-lane.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-314-real-window-adapter.md"
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

# Issue #314 slice 4 follow-up — real-scenario lane and story matrix IR

## Summary

Issue #314 slice 4 の residual follow-up。canonical mise lane
`test-ui-real-scenario` を追加し、`tests/real_window.rs` を
{theme × scale} 4 cell matrix（`synvelis-dark`/`audiolens-dark` ×
1.0/2.0）に拡張した。Windows 実 desktop session で 2 story × 4 cell
= 8 session 全てが pass し、driver-lab/golden JSONL と cell 別
PNG/meta 証拠を `evidence/issue-314-real-scenario/` に記録した。
`stories.json` の `drivers.real-os` を `mise-task` mechanism で
更新した。Issue #314 は open のまま（consumer gate・OS input
#313・macOS/Linux・Plugin Dock snapshot は別 scope）。

## Context

`docs/tasks/issue-314-real-scenario-lane.md`（plan
`docs/plans/issue-314-real-scenario-lane.md`、spec/design/test-spec
同名）に基づく。PR #554（`317094b7`）で `RealOsGuiTest`・
`real-scenario` feature・`tests/real_window.rs` が merge 済みだが、
canonical mise lane 未追加と story matrix の scale 2.0・第 2 theme
cell 未実走が残件だった。

## Implementation

### Changes

- `mise.toml`: `[tasks.test-ui-real-scenario]` を追加
  （`example-ui-gpui-real-os-*` と同じ manual lane 位置づけ、CI lane
  には追加しない）。env で driver-lab/golden evidence JSONL と
  `RUVIFORM_REAL_WINDOW_PNG_DIR` を `evidence/issue-314-real-scenario/`
  に向け、`cargo test -p ruviform-ui-gpui --features real-scenario
  --test real_window` を実行する。
- `crates/ruviform-ui-gpui/tests/real_window.rs`:
  - `THEMES`/`SCALES` 定数で 2×2 matrix を宣言し、
    `real_window_story` が cell ごとに独立した real-window session
    （`RealOsGuiTest::run`）を起こす。
  - theme/scale は `ViewMount.theme` に
    `ResolvedTheme::with_scale(ScaleContext { scale_factor,
    rem_base_px: 16.0 })` を渡して mount。scale は mapper-side
    hint で token 不変を cell ごとに assert（Fake/GPUI-simulated
    matrix と同一規則）。
  - `session_failure`: session 起動時の `UiError::Host`/
    `BackendUnavailable` のみ driver-lab evidence に
    `observed="environment_blocked"` を記録し `environment_blocked:`
    tag で panic。`Internal`/`Unsupported`/`ScenarioError`/assertion
    は通常の test failure のまま（cargo test に exit-code 区分が
    ないため JSONL observed + panic tag で区別 — #552
    `os_input::preflight` の verdict 分離と同じ思想だが、本 lane
    は OS input を注入しないため preflight 自体は適用しない）。
  - `story_golden_key`: `theme` は cell の実 mount theme、
    `scale_factor` は `CaptureMeta` の実測値を使用。
  - `png_evidence_dir`: `RUVIFORM_REAL_WINDOW_PNG_DIR` の相対 path を
    `CARGO_MANIFEST_DIR/../..`（workspace root）基準で解決 —
    `driver_lab::evidence_target` と同一規則で、cargo test の
    package-root CWD から crate 配下への漏れを防ぐ。
  - PNG/meta file 名に story・theme・scale を含め cell 間の
    上書きを防止。`golden_loop` を分離して clippy
    `too_many_lines` を解消。
- `crates/ruviform-ui-test/fixtures/driver-lab/stories.json`: 両
  story の `drivers.real-os` note を 4 cell matrix・canonical lane・
  environment_blocked 分類の実績記述に更新し、mechanism を
  `mise-task`/`test-ui-real-scenario`（package+filter 付き）に変更。
- docs index（specs/designs/plans/tasks/test-specs README）を
  doc tooling で再生成。

### Decisions During Implementation

- `environment_blocked` の表現: `cargo test` は環境不通を通常の
  nonzero exit と区別できないため、driver-lab JSONL の
  `observed` フィールドと panic message の `environment_blocked:`
  prefix の 2 点で記録する。headless 環境で実行しても
  「未実行を pass と見せない」境界を維持する。
- scale 軸の扱い: `GoldenKey.scale_factor` は要求値ではなく
  `CaptureMeta` 実測値を使う（design どおり）。PNG/meta file 名の
  scale は要求 cell 値 `{scale_factor:.1}`。
- `#![cfg(all(windows, feature = "real-scenario"))]` は維持 —
  macOS/Linux capture backend は別 scope であり、本 slice で
  非 Windows の vacuous pass を新たに作り込まない（test binary が
  0 test になるのは slice 4 からの既存契約）。

## Related Experiments

### Adopted

なし（機械的 slice のため experiment log なし）。

### Rejected

なし。

## Validation

- `mise run test-ui-real-scenario`（Windows 11 desktop session、
  interactive）: 2 tests passed / 8 real-window sessions、exit 0。
  driver-lab evidence 8 行全て `observed=pass`、golden verdict は
  cell ごとに `blocked/baseline_missing` → `pass/match`。
  `evidence/issue-314-real-scenario/png/` に 16 PNG + 8 meta.jsonl。
- 補助証拠（canonical = mise、cargo 直接実行）:
  `cargo check`/`clippy -D warnings`/`fmt --check` on
  `real_window` test target — 全て exit 0。
- `mise run fmt-check` / `check` / `lint` / `test` /
  `test-ui-*` lane / `check-ui-driver-boundary`: 実施結果は PR 本文
  の検証表を参照。

## Risks

- 対話 desktop session がない環境（CI・headless・session-0）では
  全 cell が `environment_blocked` で fail する設計 — manual lane
  であり CI 対象外のため許容。`stories.json` status は実測に基づく
  `pass` を維持するが、再実行は desktop session 前提。
- `environment_blocked` は panic + JSONL observed のみで cargo の
  exit code を区別しない — CI で誤って必須化された場合に環境不通が
  test failure と同じ見え方になる点は lane 説明と stories.json note
  で明示済み。

## Follow-ups

- Issue #314 は open のまま: consumer gate・OS input（#313）・
  macOS/Linux capture backend・Plugin Dock snapshot は別 scope。
- `real-scenario` lane の非 Windows 対応（vacuous pass の扱い）は
  macOS/Linux backend slice で再検討。
