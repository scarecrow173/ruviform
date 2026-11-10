---
id: "IMPL-314rwFv2qD6sJ9tL4wX"
type: "impl"
status: "completed"
title: "Issue #314 slice 4 residual — real-window adapter IR"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "docs/specs/issue-314-real-window-adapter.md"
    - "docs/designs/issue-314-real-window-adapter.md"
    - "docs/plans/issue-314-real-window-adapter.md"
    - "docs/tasks/issue-314-real-window-adapter.md"
    - "docs/test-specs/issue-314-real-window-adapter.md"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-314-real-window-adapter.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/real_window.rs"
      - type: "file"
        path: "evidence/issue-314-real-window/driver-lab-evidence.jsonl"
      - type: "file"
        path: "evidence/issue-314-real-window/golden-evidence.jsonl"
      - type: "file"
        path: "evidence/issue-314-real-window/LAB-SV-FADER-DOCK-idle.png"
      - type: "file"
        path: "evidence/issue-314-real-window/LAB-SV-FADER-DOCK-teardown.png"
      - type: "file"
        path: "evidence/issue-314-real-window/LAB-AL-WAVEFORM-SEEK-idle.png"
      - type: "file"
        path: "evidence/issue-314-real-window/LAB-AL-WAVEFORM-SEEK-teardown.png"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/observe.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/scenario.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/stories.json"
---

# IR — Issue #314 slice 4 residual real-window adapter

## 実装したもの

- `real-scenario` feature: `gpui` + `dep:ruviform-ui-test`、
  `gpui/test-support` は含めない（test platform は OS capture を
  stub するため pixel 証拠を名乗れない）。`gpui-test` との併用は
  想定しない。
- `GpuiDriver` の `UiJob` seam: `Send` クロージャを
  `mpsc` 経由で `Inner.ui_jobs` に積み、`drain` が window ops 適用後に
  `App` context 内で実行（`drain_ui_jobs`）。GPUI 型は UI スレッドに
  固定。
- `gpui_driver/real_window.rs`: `RealOsGuiTest`（caller スレッド
  façade + UI スレッド job pump）、`RealWindowMount`/
  `RealWindowOptions`/`CaptureMeta`。`SemanticGuiTest` 全操作 +
  `VisualGuiTest::capture_frame` + `update_view`（teardown seam）+
  `last_capture_meta`。
- capability は実装どおり: `SemanticAction`+`MultiWindow` 常時、
  `VisualCapture` は Windows のみ。`RealOsPointerOrKeyboard` は
  claim しない。
- `tests/real_window.rs`: canonical story を実 window に mount、
  共有 oracle walk、idle/teardown の実 pixel capture、
  `golden_check`（`driver_class=real-os`, `capture=RealWindow`,
  実測 scale/backend）で baseline_missing → promote → pass。
  `RUVIFORM_REAL_WINDOW_PNG_DIR` env gate で PNG/meta を出力可能。
- `stories.json`: 両 story の `drivers.real-os` を `pass` に更新
  （実績 note + `cargo-test` mechanism）。

## 検証実績（Windows 11 desktop session）

- `cargo test -p ruviform-ui-gpui --features real-scenario --test
  real_window`: 2 passed（補助証拠。canonical lane は mise.toml が
  本 slice 範囲外のため未追加）。
- `evidence/issue-314-real-window/`: driver-lab/golden JSONL +
  idle/teardown PNG 4 枚（1280×800, `win32-printwindow`, sf1.0）。
  golden verdict: `blocked/baseline_missing` → `pass/match`
  （compared=1,024,000, mismatched=0）。

## 証拠 link

- **#262（採用判定）**: real-os 層の pixel 証拠は本 IR の
  `evidence/issue-314-real-window/` — `driver_class=real-os`・
  `capture=RealWindow`・実測 backend/scale を持つ GoldenKey の
  verdict JSONL と実測 PNG。採用判定はこの証拠列を参照する。
- **#313（OS Gate）**: 本 slice は実 window + 実 pixel capture のみ。
  OS-level input（SendInput 等）は #313 scope であり、本 adapter の
  `dispatch_input` は semantic dispatch で pointer/keyboard 証拠を
  名乗らない。`capabilities()` も `RealOsPointerOrKeyboard` を
  claim しない。#313 の capability matrix 検証は別 lane
  （`example-ui-gpui-real-os-smoke` 等）が担う。

## 残件（Issue #314 は open のまま）

- consumer gate・OS input（#313）・macOS/Linux capture backend・
  Plugin Dock snapshot（#388/SV#176）は別 scope。
- `real-scenario` の canonical mise lane 未整備（mise.toml は
  編集範囲外）。現状は補助 `cargo test` lane。
- real-os story matrix は declared theme × scale 1.0 のみ
  （scale 2.0 cell・第 2 theme は未実施 — note に明記）。
