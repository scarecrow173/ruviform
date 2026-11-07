---
id: "TASK-ovbF_E6zV8WeEi0OsUIw-w"
type: "task"
status: "in-progress"
title: "Issue #426 GPUI lab binding・Story matrix・Golden 境界・Consumer 検査の作業記録"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-426-gpui-lab-binding.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lab_binding.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_lab.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/component_view.rs"
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

## 未完了（理由つき）

- [ ] A2 GPUI layer の story matrix: gpui test platform は scale 2.0 固定。scale 1.0 を再現できる経路が無いため not_run。
- [ ] A3 Pixel Golden: #314（RF-GT-05）は OPEN で Golden Utility が未実装。NotRun。
- [ ] UI-DVR-03 の GPUI 側は window deactivate による Cancel と seq 逆転のみ。実 OS focus・IME は #313/#389。
- [ ] Story の real-os lane（#313）は blocked のまま。

## 作業メモ

- cancel は `CommandIntent` を発行しない設計のため、intent 列での検証は不可。committed 値の不変と drag 解放で確認する。
- gpui の `deactivate_window` はプラットフォームの active window が対象 window の時のみ有効。`window.activate_window()` を先に呼ぶ。
