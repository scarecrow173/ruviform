---
id: "SPEC-034N9j2YKqyKxZZZyUPKFe"
type: "spec"
status: "implemented"
title: "Issue #130 media-core/dsp/audio-analysis assert_no_alloc レーン武装（AllocDisabler 宣言）"
created: "2026-10-27"
updated: "2026-10-27"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/130"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-media-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-dsp/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-analysis/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../plans/implement-issue-130-arm-assert-no-alloc.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/rt-verification-lane.md"
    - "../adr/rt-assert-no-alloc-thread-check.md"
    - "../test-specs/rt-verification-contracts.md"
    - "../specs/video-proc-rt-verification-lane.md"
    - "../impl/ir/issue-126-video-proc-rt-lane-implementation.md"
  refines:
    - "../test-specs/rt-verification-contracts.md"
  refined-by: []
  derives-from:
    - "../specs/rt-verification-lane.md"
    - "../adr/rt-assert-no-alloc-thread-check.md"
  derived-by:
    - "../plans/implement-issue-130-arm-assert-no-alloc.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/126"
    - "https://github.com/scarecrow173/ruviform/issues/22"
  defers: []
  deferred-by: []
---

# Issue #130 media-core/dsp/audio-analysis assert_no_alloc レーン武装（AllocDisabler 宣言）

## What Should Be Built

`assert_no_alloc` の測定窓は `#[global_allocator] AllocDisabler` の宣言があって初めて発火する。`ruviform-media-core`・`ruviform-dsp`・`ruviform-audio-analysis` の 3 crate は `assert_no_alloc(|| …)` を呼ぶ `rt_` テストと dev-dependency を既に持つが `AllocDisabler` 宣言がなく、窓は常に成功して何も計測していない（Issue #126 レビューで発見・pre-existing）。各 crate の lib test binary へ `#[cfg(debug_assertions)] #[global_allocator]` な `AllocDisabler` を 1 個ずつ宣言し、既存の測定窓を実際に武装する。

対象の既存測定窓（窓の内外分割は既に正しい — 本 Issue は武装のみで窓の再構成は行わない）:

- `ruviform-media-core` — `src/music.rs` の `music::tests::rt_conversion_no_alloc`（`TempoMap` query 群の窓）
- `ruviform-dsp` — `src/lib.rs` の `tests::rt_process_paths_do_not_allocate`（biquad/FIR/FFT/resampler/window の `process` 経路の窓）
- `ruviform-audio-analysis` — `src/level.rs` と `src/spectrum.rs` の各 `rt_process_allocates_nothing`（`LevelMeter::process`・`SpectrumAnalyzer::process` の窓）

## Why Now

Issue #126（video-proc RT 検証レーン）のレビューで、「`assert_no_alloc` を呼ぶが `AllocDisabler` を宣言していない」crate が 3 つ存在することが判明した。`AllocDisabler` なしの `assert_no_alloc` 窓は検出機構を持たないため常に通過し、「alloc-free を機械検証した」という記録が実際には何も検証していない状態だった。#126 IR の Follow-ups に「follow-up issue として追跡する」と記録された残件の履行であり、宣言 1 個の追加で既存窓がそのまま実効化する。

## Users and Value

- Ruviform 利用者: `TempoMap` query・DSP `process`・アナライザ `process` の RT 契約（allocation 0）が CI で実際に検出される
- コントリビュータ/レビュア: 「窓があるのに武装していない」サイレントな検証欠損を解消し、rt_ レーンの報告が実測に裏付けられる

## Scope

### In Scope

- 3 crate の lib test binary への `#[cfg(debug_assertions)] #[global_allocator] static ALLOC: assert_no_alloc::AllocDisabler` 宣言（test binary ごとに 1 個 — video-proc #126 と同形式・`mod tests` 内配置）
- `ruviform-audio-analysis` は crate-root `mod tests` が存在しないため、宣言のみを持つ `#[cfg(test)] mod tests` を `lib.rs` に新設する（plugin-clap/vst3 と同じく「1 モジュールの宣言が crate test binary 全体を武装する」方式 — video-proc の crate-root 配置に準拠）
- Sanity abort 検証: 各 crate の測定窓内へ一時的に意図的な alloc を挿入して test binary の abort（exit `0xc0000409` 等）を確認後、除去して全 pass を確認 — 武装の実効性を機械実証する
- 武装によって既存窓内の真の allocation が顕在化した場合の正直な記録（hoist が trivially correct なら修正、そうでなければ IR の Follow-ups へ記録）
- spec/plan/task/IR の作成と README index 更新・#126 IR Follow-ups 行の解決注記

