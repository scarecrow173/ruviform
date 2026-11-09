---
id: "TASK-ovbF_E6zV8WeEi0OsUIw-w"
type: "task"
status: "in-progress"
title: "Issue #426 GPUI lab binding・Story matrix・Golden 境界・Consumer 検査の作業記録"
created: "2026-11-08"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-426-gpui-lab-binding.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lab_binding.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lab_story.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/lab_story.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/golden_capture.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_lab.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/component_view.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/scenario_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/stories.json"
      - type: "file"
        path: "crates/ruviform-ui/src/view.rs"
    deleted: []
    renamed: []
    moved: []
  implements:
    - "docs/plans/issue-426-gpui-lab-binding.md"
---

# Issue #426 作業記録

## 完了（検証済み）

- [x] P1: mise-task の `package`+`filter` 検査を追加（正例 pass・負例で失敗を確認）
- [x] P2: GPUI binding test 6 件（UI-DVR-01/03×2/04/07/08）を追加。`test-ui-gpui` 46 件 pass
- [x] P3: story matrix（LAB-SV-FADER-DOCK / LAB-AL-WAVEFORM-SEEK）を追加。`test-ui-component-view` 17 件 pass
- [x] P3': `ViewHost::update` の Removed op 順序を決定化（story matrix で不安定を検出して修正）
- [x] P4: cases.json（UI-DVR-01/02/03/04/05/06/07/08/09 の GPUI binding）と stories.json（fake pass、gpui not_run）を更新。`test-ui-driver-contract` pass
- [x] P5: consumer 検査 `check-cli-headless`（audio-lens）・`check-headless-deps`（synvelis）pass
- [x] P8: 共有 story fixture `ruviform-ui-test/src/lab_story.rs` を新設。canonical `ViewSpec`・`StoryStep` oracle（`expected_lab_story_trace`）・adapter-neutral `lab_story_walk` を集約し、Fake（`scenario_contract.rs` 3 件）と `component_view.rs` matrix が同一定義を使用
- [x] P9: GPUI story matrix `ruviform-ui-gpui/tests/lab_story.rs`（2 theme × 2 scale context × 7 state、`GpuiGuiTest`/`GpuiObserver`、draw plan 行列不変）+ story-keyed `golden_check` Unsupported link（`record_golden_step` evidence）。`test-ui-gpui` に 4 件追加
- [x] P10: `golden_capture.rs` を共通 e2e 化し story-keyed 実窓 capture 2 件追加（ignored・desktop）。step 失敗後の queued Tick が window procedure 内で panic する潜在 bug を `fatal` guard で修正。ローカル Windows desktop で 3 件 pass を実測
- [x] P11: stories.json の両 story の gpui binding を `pass` に更新（scale/pixel の境界を note に明記、mechanism = `test-ui-gpui` + filter）。`test-ui-driver-contract` pass

## 未完了（理由つき）

- [ ] window pixel の scale 1.0: gpui-0.2.2 test platform は `window.scale_factor` を 2.0 固定。semantic/draw 層の scale 軸は ScaleContext で網羅済みだが、pixel 実測の scale 1.0 は real-os/Visual lane（#313/#314）の残件。
- [ ] A3 の残り: story-keyed 実窓 capture は ignored test（desktop 必須・CI 対象外）。継続的 pixel gate の lane 設計は #314 側。
- [ ] UI-DVR-03 の GPUI 側は window deactivate による Cancel と seq 逆転のみ。実 OS focus・IME は #313/#389。
- [ ] Story の real-os lane（#313）は blocked のまま。

## 作業メモ

- ~~cancel は `CommandIntent` を発行しない~~ → 訂正: `CommandIntent::Cancel` は存在し gesture cancel は intent を発行する（第 2 段の story oracle が teardown の `cancel` intent を実測）。UI-DVR-03 側は committed 値の不変と drag 解放で確認している。
- gpui の `deactivate_window` はプラットフォームの active window が対象 window の時のみ有効。`window.activate_window()` を先に呼ぶ。
