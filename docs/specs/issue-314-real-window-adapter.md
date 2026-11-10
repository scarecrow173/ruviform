---
id: "SPEC-314rwAd4pX9kLmN2vQ8sT"
type: "spec"
status: "approved"
title: "Issue #314 slice 4 residual — real-window VisualGuiTest adapter + manifest/証拠 link"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/ruviform/issues/426"
    - "https://github.com/scarecrow173/ruviform/issues/464"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-314-real-window-adapter.md"
      - type: "file"
        path: "docs/designs/issue-314-real-window-adapter.md"
      - type: "file"
        path: "docs/plans/issue-314-real-window-adapter.md"
      - type: "file"
        path: "docs/tasks/issue-314-real-window-adapter.md"
      - type: "file"
        path: "docs/test-specs/issue-314-real-window-adapter.md"
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

# Issue #314 slice 4 residual — real-window VisualGuiTest adapter

## 背景

Issue #314 の残スライス。slice 1–3, 5, 6 で Pixel Golden Utility core・
Scenario Contract 接続・Win32 実 capture・evidence fold・story-keyed
capture が統合済みだが、`ScenarioDriver::RealOs` を正直に claim する
adapter が存在せず、`stories.json` の `drivers.real-os` は `blocked`
のままだった。

## 要件

1. `SemanticGuiTest` + `VisualGuiTest` を実装する real-window adapter
   を `ruviform-ui-gpui` に追加し、`ScenarioDriver::RealOs` を名乗る。
2. capability は実装結果どおりに宣言し、未対応は `Unsupported`/
   `NotRun` を返す。semantic/draw-plan/simulated 証拠を pixel や
   real-OS 証拠に昇格しない。
3. Driver Lab canonical fixture を実 window に mount し、共有 oracle
   の semantic walk と実 pixel capture → `golden_check` を接続する。
4. `stories.json` の `drivers.real-os` を実績どおりに更新する。
5. #262（採用判定）・#313（OS Gate）への証拠 link を impl record に
   記述する。

## 対象外

consumer 製品 fixture の実接続、OS-level input（#313）、
macOS/Linux capture backend、実 Plugin Dock snapshot（#388/SV#176）、
Issue #314 の Close（consumer gate 等は別 scope）。

## 受入条件

- real-window adapter が canonical story を実 OS window で walk し、
  共有 oracle trace に一致する。
- `capture_frame` が実測 RGBA pixel のみを `Captured` で返し、
  backend 未対応時は `Unsupported`/`NotRun` を返す。
- story-keyed `GoldenKey`（`driver_class=real-os`）が evidence に
  記録され、baseline 比較 verdict が pixel 実測由来である。
- `stories.json` の `real-os` status が実行実績と一致する。
