---
id: "TASK-034N9j4dL22V16U4g7py5Z"
type: "task"
status: "done"
title: "media-core/dsp/audio-analysis lib test binary への AllocDisabler 宣言"
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
      - type: "file"
        path: "docs/impl/ir/issue-126-video-proc-rt-lane-implementation.md"
      - type: "file"
        path: "docs/adr/rt-assert-no-alloc-thread-check.md"
      - type: "file"
        path: "docs/specs/rt-verification-lane.md"
      - type: "file"
        path: "docs/designs/rt-verification-lane-design.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-130-arm-assert-no-alloc.md"
  implemented-by:
    - "docs/impl/ir/issue-130-arm-assert-no-alloc-lane-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-130-arm-assert-no-alloc.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/video-proc-assert-no-alloc-lane.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-130-arm-assert-no-alloc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rt-verification-contracts.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/126"
  defers: []
  deferred-by: []
---

# media-core/dsp/audio-analysis lib test binary への AllocDisabler 宣言

## Work

1. `crates/ruviform-media-core/src/lib.rs` — crate-root `mod tests` に `#[cfg(debug_assertions)] #[global_allocator] static ALLOC: assert_no_alloc::AllocDisabler` を宣言（crate lib test binary に 1 個 — `music::tests::rt_conversion_no_alloc` を含む全モジュールの `rt_` テストが同一アロケータを共有。integration test binary は存在しない）
2. `crates/ruviform-dsp/src/lib.rs` — `rt_process_paths_do_not_allocate` を含む crate-root `mod tests` に同宣言（crate 内の `mod tests` は lib.rs のみ）
3. `crates/ruviform-audio-analysis/src/lib.rs` — crate-root `mod tests` が存在しないため、宣言のみを持つ `#[cfg(test)] mod tests` を新設（`level::tests`・`spectrum::tests` の各 `rt_process_allocates_nothing` と `peaks::tests` が同一 lib test binary にコンパイルされる — 宣言は 1 個のみ）
4. Sanity abort 検証: 各 crate の測定窓内に一時的な `vec![0_u8; 8]` を挿入 → `mise exec -- cargo test -p <crate> rt_` で test binary abort を確認 → 除去 → 再実行で全 pass を確認（3 crate 分の記録を IR へ）
5. `docs/impl/ir/issue-126-video-proc-rt-lane-implementation.md` の Follow-ups 行へ Issue #130 解決注記
6. IR 作成・README index 行確認・doc audit clean

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-130-arm-assert-no-alloc.md](../plans/implement-issue-130-arm-assert-no-alloc.md)
- [../specs/issue-130-arm-assert-no-alloc-lane.md](../specs/issue-130-arm-assert-no-alloc-lane.md)
- [../adr/rt-assert-no-alloc-thread-check.md](../adr/rt-assert-no-alloc-thread-check.md)
- [../test-specs/rt-verification-contracts.md](../test-specs/rt-verification-contracts.md)（Rules 1–2）
- [video-proc-assert-no-alloc-lane.md](video-proc-assert-no-alloc-lane.md)（#126 同一作業の完了記録 — `mod tests` 内宣言規約・sanity abort 検証手順）
- [apply-assert-no-alloc-to-plugin-boundary.md](apply-assert-no-alloc-to-plugin-boundary.md)（#22 同一作業の完了記録）

## Dependencies

- 先行タスク: なし（`assert_no_alloc = "1.1"` は 3 crate とも dev-dep 宣言済み・`Cargo.lock` 解決済み 1.1.2 — lockfile 変更不要）

## Affected Files

- `crates/ruviform-media-core/src/lib.rs`（`mod tests` 内へ `AllocDisabler` 宣言 — `music.rs` 他 6 モジュールの `mod tests` と同一 lib test binary）
- `crates/ruviform-dsp/src/lib.rs`（`mod tests` 内へ `AllocDisabler` 宣言）
- `crates/ruviform-audio-analysis/src/lib.rs`（宣言のみを持つ `#[cfg(test)] mod tests` を新設 — `level.rs`/`peaks.rs`/`spectrum.rs` の `mod tests` と同一 lib test binary）
- `docs/impl/ir/issue-126-video-proc-rt-lane-implementation.md`（Follow-ups 解決注記）

## Done When

- [x] 3 crate の lib test binary に `AllocDisabler` が 1 個ずつ・`#[cfg(debug_assertions)]` 付きで宣言されている（重複宣言なし — 全 `mod tests` の grep 確認済み）
- [x] 3 crate 全てで sanity abort 検証（窓内の意図的 `vec![0_u8; 8]` → `memory allocation of 8 bytes failed`・exit `0xc0000409`）を実証し、除去後に全 rt_ テストが pass
- [x] 武装による真の allocation 検出はゼロ — または検出内容と対処が IR に記録されている
- [x] #126 IR の Follow-ups 行へ解決注記が記録されている

## Verification

- [x] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` 全パス（`test-rt` の対象 crate マッチ件数: media-core lib test binary で 5 件・dsp で 1 件・audio-analysis で 3 件 — うち `spectrum::tests::short_input_zero_pads_and_long_input_truncates` は substring `rt_` 偶然一致の非 RT test）
- [x] doc audit `blocking: 0`・`findings: []`

## Completion Notes

- **Allocation 検出なし** — 武装後の全対象 `rt_` テストが `assert_no_alloc` 窓内で `process`/query を実行して全件 pass。`TempoMap` query（binary search + integer math）・DSP `process` 経路（biquad/FIR/FFT/resampler/`Window::fill`）・`LevelMeter::process`・`SpectrumAnalyzer::process`（内部で `Fft::process` を呼ぶ経路を含む）のいずれでも Rust alloc は観測されず、実装修正は不要だった。
- **Lane の武装確認** — 各 crate の測定窓内へ一時的に `vec![0_u8; 8]` を挿入し、3 crate とも test binary が `memory allocation of 8 bytes failed` → abort（exit `0xc0000409`）することを実証してから除去。`AllocDisabler` が各 lib test binary の global allocator として機能していることを確認。
- **宣言配置** — video-proc #126 pattern に準拠し crate-root `mod tests` へ配置。audio-analysis は `mod tests` を持たなかったため宣言のみの `#[cfg(test)] mod tests` を `lib.rs` に新設（`level`/`spectrum`/`peaks` の `mod tests` と同一 lib test binary — 宣言は crate 内で 1 個のみ）。
- IR: [issue-130-arm-assert-no-alloc-lane-implementation.md](../impl/ir/issue-130-arm-assert-no-alloc-lane-implementation.md)
