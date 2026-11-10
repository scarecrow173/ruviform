---
id: "TSPEC-034VgdafmAWO2nuE3QoYTj"
type: "test-spec"
status: "approved"
title: "Issue #314 slice 4 follow-up — real-scenario lane + matrix test-spec"
created: "2026-11-10"
updated: "2026-11-11"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-314-real-scenario-lane.md"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-314-real-scenario-lane.md"
    - "docs/designs/issue-314-real-scenario-lane.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 4 follow-up — real-scenario lane + matrix test-spec

## Purpose

canonical `test-ui-real-scenario` lane と real-os story matrix の
残 cell 実走が、環境不通と GUI Fail を混同せず、matrix 全 cell の
semantic/pixel 証拠を正直に記録することを保証する。manual lane の
性質上 CI では検証されないため、lane 定義と evidence 出力の
契約をここに固定する。

## Feature: canonical real-scenario lane

- **Rule**: `mise.toml` に `test-ui-real-scenario` task が存在し、
  `cargo test -p ruviform-ui-gpui --features real-scenario --test
  real_window` を実行する。既存 task を変更・削除しない。
- **Rule**: lane は対話 desktop session 前提の manual lane であり、
  CI 必須 lane に含めない。env（`RUVIFORM_DRIVER_LAB_EVIDENCE`/
  `RUVIFORM_GOLDEN_EVIDENCE`/`RUVIFORM_REAL_WINDOW_PNG_DIR`）で
  `evidence/issue-314-real-scenario/` への証拠出力先を固定する。
- **Example**: `mise run test-ui-real-scenario` → 2 test
  （`lab_story_sv_fader_dock_real_window` /
  `lab_story_al_waveform_seek_real_window`）が各 4 cell を実行し、
  driver-lab/golden JSONL と PNG/meta が evidence dir に出力される。

## Feature: story matrix cells

- **Rule**: 各 story は manifest `themes`（synvelis-dark /
  audiolens-dark）× `scaleFactor`（1.0 / 2.0）の 4 cell を実 window
  session で実行し、cell ごとに共有 oracle trace・idle 非 uniform
  pixel・teardown capture・`golden_check`（baseline_missing →
  promote → pass）を検証する。
- **Rule**: `GoldenKey.theme` は cell の実 mount theme、
  `scale_factor`/`backend` は capture 実測値。theme scale は
  `ScaleContext` hint であり token 不変を cell ごとに assert する
  （Fake/GPUI-simulated 層と同一規則）。
- **Example**: `LAB-SV-FADER-DOCK` が synvelis-dark/audiolens-dark ×
  scale 1.0/2.0 の 4 cell で `expected_lab_story_trace()` に一致
  する。

## Feature: environment_blocked の区分

- **Rule**: `RealOsGuiTest::run` が `UiError::Host` または
  `UiError::BackendUnavailable` で失敗した場合、session 基盤の
  環境要因として `driver_lab::emit_evidence` に
  `observed="environment_blocked"` を記録し、
  `environment_blocked:` prefix 付きで panic する。test は fail の
  ままとし pass に見せない。
- **Rule**: `UiError::Internal`/`Unsupported`・body 内
  `ScenarioError`・assert 失敗は GUI/test レベルの失敗として
  environment_blocked を名乗らない。
- **Example**: headless/session-0 環境で lane を実行 → window 生成
  失敗 → JSONL に `observed="environment_blocked"` + panic
  `environment_blocked: ...`（GUI Fail とは別区分）。

## Guarantees

- matrix cell の semantic trace は共有 oracle と一致する（real-os
  層での theme/scale 不変性の証明）。
- pixel 層は cell ごとに実測 RGBA のみを扱い、semantic/draw-plan
  証拠を pixel に昇格しない（slice 4 と同一契約）。
- `stories.json` `drivers.real-os` の status/note/mechanism は
  実行実績と一致し、`verify_story_mechanisms` が
  `[tasks.test-ui-real-scenario]` の実在を検査できる。

## Non-goals

- OS-level input（SendInput）の検証 — #313 scope。
- macOS/Linux capture backend、consumer 製品 fixture、Plugin Dock
  snapshot。
- `cargo test` target への exit-code 区分（0/1/2/3）の導入 —
  verdict JSON runner 化は将来 scope。
- pixel の cell 間一致 assert — scale は mapper-side hint であり
  pixel 不変は oracle 要件ではない（golden loop が cell ごとに
  独立検証する）。

## Risk

- environment_blocked の境界（`Host`/`BackendUnavailable` のみ環境
  扱い）を誤ると GUI 障害を環境に見せる/その逆になる —
  `Internal`/`Unsupported`/`ScenarioError` は必ず fail 側に残す。
- matrix 化で 1 cell の失敗が story 全体を fail にする — cell ごとの
  panic は test 関数単位の失敗として正直に報告される。
