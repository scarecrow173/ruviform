---
id: "IMPL-034N9jmYNeY4BGVmBIAOIT"
type: "impl"
status: "completed"
title: "Issue #130 arm assert_no_alloc lane — implementation"
created: "2026-10-27"
updated: "2026-10-27"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/130"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-130-arm-assert-no-alloc-lane-implementation.md"
      - type: "file"
        path: "docs/specs/issue-130-arm-assert-no-alloc-lane.md"
      - type: "file"
        path: "docs/plans/implement-issue-130-arm-assert-no-alloc.md"
      - type: "file"
        path: "docs/tasks/arm-assert-no-alloc-lane.md"
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
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/arm-assert-no-alloc-lane.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-130-arm-assert-no-alloc.md"
    - "docs/specs/issue-130-arm-assert-no-alloc-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/rt-assert-no-alloc-thread-check.md"
    - "docs/test-specs/rt-verification-contracts.md"
    - "docs/designs/rt-verification-lane-design.md"
    - "docs/impl/ir/issue-126-video-proc-rt-lane-implementation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-130-arm-assert-no-alloc.md"
    - "docs/specs/issue-130-arm-assert-no-alloc-lane.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rt-verification-contracts.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/126"
    - "https://github.com/scarecrow173/ruviform/issues/22"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #130 arm assert_no_alloc lane — implementation

## Summary

`ruviform-media-core`・`ruviform-dsp`・`ruviform-audio-analysis` の 3 crate は `assert_no_alloc(|| …)` を呼ぶ `rt_` テストと dev-dependency を持ちながら `#[global_allocator] AllocDisabler` を宣言しておらず、測定窓が常に成功して何も計測していなかった（Issue #126 レビューで発見・pre-existing）。各 crate の lib test binary へ `#[cfg(debug_assertions)] #[global_allocator]` な `AllocDisabler` を 1 個ずつ宣言して既存窓を武装した — 宣言追加のみで窓・実装コードの変更はなし。3 crate 全てで「窓内への意図的 `vec![0_u8; 8]` 挿入 → test binary abort（`memory allocation of 8 bytes failed`・exit `0xc0000409`）→ 除去 → 全 pass」の sanity abort 検証を実施し、武装の実効性を機械実証した。武装による真の allocation 検出はゼロ — 全 `rt_` テストが現行コードで pass し、「alloc-free by construction」だった静的論証が実測で裏付けられた。#126 IR の Follow-ups 行は本 Issue で解決済み注記へ更新。

## Context

- Upstream: spec `docs/specs/issue-130-arm-assert-no-alloc-lane.md`（Issue #130・`status: implemented`）→ plan `docs/plans/implement-issue-130-arm-assert-no-alloc.md` → task `docs/tasks/arm-assert-no-alloc-lane.md`（単一タスク）。
- 経緯: Issue #126（video-proc RT 検証レーン）のレビューで「`assert_no_alloc` を呼ぶが `AllocDisabler` を宣言していない crate が 3 つある」ことが判明し、#126 IR の Follow-ups に「follow-up issue として追跡する」と記録された残件。`AllocDisabler` なしの `assert_no_alloc` 窓は検出機構を持たず常に通過するため、従来の rt_ テスト報告は alloc-free の機械検証を意味していなかった。
- 規約: ADR `rt-assert-no-alloc-thread-check`（新規 lane は `assert_no_alloc`・`ruviform-rt::alloc_counter` は併存維持）・test-spec `rt-verification-contracts` Rule 1–2（fixture/warm-up の確保は窓外・窓内 alloc は test binary abort で検出）・宣言形式の正本は video-proc #126（`crates/ruviform-video-proc/src/lib.rs` の `mod tests` 内宣言）。

## Implementation

### Changes

- `crates/ruviform-media-core/src/lib.rs` — crate-root `mod tests` に `#[cfg(debug_assertions)] #[global_allocator] static ALLOC: assert_no_alloc::AllocDisabler` を宣言（宣言位置: `mod tests` 先頭・`use super::*;` 直後）。`music.rs` 他 6 モジュールの `mod tests`（error/stream/time/timecode/transport/music）と同一 lib test binary にコンパイルされるため宣言は 1 個のみ — `music::tests::rt_conversion_no_alloc` の窓がこれで武装する。crate は `tests/*.rs` integration binary を持たない。
- `crates/ruviform-dsp/src/lib.rs` — `rt_process_paths_do_not_allocate` を含む crate-root `mod tests` に同宣言（`use` 群の直後・`const SR` の前）。crate 内の `mod tests` は lib.rs のみ。
- `crates/ruviform-audio-analysis/src/lib.rs` — crate-root `mod tests` が存在しなかったため、宣言のみを持つ `#[cfg(test)] mod tests` を新設（`pub use` 群の後・ファイル末尾）。`level::tests`・`spectrum::tests` の各 `rt_process_allocates_nothing` と `peaks::tests` が同一 lib test binary にコンパイルされる — plugin-clap/vst3 と同じく「1 モジュールの宣言が crate test binary 全体を武装する」方式。
- `docs/impl/ir/issue-126-video-proc-rt-lane-implementation.md` — Follow-ups の「他 3 crate の lane が未武装」行へ Issue #130 解決済み注記を追記。

