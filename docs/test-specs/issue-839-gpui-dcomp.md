---
id: "TSPEC-034ZbbClipPart000001"
type: "test-spec"
status: "approved"
title: "Issue #839 GPUI fork clip-partition fix — test spec"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/839"
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-839-gpui-dcomp.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-839-gpui-dcomp.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-839-gpui-dcomp.md"
    - "docs/designs/issue-839-gpui-dcomp.md"
    - "docs/discovery/issue-839-gpui-dcomp.md"
    - "evidence/rf532/clipfix-results.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-839-gpui-dcomp.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-839-gpui-dcomp.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 839
---

# Issue #839 clip-partition — test spec

## 検証対象

`docs/specs/issue-839-gpui-dcomp.md` の受入条件 —
baseline arm screen green `>= 0.5`（docked-in-place、治療
arm 不使用）、child style/parentage 不変、timer 駆動 sync、
reentrancy panic なし、device-lost 再構築、既存 verdict
集合の維持。

検証時点の gpui: fork `rf839-dcomp-child-raster` branch
`b3144f2` を uncommitted `[patch]` override で参照
（Cargo.lock pin 更新は次 slice へ defer）。

## 検証 lane と実施結果

| lane | 実行 | 結果 |
| --- | --- | --- |
| `mise run test-ui-plugin-dock-real`（canonical、patched gpui） | 実施 | **`real-dcomp-rasterfix` = Pass、`fix_arm = "baseline"`、baseline screen green `0.999994`**。`real-dcomp-pixel`/`bandescape`/`childinv` は既知未解決の honest Fail で exit 1（設計どおり）。`evidence/rf532/plugin-dock-real-clipfix.jsonl` |
| `--pixel-artifacts` aux run | 実施 | 同上 verdict + PNG 証跡 `evidence/rf532/clipfix-pixels/`、`evidence/rf532/plugin-dock-real-clipfix-artifacts.jsonl` |
| DComp-off aux run（`GPUI_DISABLE_DIRECT_COMPOSITION=1`） | 実施 | `real-dcomp-rasterfix` = Pass（`fix_arm = "reparent"`、baseline 0.0 → reparent 1.0）— fix は DComp path のみに効くことを確認。`evidence/rf532/plugin-dock-real-clipfix-dcomp-off.jsonl` |
| `cargo check -p gpui`（gpui clone） | 実施 | pass（warning なし、既存 taffy warning のみ） |
| standalone probe（gpui example、一時的） | 実施・削除済み | clip あり: screen `1.000` / window-DC `1.000`；clip なし: `0.000`/`1.000`；flag あり: `0.000`/`0.000` — 2 層分離の確定証拠 |
| `cargo fmt --all -- --check` | 実施 | pass |
| `mise run check-ui-gpui` | 実施 | pass（patched gpui で compile） |
| `mise run lint-ui-gpui` | 実施 | **fail（scope 外の既存 issue）**: `crates/ruviform-ui-gpui/src/gpui_driver/surface.rs:1644` の `clippy::cast_sign_loss`（`len as usize`）— main 上の既存コード + toolchain 側の lint で、本 slice の gpui 変更とは無関係。ruviform 側コードは編集範囲外のため未修正のまま記録 |
| `cargo test -p ruviform-ui-gpui --features "gpui-test uitest"` | 実施 | 78 lib + 22 consumer_observation + 4 lab_story + 7 scenario_contract + 2 windowed_uitest + 1 golden_capture 全 pass（real-window tests は desktop 要のため ignore、設計どおり） |
| `mise run test-ui-native-slot` | 実施 | 13 tests pass |
| `mise run test-ui-plugin-dock` | 実施 | lane exit 0 |
| `mise run test-ui-plugin-dock-console` | 実施 | `--list` + `--run` exit 0 |
| `mise run check-ui-driver-boundary` | 実施 | pass（dep-graph / signature / allowlist） |
| `mise run test-ui-plugin-dock-real-headless` | 実施 | 全行 `NotRun` + `environment_blocked` の fail-closed mapping、exit 0 |

## verdict の誠実性チェック

- `fix_arm = "baseline"` は arms 未実行の達成 — observed の
  `rf_arms` step に `skipped: true, reason: "baseline screen
  green already meets the threshold"` と記録され、
  reparent/topmost は一切実行されていない（AC2 の裏付け）。
- `rf_baseline_state` で container/child ともに exstyle
  `0x0`（layered/topmost/noredirectionbitmap 全て false）を
  確認 — fix が child の style を変えていない。
- 3 回の独立 run（canonical / artifacts aux / probe）で
  baseline `0.999994` / screen `1.000` を再現 — 単発の
  flake ではない。
- 残 Fail（pixel/bandescape/childinv）は unpatched baseline
  run と同一集合・同一原因（外部 window 遮蔽、
  `WS_EX_LAYERED` 適用可否の診断）で、本 fix による
  回帰ではない。
