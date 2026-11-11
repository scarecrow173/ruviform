---
id: "TASK-NhYDSYvlGjsQQzBFWEXYE7"
type: "task"
status: "done"
title: "Issue #251-T1: Stage W closeout — 実機 lane 証拠化と残存受入項目の検証・記録"
created: "2026-11-11"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-251-stage-w-real-device-closeout-plan.md"
  implemented-by:
    - "docs/impl/ir/issue-251-stage-w-real-device-closeout-slice.md"
  depends-on:
    - "docs/specs/issue-251-stage-w-real-device-closeout.md"
    - "docs/designs/issue-251-stage-w-real-device-closeout-design.md"
  blocks: []
  related:
    - "docs/test-specs/issue-251-stage-w-real-device-closeout.md"
---

# Issue #251-T1: Stage W closeout — 実機 lane 証拠化

## 作業項目

- [x] `tests/real_input_e2e.rs`: 実効 latency 集計（`callback - capture` min/max/sum + 順序違反計数）と summary 行を capture test に追加
- [x] `tests/real_input_e2e.rs`: WASAPI で `input_latency() == Ok(Some(_))`（`GetStreamLatency` 由来）を assert
- [x] `tests/real_input_e2e.rs`: `real_input_open_rejects_unsupported_configs` 新設（非 F32・範囲外 rate/layout・`Fixed(0)`・bounded 時の `Fixed` 範囲外 → すべて `Err(UnsupportedConfig)`）
- [x] `tests/real_input_e2e.rs`: duplex test に `same_clock_domain(domain, UNKNOWN)` / `same_clock_domain(UNKNOWN, UNKNOWN)` の rejection/fallback probe 表示を追加
- [x] `tests/real_input_e2e.rs`: `real_input_device_removal_notifies_typed_error` 新設（`RUVIFORM_REAL_INPUT_UNPLUG_WINDOW_SECS` env var 二重ゲート、`CONOUT$` 指示表示、`StreamInvalidated` 到達 assert、post-removal control-path 実測記録）
- [x] headless 検証: `mise run fmt-check` `check` `lint` `test` `test-rt` `build` `lint-audio-real-input`
- [x] 実機検証: `mise run test-audio-real-input` 実走 + 証拠記録（removal 検証は `IPolicyConfig::SetDefaultEndpoint` 既定 endpoint 切替 trigger で `StreamInvalidated` 実観測）
- [x] `docs/impl/ir/issue-251-stage-w-real-device-closeout-slice.md` 新規作成（Stage W matrix）
- [x] `docs/impl/ir/issue-251-audio-device-real-backend-input-plumbing-headless-slice.md` NotRun matrix 更新
- [x] `docs/*/README.md` 管理 index 再生成

## 完了条件

- 上記 headless lane が全件 Pass。 ✅
- 実機 lane の各検証行が Pass または理由つき `NotRun`/`NOT-EXERCISED` で記録される。 ✅（物理 USB 抜去経路のみ NotRun・代替 trigger で同一 typed error を実観測）
- Stage W（Windows+WASAPI）の結果が matrix/PR に「Stage M,L は NotRun — Cross OS 完了ではない」と明記される。 ✅

## 実施メモ

- 実走で callback timestamp provenance の欠陥（`IAudioClock::GetPosition` snapshot
  由来の epoch-0/~1ms stale instant → `capture > callback` violation）を発見し、
  `wasapi_input.rs` の callback instant を `QueryPerformanceCounter` 直読みへ修正。
  修正後 order violations=0・実効 latency ~10.04–10.12 ms。詳細は IR を参照。
