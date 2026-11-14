---
id: "TSPEC-cmKIBbYPL73xhK3osfi16F"
type: "test-spec"
status: "approved"
title: "Issue #630 VST3 test_gain first slice のテスト仕様"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
  changes:
    added:
      - type: "doc"
        path: "docs/test-specs/issue-630-test-plugin-suite.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/plans/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-test-plugin-suite.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-630-test-plugin-suite.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
  verified-by: []
  references:
    - "evidence/rf630/verification.md"
  defers: []
  deferred-by: []
---

# Issue #630 VST3 test_gain first slice のテスト仕様

## Purpose

`test_gain` の実 VST3 バイナリが、(a) plugin 側 ABI の直接検証（in-process COM round-trip）と (b) 本番 host bridge 経由の検証（`Vst3Library::load` → `ErasedPlugin`）の両方で、決定的な PCM/golden・sample-accurate automation・RT non-allocation の契約を満たすことを保証する。

## Feature

VST3 `test_gain` plugin（`crates/ruviform-plugin-test-vst3`）— vendor `"Ruviform Test"`、component `"test_gain"`、単一 gain パラメータ（plain `[0.0,2.0]`・default `1.0`・normalized `[0.0,1.0]` 線形写像）。

## Rules

- **R1 (ABI surface)**: `GetPluginFactory` + platform hooks（Win `InitDll`/`ExitDll`、Linux `ModuleEntry`/`ModuleExit`、macOS `bundleEntry`/`bundleExit`）を export し、factory は `"Audio Module Class"`/`"Component Controller Class"` の 2 class を登録する。component/controller の CID は adapter fixture と相異なる。
- **R2 (identity)**: `Vst3Library::descriptors()` が `"test_gain"`・vendor `"Ruviform Test"`・version = crate version の descriptor を返し、adapter fixture（`"Gain"`/`"Ruviform"`）と同一 scan で共存・区別できる。
- **R3 (param metadata)**: controller `getParameterCount` = 1、`ParameterInfo` が id `0`・title/shortTitle `"Gain"`・units `""`・`kCanAutomate`・defaultNormalizedValue `0.5`（plain default `1.0` → normalized `0.5`）。`normalizedParamToPlain`/`plainParamToNormalized` は `[0,2]` 線形写像。
- **R4 (golden PCM)**: 既知入力（例: 左 `0.5`・右 `-0.25` 定数列）に対し、default gain `1.0` で bitwise 同一出力、normalized `0.25`（plain `0.5`）適用で `in × 0.5` の出力になる。
- **R5 (sample-accurate automation)**: frame offset `k` の ParamValue イベントは frame `k` を最初の影響フレームとし、`[0,k)` は旧 gain・`[k,frames)` は新 gain で処理する。block 末尾適用・1 frame 早い/遅い適用は Fail。offset `0` はブロック全体に即時適用。同一 offset の連続イベントは last-wins。ゼロフレームブロックの offset-0 イベントは gain を次ブロックへ持続させる。
- **R6 (lifecycle)**: `initialize`→`setActive(true)`→`setupProcessing`→`setProcessing(true)`→`process`→`setProcessing(false)`→`setActive(false)`→`terminate` の順序で動作し、`Processing` 外の `process`・prepare 前の `setProcessing(true)` 等のプロトコル違反は `kResultFalse` で defined reject。
- **R7 (RT)**: `process` 経路（`prepare_block`→view→`TestGainProcessor::process`→`finish_block`）は allocation しない — `assert_no_alloc` の `rt_` テストで機械検証。
- **R8 (boundary)**: `kSample64`・負/超過 `numSamples`・null/未 align channel pointer・部分 overlap・宣言外 channel 数は defined reject（`kResultFalse`）。in-place alias（`out==in`）は bounce 経由で正しく処理される。
- **R9 (state honesty)**: component `setState`/`getState`・controller `setComponentState` は `kNotImplemented`（State は後続 class）。`createView` は null（Editor は後続 class）。
- **R10 (host path)**: 本番 host bridge が生成する `ProcessData`/`IParameterChanges`/bus 構造を plugin が正しく処理し、`process_host` 経由でも R4/R5 と同一の golden が得られる。

## Examples

- 64 frame block・入力 `[0.5, -0.25]`・offset `32` に normalized `0.25`（plain `0.5`）イベント → frames `[0,32)` は `×1.0`、`[32,64)` は `×0.5`（off-by-one の明示固定）。
- ゼロフレームブロックに offset `0` の gain イベント → 次の非ゼロブロック全体に新 gain が適用される。
- in-place（`out_ptr == in_ptr`）の 2ch block → bounce writeback 後に `in × gain` が host buffer に現れる。

## Guarantees

- 実 `.dll`/`.vst3` ファイルが `Vst3Library::load` で dlopen され、production `ErasedPlugin` 経路のみで処理される（Fake Plugin Port 非経由）。
- `rt_` テスト名が `cargo test --workspace rt_` の name filter に収集される。
- plugin 固有実装（DSP・lifecycle・bus・param 写像）は adapter fixture と独立。共有は `vst3` binding 層（SDK 相当の共通層）と neutral 契約のみ。

## Non-goals

- CLAP・L1 out-of-process・Native Editor・State Save/Reload・MIDI/Tempo/Loop・Crash/Hang/Timeout/Isolation・外部 SDK validator・consumer oracle — すべて後続 slice（NotRun）。
- `test_gain` 以外の plugin class、`ruviform-audio-test` への依存。

## Risk

- cdylib の `deps/` 解決が環境依存 → 先例踏襲 + 実測確認、失敗時は evidence に NotRun。
- in-process COM round-trip と host path が「同じ実装の二経路」に見える → host path は production `Vst3Library`/`ErasedPlugin` が `ProcessData` を生成する点で独立（plugin 側は同一バイナリを dlopen）。外部 SDK validator 不在の限界は evidence/impl-doc に明記。