### Allocation findings

- **検出された allocation: なし**。武装後の全対象 `rt_` テストが `assert_no_alloc` 窓内で `process`/query を実行して全件 pass — `TempoMap::tick_to_media`/`media_to_tick`/`musical_position`/`time_info`（binary search + integer math）、`BiquadFilter::process`・`FirFilter::process`・`Fft::process`・`LinearResampler::process`・`Window::fill`、`LevelMeter::process`、`SpectrumAnalyzer::process`（内部で `Fft::process` を呼ぶ経路を含む）のいずれでも Rust global allocator 経由の alloc は観測されなかった。実装コードの修正は不要だった。
- **Lane が実際に武装していることの検証**: 3 crate 各々で測定窓内へ一時的に `let _sanity = vec![0_u8; 8];` を挿入して `cargo test -p <crate> rt_` を実行 → 3 crate とも `memory allocation of 8 bytes failed` で test binary が abort（exit `0xc0000409`）することを確認後、当該行は全て除去した。`AllocDisabler` が各 lib test binary の global allocator として機能していることを実証。
- **既存窓の内外分割の確認**: 全対象テストで fixture 確保・ctor/prepare（`TempoMapBuilder::build`・`BiquadFilter::prepare`・`Fft::new`・`SpectrumAnalyzer::new`・`LevelMeter::new`）と assert 評価が窓外、`process`/query が窓内に配置されていることを確認 — 窓の再構成は不要だった（scope どおり武装のみ）。

## Related Experiments

### Adopted

- **sanity abort 検証（窓内への一時的な意図的 alloc）** — `AllocDisabler` 宣言後、各 crate の測定窓内へ `vec![0_u8; 8]` を一時挿入して abort を実証してから除去。#126 と同一手順で、lane が「armed かつ検出可能」であることを機械確認する手段として採用。test-spec の lane-armed 要件（`rt-verification-contracts.md` Rule 1–2）に準拠。

### Rejected

- **audio-analysis の宣言を `level.rs`/`spectrum.rs` 側 `mod tests` に置く案** — どちらのモジュールに置くかが任意性になり、かつ crate-root 配置の video-proc #126 pattern から外れるため棄却。宣言のみを持つ `#[cfg(test)] mod tests` を `lib.rs` に新設する方が「test binary 1 個・宣言 1 個」の責務配置として明示的。
- **宣言の crate-level `#[cfg(test)]` 配置（`mod tests` の外）** — `#[global_allocator]` を crate-level item として `#[cfg(test)]` 付きで置く案は、`mod tests` 内配置の既存 convention（video-proc/plugin-clap/vst3/math/audio-device 全て `mod tests` 内）と不一致となるため棄却。

## Validation

| Gate | 結果 |
| --- | --- |
| `mise run fmt-check` | exit 0 |
| `mise run check` | exit 0（workspace --all-targets） |
| `mise run lint` | exit 0（workspace --all-targets・`-D warnings`） |
| `mise run test` | exit 0 — 全 test binary pass（59 result block 全て `0 failed`・flake なし） |
| `mise run test-rt` | exit 0 — media-core lib test binary で **5 件一致・全 pass**（`rt_checked_arithmetic`・`rt_cross_base_ordering`・`rt_convert_exact_and_rounding`・`rt_drop_frame_roundtrip`・`music::tests::rt_conversion_no_alloc`）・dsp で **1 件一致・全 pass**（`rt_process_paths_do_not_allocate`）・audio-analysis で **3 件一致・全 pass**（`level::tests::rt_process_allocates_nothing`・`spectrum::tests::rt_process_allocates_nothing` + 命名が substring `rt_` に偶然一致する `spectrum::tests::short_input_zero_pads_and_long_input_truncates` 1 件 — `sho**rt_**input`・非 RT の正常付随）。workspace 全体の他 crate rt_ テストも全 pass |
| sanity abort 検証 | 3 crate 分実施 — 各測定窓内の一時的 `vec![0_u8; 8]` で test binary が `memory allocation of 8 bytes failed` → abort（exit `0xc0000409`）を確認後、全て除去・再実行で pass |
| doc audit | `blocking: 0`・`findings: []`（最終編集後に実施 — Validation 記録参照） |

