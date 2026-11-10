---
id: "TEST-314rwEt8pC4wHxJ7nQ3rS"
type: "test-spec"
status: "approved"
title: "Issue #314 slice 4 residual — real-window adapter test-spec"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "docs/specs/issue-314-real-window-adapter.md"
    - "docs/designs/issue-314-real-window-adapter.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-314-real-window-adapter.md"
---

# test-spec — real-window adapter

## Feature: semantic walk on real OS window

- **Rule**: `RealOsGuiTest` は canonical `StoryFixture` を実 window に
  mount し、`lab_story_walk` の trace が
  `expected_lab_story_trace()` と一致しなければならない。
- **Example**: `lab_story_sv_fader_dock_real_window` /
  `lab_story_al_waveform_seek_real_window` — 共有 oracle の 7 state
  （idle/focus/drag/commit/stale/invalid/teardown）を `SemanticGuiTest`
  port 経由で検証。

## Feature: pixel capture honesty

- **Rule**: `capture_frame` は `CaptureOutcome::Captured` に実測
  RGBA pixel のみを返す。非 uniform 性は populated idle frame で
  検査し、teardown（fader story は空 panel）の uniform pixel を
  失敗扱いしない。
- **Rule**: semantic snapshot / draw plan / fixture 期待値を pixel
  として返さない。backend 不在は `Unsupported`、window 前提未達は
  `NotRun`。
- **Example**: teardown capture → `golden_check` が fresh store で
  `blocked`/`baseline_missing`（= pixels が compare に到達した証拠）
  → candidate → promote → 2 回目 `pass`/`match`。

## Feature: capability honesty

- **Rule**: `capabilities()` は `SemanticAction`/`MultiWindow` を
  常時、`VisualCapture` を `cfg!(windows)` のみ claim し、
  `RealOsPointerOrKeyboard`/`SimulatedPointerOrKeyboard`/`Ime`/
  `NativePluginDock` は claim しない（unit test で固定）。

## Feature: evidence separation

- **Rule**: story-keyed `GoldenKey` は `driver_class=real-os`、
  `capture=RealWindow`、`scale_factor`/`backend` は capture 実測値。
  `record_golden_step` で `golden:<canonical>` evidence が記録される。
- **Example**: `evidence/issue-314-real-window/golden-evidence.jsonl`
  に `blocked/baseline_missing` → `pass/match`（1280×800,
  win32-printwindow, sf1.0）の 4 行。
