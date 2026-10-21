---
id: "PLAN-034JOq2pk8YxpxyUXNJGFN"
type: "plan"
status: "approved"
title: "Issue #59 chunk 分割時の per-chunk timestamp 合成の実施"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/59"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-audio-device/src/convert.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/device.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/chunk-timestamp-synthesis.md"
  implemented-by:
    - "../tasks/add-sample-rate-to-callback-state.md"
    - "../tasks/synthesize-per-chunk-timestamps.md"
    - "../tasks/add-chunk-timestamp-tests.md"
  depends-on:
    - "../adr/chunk-outputcallbackinfo-timestamp.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/chunk-timestamp-synthesis.md"
    - "../adr/chunk-outputcallbackinfo-timestamp.md"
    - "../designs/chunk-timestamp-design.md"
    - "../designs/audio-device-boundary-and-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/chunk-timestamp-contracts.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/25"
  defers: []
  deferred-by: []
---

# Issue #59 chunk 分割時の per-chunk timestamp 合成の実施 Implementation Plan

## Goal

`write_interleaved` の chunk 分割で各 chunk に `f_k / sample_rate` ナノ秒のオフセットを `callback`/`playback` instant に合成し、Issue #25 `MasterClock` 配線前提の timestamp 契約を確定する。

## Scope

単一実装ストリーム — `DataCallbackState` への `sample_rate` 追加と `write_interleaved` 内の per-chunk 合成・テスト追加。契約の明文化（合成値は推定）は doc コメントに記録する。

## Upstream Documents

- [../specs/chunk-timestamp-synthesis.md](../specs/chunk-timestamp-synthesis.md) — `SPEC-034JOoOaExdxXgKrQ80l7x`（approved）
- [../adr/chunk-outputcallbackinfo-timestamp.md](../adr/chunk-outputcallbackinfo-timestamp.md) — `ADR-034JOpkx5tdDPVGzdlb0bb`（accepted）

## Design Inputs

- [../designs/chunk-timestamp-design.md](../designs/chunk-timestamp-design.md) — `DESIGN-034JOoWdEHXnb8G1QyMzCm`（approved）

## File Map

- `crates/ruviform-audio-device/src/stream.rs` — `DataCallbackState` に `sample_rate: u32` 追加・`handle` で `write_interleaved` に渡す
- `crates/ruviform-audio-device/src/convert.rs` — `write_interleaved` に `sample_rate` 引数追加・chunk ループで per-chunk `OutputCallbackInfo` 合成・フォールバック・テスト
- `crates/ruviform-audio-device/src/device.rs` — `DataCallbackState::new` 呼び出しに `negotiated.stream.sample_rate.0` を渡す

## Implementation Sequence

- [x] `DataCallbackState` に `sample_rate: u32` を追加・`new` シグネチャに追加し、`device.rs` で `negotiated.stream.sample_rate` の `u32` を渡す
- [x] `write_interleaved` に `sample_rate: u32` 引数を追加し、chunk ループで `OutputCallbackInfo` を per-chunk 合成（`frame * 1e9 / sample_rate` ns を `MediaDuration`→`checked_add`）
- [x] `checked_add` 失敗・`playback: None`・単一 chunk の各経路のフォールバックを doc コメントとテストで担保
- [x] per-chunk timestamp の単体テストを追加（オフセット値・フォールバック・`None`・単一 chunk 無変化）
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`・`prek` 全パス

## Dependencies and Risks

- `negotiated.stream.sample_rate` は cpal `SampleRate(u32)` — `.0` で `u32` 取得。open 時に確定済みのため非 RT 経路のみで保持
- `frame * 1e9` の乗算は `u128` 演算 + `i64::try_from` 飽和で安全に（設計参照）
- `checked_add` 失敗時は元 instant フォールバック（timestamp を失わないことを優先）

## Task Breakdown

- `add-sample-rate-to-callback-state` — `DataCallbackState`/`device.rs` の `sample_rate` 配線
- `synthesize-per-chunk-timestamps` — `write_interleaved` の per-chunk 合成とフォールバック
- `add-chunk-timestamp-tests` — 単体テスト追加・doc 更新・Issue 完了確認

## Verification

- [x] chunk 分割時の k 番目 instant が base + `f_k * 1e9 / sample_rate` ns（単体テスト）
- [x] `playback: None`・`checked_add` 失敗・単一 chunk 無変化のフォールバックテスト
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`・`prek run --all-files` 全パス
- [x] 監査で本変更起因 findings 0

## Open Questions / Gaps

- backend の実測 timestamp が合成値とどれだけずれるかは実機依存 — 契約 doc に推定値である旨を明記し、Issue #25 で実機検証する

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（局所的な3ファイル変更）
- Independent work streams: none
- User approval required before dispatch: yes
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 合成ロジックの境界（`u128` 乗算 → `i64` 飽和 → `checked_add`）とフォールバック経路を重点確認