### Out of Scope

- 既存測定窓の内外分割の再構成 — 窓は pre-existing で既に正しい（fixture/prepare は窓外・`process`/query は窓内）。真の alloc が検出された場合のみ、所有レイヤーでの最小修正または正確な記録を追加判断する
- 3 crate 以外への lane 拡大・`rt_` テストの新規追加
- `ruviform-rt` の `alloc_counter` 変更・`assert_no_alloc` バージョン/dev-dep 指定形の変更（`assert_no_alloc = "1.1"` は 3 crate とも宣言済み）
- `mise run test-rt` タスク・CI `rt-verify` ジョブの変更（既存 lane が `rt_` 名で選択する）
- integration test binary（`tests/*.rs`）の新設 — 3 crate とも存在せず、lib test binary のみが対象

## Requirements

- `#[global_allocator]` は lib test binary ごとに 1 個のみ宣言すること（複数宣言はコンパイルエラー）。各 crate の全 `mod tests` が 1 つの lib test binary にコンパイルされるため、宣言位置は crate-root の `mod tests`（video-proc #126 pattern）とし、他モジュールの `rt_` テストは同一アロケータを共有する
- 宣言には `#[cfg(debug_assertions)]` を付けること（release ビルドでは `assert_no_alloc` が no-op — 既存宣言と同条件）
- 各 crate で「意図的 alloc → abort → 除去 → pass」の sanity 検証を実施し、武装の実効性を記録すること
- `mise run test-rt` が対象 crate の `rt_` テストを選択・全件パスすることを実測で示すこと
- 武装で真の allocation が検出された場合、検出内容を正直に記録し、修正したか Follow-up へ記録したかを区別すること

## Acceptance Criteria

- [x] `ruviform-media-core`/`ruviform-dsp`/`ruviform-audio-analysis` の各 lib test binary に `AllocDisabler` が 1 個ずつ宣言されている
- [x] 3 crate 全てで sanity abort 検証（意図的 alloc → test binary abort）を実証し、除去後に全 rt_ テストが pass
- [x] `mise run test-rt` で対象 `rt_` テストが選択・全件パス（media-core 5 件・dsp 1 件・audio-analysis 3 件 — 非 RT の substring 偶然一致を含む場合は件数を区別して報告）
- [x] 武装による真の allocation 検出はゼロ、または検出内容と対処（修正/Follow-up 記録）が文書化されている
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全パス
- [x] #126 IR の Follow-ups 行へ Issue #130 の解決注記が記録されている

## Deferred Design Concerns

- なし — 本 Issue は #126 IR Follow-ups の履行であり、新たな繰延は生じない（武装で真の alloc が検出された場合のみ個別記録）。

## Sources and Relations

- Issue [#130](https://github.com/scarecrow173/ruviform/issues/130): 3 crate の `assert_no_alloc` 窓が `AllocDisabler` 未宣言で無検証だった問題の武装
- 発見元・先行適用例: [issue-126-video-proc-rt-lane-implementation.md](../impl/ir/issue-126-video-proc-rt-lane-implementation.md)（#126 レビューで検出・Follow-ups に追跡対象として記録）・[video-proc-rt-verification-lane.md](video-proc-rt-verification-lane.md)（宣言形式の正本 — `crates/ruviform-video-proc/src/lib.rs` の `mod tests` 内宣言）
- Spec/ADR: [rt-verification-lane.md](rt-verification-lane.md)（Issue #22 レーンの親 spec）・[rt-assert-no-alloc-thread-check.md](../adr/rt-assert-no-alloc-thread-check.md)（`assert_no_alloc` 標準化の確定決定）
- 契約: [rt-verification-contracts.md](../test-specs/rt-verification-contracts.md) Rule 1–2（allocation-free の測定窓・warm-up 外の確保は許容）— 本 spec が同 test-spec を 3 crate へ refines 適用する
- 宣言方式の先行例: `crates/ruviform-plugin-clap/src/plugin.rs`・`crates/ruviform-plugin-vst3/src/block.rs`（1 モジュールの宣言が crate test binary 全体を武装）
- Refs: Issue [#126](https://github.com/scarecrow173/ruviform/issues/126)・[#22](https://github.com/scarecrow173/ruviform/issues/22)