## Risks

- **`rt_` name filter は substring match** — audio-analysis の `short_input_zero_pads_and_long_input_truncates` が `sho**rt_**input` で lane に混入する（#126 の `resolved_tables_repo**rt_t**he_applied_value` と同じ既知の注意点・不変更）。RT 意味論を持たない test だが窓を持たない通常 test で実害はなく、一致件数の報告では区別して記録した。
- **`AllocDisabler` の武装範囲は各 lib test binary 1 個** — 3 crate とも `tests/*.rs` integration binary を現状持たないが、今後 integration test へ `rt_` テストを追加する場合はその binary ごとに同パターンの宣言が必要（#126 IR Risks と同一の注意点）。
- **宣言の重複はコンパイルエラーで検出される** — `#[global_allocator]` は binary ごとに 1 個のみ。今後 3 crate の別モジュールへ `AllocDisabler` を追加宣言すると重複エラーになるため、追加は crate-root `mod tests` の既存宣言を確認してから行う必要がある。

## Follow-ups

- なし — #126 IR Follow-ups の履行が本 Issue で完了し、新たな繰延は生じない。`rt_` substring 混入は既知の naming caveat として継続（#126 IR Risks と同じ扱い）。

## Decisions During Implementation

- **宣言位置は crate-root `mod tests`（video-proc #126 pattern）** — media-core/dsp は既存の lib.rs `mod tests` へ、audio-analysis は `mod tests` を持たなかったため宣言のみの `#[cfg(test)] mod tests` を `lib.rs` に新設。`level.rs`/`spectrum.rs` 側へ置く案は配置の任意性と pattern 不一致で棄却（Rejected 参照）。
- **窓の再構成は実施しなかった** — scope は「武装のみ」。事前確認で全対象窓の内外分割（fixture/prepare/assert が窓外・`process`/query が窓内）が既に正しいことを確認し、実測でも alloc 検出ゼロだったため窓・実装コードともに変更なし。
- **`Cargo.toml` の dev-dep コメントは追加しなかった** — `assert_no_alloc = "1.1"` は 3 crate とも宣言済みで、video-proc がコメントを付けたのは dev-dep を新規追加したため。本 Issue は宣言のみで依存変更がなく、コメント追加は行わない判断（差分を武装に限定）。

## Sources and Relations

- Issue [#130](https://github.com/scarecrow173/ruviform/issues/130): 3 crate の `assert_no_alloc` 窓が `AllocDisabler` 未宣言で無検証だった問題の武装 — #126 レビュー発見・pre-existing。
- Spec [issue-130-arm-assert-no-alloc-lane.md](../specs/issue-130-arm-assert-no-alloc-lane.md)・Plan [implement-issue-130-arm-assert-no-alloc.md](../plans/implement-issue-130-arm-assert-no-alloc.md)・Task [arm-assert-no-alloc-lane.md](../tasks/arm-assert-no-alloc-lane.md)。
- 発見元・宣言形式の正本: [issue-126-video-proc-rt-lane-implementation.md](issue-126-video-proc-rt-lane-implementation.md)（Follow-ups 行は本 Issue で解決済み注記へ更新）・`crates/ruviform-video-proc/src/lib.rs` の `mod tests` 内宣言。
- 規約根拠: [rt-assert-no-alloc-thread-check.md](../adr/rt-assert-no-alloc-thread-check.md)・[rt-verification-contracts.md](../test-specs/rt-verification-contracts.md)（Rule 1–2）・[rt-verification-lane-design.md](../designs/rt-verification-lane-design.md)。
- Refs: Issue [#126](https://github.com/scarecrow173/ruviform/issues/126)・[#22](https://github.com/scarecrow173/ruviform/issues/22)。

## Review Handoff

- 各 lib test binary に `AllocDisabler` が 1 個のみ宣言されているか（他 `mod tests` に重複宣言がないか）・`#[cfg(debug_assertions)]` 付きか・audio-analysis の新設 `mod tests` が宣言のみで過剰でないか・sanity abort 証跡（3 crate とも exit `0xc0000409`）と `test-rt` マッチ件数（media-core 5・dsp 1・audio-analysis 3 — substring 偶然一致 1 件を区別）の報告精度を確認されたい。**GitHub issue close・merge 判定は本 IR の scope 外 — push/PR は行っていない。**
